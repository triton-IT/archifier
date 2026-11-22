//! Modèle de domaine : objets, relations, interfaces, diagrammes, histoires, revues.
//!
//! Aucune I/O, aucune horloge implicite : toute fonction qui a besoin de la date
//! la reçoit en paramètre. C'est ce qui rend le moteur testable au déterminisme près.

use crate::ids::ObjectId;
use crate::map::MapDataset;
use crate::openapi::ContractFormat;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use time::OffsetDateTime;

pub type Revision = u64;
pub type UserId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObjectType {
    Domain,
    ModelElement,
    Relationship,
    Interface,
    Diagram,
    Story,
    Review,
}

impl ObjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Domain => "DOMAIN",
            Self::ModelElement => "MODEL_ELEMENT",
            Self::Relationship => "RELATIONSHIP",
            Self::Interface => "INTERFACE",
            Self::Diagram => "DIAGRAM",
            Self::Story => "STORY",
            Self::Review => "REVIEW",
        }
    }

    pub fn route_segment(self) -> &'static str {
        match self {
            Self::Domain => "domains",
            Self::ModelElement => "elements",
            Self::Relationship => "relationships",
            Self::Interface => "interfaces",
            Self::Diagram => "diagrams",
            Self::Story => "stories",
            Self::Review => "reviews",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Domain => "Domaine",
            Self::ModelElement => "Élément",
            Self::Relationship => "Relation",
            Self::Interface => "Interface",
            Self::Diagram => "Vue",
            Self::Story => "Story",
            Self::Review => "Revue",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Lifecycle {
    Active,
    Archived,
}

impl Lifecycle {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Notation {
    Archimate,
    Bpmn,
    Uml,
    Native,
}

impl Notation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Archimate => "ARCHIMATE",
            Self::Bpmn => "BPMN",
            Self::Uml => "UML",
            Self::Native => "NATIVE",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Archimate => "ArchiMate 3.2",
            Self::Bpmn => "BPMN 2.0 Core",
            Self::Uml => "UML 2.5.1",
            Self::Native => "Natif",
        }
    }
}

/// Objet de base de tout le référentiel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepositoryObject {
    pub id: ObjectId,
    pub object_type: ObjectType,
    pub name: String,
    pub responsible_domain_id: ObjectId,
    pub lifecycle: Lifecycle,
    pub revision: Revision,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub created_by: UserId,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub updated_by: UserId,
}

impl RepositoryObject {
    pub fn is_active(&self) -> bool {
        self.lifecycle.is_active()
    }

    pub fn href(&self) -> String {
        format!("{}/{}", self.object_type.route_segment(), self.id)
    }
}

// ---------------------------------------------------------------------------
// Domaines
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Domain {
    pub object: RepositoryObject,
    pub slug: String,
    pub description: String,
}

impl Domain {
    pub fn id(&self) -> ObjectId {
        self.object.id
    }
}

// ---------------------------------------------------------------------------
// Éléments et relations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelElement {
    pub object: RepositoryObject,
    pub notation: Notation,
    pub metamodel_version: String,
    pub metaclass: String,
    pub parent_id: Option<ObjectId>,
    #[serde(default)]
    pub semantic_properties: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub object: RepositoryObject,
    pub source_id: ObjectId,
    pub target_id: ObjectId,
    pub relation_type: String,
    pub notation: Notation,
    pub metamodel_version: String,
}

// ---------------------------------------------------------------------------
// Interfaces et temporalité
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InterfaceCategory {
    Business,
    Application,
    Technical,
}

impl InterfaceCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Business => "Métier",
            Self::Application => "Applicatif",
            Self::Technical => "Technique",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContractType {
    None,
    OpenApi,
    Event,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    pub object: RepositoryObject,
    pub logical_interface_key: String,
    pub category: InterfaceCategory,
    pub provider_domain_id: ObjectId,
    pub contract_type: ContractType,
    /// Rattachements aux concepts de notation : `[{notation, metaclass, element_id}]`.
    #[serde(default)]
    pub notation_bindings: Vec<NotationBinding>,
}

impl std::ops::Deref for Interface {
    type Target = RepositoryObject;

    /// Accès en lecture aux champs d'objet (`name`, `lifecycle`, `revision`…)
    /// sans dupliquer l'état dans `Interface`.
    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotationBinding {
    pub notation: Notation,
    pub metaclass: String,
    pub element_id: Option<ObjectId>,
}

/// Statut éditorial. Orthogonal au statut temporel (BR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublicationStatus {
    Draft,
    Published,
    Withdrawn,
}

