# Spécifications techniques — Lot 2 (MVP2)

| | |
|---|---|
| **Document** | Addendum au lot 2, complément de `technical-specifications.md` §21 |
| **Version** | 0.3 — G1 franchi |
| **Date** | 18 novembre 2026 |
| **Statut** | **G1 franchi** — contrats C-01 → C-10 figés, spike T7-00 vert |
| **Ne remplace pas** | `functional-specifications.md`, `technical-specifications.md`, `technical-specifications-lot1-front-local.md`, ADR-011, ADR-012, ADR-013, ADR-014 |
| **Portée** | Adaptations du modèle, schéma CRDT, transport, persistance, projection, fusion, OpenAPI/AsyncAPI, cycle de vie, migration de métamodèle, proxy, port de dépôt |

## 0. Invariants non négociables

1. **IndexedDB uniquement** pour la persistance. Aucun serveur de données, aucun SQL, aucun fichier côté application.
2. **Le snapshot reste le format d'échange.** Le store CRDT est séparé et ne remplace pas le snapshot.
3. **Aucune clé d'API dans le bundle.** Un LLM réel passe par `archifier-proxy`, désactivé par défaut, clé lue dans l'environnement.
4. **Aucune classe ArchiMate 4 inventée.** Le profil 3.2 reste la base normative ; la préparation 4.x est une infrastructure de mapping.
5. **`archifier-core` reste sans I/O.** L'UI ne parle qu'au port de dépôt, jamais directement à IndexedDB depuis une vue.
6. **Deux plans, jamais mélangés.** Gouvernance = écriture unique + verrou optimiste + `409`. Collaboration = CRDT. Aucune transaction mixte.
7. **Pas de `unwrap` / `expect` / `panic!` / `todo!` hors tests.** Erreurs typées.
8. **BroadcastChannel est le mode nominal du MVP2.** Aucun code ne doit exiger un relais réseau pour fonctionner.
9. **Migrations de snapshot additives uniquement**, numéro consommé définitivement. Le store `collab` est nouveau et ne remplace rien.

### 0.1 Dépendances approuvées en G1

| Dépendance | Version | Usage | Justification |
|---|---|---|---|
| `yrs` | `0.25` | CRDT `Y.Text`, `Y.Map`, `Y.Array` | autorisée par ADR-014. Constat de compilation du 18/11/2026 : 0.27.4 et 0.28.1 échouent en E0658 (`if let` guards instables sur Rust 1.94) ; 0.25.0 compile et passe le spike T7-00. |
| `sha2` | `0.10` (déjà verrouillé en 0.10.9) | empreinte canonique OpenAPI/AsyncAPI | pure Rust, déjà dans le graphe, aucune nouvelle surface transitive |
| `serde_yaml_ng` | `0.10` | import YAML OpenAPI 3.0/3.1/3.2 et AsyncAPI 2.x/3.x | fork maintenu de `serde_yaml`, pur Rust via `unsafe-libyaml`, compatible WASM, MSRV 1.64 |

Alternatives écartées :

- `serde_yaml` : déprécié.
- `yaml_serde` : maintenu par la YAML Organization, mais s'appuie sur `libyaml-rs` (binding C), moins adapté au build `wasm32-unknown-unknown`.
- `serde-saphyr` : très bon candidat (panic-free, WASM), mais MSRV 1.89 alors que le workspace déclare `rust-version = "1.88"` ; à réévaluer si le workspace monte de MSRV.

Toute autre dépendance passe par un ADR et une validation explicite. `archifier-proxy` ne doit jamais être une dépendance de `archifier-ui`.

## 1. Écarts constatés avec l'état annoncé au 18/11/2026

| Réf. | Constat vérifié | Conséquence pour le lot 2 |
|---|---|---|
| E-01 | `RepositoryPort` n'existe pas comme trait dans le code. `archifier-ui` utilise `archifier_repo::Repository`, type concret, via `state::repo()`. | **Décision G1, option B (ADR-016)** : le type `Repository` est le port effectif, centralisé par `state::repo()` ; aucune vue n'accède au stockage. Pas d'extraction de trait en W1. |
| E-02 | `WriteTx` n'existe pas comme type dans le code. Le chemin d'écriture est `Repository::mutate`, privé, avec validation et persistance atomique. | Le contrat de gouvernance est conservé fonctionnellement ; le nom `WriteTx` reste celui de la cible. Ne pas renommer le chemin existant sans ADR. |
| E-03 | Aucun code de collaboration : ni CRDT, ni transport, ni présence. | W3/W4 partent de zéro, derrière les contrats C-01 → C-07. |
| E-04 | `docs/adr/ADR-015-transport-synchronisation.md` était absent au début de W0 ; ADR-015 n'existait que comme ligne du tableau §20. | **Résolu le 18/11/2026** : ADR-015 créé et accepté. |
| E-05 | `cargo fmt --all -- --check` échoue ; `cargo clippy --workspace --all-targets -- -D warnings` échoue dès `core` (`manual_range_contains`). | Normalisation mécanique en W0, avant toute vague parallèle. |
| E-06 | Deux `panic!` hors tests : `crates/repo/src/lib.rs:509` (profils embarqués) et `crates/ui/src/state.rs:29` (dépôt non installé). | **Résolu en W0** : profils illisibles ⇒ `MetamodelRegistry::default()` + `profile_error()` ; dépôt non installé ⇒ repli mémoire anti-panic documenté dans ADR-016. |
| E-07 | Le dépôt n'est pas un dépôt Git. Les worktrees d'isolation ne sont pas disponibles. | Isolation par périmètre de fichiers exclusif et intégration séquentielle par l'orchestrateur. |
| E-08 | `backlog-mvp1.md` conserve 81 lignes à `⬜` alors que la synthèse en tête distingue livré / partiel / non livré. | Ne pas réécrire le backlog lot 1 ; la ventilation de tête fait foi. Le backlog MVP2 tient l'état ligne à ligne. |

## 2. Les deux plans

### 2.1 Gouvernance — inchangée

| Contenu | Sémantique | Point d'entrée |
|---|---|---|
| domaines, éléments, relations, interfaces, versions, contrats, statuts de revue, ChangeSets | écriture unique, `revision` + `If-Match`, `409` | `Repository::mutate` (cible : `WriteTx`) |

- Toute écriture valide via `archifier-core` et `archifier-metamodel`.
- Une écriture = un enregistrement atomique du snapshot.
- Un conflit de révision produit un `409` et un dialogue recharger / comparer / forcer.
- Jamais de fusion automatique.

### 2.2 Collaboration — nouvelle

| Contenu | Sémantique | Point d'entrée |
|---|---|---|
| documentation Markdown, `name`, layout de diagramme, blocs de Story, commentaires, présence | CRDT, convergence automatique | `CollabEdit` |

- La CRDT ne crée jamais d'objet, ne publie jamais de version, ne change jamais un statut de revue.
- La CRDT ne stocke que les champs listés au §3. Les identifiants, types, relations, domaines et contrats restent en gouvernance.
- La présence est éphémère et ne transite pas par le document CRDT (§6).

### 2.3 Invariants vérifiés par des tests nommés

