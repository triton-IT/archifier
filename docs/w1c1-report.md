# Rapport de fin de sous-lot — W1c-1 (hiérarchie, Story, revues)

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | T1-09, T1-10, T1-11 |
| Statut | **Livré et vérifié** |
| Contrat | `docs/w1c-contract.md` §7 |

## 1. Commandes et résultats

| Commande | Résultat |
|---|---|
| `cargo test --workspace` | ✅ **230 tests**, 0 échec |
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| `cargo check -p archifier-ui --target wasm32-unknown-unknown` | ✅ |
| `.\scripts\build.cmd` | ✅ bundle **2,71 Mo** |

## 2. Livrables moteur

- `crates/repo/src/hierarchy.rs` : `hierarchy`, `set_parent`, profondeur 5 arêtes, `HIER-001..005`, parent archivé.
- `crates/repo/src/review_compare.rs` : `ReviewComparison`, `TemporalWindow`, filtres, anomalies par `(code, object_id, detail)`.
- `crates/repo/src/lib.rs` : `pin_story_revisions`, `unpin_story_revisions`, `save_story` validé avant mutation, `Playback.pinned_revisions/pinned_versions`.
- `crates/core/src/story.rs` : `apply_framing`, zoom, focus, opacité.
- `crates/core/src/model.rs` : `StoryStructure.pinned_versions` additif.

## 3. Livrables UI

- Fiche élément : carte Hiérarchie (parent, ancêtres, enfants, badge parent archivé), sélecteur de parent, détacher.
- Story : cadrage zoom/focus/atténuation, légende, épingler/dégeler, badge « Récit figé ».
- Revue : onglet Comparaison, sélecteur d’une autre revue, fenêtres/filtres, exposées/consommées ajoutées/retirées/modifiées, anomalies.

## 4. Tests

- `hierarchy_acceptance` : 9 tests.
- `story_acceptance` : 9 tests.
- `review_compare_acceptance` : 4 tests.
- Total workspace : 230.

## 5. Écarts assumés

- Application des filtres de revue reportée W5 ; W1c compare les instantanés et l’appartenance calculée à `now`.
- `exposed_changed`/`consumed_changed` peuvent être vides tant que les filtres ne sont pas appliqués.
- Le sélecteur de parent liste la même notation ; `Grouping`/`Native` sont refusés par `HIER-002`.
- L’état de comparaison de revues est local au composant, réinitialisé au re-rendu.
- La date du récit figé dépend de l’audit ; si absent, seul le nombre d’épingles est affiché.

## 6. Prochaine étape

W1c-2 : multi-onglets (T1-08), puis G2.
