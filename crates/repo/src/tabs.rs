//! Multi-onglets : détection d'écriture externe (T1-08).
//!
//! Chaque onglet possède un [`TabId`] de session, jamais persisté ni exporté.
//! Après une écriture locale réussie, l'onglet publie un
//! [`TabMessage::Write`] sur le canal [`TAB_CHANNEL_NAME`] ; les autres onglets
//! retiennent l'avis avec [`TabGuard`] et proposent recharger / comparer /
//! forcer. Aucune fusion automatique, aucune écriture silencieuse.
//!
//! Ce module est sans entrée-sortie : le transport navigateur
//! (`BroadcastChannel`) vit dans la couche UI, tandis que
//! [`InProcessTabTransport`] fournit un canal mémoire pour les tests natifs.
//! `TabGuard` ne voit jamais le [`Dataset`](archifier_core::Dataset) : il ne
//! peut donc ni le lire ni le modifier.

use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Nom du canal inter-onglets (`BroadcastChannel`).
pub const TAB_CHANNEL_NAME: &str = "archifier:repo:v1";

/// Identifiant de session d'un onglet. Jamais persisté, jamais exporté.
pub type TabId = String;

/// Message échangé entre onglets.
///
/// Forme de fil (JSON UTF-8) : `{"type":"write","tab":…,"at_ms":…,
/// "fingerprint":…}`, `{"type":"snapshot_request","tab":…}`,
/// `{"type":"snapshot_response","tab":…,"json":…}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TabMessage {
    /// Écriture locale réussie. Ne transporte jamais le snapshot complet.
    Write {
        tab: TabId,
        at_ms: u64,
        fingerprint: String,
    },
    /// Demande du snapshot courant pour comparaison.
    SnapshotRequest { tab: TabId },
    /// Réponse à une demande de comparaison.
    SnapshotResponse { tab: TabId, json: String },
}

impl TabMessage {
    /// Onglet émetteur du message.
    pub fn tab(&self) -> &str {
        match self {
            Self::Write { tab, .. }
            | Self::SnapshotRequest { tab }
            | Self::SnapshotResponse { tab, .. } => tab,
        }
    }

    /// Vrai uniquement pour un avis d'écriture ; les messages de comparaison
    /// ne sont pas des écritures.
    pub fn is_write(&self) -> bool {
        matches!(self, Self::Write { .. })
    }
}

/// Avis d'écriture retenu pour un onglet local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalWrite {
    pub tab: TabId,
    pub at_ms: u64,
    pub fingerprint: String,
}

/// État de la synchronisation inter-onglets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabStatus {
    /// Aucun canal disponible : l'onglet travaille seul.
    Disabled,
    /// Canal ouvert, aucun pair n'a encore écrit.
    Local,
    /// Canal ouvert et au moins un pair connu.
    Connected,
}

impl TabStatus {
    /// Vrai si un canal existe, quel qu'il soit.
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

/// Erreur typée du transport inter-onglets.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TabError {
    #[error("synchronisation inter-onglets indisponible")]
    Disabled,
    #[error("canal inter-onglets fermé")]
    Closed,
    #[error("message inter-onglets sans onglet émetteur")]
    InvalidTab,
}

/// Canal de diffusion entre onglets.
///
/// `subscribe` installe un abonnement durable ; plusieurs abonnements peuvent
/// coexister sur le même onglet. Le canal ne conserve aucun message : un
/// onglet qui arrive après une écriture ne la voit pas.
pub trait TabTransport {
    fn send(&self, message: TabMessage) -> Result<(), TabError>;
    fn subscribe(&self, on: Box<dyn Fn(TabMessage) + 'static>);
    fn status(&self) -> TabStatus;
}

type Listener = Rc<dyn Fn(TabMessage)>;

/// Un onglet abonné au canal mémoire : ses abonnements et son état d'ouverture.
struct Endpoint {
    open: Cell<bool>,
    listeners: RefCell<Vec<Listener>>,
}

impl Endpoint {
    fn new() -> Self {
        Self {
            open: Cell::new(true),
            listeners: RefCell::new(Vec::new()),
        }
    }

    fn snapshot(&self) -> Vec<Listener> {
        self.listeners.borrow().clone()
    }
}

/// Le canal partagé : disponibilité et liste des onglets abonnés.
struct Channel {
    available: Cell<bool>,
    endpoints: RefCell<Vec<Rc<Endpoint>>>,
}

/// Transport mémoire : un canal partagé, pour les tests natifs.
///
/// Chaque transport représente un onglet ; `pair()` crée deux onglets reliés
/// par le même canal, tandis que `clone()` ne crée qu'un second handle sur le
/// même onglet. Comme `BroadcastChannel`, un message n'est jamais rediffusé à
/// son émetteur ; `close()` ne ferme que l'onglet local.
#[derive(Clone)]
pub struct InProcessTabTransport {
    channel: Rc<Channel>,
    endpoint: Rc<Endpoint>,
}

impl InProcessTabTransport {
    /// Canal ouvert, avec un seul onglet.
    pub fn new() -> Self {
        Self::with_availability(true)
    }