impl Default for PublicationStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl PublicationStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "Brouillon",
            Self::Published => "Publié",
            Self::Withdrawn => "Retiré",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalStatus {
    Scheduled,
    Active,
    Expired,
}

impl TemporalStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Scheduled => "Planifiée",
            Self::Active => "Active",
            Self::Expired => "Expirée",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceVersion {
    pub id: ObjectId,
    pub interface_id: ObjectId,
    pub semantic_version: String,
    pub revision: Revision,
    pub publication_status: PublicationStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub created_by: UserId,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub available_from: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub available_until: Option<OffsetDateTime>,
    #[serde(default)]
    pub breaking_change: bool,
    pub supersedes_version_id: Option<ObjectId>,
    #[serde(default)]
    pub contract_snapshot: Value,
    /// Famille du contrat importé (`None` pour une version sans contrat).
    #[serde(default)]
    pub contract_format: Option<ContractFormat>,
    /// Document source brut, conservé octet à octet (exigence W1a).
    #[serde(default)]
    pub contract_raw: Option<String>,
    /// SHA-256 hexadécimal de la représentation canonique du contrat.
    #[serde(default)]
    pub contract_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceConsumer {
    pub id: ObjectId,
    pub interface_version_id: ObjectId,
    pub consumer_object_id: ObjectId,
    pub consumer_domain_id: ObjectId,
    #[serde(with = "time::serde::rfc3339")]
    pub declared_at: OffsetDateTime,
    pub declared_by: UserId,
}

// ---------------------------------------------------------------------------
// Diagrammes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    pub object: RepositoryObject,
    pub notation: DiagramNotation,
    pub description: String,
    /// Statut éditorial de la vue (MVP3). Distinct du cycle de vie de l'objet.
    #[serde(default)]
    pub publication_status: PublicationStatus,
    /// Première mise en publication, effacée au retrait.
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub published_at: Option<OffsetDateTime>,
    pub layout: DiagramLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagramNotation {
    Archimate,
    Bpmn,
    Uml,
    Free,
}

impl DiagramNotation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Archimate => "ArchiMate 3.2",
            Self::Bpmn => "BPMN 2.0 Core",
            Self::Uml => "UML 2.5.1",
            Self::Free => "Multi-notation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramLayout {
    #[serde(default = "default_schema")]
    pub schema: u32,
    /// Intention de la vue (schéma 2). Absente d'un layout schéma 1 avant migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<ViewIntent>,
    #[serde(default)]
    pub scope: ViewScope,
    #[serde(default)]
    pub temporal_cursor: Option<String>,
    #[serde(default)]
    pub viewport: Viewport,
    #[serde(default)]
    pub grid: Grid,
    #[serde(default)]
    pub layers: Vec<LayerBand>,
    #[serde(default)]
    pub zones: Vec<Zone>,
    #[serde(default)]
    pub framing: ViewFraming,
    #[serde(default)]
    pub nodes: Vec<DiagramNode>,
    #[serde(default)]
    pub edges: Vec<DiagramEdge>,
    #[serde(default)]
    pub drafts: Vec<DraftShape>,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
}

fn default_schema() -> u32 {
    1
}

impl Default for DiagramLayout {
    fn default() -> Self {
        Self {
            schema: 1,
            intent: None,
            scope: ViewScope::default(),
            temporal_cursor: None,
            viewport: Viewport::default(),
            grid: Grid::default(),
            layers: vec![],
            zones: vec![],
            framing: ViewFraming::default(),
            nodes: vec![],
            edges: vec![],
            drafts: vec![],
            annotations: vec![],
        }
    }
}

/// Catalogue fermé des intentions de vue (C-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViewIntent {
    Context,
    ApplicationLandscape,
    Process,
    Deployment,
    Capability,
    Migration,
    InterfaceReview,
}

