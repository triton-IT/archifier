//! Adaptateur du bus de commandes : `ViewWorld` sur le dépôt, exécution des
//! effets, faits de rendu. Aucun écran n'écrit le layout directement.

use crate::i18n::t;
use crate::state::{new_object_id, now, repo, save_last_view, App, LastView};
use archifier_core::{
    apply_view_command, default_intent_for, migrate_layout_v1_to_v2, version_temporal_status,
    CommandError, DiagramLayout, DraftShape, DraftStatus, EdgeRouting, EndPoint, InterfaceCategory,
    NodeKind, Notation, ObjectKind as ViewObjectKind, PropertyBag, PublicationStatus, RepoEffect,
    ShapeKind, Size, Target, TemporalStatus, ViewCommand, ViewCommandBody, ViewFraming, ViewIntent,
    ViewWorld, Visibility,
};
use archifier_repo::{DiagramDetail, Repository};
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;

use super::history::{HistoryEntry, SaveState};
use super::selection::Selection;
use super::tools::Tool;

#[derive(Clone)]
pub struct Designer {
    pub app: App,
    pub view_id: archifier_core::ObjectId,
    pub revision: RwSignal<u64>,
    pub layout: RwSignal<DiagramLayout>,
    pub selection: RwSignal<Selection>,
    pub tool: RwSignal<Tool>,
    pub save: RwSignal<SaveState>,
    pub error: RwSignal<Option<CommandError>>,
    pub warning: RwSignal<Option<String>>,
    /// Demande de focus sur le champ « nom » de l'inspecteur (F2 / double-clic).
    pub focus_name: RwSignal<bool>,
    /// Extrémités en cours de liaison (outil Relation).
    pub link_from: RwSignal<Option<String>>,
    pub link_to: RwSignal<Option<String>>,
    applied: Arc<Mutex<HashSet<archifier_core::ObjectId>>>,
    history: Arc<Mutex<Vec<HistoryEntry>>>,
    redo: Arc<Mutex<Vec<HistoryEntry>>>,
}

