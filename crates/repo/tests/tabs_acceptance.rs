//! Tests bloquants W1c-2 — multi-onglets et écriture externe (T1-08).
//!
//! Contrat : `docs/w1c2-contract.md` §2 à §6.
//!
//! Le canal réel (`LocalTabTransport`, `BroadcastChannel`) est wasm/UI. Ces
//! tests couvrent le transport mémoire (`InProcessTabTransport`), le
//! `TabGuard` headless et la forme des messages. Les trois choix du dialogue
//! (§5) sont représentés sans interface :
//!
//! - **Recharger** : l'avis externe est consommé, aucun état distant n'est
//!   appliqué au dépôt local ;
//! - **Comparer** : `SnapshotRequest` / `SnapshotResponse` puis `diff_values`
//!   sur les snapshots, aucune application ;
//! - **Forcer** : `TabGuard::dismiss`, aucune fusion.
//!
//! Hypothèses d'API non détaillées au contrat : `InProcessTabTransport::pair()`
//! crée deux endpoints distincts du même canal. `send` ne délivre qu'aux
//! **autres** endpoints : un onglet ne reçoit jamais ses propres messages, et
//! `subscribe` s'installe donc sur le transport récepteur. La diffusion est
//! synchrone et `close()` ne ferme que l'endpoint local. Le point unique
//! d'adaptation est `linked_transports()`.

use archifier_core::{diff_values, sha256_hex};
use archifier_repo::tabs::{
    ExternalWrite, InProcessTabTransport, TabGuard, TabMessage, TabStatus, TabTransport,
};
use archifier_repo::{MemoryStore, Repository};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;

const NOW_MS: u64 = 1_795_000_000_000; // 2026-11-18, horloge figée

/// Identifiants d'onglet volontairement distinctifs : ils ne doivent jamais
/// apparaître dans un export ni dans un snapshot persisté.
const TAB_A: &str = "tab-01K8ZP4N000000000000000T0A";
const TAB_B: &str = "tab-01K8ZP4N000000000000000T0B";

type Inbox = Rc<RefCell<Vec<TabMessage>>>;

// ---------------------------------------------------------------------------
// Adaptateurs de test
// ---------------------------------------------------------------------------

/// Deux endpoints distincts du même canal mémoire.
fn linked_transports() -> (InProcessTabTransport, InProcessTabTransport) {
    InProcessTabTransport::pair()
}

fn inbox() -> Inbox {
    Rc::new(RefCell::new(Vec::new()))
}

fn listen(transport: &InProcessTabTransport, inbox: &Inbox) {
    let sink = Rc::clone(inbox);
    transport.subscribe(Box::new(move |message| sink.borrow_mut().push(message)));
}

fn messages(inbox: &Inbox) -> Vec<TabMessage> {
    inbox.borrow().clone()
}

fn repo(seed: u64) -> (Repository, Rc<MemoryStore>) {
    let store = Rc::new(MemoryStore::new());
    (Repository::new(store.clone(), NOW_MS, seed), store)
}

fn fingerprint_of(repo: &Repository) -> String {
    sha256_hex(repo.export_json().as_bytes())
}

fn write_message(tab: &str, at_ms: u64, fingerprint: impl Into<String>) -> TabMessage {
    TabMessage::Write {
        tab: tab.to_string(),
        at_ms,
        fingerprint: fingerprint.into(),
    }
}

fn snapshot_request_message(tab: &str) -> TabMessage {
    TabMessage::SnapshotRequest {
        tab: tab.to_string(),
    }
}

fn snapshot_response_message(tab: &str, json: impl Into<String>) -> TabMessage {
    TabMessage::SnapshotResponse {
        tab: tab.to_string(),
        json: json.into(),
    }
}

/// Dernière réponse de snapshot venue de `from`, en ignorant un éventuel écho
/// de la demande sur le même transport.
fn snapshot_response_from(inbox: &Inbox, from: &str) -> Option<String> {
    messages(inbox)
        .into_iter()
        .rev()
        .find_map(|message| match message {
            TabMessage::SnapshotResponse { tab, json } if tab.as_str() == from => Some(json),
            _ => None,
        })
}

