//! Multi-onglets côté interface (T1-08) : transport navigateur et dialogue.
//!
//! Le module n'écrit jamais dans le dépôt : il publie l'empreinte d'une
//! écriture locale réussie, retient les avis des autres onglets et calcule un
//! diff de comparaison à la demande. Aucune fusion, aucune persistance.
//!
//! Ce module est déclaré par la racine du crate (`lib.rs`, `pub mod tabs;`) et
//! consommé par `crate::state`.

use archifier_core::{diff_values, FieldDifference};
use archifier_repo::tabs::{
    ExternalWrite, InProcessTabTransport, TabGuard, TabId, TabMessage, TabTransport,
};
use leptos::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(target_arch = "wasm32")]
use archifier_repo::tabs::TabError;
#[cfg(target_arch = "wasm32")]
use archifier_repo::tabs::TAB_CHANNEL_NAME;

/// Limite pratique du snapshot de comparaison (contrat §9.3).
pub const SNAPSHOT_LIMIT: usize = 8 * 1024 * 1024;

const TOO_LARGE: &str =
    "État trop volumineux pour une comparaison détaillée : rechargez cet onglet.";
const WAITING: &str = "Comparaison demandée : en attente de la réponse d'un autre onglet…";
const NO_PEER: &str = "Pair indisponible : aucun autre onglet n'a répondu.";
const BAD_SNAPSHOT: &str = "Comparaison impossible : l'instantané reçu est illisible.";

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    static REQUEST_GENERATION: Cell<u64> = const { Cell::new(0) };
}

static FALLBACK_COUNTER: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Transport navigateur (wasm) et repli natif
// ---------------------------------------------------------------------------

/// Abonné durable au canal inter-onglets.
#[cfg(target_arch = "wasm32")]
type Listener = Rc<dyn Fn(TabMessage)>;
#[cfg(target_arch = "wasm32")]
type Listeners = Rc<RefCell<Vec<Listener>>>;

#[cfg(target_arch = "wasm32")]
type MessageCallback = wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MessageEvent)>;

/// `BroadcastChannel("archifier:repo:v1")` : messages JSON UTF-8, aucun réseau.
///
/// La construction est protégée : si l'API manque ou si l'ouverture échoue, le
/// transport reste [`TabTransport::status`] `Disabled` sans paniquer.
#[cfg(target_arch = "wasm32")]
pub struct LocalTabTransport {
    channel: RefCell<Option<web_sys::BroadcastChannel>>,
    on_message: RefCell<Option<MessageCallback>>,
    listeners: Listeners,
}

#[cfg(target_arch = "wasm32")]
impl LocalTabTransport {
    /// Ouvre le canal si le navigateur le permet ; sinon transport désactivé.
    pub fn new() -> Self {
        let transport = Self {
            channel: RefCell::new(None),
            on_message: RefCell::new(None),
            listeners: Rc::new(RefCell::new(Vec::new())),
        };
        transport.connect();
        transport
    }