impl Designer {
    pub fn load(app: App, id: archifier_core::ObjectId) -> Option<Self> {
        let detail = repo().diagram_detail(id)?;
        let revision = detail.diagram.object.revision;
        let mut layout = detail.diagram.layout.clone();
        let mut revision = revision;
        if layout.schema < 2 {
            let intent = default_intent_for(notation_of(detail.diagram.notation));
            let domain = Some(detail.diagram.object.responsible_domain_id);
            let repo_ref = repo();
            let layer_of = |object_id: archifier_core::ObjectId| {
                repo_ref.element(object_id).and_then(|element| {
                    repo_ref
                        .metamodel()
                        .profile_for(element.notation)
                        .and_then(|p| {
                            p.element_by_metaclass(&element.metaclass)
                                .map(|d| d.layer.clone())
                        })
                })
            };
            let (migrated, _) = migrate_layout_v1_to_v2(&layout, intent, domain, &layer_of);
            layout = migrated;
            if layout.layers.is_empty() {
                layout.layers = layers_for_intent(intent);
            }
            if repo_ref
                .save_layout_expect(id, layout.clone(), revision)
                .is_ok()
            {
                revision += 1;
            }
        }
        let save = RwSignal::new(SaveState::Saved { revision });
        let designer = Self {
            app,
            view_id: id,
            revision: RwSignal::new(revision),
            layout: RwSignal::new(layout),
            selection: RwSignal::new(Selection::None),
            tool: RwSignal::new(Tool::Select),
            save,
            error: RwSignal::new(None),
            warning: RwSignal::new(None),
            focus_name: RwSignal::new(false),
            link_from: RwSignal::new(None),
            link_to: RwSignal::new(None),
            applied: Arc::new(Mutex::new(HashSet::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            redo: Arc::new(Mutex::new(Vec::new())),
        };
        designer.remember();
        Some(designer)
    }

    pub fn detail(&self) -> Option<DiagramDetail> {
        repo().diagram_detail(self.view_id)
    }

    pub fn title(&self) -> String {
        self.detail()
            .map(|d| d.diagram.object.name)
            .unwrap_or_else(|| t("design.untitled"))
    }

    pub fn intent(&self) -> ViewIntent {
        self.layout
            .get_untracked()
            .intent
            .unwrap_or(ViewIntent::ApplicationLandscape)
    }

    pub fn notation(&self) -> Notation {
        self.detail()
            .map(|d| notation_of(d.diagram.notation))
            .unwrap_or(Notation::Archimate)
    }

    pub fn drafts_count(&self) -> usize {
        self.layout.get().drafts.len()
    }

    pub fn remember(&self) {
        let layout = self.layout.get_untracked();
        save_last_view(&LastView {
            schema: 1,
            view_id: self.view_id,
            posture: "DESIGN".into(),
            framing: layout.framing.clone(),
            temporal_cursor: layout.temporal_cursor.clone(),
            at_ms: js_sys::Date::now(),
        });
    }

    /// Applique une commande : validation core, effets dépôt, écriture
    /// optimiste, historique local. Un refus laisse la vue intacte.
    pub fn apply(&self, body: ViewCommandBody) -> Result<(), CommandError> {
        let command_id = new_object_id();
        let revision = self.revision.get_untracked();
        let command = ViewCommand {
            command_id,
            view_id: self.view_id,
            expected_revision: Some(revision),
            body,
        };
        if self.applied.lock().unwrap().contains(&command_id) {
            return Ok(());
        }
        let world = RepoWorld::for_view(self.view_id);
        let before = self.layout.get_untracked();
        let receipt = match apply_view_command(&world, &before, revision, &command) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.error.set(Some(error.clone()));
                self.save.set(SaveState::Saved { revision });
                return Err(error);
            }
        };
        let mut projected = receipt.layout.clone();
        // La publication incrémente la révision de la vue : elle s'applique
        // après l'écriture du layout, sinon le contrôle optimiste échoue.
        let (publication, layout_effects): (Vec<RepoEffect>, Vec<RepoEffect>) = receipt
            .effects
            .iter()
            .cloned()
            .partition(|effect| matches!(effect, RepoEffect::SetPublication { .. }));
        if let Err(error) = self.write_effects(&layout_effects, &mut projected) {
            self.error.set(Some(error.clone()));
            self.save.set(SaveState::Saved { revision });
            return Err(error);
        }
        if let Err(error) = repo().save_layout_expect(self.view_id, projected.clone(), revision) {
            if error.code == "REV-409" {
                self.save.set(SaveState::Conflict);
            } else {
                self.save.set(SaveState::Error {
                    code: error.code.clone(),
                    message: error.message.clone(),
                });
            }
            self.error.set(Some(CommandError {
                code: error.code,
                path: "/save".into(),
                message_key: "error.REV-409".into(),
                detail: Some(error.message),
            }));
            return Err(self.error.get_untracked().unwrap());
        }
        if let Err(error) = self.write_effects(&publication, &mut projected) {
            self.error.set(Some(error.clone()));
            return Err(error);
        }
        let revision_for_node = repo()
            .diagram_detail(self.view_id)
            .map(|detail| detail.diagram.object.revision)
            .unwrap_or(revision + 1);
        self.applied.lock().unwrap().insert(command_id);
        if self.applied.lock().unwrap().len() > 1_000 {
            let mut applied = self.applied.lock().unwrap();
            if let Some(first) = applied.iter().next().copied() {
                applied.remove(&first);
            }
        }
        self.history.lock().unwrap().push(HistoryEntry {
            before,
            after: projected.clone(),
            revision_before: revision,
            revision_after: revision_for_node,
        });
        if self.history.lock().unwrap().len() > 100 {
            self.history.lock().unwrap().remove(0);
        }
        self.redo.lock().unwrap().clear();
        self.layout.set(projected);
        self.revision.set(revision_for_node);
        self.save.set(SaveState::Saved {
            revision: revision_for_node,
        });
        self.error.set(None);
        if let Some(warning) = receipt.warnings.first() {
            self.warning.set(Some(warning.code.clone()));
        } else {
            self.warning.set(None);
        }
        self.app.refresh();
        self.remember();
        Ok(())
    }

