//! Modèles de vue embarqués (C-05). Un fichier par intention, chargé au
//! démarrage du bundle : aucun aller-retour réseau.

use crate::design::commands::layers_for_intent;
use crate::i18n::t;
use archifier_core::{
    DiagramLayout, DiagramNotation, DraftShape, DraftStatus, Grid, LayerBand, ShapeKind,
    ViewFraming, ViewIntent, Zone, ZoneKind,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct TemplateFile {
    #[allow(dead_code)]
    schema: u32,
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    intent: ViewIntent,
    title_key: String,
    summary_key: String,
    notation: DiagramNotation,
    #[serde(default)]
    grid: Option<Grid>,
    layers: Vec<TemplateLayer>,
    #[serde(default)]
    zones: Vec<TemplateZone>,
    #[serde(default)]
    framing: ViewFraming,
    #[serde(default)]
    drafts: Vec<TemplateDraft>,
}

#[derive(Debug, Clone, Deserialize)]
struct TemplateLayer {
    id: String,
    label_key: String,
    y: f64,
    h: f64,
    #[serde(default)]
    tone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TemplateZone {
    id: String,
    kind: ZoneKind,
    label_key: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    #[serde(default)]
    layer_id: Option<String>,
    #[serde(default = "yes")]
    boundary: bool,
    #[serde(default)]
    tone: Option<String>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
struct TemplateDraft {
    id: String,
    label_key: String,
    shape: ShapeKind,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    #[serde(default)]
    suggested_metaclass: Option<String>,
    #[serde(default)]
    layer_id: Option<String>,
    #[serde(default)]
    zone_id: Option<String>,
}

const CONTEXT: &str = include_str!("../../../assets/view-templates/context.json");
const APPLICATION: &str = include_str!("../../../assets/view-templates/application-landscape.json");
const PROCESS: &str = include_str!("../../../assets/view-templates/process.json");
const DEPLOYMENT: &str = include_str!("../../../assets/view-templates/deployment.json");
const CAPABILITY: &str = include_str!("../../../assets/view-templates/capability.json");
const MIGRATION: &str = include_str!("../../../assets/view-templates/migration.json");
const INTERFACE: &str = include_str!("../../../assets/view-templates/interface-review.json");

pub fn raw(intent: ViewIntent) -> &'static str {
    match intent {
        ViewIntent::Context => CONTEXT,
        ViewIntent::ApplicationLandscape => APPLICATION,
        ViewIntent::Process => PROCESS,
        ViewIntent::Deployment => DEPLOYMENT,
        ViewIntent::Capability => CAPABILITY,
        ViewIntent::Migration => MIGRATION,
        ViewIntent::InterfaceReview => INTERFACE,
    }
}

fn template(intent: ViewIntent) -> Option<TemplateFile> {
    serde_json::from_str(raw(intent)).ok()
}

pub fn title(intent: ViewIntent) -> String {
    template(intent)
        .map(|template| t(&template.title_key))
        .unwrap_or_else(|| t(intent.label_key()))
}

pub fn summary(intent: ViewIntent) -> String {
    template(intent)
        .map(|template| t(&template.summary_key))
        .unwrap_or_else(|| t(intent.label_key()))
}

pub fn notation(intent: ViewIntent) -> DiagramNotation {
    template(intent)
        .map(|template| template.notation)
        .unwrap_or(DiagramNotation::Archimate)
}

/// Instancie un modèle pour un domaine : zones DOMAIN reçoivent le domaine,
/// les croquis restent des croquis, aucun objet n'est créé.
pub fn layout(intent: ViewIntent, domain_id: archifier_core::ObjectId) -> DiagramLayout {
    let Some(template) = template(intent) else {
        return fallback_layout(intent, domain_id);
    };
    let layers: Vec<LayerBand> = template
        .layers
        .iter()
        .map(|layer| LayerBand {
            id: layer.id.clone(),
            label: t(&layer.label_key),
            y: layer.y,
            h: layer.h,
            collapsed: false,
            tone: layer.tone.clone(),
        })
        .collect();
    let zones: Vec<Zone> = template
        .zones
        .iter()
        .map(|zone| Zone {
            id: zone.id.clone(),
            kind: zone.kind,
            domain_id: if zone.kind == ZoneKind::Domain {
                Some(domain_id)
            } else {
                None
            },
            label: t(&zone.label_key),
            x: zone.x,
            y: zone.y,
            w: zone.w,
            h: zone.h,
            layer_id: zone.layer_id.clone(),
            boundary: zone.boundary,
            collapsed: false,
            tone: zone.tone.clone(),
        })
        .collect();
    let drafts: Vec<DraftShape> = template
        .drafts
        .iter()
        .map(|draft| DraftShape {
            id: draft.id.clone(),
            label: t(&draft.label_key),
            shape: draft.shape,
            x: draft.x,
            y: draft.y,
            w: draft.w,
            h: draft.h,
            status: DraftStatus::Sketch,
            suggested_metaclass: draft.suggested_metaclass.clone(),
            suggested_domain_id: Some(domain_id),
            layer_id: draft.layer_id.clone(),
            zone_id: draft.zone_id.clone(),
            attachment: None,
            interface_id: None,
            version_id: None,
        })
        .collect();
    DiagramLayout {
        schema: 2,
        intent: Some(intent),
        scope: archifier_core::ViewScope {
            kind: archifier_core::ScopeKind::Domain,
            domain_id: Some(domain_id),
            domain_ids: vec![],
        },
        temporal_cursor: None,
        viewport: Default::default(),
        grid: template.grid.unwrap_or(Grid {
            size: 8,
            snap: true,
        }),
        layers,
        zones,
        framing: template.framing,
        nodes: vec![],
        edges: vec![],
        drafts,
        annotations: vec![],
    }
}

fn fallback_layout(intent: ViewIntent, domain_id: archifier_core::ObjectId) -> DiagramLayout {
    DiagramLayout {
        schema: 2,
        intent: Some(intent),
        scope: archifier_core::ViewScope {
            kind: archifier_core::ScopeKind::Domain,
            domain_id: Some(domain_id),
            domain_ids: vec![],
        },
        layers: layers_for_intent(intent),
        ..Default::default()
    }
}
