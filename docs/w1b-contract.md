# Contrat W1b — Diff de révisions, export Markdown, liens cassés, graphe

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | T1-04, T1-05, T1-06, T1-07 |
| Statut | **Livré et vérifié** |
| Prérequis | W1a vert (OpenAPI), UI W1a intégrée |

## 1. Types et API

### 1.1 Diff de révisions (T1-04)

```rust
// crates/core/src/history_diff.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDifference {
    pub path: String,              // JSON Pointer RFC 6901
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
}

pub fn diff_values(before: &Value, after: &Value) -> Vec<FieldDifference>;
```

```rust
// crates/repo/src/types.rs
pub struct RevisionDiff {
    pub object_id: ObjectId,
    pub from: Revision,
    pub to: Revision,
    pub changes: Vec<FieldDifference>,
}

impl Repository {
    pub fn revision_diff(
        &self,
        object_id: ObjectId,
        from: Revision,
        to: Revision,
    ) -> RepoResult<RevisionDiff>;
}
```

- `changes` triés par `path`.
- `from == to` ⇒ liste vide.
- révision absente ⇒ `RepoError` code `DIFF-001`.
- aucun accès direct au stockage depuis l'UI.

### 1.2 Export Markdown (T1-05)

```rust
// crates/core/src/export.rs
pub struct ExportFile {
    pub path: String,      // relatif au bundle, séparateur '/'
    pub content: String,
}

pub struct ExportBundle {
    pub root: String,      // slug du domaine ou de l'objet
    pub files: Vec<ExportFile>,
}

pub fn export_domain(dataset: &Dataset, domain_id: ObjectId) -> Result<ExportBundle, ExportError>;
pub fn export_object(dataset: &Dataset, object_id: ObjectId) -> Result<ExportBundle, ExportError>;
```

```rust
impl Repository {
    pub fn export_domain_markdown(&self, domain_id: ObjectId) -> RepoResult<ExportBundle>;
    pub fn export_object_markdown(&self, object_id: ObjectId) -> RepoResult<ExportBundle>;
}
```

Arborescence :

```text
<root>/index.md
<root>/domains/<id>.md
<root>/elements/<id>.md
<root>/interfaces/<id>.md
<root>/diagrams/<id>.md
<root>/diagrams/<id>.svg
<root>/diagrams/<id>.mmd
<root>/stories/<id>.md
<root>/reviews/<id>.md
<root>/relationships/<id>.md
```

`ExportFile.path` est **relatif à `root`** : `index.md`, `elements/<id>.md`, etc. Le préfixe `<root>/` n'est pas inclus dans `path`.

- `index.md` liste les objets du domaine avec liens relatifs.
- Chaque page objet contient : titre, domaine, type, propriétés, documentation, relations, liens vers les voisins.
- Les liens `arch://` sont convertis en liens relatifs vers les fichiers du bundle ; aucun `arch://` ne subsiste.
- `export_object` inclut les voisins directs pour que tous les liens résolvent.
- SVG : généré depuis `DiagramLayout.nodes`/`.edges`, viewBox calculé, rectangles + libellés, lignes pour les arêtes, aucun script, aucune ressource externe.
- Mermaid : `flowchart TD` avec un nœud par `DiagramNode` et une arête par `DiagramEdge`.
- Reproductibilité : objets triés par `ObjectId`, fichiers triés par `path`, aucune date d'export, aucune horloge courante. Les dates affichées viennent de `created_at` / `updated_at`.
- Un export de domaine sans objet produit un `index.md` valide.
- Un objet archivé est exporté en lecture seule avec un bandeau « archivé ».

### 1.3 Liens cassés (T1-06)