    /// Exécute les effets dans l'ordre, puis répare les identités allouées par
    /// le dépôt dans la projection (le layout référence toujours l'id réel).
    fn write_effects(
        &self,
        effects: &[RepoEffect],
        layout: &mut DiagramLayout,
    ) -> Result<(), CommandError> {
        let repository = repo();
        for effect in effects {
            match effect {
                RepoEffect::CreateElement {
                    object_id,
                    domain_id,
                    notation,
                    metaclass,
                    name,
                    properties,
                } => {
                    let actual = repository
                        .create_element(*domain_id, *notation, metaclass, name, properties.clone())
                        .map_err(repo_error)?;
                    if actual != *object_id {
                        rename_node_object(layout, *object_id, actual);
                    }
                }
                RepoEffect::UpdateElementName { object_id, name } => {
                    let properties = repository
                        .element(*object_id)
                        .map(|e| e.semantic_properties)
                        .unwrap_or_default();
                    repository
                        .update_element(*object_id, name, properties)
                        .map_err(repo_error)?;
                }
                RepoEffect::UpdateElementProperties {
                    object_id,
                    properties,
                } => {
                    let name = repository
                        .element(*object_id)
                        .map(|e| e.object.name)
                        .unwrap_or_default();
                    repository
                        .update_element(*object_id, &name, properties.clone())
                        .map_err(repo_error)?;
                }
                RepoEffect::CreateRelationship {
                    object_id,
                    domain_id,
                    source_id,
                    target_id,
                    relation_type,
                } => {
                    let actual = repository
                        .create_relationship(*domain_id, *source_id, *target_id, relation_type)
                        .map_err(repo_error)?;
                    if actual != *object_id {
                        for edge in &mut layout.edges {
                            if edge.relationship_id == *object_id {
                                edge.relationship_id = actual;
                            }
                        }
                    }
                }
                RepoEffect::UpdateRelationshipEndpoints {
                    relationship_id,
                    source_id,
                    target_id,
                } => {
                    repository
                        .update_relationship_endpoints(*relationship_id, *source_id, *target_id)
                        .map_err(repo_error)?;
                }
                RepoEffect::CreateInterface {
                    object_id,
                    domain_id,
                    name,
                    logical_key,
                    category,
                } => {
                    let actual = repository
                        .create_interface(*domain_id, name, logical_key, *category)
                        .map_err(repo_error)?;
                    if actual != *object_id {
                        rename_node_object(layout, *object_id, actual);
                    }
                }
                RepoEffect::CreateInterfaceVersion {
                    object_id,
                    interface_id,
                    semantic_version,
                    available_from,
                    available_until,
                } => {
                    let actual = repository
                        .create_interface_version(
                            *interface_id,
                            semantic_version,
                            *available_from,
                            *available_until,
                            false,
                            false,
                        )
                        .map_err(repo_error)?;
                    if actual != *object_id {
                        for node in &mut layout.nodes {
                            if node.version_id.as_ref() == Some(object_id) {
                                node.version_id = Some(actual);
                            }
                        }
                    }
                }
                RepoEffect::RenameInterface { object_id, name } => {
                    repository
                        .rename_interface(*object_id, name)
                        .map_err(repo_error)?;
                }
                RepoEffect::RenameDomain { domain_id, name } => {
                    let description = repository
                        .domain(*domain_id)
                        .map(|d| d.description)
                        .unwrap_or_default();
                    repository
                        .update_domain(*domain_id, name, &description)
                        .map_err(repo_error)?;
                }
                RepoEffect::SetPublication { view_id, status } => {
                    repository
                        .set_diagram_publication(*view_id, *status)
                        .map_err(repo_error)?;
                }
            }
        }
        Ok(())
    }

    pub fn undo(&self) {
        let entry = self.history.lock().unwrap().pop();
        let Some(entry) = entry else {
            return;
        };
        let revision = self.revision.get_untracked();
        if repo()
            .save_layout_expect(self.view_id, entry.before.clone(), revision)
            .is_ok()
        {
            self.layout.set(entry.before.clone());
            self.revision.set(revision + 1);
            self.save.set(SaveState::Saved {
                revision: revision + 1,
            });
            self.redo.lock().unwrap().push(entry);
            self.selection.set(Selection::None);
            self.app.refresh();
            self.remember();
        }
    }