    /// Simule un navigateur sans `BroadcastChannel`.
    pub fn disabled() -> Self {
        Self::with_availability(false)
    }

    /// Deux onglets distincts reliés par le même canal mémoire.
    pub fn pair() -> (Self, Self) {
        let first = Self::new();
        let second = first.linked_endpoint();
        (first, second)
    }

    fn with_availability(available: bool) -> Self {
        let endpoint = Rc::new(Endpoint::new());
        Self {
            channel: Rc::new(Channel {
                available: Cell::new(available),
                endpoints: RefCell::new(vec![Rc::clone(&endpoint)]),
            }),
            endpoint,
        }
    }

    /// Nouvel onglet sur le même canal, sans partager les abonnements.
    fn linked_endpoint(&self) -> Self {
        let endpoint = Rc::new(Endpoint::new());
        self.channel
            .endpoints
            .borrow_mut()
            .push(Rc::clone(&endpoint));
        Self {
            channel: Rc::clone(&self.channel),
            endpoint,
        }
    }

    /// Ferme l'onglet local (équivalent `pagehide`). Idempotent ; les autres
    /// onglets du même canal ne sont pas affectés.
    pub fn close(&self) {
        self.endpoint.open.set(false);
    }

    /// Vrai tant que l'onglet peut émettre et recevoir.
    pub fn is_open(&self) -> bool {
        self.channel.available.get() && self.endpoint.open.get()
    }

    /// Nombre d'abonnements installés sur cet onglet.
    pub fn subscriber_count(&self) -> usize {
        self.endpoint.listeners.borrow().len()
    }
}

impl Default for InProcessTabTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl TabTransport for InProcessTabTransport {
    fn send(&self, message: TabMessage) -> Result<(), TabError> {
        if !self.endpoint.open.get() {
            return Err(TabError::Closed);
        }
        if !self.channel.available.get() {
            return Err(TabError::Disabled);
        }
        if message.tab().is_empty() {
            return Err(TabError::InvalidTab);
        }
        // Copie de la liste : un abonné qui s'abonne pendant la diffusion ne
        // provoque ni emprunt concurrent ni réentrance.
        let endpoints: Vec<Rc<Endpoint>> = self.channel.endpoints.borrow().clone();
        for endpoint in endpoints {
            if Rc::ptr_eq(&endpoint, &self.endpoint) {
                continue;
            }
            for listener in endpoint.snapshot() {
                listener(message.clone());
            }
        }
        Ok(())
    }

    fn subscribe(&self, on: Box<dyn Fn(TabMessage) + 'static>) {
        self.endpoint.listeners.borrow_mut().push(Rc::from(on));
    }

    fn status(&self) -> TabStatus {
        if self.is_open() {
            TabStatus::Local
        } else {
            TabStatus::Disabled
        }
    }
}

/// Filtre des avis d'écriture pour un onglet local.
///
/// Le garde ne retient qu'un avis à la fois : le dernier `Write` externe reçu
/// remplace le précédent. Il ne modifie jamais le dépôt ; `take_external` et
/// `dismiss` ne changent que son propre état.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabGuard {
    local: TabId,
    external: Option<ExternalWrite>,
}

impl TabGuard {
    pub fn new(local: TabId) -> Self {
        Self {
            local,
            external: None,
        }
    }

    /// Observe un message du canal. Renvoie `true` si un avis externe est
    /// retenu. Les messages du propre onglet et les échanges de snapshot sont
    /// ignorés : ils ne modifient pas l'état du garde.
    pub fn observe(&mut self, message: &TabMessage) -> bool {
        let TabMessage::Write {
            tab,
            at_ms,
            fingerprint,
        } = message
        else {
            return false;
        };
        if *tab == self.local {
            return false;
        }
        self.external = Some(ExternalWrite {
            tab: tab.clone(),
            at_ms: *at_ms,
            fingerprint: fingerprint.clone(),
        });
        true
    }

    /// Retire l'avis courant, par exemple pour ouvrir le dialogue une seule
    /// fois. Aucune écriture n'est déclenchée.
    pub fn take_external(&mut self) -> Option<ExternalWrite> {
        self.external.take()
    }

    /// Ferme l'avis sans rien faire : l'onglet continue avec son état local
    /// (« Forcer »). Aucune fusion, aucune écriture.
    pub fn dismiss(&mut self) {
        self.external = None;
    }

    pub fn local_id(&self) -> &str {
        &self.local
    }

    /// Avis courant, sans le consommer.
    pub fn external(&self) -> Option<&ExternalWrite> {
        self.external.as_ref()
    }

