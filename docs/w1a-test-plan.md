# Plan de test W1a — import / export OpenAPI

| | |
|---|---|
| Date | 18 novembre 2026 (deuxième passe) |
| Périmètre | T1-01, T1-02, T1-03 |
| Contrat de référence | `docs/w1a-openapi-contract.md` §8, §9, §10 |
| Suite | `crates/repo/tests/openapi_acceptance.rs` |
| Fixtures | `fixtures/mvp2/openapi/**` |

Les tests n'utilisent que l'API publique : `parse_openapi`,
`Repository::import_openapi_preview`, `Repository::apply_openapi_import`,
`Repository::export_openapi` et `Repository::diff_openapi_versions`. Aucun accès
direct au stockage, aucun test d'UI.

## Fixtures

Toutes les variantes JSON sont dérivées mécaniquement de `petstore-3.1.json` ; un
script de vérification confirme que chaque fichier ne diffère de la base que par le
changement annoncé.

| Fichier | Rôle | Particularités |
|---|---|---|
| `petstore-3.0.yaml` | famille 3.0 | `openapi: 3.0.3`, YAML, titre `Petstore 3.0` |
| `petstore-3.1.json` | famille 3.1, document de référence | `openapi: 3.1.0`, JSON, titre `Petstore 3.1`, 4 opérations, 1 serveur |
| `petstore-3.1.yaml` | canonicalisation inter-formats | même document que `petstore-3.1.json`, syntaxe YAML |
| `petstore-3.2.yaml` | famille 3.2 | `openapi: 3.2.1`, YAML, titre `Petstore 3.2` |
| `petstore-3.1-unchanged.json` | idempotence par empreinte | sémantiquement identique à `petstore-3.1.json`, **byte-différent** : ordre des clés inversé, indentation 4 espaces, accents échappés `\u00e9` (vérifié par deep-equal) |
| `petstore-3.1-breaking.json` | diff de rupture | même titre, `2.0.0` : `deletePet` supprimé (`OAS-010`), réponse `404` de `getPet` supprimée (`OAS-011`), `Pet.id` `integer → string` (`OAS-012`), paramètre requis `expand` ajouté (`OAS-013`) |
| `petstore-3.1-parameter-removed.json` | `OAS-015` | même titre, `1.1.0`, paramètre `limit` de `listPets` supprimé, rien d'autre |
| `petstore-3.1-operation-renamed.json` | `OAS-016` | même titre, `1.1.0`, `getPet` renommé `findPet`, chemin et méthode inchangés |
| `petstore-3.1-server-changed.json` | `OAS-032` | même titre, `1.1.0`, URL de serveur `v1 → v2` et scope `write:pets` retiré |
| `petstore-3.1-duplicate-operation-id.json` | `OAS-003` | `deletePet` réutilise l'`operationId` `getPet` |
| `petstore-3.1-duplicate-semver.json` | `OAS-004` | même titre et même `info.version` `1.0.0` que la base, seule la description change |
| `petstore-3.1-no-operations.json` | `OAS-061` | document valide, `paths: {}`, `components` présent, aucune opération |
| `petstore-3.1-without-operation-id.yaml` | `OAS-WARN-001` | `operationId` retiré de `getPet` uniquement |
| `petstore-3.1-with-architecture-ids.json` | extensions hors empreinte | base + `x-architecture-interface-id` / `x-architecture-version-id` inconnus |
| `invalid-document.json` | `OAS-002` | JSON tronqué, illisible en JSON comme en YAML |

## Tests

### Tests bloquants du §8 (10)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 1 | `oas_import_3_0_3_1_3_2` | `petstore-3.0.yaml`, `petstore-3.1.json`, `petstore-3.2.yaml` | dépôt neuf, domaine « Contrats » | `parse_openapi` renvoie un `ParsedContract` par famille (`format`, titre, `raw`, 4 opérations triées, schéma `Pet`) ; l'aperçu confirme nom/version/octets ; après `apply`, 3 interfaces | — |
| 2 | `oas_raw_document_is_preserved` | `petstore-3.1.json` | import + `apply` | `contract_raw` identique octet-à-octet ; `contract_hash` = 64 hexadécimaux | — |
| 3 | `oas_import_is_idempotent_by_fingerprint` | `petstore-3.1.json` + `petstore-3.1-unchanged.json` | import + `apply` de la base | les deux textes diffèrent ; même `hash` et même `canonical` ; second aperçu `Unchanged`, même interface, `target_version_id` existante ; `apply` renvoie `written = false` ; 1 interface / 1 version | `OAS-050` (statut) |
| 4 | `oas_import_creates_preview_without_write` | `petstore-3.1.json` | dépôt neuf | aperçu `New` ; `interface_id`, `base_version_id`, `target_version_id` à `None` ; `raw`/`raw_bytes` exacts ; `stats()` à 0/0 | — |
| 5 | `oas_apply_creates_interface_and_version` | `petstore-3.0.yaml` | aperçu `New` | `written = true` ; interface `APPLICATION` / `OPENAPI` ; version `1.0.0` `DRAFT` | — |
| 6 | `oas_diff_classifies_breaking_change` | `petstore-3.1.json` puis `petstore-3.1-breaking.json` | import + `apply` de 3.1 | `UpdateProposed`, `base_version_id` = version existante, `target_version_id = None`, `BREAKING`, confirmation requise, codes `OAS-010/011/012/013` tous ruptures, aucune écriture | — |
| 7 | `oas_breaking_change_requires_confirmation` | idem | aperçu de rupture | sans confirmation : `OAS-040`, 1 version ; avec : version `2.0.0`, `breaking_change = true`, 2 versions | `OAS-040` |
| 8 | `oas_export_is_byte_reproducible` | `petstore-3.1.json` | import + `apply` | deux exports strictement égaux, JSON valide, LF final, pas de `x-architecture-snapshot-at`, `openapi: 3.1.0` | — |
| 9 | `oas_export_refuses_non_http_interface` | aucune (API) | interface `BUSINESS` + version | export refusé | `OAS-060` |
| 10 | `oas_export_injects_architecture_extensions` | `petstore-3.1.json` | import + `apply` | JSON contenant les 6 extensions ; ids réels, clé logique `petstore-3-1`, version `1.0.0`, générateur `archifier[/(version)]` | — |