    pub fn redo(&self) {
        let entry = self.redo.lock().unwrap().pop();
        let Some(entry) = entry else {
            return;
        };
        let revision = self.revision.get_untracked();
        if repo()
            .save_layout_expect(self.view_id, entry.after.clone(), revision)
            .is_ok()
        {
            self.layout.set(entry.after.clone());
            self.revision.set(revision + 1);
            self.save.set(SaveState::Saved {
                revision: revision + 1,
            });
            self.history.lock().unwrap().push(entry);
            self.selection.set(Selection::None);
            self.app.refresh();
            self.remember();
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.history.lock().unwrap().is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.lock().unwrap().is_empty()
    }

    /// Recharge la vue depuis le dépôt après un conflit.
    pub fn reload(&self) {
        if let Some(detail) = repo().diagram_detail(self.view_id) {
            self.revision.set(detail.diagram.object.revision);
            self.layout.set(detail.diagram.layout);
            self.save.set(SaveState::Saved {
                revision: detail.diagram.object.revision,
            });
            self.error.set(None);
            self.selection.set(Selection::None);
        }
    }

    /// Force l'écriture locale sur la révision courante du dépôt.
    pub fn force_save(&self) {
        let layout = self.layout.get_untracked();
        match repo().save_layout(self.view_id, layout) {
            Ok(()) => {
                let revision = repo()
                    .diagram_detail(self.view_id)
                    .map(|d| d.diagram.object.revision)
                    .unwrap_or(self.revision.get_untracked() + 1);
                self.revision.set(revision);
                self.save.set(SaveState::Saved { revision });
                self.error.set(None);
                self.app
                    .notify(t("design.save.saved").replace("{revision}", &revision.to_string()));
            }
            Err(error) => self.save.set(SaveState::Error {
                code: error.code.clone(),
                message: error.message,
            }),
        }
    }

    pub fn save_now(&self) {
        let revision = self.revision.get_untracked();
        self.save.set(SaveState::Saved { revision });
    }

    /// Documentation Markdown : édition documentaire hors bus de layout.
    /// Aucun objet, aucune relation, aucune géométrie n'est créé.
    pub fn set_documentation(&self, object_id: archifier_core::ObjectId, markdown: &str) {
        match repo().set_documentation(object_id, markdown) {
            Ok(()) => {
                self.app.refresh();
                self.app.notify(t("design.formalize.done"));
            }
            Err(error) => {
                self.save.set(SaveState::Error {
                    code: error.code.clone(),
                    message: error.message,
                });
            }
        }
    }
}

fn rename_node_object(
    layout: &mut DiagramLayout,
    hint: archifier_core::ObjectId,
    actual: archifier_core::ObjectId,
) {
    for node in &mut layout.nodes {
        if node.object_id == hint {
            node.object_id = actual;
        }
    }
}

fn notation_of(notation: archifier_core::DiagramNotation) -> Notation {
    match notation {
        archifier_core::DiagramNotation::Archimate => Notation::Archimate,
        archifier_core::DiagramNotation::Bpmn => Notation::Bpmn,
        archifier_core::DiagramNotation::Uml => Notation::Uml,
        archifier_core::DiagramNotation::Free => Notation::Native,
    }
}

fn repo_error(error: archifier_repo::RepoError) -> CommandError {
    CommandError {
        code: error.code.clone(),
        path: "/effect".into(),
        message_key: format!("error.{}", error.code),
        detail: Some(error.message),
    }
}

// ---------------------------------------------------------------------------
// Monde de validation
// ---------------------------------------------------------------------------

pub struct RepoWorld {
    view: archifier_core::ObjectId,
    repository: Rc<Repository>,
}

impl RepoWorld {
    pub fn for_view(view: archifier_core::ObjectId) -> Self {
        Self {
            view,
            repository: repo(),
        }
    }
}

impl ViewWorld for RepoWorld {
    fn view_exists(&self, view: archifier_core::ObjectId) -> bool {
        self.repository.diagram_detail(view).is_some()
    }
    fn view_revision(&self, view: archifier_core::ObjectId) -> Option<u64> {
        self.repository
            .diagram_detail(view)
            .map(|d| d.diagram.object.revision)
    }
    fn view_notation(&self, view: archifier_core::ObjectId) -> Notation {
        self.repository
            .diagram_detail(view)
            .map(|d| notation_of(d.diagram.notation))
            .unwrap_or(Notation::Archimate)
    }
    fn object_exists(&self, object: archifier_core::ObjectId) -> bool {
        self.repository.object_ref(object).is_some()
    }
    fn object_is_active(&self, object: archifier_core::ObjectId) -> bool {
        if let Some(element) = self.repository.element(object) {
            return element.object.is_active();
        }
        if let Some(interface) = self.repository.interface_detail(object) {
            return interface.interface.object.is_active();
        }
        if let Some(diagram) = self.repository.diagram_detail(object) {
            return diagram.diagram.object.is_active();
        }
        if let Some(domain) = self.repository.domain(object) {
            return domain.object.is_active();
        }
        false
    }
    fn object_name(&self, object: archifier_core::ObjectId) -> Option<String> {
        self.repository.object_ref(object).map(|r| r.name)
    }
    fn object_metaclass(&self, object: archifier_core::ObjectId) -> Option<String> {
        self.repository.element(object).map(|e| e.metaclass)
    }
    fn object_properties(&self, object: archifier_core::ObjectId) -> Option<PropertyBag> {
        self.repository
            .element(object)
            .map(|e| e.semantic_properties)
    }
    fn object_domain(&self, object: archifier_core::ObjectId) -> Option<archifier_core::ObjectId> {
        if let Some(element) = self.repository.element(object) {
            return Some(element.object.responsible_domain_id);
        }
        if let Some(interface) = self.repository.interface_detail(object) {
            return Some(interface.interface.provider_domain_id);
        }
        if let Some(diagram) = self.repository.diagram_detail(object) {
            return Some(diagram.diagram.object.responsible_domain_id);
        }
        self.repository.domain(object).map(|d| d.object.id)
    }
    fn object_kind(&self, object: archifier_core::ObjectId) -> Option<ViewObjectKind> {
        let reference = self.repository.object_ref(object)?;
        Some(match reference.object_type {
            archifier_core::ObjectType::ModelElement => ViewObjectKind::Element,
            archifier_core::ObjectType::Relationship => ViewObjectKind::Relationship,
            archifier_core::ObjectType::Interface => ViewObjectKind::Interface,
            archifier_core::ObjectType::Diagram => ViewObjectKind::Diagram,
            archifier_core::ObjectType::Domain => ViewObjectKind::Domain,
            archifier_core::ObjectType::Story => ViewObjectKind::Story,
            archifier_core::ObjectType::Review => ViewObjectKind::Review,
        })
    }
    fn relation_allowed(&self, source: &str, relation: &str, target: &str) -> bool {
        self.repository
            .metamodel()
            .profile_for(self.view_notation(self.view))
            .map(|profile| profile.relation_allowed(source, relation, target))
            .unwrap_or(false)
    }
    fn relation_type_exists(&self, notation: Notation, relation: &str) -> bool {
        self.repository
            .metamodel()
            .profile_for(notation)
            .map(|profile| {
                profile.relation(relation).is_some()
                    || profile
                        .relations
                        .iter()
                        .any(|r| r.metaclass.eq_ignore_ascii_case(relation))
            })
            .unwrap_or(false)
    }
    fn domain_exists(&self, domain: archifier_core::ObjectId) -> bool {
        self.repository.domain(domain).is_some()
    }
    fn domain_is_active(&self, domain: archifier_core::ObjectId) -> bool {
        self.repository
            .domain(domain)
            .map(|d| d.object.is_active())
            .unwrap_or(false)
    }
    fn interface_exists(&self, interface: archifier_core::ObjectId) -> bool {
        self.repository.interface_detail(interface).is_some()
    }
    fn version_belongs_to(
        &self,
        version: archifier_core::ObjectId,
        interface: archifier_core::ObjectId,
    ) -> bool {
        self.repository
            .interface_detail(interface)
            .map(|detail| detail.versions.iter().any(|v| v.version.id == version))
            .unwrap_or(false)
    }
    fn interface_version_count(&self, interface: archifier_core::ObjectId) -> usize {
        self.repository
            .interface_detail(interface)
            .map(|d| d.versions.len())
            .unwrap_or(0)
    }
    fn relationship_endpoints(
        &self,
        relationship: archifier_core::ObjectId,
    ) -> Option<(archifier_core::ObjectId, archifier_core::ObjectId)> {
        self.repository.relationship_endpoints(relationship)
    }
    fn relationship_type(&self, relationship: archifier_core::ObjectId) -> Option<String> {
        self.repository
            .relationship(relationship)
            .map(|r| r.relation_type)
    }
    fn required_properties_missing(
        &self,
        _metaclass: &str,
        _properties: &PropertyBag,
    ) -> Vec<String> {
        // Les profils livrés ne déclarent pas de propriétés requises : la validité
        // repose sur la classe, le domaine actif et la version d'interface (C-04 §3).
        Vec::new()
    }
    fn validate_properties(
        &self,
        metaclass: &str,
        properties: &PropertyBag,
    ) -> Result<(), (String, String)> {
        let notation = self.view_notation(self.view);
        let profile = self
            .repository
            .metamodel()
            .profile_for(notation)
            .ok_or_else(|| ("MM-PROFILE-001".to_string(), notation.label().to_string()))?;
        profile
            .validate_element(metaclass, properties)
            .map(|_| ())
            .map_err(|error| {
                let code = error
                    .codes()
                    .first()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "MM-ELEM-001".into());
                (code, error.to_string())
            })
    }
}

