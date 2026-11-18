# Contrat W1c-2 — Multi-onglets, détection d'écriture externe

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | T1-08 |
| Statut | **Livré et vérifié** |
| Prérequis | W1a/W1b/W1c-1 verts |

## 1. Principe

Chaque onglet possède un `TabId` de session. Après une **écriture locale réussie**
(`App::run` renvoie `Ok`), l'onglet publie un avis sur `BroadcastChannel("archifier:repo:v1")`.
Les autres onglets détectent l'écriture et proposent **Recharger / Comparer / Forcer**.
Aucune fusion automatique, aucune écriture silencieuse.

Ce mécanisme prépare T7 (collaboration) mais ne le remplace pas : il n'y a ni CRDT, ni
transport de document, ni présence.

## 2. Types headless (`crates/repo/src/tabs.rs`)

```rust
pub type TabId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabMessage {
    /// Écriture locale réussie.
    Write { tab: TabId, at_ms: u64, fingerprint: String },
    /// Demande du snapshot courant pour comparaison.
    SnapshotRequest { tab: TabId },
    /// Réponse à une demande de comparaison.
    SnapshotResponse { tab: TabId, json: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalWrite {
    pub tab: TabId,
    pub at_ms: u64,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabStatus {
    Disabled,
    Local,
    Connected,
}

pub trait TabTransport {
    fn send(&self, message: TabMessage) -> Result<(), TabError>;
    fn subscribe(&self, on: Box<dyn Fn(TabMessage) + 'static>);
    fn status(&self) -> TabStatus;
}

pub struct InProcessTabTransport { /* canal mémoire, tests */ }
pub struct TabGuard { /* local: TabId, external: Option<ExternalWrite> */ }

impl TabGuard {
    pub fn new(local: TabId) -> Self;
    pub fn observe(&mut self, message: &TabMessage) -> bool; // true si externe retenu
    pub fn take_external(&mut self) -> Option<ExternalWrite>;
    pub fn dismiss(&mut self);
    pub fn local_id(&self) -> &str;
}
```

Règles :

- `observe` ignore les messages dont `tab == local`.
- Un `Write` externe remplace l'avis précédent (le plus récent gagne).
- `SnapshotRequest`/`SnapshotResponse` ne sont pas des écritures et ne modifient pas
  l'état du dépôt.
- `TabGuard` ne persiste rien et ne modifie jamais le `Dataset`.

## 3. Transport navigateur

- `LocalTabTransport` : `BroadcastChannel("archifier:repo:v1")`, messages JSON UTF-8.
- `status()` renvoie `Local` si le canal est ouvert, `Disabled` si `BroadcastChannel` est
  indisponible.
- Aucun serveur, aucune requête réseau.
- `TabId` généré par session (`crypto.getRandomValues` si disponible, repli
  `Date.now + performance.now + compteur`), jamais persisté, jamais exporté.
- Écoute installée au démarrage ; le canal est fermé proprement sur `pagehide` si possible.

## 4. Publication des écritures

- Point unique : `App::run` (`crates/ui/src/state.rs`) après `Ok`.
- Message `Write { tab, at_ms, fingerprint }` avec
  `fingerprint = archifier_core::sha256_hex(repo().export_json())`.
- Aucune publication pour une lecture, un `Err`, un toast ou un rafraîchissement.
- Le message ne contient **pas** le snapshot complet ; il ne sert qu'à détecter.

## 5. Dialogue

Lorsqu'un `ExternalWrite` est reçu, l'UI affiche un bandeau/dialogue global :

| Action | Comportement |
|---|---|
| **Recharger** | `window.location.reload()` ; aucun état local conservé. |
| **Comparer** | envoie `SnapshotRequest` au pair ; à réception, calcule `diff_values` entre le snapshot local et le snapshot distant ; affiche les changements (chemin, avant, après). Aucune application. |
| **Forcer** | ferme l'avis et continue avec l'état local. Aucune fusion. |

