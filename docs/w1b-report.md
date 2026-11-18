# Rapport de fin de sous-lot — W1b (diff, export, liens, graphe)

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | T1-04, T1-05, T1-06, T1-07 |
| Statut | **Livré et vérifié** |
| Contrat | `docs/w1b-contract.md` §5 |

## 1. Commandes et résultats

| Commande | Résultat |
|---|---|
| `cargo test --workspace` | ✅ **186 tests**, 0 échec |
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| `cargo check -p archifier-ui --target wasm32-unknown-unknown` | ✅ |
| `.\scripts\build.cmd` | ✅ bundle **2,57 Mo** |

## 2. Livrables moteur

- `crates/core/src/history_diff.rs` : `FieldDifference`, diff JSON RFC 6901.
- `crates/core/src/export.rs` : `ExportBundle`, export domaine/objet, SVG + Mermaid, liens relatifs, pages stub.
- `crates/repo/src/diagnostics.rs` : `BrokenLink`, statut recalculé, contexte ±80, dédoublonnage.
- `crates/repo/src/export.rs` : wrappers dépôt.
- `crates/repo/src/lib.rs` : `revision_diff`, `comparable_revisions`, graphe enrichi, `stats().broken_links` corrigé.
- `crates/repo/src/types.rs` : `RevisionDiff`, `BrokenLink`, `DependencyNode.direction/lifecycle`.

## 3. Livrables UI

- **Historique des révisions** sur élément et interface : deux sélecteurs, `revision_diff`, tableau chemin/avant/après.
- **Export Markdown** domaine et objet : téléchargement du bundle, compteur de fichiers.
- **Liens invalides** dans Réglages : `DOC-LINK-001/002`, source, cible, contexte, propriétaire.
- **Graphe** `#/graph/<id>` : direction IN/OUT/BOTH, profondeur 1–6, chemins, relation, lifecycle, badge tronqué.
- `Route::Graph` + round-trip routeur.

## 4. Tests

- `history_diff_acceptance.rs` : 5 tests.
- `markdown_export_acceptance.rs` : 7 tests.
- `diagnostics_acceptance.rs` : 7 tests.
- `graph_route_parses` côté routeur.
- Total workspace : 186 tests.

## 5. Écarts assumés

- Export revue/Story reporté en W1c (FR-019 partiel).
- Téléchargement multi-fichiers sans zip : certains navigateurs peuvent filtrer les téléchargements automatiques.
- Direction par défaut du graphe : `BOTH`.
- `EXPORT-002` couvre `w/h ≤ 0` (les NaN ne passent pas par JSON).
- Pages `relationships/` liées depuis la documentation, pas listées dans l'index.
- `revision_diff` exclut `/object/revision` et `/object/updated_at`, et défensivement `/documentation*`.

## 6. Prochaine étape

W1c : T1-08 multi-onglets, T1-09 hiérarchie, T1-10 cadrage Story + épinglage, T1-11 comparaison de revues, puis G2.