```rust
// crates/repo/src/types.rs
pub struct BrokenLink {
    pub code: String,              // DOC-LINK-001 (cassé) ou DOC-LINK-002 (archivé)
    pub link: archifier_core::Link,
    pub source: ObjectRef,
    pub owner: Option<ObjectRef>,  // domaine responsable de la source
    pub target: Option<ObjectRef>, // None si l'objet cible n'existe pas
    pub context: String,           // 80 avant + 80 après l'URI, `…` si rogné
}

impl Repository {
    pub fn broken_links(&self) -> Vec<BrokenLink>;
}
```

- Inclut `LinkStatus::Broken` et `LinkStatus::Archived`.
- `context` est extrait de la documentation de la source autour de `target_uri` : 80 caractères avant + 80 après, bornes UTF-8.
- `owner` est le domaine responsable de la source (accès au propriétaire), `None` si introuvable.
- `code` vaut `DOC-LINK-001` ou `DOC-LINK-002`.
- Tri par `(source.id, target_uri, link.id)` ; dédoublonnage par `(source_object_id, target_uri)`.
- Aucune correction automatique, aucun lien supprimé.

### 1.4 Graphe de dépendances (T1-07)

L'API existante `Repository::dependencies(root, direction, depth) -> RepoResult<Graph>` est conservée.

- Profondeur bornée 1 à 6, défaut 3.
- `Graph.nodes` contient `depth`, `path`, `relation`.
- UI dédiée : route `#/graph/<id>`, sélecteur de direction (`IN`, `OUT`, `BOTH`) et de profondeur (1–6), liste groupée par profondeur avec chemins cliquables.
- Un nœud déjà visité n'est pas dupliqué ; `truncated` est affiché si la borne est atteinte.
- Aucun nouveau moteur de graphe : réutiliser `dependencies`.

## 2. Codes et erreurs

