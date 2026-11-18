//! Bus de commandes de l'Atelier (C-01).
//!
//! Toute mutation d'une vue est une `ViewCommand` sérialisable, validée ici
//! **avant** projection. Le moteur reste pur : il reçoit un `ViewWorld` en
//! lecture seule, implémenté par l'adaptateur UI au-dessus du dépôt.

use crate::ids::ObjectId;
use crate::model::{
    Attachment, BoundarySide, DiagramLayout, DraftShape, DraftStatus, DraftZone, EdgeRouting,
    InterfaceCategory, NodeKind, Notation, PublicationStatus, ShapeKind, Size, ViewFraming,
    ViewIntent, Visibility, Zone, ZoneKind,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;

/// Propriétés sémantiques générées par le profil.
pub type PropertyBag = Map<String, Value>;

pub type NodeId = String;
pub type EdgeId = String;
pub type ZoneId = String;
pub type DraftId = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewCommand {
    pub command_id: ObjectId,
    pub view_id: ObjectId,
    #[serde(default)]
    pub expected_revision: Option<u64>,
    pub body: ViewCommandBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViewCommandBody {
    PlaceDraft {
        draft: DraftShape,
    },
    PlaceExisting {
        object_id: ObjectId,
        at: crate::model::Point,
        size: Size,
        #[serde(default)]
        attachment: Option<Attachment>,
    },
    Rename {
        target: Target,
        label: String,
    },
    Formalize {
        draft_id: DraftId,
        metaclass: String,
        domain_id: ObjectId,
        #[serde(default)]
        properties: PropertyBag,
        #[serde(default)]
        new_object_id: Option<ObjectId>,
        #[serde(default)]
        new_version_id: Option<ObjectId>,
    },
    Connect {
        from: EndPoint,
        to: EndPoint,
        relation_type: String,
        new_relationship_id: ObjectId,
    },
    Reconnect {
        edge_id: EdgeId,
        to: EndPoint,
    },
    MoveNode {
        node_id: NodeId,
        delta: crate::model::Point,
    },
    ResizeNode {
        node_id: NodeId,
        size: Size,
    },
    CreateZone {
        zone: DraftZone,
    },
    SetFraming {
        framing: ViewFraming,
    },
    SetTemporalCursor {
        #[serde(with = "time::serde::rfc3339::option", default)]
        at: Option<OffsetDateTime>,
    },
    PublishView {
        visibility: Visibility,
    },
    DeleteRepresentation {
        target: Target,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Target {
    Node { node_id: NodeId },
    Edge { edge_id: EdgeId },
    Zone { zone_id: ZoneId },
    Draft { draft_id: DraftId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EndPoint {
    Node { node_id: NodeId },
    Port { node_id: NodeId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Element,
    Relationship,
    Interface,
    Diagram,
    Domain,
    Story,
    Review,
}

/// Vue en lecture seule du monde, implémentée par l'UI.
pub trait ViewWorld {
    fn view_exists(&self, view: ObjectId) -> bool;
    fn view_revision(&self, view: ObjectId) -> Option<u64>;
    fn view_notation(&self, view: ObjectId) -> Notation;
    fn object_exists(&self, object: ObjectId) -> bool;
    fn object_is_active(&self, object: ObjectId) -> bool;
    fn object_name(&self, object: ObjectId) -> Option<String>;
    fn object_metaclass(&self, object: ObjectId) -> Option<String>;
    fn object_properties(&self, object: ObjectId) -> Option<PropertyBag>;
    fn object_domain(&self, object: ObjectId) -> Option<ObjectId>;
    fn object_kind(&self, object: ObjectId) -> Option<ObjectKind>;
    fn relation_allowed(&self, source: &str, relation: &str, target: &str) -> bool;
    fn relation_type_exists(&self, notation: Notation, relation: &str) -> bool;
    fn domain_exists(&self, domain: ObjectId) -> bool;
    fn domain_is_active(&self, domain: ObjectId) -> bool;
    fn interface_exists(&self, interface: ObjectId) -> bool;
    fn version_belongs_to(&self, version: ObjectId, interface: ObjectId) -> bool;
    fn interface_version_count(&self, interface: ObjectId) -> usize;
    fn relationship_endpoints(&self, relationship: ObjectId) -> Option<(ObjectId, ObjectId)>;
    fn relationship_type(&self, relationship: ObjectId) -> Option<String>;
    fn required_properties_missing(&self, metaclass: &str, properties: &PropertyBag)
        -> Vec<String>;

    /// Valide la classe et les propriétés contre le profil chargé.
    /// Par défaut, aucune règle supplémentaire (implémentations de test).
    fn validate_properties(
        &self,
        _metaclass: &str,
        _properties: &PropertyBag,
    ) -> Result<(), (String, String)> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandError {
    pub code: String,
    pub path: String,
    pub message_key: String,
    pub detail: Option<String>,
}

impl CommandError {
    pub fn new(code: &str, path: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            path: path.into(),
            message_key: format!("error.{code}"),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandWarning {
    pub code: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepoEffect {
    CreateElement {
        object_id: ObjectId,
        domain_id: ObjectId,
        notation: Notation,
        metaclass: String,
        name: String,
        properties: PropertyBag,
    },
    UpdateElementName {
        object_id: ObjectId,
        name: String,
    },
    UpdateElementProperties {
        object_id: ObjectId,
        properties: PropertyBag,
    },
    CreateRelationship {
        object_id: ObjectId,
        domain_id: ObjectId,
        source_id: ObjectId,
        target_id: ObjectId,
        relation_type: String,
    },
    UpdateRelationshipEndpoints {
        relationship_id: ObjectId,
        source_id: ObjectId,
        target_id: ObjectId,
    },
    CreateInterface {
        object_id: ObjectId,
        domain_id: ObjectId,
        name: String,
        logical_key: String,
        category: InterfaceCategory,
    },
    CreateInterfaceVersion {
        object_id: ObjectId,
        interface_id: ObjectId,
        semantic_version: String,
        available_from: Option<OffsetDateTime>,
        available_until: Option<OffsetDateTime>,
    },
    RenameInterface {
        object_id: ObjectId,
        name: String,
    },
    RenameDomain {
        domain_id: ObjectId,
        name: String,
    },
    SetPublication {
        view_id: ObjectId,
        status: PublicationStatus,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandReceipt {
    pub command_id: ObjectId,
    pub view_id: ObjectId,
    pub layout: DiagramLayout,
    pub revision: u64,
    pub effects: Vec<RepoEffect>,
    pub warnings: Vec<CommandWarning>,
}

impl CommandReceipt {
    pub fn edge_count(&self) -> usize {
        self.layout.edges.len()
    }
}

/// Applique une commande à un layout, sans I/O.
///
/// Le layout d'entrée n'est jamais modifié : la projection part d'une copie.
pub fn apply(
    world: &dyn ViewWorld,
    layout: &DiagramLayout,
    revision: u64,
    command: &ViewCommand,
) -> Result<CommandReceipt, CommandError> {
    if let Some(expected) = command.expected_revision {
        if expected != revision {
            return Err(CommandError::new("REV-409", "/expectedRevision")
                .with_detail(format!("attendu {expected}, courant {revision}")));
        }
    }
    if !world.view_exists(command.view_id) || world.view_revision(command.view_id) != Some(revision)
    {
        if world.view_revision(command.view_id).is_some() {
            return Err(CommandError::new("REV-409", "/viewId")
                .with_detail("révision de vue désynchronisée".to_string()));
        }
        return Err(CommandError::new("DGM-000", "/viewId"));
    }
    if layout.schema > 2 {
        return Err(CommandError::new("DGM-013", "/schema"));
    }
    if layout.schema >= 2 && layout.intent.is_none() {
        return Err(CommandError::new("DGM-014", "/intent"));
    }

    let mut next = layout.clone();
    let mut effects = Vec::new();
    let mut warnings = Vec::new();

    match &command.body {
        ViewCommandBody::PlaceDraft { draft } => {
            place_draft(&mut next, draft)?;
        }
        ViewCommandBody::PlaceExisting {
            object_id,
            at,
            size,
            attachment,
        } => {
            place_existing(world, &mut next, *object_id, at, size, attachment)?;
        }
        ViewCommandBody::Rename { target, label } => {
            rename(world, &mut next, target, label, &mut effects)?;
        }
        ViewCommandBody::Formalize {
            draft_id,
            metaclass,
            domain_id,
            properties,
            new_object_id,
            new_version_id,
        } => {
            formalize(
                world,
                &mut next,
                command.view_id,
                draft_id,
                metaclass,
                *domain_id,
                properties,
                *new_object_id,
                *new_version_id,
                &mut effects,
                &mut warnings,
            )?;
        }
        ViewCommandBody::Connect {
            from,
            to,
            relation_type,
            new_relationship_id,
        } => {
            connect(
                world,
                &mut next,
                command.view_id,
                from,
                to,
                relation_type,
                *new_relationship_id,
                &mut effects,
            )?;
        }
        ViewCommandBody::Reconnect { edge_id, to } => {
            reconnect(world, &mut next, edge_id, to, &mut effects)?;
        }
        ViewCommandBody::MoveNode { node_id, delta } => {
            if !delta.x.is_finite() || !delta.y.is_finite() {
                return Err(CommandError::new("DGM-005", "/delta"));
            }
            let node = next
                .nodes
                .iter_mut()
                .find(|n| &n.id == node_id)
                .ok_or_else(|| CommandError::new("DGM-002", "/nodeId"))?;
            node.x += delta.x;
            node.y += delta.y;
            if !node.x.is_finite() || !node.y.is_finite() {
                return Err(CommandError::new("DGM-005", "/node"));
            }
        }
        ViewCommandBody::ResizeNode { node_id, size } => {
            if !valid_size(size) {
                return Err(CommandError::new("DGM-005", "/size"));
            }
            let node = next
                .nodes
                .iter_mut()
                .find(|n| &n.id == node_id)
                .ok_or_else(|| CommandError::new("DGM-002", "/nodeId"))?;
            node.w = size.w;
            node.h = size.h;
        }
        ViewCommandBody::CreateZone { zone } => {
            create_zone(world, &mut next, zone)?;
        }
        ViewCommandBody::SetFraming { framing } => {
            if let Some(zoom) = framing.zoom {
                if !zoom.is_finite() || !(0.25..=4.0).contains(&zoom) {
                    return Err(CommandError::new("LINK-002", "/framing/zoom"));
                }
            }
            for node_id in &framing.focus {
                if !next.nodes.iter().any(|n| &n.id == node_id) {
                    warnings.push(CommandWarning {
                        code: "LINK-003".to_string(),
                        detail: Some(node_id.clone()),
                    });
                }
            }
            next.framing = framing.clone();
        }
        ViewCommandBody::SetTemporalCursor { at } => {
            next.temporal_cursor = at.map(format_rfc3339);
            if let Some(value) = &next.temporal_cursor {
                if value.is_empty() {
                    return Err(CommandError::new("DGM-012", "/temporalCursor"));
                }
            }
        }
        ViewCommandBody::PublishView { visibility } => {
            publish(world, command.view_id, &next, *visibility, &mut effects)?;
        }
        ViewCommandBody::DeleteRepresentation { target } => {
            delete_representation(&mut next, target)?;
        }
    }

    Ok(CommandReceipt {
        command_id: command.command_id,
        view_id: command.view_id,
        layout: next,
        revision: revision + 1,
        effects,
        warnings,
    })
}

fn valid_size(size: &Size) -> bool {
    size.w.is_finite() && size.h.is_finite() && size.w > 0.0 && size.h > 0.0
}

fn valid_geometry(x: f64, y: f64, w: f64, h: f64) -> bool {
    x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0
}

fn format_rfc3339(at: OffsetDateTime) -> String {
    at.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| at.unix_timestamp().to_string())
}

fn place_draft(layout: &mut DiagramLayout, draft: &DraftShape) -> Result<(), CommandError> {
    if draft.id.trim().is_empty() {
        return Err(CommandError::new("DGM-007", "/draft/id"));
    }
    if !valid_geometry(draft.x, draft.y, draft.w, draft.h) {
        return Err(CommandError::new("DGM-007", "/draft"));
    }
    if id_taken(layout, &draft.id) {
        return Err(CommandError::new("DGM-002", "/draft/id"));
    }
    if let Some(zone_id) = &draft.zone_id {
        let zone = layout
            .zones
            .iter()
            .find(|z| &z.id == zone_id)
            .ok_or_else(|| CommandError::new("DGM-009", "/draft/zoneId"))?;
        if draft.shape == ShapeKind::Port && !zone.boundary {
            return Err(CommandError::new("DSG-004", "/draft/attachment")
                .with_detail("ZONE_NOT_BOUNDARY".to_string()));
        }
    }
    let mut placed = draft.clone();
    placed.status = DraftStatus::Sketch;
    layout.drafts.push(placed);
    Ok(())
}

fn place_existing(
    world: &dyn ViewWorld,
    layout: &mut DiagramLayout,
    object_id: ObjectId,
    at: &crate::model::Point,
    size: &Size,
    attachment: &Option<Attachment>,
) -> Result<(), CommandError> {
    if !world.object_exists(object_id) || !world.object_is_active(object_id) {
        return Err(CommandError::new("DGM-001", "/objectId"));
    }
    if !valid_size(size) || !at.x.is_finite() || !at.y.is_finite() {
        return Err(CommandError::new("DGM-005", "/size"));
    }
    if layout.nodes.iter().any(|n| n.object_id == object_id) {
        return Err(CommandError::new("DGM-002", "/objectId")
            .with_detail("OBJECT_ALREADY_PLACED".to_string()));
    }
    let kind = world.object_kind(object_id).ok_or_else(|| {
        CommandError::new("DGM-001", "/objectId").with_detail("UNKNOWN_OBJECT".to_string())
    })?;
    let node_kind = match kind {
        ObjectKind::Element => NodeKind::Element,
        ObjectKind::Interface => NodeKind::Interface,
        ObjectKind::Diagram => NodeKind::DiagramRef,
        _ => {
            return Err(
                CommandError::new("DGM-001", "/objectId").with_detail("NOT_PLACEABLE".to_string())
            )
        }
    };
    if attachment.is_some() {
        if kind != ObjectKind::Interface {
            return Err(CommandError::new("DSG-004", "/attachment")
                .with_detail("NOT_AN_INTERFACE".to_string()));
        }
        let port = attachment.as_ref().unwrap();
        layout
            .zones
            .iter()
            .find(|z| z.id == port.zone_id && z.boundary)
            .ok_or_else(|| CommandError::new("DGM-009", "/attachment/zoneId"))?;
    }
    layout.nodes.push(crate::model::DiagramNode {
        id: format!("n{object_id}"),
        object_id,
        kind: node_kind,
        version_id: None,
        x: at.x,
        y: at.y,
        w: size.w,
        h: size.h,
        layer_id: attachment
            .as_ref()
            .and_then(|a| layout.zones.iter().find(|z| z.id == a.zone_id))
            .and_then(|z| z.layer_id.clone()),
        zone_id: attachment.as_ref().map(|a| a.zone_id.clone()),
        collapsed: false,
        port: attachment.clone(),
        style_override: Map::new(),
    });
    Ok(())
}

fn rename(
    world: &dyn ViewWorld,
    layout: &mut DiagramLayout,
    target: &Target,
    label: &str,
    effects: &mut Vec<RepoEffect>,
) -> Result<(), CommandError> {
    let label = label.trim();
    if label.is_empty() {
        return Err(CommandError::new("OBJ-002", "/label"));
    }
    match target {
        Target::Draft { draft_id } => {
            let draft = layout
                .drafts
                .iter_mut()
                .find(|d| &d.id == draft_id)
                .ok_or_else(|| CommandError::new("DGM-007", "/draftId"))?;
            draft.label = label.to_string();
        }
        Target::Node { node_id } => {
            let object_id = layout
                .nodes
                .iter()
                .find(|n| &n.id == node_id)
                .map(|n| n.object_id)
                .ok_or_else(|| CommandError::new("DGM-002", "/nodeId"))?;
            let kind = world.object_kind(object_id).ok_or_else(|| {
                CommandError::new("DGM-001", "/nodeId").with_detail("UNKNOWN_OBJECT".to_string())
            })?;
            match kind {
                ObjectKind::Element => effects.push(RepoEffect::UpdateElementName {
                    object_id,
                    name: label.to_string(),
                }),
                ObjectKind::Interface => effects.push(RepoEffect::RenameInterface {
                    object_id,
                    name: label.to_string(),
                }),
                _ => {
                    return Err(CommandError::new("CMD-002", "/target")
                        .with_detail("RENAME_NOT_SUPPORTED".to_string()))
                }
            }
        }
        Target::Zone { zone_id } => {
            let zone = layout
                .zones
                .iter_mut()
                .find(|z| &z.id == zone_id)
                .ok_or_else(|| CommandError::new("DGM-009", "/zoneId"))?;
            match (zone.kind, zone.domain_id) {
                (ZoneKind::Domain, Some(domain_id)) => {
                    zone.label = label.to_string();
                    effects.push(RepoEffect::RenameDomain {
                        domain_id,
                        name: label.to_string(),
                    });
                }
                (ZoneKind::Domain, None) => {
                    return Err(CommandError::new("DGM-008", "/zoneId"));
                }
                (ZoneKind::Group, _) => zone.label = label.to_string(),
            }
        }
        Target::Edge { .. } => {
            return Err(CommandError::new("CMD-002", "/target")
                .with_detail("RENAME_NOT_SUPPORTED".to_string()))
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn formalize(
    world: &dyn ViewWorld,
    layout: &mut DiagramLayout,
    view_id: ObjectId,
    draft_id: &str,
    metaclass: &str,
    domain_id: ObjectId,
    properties: &PropertyBag,
    new_object_id: Option<ObjectId>,
    new_version_id: Option<ObjectId>,
    effects: &mut Vec<RepoEffect>,
    warnings: &mut Vec<CommandWarning>,
) -> Result<(), CommandError> {
    let draft = layout
        .drafts
        .iter()
        .find(|d| d.id == draft_id)
        .cloned()
        .ok_or_else(|| CommandError::new("DGM-007", "/draftId"))?;
    if draft.status != DraftStatus::Sketch {
        return Err(CommandError::new("DGM-007", "/draftId")
            .with_detail("DRAFT_ALREADY_FORMALIZED".to_string()));
    }
    if metaclass.trim().is_empty()
        || !world.domain_exists(domain_id)
        || !world.domain_is_active(domain_id)
    {
        return Err(CommandError::new("DSG-001", "/formalize").with_detail(
            if metaclass.trim().is_empty() {
                "METACLASS_REQUIRED".to_string()
            } else {
                "DOMAIN_REQUIRED".to_string()
            },
        ));
    }

    let is_port = draft.shape == ShapeKind::Port;
    let object_id;

    world
        .validate_properties(metaclass, properties)
        .map_err(|(code, detail)| CommandError::new(&code, "/properties").with_detail(detail))?;

    if is_port {
        let attachment = draft
            .attachment
            .clone()
            .or_else(|| {
                draft.zone_id.as_ref().map(|zone_id| Attachment {
                    zone_id: zone_id.clone(),
                    side: BoundarySide::East,
                    offset: 0.5,
                })
            })
            .ok_or_else(|| {
                CommandError::new("DSG-004", "/attachment")
                    .with_detail("PORT_WITHOUT_BOUNDARY".to_string())
            })?;
        layout
            .zones
            .iter()
            .find(|z| z.id == attachment.zone_id && z.boundary)
            .ok_or_else(|| CommandError::new("DGM-009", "/attachment/zoneId"))?;

        if let Some(interface_id) = draft.interface_id {
            if !world.interface_exists(interface_id) {
                return Err(CommandError::new("DSG-004", "/interfaceId"));
            }
            if let Some(version_id) = draft.version_id {
                if !world.version_belongs_to(version_id, interface_id) {
                    return Err(CommandError::new("DSG-004", "/versionId")
                        .with_detail("VERSION_NOT_IN_INTERFACE".to_string()));
                }
            }
            object_id = interface_id;
        } else {
            let created = new_object_id.ok_or_else(|| {
                CommandError::new("CMD-002", "/newObjectId")
                    .with_detail("INTERFACE_ID_REQUIRED".to_string())
            })?;
            let version_id = new_version_id.ok_or_else(|| {
                CommandError::new("CMD-002", "/newVersionId")
                    .with_detail("VERSION_ID_REQUIRED".to_string())
            })?;
            let version_label = properties
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string();
            let category = category_from(properties);
            effects.push(RepoEffect::CreateInterface {
                object_id: created,
                domain_id,
                name: draft.label.clone(),
                logical_key: logical_key(&draft.label, created),
                category,
            });
            effects.push(RepoEffect::CreateInterfaceVersion {
                object_id: version_id,
                interface_id: created,
                semantic_version: version_label,
                available_from: None,
                available_until: None,
            });
            object_id = created;
        }
        layout.drafts.retain(|d| d.id != draft_id);
        layout.nodes.push(crate::model::DiagramNode {
            id: draft.id.clone(),
            object_id,
            kind: NodeKind::Interface,
            version_id: draft.version_id,
            x: draft.x,
            y: draft.y,
            w: draft.w.max(24.0),
            h: draft.h.max(24.0),
            layer_id: draft.layer_id.clone(),
            zone_id: Some(attachment.zone_id.clone()),
            collapsed: false,
            port: Some(attachment),
            style_override: Map::new(),
        });
        return Ok(());
    }

    let created = new_object_id.ok_or_else(|| {
        CommandError::new("CMD-002", "/newObjectId").with_detail("OBJECT_ID_REQUIRED".to_string())
    })?;
    let missing = world.required_properties_missing(metaclass, properties);
    if !missing.is_empty() {
        warnings.push(CommandWarning {
            code: "DSG-001".to_string(),
            detail: Some(missing.join(",")),
        });
    }
    let notation = world.view_notation(view_id);
    effects.push(RepoEffect::CreateElement {
        object_id: created,
        domain_id,
        notation,
        metaclass: metaclass.to_string(),
        name: draft.label.clone(),
        properties: properties.clone(),
    });
    layout.drafts.retain(|d| d.id != draft_id);
    layout.nodes.push(crate::model::DiagramNode {
        id: draft.id.clone(),
        object_id: created,
        kind: NodeKind::Element,
        version_id: None,
        x: draft.x,
        y: draft.y,
        w: draft.w,
        h: draft.h,
        layer_id: draft.layer_id.clone(),
        zone_id: draft.zone_id.clone(),
        collapsed: false,
        port: None,
        style_override: Map::new(),
    });
    Ok(())
}

/// La notation est portée par la vue, pas par le layout ; elle est lue via
/// `ViewWorld::view_notation(command.view_id)`.
pub fn category_from(properties: &PropertyBag) -> InterfaceCategory {
    match properties
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("APPLICATION")
        .to_ascii_uppercase()
        .as_str()
    {
        "BUSINESS" => InterfaceCategory::Business,
        "TECHNICAL" => InterfaceCategory::Technical,
        _ => InterfaceCategory::Application,
    }
}

pub fn logical_key(label: &str, id: ObjectId) -> String {
    let slug: String = label
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let suffix = &id.as_str()[id.as_str().len().saturating_sub(6)..];
    if slug.is_empty() {
        format!("interface-{}", suffix.to_lowercase())
    } else {
        format!("{slug}-{}", suffix.to_lowercase())
    }
}

fn connect(
    world: &dyn ViewWorld,
    layout: &mut DiagramLayout,
    view_id: ObjectId,
    from: &EndPoint,
    to: &EndPoint,
    relation_type: &str,
    new_relationship_id: ObjectId,
    effects: &mut Vec<RepoEffect>,
) -> Result<(), CommandError> {
    let (source_node, source_object) = resolve_endpoint(layout, from)?;
    let (_, target_object) = resolve_endpoint(layout, to)?;
    if source_object == target_object {
        return Err(CommandError::new("MM-REL-002", "/to"));
    }
    let source_class = world.object_metaclass(source_object).ok_or_else(|| {
        CommandError::new("DSG-002", "/from").with_detail("SOURCE_NOT_FORMALIZED".to_string())
    })?;
    let target_class = world.object_metaclass(target_object).ok_or_else(|| {
        CommandError::new("DSG-002", "/to").with_detail("TARGET_NOT_FORMALIZED".to_string())
    })?;
    if !world.relation_type_exists(world.view_notation(view_id), relation_type) {
        return Err(
            CommandError::new("MM-REL-003", "/relationType").with_detail(relation_type.to_string())
        );
    }
    if !world.relation_allowed(&source_class, relation_type, &target_class) {
        return Err(CommandError::new("DSG-002", "/relationType")
            .with_detail(format!("{source_class} -{relation_type}-> {target_class}")));
    }
    let edge_id = new_relationship_id.to_string();
    if layout.edges.iter().any(|e| e.id == edge_id) {
        return Err(CommandError::new("DGM-002", "/newRelationshipId"));
    }
    let domain_id = world
        .object_domain(source_object)
        .or_else(|| world.object_domain(target_object))
        .ok_or_else(|| CommandError::new("DSG-001", "/from"))?;
    effects.push(RepoEffect::CreateRelationship {
        object_id: new_relationship_id,
        domain_id,
        source_id: source_object,
        target_id: target_object,
        relation_type: relation_type.to_string(),
    });
    layout.edges.push(crate::model::DiagramEdge {
        id: edge_id,
        relationship_id: new_relationship_id,
        waypoints: vec![],
        label_visible: true,
        routing: EdgeRouting::Orthogonal,
        layer_id: source_node.layer_id.clone(),
    });
    Ok(())
}

fn reconnect(
    world: &dyn ViewWorld,
    layout: &mut DiagramLayout,
    edge_id: &str,
    to: &EndPoint,
    effects: &mut Vec<RepoEffect>,
) -> Result<(), CommandError> {
    let edge = layout
        .edges
        .iter()
        .find(|e| e.id == edge_id)
        .cloned()
        .ok_or_else(|| CommandError::new("DGM-002", "/edgeId"))?;
    let (_, target_object) = resolve_endpoint(layout, to)?;
    let (source_object, _) = world
        .relationship_endpoints(edge.relationship_id)
        .ok_or_else(|| CommandError::new("DGM-004", "/edgeId"))?;
    let relation_type = world
        .relationship_type(edge.relationship_id)
        .ok_or_else(|| CommandError::new("DGM-004", "/edgeId"))?;
    let source_class = world
        .object_metaclass(source_object)
        .ok_or_else(|| CommandError::new("DSG-002", "/from"))?;
    let target_class = world
        .object_metaclass(target_object)
        .ok_or_else(|| CommandError::new("DSG-002", "/to"))?;
    if !world.relation_allowed(&source_class, &relation_type, &target_class) {
        return Err(CommandError::new("DSG-002", "/to")
            .with_detail(format!("{source_class} -{relation_type}-> {target_class}")));
    }
    effects.push(RepoEffect::UpdateRelationshipEndpoints {
        relationship_id: edge.relationship_id,
        source_id: source_object,
        target_id: target_object,
    });
    Ok(())
}

fn resolve_endpoint(
    layout: &DiagramLayout,
    endpoint: &EndPoint,
) -> Result<(crate::model::DiagramNode, ObjectId), CommandError> {
    let node_id = match endpoint {
        EndPoint::Node { node_id } | EndPoint::Port { node_id } => node_id,
    };
    layout
        .nodes
        .iter()
        .find(|n| &n.id == node_id)
        .cloned()
        .map(|n| {
            let id = n.object_id;
            (n, id)
        })
        .ok_or_else(|| {
            CommandError::new("DSG-002", "/endpoint")
                .with_detail("DRAFT_NOT_FORMALIZED".to_string())
        })
}

fn create_zone(
    world: &dyn ViewWorld,
    layout: &mut DiagramLayout,
    zone: &DraftZone,
) -> Result<(), CommandError> {
    if zone.id.trim().is_empty() || id_taken(layout, &zone.id) {
        return Err(CommandError::new("DGM-002", "/zone/id"));
    }
    if !valid_geometry(zone.x, zone.y, zone.w, zone.h) {
        return Err(CommandError::new("DGM-005", "/zone"));
    }
    if zone.kind == ZoneKind::Domain {
        let domain_id = zone
            .domain_id
            .ok_or_else(|| CommandError::new("DGM-008", "/zone/domainId"))?;
        if !world.domain_exists(domain_id) {
            return Err(CommandError::new("DGM-008", "/zone/domainId"));
        }
        if !world.domain_is_active(domain_id) {
            return Err(CommandError::new("DGM-008-W", "/zone/domainId"));
        }
    }
    if let Some(layer_id) = &zone.layer_id {
        if !layout.layers.iter().any(|l| &l.id == layer_id) {
            return Err(CommandError::new("DGM-009", "/zone/layerId"));
        }
    }
    layout.zones.push(Zone {
        id: zone.id.clone(),
        kind: zone.kind,
        domain_id: zone.domain_id,
        label: zone.label.clone(),
        x: zone.x,
        y: zone.y,
        w: zone.w,
        h: zone.h,
        layer_id: zone.layer_id.clone(),
        boundary: zone.boundary,
        collapsed: false,
        tone: None,
    });
    Ok(())
}

fn publish(
    world: &dyn ViewWorld,
    view_id: ObjectId,
    layout: &DiagramLayout,
    visibility: Visibility,
    effects: &mut Vec<RepoEffect>,
) -> Result<(), CommandError> {
    if layout.intent.is_none() {
        return Err(CommandError::new("DGM-014", "/intent"));
    }
    if !layout.drafts.is_empty() {
        let ids: Vec<String> = layout.drafts.iter().map(|d| d.id.clone()).collect();
        return Err(CommandError::new("DSG-003", "/drafts").with_detail(ids.join(",")));
    }
    for node in &layout.nodes {
        if !world.object_exists(node.object_id) {
            return Err(
                CommandError::new("DGM-001", "/node").with_detail(node.object_id.to_string())
            );
        }
        match node.kind {
            NodeKind::Interface | NodeKind::DiagramRef => {}
            _ => {
                let metaclass = world.object_metaclass(node.object_id).ok_or_else(|| {
                    CommandError::new("DSG-001", "/node").with_detail("NO_METACLASS".to_string())
                })?;
                let properties = world.object_properties(node.object_id).unwrap_or_default();
                let missing = world.required_properties_missing(&metaclass, &properties);
                if !missing.is_empty() {
                    return Err(CommandError::new("DSG-001", "/node").with_detail(format!(
                        "{}:{}",
                        node.object_id,
                        missing.join(",")
                    )));
                }
            }
        }
    }
    for zone in &layout.zones {
        if zone.kind == ZoneKind::Domain {
            match zone.domain_id {
                Some(domain_id) if world.domain_exists(domain_id) => {}
                _ => return Err(CommandError::new("DGM-008", "/zone")),
            }
        }
    }
    let status = match visibility {
        Visibility::Private => PublicationStatus::Draft,
        Visibility::Domain | Visibility::Public => PublicationStatus::Published,
    };
    effects.push(RepoEffect::SetPublication { view_id, status });
    Ok(())
}

fn delete_representation(layout: &mut DiagramLayout, target: &Target) -> Result<(), CommandError> {
    match target {
        Target::Node { node_id } => {
            let before = layout.nodes.len();
            layout.nodes.retain(|n| &n.id != node_id);
            if layout.nodes.len() == before {
                return Err(CommandError::new("DGM-002", "/nodeId"));
            }
        }
        Target::Edge { edge_id } => {
            let before = layout.edges.len();
            layout.edges.retain(|e| &e.id != edge_id);
            if layout.edges.len() == before {
                return Err(CommandError::new("DGM-002", "/edgeId"));
            }
        }
        Target::Zone { zone_id } => {
            let before = layout.zones.len();
            layout.zones.retain(|z| &z.id != zone_id);
            if layout.zones.len() == before {
                return Err(CommandError::new("DGM-009", "/zoneId"));
            }
            for node in &mut layout.nodes {
                if node.zone_id.as_deref() == Some(zone_id) {
                    node.zone_id = None;
                    node.port = None;
                }
            }
            for draft in &mut layout.drafts {
                if draft.zone_id.as_deref() == Some(zone_id) {
                    draft.zone_id = None;
                    draft.attachment = None;
                }
            }
        }
        Target::Draft { draft_id } => {
            let before = layout.drafts.len();
            layout.drafts.retain(|d| &d.id != draft_id);
            if layout.drafts.len() == before {
                return Err(CommandError::new("DGM-007", "/draftId"));
            }
        }
    }
    Ok(())
}

fn id_taken(layout: &DiagramLayout, id: &str) -> bool {
    layout.nodes.iter().any(|n| n.id == id)
        || layout.edges.iter().any(|e| e.id == id)
        || layout.zones.iter().any(|z| z.id == id)
        || layout.drafts.iter().any(|d| d.id == id)
}

/// Validation de forme du layout, indépendante du dépôt.
pub fn validate_layout_shape(layout: &DiagramLayout) -> Vec<CommandWarning> {
    let mut warnings = Vec::new();
    if layout.schema > 2 {
        warnings.push(CommandWarning {
            code: "DGM-013".into(),
            detail: None,
        });
    }
    if layout.schema >= 2 && layout.intent.is_none() {
        warnings.push(CommandWarning {
            code: "DGM-014".into(),
            detail: None,
        });
    }
    for draft in &layout.drafts {
        if draft.status != DraftStatus::Sketch {
            warnings.push(CommandWarning {
                code: "DGM-015".into(),
                detail: Some(draft.id.clone()),
            });
        }
    }
    warnings
}

// ---------------------------------------------------------------------------
// Migration schéma 1 → schéma 2 (C-02 §5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationNote {
    pub code: String,
    pub detail: Option<String>,
}

/// Migre un layout schéma 1 vers le schéma 2, sans toucher à la géométrie.
///
/// `layer_of` fournit la couche d'un objet quand elle est connue ; elle n'est
/// jamais devinée depuis la position.
pub fn migrate_layout_v1_to_v2(
    layout: &DiagramLayout,
    intent: ViewIntent,
    scope_domain: Option<ObjectId>,
    layer_of: &dyn Fn(ObjectId) -> Option<String>,
) -> (DiagramLayout, Vec<MigrationNote>) {
    if layout.schema >= 2 {
        return (layout.clone(), vec![]);
    }
    let mut notes = vec![MigrationNote {
        code: "INTENT_DEFAULTED".into(),
        detail: Some(intent.key().to_string()),
    }];
    let mut migrated = layout.clone();
    migrated.schema = 2;
    migrated.intent = Some(intent);
    migrated.scope = crate::model::ViewScope {
        kind: if scope_domain.is_some() {
            crate::model::ScopeKind::Domain
        } else {
            crate::model::ScopeKind::Global
        },
        domain_id: scope_domain,
        domain_ids: vec![],
    };
    migrated.grid = Default::default();
    migrated.framing = ViewFraming {
        zoom: Some(layout.viewport.zoom),
        x: layout.viewport.x,
        y: layout.viewport.y,
        focus: vec![],
        dim_others: false,
        caption: None,
    };
    for node in &mut migrated.nodes {
        if node.layer_id.is_none() {
            node.layer_id = layer_of(node.object_id);
        }
    }
    notes.push(MigrationNote {
        code: "LAYOUT_MIGRATED".into(),
        detail: Some(format!("{} formes", migrated.nodes.len())),
    });
    (migrated, notes)
}

/// Intention par défaut d'une migration, dérivée de la notation seulement.
pub fn default_intent_for(notation: Notation) -> ViewIntent {
    match notation {
        Notation::Archimate => ViewIntent::ApplicationLandscape,
        Notation::Bpmn => ViewIntent::Process,
        Notation::Uml => ViewIntent::Deployment,
        Notation::Native => ViewIntent::Context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagramNode, NodeKind, ViewScope};
    use std::collections::HashMap;

    const VIEW: ObjectId = ObjectId::from_parts(10, 1);
    const DOMAIN: ObjectId = ObjectId::from_parts(10, 2);

    #[derive(Clone)]
    struct Obj {
        name: String,
        metaclass: Option<String>,
        kind: ObjectKind,
        domain: Option<ObjectId>,
        active: bool,
        properties: PropertyBag,
        versions: usize,
    }

    struct World {
        revision: u64,
        notation: Notation,
        objects: HashMap<ObjectId, Obj>,
        relations: HashMap<ObjectId, (ObjectId, ObjectId, String)>,
        allowed: Vec<(String, String, String)>,
        domains: Vec<(ObjectId, bool)>,
    }

    impl World {
        fn new() -> Self {
            let mut objects = HashMap::new();
            objects.insert(
                DOMAIN,
                Obj {
                    name: "Paiements".into(),
                    metaclass: None,
                    kind: ObjectKind::Domain,
                    domain: None,
                    active: true,
                    properties: Map::new(),
                    versions: 0,
                },
            );
            Self {
                revision: 5,
                notation: Notation::Archimate,
                objects,
                relations: HashMap::new(),
                allowed: Vec::new(),
                domains: vec![(DOMAIN, true)],
            }
        }

        fn element(&mut self, id: ObjectId, metaclass: &str) {
            self.objects.insert(
                id,
                Obj {
                    name: metaclass.into(),
                    metaclass: Some(metaclass.into()),
                    kind: ObjectKind::Element,
                    domain: Some(DOMAIN),
                    active: true,
                    properties: Map::new(),
                    versions: 0,
                },
            );
        }

        fn interface(&mut self, id: ObjectId) {
            self.objects.insert(
                id,
                Obj {
                    name: "Payment API".into(),
                    metaclass: None,
                    kind: ObjectKind::Interface,
                    domain: Some(DOMAIN),
                    active: true,
                    properties: Map::new(),
                    versions: 1,
                },
            );
        }
    }

    impl ViewWorld for World {
        fn view_exists(&self, view: ObjectId) -> bool {
            view == VIEW
        }
        fn view_revision(&self, view: ObjectId) -> Option<u64> {
            (view == VIEW).then_some(self.revision)
        }
        fn view_notation(&self, _view: ObjectId) -> Notation {
            self.notation
        }
        fn object_exists(&self, object: ObjectId) -> bool {
            self.objects.contains_key(&object) || self.domains.iter().any(|(id, _)| *id == object)
        }
        fn object_is_active(&self, object: ObjectId) -> bool {
            self.objects
                .get(&object)
                .map(|o| o.active)
                .or_else(|| {
                    self.domains
                        .iter()
                        .find(|(id, _)| *id == object)
                        .map(|(_, a)| *a)
                })
                .unwrap_or(false)
        }
        fn object_name(&self, object: ObjectId) -> Option<String> {
            self.objects.get(&object).map(|o| o.name.clone())
        }
        fn object_metaclass(&self, object: ObjectId) -> Option<String> {
            self.objects.get(&object).and_then(|o| o.metaclass.clone())
        }
        fn object_properties(&self, object: ObjectId) -> Option<PropertyBag> {
            self.objects.get(&object).map(|o| o.properties.clone())
        }
        fn object_domain(&self, object: ObjectId) -> Option<ObjectId> {
            self.objects.get(&object).and_then(|o| o.domain)
        }
        fn object_kind(&self, object: ObjectId) -> Option<ObjectKind> {
            self.objects.get(&object).map(|o| o.kind)
        }
        fn relation_allowed(&self, source: &str, relation: &str, target: &str) -> bool {
            self.allowed
                .iter()
                .any(|(s, r, t)| s == source && r == relation && t == target)
        }
        fn relation_type_exists(&self, _notation: Notation, relation: &str) -> bool {
            relation == "serving" || relation == "composition"
        }
        fn domain_exists(&self, domain: ObjectId) -> bool {
            self.domains.iter().any(|(id, _)| *id == domain)
        }
        fn domain_is_active(&self, domain: ObjectId) -> bool {
            self.domains
                .iter()
                .any(|(id, active)| *id == domain && *active)
        }
        fn interface_exists(&self, interface: ObjectId) -> bool {
            self.objects
                .get(&interface)
                .map(|o| o.kind == ObjectKind::Interface)
                .unwrap_or(false)
        }
        fn version_belongs_to(&self, _version: ObjectId, interface: ObjectId) -> bool {
            self.interface_exists(interface)
        }
        fn interface_version_count(&self, interface: ObjectId) -> usize {
            self.objects
                .get(&interface)
                .map(|o| o.versions)
                .unwrap_or(0)
        }
        fn relationship_endpoints(&self, relationship: ObjectId) -> Option<(ObjectId, ObjectId)> {
            self.relations.get(&relationship).map(|(s, t, _)| (*s, *t))
        }
        fn relationship_type(&self, relationship: ObjectId) -> Option<String> {
            self.relations.get(&relationship).map(|(_, _, r)| r.clone())
        }
        fn required_properties_missing(&self, metaclass: &str, props: &PropertyBag) -> Vec<String> {
            if metaclass == "ApplicationComponent" && !props.contains_key("criticality") {
                vec!["criticality".into()]
            } else {
                vec![]
            }
        }
    }

    fn layout2() -> DiagramLayout {
        DiagramLayout {
            schema: 2,
            intent: Some(ViewIntent::ApplicationLandscape),
            scope: ViewScope {
                kind: crate::model::ScopeKind::Domain,
                domain_id: Some(DOMAIN),
                domain_ids: vec![],
            },
            layers: vec![crate::model::LayerBand {
                id: "application".into(),
                label: "Applicatif".into(),
                y: 0.0,
                h: 300.0,
                collapsed: false,
                tone: Some("application".into()),
            }],
            zones: vec![Zone {
                id: "z1".into(),
                kind: ZoneKind::Domain,
                domain_id: Some(DOMAIN),
                label: "Paiements".into(),
                x: 300.0,
                y: 20.0,
                w: 400.0,
                h: 260.0,
                layer_id: Some("application".into()),
                boundary: true,
                collapsed: false,
                tone: Some("domain-1".into()),
            }],
            ..Default::default()
        }
    }

    fn cmd(body: ViewCommandBody) -> ViewCommand {
        ViewCommand {
            command_id: ObjectId::from_parts(10, 99),
            view_id: VIEW,
            expected_revision: Some(5),
            body,
        }
    }

    fn draft(id: &str, shape: ShapeKind) -> DraftShape {
        DraftShape {
            id: id.into(),
            label: "Pricing".into(),
            shape,
            x: 60.0,
            y: 80.0,
            w: 160.0,
            h: 72.0,
            status: DraftStatus::Sketch,
            suggested_metaclass: Some("ApplicationService".into()),
            suggested_domain_id: Some(DOMAIN),
            layer_id: Some("application".into()),
            zone_id: None,
            attachment: None,
            interface_id: None,
            version_id: None,
        }
    }

    fn id(n: u128) -> ObjectId {
        ObjectId::from_parts(20, n)
    }

    #[test]
    fn place_draft_creates_named_sketch_without_object() {
        let world = World::new();
        let receipt = apply(
            &world,
            &layout2(),
            5,
            &cmd(ViewCommandBody::PlaceDraft {
                draft: draft("d1", ShapeKind::Rect),
            }),
        )
        .unwrap();
        assert_eq!(receipt.layout.drafts.len(), 1);
        assert_eq!(receipt.layout.drafts[0].label, "Pricing");
        assert!(receipt.effects.is_empty());
        assert_eq!(receipt.revision, 6);
    }

    #[test]
    fn place_existing_does_not_duplicate_object() {
        let mut world = World::new();
        world.element(id(1), "ApplicationService");
        let place = ViewCommandBody::PlaceExisting {
            object_id: id(1),
            at: crate::model::Point { x: 10.0, y: 20.0 },
            size: Size { w: 120.0, h: 60.0 },
            attachment: None,
        };
        let first = apply(&world, &layout2(), 5, &cmd(place.clone())).unwrap();
        world.revision = 6;
        let mut second_command = cmd(place);
        second_command.expected_revision = Some(6);
        let second = apply(&world, &first.layout, 6, &second_command);
        assert_eq!(second.unwrap_err().code, "DGM-002");
        assert!(first.effects.is_empty());
    }

    #[test]
    fn rename_draft_does_not_write_repository() {
        let world = World::new();
        let mut layout = layout2();
        layout.drafts.push(draft("d1", ShapeKind::Rect));
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::Rename {
                target: Target::Draft {
                    draft_id: "d1".into(),
                },
                label: "Encaissement".into(),
            }),
        )
        .unwrap();
        assert_eq!(receipt.layout.drafts[0].label, "Encaissement");
        assert!(receipt.effects.is_empty());
    }

    #[test]
    fn formalize_requires_metaclass_and_domain() {
        let world = World::new();
        let mut layout = layout2();
        layout.drafts.push(draft("d1", ShapeKind::Rect));
        let err = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::Formalize {
                draft_id: "d1".into(),
                metaclass: String::new(),
                domain_id: DOMAIN,
                properties: Map::new(),
                new_object_id: Some(id(2)),
                new_version_id: None,
            }),
        )
        .unwrap_err();
        assert_eq!(err.code, "DSG-001");
        assert_eq!(layout.drafts.len(), 1);
    }

    #[test]
    fn formalize_creates_element_and_replaces_draft() {
        let world = World::new();
        let mut layout = layout2();
        layout.drafts.push(draft("d1", ShapeKind::Rect));
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::Formalize {
                draft_id: "d1".into(),
                metaclass: "ApplicationService".into(),
                domain_id: DOMAIN,
                properties: Map::new(),
                new_object_id: Some(id(2)),
                new_version_id: None,
            }),
        )
        .unwrap();
        assert!(receipt.layout.drafts.is_empty());
        assert_eq!(receipt.layout.nodes.len(), 1);
        assert_eq!(receipt.layout.nodes[0].id, "d1");
        assert_eq!(receipt.layout.nodes[0].object_id, id(2));
        assert!(matches!(
            receipt.effects[0],
            RepoEffect::CreateElement { .. }
        ));
    }

    #[test]
    fn formalize_port_without_interface_is_refused() {
        let world = World::new();
        let mut layout = layout2();
        let mut port = draft("d1", ShapeKind::Port);
        port.attachment = Some(Attachment {
            zone_id: "z1".into(),
            side: BoundarySide::East,
            offset: 0.5,
        });
        layout.drafts.push(port);
        let err = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::Formalize {
                draft_id: "d1".into(),
                metaclass: "ApplicationInterface".into(),
                domain_id: DOMAIN,
                properties: Map::new(),
                new_object_id: None,
                new_version_id: None,
            }),
        )
        .unwrap_err();
        assert_eq!(err.code, "CMD-002");
    }

    #[test]
    fn formalize_port_creates_interface_and_version() {
        let world = World::new();
        let mut layout = layout2();
        let mut port = draft("d1", ShapeKind::Port);
        port.attachment = Some(Attachment {
            zone_id: "z1".into(),
            side: BoundarySide::East,
            offset: 0.5,
        });
        layout.drafts.push(port);
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::Formalize {
                draft_id: "d1".into(),
                metaclass: "ApplicationInterface".into(),
                domain_id: DOMAIN,
                properties: Map::new(),
                new_object_id: Some(id(3)),
                new_version_id: Some(id(4)),
            }),
        )
        .unwrap();
        assert_eq!(receipt.layout.nodes[0].kind, NodeKind::Interface);
        assert!(receipt.layout.nodes[0].port.is_some());
        assert_eq!(receipt.effects.len(), 2);
    }

    #[test]
    fn connect_forbidden_relation_is_rejected_before_creation() {
        let mut world = World::new();
        world.element(id(1), "ApplicationService");
        world.element(id(2), "BusinessActor");
        let mut layout = layout2();
        layout.nodes.push(node("n1", id(1)));
        layout.nodes.push(node("n2", id(2)));
        let err = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::Connect {
                from: EndPoint::Node {
                    node_id: "n1".into(),
                },
                to: EndPoint::Node {
                    node_id: "n2".into(),
                },
                relation_type: "serving".into(),
                new_relationship_id: id(9),
            }),
        )
        .unwrap_err();
        assert_eq!(err.code, "DSG-002");
        assert!(layout.edges.is_empty());
    }

    #[test]
    fn connect_allowed_relation_creates_one_relationship() {
        let mut world = World::new();
        world.element(id(1), "ApplicationService");
        world.element(id(2), "ApplicationComponent");
        world.allowed.push((
            "ApplicationService".into(),
            "serving".into(),
            "ApplicationComponent".into(),
        ));
        let mut layout = layout2();
        layout.nodes.push(node("n1", id(1)));
        layout.nodes.push(node("n2", id(2)));
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::Connect {
                from: EndPoint::Node {
                    node_id: "n1".into(),
                },
                to: EndPoint::Node {
                    node_id: "n2".into(),
                },
                relation_type: "serving".into(),
                new_relationship_id: id(9),
            }),
        )
        .unwrap();
        assert_eq!(receipt.layout.edges.len(), 1);
        assert!(matches!(
            receipt.effects[0],
            RepoEffect::CreateRelationship { .. }
        ));
    }

    #[test]
    fn connect_from_sketch_is_refused_until_formalized() {
        let world = World::new();
        let mut layout = layout2();
        layout.drafts.push(draft("d1", ShapeKind::Rect));
        let err = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::Connect {
                from: EndPoint::Node {
                    node_id: "d1".into(),
                },
                to: EndPoint::Node {
                    node_id: "d1".into(),
                },
                relation_type: "serving".into(),
                new_relationship_id: id(9),
            }),
        )
        .unwrap_err();
        assert_eq!(err.code, "DSG-002");
        assert_eq!(err.detail.as_deref(), Some("DRAFT_NOT_FORMALIZED"));
    }

    #[test]
    fn delete_representation_keeps_object() {
        let mut world = World::new();
        world.element(id(1), "ApplicationService");
        let mut layout = layout2();
        layout.nodes.push(node("n1", id(1)));
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::DeleteRepresentation {
                target: Target::Node {
                    node_id: "n1".into(),
                },
            }),
        )
        .unwrap();
        assert!(receipt.layout.nodes.is_empty());
        assert!(receipt.effects.is_empty());
        assert!(world.object_exists(id(1)));
    }

    #[test]
    fn publish_view_with_sketch_is_refused() {
        let mut world = World::new();
        world.element(id(1), "ApplicationService");
        let mut layout = layout2();
        layout.nodes.push(node("n1", id(1)));
        layout.drafts.push(draft("d1", ShapeKind::Rect));
        let err = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::PublishView {
                visibility: Visibility::Public,
            }),
        )
        .unwrap_err();
        assert_eq!(err.code, "DSG-003");
    }

    #[test]
    fn publish_view_incomplete_element_is_refused() {
        let mut world = World::new();
        world.element(id(1), "ApplicationComponent");
        let mut layout = layout2();
        layout.nodes.push(node("n1", id(1)));
        let err = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::PublishView {
                visibility: Visibility::Public,
            }),
        )
        .unwrap_err();
        assert_eq!(err.code, "DSG-001");
        assert!(err.detail.unwrap().contains("criticality"));
    }

    #[test]
    fn publish_view_sets_publication_effect() {
        let mut world = World::new();
        world.element(id(1), "ApplicationService");
        let mut layout = layout2();
        layout.nodes.push(node("n1", id(1)));
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::PublishView {
                visibility: Visibility::Public,
            }),
        )
        .unwrap();
        assert!(matches!(
            receipt.effects[0],
            RepoEffect::SetPublication {
                status: PublicationStatus::Published,
                ..
            }
        ));
    }

    #[test]
    fn stale_revision_returns_conflict_without_data_loss() {
        let world = World::new();
        let mut command = cmd(ViewCommandBody::PlaceDraft {
            draft: draft("d1", ShapeKind::Rect),
        });
        command.expected_revision = Some(4);
        let err = apply(&world, &layout2(), 5, &command).unwrap_err();
        assert_eq!(err.code, "REV-409");
    }

    #[test]
    fn temporal_cursor_does_not_mutate_layout() {
        let world = World::new();
        let mut layout = layout2();
        layout.nodes.push(node("n1", id(1)));
        let before = layout.nodes.clone();
        let at = time::OffsetDateTime::parse(
            "2027-08-01T00:00:00Z",
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap();
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::SetTemporalCursor { at: Some(at) }),
        )
        .unwrap();
        assert_eq!(receipt.layout.nodes, before);
        assert_eq!(
            receipt.layout.temporal_cursor.as_deref(),
            Some("2027-08-01T00:00:00Z")
        );
    }

    #[test]
    fn view_without_intent_is_refused() {
        let world = World::new();
        let mut layout = layout2();
        layout.intent = None;
        let err = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::PlaceDraft {
                draft: draft("d1", ShapeKind::Rect),
            }),
        )
        .unwrap_err();
        assert_eq!(err.code, "DGM-014");
    }

    #[test]
    fn move_node_does_not_touch_repository() {
        let world = World::new();
        let mut layout = layout2();
        layout.nodes.push(node("n1", id(1)));
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::MoveNode {
                node_id: "n1".into(),
                delta: crate::model::Point { x: 8.0, y: -8.0 },
            }),
        )
        .unwrap();
        assert!(receipt.effects.is_empty());
        assert_eq!(receipt.layout.nodes[0].x, 18.0);
    }

    #[test]
    fn migration_preserves_node_geometry() {
        let mut layout = DiagramLayout::default();
        layout.nodes.push(node("n1", id(1)));
        let (migrated, notes) = migrate_layout_v1_to_v2(
            &layout,
            ViewIntent::ApplicationLandscape,
            Some(DOMAIN),
            &|_| None,
        );
        assert_eq!(migrated.schema, 2);
        assert_eq!(migrated.nodes[0].x, 10.0);
        assert_eq!(migrated.nodes[0].y, 20.0);
        assert_eq!(migrated.nodes[0].w, 120.0);
        assert_eq!(migrated.nodes[0].h, 60.0);
        assert!(
            !migrated.layers.iter().any(|l| l.id == "application") || migrated.layers.is_empty()
        );
        assert!(notes.iter().any(|n| n.code == "INTENT_DEFAULTED"));
    }

    #[test]
    fn migration_is_idempotent() {
        let mut layout = DiagramLayout::default();
        layout.nodes.push(node("n1", id(1)));
        let (once, _) = migrate_layout_v1_to_v2(&layout, ViewIntent::Process, None, &|_| None);
        let (twice, notes) = migrate_layout_v1_to_v2(&once, ViewIntent::Process, None, &|_| None);
        assert_eq!(once, twice);
        assert!(notes.is_empty());
    }

    #[test]
    fn delete_zone_detaches_ports() {
        let world = World::new();
        let mut layout = layout2();
        let mut n = node("p1", id(5));
        n.kind = NodeKind::Interface;
        n.zone_id = Some("z1".into());
        n.port = Some(Attachment {
            zone_id: "z1".into(),
            side: BoundarySide::East,
            offset: 0.5,
        });
        layout.nodes.push(n);
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::DeleteRepresentation {
                target: Target::Zone {
                    zone_id: "z1".into(),
                },
            }),
        )
        .unwrap();
        assert!(receipt.layout.zones.is_empty());
        assert!(receipt.layout.nodes[0].port.is_none());
        assert!(receipt.layout.nodes[0].zone_id.is_none());
    }

    #[test]
    fn formalize_port_with_existing_interface_creates_no_object() {
        let mut world = World::new();
        world.interface(id(7));
        let mut layout = layout2();
        let mut port = draft("d1", ShapeKind::Port);
        port.attachment = Some(Attachment {
            zone_id: "z1".into(),
            side: BoundarySide::East,
            offset: 0.5,
        });
        port.interface_id = Some(id(7));
        layout.drafts.push(port);
        let receipt = apply(
            &world,
            &layout,
            5,
            &cmd(ViewCommandBody::Formalize {
                draft_id: "d1".into(),
                metaclass: "ApplicationInterface".into(),
                domain_id: DOMAIN,
                properties: Map::new(),
                new_object_id: None,
                new_version_id: None,
            }),
        )
        .unwrap();
        assert!(receipt.effects.is_empty());
        assert_eq!(receipt.layout.nodes[0].object_id, id(7));
    }

    fn node(node_id: &str, object_id: ObjectId) -> DiagramNode {
        DiagramNode {
            id: node_id.into(),
            object_id,
            kind: NodeKind::Element,
            version_id: None,
            x: 10.0,
            y: 20.0,
            w: 120.0,
            h: 60.0,
            layer_id: Some("application".into()),
            zone_id: None,
            collapsed: false,
            port: None,
            style_override: Map::new(),
        }
    }
}