| Code | Invariant | Test |
|---|---|---|
| CR-001 | Le plan de gouvernance ne transite jamais par la CRDT. | `cr_001_governance_never_uses_crdt` |
| CR-002 | Une édition collaborative ne peut ni créer un objet, ni publier une version, ni clore une revue. | `cr_002_collab_cannot_create_object`, `cr_002_collab_cannot_publish_or_close_review` |
| CR-003 | Une fusion ne supprime jamais un objet ; au pire, elle le signale non conforme. | `merge_violation_does_not_delete` |
| CR-004 | La présence n'est jamais persistée. | `presence_is_never_persisted` |
| CR-005 | Deux éditions du même champ convergent vers une valeur unique et reproductible. | `cr_005_same_field_converges` |
| CR-006 | L'état fusionné est identique quel que soit l'ordre d'arrivée. | `cr_006_order_independent` |
| CR-007 | Un client hors ligne ne peut pas présenter son état comme publié. | `offline_state_is_not_presented_as_published` |

Deux précisions de vocabulaire :

- **CR-002** est interprété comme : une édition collaborative ne peut ni créer un objet, ni publier une version, ni changer un statut de revue (ouverture/fermeture). La formulation fonctionnelle « publier une version ni clore une revue » est un sous-ensemble ; l'extension « créer un objet » est structurelle et vérifiée par `cr_002_collab_cannot_create_object`.
- **« onglets et fenêtres »** désigne toute paire de contextes de même origine (onglets, fenêtres, PWA) capables de partager un `BroadcastChannel`. Le mode nominal couvre les deux.

## 3. Schéma CRDT par champ — contrat C-01

### 3.1 Identité et cycle de vie d'un document

```text
DocId = "doc:" + ULID de l'objet
```

Un document par objet collaboratif. Le document est créé **seulement si l'objet existe déjà** dans le snapshot. Le type de document est porté par `meta.doc_kind`.

| Champ racine | Type Yrs | Rôle |
|---|---|---|
| `meta` | `Y.Map` | métadonnées immuables ou monotones |
| `meta.schema` | `number` | version du schéma CRDT, valeur initiale `1` |
| `meta.object_id` | `string` | ULID de l'objet, vérifié à l'ouverture |
| `meta.doc_kind` | `string` | `OBJECT`, `DIAGRAM`, `STORY` |
| `name` | `Y.Text` | nom éditable de tout objet |
| `documentation` | `Y.Text` | documentation Markdown courante |
| `comments` | `Y.Array<Y.Map>` | commentaires ajoutés concurremment |
| `layout` | `Y.Map` | présent uniquement pour `DIAGRAM` |
| `story` | `Y.Map` | présent uniquement pour `STORY` |

Règles :

- `meta.object_id`, `meta.doc_kind` et les identifiants de nœuds/blocs sont **immuables** : une tentative de divergence est refusée (`COLLAB-IMMUTABLE`), jamais corrigée silencieusement.
- Un `schema` inconnu rend le document **lecture seule** (`COLLAB-SCHEMA`) ; reconstruction depuis le snapshot uniquement sur action explicite.
- Un document n'est jamais supprimé par une édition collaborative.
- Un objet archivé est en lecture seule : `CollabEdit` renvoie `COLLAB-ARCHIVED`.

### 3.2 Tableau par champ

| Chemin | Type Yrs | Source de vérité | Projection snapshot | Conflit | Interdit |
|---|---|---|---|---|---|
| `meta.schema` | `number` | document | — | LWW identique | non |
| `meta.object_id` | `string` | seed | vérification | immuable | non |
| `meta.doc_kind` | `string` | seed | — | immuable | non |
| `name` | `Y.Text` | CRDT | `RepositoryObject.name` | insertion CRDT | non |
| `documentation` | `Y.Text` | CRDT | `DocumentationBlock.markdown` | insertion CRDT | non |
| `comments` | `Y.Array<Y.Map>` | CRDT | `Dataset.comments` | ajout concurrent préservé ; ordre de projection `(created_at, id)` | suppression automatique |
| `comments[i].id` | `string` | client | `Comment.id` | immuable | non |
| `comments[i].author_id` | `string` | client | `Comment.author_id` | immuable | non |
| `comments[i].created_at` | `string` RFC 3339 | client | `Comment.created_at` | immuable | non |
| `comments[i].markdown` | `Y.Text` | CRDT | `Comment.markdown` | insertion CRDT | non |
| `layout.nodes` | `Y.Array<Y.Map>` | CRDT | `DiagramLayout.nodes` | ajout concurrent préservé ; doublon d'`id` ⇒ `MERGE_VIOLATION` | suppression automatique |
| `layout.nodes[i].id` | `string` | client | `DiagramNode.id` | immuable | non |
| `layout.nodes[i].object_id` | `string` | client | `DiagramNode.object_id` | immuable | non |
| `layout.nodes[i].kind` | `string` | client | `DiagramNode.kind` | immuable | non |
| `layout.nodes[i].version_id` | `string | null` | client | `DiagramNode.version_id` | LWW | non |
| `layout.nodes[i].position` | `string JSON {"x":number,"y":number}` | CRDT | `DiagramNode.x` / `.y` | LWW **atomique** sur une seule clé : jamais un mélange x/y | non |
| `layout.nodes[i].style_override` | `Y.Map<string, string JSON>` | CRDT | `DiagramNode.style_override` | LWW par clé | non |
| `layout.edges` | `Y.Array<Y.Map>` | CRDT | `DiagramLayout.edges` | ajout concurrent préservé | suppression automatique |
| `layout.edges[i].id` | `string` | client | `DiagramEdge.id` | immuable | non |
| `layout.edges[i].relationship_id` | `string` | client | `DiagramEdge.relationship_id` | immuable | non |
| `layout.edges[i].waypoints` | `Y.Array<Y.Map{x,y}>` | CRDT | `DiagramEdge.waypoints` | ajout/suppression convergents ; ordre du tableau | non |
| `layout.edges[i].label_visible` | `boolean` | CRDT | `DiagramEdge.label_visible` | LWW | non |
| `layout.annotations` | `Y.Array<Y.Map>` | CRDT | `DiagramLayout.annotations` | ajout concurrent préservé | suppression automatique |
| `layout.annotations[i].id` | `string` | client | `Annotation.id` | immuable | non |
| `layout.annotations[i].text` | `Y.Text` | CRDT | `Annotation.text` | insertion CRDT | non |
| `layout.annotations[i].x`, `.y`, `.w`, `.h` | `number` | CRDT | champs homonymes | LWW par clé | non |
| `story.chapters` | `Y.Array<Y.Map>` | CRDT | `StoryStructure.chapters` | ajout/déplacement/suppression convergents ; ordre du tableau | suppression automatique d'un chapitre non vide |
| `story.chapters[i].id` | `string` | client | `Chapter.id` | immuable | non |
| `story.chapters[i].title` | `Y.Text` | CRDT | `Chapter.title` | insertion CRDT | non |
| `story.chapters[i].blocks` | `Y.Array<Y.Map>` | CRDT | `Chapter.blocks` | ajout/déplacement convergents | non |
| `story.chapters[i].blocks[j].id` | `string` | client | `StoryBlock::id` | immuable | non |
| `story.chapters[i].blocks[j].kind` | `string` | client | variante du bloc | immuable ; changer de type = supprimer + ajouter, refusé | non |
| `…MARKDOWN.markdown` | `Y.Text` | CRDT | `StoryBlock::Markdown.markdown` | insertion CRDT | non |
| `…DIAGRAM.diagram_id` | `string` | client | `StoryBlock::Diagram.diagram_id` | immuable | non |
| `…DIAGRAM.framing.zoom` | `number | null` | CRDT | `DiagramFraming.zoom` | LWW | non |
| `…DIAGRAM.framing.focus_element_ids` | `Y.Array<string>` | CRDT | `DiagramFraming.focus_element_ids` | ajout/suppression convergents | non |
| `…DIAGRAM.framing.dimmed_element_ids` | `Y.Array<string>` | CRDT | `DiagramFraming.dimmed_element_ids` | ajout/suppression convergents | non |
| `…DIAGRAM.framing.caption` | `Y.Text` | CRDT | `DiagramFraming.caption` | insertion CRDT | non |
| `…VIEW.view` | `string` | client | `StoryBlock::View.view` | immuable | non |
| `…VIEW.caption` | `Y.Text` | CRDT | `StoryBlock::View.caption` | insertion CRDT | non |
| `…ELEMENT.element_id` | `string` | client | `StoryBlock::Element.element_id` | immuable | non |
| `…ELEMENT.highlight` | `boolean` | CRDT | `StoryBlock::Element.highlight` | LWW | non |
| `…INTERFACE.interface_id` | `string` | client | `StoryBlock::Interface.interface_id` | immuable | non |
| `…INTERFACE.version_id` | `string | null` | client | `StoryBlock::Interface.version_id` | LWW | non |
| `…TRANSITION.label` | `Y.Text` | CRDT | `StoryBlock::Transition.label` | insertion CRDT | non |
| `…TRANSITION.target_block_id` | `string` | client | `StoryBlock::Transition.target_block_id` | LWW | non |

