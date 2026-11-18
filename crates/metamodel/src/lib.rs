//! Métamodèles pilotés par profils : aucun nom de classe ArchiMate, BPMN ou UML
//! n'est codé en dur dans la logique. Ajouter `archimate:4.x` consiste à déposer
//! un fichier de profil, sans migration des identifiants existants (NFR-014).

use archifier_core::{default_profile_for, Notation, ValidationError, ValidationIssue};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

const ARCHIMATE: &str = include_str!("../profiles/archimate-3.2.toml");
const BPMN: &str = include_str!("../profiles/bpmn-2.0.2-core.toml");
const UML: &str = include_str!("../profiles/uml-2.5.1-static.toml");
const NATIVE: &str = include_str!("../profiles/native-1.0.toml");

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MetamodelError {
    #[error("profil illisible : {0}")]
    Parse(String),
    #[error("profil inconnu : {0}")]
    UnknownProfile(String),
    #[error("profil invalide : {0}")]
    Lint(String),
}

#[derive(Debug, Clone, Deserialize)]
struct RawProfile {
    profile: RawMeta,
}

#[derive(Debug, Clone, Deserialize)]
struct RawMeta {
    id: String,
    notation: String,
    language: String,
    version: String,
    #[serde(default)]
    target_family: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    reference: String,
    #[serde(default)]
    common_properties: Vec<String>,
    #[serde(default)]
    layers: Vec<Layer>,
    #[serde(default)]
    elements: Vec<RawElement>,
    #[serde(default)]
    relations: Vec<RelationDef>,
    #[serde(default)]
    relation_rules: Vec<RawRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawElement {
    id: String,
    metaclass: String,
    layer: String,
    label: String,
    #[serde(default)]
    glyph: String,
    #[serde(default, rename = "abstract")]
    is_abstract: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct RawRule {
    relation: String,
    sources: Vec<String>,
    targets: Vec<String>,
    #[serde(default)]
    same_layer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Layer {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RelationDef {
    pub id: String,
    pub metaclass: String,
    pub label: String,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub symmetric: bool,
    #[serde(default)]
    pub transitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementDef {
    pub id: String,
    pub metaclass: String,
    pub layer: String,
    pub label: String,
    pub glyph: String,
    pub is_abstract: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub relation: String,
    pub sources: Vec<String>,
    pub targets: Vec<String>,
    pub same_layer: bool,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub id: String,
    pub notation: String,
    pub language: String,
    pub version: String,
    pub target_family: String,
    pub status: String,
    pub reference: String,
    pub common_properties: Vec<String>,
    pub layers: Vec<Layer>,
    pub elements: Vec<ElementDef>,
    pub relations: Vec<RelationDef>,
    pub rules: Vec<Rule>,
    by_id: HashMap<String, usize>,
    by_metaclass: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub id: String,
    pub metaclass: String,
    pub label: String,
    pub glyph: String,
    pub is_abstract: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteGroup {
    pub layer_id: String,
    pub layer_label: String,
    pub entries: Vec<PaletteEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationResolution {
    /// La relation appartient à la notation des deux extrémités.
    SameNotation,
    /// La relation traverse les notations : métamodèle natif.
    Native,
}

impl Profile {
    pub fn parse(source: &str) -> Result<Self, MetamodelError> {
        let raw: RawProfile =
            toml::from_str(source).map_err(|e| MetamodelError::Parse(e.to_string()))?;
        let raw = raw.profile;
        let elements: Vec<ElementDef> = raw
            .elements
            .into_iter()
            .map(|e| ElementDef {
                id: e.id,
                metaclass: e.metaclass,
                layer: e.layer,
                label: e.label,
                glyph: e.glyph,
                is_abstract: e.is_abstract,
            })
            .collect();

        let mut by_id = HashMap::new();
        let mut by_metaclass = HashMap::new();
        for (index, element) in elements.iter().enumerate() {
            by_id.insert(element.id.clone(), index);
            by_metaclass.insert(element.metaclass.clone(), index);
        }

        let profile = Profile {
            id: raw.id,
            notation: raw.notation,
            language: raw.language,
            version: raw.version,
            target_family: raw.target_family,
            status: raw.status,
            reference: raw.reference,
            common_properties: raw.common_properties,
            layers: raw.layers,
            elements,
            relations: raw.relations,
            rules: raw
                .relation_rules
                .into_iter()
                .map(|r| Rule {
                    relation: r.relation,
                    sources: r.sources,
                    targets: r.targets,
                    same_layer: r.same_layer,
                })
                .collect(),
            by_id,
            by_metaclass,
        };
        profile.lint()?;
        Ok(profile)
    }

    /// Vérifications structurelles : un profil incohérent est refusé au
    /// démarrage plutôt que de produire des validations silencieusement fausses.
    fn lint(&self) -> Result<(), MetamodelError> {
        let layer_ids: HashSet<&str> = self.layers.iter().map(|l| l.id.as_str()).collect();
        let relation_ids: HashSet<&str> = self.relations.iter().map(|r| r.id.as_str()).collect();
        let mut seen_ids = HashSet::new();
        let mut seen_metaclasses = HashSet::new();
        let mut seen_relations = HashSet::new();

        for element in &self.elements {
            if !seen_ids.insert(element.id.as_str()) {
                return Err(MetamodelError::Lint(format!(
                    "identifiant dupliqué : {}",
                    element.id
                )));
            }
            if !seen_metaclasses.insert(element.metaclass.as_str()) {
                return Err(MetamodelError::Lint(format!(
                    "métaclasse dupliquée : {}",
                    element.metaclass
                )));
            }
            if !layer_ids.contains(element.layer.as_str()) {
                return Err(MetamodelError::Lint(format!(
                    "couche inconnue « {} » pour {}",
                    element.layer, element.id
                )));
            }
        }

        for relation in &self.relations {
            if !seen_relations.insert(relation.id.as_str()) {
                return Err(MetamodelError::Lint(format!(
                    "relation dupliquée : {}",
                    relation.id
                )));
            }
            if relation.symmetric && relation.direction == "DIRECTED" {
                return Err(MetamodelError::Lint(format!(
                    "relation symétrique mais dirigée : {}",
                    relation.id
                )));
            }
        }

        for rule in &self.rules {
            if !relation_ids.contains(rule.relation.as_str()) {
                return Err(MetamodelError::Lint(format!(
                    "[{}] règle sur une relation inconnue : {} (connues : {:?})",
                    self.id, rule.relation, relation_ids
                )));
            }
            for selector in rule.sources.iter().chain(rule.targets.iter()) {
                self.resolve_selector(selector).map_err(|e| {
                    MetamodelError::Lint(format!("règle « {} » : {e}", rule.relation))
                })?;
            }
        }
        Ok(())
    }

    /// Un sélecteur doit désigner au moins un élément, sinon la règle est morte
    /// et la matrice annoncée ne correspond pas au comportement réel.
    fn resolve_selector(&self, selector: &str) -> Result<(), String> {
        if selector == "*" {
            return Ok(());
        }
        if let Some(layer) = selector.strip_prefix("layer:") {
            if self.layers.iter().any(|l| l.id == layer) {
                return Ok(());
            }
            return Err(format!("couche inconnue « {layer} »"));
        }
        if let Some(id) = selector.strip_prefix("class:") {
            if self.by_id.contains_key(id) {
                return Ok(());
            }
            return Err(format!("classe inconnue « {id} »"));
        }
        Err(format!("sélecteur invalide « {selector} »"))
    }

    pub fn element(&self, id: &str) -> Option<&ElementDef> {
        self.by_id.get(id).map(|i| &self.elements[*i])
    }

    pub fn element_by_metaclass(&self, metaclass: &str) -> Option<&ElementDef> {
        self.by_metaclass.get(metaclass).map(|i| &self.elements[*i])
    }

    pub fn relation(&self, id: &str) -> Option<&RelationDef> {
        self.relations.iter().find(|r| r.id == id)
    }

    /// Résolution d'un identifiant d'élément : accepte l'id du profil ou la métaclasse.
    pub fn resolve_element(&self, key: &str) -> Option<&ElementDef> {
        self.element(key).or_else(|| self.element_by_metaclass(key))
    }

    fn selector_matches(&self, selector: &str, element: &ElementDef) -> bool {
        if selector == "*" {
            return true;
        }
        if let Some(layer) = selector.strip_prefix("layer:") {
            return element.layer == layer;
        }
        if let Some(id) = selector.strip_prefix("class:") {
            return element.id == id;
        }
        false
    }

    pub fn relation_allowed(&self, source: &str, relation: &str, target: &str) -> bool {
        let Some(src) = self.resolve_element(source) else {
            return false;
        };
        let Some(tgt) = self.resolve_element(target) else {
            return false;
        };
        self.rules.iter().any(|rule| {
            rule.relation == relation
                && rule.sources.iter().any(|s| self.selector_matches(s, src))
                && rule.targets.iter().any(|s| self.selector_matches(s, tgt))
                && (!rule.same_layer || src.layer == tgt.layer)
        })
    }

    pub fn palette(&self) -> Vec<PaletteGroup> {
        let mut layers = self.layers.clone();
        layers.sort_by_key(|l| l.order);
        layers
            .into_iter()
            .map(|layer| PaletteGroup {
                entries: self
                    .elements
                    .iter()
                    .filter(|e| e.layer == layer.id)
                    .map(|e| PaletteEntry {
                        id: e.id.clone(),
                        metaclass: e.metaclass.clone(),
                        label: e.label.clone(),
                        glyph: e.glyph.clone(),
                        is_abstract: e.is_abstract,
                    })
                    .collect(),
                layer_id: layer.id,
                layer_label: layer.label,
            })
            .filter(|group| !group.entries.is_empty())
            .collect()
    }

    /// MM-ELEM-001 / MM-ELEM-002 / MM-PROP-001.
    pub fn validate_element(
        &self,
        class_key: &str,
        properties: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<&ElementDef, ValidationError> {
        let Some(element) = self.resolve_element(class_key) else {
            return Err(ValidationError::new(vec![ValidationIssue::error(
                "MM-ELEM-001",
                "/metaclass",
                format!(
                    "La classe « {class_key} » n'est pas couverte par le profil {} (sous-ensemble MVP1).",
                    self.id
                ),
            )]));
        };
        if element.is_abstract {
            return Err(ValidationError::new(vec![ValidationIssue::error(
                "MM-ELEM-002",
                "/metaclass",
                format!(
                    "« {} » est abstraite et ne peut pas être instanciée.",
                    element.metaclass
                ),
            )]));
        }
        let allowed: HashSet<&str> = self.common_properties.iter().map(|s| s.as_str()).collect();
        let unknown: Vec<&String> = properties
            .keys()
            .filter(|k| !allowed.contains(k.as_str()) && !k.starts_with("x-"))
            .collect();
        if let Some(key) = unknown.first() {
            return Err(ValidationError::new(vec![ValidationIssue::error(
                "MM-PROP-001",
                format!("/properties/{key}"),
                format!(
                    "La propriété « {key} » n'est pas déclarée par le profil {}.",
                    self.id
                ),
            )]));
        }
        Ok(element)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetamodelRegistry {
    profiles: BTreeMap<String, Profile>,
}

impl MetamodelRegistry {
    pub fn load_embedded() -> Result<Self, MetamodelError> {
        let mut registry = Self::default();
        for source in [ARCHIMATE, BPMN, UML, NATIVE] {
            registry.register(Profile::parse(source)?)?;
        }
        Ok(registry)
    }

    pub fn register(&mut self, profile: Profile) -> Result<(), MetamodelError> {
        if self.profiles.contains_key(&profile.id) {
            return Err(MetamodelError::Lint(format!(
                "profil déjà chargé : {}",
                profile.id
            )));
        }
        self.profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.get(id)
    }

    pub fn profile_for(&self, notation: Notation) -> Option<&Profile> {
        self.profile(default_profile_for(notation))
    }

    pub fn require(&self, id: &str) -> Result<&Profile, MetamodelError> {
        self.profile(id)
            .ok_or_else(|| MetamodelError::UnknownProfile(id.to_string()))
    }

    pub fn ids(&self) -> Vec<&str> {
        self.profiles.keys().map(|s| s.as_str()).collect()
    }

    /// Applique la règle de sélection de notation du §5.4 :
    /// même notation et couple autorisé ⇒ notation ; sinon relation native.
    pub fn resolve_relation_notation(
        &self,
        source: Option<(&str, &str)>,
        relation: &str,
        target: Option<(&str, &str)>,
    ) -> Result<RelationResolution, ValidationError> {
        if let (Some((source_profile, source_class)), Some((target_profile, target_class))) =
            (source, target)
        {
            if source_profile == target_profile {
                if let Some(profile) = self.profile(source_profile) {
                    if profile.relation(relation).is_some()
                        && profile.relation_allowed(source_class, relation, target_class)
                    {
                        return Ok(RelationResolution::SameNotation);
                    }
                }
            }
        }

        let native = self.require("native:1.0").map_err(|e| {
            ValidationError::new(vec![ValidationIssue::error(
                "MM-PROFILE-001",
                "/relationType",
                e.to_string(),
            )])
        })?;
        if native.relation(relation).is_some() {
            return Ok(RelationResolution::Native);
        }

        Err(ValidationError::new(vec![ValidationIssue::error(
            "MM-REL-003",
            "/relationType",
            format!(
                "La relation « {relation} » n'est autorisée ni entre ces métaclasses, ni comme relation native."
            ),
        )]))
    }

    /// Contrôle un élément prêt à être écrit : profil connu, classe couverte,
    /// propriétés déclarées.
    pub fn validate_element(
        &self,
        profile_id: &str,
        class_key: &str,
        properties: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<&ElementDef, ValidationError> {
        let profile = self.profile(profile_id).ok_or_else(|| {
            ValidationError::new(vec![ValidationIssue::error(
                "MM-PROFILE-001",
                "/metamodelVersion",
                format!("Profil « {profile_id} » inconnu."),
            )])
        })?;
        profile.validate_element(class_key, properties)
    }

    /// Couverture effective, pour le rapport `docs/metamodel-coverage.md`.
    pub fn coverage(&self) -> Vec<(String, usize, usize, usize)> {
        self.profiles
            .values()
            .map(|p| {
                let allowed_pairs = p
                    .elements
                    .iter()
                    .flat_map(|s| p.relations.iter().map(move |r| (s, r)))
                    .filter(|(s, r)| p.relation_allowed(&s.id, &r.id, &s.id))
                    .count();
                (
                    p.id.clone(),
                    p.elements.len(),
                    p.relations.len(),
                    allowed_pairs,
                )
            })
            .collect()
    }
}

/// Rapport lisible de couverture des profils.
pub fn coverage_report(registry: &MetamodelRegistry) -> String {
    let mut out = String::from("# Couverture des métamodèles\n\n");
    out.push_str("| Profil | Classes | Relations | Classes auto-reliables |\n");
    out.push_str("|---|---|---|---|\n");
    for (id, classes, relations, pairs) in registry.coverage() {
        out.push_str(&format!("| `{id}` | {classes} | {relations} | {pairs} |\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registry() -> MetamodelRegistry {
        MetamodelRegistry::load_embedded().expect("profils embarqués valides")
    }

    #[test]
    fn loads_all_embedded_profiles() {
        let registry = registry();
        assert_eq!(registry.ids().len(), 4);
        for id in [
            "archimate:3.2",
            "bpmn:2.0.2-core",
            "uml:2.5.1-static",
            "native:1.0",
        ] {
            assert!(registry.profile(id).is_some(), "profil manquant : {id}");
        }
    }

    #[test]
    fn archimate_subset_has_42_classes() {
        let registry = registry();
        let archimate = registry.require("archimate:3.2").unwrap();
        assert_eq!(archimate.elements.len(), 42, "sous-ensemble annoncé");
        assert_eq!(archimate.relations.len(), 11);
    }

    #[test]
    fn allows_serving_between_layers_and_refuses_composition_across_layers() {
        let registry = registry();
        let archimate = registry.require("archimate:3.2").unwrap();
        assert!(archimate.relation_allowed("business-service", "serving", "application-component"));
        assert!(archimate.relation_allowed(
            "application-component",
            "serving",
            "technology-service"
        ));
        assert!(!archimate.relation_allowed(
            "business-service",
            "composition",
            "application-component"
        ));
        assert!(archimate.relation_allowed(
            "application-component",
            "composition",
            "application-interface"
        ));
    }

    #[test]
    fn refuses_unknown_relation_and_class() {
        let registry = registry();
        let archimate = registry.require("archimate:3.2").unwrap();
        assert!(!archimate.relation_allowed("business-actor", "aggregation", "n-existe-pas"));
        assert!(!archimate.relation_allowed(
            "business-actor",
            "composition",
            "application-component"
        ));
    }

    #[test]
    fn abstract_class_cannot_be_instantiated() {
        let registry = registry();
        let empty = serde_json::Map::new();
        let err = registry
            .validate_element("archimate:3.2", "business-behavior", &empty)
            .unwrap_err();
        assert_eq!(err.codes(), vec!["MM-ELEM-002"]);
    }

    #[test]
    fn unknown_element_reports_announced_subset() {
        let registry = registry();
        let empty = serde_json::Map::new();
        let err = registry
            .validate_element("archimate:3.2", "Junction", &empty)
            .unwrap_err();
        assert_eq!(err.codes(), vec!["MM-ELEM-001"]);
        assert!(err.to_string().contains("sous-ensemble MVP1"));
    }

    #[test]
    fn undeclared_property_is_refused() {
        let registry = registry();
        let mut props = serde_json::Map::new();
        props.insert("couleur_préférée".into(), json!("bleu"));
        let err = registry
            .validate_element("archimate:3.2", "business-actor", &props)
            .unwrap_err();
        assert_eq!(err.codes(), vec!["MM-PROP-001"]);
        props.remove("couleur_préférée");
        props.insert("owner".into(), json!("DSI"));
        assert!(registry
            .validate_element("archimate:3.2", "business-actor", &props)
            .is_ok());
    }

    #[test]
    fn cross_notation_relation_falls_back_to_native() {
        let registry = registry();
        let resolved = registry
            .resolve_relation_notation(
                Some(("bpmn:2.0.2-core", "service-task")),
                "refines",
                Some(("archimate:3.2", "business-process")),
            )
            .unwrap();
        assert_eq!(resolved, RelationResolution::Native);
    }

    #[test]
    fn unsupported_cross_notation_relation_is_refused() {
        let registry = registry();
        let err = registry
            .resolve_relation_notation(
                Some(("bpmn:2.0.2-core", "service-task")),
                "serving",
                Some(("archimate:3.2", "business-process")),
            )
            .unwrap_err();
        assert_eq!(err.codes(), vec!["MM-REL-003"]);
    }

    #[test]
    fn bpmn_sequence_flow_cannot_target_a_participant() {
        let registry = registry();
        let bpmn = registry.require("bpmn:2.0.2-core").unwrap();
        assert!(bpmn.relation_allowed("service-task", "sequence-flow", "exclusive-gateway"));
        assert!(!bpmn.relation_allowed("service-task", "sequence-flow", "participant"));
    }

    #[test]
    fn uml_deployment_goes_from_artifact_to_node() {
        let registry = registry();
        let uml = registry.require("uml:2.5.1-static").unwrap();
        assert!(uml.relation_allowed("artifact", "deployment", "node"));
        assert!(!uml.relation_allowed("node", "deployment", "artifact"));
    }

    #[test]
    fn every_selector_resolves_to_a_real_element() {
        // Le lint l'impose déjà ; ce test vérifie qu'aucun profil ne passe par
        // une règle morte une fois chargé.
        let registry = registry();
        for profile in registry.profiles.values() {
            for rule in &profile.rules {
                for selector in rule.sources.iter().chain(rule.targets.iter()) {
                    assert!(
                        profile.resolve_selector(selector).is_ok(),
                        "{} : sélecteur mort {selector}",
                        profile.id
                    );
                    if let Some(layer) = selector.strip_prefix("layer:") {
                        assert!(
                            profile.elements.iter().any(|e| e.layer == layer),
                            "{} : couche {layer} sans classe",
                            profile.id
                        );
                    }
                }
            }
        }
    }
}