    pub fn has_external(&self) -> bool {
        self.external.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{seed, MemoryStore, Repository};

    type Inbox = Rc<RefCell<Vec<TabMessage>>>;
    type Probe = (Inbox, Box<dyn Fn(TabMessage)>);

    fn write(tab: &str, at_ms: u64, fingerprint: &str) -> TabMessage {
        TabMessage::Write {
            tab: tab.into(),
            at_ms,
            fingerprint: fingerprint.into(),
        }
    }

    fn inbox() -> Inbox {
        Rc::new(RefCell::new(Vec::new()))
    }

    fn listen(transport: &InProcessTabTransport, inbox: &Inbox) {
        let sink = Rc::clone(inbox);
        transport.subscribe(Box::new(move |message: TabMessage| {
            sink.borrow_mut().push(message)
        }));
    }

    fn collecting() -> Probe {
        let inbox = inbox();
        let sink = Rc::clone(&inbox);
        let listener = Box::new(move |message: TabMessage| sink.borrow_mut().push(message));
        (inbox, listener)
    }

    #[test]
    fn external_write_is_detected() {
        let (tab_a, tab_b) = InProcessTabTransport::pair();
        let heard = inbox();
        listen(&tab_b, &heard);

        let guard = Rc::new(RefCell::new(TabGuard::new("tab-a".into())));
        let pending = Rc::clone(&guard);
        let observed = Rc::new(Cell::new(false));
        let flag = Rc::clone(&observed);
        tab_b.subscribe(Box::new(move |message: TabMessage| {
            if pending.borrow_mut().observe(&message) {
                flag.set(true);
            }
        }));

        let alert = write("tab-b", 1_000, "abc");
        tab_a.send(alert.clone()).expect("diffusion");

        assert_eq!(heard.borrow().as_slice(), std::slice::from_ref(&alert));
        assert!(observed.get());
        let external = guard.borrow_mut().take_external().expect("avis retenu");
        assert_eq!(external.tab, "tab-b");
        assert_eq!(external.at_ms, 1_000);
        assert_eq!(external.fingerprint, "abc");
    }

    #[test]
    fn external_write_ignores_own_tab() {
        let mut guard = TabGuard::new("tab-a".into());
        assert!(!guard.observe(&write("tab-a", 5, "self")));
        assert!(!guard.has_external());
        assert!(guard.take_external().is_none());
    }

    #[test]
    fn external_write_replaces_last_notice() {
        let mut guard = TabGuard::new("tab-a".into());
        assert!(guard.observe(&write("tab-b", 10, "first")));
        assert!(guard.observe(&write("tab-c", 20, "second")));

        let external = guard.external().expect("avis courant");
        assert_eq!(external.tab, "tab-c");
        assert_eq!(external.fingerprint, "second");
        assert_eq!(guard.take_external().map(|w| w.tab), Some("tab-c".into()));
        assert!(guard.take_external().is_none());
    }

    #[test]
    fn tab_guard_dismisses_without_writing() {
        let repo = Repository::new(Rc::new(MemoryStore::new()), 1_795_000_000_000, 42);
        seed::seed_demo(&repo).expect("jeu de démonstration");
        let before = repo.export_json();

        let mut guard = TabGuard::new("tab-a".into());
        let fingerprint = archifier_core::sha256_hex(before.as_bytes());
        assert!(guard.observe(&write("tab-b", 1_000, &fingerprint)));
        guard.observe(&TabMessage::SnapshotResponse {
            tab: "tab-b".into(),
            json: before.clone(),
        });
        guard.dismiss();

        assert!(!guard.has_external());
        assert_eq!(guard.local_id(), "tab-a");
        assert_eq!(before, repo.export_json());
    }

    #[test]
    fn snapshot_messages_are_not_writes() {
        let mut guard = TabGuard::new("tab-a".into());
        assert!(guard.observe(&write("tab-b", 1, "fp")));

        let request = TabMessage::SnapshotRequest {
            tab: "tab-b".into(),
        };
        let response = TabMessage::SnapshotResponse {
            tab: "tab-b".into(),
            json: "{}".into(),
        };
        assert!(!guard.observe(&request));
        assert!(!guard.observe(&response));
        assert!(!request.is_write());
        assert!(!response.is_write());

        // L'avis d'écriture précédent reste intact et unique.
        let external = guard.take_external().expect("avis conservé");
        assert_eq!(external.fingerprint, "fp");
        assert!(!guard.has_external());
    }

    #[test]
    fn snapshot_messages_change_nothing_when_no_write_is_pending() {
        let mut guard = TabGuard::new("tab-a".into());
        assert!(!guard.observe(&TabMessage::SnapshotRequest {
            tab: "tab-b".into(),
        }));
        assert!(!guard.observe(&TabMessage::SnapshotResponse {
            tab: "tab-b".into(),
            json: "{}".into(),
        }));
        assert!(!guard.has_external());
    }

    #[test]
    fn transport_broadcasts_to_multiple_subscribers() {
        let (tab_a, tab_b) = InProcessTabTransport::pair();
        let (first, first_listener) = collecting();
        let (second, second_listener) = collecting();

        tab_b.subscribe(first_listener);
        tab_b.subscribe(second_listener);
        assert_eq!(tab_b.subscriber_count(), 2);

        let alert = write("tab-a", 7, "fp");
        tab_a.send(alert.clone()).expect("diffusion");

        assert_eq!(first.borrow().as_slice(), std::slice::from_ref(&alert));
        assert_eq!(second.borrow().as_slice(), std::slice::from_ref(&alert));
    }

    #[test]
    fn transport_does_not_echo_to_sender() {
        let (tab_a, tab_b) = InProcessTabTransport::pair();
        let own = inbox();
        let other = inbox();
        listen(&tab_a, &own);
        listen(&tab_b, &other);

        tab_a.send(write("tab-a", 1, "fp")).expect("diffusion");

        assert!(own.borrow().is_empty(), "pas d'écho vers l'émetteur");
        assert_eq!(other.borrow().len(), 1);
    }

    #[test]
    fn cloned_transport_shares_its_endpoint() {
        let tab_a = InProcessTabTransport::new();
        let handle = tab_a.clone();
        let seen = inbox();
        listen(&handle, &seen);

        assert_eq!(tab_a.subscriber_count(), 1, "même onglet, mêmes abonnés");
        tab_a.send(write("tab-a", 1, "fp")).expect("diffusion");
        assert!(
            seen.borrow().is_empty(),
            "un clone n'est pas un second onglet"
        );
    }

    #[test]
    fn transport_delivers_every_message_kind() {
        let (tab_a, tab_b) = InProcessTabTransport::pair();
        let seen = inbox();
        listen(&tab_b, &seen);

        let request = TabMessage::SnapshotRequest {
            tab: "tab-a".into(),
        };
        tab_a.send(request.clone()).expect("demande");
        tab_a
            .send(TabMessage::SnapshotResponse {
                tab: "tab-a".into(),
                json: "{}".into(),
            })
            .expect("réponse");

        let messages = seen.borrow();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], request);
        assert_eq!(messages[0].tab(), "tab-a");
        assert!(!messages[1].is_write());
    }

    #[test]
    fn tab_transport_status_is_local_or_disabled() {
        let (tab_a, tab_b) = InProcessTabTransport::pair();
        assert_eq!(tab_a.status(), TabStatus::Local);
        assert_eq!(tab_b.status(), TabStatus::Local);
        assert!(tab_a.status().is_enabled());

        let disabled = InProcessTabTransport::disabled();
        assert_eq!(disabled.status(), TabStatus::Disabled);
        assert!(!disabled.status().is_enabled());
        assert_eq!(
            disabled.send(write("tab-a", 1, "fp")),
            Err(TabError::Disabled)
        );

        tab_a.close();
        assert_eq!(tab_a.status(), TabStatus::Disabled);
        assert_eq!(tab_b.status(), TabStatus::Local, "l'autre onglet survit");
        assert_eq!(tab_a.send(write("tab-a", 1, "fp")), Err(TabError::Closed));
        assert!(tab_b.send(write("tab-b", 1, "fp")).is_ok());
    }

    #[test]
    fn closed_transport_is_idempotent() {
        let transport = InProcessTabTransport::new();
        transport.close();
        transport.close();
        assert!(!transport.is_open());
        assert_eq!(transport.status(), TabStatus::Disabled);
    }

    #[test]
    fn transport_rejects_message_without_tab() {
        let transport = InProcessTabTransport::new();
        assert_eq!(
            transport.send(write("", 1, "fp")),
            Err(TabError::InvalidTab)
        );
    }

    #[test]
    fn write_message_serialises_with_type_tag() {
        let json = serde_json::to_value(write("tab-b", 1_000, "fp")).expect("sérialisation");
        assert_eq!(json["type"], serde_json::json!("write"));
        assert_eq!(json["tab"], serde_json::json!("tab-b"));
        assert_eq!(json["at_ms"], serde_json::json!(1_000));
        assert_eq!(json["fingerprint"], serde_json::json!("fp"));

        let decoded: TabMessage = serde_json::from_value(json).expect("désérialisation");
        assert_eq!(decoded, write("tab-b", 1_000, "fp"));

        let envelope = serde_json::to_value(TabMessage::SnapshotRequest {
            tab: "tab-a".into(),
        })
        .expect("sérialisation");
        assert_eq!(envelope["type"], serde_json::json!("snapshot_request"));
    }
}