### 3.3 Champs explicitement interdits en CRDT

| Champ | Plan | Motif |
|---|---|---|
| `RepositoryObject.id`, `object_type`, `responsible_domain_id`, `lifecycle`, `revision`, `created_at`, `created_by`, `updated_at`, `updated_by` | gouvernance | identité et cycle de vie |
| `InterfaceVersion.contract_snapshot`, `contract_raw`, `contract_hash`, `publication_status`, `available_from`, `available_until`, `breaking_change`, `supersedes_version_id` | gouvernance | `CR-001`, `BR-010` |
| `Review.status`, `filter_snapshot`, `temporal_window_*`, `markdown_summary` | gouvernance | `CR-002` |
| `Dataset.elements`, `relationships`, `interfaces`, `interface_versions`, `interface_consumers`, `domains`, `audit`, `links` | gouvernance | création et publication |
| `StoryStructure.pinned_revisions` | gouvernance | épingle une révision publiée ; la CRDT ne fige pas un récit |
| `DiagramLayout.viewport` | UI locale | le viewport est propre à chaque fenêtre ; il ne doit pas sauter lors d'un pan concurrent |
| `Comment.id`, `target_object_id` | client / structure | identité du commentaire, jamais réécrite |

### 3.4 Encodage et évolution

- Encodage des mises à jour : binaire Yrs v1. Le nom exact des fonctions est vérifié au spike T7-00 ; le contrat porte sur l'encodage, pas sur l'API.
- Indices de texte : **unités UTF-16** à la frontière UI (sélection DOM), conversion centralisée dans `crates/collab`. Le spike couvre accents, emoji et caractères combinés.
- `schema = 1` pour le MVP2. Toute évolution est additive : nouveau champ avec valeur par défaut, jamais de réinterprétation d'un champ existant.
- Un document dont le `schema` est inconnu n'est jamais migré automatiquement.

## 4. `CollabEdit` — contrat C-02

```rust
pub type PeerId = String;   // identifiant de session, jamais persisté
pub type DocId = String;    // "doc:<ULID>"

#[derive(Debug, Clone, PartialEq)]
pub enum TextPath {
    Name,
    Documentation,
    Comment { comment_id: String },
    Annotation { annotation_id: String },
    ChapterTitle { chapter_id: String },
    BlockMarkdown { chapter_id: String, block_id: String },
    BlockCaption { chapter_id: String, block_id: String },
    BlockLabel { chapter_id: String, block_id: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommentDraft {
    pub id: ObjectId,        // ULID généré par la session, jamais un objet de gouvernance
    pub author_id: String,
    pub created_at: String,  // RFC 3339 UTC
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CollabEdit {
    Text {
        doc: DocId,
        path: TextPath,
        index: u32,      // unités UTF-16
        delete: u32,     // unités UTF-16
        insert: String,
    },
    NodeMove { doc: DocId, node_id: String, x: f64, y: f64 },
    NodeAdd { doc: DocId, node: DiagramNode },
    NodeRemove { doc: DocId, node_id: String },
    EdgeAdd { doc: DocId, edge: DiagramEdge },
    EdgeRemove { doc: DocId, edge_id: String },
    EdgeWaypoints { doc: DocId, edge_id: String, waypoints: Vec<Point> },
    AnnotationAdd { doc: DocId, annotation: Annotation },
    AnnotationRemove { doc: DocId, annotation_id: String },
    StoryChapterAdd { doc: DocId, index: u32, chapter: Chapter },
    StoryChapterRemove { doc: DocId, chapter_id: String },
    StoryChapterMove { doc: DocId, chapter_id: String, to_index: u32 },
    StoryBlockAdd { doc: DocId, chapter_id: String, index: u32, block: StoryBlock },
    StoryBlockRemove { doc: DocId, chapter_id: String, block_id: String },
    StoryBlockMove { doc: DocId, chapter_id: String, block_id: String, to_index: u32 },
    CommentAdd { doc: DocId, comment: CommentDraft },
}
```

Règles :

- `CollabEdit` n'a **aucune variante** permettant de créer un objet, une relation, une version ou de changer un statut de revue. C'est une garantie de compilation, pas une convention.
- `NodeMove` écrit une clé `position` **atomique** (`{"x":…,"y":…}`), pas deux clés séparées : deux déplacements concurrents convergent vers une position complète, jamais un mélange x de l'un et y de l'autre. `w` et `h` restent dans le snapshot et ne sont pas modifiés par `CollabEdit` au MVP2.
- `StoryBlockRemove` refuse un bloc référencé par une transition entrante (`COLLAB-REFERENCED`) : la CRDT ne casse pas silencieusement un récit.
- `StoryChapterRemove` refuse un chapitre non vide (`COLLAB-NON-EMPTY`) : les blocs doivent être déplacés ou supprimés explicitement avant le chapitre.
- Toute édition sur un objet archivé est refusée (`COLLAB-ARCHIVED`).
- Toute édition dont le `doc` ne correspond pas à l'objet est refusée (`COLLAB-DOC-MISMATCH`).
- Les erreurs sont typées dans `CollabError` : `DocNotFound`, `ObjectNotFound`, `FieldForbidden`, `InvalidPath`, `Immutable`, `Archived`, `Referenced`, `NonEmpty`, `Decode`, `Encode`, `Store`, `SchemaUnsupported`.

### 4.1 Surface du moteur

```rust
pub struct CollabEngine { /* ... */ }

impl CollabEngine {
    pub fn ensure_doc(&self, object_id: ObjectId, seed: CollabSeed) -> Result<DocId, CollabError>;
    pub fn apply(&mut self, edit: CollabEdit) -> Result<CollabChange, CollabError>;
    pub fn apply_remote(&mut self, message: &SyncMessage) -> Result<Option<MergeOutcome>, CollabError>;
    pub fn state_vector(&self, doc: &DocId) -> Result<Vec<u8>, CollabError>;
    pub fn update_since(&self, doc: &DocId, state_vector: &[u8]) -> Result<Vec<u8>, CollabError>;
    pub fn snapshot(&self, doc: &DocId) -> Result<CollabSnapshot, CollabError>;
    pub fn presence(&self) -> Vec<PresenceState>;
    pub fn expire_presence(&mut self, now_ms: u64) -> Vec<PeerId>;
}
```

