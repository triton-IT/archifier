# Rapport de vague — W1 (dette du lot 1)

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | T1-01 → T1-11 |
| Porte | **G2 franchie** |
| Tests | **253 tests**, 0 échec |
| Bundle | **2,76 Mo** |

## Sous-lots

| Sous-lot | Points | Livrables | Rapport |
|---|---|---|---|
| W1a | T1-01/02/03 | OpenAPI import/export/diff, 22 tests, UI | `docs/w1a-report.md` |
| W1b | T1-04/05/06/07 | Diff de révisions, export Markdown, liens invalides, graphe, 20 tests, UI | `docs/w1b-report.md` |
| W1c-1 | T1-09/10/11 | Hiérarchie, cadrage Story, épinglage, comparaison de revues, 22 tests, UI | `docs/w1c1-report.md` |
| W1c-2 | T1-08 | Multi-onglets, 9 tests + 14 unitaires, UI | `docs/w1c2-report.md` |

## Portes vérifiées

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p archifier-ui --target wasm32-unknown-unknown
.\scripts\build.cmd
```

Toutes vertes au 18/11/2026.

## Écarts et limites connues

- W1a : preview OpenAPI en mémoire, export JSON (pas YAML), snapshot normalisé lossy mais brut conservé, `export_openapi_raw` et JCS reportés.
- W1b : export revue/Story reporté W1c/W5 ; export multi-fichiers sans zip ; direction de graphe par défaut `BOTH`.
- W1c-1 : application des filtres de revue reportée W5 ; `exposed_changed`/`consumed_changed` peuvent être vides.
- W1c-2 : dernier écrivain gagnant, aucun verrou de snapshot ; comparaison plafonnée 8 Mio ; cycle de vie best-effort.
- Les écarts MVP1 corrigés (P-01 import persistant, reset via le port, route Relation) sont notés dans `backlog-mvp1.md`.

## Suite

W2 (notations T3) puis W3/W4 (collaboration T7). G2 est franchie, aucune dette bloquante W1 ne reste ouverte.