// ---------------------------------------------------------------------------
// Faits de rendu (le canvas ne lit jamais le dépôt)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct NodeFacts {
    pub name: String,
    pub type_label: String,
    pub domain_name: String,
    pub glyph: String,
    pub temporal: Option<TemporalStatus>,
    pub version_label: Option<String>,
    pub archived: bool,
}

pub fn node_facts(
    layout: &DiagramLayout,
    at: Option<OffsetDateTime>,
) -> HashMap<String, NodeFacts> {
    let repository = repo();
    let mut facts = HashMap::new();
    let when = at.unwrap_or_else(now);
    for node in &layout.nodes {
        let reference = repository.object_ref(node.object_id);
        let mut type_label = String::new();
        let mut glyph = "rect".to_string();
        let mut archived = false;
        let mut temporal = None;
        let mut version_label = None;

        if let Some(element) = repository.element(node.object_id) {
            archived = !element.object.is_active();
            if let Some(profile) = repository.metamodel().profile_for(element.notation) {
                if let Some(def) = profile.element_by_metaclass(&element.metaclass) {
                    type_label = def.label.clone();
                    glyph = def.glyph.clone();
                }
            }
            if type_label.is_empty() {
                type_label = element.metaclass.clone();
            }
        } else if let Some(interface) = repository.interface_detail(node.object_id) {
            archived = !interface.interface.object.is_active();
            type_label = format!("Interface {}", interface.interface.category.label());
            glyph = "interface".into();
            let version_id = node
                .version_id
                .or_else(|| interface.versions.first().map(|v| v.version.id));
            if let Some(version) = version_id.and_then(|id| {
                interface
                    .versions
                    .iter()
                    .find(|v| v.version.id == id)
                    .map(|v| v.version.clone())
            }) {
                temporal = version_temporal_status(&version, when);
                version_label = Some(version.semantic_version);
            }
        } else if let Some(diagram) = repository.diagram_detail(node.object_id) {
            archived = !diagram.diagram.object.is_active();
            type_label = "Vue".into();
            glyph = "diagram".into();
        }

        let domain_name = repository
            .element(node.object_id)
            .map(|e| e.object.responsible_domain_id)
            .or_else(|| {
                repository
                    .interface_detail(node.object_id)
                    .map(|i| i.interface.provider_domain_id)
            })
            .and_then(|id| repository.domain(id).map(|d| d.object.name))
            .unwrap_or_else(|| "—".into());

        facts.insert(
            node.id.clone(),
            NodeFacts {
                name: reference.map(|r| r.name).unwrap_or_else(|| "?".into()),
                type_label,
                domain_name,
                glyph,
                temporal,
                version_label,
                archived,
            },
        );
    }
    facts
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeLine {
    pub id: String,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub label: String,
    pub dimmed: bool,
}

pub fn edge_lines(layout: &DiagramLayout) -> Vec<EdgeLine> {
    let repository = repo();
    let mut lines = Vec::new();
    for edge in &layout.edges {
        let Some((source, target)) = repository.relationship_endpoints(edge.relationship_id) else {
            continue;
        };
        let Some(from) = layout.nodes.iter().find(|n| n.object_id == source) else {
            continue;
        };
        let Some(to) = layout.nodes.iter().find(|n| n.object_id == target) else {
            continue;
        };
        let label = repository
            .relationship(edge.relationship_id)
            .map(|r| r.relation_type)
            .unwrap_or_default();
        lines.push(EdgeLine {
            id: edge.id.clone(),
            x1: from.x + from.w / 2.0,
            y1: from.y + from.h / 2.0,
            x2: to.x + to.w / 2.0,
            y2: to.y + to.h / 2.0,
            label,
            dimmed: !layout.framing.focus.is_empty()
                && !layout.framing.dim_others
                && !layout.framing.focus.contains(&from.id)
                && !layout.framing.focus.contains(&to.id),
        });
    }
    lines
}

// ---------------------------------------------------------------------------
// Intentions, palettes, domaines
// ---------------------------------------------------------------------------

pub fn intent_layers(intent: ViewIntent) -> Vec<(&'static str, &'static str, f64)> {
    match intent {
        ViewIntent::Context => vec![
            ("motivation", "layer.motivation", 180.0),
            ("business", "layer.business", 240.0),
            ("application", "layer.application", 260.0),
            ("technology", "layer.technology", 200.0),
        ],
        ViewIntent::ApplicationLandscape => vec![
            ("business", "layer.business", 200.0),
            ("application", "layer.application", 340.0),
            ("technology", "layer.technology", 200.0),
        ],
        ViewIntent::Process => vec![
            ("process", "layer.process", 360.0),
            ("flow", "layer.flow", 240.0),
            ("data", "layer.data", 160.0),
            ("annotation", "layer.annotation", 120.0),
        ],
        ViewIntent::Deployment => vec![
            ("structure", "layer.structure", 200.0),
            ("deployment", "layer.deployment", 340.0),
        ],
        ViewIntent::Capability => vec![
            ("motivation", "layer.motivation", 180.0),
            ("business", "layer.business", 320.0),
            ("implementation", "layer.implementation", 220.0),
        ],
        ViewIntent::Migration => vec![
            ("business", "layer.business", 180.0),
            ("application", "layer.application", 260.0),
            ("technology", "layer.technology", 200.0),
            ("implementation", "layer.implementation", 220.0),
        ],
        ViewIntent::InterfaceReview => vec![
            ("business", "layer.business", 220.0),
            ("application", "layer.application", 320.0),
        ],
    }
}

pub fn layers_for_intent(intent: ViewIntent) -> Vec<archifier_core::LayerBand> {
    let mut y = 0.0;
    intent_layers(intent)
        .into_iter()
        .map(|(id, label_key, h)| {
            let band = archifier_core::LayerBand {
                id: id.into(),
                label: t(label_key),
                y,
                h,
                collapsed: false,
                tone: Some(id.into()),
            };
            y += h + 16.0;
            band
        })
        .collect()
}

pub fn palette(intent: ViewIntent, notation: Notation) -> Vec<archifier_metamodel::PaletteGroup> {
    let allowed = intent_layers(intent)
        .into_iter()
        .map(|(id, _, _)| id)
        .collect::<HashSet<_>>();
    repo()
        .palette(&archifier_core::default_profile_for(notation))
        .into_iter()
        .filter(|group| allowed.contains(group.layer_id.as_str()))
        .collect()
}

pub fn intent_layers_of_group(group: &archifier_metamodel::PaletteGroup) -> String {
    t(&format!("layer.{}", group.layer_id))
}

pub fn relation_choices(notation: Notation, source: &str, target: &str) -> Vec<(String, String)> {
    let repository = repo();
    let Some(profile) = repository.metamodel().profile_for(notation) else {
        return Vec::new();
    };
    profile
        .relations
        .iter()
        .filter(|relation| relation_matches(profile, source, target, &relation.id))
        .map(|relation| (relation.id.clone(), relation.label.clone()))
        .collect()
}

fn relation_matches(
    profile: &archifier_metamodel::Profile,
    source: &str,
    target: &str,
    relation: &str,
) -> bool {
    profile.relation_allowed(source, relation, target)
}

pub fn domains() -> Vec<archifier_repo::DomainSummary> {
    repo().domains()
}

/// Crée une vue et son layout depuis un modèle. Seul point d'écriture de
/// layout en dehors des commandes : c'est une création, pas une mutation.
pub fn create_view(
    intent: ViewIntent,
    domain_id: archifier_core::ObjectId,
) -> Result<archifier_core::ObjectId, archifier_repo::RepoError> {
    let repository = repo();
    let domain_name = repository
        .domain(domain_id)
        .map(|d| d.object.name)
        .unwrap_or_default();
    let name = format!("{} — {}", crate::templates::title(intent), domain_name);
    let id = repository.create_diagram(domain_id, &name, crate::templates::notation(intent))?;
    repository.save_layout(id, crate::templates::layout(intent, domain_id))?;
    Ok(id)
}

pub fn draft_shape(intent: ViewIntent, index: usize) -> DraftShape {
    let layers = layers_for_intent(intent);
    let layer = layers
        .iter()
        .find(|l| l.id == "application")
        .or_else(|| layers.get(1))
        .or_else(|| layers.first());
    let x = 80.0 + (index % 6) as f64 * 200.0;
    let y = layer.map(|l| l.y + 48.0).unwrap_or(80.0);
    DraftShape {
        id: format!("d{}", new_object_id()),
        label: t("design.untitled_shape"),
        shape: ShapeKind::Rect,
        x,
        y,
        w: 168.0,
        h: 72.0,
        status: DraftStatus::Sketch,
        suggested_metaclass: None,
        suggested_domain_id: None,
        layer_id: layer.map(|l| l.id.to_string()),
        zone_id: None,
        attachment: None,
        interface_id: None,
        version_id: None,
    }
}

pub fn category_from_key(key: &str) -> InterfaceCategory {
    match key {
        "BUSINESS" => InterfaceCategory::Business,
        "TECHNICAL" => InterfaceCategory::Technical,
        _ => InterfaceCategory::Application,
    }
}

pub fn publish_label(status: PublicationStatus) -> &'static str {
    match status {
        PublicationStatus::Draft => "design.status.sketch",
        PublicationStatus::Published => "design.status.published",
        PublicationStatus::Withdrawn => "design.status.withdrawn",
    }
}