`CollabSnapshot` est une vue normalisée (JSON à clés triées) utilisée pour les tests de convergence et la projection. Elle ne contient jamais `PeerId` ni présence.

### 4.2 Types auxiliaires et surface `Repository`

Ces types sont gelés avec C-02. Ils évitent que W3/W4 inventent la couture.

```rust
pub enum DocKind {
    Object,
    Diagram,
    Story,
}

pub struct CollabSeed {
    pub object_id: ObjectId,
    pub doc_kind: DocKind,
    pub name: String,
    pub documentation: String,
    pub comments: Vec<Comment>,
    pub layout: Option<DiagramLayout>,
    pub story: Option<StoryStructure>,
}

pub enum CollabField {
    Name,
    Documentation,
    Comments,
    Layout,
    Story,
}

pub struct CollabChange {
    pub doc: DocId,
    pub fields: Vec<CollabField>,
    pub bytes: usize,
    pub updated_at_ms: u64,
}

pub struct CollabSnapshot {
    pub doc: DocId,
    pub object_id: ObjectId,
    pub schema: u32,
    pub name: String,
    pub documentation: String,
    pub comments: Vec<Comment>,
    pub layout: Option<DiagramLayout>,
    pub story: Option<StoryStructure>,
}

pub enum CollabEvent {
    Applied(CollabChange),
    Remote { doc: DocId, bytes: usize },
    PresenceChanged(Vec<PresenceState>),
    MergeDiagnostics(Vec<MergeDiagnostic>),
}

pub struct MergeOutcome {
    pub applied: bool,
    pub change: Option<CollabChange>,
    pub diagnostics: Vec<MergeDiagnostic>,
}

pub enum ProjectionOutcome {
    Written { object_id: ObjectId, revision: Revision },
    Unchanged { object_id: ObjectId },
    Rejected { object_id: ObjectId, issues: Vec<ValidationIssue> },
}

impl Repository {
    pub fn ensure_collab_doc(&self, object_id: ObjectId) -> RepoResult<DocId>;
    pub fn apply_collab_edit(&self, edit: CollabEdit) -> RepoResult<CollabChange>;
    pub fn subscribe_collab(&self, on: Box<dyn Fn(CollabEvent) + 'static>) -> SubscriptionId;
    pub fn collab_presence(&self) -> Vec<PresenceState>;
    pub fn flush_collab(&self, doc: &DocId) -> RepoResult<ProjectionOutcome>;
    pub fn merge_diagnostics(&self) -> Vec<MergeDiagnostic>;
    /// Superposition lecture : nom CRDT si le document est chargé, sinon snapshot.
    pub fn display_name(&self, id: ObjectId) -> String;
}
```

Correspondance `ObjectType → DocKind` :

| Objet | `DocKind` | Champs collaboratifs |
|---|---|---|
| Domain, ModelElement, Relationship, Interface, Review | `Object` | `name`, `documentation`, `comments` |
| Diagram | `Diagram` | `name`, `documentation`, `comments`, `layout` |
| Story | `Story` | `name`, `documentation`, `comments`, `story` |
| InterfaceVersion | **aucun document** | contrat interdit en CRDT |

Le store `collab` (`CollabStore::list`) sert d'index des documents connus : il n'existe **pas** de document racine d'index séparé.

## 5. `SyncTransport` — contrat C-03

```rust
pub type PeerId = String;
pub type DocId = String;
pub type SubscriptionId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Disconnected,
    Local,
    Connected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncMessage {
    /// Mise à jour différentielle (encodage binaire Yrs).
    Update { doc: DocId, from: PeerId, payload: Vec<u8> },
    /// Rattrapage : le pair demande ce qui lui manque depuis son vecteur d'état.
    StateVector { doc: DocId, from: PeerId, payload: Vec<u8> },
    /// Présence : éphémère, jamais persistée.
    Awareness { from: PeerId, payload: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SyncError {
    #[error("transport indisponible")]
    Unavailable,
    #[error("message trop volumineux : {0} octets")]
    TooLarge(usize),
    #[error("encodage invalide")]
    Encoding,
    #[error("transport fermé")]
    Closed,
}

pub trait SyncTransport {
    fn send(&self, message: SyncMessage) -> Result<(), SyncError>;
    fn subscribe(&self, on: Box<dyn Fn(SyncMessage) + 'static>) -> SubscriptionId;
    fn unsubscribe(&self, id: SubscriptionId);
    fn peers(&self) -> Vec<PeerId>;
    fn status(&self) -> SyncStatus;
}
```

### 5.1 Enveloppe binaire

Tous les messages sont encodés dans une enveloppe binaire, pour éviter qu'un payload texte soit interprété par le canal :

```text
magic      : 4 octets  "AFR1"
kind       : u8        1 = Update, 2 = StateVector, 3 = Awareness
doc_len    : u16 LE    longueur de doc_id en octets UTF-8 (0 pour Awareness)
doc        : doc_len octets
from_len   : u16 LE    longueur de peer_id en octets UTF-8
from       : from_len octets
payload_len: u32 LE
payload    : payload_len octets
```

- `payload` d'un `Update` ou `StateVector` : encodage binaire Yrs v1.
- `payload` d'un `Awareness` : JSON UTF-8, schéma §6.
- Limite dure : 1 Mio par message. Un `Update` de plus de 64 Kio est journalisé en mode debug et refusé au-delà de 1 Mio.
- Aucune donnée de gouvernance ne transite par ce canal.

### 5.2 Implémentations

| Implémentation | Statut | Transport | Usage |
|---|---|---|---|
| `LocalBroadcastTransport` | **nominal MVP2** | `BroadcastChannel("archifier:collab:v1")` | deux fenêtres du même navigateur et de la même origine |
| `InProcessTransport` | tests | canal en mémoire, ordre et partitions contrôlables | convergence, retard, perte, reconnexion |
| `WebSocketTransport` | préparé, **non activé** | `RelaySocket` injecté (trait), relais en mémoire dans les tests | lot 3, invisible pour l'utilisateur |

`archifier:collab:v1` est la seule constante de canal. Le trait de C-03 (avec `SubscriptionId`, `unsubscribe`, `peers`, `status`) est la seule signature de référence ; le tableau §21.4 et ADR-014 sont amendés par ce contrat.

- `WebSocketTransport` ne doit être instancié par aucun chemin UI. Un test d'architecture vérifie que `crates/ui` ne le mentionne pas.
- `LocalBroadcastTransport` expose ses limites dans l'UI : même origine, même profil de navigateur, aucune frontière de sécurité.
- `status()` renvoie `Local` pour BroadcastChannel, `Connected` pour un relais actif, `Disconnected` si le canal est indisponible. Un transport indisponible ne bloque jamais l'édition locale : bandeau « synchronisation inactive ».

### 5.3 Rattrapage