    /// (Ré)installe l'écoute si le canal n'est pas déjà ouvert.
    pub fn connect(&self) {
        use wasm_bindgen::JsCast;

        if self.channel.borrow().is_some() {
            return;
        }
        let Ok(channel) = web_sys::BroadcastChannel::new(TAB_CHANNEL_NAME) else {
            return;
        };
        let listeners = Rc::clone(&self.listeners);
        let callback = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
            move |event: web_sys::MessageEvent| {
                let Some(raw) = event.data().as_string() else {
                    return;
                };
                let Ok(message) = serde_json::from_str::<TabMessage>(&raw) else {
                    return;
                };
                // Copie de la liste : un abonné qui s'abonne pendant la
                // diffusion ne provoque ni emprunt concurrent ni réentrance.
                let recipients: Vec<Listener> = listeners.borrow().clone();
                for listener in recipients {
                    listener(message.clone());
                }
            },
        );
        channel.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        *self.on_message.borrow_mut() = Some(callback);
        *self.channel.borrow_mut() = Some(channel);
    }

    /// Ferme le canal (équivalent `pagehide`). Idempotent.
    pub fn close(&self) {
        if let Some(channel) = self.channel.borrow_mut().take() {
            channel.set_onmessage(None);
            channel.close();
        }
        self.on_message.borrow_mut().take();
    }

    pub fn is_open(&self) -> bool {
        self.channel.borrow().is_some()
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for LocalTabTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl TabTransport for LocalTabTransport {
    fn send(&self, message: TabMessage) -> Result<(), TabError> {
        if message.tab().is_empty() {
            return Err(TabError::InvalidTab);
        }
        let Ok(json) = serde_json::to_string(&message) else {
            return Err(TabError::Closed);
        };
        let channel = self.channel.borrow();
        let Some(channel) = channel.as_ref() else {
            return Err(TabError::Disabled);
        };
        channel
            .post_message(&wasm_bindgen::JsValue::from_str(&json))
            .map_err(|_| TabError::Closed)
    }

    fn subscribe(&self, on: Box<dyn Fn(TabMessage) + 'static>) {
        self.listeners.borrow_mut().push(Rc::from(on));
    }

    fn status(&self) -> archifier_repo::tabs::TabStatus {
        if self.is_open() {
            archifier_repo::tabs::TabStatus::Local
        } else {
            archifier_repo::tabs::TabStatus::Disabled
        }
    }
}

/// Connexion de l'onglet : canal navigateur en wasm, canal mémoire désactivé
/// hors navigateur (tests natifs, `cargo check`).
enum Conn {
    #[cfg(target_arch = "wasm32")]
    Browser(Rc<LocalTabTransport>),
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Memory(Rc<InProcessTabTransport>),
}

impl Clone for Conn {
    fn clone(&self) -> Self {
        match self {
            #[cfg(target_arch = "wasm32")]
            Self::Browser(transport) => Self::Browser(Rc::clone(transport)),
            Self::Memory(transport) => Self::Memory(Rc::clone(transport)),
        }
    }
}

impl Conn {
    fn open() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self::Browser(Rc::new(LocalTabTransport::new()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::Memory(Rc::new(InProcessTabTransport::disabled()))
        }
    }

    fn transport(&self) -> &dyn TabTransport {
        match self {
            #[cfg(target_arch = "wasm32")]
            Self::Browser(transport) => transport.as_ref(),
            Self::Memory(transport) => transport.as_ref(),
        }
    }

    fn close(&self) {
        match self {
            #[cfg(target_arch = "wasm32")]
            Self::Browser(transport) => transport.close(),
            Self::Memory(transport) => transport.close(),
        }
    }

    fn resume(&self) {
        match self {
            #[cfg(target_arch = "wasm32")]
            Self::Browser(transport) => transport.connect(),
            Self::Memory(_) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Session d'onglet
// ---------------------------------------------------------------------------

struct Session {
    id: TabId,
    conn: Conn,
    guard: Rc<RefCell<TabGuard>>,
    bound: Cell<bool>,
    write: Cell<Option<RwSignal<Option<ExternalWrite>>>>,
    notice: Cell<Option<RwSignal<Option<String>>>>,
    changes: Cell<Option<RwSignal<Option<Vec<FieldDifference>>>>>,
}

/// Installe la session de l'onglet : identifiant de session, transport et
/// garde. À appeler une fois, avant le montage.
pub fn install() {
    let id = new_tab_id();
    let guard = Rc::new(RefCell::new(TabGuard::new(id.clone())));
    SESSION.with(|slot| {
        *slot.borrow_mut() = Some(Session {
            id,
            conn: Conn::open(),
            guard,
            bound: Cell::new(false),
            write: Cell::new(None),
            notice: Cell::new(None),
            changes: Cell::new(None),
        });
    });
}

/// Abonne l'interface au canal : avis d'écriture, demandes et réponses de
/// snapshot. Un seul abonnement par session ; les signaux sont réutilisés.
pub fn bind(
    external_write: RwSignal<Option<ExternalWrite>>,
    external_diff: RwSignal<Option<Vec<FieldDifference>>>,
    external_note: RwSignal<Option<String>>,
) {
    let Some((id, conn, guard)) = parts() else {
        return;
    };
    let already_bound = SESSION.with(|slot| match slot.borrow().as_ref() {
        Some(session) => {
            session.write.set(Some(external_write));
            session.notice.set(Some(external_note));
            session.changes.set(Some(external_diff));
            session.bound.replace(true)
        }
        None => true,
    });
    if already_bound {
        return;
    }

    let local = id;
    let source = conn.clone();
    let guard_for_listener = Rc::clone(&guard);
    conn.transport()
        .subscribe(Box::new(move |message: TabMessage| {
            let retained = {
                let mut guard = guard_for_listener.borrow_mut();
                if guard.observe(&message) {
                    guard.external().cloned()
                } else {
                    None
                }
            };
            if let Some(write) = retained {
                external_diff.set(None);
                external_note.set(None);
                external_write.set(Some(write));
                return;
            }
            match message {
                // Message du propre onglet ou avis déjà traité : rien à faire.
                TabMessage::Write { .. } => {}
                TabMessage::SnapshotRequest { tab } => {
                    if tab == local {
                        return;
                    }
                    let json = crate::state::repo().export_json();
                    if json.len() > SNAPSHOT_LIMIT {
                        external_note.set(Some(TOO_LARGE.into()));
                        return;
                    }
                    let _ = source.transport().send(TabMessage::SnapshotResponse {
                        tab: local.clone(),
                        json,
                    });
                }
                TabMessage::SnapshotResponse { tab, json } => {
                    if tab == local {
                        return;
                    }
                    compare(json, external_diff, external_note);
                }
            }
        }));
}

/// Publie l'avis d'écriture (`fingerprint` = SHA-256 de l'export).
pub fn publish_write(fingerprint: &str) {
    let Some((id, conn, _guard)) = parts() else {
        return;
    };
    if !conn.transport().status().is_enabled() {
        return;
    }
    let _ = conn.transport().send(TabMessage::Write {
        tab: id,
        at_ms: js_sys::Date::now() as u64,
        fingerprint: fingerprint.to_string(),
    });
}

/// Demande le snapshot courant aux pairs pour comparaison (aucune écriture).
pub fn request_snapshot() {
    let Some((id, conn, _guard)) = parts() else {
        return;
    };
    let notice = SESSION.with(|slot| slot.borrow().as_ref().and_then(|s| s.notice.get()));
    let changes = SESSION.with(|slot| slot.borrow().as_ref().and_then(|s| s.changes.get()));
    if !conn.transport().status().is_enabled() {
        if let Some(notice) = notice {
            notice.set(Some(
                "Synchronisation inter-onglets indisponible dans ce navigateur.".into(),
            ));
        }
        return;
    }
    if let Some(changes) = changes {
        changes.set(None);
    }
    if let Some(notice) = notice {
        notice.set(Some(WAITING.into()));
    }
    let _ = conn
        .transport()
        .send(TabMessage::SnapshotRequest { tab: id });
    arm_timeout();
}

/// Retire l'avis retenu sans rien appliquer (« Forcer »).
pub fn dismiss() {
    SESSION.with(|slot| {
        if let Some(session) = slot.borrow().as_ref() {
            session.guard.borrow_mut().dismiss();
        }
    });
}

/// Ferme le canal (`pagehide`, page non conservée en bfcache).
pub fn close() {
    let Some((_id, conn, _guard)) = parts() else {
        return;
    };
    conn.close();
}

/// Réabonne le canal au retour de la page (`pageshow`, visibilité).
///
/// Vérification best-effort : un message manqué en arrière-plan n'est pas
/// rattrapé, aucun snapshot n'est demandé automatiquement (cela ouvrirait un
/// dialogue sans action de l'utilisateur).
pub fn resume() {
    let Some((_id, conn, _guard)) = parts() else {
        return;
    };
    conn.resume();
}

// ---------------------------------------------------------------------------
// Internes
// ---------------------------------------------------------------------------

fn parts() -> Option<(TabId, Conn, Rc<RefCell<TabGuard>>)> {
    SESSION.with(|slot| {
        slot.borrow().as_ref().map(|session| {
            (
                session.id.clone(),
                session.conn.clone(),
                Rc::clone(&session.guard),
            )
        })
    })
}

/// Diff local → distant, sans application ni persistance.
fn compare(
    json: String,
    external_diff: RwSignal<Option<Vec<FieldDifference>>>,
    external_note: RwSignal<Option<String>>,
) {
    if json.len() > SNAPSHOT_LIMIT {
        external_diff.set(None);
        external_note.set(Some(TOO_LARGE.into()));
        return;
    }
    let local_json = crate::state::repo().export_json();
    match (
        serde_json::from_str::<serde_json::Value>(&local_json),
        serde_json::from_str::<serde_json::Value>(&json),
    ) {
        (Ok(local), Ok(remote)) => {
            external_note.set(None);
            external_diff.set(Some(diff_values(&local, &remote)));
        }
        _ => {
            external_diff.set(None);
            external_note.set(Some(BAD_SNAPSHOT.into()));
        }
    }
}

/// « Pair indisponible » si aucune réponse n'arrive dans le délai imparti.
fn arm_timeout() {
    use wasm_bindgen::JsCast;

    let generation = REQUEST_GENERATION.with(|slot| {
        let next = slot.get().wrapping_add(1);
        slot.set(next);
        next
    });
    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
        if REQUEST_GENERATION.with(Cell::get) != generation {
            return;
        }
        SESSION.with(|slot| {
            if let Some(session) = slot.borrow().as_ref() {
                // Dialogue refermé entre-temps : ne pas le rouvrir.
                if let Some(write) = session.write.get() {
                    if write.get_untracked().is_none() {
                        return;
                    }
                } else {
                    return;
                }
                if let Some(notice) = session.notice.get() {
                    if notice.get_untracked().as_deref() == Some(WAITING) {
                        notice.set(Some(NO_PEER.into()));
                    }
                }
            }
        });
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.unchecked_ref::<js_sys::Function>(),
        1_500,
    );
}

fn new_tab_id() -> TabId {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(id) = random_tab_id() {
            return id;
        }
    }
    fallback_tab_id()
}

/// 128 bits via `crypto.getRandomValues` ; `None` si l'API manque.
#[cfg(target_arch = "wasm32")]
fn random_tab_id() -> Option<TabId> {
    use wasm_bindgen::{JsCast, JsValue};

    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &JsValue::from_str("crypto")).ok()?;
    if crypto.is_undefined() || crypto.is_null() {
        return None;
    }
    let function = js_sys::Reflect::get(&crypto, &JsValue::from_str("getRandomValues")).ok()?;
    let function = function.dyn_into::<js_sys::Function>().ok()?;
    let buffer = js_sys::Uint8Array::new_with_length(16);
    function.call1(&crypto, &buffer).ok()?;
    let mut bytes = [0u8; 16];
    buffer.copy_to(&mut bytes);
    Some(format!("tab-{}", hex(&bytes)))
}

/// Repli session : temps + aléa + compteur, jamais persisté.
fn fallback_tab_id() -> TabId {
    let counter = FALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed);
    #[cfg(target_arch = "wasm32")]
    {
        let now = js_sys::Date::now() as u64;
        let first = (js_sys::Math::random() * 1_000_000_000.0) as u64;
        let second = (js_sys::Math::random() * 1_000_000_000.0) as u64;
        format!("tab-{now:x}-{counter:x}-{first:x}-{second:x}")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        format!("tab-native-{counter:x}")
    }
}

#[cfg(target_arch = "wasm32")]
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