fn snapshot_request_from(inbox: &Inbox, from: &str) -> bool {
    messages(inbox).iter().any(|message| {
        matches!(
            message,
            TabMessage::SnapshotRequest { tab } if tab.as_str() == from
        )
    })
}

fn latest_write(inbox: &Inbox) -> Option<TabMessage> {
    messages(inbox)
        .into_iter()
        .rev()
        .find(|message| matches!(message, TabMessage::Write { .. }))
}

fn collect_keys(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                out.push(key.clone());
                collect_keys(item, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_keys(item, out);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Tests bloquants §6
// ---------------------------------------------------------------------------

#[test]
fn external_write_is_detected() {
    let (tab_a, tab_b) = linked_transports();
    let heard = inbox();
    listen(&tab_b, &heard);

    let mut guard = TabGuard::new(TAB_B.to_string());
    assert_eq!(guard.local_id(), TAB_B);

    let first = write_message(TAB_A, NOW_MS, "a".repeat(64));
    assert!(
        tab_a.send(first.clone()).is_ok(),
        "le canal mémoire accepte l'avis d'écriture"
    );

    assert_eq!(
        messages(&heard),
        vec![first.clone()],
        "l'autre onglet reçoit exactement l'avis émis"
    );
    assert!(guard.observe(&first), "écriture d'un autre onglet retenue");

    let external = guard.take_external().expect("avis externe disponible");
    assert_eq!(
        external,
        ExternalWrite {
            tab: TAB_A.to_string(),
            at_ms: NOW_MS,
            fingerprint: "a".repeat(64),
        }
    );
    assert!(
        guard.take_external().is_none(),
        "l'avis est consommé une seule fois"
    );

    // Le plus récent gagne : un second avis remplace le premier.
    let second = write_message(TAB_A, NOW_MS + 5, "b".repeat(64));
    assert!(tab_a.send(second.clone()).is_ok());
    assert!(guard.observe(&second));
    let latest = guard.take_external().expect("second avis");
    assert_eq!(latest.at_ms, NOW_MS + 5);
    assert_eq!(latest.fingerprint, "b".repeat(64));
}

#[test]
fn external_write_ignores_own_tab() {
    let mut guard = TabGuard::new(TAB_A.to_string());

    let own_write = write_message(TAB_A, NOW_MS, "own");
    assert!(!guard.observe(&own_write), "sa propre écriture est ignorée");
    assert!(guard.take_external().is_none());

    // Les messages de snapshot ne sont pas des écritures, même émis par soi.
    assert!(!guard.observe(&snapshot_request_message(TAB_A)));
    assert!(!guard.observe(&snapshot_response_message(TAB_A, "{}")));
    assert!(guard.take_external().is_none());

    // Un avis externe déjà retenu n'est pas écrasé par une écriture locale.
    let external = write_message(TAB_B, NOW_MS + 1, "external");
    assert!(guard.observe(&external));
    assert!(!guard.observe(&own_write));
    let pending = guard.take_external().expect("l'avis externe reste retenu");
    assert_eq!(pending.tab, TAB_B);
}

#[test]
fn external_write_offers_reload_compare_force() {
    let (repo_a, store_a) = repo(42);
    repo_a
        .create_domain("Paiements", "écrit par l'autre onglet")
        .expect("domaine distant");
    let before_a = repo_a.export_json();
    let saves_a = store_a.save_count();

    let (repo_b, store_b) = repo(43);
    repo_b
        .create_domain("Commande", "état local")
        .expect("domaine local");
    let before_b = repo_b.export_json();
    let saves_b = store_b.save_count();

    let (tab_a, tab_b) = linked_transports();
    let a_inbox = inbox();
    listen(&tab_a, &a_inbox);
    let b_inbox = inbox();
    listen(&tab_b, &b_inbox);

    let mut guard_a = TabGuard::new(TAB_A.to_string());
    let mut guard_b = TabGuard::new(TAB_B.to_string());

    // L'avis d'écriture suffit à proposer les trois choix, sans snapshot.
    assert!(tab_a
        .send(write_message(TAB_A, NOW_MS, fingerprint_of(&repo_a)))
        .is_ok());
    let notice = latest_write(&b_inbox).expect("avis reçu par B");
    assert!(guard_b.observe(&notice));

    // --- Comparer : demande de snapshot au pair, sans application. ----------
    assert!(tab_b.send(snapshot_request_message(TAB_B)).is_ok());
    assert!(
        snapshot_request_from(&a_inbox, TAB_B),
        "A reçoit la demande de B"
    );
    assert!(
        !guard_a.observe(&snapshot_request_message(TAB_B)),
        "une demande de snapshot n'est pas une écriture"
    );
    assert!(guard_a.take_external().is_none());

    assert!(tab_a
        .send(snapshot_response_message(TAB_A, repo_a.export_json()))
        .is_ok());
    let remote_json = snapshot_response_from(&b_inbox, TAB_A).expect("A répond à B");

    let local: Value = serde_json::from_str(&before_b).expect("snapshot local");
    let remote: Value = serde_json::from_str(&remote_json).expect("snapshot distant");
    let diff = diff_values(&local, &remote);
    assert!(!diff.is_empty(), "les deux états diffèrent");
    assert!(
        diff.iter().any(|difference| {
            difference.path == "/domains/0/object/name"
                && difference.before == Some(json!("Commande"))
                && difference.after == Some(json!("Paiements"))
        }),
        "le diff porte le changement de domaine : {diff:?}"
    );
    assert_eq!(repo_b.export_json(), before_b, "Comparer n'applique rien");
    assert_eq!(repo_a.export_json(), before_a, "Comparer ne modifie pas A");
    assert_eq!(store_b.save_count(), saves_b, "Comparer n'écrit pas");
    assert_eq!(store_a.save_count(), saves_a);

    // --- Recharger : l'avis est consommé, aucun état distant appliqué. ------
    let pending = guard_b.take_external().expect("avis pour Recharger");
    assert_eq!(pending.tab, TAB_A);
    assert_eq!(
        repo_b.export_json(),
        before_b,
        "consommer l'avis ne recharge pas en headless et ne fusionne pas"
    );

    // --- Forcer : ferme l'avis et continue avec l'état local. ---------------
    assert!(tab_a
        .send(write_message(TAB_A, NOW_MS + 1, fingerprint_of(&repo_a)))
        .is_ok());
    let notice = latest_write(&b_inbox).expect("second avis");
    assert!(guard_b.observe(&notice));
    guard_b.dismiss();
    assert!(
        guard_b.take_external().is_none(),
        "Forcer ferme l'avis externe"
    );
    guard_b.dismiss(); // idempotent
    assert_eq!(repo_b.export_json(), before_b, "Forcer ne fusionne pas");
    assert_eq!(store_b.save_count(), saves_b, "Forcer n'écrit pas");
}

#[test]
fn external_write_is_never_auto_merged() {
    let (repo_a, _store_a) = repo(42);
    repo_a
        .create_domain("Externe", "écrit par l'autre onglet")
        .expect("domaine distant");
    let before_a = repo_a.export_json();

    let (repo_b, store_b) = repo(43);
    repo_b
        .create_domain("Locale", "état local")
        .expect("domaine local");
    let before_b = repo_b.export_json();
    let saves_b = store_b.save_count();

    let (tab_a, tab_b) = linked_transports();
    let heard = inbox();
    listen(&tab_b, &heard);
    let mut guard = TabGuard::new(TAB_B.to_string());

    let expected_fingerprint = fingerprint_of(&repo_a);
    let message = write_message(TAB_A, NOW_MS, expected_fingerprint.clone());

    // Le message ne porte que de quoi détecter : pas de document, pas de
    // snapshot, pas de charge utile applicable.
    match &message {
        TabMessage::Write {
            tab,
            at_ms,
            fingerprint,
        } => {
            assert_eq!(tab, TAB_A);
            assert_eq!(*at_ms, NOW_MS);
            assert_eq!(fingerprint, &expected_fingerprint);
            assert_eq!(fingerprint.len(), 64, "empreinte SHA-256 hexadécimale");
            assert!(
                fingerprint
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
                "hexadécimal minuscule : {fingerprint}"
            );
            assert_eq!(
                fingerprint,
                &sha256_hex(before_a.as_bytes()),
                "l'empreinte couvre l'export du dépôt distant"
            );
        }
        other => panic!("un avis d'écriture est attendu, reçu {other:?}"),
    }

    assert!(tab_a.send(message.clone()).is_ok());
    let notice = latest_write(&heard).expect("avis reçu");
    assert!(guard.observe(&notice));

    // Détecter n'est pas appliquer : aucun objet distant n'entre dans le
    // Dataset local, aucune écriture silencieuse n'est déclenchée.
    let pending = guard.take_external().expect("avis externe");
    assert_eq!(pending.tab, TAB_A);
    assert_eq!(pending.at_ms, NOW_MS);
    assert_eq!(pending.fingerprint, expected_fingerprint);
    assert_eq!(repo_b.export_json(), before_b, "aucune fusion automatique");
    assert_eq!(
        store_b.save_count(),
        saves_b,
        "aucune écriture silencieuse dans le store"
    );
    assert!(
        !repo_b.export_json().contains("Externe"),
        "le domaine de l'autre onglet n'est pas importé"
    );
    assert!(
        repo_b
            .domains()
            .iter()
            .all(|domain| domain.name == "Locale"),
        "le Dataset local reste inchangé"
    );
    assert_eq!(
        repo_a.export_json(),
        before_a,
        "l'autre onglet reste intact"
    );
}

#[test]
fn snapshot_exchange_is_read_only() {
    let (repo_a, store_a) = repo(42);
    repo_a
        .create_domain("Paiements", "distant")
        .expect("domaine distant");
    let before_a = repo_a.export_json();
    let saves_a = store_a.save_count();

    let (repo_b, store_b) = repo(43);
    repo_b
        .create_domain("Commande", "local")
        .expect("domaine local");
    let before_b = repo_b.export_json();
    let saves_b = store_b.save_count();

    let (tab_a, tab_b) = linked_transports();
    let a_inbox = inbox();
    listen(&tab_a, &a_inbox);
    let b_inbox = inbox();
    listen(&tab_b, &b_inbox);

    let mut guard_a = TabGuard::new(TAB_A.to_string());
    let mut guard_b = TabGuard::new(TAB_B.to_string());

    // B demande le snapshot courant de A.
    assert!(tab_b.send(snapshot_request_message(TAB_B)).is_ok());
    assert!(snapshot_request_from(&a_inbox, TAB_B));
    assert!(!guard_a.observe(&snapshot_request_message(TAB_B)));
    assert!(guard_a.take_external().is_none());
    assert_eq!(repo_a.export_json(), before_a, "une demande ne lit rien");
    assert_eq!(store_a.save_count(), saves_a);

    // A répond par son snapshot ; B l'observe sans le traiter comme écriture.
    assert!(tab_a
        .send(snapshot_response_message(TAB_A, before_a.clone()))
        .is_ok());
    let remote_json = snapshot_response_from(&b_inbox, TAB_A).expect("réponse reçue");
    assert!(!guard_b.observe(&snapshot_response_message(TAB_A, remote_json.as_str())));
    assert!(
        guard_b.take_external().is_none(),
        "une réponse de snapshot n'est pas une écriture externe"
    );

    // La comparaison est pure : elle lit les deux JSON et n'applique rien.
    let local: Value = serde_json::from_str(&before_b).expect("snapshot local");
    let remote: Value = serde_json::from_str(&remote_json).expect("snapshot distant");
    assert!(!diff_values(&local, &remote).is_empty());

    assert_eq!(repo_a.export_json(), before_a);
    assert_eq!(repo_b.export_json(), before_b);
    assert_eq!(store_a.save_count(), saves_a);
    assert_eq!(store_b.save_count(), saves_b);
    let persisted_b = store_b.snapshot().expect("snapshot local persisté");
    assert_eq!(
        serde_json::from_str::<Value>(&persisted_b).expect("snapshot persisté lisible"),
        local,
        "le snapshot distant n'est jamais persisté"
    );
    assert!(
        !persisted_b.contains("Paiements"),
        "aucune donnée distante dans le stockage local"
    );
}

#[test]
fn snapshot_exchange_returns_current_json() {
    let (repo_a, _store_a) = repo(42);
    repo_a
        .create_domain("Avant", "premier état")
        .expect("premier domaine");
    let early = repo_a.export_json();

    // Écriture locale réussie après coup : la réponse doit refléter l'état
    // courant, jamais le snapshot initial.
    repo_a
        .create_domain("Après", "second état")
        .expect("second domaine");
    let current = repo_a.export_json();
    assert_ne!(early, current);

    let (tab_a, tab_b) = linked_transports();
    let a_inbox = inbox();
    listen(&tab_a, &a_inbox);
    let b_inbox = inbox();
    listen(&tab_b, &b_inbox);

    assert!(tab_b.send(snapshot_request_message(TAB_B)).is_ok());
    assert!(snapshot_request_from(&a_inbox, TAB_B));

    assert!(tab_a
        .send(snapshot_response_message(TAB_A, repo_a.export_json()))
        .is_ok());
    let remote_json = snapshot_response_from(&b_inbox, TAB_A).expect("réponse reçue");

    assert_eq!(
        remote_json, current,
        "la réponse est l'export courant, octet pour octet"
    );
    assert_ne!(remote_json, early, "pas de cache du premier snapshot");
    assert!(remote_json.contains("Après"));
    let decoded: Value = serde_json::from_str(&remote_json).expect("JSON valide");
    assert_eq!(
        decoded,
        serde_json::from_str::<Value>(&current).expect("export courant"),
        "le JSON est relisible tel quel"
    );
    assert_eq!(repo_a.export_json(), current, "l'échange n'écrit pas");
}

#[test]
fn tab_guard_dismisses_without_writing() {
    let (repo_b, store_b) = repo(43);
    repo_b
        .create_domain("Locale", "état local")
        .expect("domaine local");
    let before = repo_b.export_json();
    let saves = store_b.save_count();

    let mut guard = TabGuard::new(TAB_B.to_string());
    let external = write_message(TAB_A, NOW_MS, "external");
    assert!(guard.observe(&external));
    assert_eq!(guard.take_external().expect("avis").tab, TAB_A);

    // Nouvel avis puis Forcer.
    let newer = write_message(TAB_A, NOW_MS + 1, "newer");
    assert!(guard.observe(&newer));
    guard.dismiss();
    assert!(
        guard.take_external().is_none(),
        "dismiss ferme l'avis sans l'appliquer"
    );
    guard.dismiss();
    assert!(guard.take_external().is_none(), "dismiss est idempotent");

    assert_eq!(repo_b.export_json(), before, "aucune écriture");
    assert_eq!(store_b.save_count(), saves, "aucune persistance");
    assert_eq!(guard.local_id(), TAB_B, "l'identité locale est conservée");

    // Après dismiss, le guard reste utilisable pour un avis ultérieur.
    let later = write_message(TAB_A, NOW_MS + 2, "later");
    assert!(guard.observe(&later));
    assert_eq!(
        guard.take_external().expect("avis ultérieur").at_ms,
        NOW_MS + 2
    );
    assert_eq!(repo_b.export_json(), before);
}

#[test]
fn tab_transport_status_is_local_or_disabled() {
    let (tab_a, tab_b) = linked_transports();

    for status in [tab_a.status(), tab_b.status()] {
        assert!(
            matches!(status, TabStatus::Local | TabStatus::Disabled),
            "un transport local ne se déclare ni réseau ni connecté : {status:?}"
        );
        assert_ne!(
            status,
            TabStatus::Connected,
            "Connected est réservé à T7, aucun relais réseau au W1c-2"
        );
    }

    // Un canal mémoire ouvert est Local ; un navigateur sans canal est Disabled.
    assert_eq!(tab_a.status(), TabStatus::Local);
    assert_eq!(tab_b.status(), TabStatus::Local);
    let disabled = InProcessTabTransport::disabled();
    assert_eq!(disabled.status(), TabStatus::Disabled);
    assert!(!disabled.status().is_enabled());
    assert!(tab_a.status().is_enabled());

    // La fermeture (`pagehide`) ne concerne que l'onglet local.
    tab_a.close();
    assert_eq!(tab_a.status(), TabStatus::Disabled);
    assert_eq!(
        tab_b.status(),
        TabStatus::Local,
        "la fermeture d'un onglet ne ferme pas le canal des autres"
    );
    assert!(
        tab_b
            .send(write_message(TAB_B, NOW_MS, "after-close"))
            .is_ok(),
        "l'autre onglet peut toujours envoyer après la fermeture du premier"
    );

    // Les trois états du contrat restent distincts et copiables.
    let copied = tab_b.status();
    assert_eq!(copied, tab_b.status());
    assert_ne!(TabStatus::Local, TabStatus::Disabled);
    assert_ne!(TabStatus::Local, TabStatus::Connected);
}

#[test]
fn tab_id_is_never_persisted() {
    let (repo_a, store_a) = repo(42);
    let mut guard = TabGuard::new(TAB_A.to_string());
    let (tab_a, _tab_b) = linked_transports();

    // Une écriture locale réussie, puis publication comme le ferait `App::run`.
    repo_a
        .create_domain("Paiements", "sans identifiant d'onglet")
        .expect("domaine");
    let snapshot = repo_a.export_json();
    assert!(tab_a
        .send(write_message(
            TAB_A,
            NOW_MS,
            sha256_hex(snapshot.as_bytes())
        ))
        .is_ok());

    assert!(
        !snapshot.contains(TAB_A),
        "le TabId n'est pas dans l'export"
    );
    assert!(!snapshot.contains("tab_id"));
    assert!(!snapshot.contains("tabId"));

    let persisted = store_a
        .snapshot()
        .expect("le dépôt persiste son snapshot après écriture");
    assert!(!persisted.contains(TAB_A), "le TabId n'est pas persisté");
    assert!(!persisted.contains(TAB_B), "aucun identifiant d'onglet");
    assert!(!persisted.contains("tab_id"));
    assert!(!persisted.contains("tabId"));

    // Aucune clé d'onglet dans l'arbre JSON exporté.
    let value: Value = serde_json::from_str(&snapshot).expect("export JSON valide");
    let mut keys = Vec::new();
    collect_keys(&value, &mut keys);
    let forbidden = ["tab", "tabs", "tab_id", "tabid"];
    assert!(
        !keys.iter().any(|key| {
            let normalized = key.to_ascii_lowercase();
            forbidden.contains(&normalized.as_str())
        }),
        "aucune clé d'onglet dans le Dataset : {keys:?}"
    );

    // Les messages de l'autre onglet ne persistent rien non plus.
    let saves = store_a.save_count();
    let external = write_message(TAB_B, NOW_MS + 1, "external");
    assert!(guard.observe(&external));
    guard.dismiss();
    assert_eq!(store_a.save_count(), saves, "le canal n'écrit pas");
    assert_eq!(repo_a.export_json(), snapshot, "le Dataset est inchangé");
    assert!(
        !store_a.snapshot().expect("snapshot").contains(TAB_B),
        "aucun TabId distant persisté"
    );
}