| Code | Sens |
|---|---|
| `DIFF-001` | révision absente ou objet inconnu |
| `EXPORT-001` | domaine/objet inconnu |
| `EXPORT-002` | diagramme illisible (layout absent) |
| `LINK-001` | lien cassé détecté (informationnel, pas une erreur d'écriture) |

## 3. UI

| Écran | Comportement |
|---|---|
| Fiche élément / interface | Panneau **Historique** : deux sélecteurs de révision, bouton **Comparer**, tableau `path / avant / après`. |
| Page domaine | Bouton **Exporter le domaine en Markdown** ; téléchargement des fichiers du bundle (index + pages + SVG/Mermaid). |
| Fiche objet | Bouton **Exporter en Markdown** ; même bundle réduit aux voisins directs. |
| Réglages / diagnostic | Panneau **Liens cassés** : source, cible, statut, extrait, lien vers le propriétaire. |
| Graphe | Route dédiée `#/graph/<id>` avec direction et profondeur. |

- Téléchargement : `Blob` + ancre, pas de dépendance zip. L'UI peut télécharger l'`index.md` et proposer les autres fichiers dans une liste ; le bundle reste la source de vérité testée côté moteur.
- Aucun `unwrap` / `expect` / `panic!` hors tests.
- Aucun accès direct au stockage : `repo()` uniquement.

## 5. Décisions de revue (gelées)

### 5.1 Périmètre FR-019

W1b livre l'export **domaine et objet** (T1-05). L'export **revue** et **Story** de FR-019 est explicitement reporté en W1c, tracé comme extension de T1-05. Le bundle contient néanmoins les pages `reviews/` et `stories/` lorsqu'elles sont atteintes par un lien de documentation, en mode stub.

### 5.2 Arborescence unique

L'arborescence du §1.2 prévaut sur `technical-specifications.md` §12.2 et `functional-specifications.md` §298-311 pour W1b. Elle ajoute :

```text
<root>/relationships/<id>.md
```

- `arch://relationship/<id>` → `relationships/<id>.md`.
- `arch://version/<version_id>` → `interfaces/<interface_id>.md#v<semver>` (lookup dans `Dataset.interface_versions`).
- `arch://interface/<id>` → `interfaces/<id>.md`, etc.
- Une URI résoluble devient un lien relatif ; une URI non résoluble devient du texte suivi de `> ⚠ lien cassé vers arch://...` (aucune suppression silencieuse).
- Un test vérifie qu'aucun `arch://` ne subsiste **dans les liens générés** ; les notes de lien cassé peuvent en citer un.

### 5.3 Révisions et documentation

- Le payload de `revision_diff` est le `Value` **stocké** par `revision_snapshot` (agrégat par type : `ModelElement`, `Interface`, `Diagram`, etc.), pas seulement `RepositoryObject`.
- La documentation n'est pas dans ce payload : elle reste consultable via `documentation_history`, et le panneau affiche les deux sources séparément.
- Champs exclus du diff : `/object/revision`, `/object/updated_at` (métadonnées affichées à part).
- `from == to` ⇒ vide, **après** vérification que les deux snapshots existent (`DIFF-001` prioritaire).
- Les révisions sélectionnables sont `history()` filtré par `revision_snapshot(id, r).is_some()`.

### 5.4 Liens cassés et archivés

- `broken_links()` **recalcule** le statut à la lecture depuis le `lifecycle` courant : `BROKEN` si la cible n'existe pas, `ARCHIVED` si elle existe mais n'est plus active. `Link.status` persisté n'est qu'un cache.
- `owner: Option<ObjectRef>` : domaine responsable de la **source**, `None` si introuvable.
- `context` = 80 caractères avant + 80 après l'occurrence de `target_uri`, sur la ligne courante, bornes UTF-8, `…` si rogné.
- Dédoublonnage par `(source_object_id, target_uri)` ; première occurrence pour le contexte ; tri `(source.id, target_uri, link.id)`.
- Codes conservés : `DOC-LINK-001` (cassé), `DOC-LINK-002` (archivé). `LINK-001` est supprimé.
- Panneau UI : **Liens invalides**, avec libellés distincts `Cassé` / `Archivé`.
- `stats().broken_links` doit utiliser le même recalcul (sinon il reste faux).

### 5.5 Erreurs et dérives

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    #[error("objet ou domaine inconnu")]
    UnknownSubject,
    #[error("diagramme illisible : {0}")]
    UnreadableDiagram(String),
}
```

- Mapping repo : `EXPORT-001` (sujet inconnu), `EXPORT-002` (diagramme illisible/coordonnées non finies).
- `ExportFile`, `ExportBundle`, `RevisionDiff`, `BrokenLink`, `FieldDifference` : `Debug + Clone + PartialEq + Serialize + Deserialize`.
- `FieldChange` mort est supprimé ; le type unique est `FieldDifference`.

### 5.6 Reproductibilité de l'export

- UTF-8 sans BOM, `\n`, saut de ligne final, aucune itération `HashMap`/`HashSet`.
- `files` trié par `path` (octets), `index.md` trié par `ObjectId`.
- `ExportFile.path` est relatif à `ExportBundle.root` (`"index.md"`, `"elements/<id>.md"`).
- `root` = `domain.slug` ; pour un objet, `slugify(name)` ou `id` si vide ; interdire `.`/`..` et séparateurs.
- L'UI aplatit `/` en `-` dans le nom de fichier proposé (pas de zip).
- Aucune date d'export ; les dates viennent de `created_at`/`updated_at`.

### 5.7 SVG et Mermaid

- SVG : bbox = union des `(x,y,w,h)` des nœuds + padding 8 ; `viewBox="minx miny width height"` avec `{:.1}` ; libellés échappés (`&`, `<`, `>`, `"`) ; `style_override` ignoré ; viewport et annotations ignorés ; nœud non résolu ⇒ `Objet supprimé` ; diagramme sans nœud ⇒ ni SVG ni Mermaid, sans erreur.
- Coordonnée non finie ou `w/h ≤ 0` ⇒ `EXPORT-002`. Un JSON ne peut pas porter `NaN` ; le cas `w/h ≤ 0` couvre les données importées corrompues.
- Mermaid : `flowchart LR` (§5.7 fait foi sur §1.2) ; ids `n0`, `n1`, … dans l'ordre du tableau ; libellés = nom d'objet échappé ; arête `nX -->|relation_type| nY` via `Dataset.relationships` et le premier nœud dont `object_id` correspond ; arête non résolue ignorée.
- Bandeau archivé : `> **Archivé** — consultation seule, dernière révision` en première ligne après le titre.