### Canonicalisation et avertissements (2)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 11 | `yaml_and_json_share_fingerprint` | `petstore-3.1.json` + `petstore-3.1.yaml` | dépôt neuf | `hash` et `canonical` identiques ; après `apply` JSON, le YAML est `Unchanged`, même interface | — |
| 12 | `oas_warn_001_generated_operation_id` | `petstore-3.1-without-operation-id.yaml` | dépôt neuf | aperçu accepté, `warnings` non vide et contenant `OAS-WARN-001` ; `contract_snapshot.operations[*].operation_id` non vides | `OAS-WARN-001` |

### Erreurs typées (4)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 13 | `oas_002_invalid_document` | `invalid-document.json` | dépôt neuf | aperçu refusé, aucune écriture | `OAS-002` |
| 14 | `oas_003_duplicate_operation_id` | `petstore-3.1-duplicate-operation-id.json` | dépôt neuf | aperçu refusé, aucune interface | `OAS-003` |
| 15 | `oas_004_duplicate_semver` | `petstore-3.1.json` puis `petstore-3.1-duplicate-semver.json` | import + `apply` de la base | refus à l'aperçu **ou** à l'application, aucune version ajoutée | `OAS-004` |
| 16 | `oas_061_version_without_contract` | `petstore-3.1-no-operations.json` | import + `apply` | version créée mais export refusé | `OAS-061` |

### Changements de diff (3)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 17 | `oas_015_parameter_removed` | `petstore-3.1-parameter-removed.json` | import + `apply` de 3.1 | `UpdateProposed`, `base_version_id` renseignée, changement `OAS-015`, classification `BREAKING`, aucune écriture | `OAS-015` |
| 18 | `oas_016_operation_id_renamed` | `petstore-3.1-operation-renamed.json` | import + `apply` de 3.1 | changement `OAS-016`, **aucun** `OAS-010`, classification `BREAKING`, aucune écriture | `OAS-016` |
| 19 | `oas_032_server_or_security_changed` | `petstore-3.1-server-changed.json` | import + `apply` de 3.1 | changement `OAS-032`, classification `UNKNOWN`, confirmation requise, aucune écriture | `OAS-032` |

### Confirmation d'un inconnu et diff stocké (2)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 20 | `oas_unknown_change_requires_confirmation` | `petstore-3.1.json` puis `petstore-3.1-server-changed.json` | aperçu `UNKNOWN` | sans confirmation : `OAS-040`, 1 version ; avec : version `1.1.0`, `breaking_change = false` | `OAS-040` |
| 21 | `oas_diff_two_stored_versions` | `petstore-3.1.json` puis `petstore-3.1-breaking.json` | deux versions stockées | diff `BREAKING`, confirmation requise, codes `OAS-010/011/012/013` | — |

### Test complémentaire d'empreinte (1)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 22 | `oas_architecture_extensions_do_not_change_fingerprint` | `petstore-3.1.json` + `petstore-3.1-with-architecture-ids.json` | import + `apply` de 3.1 | second aperçu `Unchanged`, même interface, 1 version | — |

## Traçabilité code → test

| Code | Nature | Test(s) |
|---|---|---|
| `OAS-002` | erreur : document illisible | 13 |
| `OAS-003` | erreur : `operationId` dupliqué | 14 |
| `OAS-004` | erreur : SemVer dupliquée | 15 |
| `OAS-010` | changement : opération/chemin supprimé | 6, 21 |
| `OAS-011` | changement : réponse supprimée | 6, 21 |
| `OAS-012` | changement : type changé | 6, 21 |
| `OAS-013` | changement : paramètre requis ajouté | 6, 21 |
| `OAS-015` | changement : paramètre supprimé | 17 |
| `OAS-016` | changement : `operationId` renommé | 18 |
| `OAS-032` | changement : serveur/sécurité modifié | 19, 20 |
| `OAS-040` | blocage : confirmation requise | 7, 20 |
| `OAS-050` | statut : empreinte déjà importée | 3, 11, 22 |
| `OAS-060` | refus : export non HTTP | 9 |
| `OAS-061` | refus : version sans contrat | 16 |
| `OAS-WARN-001` | avertissement : `operationId` généré | 12 |

