# Rapport de fin de sous-lot — W1a (OpenAPI)

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | T1-01, T1-02, T1-03 |
| Statut | **Livré et vérifié** |
| Contrat | `docs/w1a-openapi-contract.md` §10 |

## 1. Commandes et résultats

| Commande | Résultat |
|---|---|
| `cargo test --workspace` | ✅ **139 tests**, 0 échec |
| `cargo test -p archifier-repo --test openapi_acceptance` | ✅ **22/22** |
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| `cargo check -p archifier-ui --target wasm32-unknown-unknown` | ✅ |
| `.\scripts\build.cmd` | ✅ bundle **2,29 Mo** de wasm |

## 2. Livrables moteur

- `crates/core/src/openapi.rs` : parse JSON/YAML 3.0/3.1/3.2, empreinte canonique SHA-256, snapshot DTO, SemVer, extensions, limite 2 Mio/BOM.
- `crates/core/src/contract_diff.rs` : diff OpenAPI, codes `OAS-010…016`, `020/021`, `030/031/032`, classification `Breaking > Unknown > Compatible`.
- `crates/repo/src/openapi.rs` : preview opposable, apply recalculé, export JSON canonique, diff de deux versions.
- `crates/core/src/model.rs` : `contract_format`, `contract_raw`, `contract_hash` additifs sur `InterfaceVersion`.
- `crates/repo/src/lib.rs` : index d'idempotence `(ContractFormat, hash)`, intégration d'un seul `mutate`.
- `crates/repo/src/types.rs` : `OpenApiImportStatus`, `OpenApiImportPreview`, `OpenApiImportOutcome`.

## 3. Livrables UI

- Page domaine : import fichier/texte, **Analyser**, aperçu, diff, warnings, confirmation de rupture décochée, **Appliquer**.
- Fiche interface : **Exporter OpenAPI** (application + contrat OpenAPI uniquement), téléchargement JSON, **Comparer deux versions**.
- Aucun accès direct au stockage ; `repo()` uniquement.
- Erreurs `OAS-*` affichées avec leur code.

## 4. Fixtures et tests

- 14 fixtures dans `fixtures/mvp2/openapi/**` (3.0 YAML, 3.1 JSON/YAML, 3.2 YAML, idempotence byte-différente, rupture, extensions, operationId dupliqué/absent, SemVer dupliquée, paramètre retiré, operationId renommé, serveur modifié, sans opération, invalide).
- 22 tests d'acceptation dans `crates/repo/tests/openapi_acceptance.rs`.
- Plan de test : `docs/w1a-test-plan.md`.

## 5. Décisions et écarts assumés

- Preview **purement en mémoire** (dérogation à `technical-specifications.md` §11.1) ; pas d'`openapi_documents`, pas de `job_id`, pas de ChangeSet.
- Export **JSON UTF-8 canonique**, pas YAML.
- Les `x-architecture-*` sont exclus récursivement de l'empreinte.
- `contract_raw` conserve le document brut ; le snapshot normalisé est volontairement lossy (descriptions d'opérations/réponses, `x-*` imbriqués non conservés).
- `export_openapi_raw` et JCS RFC 8785 complet sont reportés (W1b/W5).
- Les clés JSON dupliquées restent acceptées en « dernière valeur gagne » par `serde_json` ; YAML les refuse.
- `$ref` internes hors `components.schemas` non signalés `OAS-030`.
- `OAS-WARN-002` (export lossy) non émis.
- Import fichier non couvert par un E2E DOM dans ce lot.

## 6. Points pour W1b/W1c

- Ajouter un test E2E du panneau d'import (fichier + texte).
- Sélecteur de version à l'export si nécessaire.
- Mesurer l'impact du bundle : 1,95 → 2,29 Mo après parsing OpenAPI, toujours sous le budget de 4 Mo.