### 5.8 Contenu des pages

- `index.md` : `# <Domaine>`, ligne slug/id, sections `## Domaines`, `## Éléments`, `## Interfaces`, `## Diagrammes`, `## Histoires`, `## Revues`, puces `- [Nom](<segment>/<id>.md)` triées par `ObjectId`, section vide omise, ligne finale.
- Page interface : identité, domaine, catégorie, versions (semver, statut, fenêtre), contrat (format + hash), consommateurs, dépendances, diagrammes.
- `export_object` inclut la fermeture : page racine + liens documentaires + relations entrantes/sortantes + consommateurs + nœuds des diagrammes inclus, avec pages stub pour les objets hors domaine. Un lien non résoluble est rendu en texte + note.

### 5.9 Graphe

- `dependencies` est conservé tel quel : toutes relations actives + consommation. Écart assumé à `technical-specifications.md` §10.2, filtre par type reporté.
- `clamp(1,6)`, défaut UI 3 ; le `400 QUERY-DEPTH-001` est un sujet HTTP lot 2.
- `DependencyNode` gagne `direction: Direction` et `lifecycle: Lifecycle`.
- Tri `(depth, path)` ; `BOTH` est une union étiquetée par `direction` si le champ est ajouté.
- `truncated` n'est vrai que si un voisin est découvert au-delà de la borne ou si la borne dure de 500 nœuds est atteinte.
- Les cibles archivées sont incluses et signalées par `lifecycle`.
- FR-028 reste couvert par le panneau `usages` existant ; le graphe ne traite que relations et consommation.
- Route `#/graph/<id>` (deux segments), types admis : élément et interface ; direction et profondeur en état local, non partageables ; `Route::Graph` + `to_hash` + round-trip.
- Erreur d'objet inconnu : `NOT-FOUND` (comme `dependencies`).
- Test unitaire dans `router.rs` + mise à jour de `hash_roundtrip_for_each_route`.

### 5.10 UI

- Panneau historique : **Historique des révisions**, bouton **Comparer les révisions** (ne pas confondre avec **Comparer deux versions** d'OpenAPI).
- L'aide de téléchargement est déplacée dans `components`/`state`, pas dans `settings.rs`.
- Le panneau des liens s'appelle **Liens invalides** et affiche `link.status`.

## 6. Tests bloquants

- `revision_diff_is_field_by_field`
- `revision_diff_is_sorted`
- `revision_diff_is_immutable`
- `revision_diff_missing_revision_is_diff_001`
- `revision_diff_excludes_volatile_metadata`
- `markdown_export_has_valid_relative_links`
- `markdown_export_is_reproducible`
- `markdown_export_includes_svg_and_mermaid`
- `markdown_export_includes_neighbors_for_object`
- `markdown_export_empty_domain_has_index`
- `markdown_export_archived_object_has_banner`
- `markdown_export_non_finite_coordinate_is_export_002`
- `markdown_export_unknown_subject_is_export_001`
- `broken_link_diagnostic_has_context_and_owner`
- `broken_link_diagnostic_includes_archived`
- `broken_link_diagnostic_deduplicates`
- `dependency_graph_shows_paths_and_depth`
- `dependency_graph_is_bounded_to_six`
- `dependency_graph_both_direction_is_labelled`
- `dependency_graph_archived_target_is_flagged`
- `graph_route_parses`

Les tests de route vivent dans `crates/ui/src/router.rs` (unitaire) ; les autres dans `crates/repo/tests/**`.

