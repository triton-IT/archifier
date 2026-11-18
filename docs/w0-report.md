# Rapport de fin de lot — W0 (audit, contrats, spike CRDT)

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | W0 : audit, documents, ADR, spike T7-00, remise à niveau des portes |
| Porte | **G1 franchie** |
| Agents | Orchestrateur, analyste/architecte, dev moteur, relecteur fonctionnel, relecteur qualité |

## 1. Commandes exécutées et résultats

| Commande | Résultat |
|---|---|
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo test --workspace` | ✅ **83 tests**, 0 échec |
| `cargo check -p archifier-ui --target wasm32-unknown-unknown` | ✅ exit 0 |
| `.\scripts\build.cmd` | ✅ bundle **1,95 Mo** de wasm |
| `powershell -File .\scripts\spike-bundle.ps1` | ✅ delta Yrs **296,9 Kio** / **332,3 Kio** < 900 Kio |
| `cargo test -p archifier-collab` | ✅ **6 tests** |
| Sauvegarde pré-formatage | `archifier-backup-20261118-141823.zip` |

## 2. Livrables

### Documents

- `docs/backlog-mvp2.md` **v0.3** : 41 points, contrats C-01 → C-10 figés, décisions O-01 → O-07, découpage W0 → W7, portes G1 → G7, historique.
- `technical-specifications-lot2.md` **v0.3** : schéma CRDT par champ, types auxiliaires, `CollabEdit`, `SyncTransport`, présence/wire, stores v2, projection/historique, fusion, OpenAPI/AsyncAPI, cycle de vie, migration, IA, port de dépôt, résultats du spike.
- `docs/adr/ADR-011-snapshot-unique-indexeddb.md` (rétro-ADR).
- `docs/adr/ADR-012-routage-par-fragment.md` (rétro-ADR).
- `docs/adr/ADR-013-pas-ia-generative-navigateur.md` (rétro-ADR).
- `docs/adr/ADR-015-transport-synchronisation.md` (accepté).
- `docs/adr/ADR-016-port-depot-type-repository.md` (accepté).
- `docs/adr/ADR-017-proxy-llm-local.md` (proposé W6).
- `docs/adr/ADR-018-contrats-externes-openapi-asyncapi.md` (proposé W1/W5).
- `docs/adr/ADR-019-politique-cycle-de-vie-interfaces.md` (proposé W5).
- `docs/adr/ADR-020-virtualisation-canvas.md` (proposé W7).
- `docs/adr/ADR-021-migration-metamodele-archimate-4.md` (proposé W2).
- `docs/metamodel-coverage.md` régénéré en UTF-8.
- `README.md` : build via `scripts/build.cmd`, 83 tests, frontière UI corrigée.
- `backlog-mvp1.md` : synthèse corrigée (partiels réels), baseline de tests.
- `technical-specifications.md` : amendements ADR-016, ADR-021, §21.4 aligné sur C-03, journal ADR complété.
- `technical-specifications-lot1-front-local.md` : amendement ADR-016 au §2.

### Code et outillage

- `crates/collab` : crate headless Yrs 0.25, `TextDocument` UTF-16, erreurs typées.
- `crates/collab/tests/spike.rs` : 6 tests (convergence, taille de frappe, décodage, UTF-16, frontière de surrogate, erreur typée).
- `crates/collab/examples/spike_{baseline,wasm,wasm_schema}.rs` : mesure du delta WASM.
- `scripts/spike-bundle.ps1` : assertion automatisée du budget bundle.
- `scripts/build.cmd` : wrapper reproductible de `build.ps1`.
- `crates/repo/src/storage.rs` : `SnapshotStore::reset`.
- `crates/repo/src/idb.rs` : reset IndexedDB via le port.
- `crates/repo/src/lib.rs` : `reset_storage`, import persistant, plus de `panic!` sur les profils.
- `crates/ui/src/state.rs` : repli anti-panic documenté, `request_reseed`.
- `crates/ui/src/views/settings.rs` : reset via `repo().reset_storage()`, plus d'accès IndexedDB direct.
- `crates/ui/tests/architecture.rs` : 3 tests d'architecture.
- `crates/repo/tests/storage_boundary.rs` : import persistant + reset.

## 3. Spike T7-00 — mesures

| Porte | Seuil | Mesure | Verdict |
|---|---|---|---|
| 20 000 caractères + 200 insertions concurrentes | aucune perte | 20 200 caractères, 200 marqueurs uniques, réplicas identiques | ✅ |
| Taille d'une frappe | < 2 Kio | 17 octets (mot + emoji : 21) | ✅ |
| Décodage + application 20 k | < 30 ms | release 27,3 µs / 32,6 µs / 87,9 µs | ✅ |
| Delta WASM brut | < +900 Kio | `Y.Text` +296,9 Kio ; schéma C-01 +332,3 Kio | ✅ |

**Yrs 0.25.0** est épinglé : 0.27.4 et 0.28.1 échouent en E0658 (`if let` guards instables
sur Rust 1.94). Aucun repli Automerge.

## 4. Corrections issues des relectures

| Point | Correction |
|---|---|
| Port décrit comme trait | ADR-016 + amendements §21.1 / addendum lot 1 ; tests d'architecture |
| Scénario « relations incompatibles » | lecture gouvernance : seconde écriture refusée, aucune suppression ; `MERGE_VIOLATION` réservé aux vues fusionnées |
| 9 tests non rattachés | rattachés à T7-02 / T7-03 / T7-04 / T7-06 |
| Types CRDT manquants | `CollabSeed`, `CollabChange`, `CollabSnapshot`, `MergeOutcome`, `CollabEvent`, `ProjectionOutcome`, `DocKind`, `PresenceField`, `AwarenessState` |
| `StoryChapterRemove` non gardé | `COLLAB-NON-EMPTY` |
| Présence non sérialisée | `AwarenessState` + JSON, `left`, test d'encodage |
| Canal divergent | `archifier:collab:v1` seule constante |
| Position de nœud hybride | `position` atomique LWW, pas deux clés |
| Hors ligne mal défini | partition du transport, une rafale par période hors ligne |
| Projection vs écriture externe | origine `COLLAB_PROJECTION`, pas de dialogue pour la même session |
| Import non persisté | `load_json` sauvegarde, test dédié |
| Reset IndexedDB dans une vue | `Repository::reset_storage` + `request_reseed` |
| Repli dans `state.rs` | exception anti-panic documentée dans ADR-016 |
| Tests d'architecture annoncés absents | 3 tests ajoutés |
| ADR-011/012/013 fantômes | rétro-ADR créés |
| Rapport de couverture mal encodé | régénéré en UTF-8 |
| `serde_yaml_ng` / `sha2` non déclarés | ajoutés au workspace et à `core` |

## 5. Points différés (W1+)

- Route `Relationship` et fiche relation (lien de recherche actuellement cassé).
- Éditeur de Story (H-01/H-05), vues de revue entrantes/sortantes (I-02).
- Sélecteur de date T (J-04), filtres d'interface (E-09), filtres de recherche (K-02).
- Archivage de domaine couvrant relations, diagrammes, stories et revues (B-02).
- Relation par tirage (F-02) ou acceptation explicite du picker.
- `RawRule` : `allow_any` / `allow_cross_layer_downward` silencieusement ignorés (W2).
- Playwright E2E, budgets 500 nœuds, accessibilité, centralisation des chaînes (W7).
- Mesure du delta réel sur `pkg/archifier_ui_bg.wasm` au câblage Yrs (W4).
- `localStorage` reste utilisé pour des marqueurs non métier (`last_seen`, `force_reseed`) ; les données restent dans IndexedDB.

## 6. État des portes

- **G1 : franchie.** Contrats C-01 → C-10 figés, spike T7-00 vert, ADR matérialisés, dépendances déclarées, tests d'architecture présents.
- **G2 : non démarrée.** W1 (T1-01 → T1-11) reste à exécuter.
- Le bundle UI n'inclut pas encore Yrs : la mesure de +900 Kio est portée par les exemples WASM du crate `collab`, pas par `pkg/archifier_ui_bg.wasm` (mesure à refaire en W4).