1. À l'ouverture d'un document, chaque pair envoie `StateVector`.
2. Le pair qui reçoit répond `Update` avec seulement le delta manquant.
3. Un `Update` ne renvoie jamais le document complet à chaque frappe.
4. En cas de partition, les mises à jour locales restent dans le document et dans le store `collab` ; à la reconnexion, l'échange de vecteurs d'état suffit.
5. Aucun `Update` n'est appliqué deux fois : l'application est idempotente par construction CRDT.
6. « Hors ligne » signifie **aucun pair joignable / transport partitionné**, pas « pas de réseau » : l'application est déjà locale. L'édition reste complète hors ligne ; NFR-016 est vérifié par `offline_edits_merge_on_reconnect` sur `InProcessTransport`.

## 6. Présence — contrat C-04

La présence ne réside **pas** dans le document CRDT. Elle est portée par `Awareness` et un registre en mémoire.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceField {
    Name,
    Documentation,
    Layout,
    Story,
    Comments,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceState {
    pub peer_id: PeerId,
    pub display_name: String,
    pub object_id: Option<ObjectId>,
    pub field: Option<PresenceField>,
    pub cursor: Option<Cursor>,         // anchor/head en unités UTF-16
    pub color: String,                  // affectation déterministe par peer_id
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub anchor: u32,
    pub head: u32,
}

/// Charge utile de `SyncMessage::Awareness`, encodée en JSON UTF-8.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwarenessState {
    pub v: u32,                         // 1
    pub peer_id: PeerId,
    pub display_name: String,
    pub object_id: Option<ObjectId>,
    pub field: Option<PresenceField>,
    pub cursor: Option<Cursor>,
    pub color: String,
    pub updated_at_ms: u64,
    pub left: bool,
}
```

Exemple de payload :

```json
{
  "v": 1,
  "peer_id": "01J...",
  "display_name": "Régis",
  "object_id": "01J...",
  "field": "DOCUMENTATION",
  "cursor": { "anchor": 12, "head": 12 },
  "color": "#3b82f6",
  "updated_at_ms": 1763462400000,
  "left": false
}
```

Règles :

- `PeerId` est généré par session à partir de `crypto.getRandomValues` si disponible ; repli déterministe documenté pour les tests. Jamais persisté, jamais exporté, jamais inclus dans un snapshot.
- Battement toutes les 10 s. Expiration à 30 s sans battement.
- Un message `Awareness` avec `left: true` retire immédiatement le pair.
- Fermeture propre sur `pagehide` et `visibilitychange` (message `left` + flush).
- L'UI affiche des pastilles discrètes sur la fiche et le canvas ; pas de curseur intrusif par défaut.
- Un test d'export vérifie qu'aucun `peer_id`, `display_name` de présence ou `cursor` n'apparaît dans le snapshot.

## 7. Persistance — contrat C-05

### 7.1 Stores IndexedDB `collab` et `local_state`

- Base `archifier`, version **2** (la version 1 ne contenait que le store `snapshot`).
- Migration additive : création des stores `collab` et `local_state` s'ils sont absents ; le store `snapshot` n'est ni recréé, ni vidé, ni transformé.
- Clé du store `collab` : `doc_id` (`doc:<ULID>`). Enregistrement :

```text
{
  doc_id:    string,
  schema:    number,
  updated_at: number,   // ms UTC
  payload:   Uint8Array, // état Yrs encodé
  seeded:    boolean
}
```

- Store `local_state` : clé `key`, valeur `value` (chaîne). Il ne contient que des préférences locales et des acquittements de notifications. Il n'est **jamais** dans le snapshot, jamais exporté, jamais synchronisé.
- Aucun index secondaire n'est nécessaire au MVP2. L'index des documents connus est le store `collab` lui-même (`getAllKeys`).
- Les deux stores ne remplacent jamais le snapshot et ne participent jamais à sa transaction d'écriture.

### 7.2 Trait `CollabStore`

```rust
pub struct CollabRecord {
    pub doc_id: DocId,
    pub schema: u32,
    pub updated_at_ms: u64,
    pub payload: Vec<u8>,
    pub seeded: bool,
}

pub trait CollabStore {
    /// Chargement paresseux : callback appelé une fois, jamais de blocage de l'UI.
    fn load(&self, doc: &DocId, on: Box<dyn FnOnce(Result<Option<CollabRecord>, StoreError>)>);
    fn save(&self, record: CollabRecord) -> Result<(), StoreError>;
    fn delete(&self, doc: &DocId) -> Result<(), StoreError>;
    fn list(&self) -> Vec<DocId>;
}

pub trait LocalStateStore {
    fn get(&self, key: &str) -> Option<String>;
    fn put(&self, key: &str, value: &str) -> Result<(), StoreError>;
    fn delete(&self, key: &str) -> Result<(), StoreError>;
}
```

- `MemoryCollabStore` pour les tests natifs.
- `IdbCollabStore` pour le navigateur, dans `crates/repo/src/idb.rs` (ou un module dédié `idb_collab.rs`).
- Un enregistrement illisible n'est jamais supprimé silencieusement : il est déplacé en quarantaine (`quarantine:<doc>:<timestamp>`) et l'UI affiche un avertissement avec proposition de reconstruction depuis le snapshot.

### 7.3 Démarrage

```text
1. ouvrir la base v2 (migration additive)
2. charger la liste des DocId connus
3. à l'ouverture d'un objet éditable :
   a. si le document existe : décoder
   b. sinon : créer, initialiser depuis le snapshot, marquer seeded = true, sauvegarder
   c. si le décodage échoue : quarantaine + reconstruction explicite, jamais silencieuse
4. s'abonner au transport et envoyer les StateVector des documents ouverts
5. démarrer le battement de présence
6. brancher le flush débouncé
```

- Initialisation idempotente : un enregistrement `seeded = true` dans `collab` n'est jamais réinitialisé depuis le snapshot, même si les deux divergent. `CollabRecord.seeded` est le seul drapeau ; il n'est pas dupliqué dans le document.
- `NFR-020` : ouvrir un domaine ne charge pas les documents de ses 200 éléments. Seuls les objets affichés / édités sont chargés.
- Le chargement d'un document cible < 30 ms pour 20 000 caractères (mesure spike).

### 7.4 Flush et compaction

- Flush disque débouncé à **400 ms** après la dernière édition locale.
- Flush forcé sur `visibilitychange`, `pagehide`, navigation interne et fermeture.
- Compaction au repos : après **2 s** sans édition ou **50 mises à jour**, l'état complet est réencodé et remplace atomiquement `payload`.
- La compaction est purement locale : elle ne change pas l'état logique et n'oblige aucun pair à renvoyer le document.
- Si le quota est dépassé, l'écriture échoue avec `StoreError::QuotaExceeded` ; l'UI propose l'export et ne supprime jamais un document automatiquement.

## 8. Projection vers le snapshot et historique — contrat C-06

### 8.1 Lecture : superposition

La CRDT est la source de vérité des champs collaboratifs. Les lectures UI passent par des accesseurs qui superposent le document chargé au snapshot :

| Lecture | Source |
|---|---|
| `Repository::display_name(id)` | `name` CRDT si document chargé, sinon `RepositoryObject.name` |
| `Repository::documentation(id)` | `documentation` CRDT si chargé, sinon dernier `DocumentationBlock` |
| `Repository::diagram_detail(id)` | `layout` CRDT si chargé, sinon `Diagram.layout` |
| `Repository::story_detail(id)` / `playback(id)` | `story` CRDT si chargé, sinon `Story.structure` |
| commentaires d'une revue / d'un objet | `comments` CRDT si chargé, sinon `Dataset.comments` |

Une vue ne lit jamais directement IndexedDB. Elle ne connaît que le type `Repository`, via `state::repo()` (ADR-016).

### 8.2 Écriture : projection explicite

```rust
pub enum ProjectionTarget {
    Name { object_id: ObjectId, value: String },
    Documentation { object_id: ObjectId, markdown: String },
    Comments { target_object_id: ObjectId, comments: Vec<Comment> },
    Layout { diagram_id: ObjectId, layout: DiagramLayout },
    Story { story_id: ObjectId, structure: StoryStructure },
}