impl ViewIntent {
    pub const ALL: [Self; 7] = [
        Self::Context,
        Self::ApplicationLandscape,
        Self::Process,
        Self::Deployment,
        Self::Capability,
        Self::Migration,
        Self::InterfaceReview,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Context => "CONTEXT",
            Self::ApplicationLandscape => "APPLICATION_LANDSCAPE",
            Self::Process => "PROCESS",
            Self::Deployment => "DEPLOYMENT",
            Self::Capability => "CAPABILITY",
            Self::Migration => "MIGRATION",
            Self::InterfaceReview => "INTERFACE_REVIEW",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|intent| intent.key().eq_ignore_ascii_case(value.trim()))
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Context => "intent.context.label",
            Self::ApplicationLandscape => "intent.application_landscape.label",
            Self::Process => "intent.process.label",
            Self::Deployment => "intent.deployment.label",
            Self::Capability => "intent.capability.label",
            Self::Migration => "intent.migration.label",
            Self::InterfaceReview => "intent.interface_review.label",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScopeKind {
    Global,
    Domain,
    MultiDomain,
}

impl Default for ScopeKind {
    fn default() -> Self {
        Self::Global
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewScope {
    #[serde(default)]
    pub kind: ScopeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<ObjectId>,
    #[serde(default)]
    pub domain_ids: Vec<ObjectId>,
}

impl Default for ViewScope {
    fn default() -> Self {
        Self {
            kind: ScopeKind::Global,
            domain_id: None,
            domain_ids: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub size: u32,
    pub snap: bool,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            size: 8,
            snap: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerBand {
    pub id: String,
    pub label: String,
    pub y: f64,
    pub h: f64,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ZoneKind {
    Domain,
    Group,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub id: String,
    pub kind: ZoneKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<ObjectId>,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<String>,
    #[serde(default = "default_true")]
    pub boundary: bool,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoundarySide {
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub zone_id: String,
    pub side: BoundarySide,
    /// Position relative le long du côté, dans `[0.0, 1.0]`.
    pub offset: f64,
}

/// Zone demandée par une commande, avant projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftZone {
    pub id: String,
    pub kind: ZoneKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<ObjectId>,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<String>,
    #[serde(default = "default_true")]
    pub boundary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShapeKind {
    Rect,
    Rounded,
    Port,
    Sticky,
    Boundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DraftStatus {
    Sketch,
    Formalized,
    Valid,
}

impl Default for DraftStatus {
    fn default() -> Self {
        Self::Sketch
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftShape {
    pub id: String,
    pub label: String,
    pub shape: ShapeKind,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub status: DraftStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_metaclass: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_domain_id: Option<ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<Attachment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface_id: Option<ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<ObjectId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewFraming {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f64>,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub focus: Vec<String>,
    #[serde(default)]
    pub dim_others: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

impl Default for ViewFraming {
    fn default() -> Self {
        Self {
            zoom: None,
            x: 0.0,
            y: 0.0,
            focus: vec![],
            dim_others: false,
            caption: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeKind {
    Element,
    Interface,
    DiagramRef,
    Group,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramNode {
    pub id: String,
    pub object_id: ObjectId,
    pub kind: NodeKind,
    /// Version d'interface représentée, si le nœud est une version précise.
    #[serde(default)]
    pub version_id: Option<ObjectId>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Bande de couche d'appartenance (repère de lecture, schéma 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<String>,
    /// Zone d'appartenance visuelle (schéma 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_id: Option<String>,
    #[serde(default)]
    pub collapsed: bool,
    /// Ancrage d'un port sur une frontière de domaine (schéma 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<Attachment>,
    #[serde(default)]
    pub style_override: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeRouting {
    Orthogonal,
    Straight,
}

impl Default for EdgeRouting {
    fn default() -> Self {
        Self::Orthogonal
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramEdge {
    pub id: String,
    pub relationship_id: ObjectId,
    #[serde(default)]
    pub waypoints: Vec<Point>,
    #[serde(default = "default_true")]
    pub label_visible: bool,
    #[serde(default)]
    pub routing: EdgeRouting,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

/// Visibilité d'une vue publiée (C-01, C-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Visibility {
    Private,
    Domain,
    Public,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

// ---------------------------------------------------------------------------
// Stories
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub object: RepositoryObject,
    pub structure: StoryStructure,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StoryStructure {
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    /// Révisions épinglées pour figer un récit historique.
    #[serde(default)]
    pub pinned_revisions: BTreeMap<ObjectId, Revision>,
    /// Versions d'interface épinglées, indexées par identifiant d'interface.
    #[serde(default)]
    pub pinned_versions: BTreeMap<ObjectId, ObjectId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub blocks: Vec<StoryBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StoryBlock {
    Markdown {
        id: String,
        markdown: String,
    },
    Diagram {
        id: String,
        diagram_id: ObjectId,
        #[serde(default)]
        framing: DiagramFraming,
    },
    View {
        id: String,
        view: String,
        #[serde(default)]
        caption: Option<String>,
    },
    Element {
        id: String,
        element_id: ObjectId,
        #[serde(default)]
        highlight: bool,
    },
    Interface {
        id: String,
        interface_id: ObjectId,
        #[serde(default)]
        version_id: Option<ObjectId>,
    },
    Transition {
        id: String,
        label: String,
        target_block_id: String,
    },
}

impl StoryBlock {
    pub fn id(&self) -> &str {
        match self {
            Self::Markdown { id, .. }
            | Self::Diagram { id, .. }
            | Self::View { id, .. }
            | Self::Element { id, .. }
            | Self::Interface { id, .. }
            | Self::Transition { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiagramFraming {
    #[serde(default)]
    pub zoom: Option<f32>,
    #[serde(default)]
    pub focus_element_ids: Vec<ObjectId>,
    #[serde(default)]
    pub dimmed_element_ids: Vec<ObjectId>,
    #[serde(default)]
    pub caption: Option<String>,
}

// ---------------------------------------------------------------------------
// Revues
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Review {
    pub object: RepositoryObject,
    pub domain_id: ObjectId,
    #[serde(default)]
    pub filter_snapshot: Value,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub temporal_window_start: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub temporal_window_end: Option<OffsetDateTime>,
    pub status: ReviewStatus,
    #[serde(default)]
    pub markdown_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewStatus {
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewView {
    Summary,
    Exposed,
    Consumed,
    Incoming,
    Outgoing,
    Calendar,
    Contracts,
    Consumers,
    Detail,
}

// ---------------------------------------------------------------------------
// Documentation, commentaires, liens, audit
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentationBlock {
    pub id: ObjectId,
    pub owner_object_id: ObjectId,
    pub markdown: String,
    pub revision: Revision,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub created_by: UserId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: ObjectId,
    pub target_object_id: ObjectId,
    pub author_id: UserId,
    pub markdown: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LinkType {
    Reference,
    Describes,
    Refines,
    Implements,
    Realizes,
    Exposes,
    Consumes,
    TraceTo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LinkStatus {
    Ok,
    Archived,
    Broken,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub id: ObjectId,
    pub source_object_id: ObjectId,
    pub target_uri: String,
    pub target_object_id: Option<ObjectId>,
    pub link_type: LinkType,
    pub status: LinkStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: ObjectId,
    pub actor_id: UserId,
    pub operation: String,
    pub object_id: Option<ObjectId>,
    pub before_revision: Option<Revision>,
    pub after_revision: Option<Revision>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub display_name: String,
    #[serde(default)]
    pub initials: String,
}

/// Résumé d'objet pour les listes et la recherche.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectSummary {
    pub id: ObjectId,
    pub object_type: ObjectType,
    pub name: String,
    pub domain_id: ObjectId,
    pub domain_name: String,
    #[serde(default)]
    pub notation: Option<Notation>,
    #[serde(default)]
    pub metaclass: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    pub lifecycle: Lifecycle,
}

/// Instantané complet du dépôt : même forme que le schéma SQL, sérialisable
/// en un fichier `.archifier.json` (sauvegarde, fixtures, import au lot 2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Dataset {
    #[serde(default)]
    pub users: Vec<User>,
    #[serde(default)]
    pub domains: Vec<Domain>,
    #[serde(default)]
    pub elements: Vec<ModelElement>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    #[serde(default)]
    pub interfaces: Vec<Interface>,
    #[serde(default)]
    pub interface_versions: Vec<InterfaceVersion>,
    #[serde(default)]
    pub interface_consumers: Vec<InterfaceConsumer>,
    #[serde(default)]
    pub diagrams: Vec<Diagram>,
    #[serde(default)]
    pub stories: Vec<Story>,
    #[serde(default)]
    pub reviews: Vec<Review>,
    #[serde(default)]
    pub documentation: Vec<DocumentationBlock>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub audit: Vec<AuditEvent>,
    /// Instantané JSON par (object_id, revision).
    #[serde(default)]
    pub revisions: BTreeMap<String, BTreeMap<String, Value>>,
    /// Cible v0.4 : couche native produit/équipe/value stream et ancres de
    /// carte. Additif : un snapshot MVP3 se charge avec une carte vide.
    #[serde(default)]
    pub map: MapDataset,
}