pub fn temporal_key(status: Option<TemporalStatus>) -> Option<&'static str> {
    status.map(|s| match s {
        TemporalStatus::Scheduled => "design.temporal.scheduled",
        TemporalStatus::Active => "design.temporal.active",
        TemporalStatus::Expired => "design.temporal.expired",
    })
}

pub fn status_key(status: DraftStatus) -> &'static str {
    match status {
        DraftStatus::Sketch => "design.status.sketch",
        DraftStatus::Formalized => "design.status.formalized",
        DraftStatus::Valid => "design.status.valid",
    }
}

pub fn target_of_selection(selection: &Selection) -> Option<Target> {
    match selection {
        Selection::Node(id) => Some(Target::Node {
            node_id: id.clone(),
        }),
        Selection::Edge(id) => Some(Target::Edge {
            edge_id: id.clone(),
        }),
        Selection::Zone(id) => Some(Target::Zone {
            zone_id: id.clone(),
        }),
        Selection::Draft(id) => Some(Target::Draft {
            draft_id: id.clone(),
        }),
        Selection::None => None,
    }
}

pub fn endpoint_of(node_id: &str, is_port: bool) -> EndPoint {
    if is_port {
        EndPoint::Port {
            node_id: node_id.to_string(),
        }
    } else {
        EndPoint::Node {
            node_id: node_id.to_string(),
        }
    }
}