pub struct ProjectionCause {
    pub peer_id: PeerId,
    pub actor_id: String,
    pub burst_id: String,
}
```

- `ProjectionTarget` ne contient **aucune** variante interface, version, contrat, revue ou relation. La projection ne peut donc pas publier ni changer un statut (`CR-002`).
- La projection écrit dans le snapshot par un chemin dédié, **jamais** par `apply_changeset` et **jamais** dans la même transaction que le store `collab`.
- `Story` préserve `pinned_revisions` du snapshot : la CRDT ne possède pas ce champ.
- `Layout` ne projette pas `viewport`.
- La projection est idempotente par empreinte de contenu : si le snapshot contient déjà la valeur, aucune révision n'est ajoutée.
- Un échec de projection ne perd pas l'édition : le document CRDT reste la source de vérité et sera reprojeté au prochain flush.
- `ProjectionCause` marque l'origine `COLLAB_PROJECTION` (acteur, `peer_id`, `burst_id`). La détection d'écriture externe de T1-08 ignore les projections de la même session et ne déclenche pas le dialogue recharger / comparer / forcer. Test : `collab_projection_does_not_trigger_external_write_dialog`.
- La projection incrémente `revision` et `updated_at` **uniquement si le contenu projeté a changé**. Une écriture de gouvernance concurrente utilise la nouvelle révision ; un `409` reste possible et n'est jamais résolu par la CRDT. Test : `governance_write_after_projection_uses_new_revision`.

### 8.3 Regroupement d'historique (`FR-038`)

| Paramètre | Valeur |
|---|---|
| Fenêtre d'inactivité | 2 000 ms |
| Nombre maximal d'éditions par rafale | 30 |
| Période hors ligne / partition | une seule rafale pour toute la période, plafonnée à 30 éditions ; flush à la reconnexion |
| Flush explicite | navigation, `visibilitychange`, `pagehide` |
| Résumé de révision | « Édition collaborative » |
| Auteur | utilisateur de la session |
| Contenu identique | aucune révision |

- Une rafale produit **une** révision, pas une par frappe.
- Deux rafales séparées par plus de 2 s produisent deux révisions.
- Un flush explicite clôt la rafale courante.
- La révision est écrite par la projection, dans le plan gouvernance, mais **pas** par la CRDT : c'est une conséquence explicite et séparée.
- Aucune révision n'est écrite pour une simple présence, un déplacement annulé ou un contenu inchangé.

## 9. Validation après fusion — contrat C-07

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct MergeDiagnostic {
    pub id: String,
    pub doc: DocId,
    pub object_id: ObjectId,
    pub code: &'static str,          // "MERGE_VIOLATION"
    pub issues: Vec<ValidationIssue>, // codes OBJ-*, BR-*, MM-*, DGM-* (dont DGM-004), STORY-*
    pub peer_id: Option<PeerId>,
    pub detected_at_ms: u64,
}
```

Procédure :

```text
apply(update distant)
  └─ reconstruire la vue candidate depuis le document fusionné
     └─ archifier_core::validate_*() sur la vue concernée
        ├─ conforme      → projection autorisée
        └─ non conforme  → MergeDiagnostic local (jamais persisté)
                         + objet marqué « non conforme » dans l'UI
                         + aucune suppression, aucune correction automatique
```

- Le diagnostic est **local et éphémère** : il n'est ni dans le snapshot, ni dans le store `collab`, ni exporté.
- L'UI liste les codes de règle violés et, si identifiable, le pair à l'origine de la branche.
- L'utilisateur arbitre : annuler sa propre édition, archiver, ou corriger. Le système ne décide pas.
- Cas couverts : nom vide ou hors bornes, nœud dupliqué, nœud référençant un objet absent/archivé, bloc de Story référençant un objet absent, transition vers un bloc absent, suppression d'un chapitre non vide refusée (`COLLAB-NON-EMPTY`).
- **Relations incompatibles** : une relation est un objet de gouvernance. La seconde écriture concurrente est refusée par le verrou optimiste (`409`) ou par la validation de métamodèle, et aucune relation n'est supprimée. Le diagnostic `MERGE_VIOLATION` est réservé aux vues collaboratives fusionnées (layout, Story) ; il ne justifie jamais de faire transiter une relation par la CRDT (`CR-001`). Test d'acceptation : `incompatible_relations_are_refused_without_deletion`.

## 10. OpenAPI — adaptations du modèle

### 10.1 Champs additifs de `InterfaceVersion`

```rust
pub struct InterfaceVersion {
    // … champs existants …
    #[serde(default)]
    pub contract_format: Option<ContractFormat>,
    #[serde(default)]
    pub contract_raw: Option<String>,      // document brut, texte exact
    #[serde(default)]
    pub contract_hash: Option<String>,     // empreinte canonique hexadécimale
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContractFormat {
    OpenApi30,
    OpenApi31,
    OpenApi32,
    AsyncApi2,
    AsyncApi3,
}
```

- Migration additive : les snapshots existants désérialisent avec `None`.
- `contract_snapshot` (normalisé) reste présent ; `contract_raw` conserve l'octet-à-octet pour l'export et la réimportation.
- `contract_hash` est l'empreinte SHA-256 du JSON canonique (clés triées, sans espaces significatifs). Un import du même document ne crée ni version ni interface (`OAS-050`).
- L'empreinte **exclut récursivement** le namespace `x-architecture-*` : un export enrichi puis réimporté produit la même empreinte (ADR-018).
- `sourceFingerprint` (spécification fonctionnelle) et `contract_hash` désignent le même champ ; le nom de code est `contract_hash`.
- L'import accepte JSON et YAML (`serde_yaml_ng 0.10`, approuvé en G1) ; le document brut est conservé sous sa forme textuelle d'origine.
- `sha2 0.10` est déjà verrouillé dans le graphe (0.10.9) ; aucune nouvelle surface transitive n'est introduite.

### 10.2 Codes de règle proposés

| Code | Signification | Classement diff |
|---|---|---|
| `OAS-001` | version OpenAPI non supportée | erreur |
| `OAS-002` | document illisible | erreur |
| `OAS-003` | `operationId` dupliqué | erreur |
| `OAS-010` | chemin ou opération supprimé | rupture |
| `OAS-011` | réponse supprimée | rupture |
| `OAS-012` | type de schéma changé | rupture |
| `OAS-013` | paramètre requis ajouté | rupture |
| `OAS-014` | schéma de réponse restreint | rupture |
| `OAS-020` | description / résumé modifié | compatible |
| `OAS-021` | champ optionnel ajouté | compatible |
| `OAS-030` | `$ref` non résolu | inconnu |
| `OAS-031` | extension `x-*` modifiée | inconnu |
| `OAS-040` | rupture non confirmée | blocage |
| `OAS-050` | empreinte déjà importée | idempotence |
| `OAS-060` | export demandé pour une interface non HTTP | refus |

