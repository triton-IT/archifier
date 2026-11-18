# Contrat W1a — OpenAPI import / export / diff

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | T1-01, T1-02, T1-03 |
| Statut | **Gelé pour W1a** |
| Périmètre fichiers | `crates/core/src/openapi.rs`, `crates/core/src/contract_diff.rs`, `crates/repo/src/openapi.rs`, `crates/repo/src/lib.rs` (écrivain unique), `crates/ui/src/views/**`, `fixtures/mvp2/openapi/**`, `crates/repo/tests/openapi_acceptance.rs` |

## 1. Types core

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractFormat {
    #[serde(rename = "OPENAPI_3_0")]
    OpenApi30,
    #[serde(rename = "OPENAPI_3_1")]
    OpenApi31,
    #[serde(rename = "OPENAPI_3_2")]
    OpenApi32,
    #[serde(rename = "ASYNCAPI_2")]
    AsyncApi2,
    #[serde(rename = "ASYNCAPI_3")]
    AsyncApi3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedContract {
    pub format: ContractFormat,
    pub raw: String,
    pub canonical: String,
    pub hash: String,
    pub title: String,
    pub version: String,
    pub description: String,
    pub servers: Vec<String>,
    pub security: Vec<String>,
    pub operations: Vec<ContractOperation>,
    pub schemas: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractOperation {
    pub operation_id: String,
    pub method: String,       // GET, POST, ...
    pub path: String,
    pub parameters: Vec<serde_json::Value>,
    pub request_body: Option<serde_json::Value>,
    pub responses: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiffClass {
    Compatible,
    Breaking,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractChange {
    pub code: String,          // OAS-010, OAS-012, ...
    pub path: String,          // /paths/...
    pub message: String,
    pub class: DiffClass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractDiff {
    pub changes: Vec<ContractChange>,
    pub classification: DiffClass,
    pub requires_confirmation: bool,
}
```

## 2. Parsing et empreinte

- Entrée : texte JSON ou YAML.
- Détection : champ `openapi` = `3.0.x`, `3.1.x`, `3.2.x`.
- Erreurs typées : `OAS-001` version non supportée, `OAS-002` document illisible.
- `canonical` : JSON canonique à clés triées, sans espaces significatifs, **hors namespace `x-architecture-*`**.
- `hash` : SHA-256 hexadécimal de `canonical`.
- `raw` : texte d'origine conservé **octet-à-octet** (chaîne UTF-8).
- `operations` : triées par `(path, method)`.
- `operationId` absent : généré `method_path`, avertissement `OAS-WARN-001`.

## 3. Types repo

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpenApiImportStatus {
    New,
    UpdateProposed,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenApiImportPreview {
    pub status: OpenApiImportStatus,
    pub domain_id: ObjectId,
    pub interface_id: Option<ObjectId>,
    pub version_id: Option<ObjectId>,
    pub interface_name: String,
    pub semantic_version: String,
    pub diff: ContractDiff,
    pub raw_bytes: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenApiImportOutcome {
    pub status: OpenApiImportStatus,
    pub interface_id: ObjectId,
    pub version_id: Option<ObjectId>,
    pub written: bool,
}

impl Repository {
    pub fn import_openapi_preview(
        &self,
        domain_id: ObjectId,
        raw: &str,
    ) -> RepoResult<OpenApiImportPreview>;

    pub fn apply_openapi_import(
        &self,
        preview: &OpenApiImportPreview,
        confirm_breaking: bool,
    ) -> RepoResult<OpenApiImportOutcome>;

    pub fn export_openapi(&self, version_id: ObjectId) -> RepoResult<String>;
}
```

## 4. Règles d'identité

Ordre de résolution :

1. `x-architecture-version-id` connu → mise à jour de cette version ;
2. `x-architecture-interface-id` connu → nouvelle version de cette interface ;
3. `hash` déjà présent sur une version connue → `Unchanged`, aucune écriture ;
4. clé logique dérivée du titre (`slugify(info.title)`) déjà présente → `UpdateProposed` ;
5. sinon → `New`, nouvelle interface proposée.

- Un `Unchanged` ne crée ni interface ni version.
- Un `UpdateProposed` crée une **nouvelle version** (jamais une réécriture silencieuse).
- Le document brut et le hash sont stockés dans `InterfaceVersion.contract_raw` et `contract_hash`.
- `contract_snapshot` contient le modèle normalisé (serveurs, opérations, schémas, sécurité).

## 5. Diff

| Code | Changement | Classe |
|---|---|---|
| `OAS-010` | opération ou chemin supprimé | rupture |
| `OAS-011` | réponse supprimée | rupture |
| `OAS-012` | type de schéma changé | rupture |
| `OAS-013` | paramètre requis ajouté | rupture |
| `OAS-014` | schéma de réponse restreint | rupture |
| `OAS-015` | paramètre supprimé | rupture |
| `OAS-016` | `operationId` renommé | rupture |
| `OAS-020` | description / résumé modifié | compatible |
| `OAS-021` | champ optionnel ajouté | compatible |
| `OAS-030` | `$ref` non résolu | inconnu |
| `OAS-031` | extension `x-*` modifiée | inconnu |
| `OAS-032` | serveur ou sécurité modifié | inconnu |
| `OAS-040` | rupture non confirmée | blocage |
| `OAS-050` | empreinte déjà importée | idempotence |
| `OAS-060` | export demandé pour une interface non HTTP | refus |

- `requires_confirmation = classification == Breaking`.
- `apply_openapi_import(..., confirm_breaking = false)` sur une rupture ⇒ `RepoError` de code `OAS-040`, aucune écriture.
- Avec `confirm_breaking = true`, la version est créée avec `breaking_change = true`.

## 6. Export

- Autorisé uniquement si `category == APPLICATION` et `contract_type == OpenApi`.
- Sinon `RepoError` code `OAS-060`.
- Génère un document **OpenAPI 3.1.0** à partir de `contract_snapshot`.
- Injecte :
  - `x-architecture-interface-id`
  - `x-architecture-version-id`
  - `x-architecture-domain-id`
  - `x-architecture-logical-key`
  - `x-architecture-version`
  - `x-architecture-generated-by: archifier`
- Aucun horodatage volatile : utiliser `created_at` de la version ou une date figée.
- Sortie **JSON canonique** (clés triées, indentation stable) ou YAML déterministe ; le test compare octet-à-octet deux exports successifs.
- Les `x-architecture-*` sont exclus de l'empreinte d'idempotence.

## 7. UI minimale

- Écran Réglages ou fiche interface : zone de texte pour coller un document, bouton **Analyser**, affichage du statut et du diff, case **Confirmer la rupture**, bouton **Appliquer**, bouton **Exporter** pour une version compatible.
- Aucun accès direct au stockage : uniquement `repo()`.
- Les erreurs `OAS-*` sont affichées avec leur code.

## 8. Tests bloquants

- `oas_import_3_0_3_1_3_2` : les trois familles produisent un `ParsedContract`.
- `oas_raw_document_is_preserved` : `contract_raw` est identique à l'entrée.
- `oas_import_is_idempotent_by_fingerprint` : deux imports identiques ⇒ `Unchanged`, une seule interface.
- `oas_import_creates_preview_without_write` : `preview` ne crée rien.
- `oas_apply_creates_interface_and_version` : `apply` crée l'interface + version.
- `oas_diff_classifies_breaking_change` : suppression d'opération ⇒ `Breaking`.
- `oas_breaking_change_requires_confirmation` : sans confirmation ⇒ `OAS-040` ; avec ⇒ version `breaking_change = true`.
- `oas_export_is_byte_reproducible` : deux exports identiques.
- `oas_export_refuses_non_http_interface` : interface métier ⇒ `OAS-060`.
- `oas_export_injects_architecture_extensions` : présence des `x-architecture-*`.

## 9. Précisions d'intégration (gelées après revue du testeur)

1. **Réexports** : `crates/repo/src/lib.rs` expose `pub use openapi::*;` pour que `archifier_repo::OpenApiImportStatus` soit disponible. `crates/core/src/lib.rs` expose `pub use openapi::*;` et `pub use contract_diff::*;`.
2. **Champs bruts** : `contract_raw: Option<String>` et `contract_hash: Option<String>` sont portés par `InterfaceVersion`, sérialisés, avec `#[serde(default)]`. Pas de table séparée au MVP2.
3. **Format d'export** : JSON canonique (clés triées, indentation stable) ; c'est un document OpenAPI 3.1 valide. Le YAML reste accepté à l'import.
4. **Code d'export refusé** : `OAS-060` est le code du contrat W1a ; `OAS-EXPORT-001` de la spec technique est un alias historique, non utilisé dans le code MVP2.
5. **Générateur** : `x-architecture-generated-by: archifier` (constante, sans version de build, pour garantir la reproductibilité).
6. **Aperçu `New`** : `interface_id` et `version_id` valent `None`. **Aperçu `Unchanged`** : `interface_id` et `version_id` pointent sur l'existant. **`UpdateProposed`** : `interface_id` existe, `version_id` vaut `None` tant que la nouvelle version n'est pas appliquée.
7. **`apply` sur `Unchanged`** : renvoie `OpenApiImportOutcome { written: false, version_id: Some(existant) }`, sans erreur et sans écriture.
8. **Catégorie à l'import** : l'interface créée est `APPLICATION` avec `contract_type = OpenApi`. C'est ce qui rend l'export possible ; une interface `BUSINESS`/`TECHNICAL` ou un autre contrat est refusée à l'export.
9. **Parsing public** : `archifier_core::openapi::parse_openapi(raw: &str) -> Result<ParsedContract, OpenApiError>` est public et testable indépendamment du dépôt.
10. **`x-architecture-version-id` connu** : si la version ciblée est `DRAFT`, elle est mise à jour ; si elle est `PUBLISHED`, une nouvelle version est proposée (`UpdateProposed`) pour préserver l'immutabilité. Un test dédié peut être ajouté en W1b.
11. **`OAS-WARN-001`** : `operationId` absent ⇒ identifiant généré `method_path` + avertissement dans `warnings`. Une fixture sans `operationId` doit être ajoutée au second passage du testeur.

## 10. Décisions de revue (gelées après relecture)

### 10.1 Preview opposable et apply recalculé

La preview transporte le document brut et tout ce qui permet à `apply` de recalculer, sans faire confiance à l'appelant :

```rust
pub struct OpenApiImportPreview {
    pub status: OpenApiImportStatus,
    pub domain_id: ObjectId,
    pub interface_id: Option<ObjectId>,
    pub base_version_id: Option<ObjectId>,   // version de référence du diff
    pub target_version_id: Option<ObjectId>, // version existante à mettre à jour (DRAFT) ou None
    pub interface_name: String,
    pub semantic_version: String,
    pub format: ContractFormat,
    pub raw: String,
    pub canonical: String,
    pub hash: String,
    pub diff: ContractDiff,
    pub raw_bytes: usize,
    pub warnings: Vec<String>,
    pub expected_interface_revision: Option<u64>,
}

pub fn apply_openapi_import(
    &self,
    preview: &OpenApiImportPreview,
    confirm_breaking: bool,
) -> RepoResult<OpenApiImportOutcome>;
```

`apply` :

1. recalcule `format`, `canonical`, `hash` depuis `preview.raw` ;
2. refuse si le hash recalculé diffère de `preview.hash` (`OAS-041` `STALE`) ;
3. recalcule le diff contre la version de base ;
4. vérifie la révision de l'interface cible si elle existe (`OAS-041` si périmée) ;
5. refuse une rupture/inconnu sans confirmation (`OAS-040`) ;
6. écrit interface + version dans **un seul** `mutate`.

### 10.2 Identité et immuabilité

Ordre exact :

1. parser, calculer `(format, hash)` ;
2. `x-architecture-version-id` connu :
   - même hash que cette version ⇒ `Unchanged` ;
   - hash différent et version `DRAFT` ⇒ `UpdateProposed` ciblant cette version ;
   - hash différent et version `PUBLISHED` ⇒ `UpdateProposed` avec nouvelle version (`supersedes_version_id`) ;
3. `x-architecture-interface-id` connu :
   - hash déjà présent sur une version de l'interface ⇒ `Unchanged` ;
   - sinon `UpdateProposed` avec nouvelle version ;
4. hash déjà présent sur n'importe quelle version ⇒ `Unchanged` (clé `(format, hash)`) ;
5. clé logique dérivée du titre déjà présente **dans le même domaine** ⇒ `UpdateProposed` ;
6. sinon `New`.

Règles dures :

- Une version `PUBLISHED` n'est **jamais** réécrite.
- Une version importée est `DRAFT`, `available_from = available_until = None`, `revision = 1`, `supersedes_version_id = base_version_id`.
- Interface nouvelle : `category = APPLICATION`, `contract_type = OpenApi`, domaine responsable = `domain_id`.
- Interface existante : refus si `category != APPLICATION` (`OAS-007`).
- SemVer dupliquée sur l'interface ⇒ `OAS-004`, aucune écriture.
- `info.version` non SemVer : normalisation `1.0` → `1.0.0`, `1` → `1.0.0` ; si impossible ⇒ `OAS-006`.
- Clé logique vide/&lt;3/&gt;80 : troncature et fallback `openapi-<hash8>` ; une clé existant dans un autre domaine n'est jamais rattachée automatiquement (`OAS-005`).

### 10.3 `contract_snapshot` exact

`contract_snapshot` est un DTO normalisé, **jamais** `ParsedContract` :

```json
{
  "openapi_version": "3.1.0",
  "servers": ["https://api.example.com/v1"],
  "security_schemes": ["oauth2"],
  "operations": [
    {
      "operation_id": "getPet",
      "method": "GET",
      "path": "/pets/{id}",
      "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
      "request_body": null,
      "responses": [{ "status": "200", "schema_ref": "#/components/schemas/Pet" }]
    }
  ],
  "schemas": { "Pet": { "type": "object" } },
  "extensions": { "x-internal-tag": "..." }
}
```

- `operations` triées par `(path, method)` ; `method` en majuscules.
- `extensions` : extensions `x-*` racine, **hors** `x-architecture-*`.
- `contract_raw`, `canonical` et `hash` ne sont pas dans `contract_snapshot`.
- `contract_format`, `contract_raw`, `contract_hash` sont sur `InterfaceVersion`, `#[serde(default)]`.

### 10.4 Codes séparés

| Catégorie | Codes |
|---|---|
| Erreurs | `OAS-001` version non supportée, `OAS-002` document illisible, `OAS-003` operationId dupliqué, `OAS-004` SemVer dupliquée, `OAS-005` clé logique d'un autre domaine, `OAS-006` SemVer non normalisable, `OAS-007` catégorie incompatible, `OAS-040` confirmation requise, `OAS-041` preview périmée, `OAS-060` export non applicable, `OAS-061` version sans contrat |
| Changements | `OAS-010` opération/chemin supprimé, `OAS-011` réponse supprimée, `OAS-012` type changé, `OAS-013` paramètre requis ajouté, `OAS-014` schéma restreint, `OAS-015` paramètre supprimé, `OAS-016` operationId renommé, `OAS-020` description modifiée, `OAS-021` champ optionnel ajouté, `OAS-030` `$ref` interne non résolu, `OAS-031` extension `x-*` modifiée, `OAS-032` serveur/sécurité modifié |
| Statut | `OAS-050` empreinte déjà importée (`Unchanged`, pas une erreur) |
| Avertissements | `OAS-WARN-001` operationId généré, `OAS-WARN-002` export normalisé lossy |

Sévérité du diff : `Breaking > Unknown > Compatible`. `requires_confirmation = (classification == Breaking || classification == Unknown)`. `breaking_change = (classification == Breaking) && confirm_breaking`.

### 10.5 Export

- JSON UTF-8, clés triées à tous les niveaux, indentation 2 espaces, LF, saut de ligne final.
- Aucun `x-architecture-snapshot-at`, aucune date volatile.
- `x-architecture-generated-by: archifier` (constante, sans version de build).
- Refus si `contract_snapshot` absent, vide ou sans opération (`OAS-061`).
- `export_openapi_raw(version_id)` : optionnel W1b, restitution exacte de `contract_raw`.

### 10.6 Bordure du brut

- Entrée UTF-8 uniquement ; BOM, UTF-16 et non-UTF-8 refusés (`OAS-002`).
- Plafond 2 Mio ; profondeur 128 ; un seul document YAML ; clés dupliquées refusées.
- `$ref` externes jamais récupérés ; `$ref` internes non résolus ⇒ `OAS-030`.
- `contract_raw` conserve la chaîne UTF-8 exacte ; `raw_bytes` = longueur en octets.

### 10.7 UI

- Import sur la fiche interface / le domaine, avec sélecteur de domaine et import fichier **ou** texte.
- Case de confirmation de rupture **décochée** par défaut.
- Export sur la fiche interface, téléchargement `Blob`.
- Comparaison de deux versions stockées : `Repository::diff_openapi_versions(from, to) -> RepoResult<ContractDiff>` et panneau dédié (T1-03).
- Un document sans opération est importable (interface + version), mais son export est refusé (`OAS-061`).

### 10.8 Reports explicites

- Proposition d'objets de données depuis `components.schemas` : **hors W1a**, noté pour W1b.
- `export_openapi_raw` : W1b si le temps manque.
- JCS RFC 8785 complet : W1a utilise un sous-ensemble documenté (clés triées, nombres sérialisés par `serde_json`) ; un ADR JCS pourra être ajouté si un besoin d'interopérabilité strict apparaît.


