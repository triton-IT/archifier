//! Validations déterministes, sans I/O. Les codes de règle sont stables et
//! destinés à être affichés tels quels à l'utilisateur.

use crate::model::{
    Diagram, DiagramLayout, DraftStatus, Interface, InterfaceVersion, Lifecycle, ObjectType,
    RepositoryObject, Story, StoryBlock, ZoneKind,
};
use crate::temporal::publication_issues;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub severity: Severity,
    pub path: String,
    pub message: String,
}

impl ValidationIssue {
    pub fn error(code: &'static str, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            severity: Severity::Error,
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn warning(
        code: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.to_string(),
            severity: Severity::Warning,
            path: path.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationError {
    pub fn new(issues: Vec<ValidationIssue>) -> Self {
        Self { issues }
    }

    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    pub fn codes(&self) -> Vec<&str> {
        self.issues.iter().map(|i| i.code.as_str()).collect()
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let joined = self
            .issues
            .iter()
            .map(|i| format!("{} : {}", i.code, i.message))
            .collect::<Vec<_>>()
            .join(" | ");
        f.write_str(&joined)
    }
}

impl std::error::Error for ValidationError {}

type Issues = Vec<ValidationIssue>;

fn collect(issues: Issues) -> Result<(), ValidationError> {
    if issues.iter().any(|i| i.severity == Severity::Error) {
        Err(ValidationError::new(issues))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// OBJ-* : invariant de tout objet
// ---------------------------------------------------------------------------

pub fn validate_object(
    object: &RepositoryObject,
    domain_is_active: bool,
) -> Result<(), ValidationError> {
    let mut issues = Issues::new();

    if object.revision < 1 {
        issues.push(ValidationIssue::error(
            "OBJ-001",
            "/revision",
            "La révision doit être ≥ 1.",
        ));
    }
    if object.name.chars().any(|c| c.is_control()) {
        issues.push(ValidationIssue::error(
            "OBJ-002",
            "/name",
            "Le nom contient des caractères de contrôle.",
        ));
    }
    if !domain_is_active {
        issues.push(ValidationIssue::error(
            "OBJ-003",
            "/responsibleDomainId",
            "Le domaine responsable est introuvable ou archivé.",
        ));
    }
    if object.object_type == ObjectType::Domain && object.responsible_domain_id != object.id {
        issues.push(ValidationIssue::error(
            "OBJ-006",
            "/responsibleDomainId",
            "Un domaine est son propre domaine responsable.",
        ));
    }
    if object.lifecycle == Lifecycle::Archived && object.name.trim().is_empty() {
        issues.push(ValidationIssue::warning(
            "OBJ-002",
            "/name",
            "Un objet archivé conserve son nom pour rester référençable.",
        ));
    }

    collect(issues)
}

// ---------------------------------------------------------------------------
// Domaines
// ---------------------------------------------------------------------------

pub fn validate_domain_name(name: &str) -> Result<(), ValidationError> {
    let mut issues = Issues::new();
    let len = name.trim().chars().count();
    if !(2..=80).contains(&len) {
        issues.push(ValidationIssue::error(
            "DOM-002",
            "/name",
            "Le nom d'un domaine doit contenir entre 2 et 80 caractères.",
        ));
    }
    collect(issues)
}

pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut previous_dash = true;
    for ch in name.chars().flat_map(char::to_lowercase) {
        let mapped = match ch {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ÿ' => 'y',
            'ñ' => 'n',
            'œ' => 'o',
            'æ' => 'a',
            other => other,
        };
        if mapped.is_ascii_alphanumeric() {
            out.push(mapped.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            out.push('-');
            previous_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

// ---------------------------------------------------------------------------
// Interfaces
// ---------------------------------------------------------------------------

pub fn validate_interface(interface: &Interface) -> Result<(), ValidationError> {
    let mut issues = Issues::new();

    if interface.object.responsible_domain_id != interface.provider_domain_id {
        issues.push(ValidationIssue::error(
            "OBJ-005",
            "/providerDomainId",
            "Le domaine fournisseur doit être le domaine responsable de l'interface.",
        ));
    }
    if !is_valid_key(&interface.logical_interface_key) {
        issues.push(ValidationIssue::error(
            "IFACE-002",
            "/logicalInterfaceKey",
            "La clé logique doit être en minuscules ASCII, séparée par des tirets (3 à 80 caractères).",
        ));
    }
    collect(issues)
}

pub fn is_valid_key(key: &str) -> bool {
    let len = key.len();
    if !(3..=80).contains(&len) {
        return false;
    }
    key.split('-').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    })
}

pub fn validate_interface_version(v: &InterfaceVersion) -> Result<(), ValidationError> {
    let mut issues = Issues::new();
    if !is_semver(&v.semantic_version) {
        issues.push(ValidationIssue::error(
            "IFACE-003",
            "/semanticVersion",
            "La version doit respecter SemVer 2.0.0 (ex. 1.4.0).",
        ));
    }
    if matches!(
        v.publication_status,
        crate::model::PublicationStatus::Published
    ) {
        for code in publication_issues(v) {
            let message = match code {
                "BR-001" => {
                    "BR-001 : une version publiée doit définir availableFrom et availableUntil."
                }
                "BR-002" => "BR-002 : availableUntil doit être postérieur à availableFrom.",
                _ => "Fenêtre de disponibilité invalide.",
            };
            let path = if code == "BR-001" {
                "/availableFrom"
            } else {
                "/availableUntil"
            };
            issues.push(ValidationIssue::error(code, path, message));
        }
    }
    collect(issues)
}

pub fn is_semver(s: &str) -> bool {
    let core = s.split(['-', '+']).next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| {
        !p.is_empty()
            && p.bytes().all(|b| b.is_ascii_digit())
            && (p.len() == 1 || !p.starts_with('0'))
    })
}

// ---------------------------------------------------------------------------
// Diagrammes
// ---------------------------------------------------------------------------

pub const MAX_NODES_PER_DIAGRAM: usize = 1000;

pub fn validate_diagram(
    diagram: &Diagram,
    object_exists: &dyn Fn(crate::ids::ObjectId) -> bool,
) -> Result<(), ValidationError> {
    let mut issues = Issues::new();
    validate_layout(&diagram.layout, object_exists, &mut issues);
    collect(issues)
}

pub fn validate_layout(
    layout: &DiagramLayout,
    object_exists: &dyn Fn(crate::ids::ObjectId) -> bool,
    issues: &mut Issues,
) {
    if layout.nodes.len() > MAX_NODES_PER_DIAGRAM {
        issues.push(ValidationIssue::error(
            "DGM-006",
            "/layout/nodes",
            format!("Un diagramme ne peut pas dépasser {MAX_NODES_PER_DIAGRAM} nœuds."),
        ));
    }
    let mut seen = HashSet::new();
    for (i, node) in layout.nodes.iter().enumerate() {
        if !seen.insert(node.id.as_str()) {
            issues.push(ValidationIssue::error(
                "DGM-002",
                format!("/layout/nodes/{i}/id"),
                "Identifiant de nœud dupliqué dans le diagramme.",
            ));
        }
        if !object_exists(node.object_id) {
            issues.push(ValidationIssue::error(
                "DGM-001",
                format!("/layout/nodes/{i}/objectId"),
                "Le nœud référence un objet inexistant.",
            ));
        }
        if !node.x.is_finite() || !node.y.is_finite() || node.w <= 0.0 || node.h <= 0.0 {
            issues.push(ValidationIssue::error(
                "DGM-005",
                format!("/layout/nodes/{i}"),
                "Géométrie invalide : coordonnées finies et dimensions strictement positives requises.",
            ));
        }
    }
    let mut edge_ids = HashSet::new();
    for (i, edge) in layout.edges.iter().enumerate() {
        if !edge_ids.insert(edge.id.as_str()) {
            issues.push(ValidationIssue::error(
                "DGM-002",
                format!("/layout/edges/{i}/id"),
                "Identifiant d'arête dupliqué dans le diagramme.",
            ));
        }
        if !object_exists(edge.relationship_id) {
            issues.push(ValidationIssue::error(
                "DGM-004",
                format!("/layout/edges/{i}/relationshipId"),
                "L'arête référence une relation inexistante.",
            ));
        }
    }

    // --- Schéma 2 ---------------------------------------------------------
    if layout.schema > 2 {
        issues.push(ValidationIssue::error(
            "DGM-013",
            "/layout/schema",
            "Cette vue a été produite par une version plus récente d'Archifier.",
        ));
    }
    if layout.schema >= 2 && layout.intent.is_none() {
        issues.push(ValidationIssue::error(
            "DGM-014",
            "/layout/intent",
            "Une vue doit porter une intention.",
        ));
    }
    if let Some(cursor) = &layout.temporal_cursor {
        if time::OffsetDateTime::parse(cursor, &time::format_description::well_known::Rfc3339)
            .is_err()
        {
            issues.push(ValidationIssue::warning(
                "DGM-012",
                "/layout/temporalCursor",
                "Le curseur temporel est illisible ; le rendu utilise la date du jour.",
            ));
        }
    }

    let layer_ids: HashSet<&str> = layout.layers.iter().map(|l| l.id.as_str()).collect();
    let mut seen_layers = HashSet::new();
    for (i, layer) in layout.layers.iter().enumerate() {
        if !seen_layers.insert(layer.id.as_str()) {
            issues.push(ValidationIssue::warning(
                "DGM-010",
                format!("/layout/layers/{i}/id"),
                "Identifiant de bande dupliqué.",
            ));
        }
        if !layer.h.is_finite() || layer.h <= 0.0 || !layer.y.is_finite() {
            issues.push(ValidationIssue::error(
                "DGM-005",
                format!("/layout/layers/{i}"),
                "La bande a une géométrie invalide.",
            ));
        }
    }

    let zone_ids: HashSet<&str> = layout.zones.iter().map(|z| z.id.as_str()).collect();
    for (i, zone) in layout.zones.iter().enumerate() {
        if !zone.x.is_finite()
            || !zone.y.is_finite()
            || !zone.w.is_finite()
            || !zone.h.is_finite()
            || zone.w <= 0.0
            || zone.h <= 0.0
        {
            issues.push(ValidationIssue::error(
                "DGM-005",
                format!("/layout/zones/{i}"),
                "La zone a une géométrie invalide.",
            ));
        }
        if zone.kind == ZoneKind::Domain {
            match zone.domain_id {
                Some(domain_id) if object_exists(domain_id) => {}
                _ => issues.push(ValidationIssue::error(
                    "DGM-008",
                    format!("/layout/zones/{i}/domainId"),
                    "Une zone de domaine doit référencer un domaine existant.",
                )),
            }
        }
        if let Some(layer_id) = &zone.layer_id {
            if !layer_ids.contains(layer_id.as_str()) {
                issues.push(ValidationIssue::error(
                    "DGM-009",
                    format!("/layout/zones/{i}/layerId"),
                    "La zone référence une bande absente de la vue.",
                ));
            }
        }
    }

    for (i, node) in layout.nodes.iter().enumerate() {
        if let Some(zone_id) = &node.zone_id {
            if !zone_ids.contains(zone_id.as_str()) {
                issues.push(ValidationIssue::error(
                    "DGM-009",
                    format!("/layout/nodes/{i}/zoneId"),
                    "La forme référence une zone absente de la vue.",
                ));
            }
        }
        if let Some(layer_id) = &node.layer_id {
            if !layer_ids.contains(layer_id.as_str()) {
                issues.push(ValidationIssue::error(
                    "DGM-009",
                    format!("/layout/nodes/{i}/layerId"),
                    "La forme référence une bande absente de la vue.",
                ));
            }
        }
        if let Some(port) = &node.port {
            let on_boundary = layout
                .zones
                .iter()
                .find(|z| z.id == port.zone_id)
                .map(|zone| port_is_on_side(port, zone, node))
                .unwrap_or(false);
            if !on_boundary {
                issues.push(ValidationIssue::warning(
                    "DGM-011",
                    format!("/layout/nodes/{i}/port"),
                    "Le port n'est pas posé sur la frontière déclarée.",
                ));
            }
        }
    }

    let mut draft_ids = HashSet::new();
    for (i, draft) in layout.drafts.iter().enumerate() {
        if !draft_ids.insert(draft.id.as_str()) {
            issues.push(ValidationIssue::error(
                "DGM-007",
                format!("/layout/drafts/{i}/id"),
                "Identifiant de croquis dupliqué.",
            ));
        }
        if !draft.x.is_finite()
            || !draft.y.is_finite()
            || !draft.w.is_finite()
            || !draft.h.is_finite()
            || draft.w <= 0.0
            || draft.h <= 0.0
        {
            issues.push(ValidationIssue::error(
                "DGM-007",
                format!("/layout/drafts/{i}"),
                "Le croquis a une géométrie invalide.",
            ));
        }
        if draft.status != DraftStatus::Sketch {
            issues.push(ValidationIssue::warning(
                "DGM-015",
                format!("/layout/drafts/{i}/status"),
                "Un croquis persisté doit être au statut SKETCH.",
            ));
        }
    }
}

fn port_is_on_side(
    port: &crate::model::Attachment,
    zone: &crate::model::Zone,
    node: &crate::model::DiagramNode,
) -> bool {
    use crate::model::BoundarySide;
    const TOLERANCE: f64 = 6.0;
    let cx = node.x + node.w / 2.0;
    let cy = node.y + node.h / 2.0;
    match port.side {
        BoundarySide::North => (cy - zone.y).abs() <= TOLERANCE,
        BoundarySide::South => (cy - (zone.y + zone.h)).abs() <= TOLERANCE,
        BoundarySide::East => (cx - (zone.x + zone.w)).abs() <= TOLERANCE,
        BoundarySide::West => (cx - zone.x).abs() <= TOLERANCE,
    }
}

// ---------------------------------------------------------------------------
// Stories
// ---------------------------------------------------------------------------

pub fn validate_story(story: &Story) -> Result<(), ValidationError> {
    let mut issues = Issues::new();
    let all_blocks: HashSet<&str> = story
        .structure
        .chapters
        .iter()
        .flat_map(|c| c.blocks.iter().map(|b| b.id()))
        .collect();

    for (ci, chapter) in story.structure.chapters.iter().enumerate() {
        let block_ids: HashSet<&str> = chapter.blocks.iter().map(|b| b.id()).collect();
        if block_ids.len() != chapter.blocks.len() {
            issues.push(ValidationIssue::error(
                "STORY-001",
                format!("/structure/chapters/{ci}/blocks"),
                "Deux blocs d'un même chapitre partagent le même identifiant.",
            ));
        }
        for (bi, block) in chapter.blocks.iter().enumerate() {
            if let StoryBlock::Transition {
                target_block_id, ..
            } = block
            {
                // Une transition peut traverser les chapitres : c'est même le cas
                // d'usage principal du storytelling guidé.
                if !all_blocks.contains(target_block_id.as_str()) {
                    issues.push(ValidationIssue::error(
                        "STORY-002",
                        format!("/structure/chapters/{ci}/blocks/{bi}/targetBlockId"),
                        "La transition cible un bloc inexistant dans la story.",
                    ));
                }
            }
        }
    }
    collect(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ObjectId;
    use time::macros::datetime;

    fn object(name: &str) -> RepositoryObject {
        let id = ObjectId::from_parts(1, 1);
        RepositoryObject {
            id,
            object_type: ObjectType::ModelElement,
            name: name.into(),
            responsible_domain_id: ObjectId::from_parts(1, 2),
            lifecycle: Lifecycle::Active,
            revision: 1,
            created_at: datetime!(2026-01-01 00:00 UTC),
            created_by: "u".into(),
            updated_at: datetime!(2026-01-01 00:00 UTC),
            updated_by: "u".into(),
        }
    }

    #[test]
    fn obj_003_requires_active_domain() {
        let err = validate_object(&object("Paiements"), false).unwrap_err();
        assert_eq!(err.codes(), vec!["OBJ-003"]);
    }

    #[test]
    fn obj_002_rejects_control_chars() {
        let err = validate_object(&object("Pai\u{7}ements"), true).unwrap_err();
        assert_eq!(err.codes(), vec!["OBJ-002"]);
    }

    #[test]
    fn semver_validation() {
        assert!(is_semver("1.0.0"));
        assert!(is_semver("10.20.30-rc.1+build.5"));
        assert!(!is_semver("1.0"));
        assert!(!is_semver("01.0.0"));
        assert!(!is_semver("1.0.x"));
    }

    #[test]
    fn slugify_handles_french_accents() {
        assert_eq!(slugify("Paiements & Cartes"), "paiements-cartes");
        assert_eq!(slugify("Données Référentielles"), "donnees-referentielles");
        assert_eq!(slugify("  État—majeur  "), "etat-majeur");
    }

    #[test]
    fn interface_key_format() {
        assert!(is_valid_key("customer-api"));
        assert!(!is_valid_key("Customer-API"));
        assert!(!is_valid_key("customer--api"));
        assert!(!is_valid_key("ab"));
    }

    #[test]
    fn story_transition_must_resolve() {
        use crate::model::{Chapter, StoryStructure};
        let story = Story {
            object: object("Parcours"),
            structure: StoryStructure {
                chapters: vec![Chapter {
                    id: "c1".into(),
                    title: "Début".into(),
                    blocks: vec![StoryBlock::Transition {
                        id: "b1".into(),
                        label: "Suite".into(),
                        target_block_id: "absent".into(),
                    }],
                }],
                pinned_revisions: Default::default(),
                pinned_versions: Default::default(),
            },
        };
        let err = validate_story(&story).unwrap_err();
        assert_eq!(err.codes(), vec!["STORY-002"]);
    }
}