### 10.3 Export reproductible

- Réservé aux interfaces `APPLICATION` avec `contract_type = OpenApi` et un contrat HTTP.
- Extensions : `x-architecture-interface-id`, `x-architecture-version-id`, `x-architecture-domain-id`, `x-architecture-logical-key`, `x-architecture-version`, `x-architecture-generated-by`.
- Aucun horodatage volatile : toute date vient de `created_at` / `available_from`, jamais de l'horloge d'export.
- Ordre des clés et indentation fixés ; deux exports de la même révision sont identiques octet-à-octet (`NFR-009`).

## 11. AsyncAPI — adaptations du modèle

- `Interface.contract_type = Event` ; `contract_format = AsyncApi2 | AsyncApi3`.
- Même conservation du brut que OpenAPI : `contract_raw`, `contract_snapshot`, `contract_hash`.
- Le modèle normalisé couvre : `info`, `servers`, `channels`, `operations`, `messages`, `schemas`, `security`, `bindings` connus. Les bindings inconnus restent dans le brut.
- Import : détection de version par `asyncapi` (2.x / 3.x), erreurs `ASYNC-001` à `ASYNC-003`.
- Export : reproductible, extensions `x-architecture-*`, réservé aux interfaces événementielles.
- Diff : suppression de canal / message = rupture ; ajout de champ optionnel = compatible ; `$ref` non résolu = inconnu.
- Aucun objet de référentiel supplémentaire n'est créé : les canaux et messages restent dans le contrat, pas dans la palette.

## 12. Cycle de vie et notifications

### 12.1 Politique de cycle de vie

Champs additifs sur `InterfaceVersion` :

```rust
#[serde(default)]
pub deprecation: Option<DeprecationPolicy>,

pub struct DeprecationPolicy {
    pub announced_at: OffsetDateTime,
    pub retirement_at: OffsetDateTime,
    pub replacement_version_id: Option<ObjectId>,
    pub breaking_policy: BreakingPolicy,
}

pub enum BreakingPolicy {
    Announce,             // rupture annoncée
    RequireConfirmation,  // rupture exige une confirmation explicite
    Forbid,               // rupture interdite pendant la fenêtre
}
```

Règles proposées :

| Code | Règle |
|---|---|
| `IFACE-010` | `retirement_at` doit être ≥ `available_from` |
| `IFACE-011` | `retirement_at` doit être ≤ `available_until` |
| `IFACE-012` | `replacement_version_id` doit exister et appartenir à la même interface |
| `IFACE-013` | une rupture exige une `BreakingPolicy` explicite |
| `IFACE-014` | une version dépréciée encore consommée sans acquittement est signalée |
| `IFACE-015` | un consommateur orphelin est signalé |
| `IFACE-016` | une rupture non consommée est signalée |

Les règles de fenêtre ne s'appliquent que lorsque les bornes optionnelles existent : un brouillon peut ne pas définir `available_from` / `available_until`. Une version publiée a les deux bornes (`BR-001`), donc `IFACE-010`/`IFACE-011` s'appliquent pleinement à ce moment-là. `announced_at` peut précéder `available_from`.

Toute évolution est additive (`serde(default)`), sans changement d'identifiant.

### 12.2 Centre de notifications

- Liste **dérivée** : échéances, expirations, ruptures, liens cassés, retraits proches.
- Aucun workflow d'approbation, aucun statut persistant de type « approuvé ».
- Les acquittements / masquages locaux sont stockés dans le store `local_state` (IndexedDB, créé en v2), jamais dans le snapshot et jamais exportés.
- Un acquittement ne modifie jamais l'objet source.

## 13. Migration de métamodèle 3.2 → 4.x

- Fichier `crates/metamodel/mappings/archimate-3.2-to-4.x.toml` :
  - `source_profile = "archimate:3.2"`
  - `target_profile = "archimate:4.x"`
  - `status = "unpublished"`
  - `classes = []` tant que The Open Group n'a pas publié de spécification.
  - `default_policy = "preserve"` (un objet non mappé conserve son type et son identifiant).
- Outil de diagnostic : pour chaque objet, indique `compatible`, `mapped`, `unmapped`, `requires_review`. Aucun objet n'est migré automatiquement.
- Aucune classe `archimate:4.*` n'est déclarée dans un profil chargeable. Un test échoue si un identifiant de classe commence par `archimate:4`.
- `docs/metamodel-coverage.md` est généré par `coverage_report()` ; à ce stade il donne les compteurs par profil. W2 l'étend aux classes, relations, couverture et lacunes (ADR-021).
- T3-05 : la palette et l'inspecteur lisent le profil au runtime. Ajouter une classe dans un TOML doit suffire ; un test charge un profil synthétique et vérifie la palette et les propriétés sans modification de code.

## 14. IA locale et proxy

### 14.1 Frontière

- `archifier-ai` ne dépend pas de l'UI et ne reçoit qu'un contexte en lecture.
- `MockProvider` est déterministe, exécute réellement les outils sur le dépôt, et annonce ses recettes : impact, consommateurs, proposition de vue, recherche conceptuelle, explication de règle.
- Un repli honnête liste les scénarios supportés ; il ne prétend jamais être un LLM.
- Toute réponse non sourcée porte la mention explicite « réponse non sourcée ».
- Aucune écriture directe : toute proposition passe par un `ChangeSet`, une validation déterministe, puis « Valider » et « Appliquer » distincts.
- `STALE` si une révision a bougé entre la proposition et l'application.

### 14.2 Proxy `archifier-proxy`

| Contrainte | Valeur |
|---|---|
| Binaire | Rust, port 8787 |
| Clé | variable d'environnement uniquement, jamais dans le bundle, jamais en `localStorage` |
| CORS | `http://127.0.0.1:4173` uniquement |
| Journalisation | aucun prompt par défaut |
| Absence | proxy désactivé, l'UI le signale, aucune requête réseau |
| Activation | explicite, locale, jamais par défaut |

Le proxy est un crate séparé, non dépendance de `archifier-ui`. Un test vérifie qu'aucune chaîne ressemblant à une clé (`sk-…`, `Bearer …`) n'est présente dans `pkg/`.

## 15. Port de dépôt — contrat C-08 (décision G1 : option B)

### 15.1 Constat

Au 18/11/2026 :

- `archifier-ui` importe `archifier_repo::Repository` (type concret) dans `state.rs`.
- `boot.rs` importe `archifier_repo::idb::IdbStore` pour le câblage initial.
- Aucune vue n'importe `idb` : l'accès direct à IndexedDB est déjà limité au démarrage.
- Il n'existe ni trait `RepositoryPort`, ni type `WriteTx`.

### 15.2 Décision : le type `Repository` est le port

L'arbitrage G1 retient **l'option B**, formalisée par `docs/adr/ADR-016-port-depot-type-repository.md` :

1. L'API publique de `archifier_repo::Repository` **est** le contrat de dépôt du MVP2. Elle est gelée pour la durée du lot 2.
2. `crates/ui/src/state.rs` expose `repo()` et reste le **seul** point d'accès de l'UI au dépôt.
3. Aucune vue n'importe `Repository`, `idb`, `SnapshotStore` ou `CollabStore`.
4. `boot.rs` est la **racine de composition normale** : c'est le seul fichier autorisé à connaître `IdbStore`, `MemoryStore` pour le démarrage réel et, au lot 2, `IdbCollabStore`. Exception anti-panic documentée : `state::repo()` peut construire un dépôt vide en mémoire si `install` n'a pas eu lieu (ADR-016 §3).
5. Le chemin d'écriture de gouvernance reste `Repository::mutate` ; `WriteTx` demeure une cible de nommage, pas un type du MVP2.
6. Il n'y a **pas** d'extraction de trait en W1. Toute demande d'évolution de l'API publique passe par l'orchestrateur.

