# Plan de test W1c-2 — multi-onglets et écriture externe

| | |
|---|---|
| Date | 18 novembre 2026 |
| Périmètre | T1-08 (détection d'écriture externe, dialogue recharger / comparer / forcer) |
| Contrat de référence | `docs/w1c2-contract.md` §2 à §6 |
| Suite | `crates/repo/tests/tabs_acceptance.rs` |
| Fixtures | Aucune : dépôts construits en mémoire (`Repository::new` + `create_domain`), horloge figée |
| État | 9 tests bloquants écrits, alignés sur l'API livrée de `crates/repo/src/tabs.rs`. Aucun `cargo` lancé. Aucun test existant ni code de production modifié. |

## Principes

- Les tests n'utilisent que l'API publique du contrat : `TabGuard`, `TabMessage`,
  `ExternalWrite`, `TabStatus`, `TabTransport`, `InProcessTabTransport`,
  `Repository::export_json`, `MemoryStore::snapshot` / `save_count` et
  `archifier_core::diff_values` / `sha256_hex`.
- Chaque dépôt est reconstruit par test avec une horloge figée
  (`NOW_MS = 1_795_000_000_000`, 18 novembre 2026) et un `MemoryStore` neuf.
- Les identifiants d'onglet sont volontairement distinctifs
  (`tab-…T0A`, `tab-…T0B`) pour prouver qu'ils n'entrent ni dans l'export ni
  dans le snapshot persisté.
- Aucun test ne dépend du texte d'une erreur : `send` est vérifié par `is_ok()`
  et aucun test ne nomme `TabError`.
- L'absence de mutation est vérifiée par `export_json()` **et**, quand une
  écriture serait possible, par `MemoryStore::save_count()`.

## Cas imposés

| Cas | Couverture |
|---|---|
| Deux transports en mémoire | tests 1, 3, 4, 5, 6, 8, 9 via `linked_transports()` |
| Messages de snapshot | tests 2, 3, 5, 6 (`SnapshotRequest`, `SnapshotResponse`) |
| Absence de mutation du `Dataset` | tests 3, 4, 5, 6, 7, 9 (`export_json` et `save_count` avant/après) |
| Export JSON sans `TabId` | test 9 (valeur brute, snapshot persisté, clés de l'arbre JSON) |

## Tests bloquants du §6

| # | Test | Transport | Précondition | Attendu |
|---|---|---|---|---|
| 1 | `external_write_is_detected` | deux transports reliés | guard local `TAB_B` ; A émet `Write { tab: TAB_A, at_ms, fingerprint }` | B reçoit exactement l'avis ; `observe` rend `true` ; `take_external` rend les trois champs ; second `take` rend `None` ; un second avis remplace le premier (« le plus récent gagne ») |
| 2 | `external_write_ignores_own_tab` | aucun (guard seul) | guard local `TAB_A` | `Write`, `SnapshotRequest`, `SnapshotResponse` de `TAB_A` rendent `observe = false` et ne créent aucun avis ; un avis externe déjà retenu survit à une écriture locale ultérieure |
| 3 | `external_write_offers_reload_compare_force` | deux transports reliés | deux dépôts distincts (`Paiements` distant, `Commande` local) ; avis externe observé par B | **Comparer** : B émet `SnapshotRequest`, A répond `SnapshotResponse`, `diff_values` montre le changement `/domains/0/object/name` `Commande → Paiements` sans rien appliquer ; **Recharger** : consommer l'avis ne modifie ni `export_json` ni le store ; **Forcer** : `dismiss` ferme l'avis, idempotent, sans fusion ni écriture ; A n'écrit jamais |
| 4 | `external_write_is_never_auto_merged` | deux transports reliés | A a écrit `Externe`, B détient `Locale` ; avis externe avec empreinte SHA-256 de l'export A | le message `Write` ne porte que `tab`, `at_ms`, `fingerprint` (64 hex minuscules, égal à `sha256_hex(export A)`) ; après réception, l'export de B, ses domaines et son `save_count` sont inchangés ; `Externe` n'entre jamais dans B ; A reste intact |
| 5 | `snapshot_exchange_is_read_only` | deux transports reliés | A = `Paiements`, B = `Commande` ; demande de B puis réponse de A | ni la demande ni la réponse ne sont des écritures (`observe = false`, aucun `ExternalWrite`) ; les deux `export_json`, les deux `save_count` et le snapshot persisté de B sont inchangés ; le JSON distant n'est jamais persisté (le store de B ne contient pas `Paiements`) ; le diff est calculé sans application |
| 6 | `snapshot_exchange_returns_current_json` | deux transports reliés | A écrit `Avant`, capture `early`, écrit `Après`, capture `current` | la réponse est **octet pour octet** l'export courant (`current`), différente de `early`, contient `Après`, se relit en JSON ; l'échange ne modifie pas A (pas de cache du premier snapshot) |
| 7 | `tab_guard_dismisses_without_writing` | aucun (guard seul) | avis externe retenu puis second avis | `dismiss` vide `take_external`, un second `dismiss` est sans effet, `export_json` et `save_count` sont inchangés, `local_id` est conservé ; un avis ultérieur est encore retenu |
| 8 | `tab_transport_status_is_local_or_disabled` | deux transports reliés | canal mémoire ouvert, `disabled()`, puis `close()` | chaque `status()` vaut `Local` ou `Disabled`, jamais `Connected` (aucun réseau au W1c-2) ; un canal ouvert vaut `Local`, `disabled()` et `close()` valent `Disabled`, y compris pour le clone du canal ; les trois états du contrat sont distincts et copiables |
| 9 | `tab_id_is_never_persisted` | deux transports reliés | écriture locale réussie (`create_domain`), puis `Write` portant `TAB_A` | ni l'export, ni le snapshot `MemoryStore`, ni les clés de l'arbre JSON ne contiennent `TAB_A` ou une clé `tab`/`tabs`/`tab_id`/`tabId` ; l'avis externe `TAB_B` puis `dismiss` n'écrivent rien et ne persistent aucun identifiant |

## Vérifications transverses

1. **Canaux isolés** : chaque test crée ses propres transports ; aucun état
   global partagé entre tests.
2. **Détection sans charge utile** : `TabMessage::Write` ne contient pas de
   snapshot complet ; le test 4 le prouve par déstructuration et par l'empreinte.
3. **Aucune fusion** : observer, consommer, comparer ou forcer ne modifie jamais
   `export_json()` du dépôt local.
4. **Aucune écriture silencieuse** : `MemoryStore::save_count()` reste constant
   pendant tous les échanges de messages et toutes les actions du dialogue.
5. **Snapshot vivant** : la réponse de comparaison est recalculée à la demande
   (`export_json()` au moment de la réponse), pas mise en cache.
6. **Identité locale** : le guard ignore strictement son propre `TabId`, y
   compris pour les messages de snapshot.
7. **`TabId` hors du document** : l'identifiant de session reste un état de
   transport ; le test 9 couvre l'export, la persistance et les clés du Dataset.

## Hypothèses d'API et points incertains

1. **Construction du transport mémoire.** Les tests utilisent `new()`, `pair()`,
   `disabled()` et `close()`. `pair()` crée deux endpoints distincts sur un canal
   partagé ; `clone()` reste un handle du même endpoint. `send` ne délivre pas à
   l'émetteur ; les tests s'abonnent sur le récepteur. `close()` ne ferme que
   l'endpoint local.
2. **Livraison synchrone.** Les tests lisent la boîte de réception juste après
   `send`. L'implémentation actuelle diffuse de façon synchrone ; si elle
   devenait asynchrone, ajouter une attente bornée dans `listen` / `messages`,
   sans changer les assertions.
3. **`status()` du transport navigateur.** Le test 8 couvre le transport
   mémoire (`Local`, puis `Disabled` après `close()`, et `disabled()`), jamais
   `Connected`. Le `LocalTabTransport` wasm (`crates/ui/src/tabs.rs`) reste à
   couvrir côté UI / e2e : le contrat lui impose `Local` ou `Disabled`.
4. **Dialogue sans interface.** Aucun type d'action (`Reload` / `Compare` /
   `Force`) n'est défini au §2. Le test 3 représente les trois choix par leurs
   effets headless : consommation de l'avis, échange de snapshots + diff,
   `dismiss`. Le bandeau et ses libellés restent couverts par
   `tests/e2e/tabs.spec.ts` (W7).
5. **Adressage des snapshots.** `SnapshotRequest` / `SnapshotResponse` ne
   portent qu'un `tab`. Les tests l'interprètent comme l'émetteur et filtrent la
   réponse par `tab == TAB_A`. Le répondeur est le transport récepteur ; aucun
   écho n'est attendu.
6. **Abonnements.** `subscribe` accumule les écouteurs (`subscriber_count`).
   Les tests installent un abonné par transport ; ils ne dépendent pas de
   l'ordre de notification.
7. **Empreinte.** Le test 4 vérifie `fingerprint = sha256_hex(export_json())`
   et le format 64 hex minuscules. La canonicalisation exacte utilisée par
   `App::run` (compacte ou pretty) n'est pas observable en headless : le test
   vérifie la cohérence avec `Repository::export_json`, pas le chemin UI.
8. **Sérialisation persistée.** `MemoryStore` reçoit le JSON compact de
   `Repository::mutate`, alors que `export_json()` est pretty. Les comparaisons
   de contenu persisté passent par `serde_json::from_str::<Value>`, jamais par
   égalité de chaînes entre les deux formats.
9. **Chemin du module.** Les tests importent `archifier_repo::tabs::*`,
   conformément au §7 (`lib.rs`, export du module uniquement). Si le module est
   ré-exporté à la racine, l'import peut être simplifié.

## Exécution prévue (après le dev moteur)

```powershell
cargo test -p archifier-repo --test tabs_acceptance
```

Aucun test existant n'est modifié : `acceptance.rs`, `storage_boundary.rs`,
`openapi_acceptance.rs`, `history_diff_acceptance.rs`,
`markdown_export_acceptance.rs`, `diagnostics_acceptance.rs`,
`hierarchy_acceptance.rs`, `story_acceptance.rs` et
`review_compare_acceptance.rs` restent inchangés.