## Couverture des règles

| Règle | Test(s) |
|---|---|
| Détection 3.0 / 3.1 / 3.2 et `ParsedContract` (§2, §9.9) | 1 |
| `raw` conservé, `hash` SHA-256 (§2) | 1, 2, 4 |
| Canonicalisation : ordre des clés et espaces ignorés (§2, §10.8) | 3 |
| Canonicalisation : YAML = JSON (§2) | 11 |
| `x-architecture-*` hors empreinte (§2, §10.2) | 22 |
| Opérations triées, méthodes en majuscules (§2, §10.3) | 1 |
| Aperçu sans écriture (§10.1) | 4 |
| `apply` : interface `APPLICATION`/`OPENAPI`, version `DRAFT` (§10.2) | 5 |
| `Unchanged` sans écriture, `target_version_id` existante (§9.6, §9.7) | 3 |
| `UpdateProposed` = nouvelle version, `base_version_id` renseignée (§10.1, §10.2) | 6, 17, 18, 19 |
| Sévérité `Breaking > Unknown > Compatible` (§10.4) | 6, 17, 18, 19 |
| `requires_confirmation` pour `BREAKING` et `UNKNOWN` (§10.4) | 6, 7, 19, 20 |
| `breaking_change` seulement si rupture confirmée (§10.4) | 7, 20 |
| Diff de codes séparés (§10.4) | 6, 17, 18, 19 |
| Export JSON canonique, déterministe, LF final (§9.3, §10.5) | 8 |
| Export HTTP uniquement (§6) | 9 |
| Extensions injectées (§6, §10.5) | 10 |
| Refus d'une version sans opération (§10.5) | 16 |
| Diff entre deux versions stockées (§10.7) | 21 |
| Avertissement `OAS-WARN-001` (§9.11) | 12 |

## Hypothèses et points à confirmer

1. **Signature de `diff_openapi_versions`.** §10.7 mentionne
   `diff_openapi_versions(from, to) -> ContractDiff` sans fixer le porteur ni le
   retour. Le test 21 appelle `Repository::diff_openapi_versions(from, to)` et
   suppose un `RepoResult<ContractDiff>` (cohérent avec les autres lectures du
   dépôt). À figer dans le contrat si le moteur expose autre chose.
2. **Étape du refus `OAS-004`.** §10.2 dit « aucune écriture » sans préciser si le
   refus intervient à l'aperçu ou à l'application. Le test 15 enchaîne les deux et
   vérifie l'absence d'écriture, ce qui reste valable dans les deux cas.
3. **Classes de `OAS-015` et `OAS-016`.** §10.4 ne donne pas la classe de ces deux
   changements. Les tests 17 et 18 attendent `BREAKING`, conformément à
   `technical-specifications.md` §11.3 (« suppression de paramètre » et
   « renommage d'`operationId` »).
4. **Contenu des avertissements.** Le test 12 attend que la chaîne de
   `preview.warnings` contienne `OAS-WARN-001`. Le contrat nomme le code mais pas
   le format exact du message.
5. **Document sans opération.** Le test 16 suppose qu'un document valide avec
   `paths: {}` et `components` est importable, et que seul l'export est refusé
   (`OAS-061`). Si l'import refuse ce document, le cas devra être couvert par une
   version créée sans snapshot.
6. **Signature du générateur.** §6 et §9.5 imposent `archifier/0.2.0`, §10.5 impose
   `archifier` (« constante, sans version de build »). Le test 10 accepte les deux
   formes ; trancher avant de figer l'octet-à-octet d'un export.
7. **`x-architecture-version`.** Le test 10 attend la valeur `1.0.0` (version
   sémantique de la version exportée), le contrat ne précise pas la source exacte.
8. **`OAS-032` pour serveur et sécurité.** Une seule fixture modifie les deux et
   n'attend qu'un code `OAS-032` ; si le moteur émet deux changements distincts,
   l'assertion `any` reste valable, mais la classification `UNKNOWN` doit primer.
9. **Titre exporté.** Le test 8 attend `info.title` = nom de l'interface
   (`Petstore 3.1`) ; le contrat ne détaille pas la reconstruction du titre depuis
   `contract_snapshot`.
10. **Champs `contract_format`.** §10.3 ajoute `contract_format` sur
    `InterfaceVersion`, mais la valeur sérialisée de `ContractFormat` n'est pas
    fixée (le renommage `SCREAMING_SNAKE_CASE` de `OpenApi31` est ambigu). Les
    tests ne l'assertent pas.