### 15.3 Invariants et tests d'architecture

| Invariant | Test |
|---|---|
| Aucune vue n'importe le stockage | `ui_views_do_not_touch_storage` |
| `boot.rs` est la seule racine qui connaît `IdbStore` | `idb_store_is_only_known_by_boot` |
| Le repli mémoire est une exception documentée | `memory_fallback_is_documented_exception` |
| Aucune vue ne construit un `Repository` | inclus dans `ui_views_do_not_touch_storage` (interdiction `Repository::new` et `archifier_repo::Repository`) |
| Toute nouvelle méthode publique utilisée par l'UI a un test | revue de porte G2/G4b |

### 15.4 Conséquence pour le lot 3

Un futur `HttpRepository` ne sera pas substituable par simple changement de type. Il devra soit exposer la même API publique que `Repository`, soit être caché derrière une façade évaluée dans un ADR ultérieur. Cette dette est acceptée et documentée dans ADR-016.

## 16. Virtualisation du canvas et budgets

- `ADR-020` : virtualisation du canvas.
- Rendu SVG : seuls les nœuds et arêtes **visibles** (viewport + marge) sont montés ; les nœuds sélectionnés et leurs voisins directs restent montés.
- Les arêtes dont les deux extrémités sont hors viewport ne sont pas rendues.
- Au-delà de 200 nœuds, un niveau de détail réduit les libellés à la sélection et au survol.
- Le layout reste la source de vérité ; la virtualisation est purement un rendu.
- Budgets : 500 nœuds à 60 fps, fiche < 100 ms, recherche < 50 ms, frappe collaborative < 16 ms perçue, bundle ≤ 4 Mo gzip.
- Aucune dépendance de virtualisation : le culling est un calcul de rectangles.

## 17. Tests et portes

### 17.1 Pyramide

| Niveau | Emplacement | Contenu |
|---|---|---|
| Unitaire moteur | `#[cfg(test)]` dans `crates/core`, `crates/collab`, `crates/metamodel` | règles, convergence, encodage, validation |
| Intégration dépôt | `crates/repo/tests/**` | seed idempotent, projection, histoire, IndexedDB simulé |
| Transport | `crates/collab/tests/**` | `InProcessTransport`, partitions, vecteurs d'état, présence |
| Architecture | `crates/*/tests/**` | pas d'`idb` dans les vues, pas de clé, pas de classe 4, pas de transport WS dans l'UI |
| E2E | `tests/e2e/**` | 4 chaînes de recette, 8 Gherkin, 5 scénarios collaboration, deux fenêtres |

### 17.2 Scénarios de collaboration E2E

1. Deux fenêtres éditent la même documentation : les deux paragraphes sont présents, aucune frappe perdue.
2. Déplacement concurrent d'un nœud : même position finale, aucun nœud dupliqué.
3. Hors ligne puis fusion : trois blocs de Story fusionnés, une seule révision regroupée.
4. Relations incompatibles : seconde écriture refusée (`409` ou validation de métamodèle), aucune relation supprimée ; `MERGE_VIOLATION` testé séparément sur une vue collaborative fusionnée.
5. Présence : l'autre fenêtre voit qui édite, la présence disparaît à la fermeture.

### 17.3 Commandes de porte

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p archifier-ui --target wasm32-unknown-unknown
.\scripts\build.cmd
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\spike-bundle.ps1
npx playwright install chromium
npx playwright test
```

Note d'environnement : `.\scripts\build.cmd` est le wrapper reproductible ; il appelle `build.ps1` avec `-ExecutionPolicy Bypass` sans modifier la politique de la machine.

### 17.4 Résultats du spike T7-00 (18/11/2026)

| Porte | Seuil | Mesure | Verdict |
|---|---|---|---|
| 20 000 caractères + 200 insertions concurrentes | aucune perte | 20 200 caractères, 200 marqueurs uniques, réplicas identiques | ✅ |
| Taille d'une frappe | < 2 Kio | 17 octets (mot + emoji : 21) | ✅ |
| Décodage + application 20 k | < 30 ms | release 27,3 µs / 32,6 µs / 87,9 µs (min/méd/max) ; debug 0,85–1,81 ms | ✅ |
| Delta WASM brut | < +900 Kio | `Y.Text` +296,9 Kio ; schéma C-01 +332,3 Kio | ✅ |

Aucun repli Automerge. Le détail et les commandes sont dans `crates/collab/README.md` ; la vérification automatisée du budget bundle est `scripts/spike-bundle.ps1`.

## 18. Décisions G1 et décisions différées

| Réf. | Décision | Statut |
|---|---|---|
| D-01 | `RepositoryPort` : **option B**, le type `Repository` est le port, centralisé par `state::repo()` (ADR-016) | ✅ rendu |
| D-02 | Normalisation `fmt` / `clippy` en W0, orchestrateur seul, aucun changement de comportement | ✅ fait |
| D-03 | Suppression des deux `panic!` hors tests : profils illisibles + repli anti-panic | ✅ fait |
| D-04 | Scénario « relations incompatibles » : **option A**, la seconde écriture est refusée par la gouvernance (`409`/validation), aucune relation supprimée ; `MERGE_VIOLATION` reste réservé aux vues collaboratives fusionnées | ✅ rendu |
| D-05 | ADR-015 : matérialisé le 18/11/2026 (`docs/adr/ADR-015-transport-synchronisation.md`) | ✅ fait |
| D-06 | Parseur YAML : `serde_yaml_ng = "0.10"` autorisé | ✅ rendu |
| D-07 | `sha2 = "0.10"` autorisé (déjà verrouillé en 0.10.9) | ✅ rendu |
| D-08 | `local_state` IndexedDB pour les acquittements de notifications : inclus dans la migration v2, jamais exporté | ✅ figé |
| D-09 | Champs `contract_raw` / `contract_hash` / politique de cycle de vie : ajouts additifs `serde(default)`, ADR-019 | à figer W1/W5 |
| D-10 | Virtualisation du canvas : ADR-020, culling sans dépendance | à figer W7 |
| D-11 | `yrs` épinglé en `0.25` : 0.27/0.28 échouent en E0658 sur Rust 1.94, 0.25 compile et passe T7-00 | ✅ fait |

## Historique

| Version | Date | Changement |
|---|---|---|
| 0.1 | 18/11/2026 | Squelette G1 : schéma CRDT par champ, `CollabEdit`, `SyncTransport`, présence, persistance, projection, fusion, OpenAPI/AsyncAPI, cycle de vie, migration, IA, port de dépôt. |
| 0.2 | 18/11/2026 | Arbitrages G1 intégrés : port de dépôt = type `Repository` (ADR-016), relations incompatibles = 409 + `MERGE_VIOLATION`, `serde_yaml_ng` et `sha2` approuvés. |
| 0.3 | 18/11/2026 | G1 franchi : schéma CRDT complété, types auxiliaires gelés, présence/wire, stores v2, projection et historique, spike T7-00 mesuré, ADR-011/012/013 matérialisés. |
