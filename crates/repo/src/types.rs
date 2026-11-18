//! Types de lecture exposés à l'interface. Aucun ne porte de logique métier :
//! ils sont dérivés de l'état validé du dépôt.

use archifier_core::{
    ContractDiff, ContractFormat, Diagram, DiagramNotation, FieldDifference, Interface,
    InterfaceCategory, InterfaceVersion, Lifecycle, Link, LinkStatus, NodeKind, Notation, ObjectId,
    ObjectType, PublicationStatus, ReviewStatus, Revision, TemporalStatus, ViewFraming, ViewIntent,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainSummary {
    pub id: ObjectId,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub lifecycle: Lifecycle,
    pub counts: DomainCounts,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainCounts {
    pub elements: usize,
    pub interfaces: usize,
    pub diagrams: usize,
    pub stories: usize,
    pub reviews: usize,
    pub decisions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainOverview {
    pub domain: DomainSummary,
    pub published_diagrams: Vec<ObjectRef>,
    pub exposed_interfaces: Vec<InterfaceRow>,
    pub consumed_interfaces: Vec<InterfaceRow>,
    pub dependencies_out: Vec<DomainLink>,
    pub dependencies_in: Vec<DomainLink>,
    pub stories: Vec<ObjectRef>,
    pub reviews: Vec<ObjectRef>,
    pub anomalies: Vec<Anomaly>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectRef {
    pub id: ObjectId,
    pub object_type: ObjectType,
    pub name: String,
    pub subtitle: Option<String>,
    pub href: String,
}

impl ObjectRef {
    pub fn of(object_type: ObjectType, id: ObjectId, name: impl Into<String>) -> Self {
        Self {
            id,
            object_type,
            name: name.into(),
            subtitle: None,
            href: format!("#/view/{}/{}", object_type.route_segment(), id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainLink {
    pub domain_id: ObjectId,
    pub domain_name: String,
    pub weight: usize,
    pub via: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceRow {
    pub id: ObjectId,
    pub name: String,
    pub logical_key: String,
    pub category: InterfaceCategory,
    pub provider_domain_id: ObjectId,
    pub provider_domain_name: String,
    pub lifecycle: Lifecycle,
    pub status: Option<TemporalStatus>,
    pub active_versions: usize,
    pub total_versions: usize,
    pub breaking_versions: usize,
    pub consumers: usize,
    pub next_change: Option<OffsetDateTime>,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionSummary {
    pub version: InterfaceVersion,
    pub status: Option<TemporalStatus>,
    pub consumer_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineSegment {
    pub version_id: ObjectId,
    pub semantic_version: String,
    pub status: Option<TemporalStatus>,
    pub publication_status: PublicationStatus,
    pub from: Option<OffsetDateTime>,
    pub until: Option<OffsetDateTime>,
    pub consumers: usize,
    pub breaking_change: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineOverlap {
    pub from: OffsetDateTime,
    pub until: OffsetDateTime,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub interface_id: ObjectId,
    pub interface_name: String,
    pub now: OffsetDateTime,
    pub segments: Vec<TimelineSegment>,
    pub overlaps: Vec<TimelineOverlap>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Direction {
    In,
    Out,
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DependencyNode {
    pub object_id: ObjectId,
    pub name: String,
    pub object_type: ObjectType,
    pub domain_id: ObjectId,
    pub domain_name: String,
    pub depth: usize,
    pub path: Vec<ObjectId>,
    pub relation: String,
    /// Sens réel de l'arête depuis le nœud courant (`IN` ou `OUT`).
    pub direction: Direction,
    /// Cycle de vie de la cible : une cible archivée reste visible et signalée.
    pub lifecycle: Lifecycle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub root: ObjectId,
    pub nodes: Vec<DependencyNode>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usages {
    pub object_id: ObjectId,
    pub diagrams: Vec<ObjectRef>,
    pub stories: Vec<ObjectRef>,
    pub interfaces: Vec<ObjectRef>,
    pub reviews: Vec<ObjectRef>,
    pub inbound: Vec<DependencyNode>,
    pub outbound: Vec<DependencyNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub object_id: ObjectId,
    pub object_type: ObjectType,
    pub title: String,
    pub excerpt: String,
    pub domain_name: String,
    pub score: f32,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisionMeta {
    pub revision: u64,
    pub created_at: OffsetDateTime,
    pub created_by: String,
    pub summary: String,
}

/// Diff entre deux révisions d'un même objet (T1-04).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisionDiff {
    pub object_id: ObjectId,
    pub from: Revision,
    pub to: Revision,
    /// Différences triées par pointeur JSON.
    pub changes: Vec<FieldDifference>,
}

/// Lien de documentation à réparer (T1-06).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrokenLink {
    /// `DOC-LINK-001` (cassé) ou `DOC-LINK-002` (archivé).
    pub code: String,
    pub link: Link,
    pub source: ObjectRef,
    /// Domaine responsable de la source, `None` si introuvable.
    pub owner: Option<ObjectRef>,
    /// `None` quand l'objet cible n'existe pas.
    pub target: Option<ObjectRef>,
    /// 80 caractères avant et 80 après l'URI dans la ligne Markdown.
    pub context: String,
}

impl BrokenLink {
    /// Code documentaire stable, indépendant du cache `Link.status`.
    pub fn code_for(status: LinkStatus) -> &'static str {
        match status {
            LinkStatus::Broken => "DOC-LINK-001",
            LinkStatus::Archived => "DOC-LINK-002",
            LinkStatus::Ok => "DOC-LINK-000",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnomalyCode {
    DependencyOnExpiredInterface,
    ConsumerOnScheduledInterface,
    BreakingChangeNotConfirmed,
    BrokenDocumentationLink,
    ArchivedReference,
}

impl AnomalyCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DependencyOnExpiredInterface => "DEPENDENCY_ON_EXPIRED_INTERFACE",
            Self::ConsumerOnScheduledInterface => "CONSUMER_ON_SCHEDULED_INTERFACE",
            Self::BreakingChangeNotConfirmed => "BREAKING_CHANGE_NOT_CONFIRMED",
            Self::BrokenDocumentationLink => "BROKEN_DOCUMENTATION_LINK",
            Self::ArchivedReference => "ARCHIVED_REFERENCE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anomaly {
    pub code: AnomalyCode,
    pub message: String,
    pub object_id: ObjectId,
    pub href: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub domains: usize,
    pub elements: usize,
    pub relationships: usize,
    pub interfaces: usize,
    pub versions: usize,
    pub diagrams: usize,
    pub stories: usize,
    pub reviews: usize,
    pub documentations: usize,
    pub links: usize,
    pub broken_links: usize,
    pub persisted_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceDetail {
    pub interface: Interface,
    pub provider_domain_name: String,
    pub versions: Vec<VersionSummary>,
    pub consumers: Vec<ObjectRef>,
    pub diagrams: Vec<ObjectRef>,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementDetail {
    pub element: archifier_core::ModelElement,
    pub domain_name: String,
    pub parent_name: Option<String>,
    pub outgoing: Vec<RelationshipView>,
    pub incoming: Vec<RelationshipView>,
    pub usages: Usages,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipView {
    pub id: ObjectId,
    pub name: String,
    pub relation_type: String,
    pub notation: Notation,
    pub other_id: ObjectId,
    pub other_name: String,
    pub other_type: ObjectType,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramDetail {
    pub diagram: Diagram,
    pub domain_name: String,
    pub node_count: usize,
    pub unknown_nodes: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewDetail {
    pub review: archifier_core::Review,
    pub domain_name: String,
    pub anomalies: Vec<Anomaly>,
    pub comments: Vec<archifier_core::Comment>,
    pub exposed: Vec<InterfaceRow>,
    pub consumed: Vec<InterfaceRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryDetail {
    pub story: archifier_core::Story,
    pub domain_name: String,
    pub block_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackBlock {
    pub block_id: String,
    pub kind: String,
    pub title: String,
    pub markdown_html: Option<String>,
    pub diagram: Option<Diagram>,
    pub element: Option<ObjectRef>,
    pub interface: Option<InterfaceDetail>,
    pub transition_label: Option<String>,
    pub transition_target: Option<String>,
    pub focus_element_ids: Vec<ObjectId>,
    pub dimmed_element_ids: Vec<ObjectId>,
    /// Zoom effectif du cadrage (0.5–3.0, 1.0 par défaut).
    pub zoom: f32,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackChapter {
    pub id: String,
    pub title: String,
    pub blocks: Vec<PlaybackBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playback {
    pub story_id: ObjectId,
    pub title: String,
    pub chapters: Vec<PlaybackChapter>,
    /// Révisions figées du récit, telles qu'enregistrées dans la Story.
    pub pinned_revisions: std::collections::BTreeMap<ObjectId, Revision>,
    /// Versions d'interface figées, indexées par interface.
    pub pinned_versions: std::collections::BTreeMap<ObjectId, ObjectId>,
}

// ---------------------------------------------------------------------------
// Hiérarchie parent/enfant (T1-09)
// ---------------------------------------------------------------------------

/// Enfant direct d'un élément, avec sa profondeur absolue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HierarchyNode {
    pub object: ObjectRef,
    pub parent_id: Option<ObjectId>,
    /// Profondeur de l'enfant dans la chaîne complète (racine = 1).
    pub depth: u8,
    /// Vrai quand le parent direct est archivé : badge UI, sans code d'erreur.
    pub archived_parent: bool,
}

/// Vue hiérarchique d'un élément : enfants directs et chaîne d'ancêtres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hierarchy {
    pub root: ObjectRef,
    /// Profondeur de l'objet interrogé, en arêtes depuis la racine (0 = racine).
    pub root_depth: u8,
    /// Enfants directs, triés par nom (insensible à la casse) puis identifiant.
    pub children: Vec<HierarchyNode>,
    /// Ancêtres du parent direct vers la racine.
    pub ancestors: Vec<ObjectRef>,
    /// Parent direct de l'objet interrogé, s'il existe.
    pub parent: Option<ObjectRef>,
    /// Vrai quand le parent direct de l'objet interrogé est archivé.
    pub parent_archived: bool,
}

// ---------------------------------------------------------------------------
// Comparaison de revues (T1-11)
// ---------------------------------------------------------------------------

/// Fenêtre temporelle d'une revue, éventuellement ouverte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalWindow {
    pub start: Option<OffsetDateTime>,
    pub end: Option<OffsetDateTime>,
}

/// Différence de deux revues : ensembles exposés/consommés et anomalies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewComparison {
    pub from: ObjectRef,
    pub to: ObjectRef,
    /// Fenêtre temporelle de la revue de départ, si elle en porte une.
    pub window_from: Option<TemporalWindow>,
    /// Fenêtre temporelle de la revue d'arrivée.
    pub window_to: Option<TemporalWindow>,
    /// Instantané de filtres de la revue de départ.
    pub filter_from: Value,
    /// Instantané de filtres de la revue d'arrivée.
    pub filter_to: Value,
    pub exposed_added: Vec<InterfaceRow>,
    pub exposed_removed: Vec<InterfaceRow>,
    /// Interfaces exposées présentes des deux côtés mais différentes.
    pub exposed_changed: Vec<InterfaceRow>,
    pub consumed_added: Vec<InterfaceRow>,
    pub consumed_removed: Vec<InterfaceRow>,
    /// Interfaces consommées présentes des deux côtés mais différentes.
    pub consumed_changed: Vec<InterfaceRow>,
    pub anomalies_added: Vec<Anomaly>,
    pub anomalies_removed: Vec<Anomaly>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFilterType {
    Element,
    Interface,
    Diagram,
    Story,
    Review,
    Domain,
}

impl ObjectFilterType {
    pub fn matches(self, object_type: ObjectType) -> bool {
        matches!(
            (self, object_type),
            (Self::Element, ObjectType::ModelElement)
                | (Self::Interface, ObjectType::Interface)
                | (Self::Diagram, ObjectType::Diagram)
                | (Self::Story, ObjectType::Story)
                | (Self::Review, ObjectType::Review)
                | (Self::Domain, ObjectType::Domain)
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct ElementFilter {
    pub domain_id: Option<ObjectId>,
    pub notation: Option<Notation>,
    pub query: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct InterfaceFilter {
    pub domain_id: Option<ObjectId>,
    pub category: Option<InterfaceCategory>,
    pub status: Option<TemporalStatus>,
    pub query: Option<String>,
    pub consumer_id: Option<ObjectId>,
}

/// Marqueur de statut d'une revue, réexporté pour l'UI.
pub use archifier_core::ReviewStatus as ReviewStatusAlias;
pub fn review_status_label(status: ReviewStatus) -> &'static str {
    match status {
        ReviewStatus::Open => "Ouverte",
        ReviewStatus::Closed => "Clôturée",
    }
}

// ---------------------------------------------------------------------------
// Import OpenAPI (W1a)
// ---------------------------------------------------------------------------

/// Statut d'un aperçu d'import OpenAPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpenApiImportStatus {
    New,
    UpdateProposed,
    Unchanged,
}

/// Aperçu produit par `import_openapi_preview`, sans aucune écriture.
///
/// La preview est opposable : elle transporte le brut, le canonique et le hash
/// pour qu'`apply` recalcule tout sans faire confiance à l'appelant (§10.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenApiImportPreview {
    pub status: OpenApiImportStatus,
    pub domain_id: ObjectId,
    pub interface_id: Option<ObjectId>,
    /// Version de référence du diff.
    pub base_version_id: Option<ObjectId>,
    /// Version DRAFT existante à mettre à jour, ou `None`.
    pub target_version_id: Option<ObjectId>,
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

/// Résultat d'un `apply_openapi_import`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenApiImportOutcome {
    pub status: OpenApiImportStatus,
    pub interface_id: ObjectId,
    pub version_id: Option<ObjectId>,
    pub written: bool,
}

// ---------------------------------------------------------------------------
// Atelier de vues (MVP3, C-09 §4.2)
// ---------------------------------------------------------------------------

/// Filtre des couvertures de la Bibliothèque.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ViewFilter {
    pub domain_id: Option<ObjectId>,
    pub intent: Option<ViewIntent>,
    pub notation: Option<DiagramNotation>,
    pub publication: Option<PublicationStatus>,
    pub owner: Option<String>,
    pub query: Option<String>,
}

/// Nud réduit pour dessiner une vignette de cadrage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniNode {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub kind: NodeKind,
}

/// Couverture de vue pour la Bibliothèque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewCard {
    pub id: ObjectId,
    pub title: String,
    pub domain_id: ObjectId,
    pub domain_name: String,
    pub intent: ViewIntent,
    pub notation: DiagramNotation,
    pub node_count: usize,
    pub draft_count: usize,
    pub publication_status: PublicationStatus,
    pub owner: String,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
    pub framing: ViewFraming,
    pub mini_nodes: Vec<MiniNode>,
}