- Si aucun `SnapshotResponse` n'arrive (pair fermé), afficher « pair indisponible ».
- Le dialogue ne s'ouvre jamais pour sa propre écriture.
- Aucune écriture n'est déclenchée par le dialogue.
- Le snapshot distant n'est jamais persisté ni exporté.

## 6. Tests bloquants

- `external_write_is_detected`
- `external_write_ignores_own_tab`
- `external_write_offers_reload_compare_force`
- `external_write_is_never_auto_merged`
- `snapshot_exchange_is_read_only`
- `snapshot_exchange_returns_current_json`
- `tab_guard_dismisses_without_writing`
- `tab_transport_status_is_local_or_disabled`
- `tab_id_is_never_persisted` (export/JSON ne contient pas le TabId)

## 7. Périmètre fichiers

- `crates/repo/src/tabs.rs` (nouveau)
- `crates/repo/src/lib.rs` (export du module uniquement)
- `crates/ui/src/tabs.rs` (nouveau, `LocalTabTransport`)
- `crates/ui/src/lib.rs` (`pub mod tabs;`)
- `crates/ui/src/state.rs` (`App::run`, état `external_write`)
- `crates/ui/src/components.rs` (dialogue)
- `crates/ui/src/boot.rs` (installation/fermeture)
- `crates/ui/src/views/settings.rs` (publication après import/reset)
- `crates/ui/src/views/domain.rs` (publication après import OpenAPI)
- `crates/ui/Cargo.toml` (features `BroadcastChannel`, `MessageEvent`, déjà présentes)
- `crates/repo/tests/tabs_acceptance.rs` (testeur)
- `tests/e2e/tabs.spec.ts` (W7)

## 8. Hors périmètre

- Fusion automatique, CRDT, présence, relais réseau.
- Synchronisation entre profils de navigateur ou machines.
- Résolution de conflit de gouvernance : le verrou optimiste reste la règle.

## 9. Décisions pragmatiques T1-08

T1-08 est une **détection**, pas un protocole de fusion. Les limites suivantes sont assumées et testées comme telles :

1. **Publication après persistance** : `App::run` ne publie que si `repo().persist_error().is_none()`. Une erreur asynchrone d'IndexedDB peut encore survenir après le `put` ; le bandeau de persistance reste la source de vérité. Test `no_publish_when_persistence_fails` avec `FailingStore`.
2. **Dernier écrivain gagnant** : le snapshot unique n'a pas de `If-Match`. T1-08 détecte et propose recharger/comparer/forcer ; il ne verrouille pas. Le verrou de snapshot est un sujet T7/W3. Aucune fusion, aucune écriture déclenchée par le dialogue.
3. **Snapshot de comparaison** : `SnapshotResponse` transporte l'export JSON courant pour le MVP local. Limite pratique : 8 Mio ; au-delà, l'UI affiche « état trop volumineux pour une comparaison détaillée » et propose recharger. Un seul pair répond en pratique (deux onglets) ; `tab` identifie l'émetteur.
4. **Cycle de vie best-effort** : fermeture sur `pagehide` si `persisted == false` ; sur `pageshow`/`visibilitychange`, l'onglet se réabonne et compare son empreinte à la dernière connue. Un message manqué en arrière-plan n'est pas garanti.
5. **TabId** : 128 bits si possible, sinon combinaison temps + aléatoire ; session = chargement de page ; jamais persisté, jamais exporté, jamais utilisé comme acteur.
6. **Frontière de confiance** : tout script de la même origine peut émettre un faux `Write` ; aucun message n'est authentifié. Un faux message ne peut que provoquer un dialogue, jamais une écriture.
7. **Point de publication** : `App::run` après `Ok`, plus import/reset/seed qui court-circuitent `run`. Les vues appellent `app.publish_write()` après ces opérations.

Ces décisions sont des écarts assumés à `technical-specifications-lot1-front-local.md` §3.4 (canal `archifier:changes`) : W1c-2 utilise `archifier:repo:v1` et un avis d'empreinte.