pub fn move_delta(x: f64, y: f64) -> archifier_core::Point {
    archifier_core::Point { x, y }
}

pub fn size_of(w: f64, h: f64) -> Size {
    Size { w, h }
}

pub fn framing_with(zoom: f64, x: f64, y: f64, focus: Vec<String>) -> ViewFraming {
    ViewFraming {
        zoom: Some(zoom.clamp(0.25, 4.0)),
        x,
        y,
        focus,
        dim_others: false,
        caption: None,
    }
}

pub fn publish_visibility() -> Visibility {
    Visibility::Public
}

pub fn zone_edge_routing() -> EdgeRouting {
    EdgeRouting::Orthogonal
}

pub fn node_is_port(layout: &DiagramLayout, node_id: &str) -> bool {
    layout
        .nodes
        .iter()
        .find(|n| n.id == node_id)
        .map(|n| n.kind == NodeKind::Interface && n.port.is_some())
        .unwrap_or(false)
}

pub fn export_svg(designer: &Designer) -> String {
    let layout = designer.layout.get_untracked();
    let facts = node_facts(&layout, None);
    let mut out = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"800\" viewBox=\"0 0 1200 800\">",
    );
    for layer in &layout.layers {
        out.push_str(&format!(
            "<rect x=\"0\" y=\"{}\" width=\"1200\" height=\"{}\" fill=\"#f7f9fd\" stroke=\"#dde2ea\"/>",
            layer.y, layer.h
        ));
        out.push_str(&format!(
            "<text x=\"12\" y=\"{}\" font-size=\"12\" fill=\"#48566a\">{}</text>",
            layer.y + 18.0,
            escape_xml(&layer.label)
        ));
    }
    for zone in &layout.zones {
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"#9fb2cc\" stroke-width=\"2\"/>",
            zone.x, zone.y, zone.w, zone.h
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"13\" fill=\"#1b2430\">{}</text>",
            zone.x + 10.0,
            zone.y + 20.0,
            escape_xml(&zone.label)
        ));
    }
    for line in edge_lines(&layout) {
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#6b7787\" stroke-width=\"1.5\"/>",
            line.x1, line.y1, line.x2, line.y2
        ));
    }
    for node in &layout.nodes {
        let Some(info) = facts.get(&node.id) else {
            continue;
        };
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"#ffffff\" stroke=\"#c6cedb\"/>",
            node.x, node.y, node.w, node.h
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"13\" font-weight=\"600\" fill=\"#1b2430\">{}</text>",
            node.x + 10.0,
            node.y + 24.0,
            escape_xml(&info.name)
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"#6b7787\">{}</text>",
            node.x + 10.0,
            node.y + 42.0,
            escape_xml(&info.type_label)
        ));
    }
    if !layout.drafts.is_empty() {
        out.push_str(&format!(
            "<text x=\"12\" y=\"780\" font-size=\"11\" fill=\"#6b7787\">{} croquis exclus de l'export</text>",
            layout.drafts.len()
        ));
    }
    out.push_str("</svg>");
    out
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn export_markdown(designer: &Designer) -> String {
    let layout = designer.layout.get_untracked();
    let facts = node_facts(&layout, None);
    let mut out = format!("# {}\n\n", designer.title());
    out.push_str(&format!(
        "- Intention : {}\n- Notation : {}\n- Révision : {}\n- Formes : {}\n- Croquis : {}\n\n",
        t(designer.intent().label_key()),
        designer.notation().label(),
        designer.revision.get_untracked(),
        layout.nodes.len(),
        layout.drafts.len()
    ));
    for layer in &layout.layers {
        out.push_str(&format!("## {}\n\n", layer.label));
        for node in layout
            .nodes
            .iter()
            .filter(|n| n.layer_id.as_deref() == Some(layer.id.as_str()))
        {
            if let Some(info) = facts.get(&node.id) {
                out.push_str(&format!(
                    "- **{}** — {} · {}\n",
                    info.name, info.type_label, info.domain_name
                ));
            }
        }
        out.push('\n');
    }
    if !layout.drafts.is_empty() {
        out.push_str(&format!(
            "\n_{} croquis exclus de l'export._\n",
            layout.drafts.len()
        ));
    }
    out
}

pub fn export_name(designer: &Designer, extension: &str) -> String {
    let raw = designer.title();
    let slug: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    format!("{}.{}", slug.trim_matches('-'), extension)
}
