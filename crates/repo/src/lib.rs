//! Dépôt local : source de vérité en mémoire, persistance derrière
//! `SnapshotStore`, index reconstruit après chaque écriture.
//!
//! Chaque commande valide via `archifier-core` et `archifier-metamodel` avant
//! de muter l'état : aucune écriture ne peut contourner une règle métier.

pub mod diagnostics;
pub mod export;
pub mod hierarchy;
pub mod map;
pub mod openapi;
pub mod review_compare;
pub mod seed;
pub mod storage;
pub mod tabs;
pub mod types;

#[cfg(target_arch = "wasm32")]
pub mod idb;

pub use archifier_core::export::{ExportBundle, ExportError, ExportFile};
pub use openapi::*;
pub use storage::{FailingStore, MemoryStore, SnapshotStore, StoreError};
pub use types::*;

use archifier_core::changeset::{
    validate_changeset, ChangeOperation, ChangeOrigin, ChangeSet, ChangeSetStatus,
    ObjectRef as CsRef,
};
use archifier_core::temporal::{publication_issues, version_temporal_status};
use archifier_core::{
    diff_values, validate_diagram, validate_domain_name, validate_interface,
    validate_interface_version, validate_object, validate_story, AuditEvent, Comment,
    ContractFormat, Dataset, Diagram, DiagramEdge, DiagramLayout, DiagramNode, DiagramNotation,
    DocumentationBlock, Domain, IdGenerator, Interface, InterfaceCategory, InterfaceConsumer,
    InterfaceVersion, Lifecycle, ModelElement, Notation, ObjectId, ObjectSummary, ObjectType,
    PublicationStatus, Relationship, RepositoryObject, Review, ReviewStatus, Revision, Severity,
    Story, TemporalStatus, ValidationError, ValidationIssue, ViewIntent, APP_VERSION,
};
use archifier_metamodel::{MetamodelRegistry, PaletteGroup, RelationResolution};
use serde_json::{json, Map, Value};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::rc::Rc;
use time::OffsetDateTime;

// ---------------------------------------------------------------------------
// Erreurs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RepoError {
    pub code: String,
    pub message: String,
    pub issues: Vec<ValidationIssue>,
}

impl RepoError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            issues: vec![],
        }
    }

    pub fn not_found(what: &str) -> Self {
        Self::new("NOT-FOUND", format!("{what} introuvable."))
    }

    pub fn from_validation(error: ValidationError) -> Self {
        let code = error
            .issues
            .first()
            .map(|i| i.code.clone())
            .unwrap_or_else(|| "VALIDATION".into());
        let message = error
            .issues
            .iter()
            .map(|i| format!("{} : {}", i.code, i.message))
            .collect::<Vec<_>>()
            .join(" · ");
        Self {
            code,
            message,
            issues: error.issues,
        }
    }

    pub fn is(&self, code: &str) -> bool {
        self.code == code
    }
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for RepoError {}

pub type RepoResult<T> = Result<T, RepoError>;

// ---------------------------------------------------------------------------
// Index
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Index {
    object_type: HashMap<ObjectId, ObjectType>,
    domain_by_slug: HashMap<String, ObjectId>,
    interface_by_key: HashMap<String, ObjectId>,
    versions_by_interface: HashMap<ObjectId, Vec<ObjectId>>,
    consumers_by_version: HashMap<ObjectId, BTreeSet<ObjectId>>,
    documentation_by_object: HashMap<ObjectId, String>,
    tokens: HashMap<ObjectId, (Vec<String>, Vec<String>)>,
    /// Index d'idempotence OpenAPI `(format, hash) -> version_id`.
    contract_by_hash: HashMap<(ContractFormat, String), ObjectId>,
}

impl Index {
    fn build(data: &Dataset) -> Self {
        let mut index = Index::default();

        for domain in &data.domains {
            index
                .object_type
                .insert(domain.object.id, ObjectType::Domain);
            index
                .domain_by_slug
                .insert(domain.slug.clone(), domain.object.id);
        }
        for element in &data.elements {
            index
                .object_type
                .insert(element.object.id, ObjectType::ModelElement);
        }
        for relationship in &data.relationships {
            index
                .object_type
                .insert(relationship.object.id, ObjectType::Relationship);
        }
        for interface in &data.interfaces {
            index
                .object_type
                .insert(interface.object.id, ObjectType::Interface);
            index
                .interface_by_key
                .insert(interface.logical_interface_key.clone(), interface.object.id);
        }
        for version in &data.interface_versions {
            index
                .versions_by_interface
                .entry(version.interface_id)
                .or_default()
                .push(version.id);
            if let (Some(format), Some(hash)) =
                (version.contract_format, version.contract_hash.as_ref())
            {
                index
                    .contract_by_hash
                    .insert((format, hash.clone()), version.id);
            }
        }
        for consumer in &data.interface_consumers {
            index
                .consumers_by_version
                .entry(consumer.interface_version_id)
                .or_default()
                .insert(consumer.consumer_object_id);
        }
        for diagram in &data.diagrams {
            index
                .object_type
                .insert(diagram.object.id, ObjectType::Diagram);
        }
        for story in &data.stories {
            index.object_type.insert(story.object.id, ObjectType::Story);
        }
        for review in &data.reviews {
            index
                .object_type
                .insert(review.object.id, ObjectType::Review);
        }

        // Documentation courante par objet.
        let mut latest: HashMap<ObjectId, &DocumentationBlock> = HashMap::new();
        for block in &data.documentation {
            let keep = latest
                .get(&block.owner_object_id)
                .map(|current| block.revision >= current.revision)
                .unwrap_or(true);
            if keep {
                latest.insert(block.owner_object_id, block);
            }
        }
        for (object_id, block) in &latest {
            index
                .documentation_by_object
                .insert(*object_id, block.markdown.clone());
        }

        // Index plein texte : titre pondéré, corps séparé.
        for id in index.object_type.keys().copied().collect::<Vec<_>>() {
            let title = index.object_name(data, id).unwrap_or_default();
            let mut body = index
                .documentation_by_object
                .get(&id)
                .cloned()
                .unwrap_or_default();
            if let Some(element) = data.elements.iter().find(|e| e.object.id == id) {
                body.push(' ');
                body.push_str(&element.metaclass);
            }
            if let Some(interface) = data.interfaces.iter().find(|i| i.object.id == id) {
                body.push(' ');
                body.push_str(&interface.logical_interface_key);
                for version in data
                    .interface_versions
                    .iter()
                    .filter(|v| v.interface_id == id)
                {
                    body.push(' ');
                    body.push_str(&version.semantic_version);
                }
            }
            index.tokens.insert(id, (tokenize(&title), tokenize(&body)));
        }

        index
    }

    fn object_name(&self, data: &Dataset, id: ObjectId) -> Option<String> {
        data.domains
            .iter()
            .find(|d| d.object.id == id)
            .map(|d| d.object.name.clone())
            .or_else(|| {
                data.elements
                    .iter()
                    .find(|e| e.object.id == id)
                    .map(|e| e.object.name.clone())
            })
            .or_else(|| {
                data.relationships
                    .iter()
                    .find(|r| r.object.id == id)
                    .map(|r| r.object.name.clone())
            })
            .or_else(|| {
                data.interfaces
                    .iter()
                    .find(|i| i.object.id == id)
                    .map(|i| i.object.name.clone())
            })
            .or_else(|| {
                data.diagrams
                    .iter()
                    .find(|d| d.object.id == id)
                    .map(|d| d.object.name.clone())
            })
            .or_else(|| {
                data.stories
                    .iter()
                    .find(|s| s.object.id == id)
                    .map(|s| s.object.name.clone())
            })
            .or_else(|| {
                data.reviews
                    .iter()
                    .find(|r| r.object.id == id)
                    .map(|r| r.object.name.clone())
            })
    }
}

fn normalize_token(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars().flat_map(char::to_lowercase) {
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
            out.push(mapped);
        } else {
            out.push(' ');
        }
    }
    out
}

fn tokenize(text: &str) -> Vec<String> {
    normalize_token(text)
        .split_whitespace()
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_string())
        .collect()
}

/// Distance d'édition bornée, pour la tolérance aux fautes de frappe.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > 2 {
        return 3;
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

// ---------------------------------------------------------------------------
// État interne
// ---------------------------------------------------------------------------

struct Inner {
    data: Dataset,
    index: Index,
    dirty: bool,
    touched: Vec<ObjectId>,
}

impl Inner {
    fn reindex(&mut self) {
        self.index = Index::build(&self.data);
    }

    fn find_domain(&self, id: ObjectId) -> Option<&Domain> {
        self.data.domains.iter().find(|d| d.object.id == id)
    }

    fn find_element(&self, id: ObjectId) -> Option<&ModelElement> {
        self.data.elements.iter().find(|e| e.object.id == id)
    }

    fn find_interface(&self, id: ObjectId) -> Option<&Interface> {
        self.data.interfaces.iter().find(|i| i.object.id == id)
    }

    fn domain_name(&self, id: ObjectId) -> String {
        self.find_domain(id)
            .map(|d| d.object.name.clone())
            .unwrap_or_else(|| "—".into())
    }

    fn active_domain(&self, id: ObjectId) -> bool {
        self.find_domain(id)
            .map(|d| d.object.is_active())
            .unwrap_or(false)
    }

    fn object_exists(&self, id: ObjectId) -> bool {
        self.index.object_type.contains_key(&id)
    }

    fn is_active(&self, id: ObjectId) -> bool {
        self.object_by_id(id)
            .map(|o| o.is_active())
            .unwrap_or(false)
    }

    fn object_by_id(&self, id: ObjectId) -> Option<RepositoryObject> {
        if let Some(d) = self.find_domain(id) {
            return Some(d.object.clone());
        }
        if let Some(e) = self.find_element(id) {
            return Some(e.object.clone());
        }
        if let Some(r) = self.data.relationships.iter().find(|r| r.object.id == id) {
            return Some(r.object.clone());
        }
        if let Some(i) = self.find_interface(id) {
            return Some(i.object.clone());
        }
        if let Some(d) = self.data.diagrams.iter().find(|d| d.object.id == id) {
            return Some(d.object.clone());
        }
        if let Some(s) = self.data.stories.iter().find(|s| s.object.id == id) {
            return Some(s.object.clone());
        }
        if let Some(r) = self.data.reviews.iter().find(|r| r.object.id == id) {
            return Some(r.object.clone());
        }
        None
    }

    fn require_active(&self, id: ObjectId) -> RepoResult<RepositoryObject> {
        let object = self
            .object_by_id(id)
            .ok_or_else(|| RepoError::not_found("Objet"))?;
        if !object.is_active() {
            return Err(RepoError::new(
                "OBJ-007",
                format!("« {} » est archivé : lecture seule.", object.name),
            ));
        }
        Ok(object)
    }

    fn snapshot(&self, id: ObjectId) -> Option<Value> {
        if let Some(d) = self.find_domain(id) {
            return serde_json::to_value(d).ok();
        }
        if let Some(e) = self.find_element(id) {
            return serde_json::to_value(e).ok();
        }
        if let Some(r) = self.data.relationships.iter().find(|r| r.object.id == id) {
            return serde_json::to_value(r).ok();
        }
        if let Some(i) = self.find_interface(id) {
            return serde_json::to_value(i).ok();
        }
        if let Some(d) = self.data.diagrams.iter().find(|d| d.object.id == id) {
            return serde_json::to_value(d).ok();
        }
        if let Some(s) = self.data.stories.iter().find(|s| s.object.id == id) {
            return serde_json::to_value(s).ok();
        }
        if let Some(r) = self.data.reviews.iter().find(|r| r.object.id == id) {
            return serde_json::to_value(r).ok();
        }
        None
    }

    fn record(&mut self, id: ObjectId, summary: impl Into<String>, now: OffsetDateTime, by: &str) {
        let revision = self.object_by_id(id).map(|o| o.revision).unwrap_or(1);
        let value = self.snapshot(id).unwrap_or(Value::Null);
        self.data
            .revisions
            .entry(id.to_string())
            .or_default()
            .insert(revision.to_string(), value);
        self.data.audit.push(AuditEvent {
            id: ObjectId::from_parts(
                now.unix_timestamp_nanos() as u64 / 1_000_000,
                revision as u128 + 7,
            ),
            actor_id: by.to_string(),
            operation: summary.into(),
            object_id: Some(id),
            before_revision: revision.checked_sub(1),
            after_revision: Some(revision),
            created_at: now,
            metadata: json!({ "app": APP_VERSION }),
        });
        self.touched.push(id);
    }

    fn sorted_summaries(&self, filter: impl Fn(ObjectType) -> bool) -> Vec<ObjectSummary> {
        let mut out = Vec::new();
        for (id, object_type) in &self.index.object_type {
            if !filter(*object_type) {
                continue;
            }
            if let Some(summary) = self.summarize(*id) {
                out.push(summary);
            }
        }
        out.sort_by_key(|s| s.id);
        out
    }

    fn summarize(&self, id: ObjectId) -> Option<ObjectSummary> {
        let object = self.object_by_id(id)?;
        let mut summary = ObjectSummary {
            id,
            object_type: object.object_type,
            name: object.name.clone(),
            domain_id: object.responsible_domain_id,
            domain_name: self.domain_name(object.responsible_domain_id),
            notation: None,
            metaclass: None,
            subtitle: None,
            lifecycle: object.lifecycle,
        };
        match object.object_type {
            ObjectType::ModelElement => {
                if let Some(element) = self.find_element(id) {
                    summary.notation = Some(element.notation);
                    summary.metaclass = Some(element.metaclass.clone());
                    summary.subtitle = Some(element.metaclass.clone());
                }
            }
            ObjectType::Interface => {
                if let Some(interface) = self.find_interface(id) {
                    summary.subtitle = Some(format!(
                        "{} · {}",
                        interface.category.label(),
                        interface.logical_interface_key
                    ));
                }
            }
            ObjectType::Relationship => {
                if let Some(rel) = self.data.relationships.iter().find(|r| r.object.id == id) {
                    summary.notation = Some(rel.notation);
                    summary.subtitle = Some(rel.relation_type.clone());
                }
            }
            ObjectType::Diagram => {
                if let Some(diagram) = self.data.diagrams.iter().find(|d| d.object.id == id) {
                    summary.subtitle = Some(format!(
                        "{} · {} nœuds",
                        notation_label(diagram.notation),
                        diagram.layout.nodes.len()
                    ));
                }
            }
            _ => {}
        }
        Some(summary)
    }
}

fn notation_label(notation: DiagramNotation) -> &'static str {
    match notation {
        DiagramNotation::Archimate => "ArchiMate",
        DiagramNotation::Bpmn => "BPMN",
        DiagramNotation::Uml => "UML",
        DiagramNotation::Free => "Libre",
    }
}

// ---------------------------------------------------------------------------
// Dépôt
// ---------------------------------------------------------------------------

pub struct Repository {
    metamodel: MetamodelRegistry,
    store: Rc<dyn SnapshotStore>,
    inner: RefCell<Inner>,
    now_ms: Cell<u64>,
    ids: RefCell<IdGenerator>,
    persist_error: RefCell<Option<String>>,
    current_user: RefCell<String>,
    storage_evicted: Cell<bool>,
    profile_error: RefCell<Option<String>>,
}

impl Repository {
    pub fn new(store: Rc<dyn SnapshotStore>, now_ms: u64, seed: u64) -> Self {
        let (metamodel, profile_error) = match MetamodelRegistry::load_embedded() {
            Ok(registry) => (registry, None),
            Err(error) => (MetamodelRegistry::default(), Some(error.to_string())),
        };
        Self {
            metamodel,
            store,
            inner: RefCell::new(Inner {
                data: Dataset::default(),
                index: Index::default(),
                dirty: false,
                touched: vec![],
            }),
            now_ms: Cell::new(now_ms),
            ids: RefCell::new(IdGenerator::new(seed)),
            persist_error: RefCell::new(None),
            current_user: RefCell::new("Utilisateur".into()),
            storage_evicted: Cell::new(false),
            profile_error: RefCell::new(profile_error),
        }
    }

    pub fn from_json(
        store: Rc<dyn SnapshotStore>,
        json: &str,
        now_ms: u64,
        seed: u64,
    ) -> RepoResult<Self> {
        let repo = Self::new(store, now_ms, seed);
        repo.load_json(json)?;
        Ok(repo)
    }

    // -- horloge et identité -------------------------------------------------

    pub fn set_now_ms(&self, ms: u64) {
        self.now_ms.set(ms);
    }

    pub fn now_ms(&self) -> u64 {
        self.now_ms.get()
    }

    pub fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp_nanos(self.now_ms.get() as i128 * 1_000_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
    }

    pub fn set_current_user(&self, user: impl Into<String>) {
        *self.current_user.borrow_mut() = user.into();
    }

    pub fn current_user(&self) -> String {
        self.current_user.borrow().clone()
    }

    pub fn users(&self) -> Vec<archifier_core::User> {
        self.inner.borrow().data.users.clone()
    }

    pub fn persist_error(&self) -> Option<String> {
        self.persist_error.borrow().clone()
    }

    pub fn profile_error(&self) -> Option<String> {
        self.profile_error.borrow().clone()
    }

    pub fn storage_evicted(&self) -> bool {
        self.storage_evicted.get() || self.store.eviction_suspected()
    }

    pub fn store_error(&self) -> Option<String> {
        self.store.take_error()
    }

    /// Efface le stockage persistant et l'état en mémoire. L'UI doit recharger
    /// ensuite ; le drapeau de réamorçage est géré par la couche d'état.
    pub fn reset_storage(&self) -> RepoResult<()> {
        self.store
            .reset()
            .map_err(|error| RepoError::new("STORE-003", error.to_string()))?;
        let mut guard = self.inner.borrow_mut();
        guard.data = Dataset::default();
        guard.reindex();
        Ok(())
    }

    pub fn metamodel(&self) -> &MetamodelRegistry {
        &self.metamodel
    }

    fn new_id(&self) -> ObjectId {
        self.ids.borrow_mut().next_id(self.now_ms.get())
    }

    // -- persistance ---------------------------------------------------------

    fn mutate<T>(&self, f: impl FnOnce(&mut Inner) -> RepoResult<T>) -> RepoResult<T> {
        let mut guard = self.inner.borrow_mut();
        let result = f(&mut guard)?;
        guard.reindex();
        guard.dirty = false;
        let json = serde_json::to_string(&guard.data)
            .map_err(|e| RepoError::new("STORE-001", format!("Sérialisation impossible : {e}")))?;
        drop(guard);
        if let Err(error) = self.store.save(&json) {
            *self.persist_error.borrow_mut() = Some(error.to_string());
        } else {
            *self.persist_error.borrow_mut() = None;
        }
        Ok(result)
    }

    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(&self.inner.borrow().data).unwrap_or_else(|_| "{}".into())
    }

    pub fn load_json(&self, json: &str) -> RepoResult<()> {
        let data: Dataset = serde_json::from_str(json)
            .map_err(|e| RepoError::new("STORE-002", format!("Snapshot illisible : {e}")))?;
        {
            let mut guard = self.inner.borrow_mut();
            guard.data = data;
            guard.reindex();
        }
        self.store
            .save(json)
            .map_err(|error| RepoError::new("STORE-004", error.to_string()))?;
        Ok(())
    }

    /// Reconstruit l'index après une écriture externe (autre onglet).
    pub fn refresh(&self) {
        self.inner.borrow_mut().reindex();
    }

    // -- lectures de base ----------------------------------------------------

    pub fn domains(&self) -> Vec<DomainSummary> {
        let inner = self.inner.borrow();
        let mut out: Vec<DomainSummary> = inner
            .data
            .domains
            .iter()
            .map(|domain| {
                let counts = DomainCounts {
                    elements: inner
                        .data
                        .elements
                        .iter()
                        .filter(|e| e.object.responsible_domain_id == domain.object.id)
                        .count(),
                    interfaces: inner
                        .data
                        .interfaces
                        .iter()
                        .filter(|i| i.provider_domain_id == domain.object.id)
                        .count(),
                    diagrams: inner
                        .data
                        .diagrams
                        .iter()
                        .filter(|d| d.object.responsible_domain_id == domain.object.id)
                        .count(),
                    stories: inner
                        .data
                        .stories
                        .iter()
                        .filter(|s| s.object.responsible_domain_id == domain.object.id)
                        .count(),
                    reviews: inner
                        .data
                        .reviews
                        .iter()
                        .filter(|r| r.domain_id == domain.object.id)
                        .count(),
                    decisions: 0,
                };
                DomainSummary {
                    id: domain.object.id,
                    name: domain.object.name.clone(),
                    slug: domain.slug.clone(),
                    description: domain.description.clone(),
                    lifecycle: domain.object.lifecycle,
                    counts,
                }
            })
            .collect();
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out
    }

    pub fn domain(&self, id: ObjectId) -> Option<Domain> {
        self.inner.borrow().find_domain(id).cloned()
    }

    pub fn domain_overview(&self, id: ObjectId) -> RepoResult<DomainOverview> {
        let inner = self.inner.borrow();
        let domain = inner
            .find_domain(id)
            .ok_or_else(|| RepoError::not_found("Domaine"))?;
        let now = self.now();

        let published_diagrams = inner
            .data
            .diagrams
            .iter()
            .filter(|d| {
                d.object.responsible_domain_id == id && d.object.lifecycle == Lifecycle::Active
            })
            .map(|d| {
                let mut r = ObjectRef::of(ObjectType::Diagram, d.object.id, d.object.name.clone());
                r.subtitle = Some(format!(
                    "{} · {} nœuds",
                    notation_label(d.notation),
                    d.layout.nodes.len()
                ));
                r
            })
            .collect();

        let exposed: Vec<InterfaceRow> = inner
            .data
            .interfaces
            .iter()
            .filter(|i| i.provider_domain_id == id)
            .map(|i| interface_row(&inner, i, now))
            .collect();

        // Interfaces consommées par un objet du domaine.
        let mut consumed_ids = BTreeSet::new();
        for consumer in &inner.data.interface_consumers {
            if consumer.consumer_domain_id == id {
                if let Some(version) = inner
                    .data
                    .interface_versions
                    .iter()
                    .find(|v| v.id == consumer.interface_version_id)
                {
                    consumed_ids.insert(version.interface_id);
                }
            }
        }
        let consumed: Vec<InterfaceRow> = inner
            .data
            .interfaces
            .iter()
            .filter(|i| consumed_ids.contains(&i.object.id))
            .map(|i| interface_row(&inner, i, now))
            .collect();

        let mut anomalies = Vec::new();
        for consumer in &inner.data.interface_consumers {
            if consumer.consumer_domain_id != id {
                continue;
            }
            if let Some(version) = inner
                .data
                .interface_versions
                .iter()
                .find(|v| v.id == consumer.interface_version_id)
            {
                let interface_name = inner
                    .find_interface(version.interface_id)
                    .map(|i| i.object.name.clone())
                    .unwrap_or_default();
                match version_temporal_status(version, now) {
                    Some(TemporalStatus::Expired) => anomalies.push(Anomaly {
                        code: AnomalyCode::DependencyOnExpiredInterface,
                        message: format!(
                            "{} {} est expirée depuis le {}.",
                            interface_name,
                            version.semantic_version,
                            version
                                .available_until
                                .map(format_date)
                                .unwrap_or_else(|| "—".into())
                        ),
                        object_id: version.interface_id,
                        href: format!("#/view/interfaces/{}", version.interface_id),
                        detail: None,
                    }),
                    Some(TemporalStatus::Scheduled) => anomalies.push(Anomaly {
                        code: AnomalyCode::ConsumerOnScheduledInterface,
                        message: format!(
                            "{} {} n'est pas encore disponible.",
                            interface_name, version.semantic_version
                        ),
                        object_id: version.interface_id,
                        href: format!("#/view/interfaces/{}", version.interface_id),
                        detail: None,
                    }),
                    _ => {}
                }
            }
        }

        let dependencies_out = domain_links(&inner, id, Direction::Out);
        let dependencies_in = domain_links(&inner, id, Direction::In);

        let stories = inner
            .data
            .stories
            .iter()
            .filter(|s| {
                s.object.responsible_domain_id == id || story_mentions_domain(&inner.data, s, id)
            })
            .map(|s| ObjectRef::of(ObjectType::Story, s.object.id, s.object.name.clone()))
            .collect();

        let reviews = inner
            .data
            .reviews
            .iter()
            .filter(|r| r.domain_id == id)
            .map(|r| {
                let mut o = ObjectRef::of(ObjectType::Review, r.object.id, r.object.name.clone());
                o.subtitle = Some(review_status_label(r.status).into());
                o
            })
            .collect();

        let counts = DomainCounts {
            elements: inner
                .data
                .elements
                .iter()
                .filter(|e| e.object.responsible_domain_id == id)
                .count(),
            interfaces: inner
                .data
                .interfaces
                .iter()
                .filter(|i| i.provider_domain_id == id)
                .count(),
            diagrams: inner
                .data
                .diagrams
                .iter()
                .filter(|d| d.object.responsible_domain_id == id)
                .count(),
            stories: inner
                .data
                .stories
                .iter()
                .filter(|s| s.object.responsible_domain_id == id)
                .count(),
            reviews: inner
                .data
                .reviews
                .iter()
                .filter(|r| r.domain_id == id)
                .count(),
            decisions: anomalies.len(),
        };

        Ok(DomainOverview {
            domain: DomainSummary {
                id: domain.object.id,
                name: domain.object.name.clone(),
                slug: domain.slug.clone(),
                description: domain.description.clone(),
                lifecycle: domain.object.lifecycle,
                counts,
            },
            published_diagrams,
            exposed_interfaces: exposed,
            consumed_interfaces: consumed,
            dependencies_out,
            dependencies_in,
            stories,
            reviews,
            anomalies,
        })
    }

    pub fn elements(&self, filter: &ElementFilter) -> Vec<ObjectSummary> {
        let inner = self.inner.borrow();
        let query = filter.query.as_deref().map(normalize_token);
        let mut out: Vec<ObjectSummary> = inner
            .data
            .elements
            .iter()
            .filter(|e| {
                filter
                    .domain_id
                    .map(|d| e.object.responsible_domain_id == d)
                    .unwrap_or(true)
                    && filter.notation.map(|n| e.notation == n).unwrap_or(true)
                    && query
                        .as_ref()
                        .map(|q| normalize_token(&e.object.name).contains(q.as_str()))
                        .unwrap_or(true)
            })
            .filter_map(|e| inner.summarize(e.object.id))
            .collect();
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out
    }

    pub fn element(&self, id: ObjectId) -> Option<ModelElement> {
        self.inner.borrow().find_element(id).cloned()
    }

    pub fn element_detail(&self, id: ObjectId) -> Option<ElementDetail> {
        let inner = self.inner.borrow();
        let element = inner.find_element(id)?.clone();
        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();
        for relationship in &inner.data.relationships {
            let other_id = if relationship.source_id == id {
                Some(relationship.target_id)
            } else if relationship.target_id == id {
                Some(relationship.source_id)
            } else {
                None
            };
            let Some(other_id) = other_id else { continue };
            let Some(other) = inner.object_by_id(other_id) else {
                continue;
            };
            let view = RelationshipView {
                id: relationship.object.id,
                name: relationship.object.name.clone(),
                relation_type: relationship.relation_type.clone(),
                notation: relationship.notation,
                other_id,
                other_name: other.name.clone(),
                other_type: other.object_type,
                href: format!("#/view/{}/{}", other.object_type.route_segment(), other_id),
            };
            if relationship.source_id == id {
                outgoing.push(view);
            } else {
                incoming.push(view);
            }
        }
        let usages = usages_of(&inner, id);
        Some(ElementDetail {
            domain_name: inner.domain_name(element.object.responsible_domain_id),
            parent_name: element
                .parent_id
                .and_then(|p| inner.object_by_id(p))
                .map(|o| o.name),
            element,
            outgoing,
            incoming,
            usages,
            documentation: inner
                .index
                .documentation_by_object
                .get(&id)
                .cloned()
                .unwrap_or_default(),
        })
    }

    pub fn dependencies(
        &self,
        root: ObjectId,
        direction: Direction,
        depth: usize,
    ) -> RepoResult<Graph> {
        let inner = self.inner.borrow();
        if !inner.object_exists(root) {
            return Err(RepoError::not_found("Objet"));
        }
        let depth = depth.clamp(1, 6);
        let mut nodes = Vec::new();
        let mut seen: HashSet<ObjectId> = HashSet::new();
        seen.insert(root);
        let mut queue: VecDeque<(ObjectId, usize, Vec<ObjectId>)> = VecDeque::new();
        queue.push_back((root, 0, vec![root]));
        let mut truncated = false;

        while let Some((current, level, path)) = queue.pop_front() {
            if level >= depth {
                // §5.9 : `truncated` seulement si un voisin réel dépasse la
                // borne, jamais parce qu'on a simplement atteint la borne.
                if neighbours(&inner, current, direction)
                    .iter()
                    .any(|(other, _, _)| !seen.contains(other))
                {
                    truncated = true;
                }
                continue;
            }
            for (other, relation, edge_direction) in neighbours(&inner, current, direction) {
                if !seen.insert(other) {
                    continue;
                }
                let Some(object) = inner.object_by_id(other) else {
                    continue;
                };
                let mut next_path = path.clone();
                next_path.push(other);
                nodes.push(DependencyNode {
                    object_id: other,
                    name: object.name.clone(),
                    object_type: object.object_type,
                    domain_id: object.responsible_domain_id,
                    domain_name: inner.domain_name(object.responsible_domain_id),
                    depth: level + 1,
                    path: next_path.clone(),
                    relation: relation.to_string(),
                    direction: edge_direction,
                    lifecycle: object.lifecycle,
                });
                queue.push_back((other, level + 1, next_path));
                if nodes.len() >= 500 {
                    truncated = true;
                    queue.clear();
                    break;
                }
            }
        }
        nodes.sort_by(|a, b| (a.depth, a.path.as_slice()).cmp(&(b.depth, b.path.as_slice())));
        Ok(Graph {
            root,
            nodes,
            truncated,
        })
    }

    pub fn interfaces(&self, filter: &InterfaceFilter) -> Vec<InterfaceRow> {
        let inner = self.inner.borrow();
        let now = self.now();
        let query = filter.query.as_deref().map(normalize_token);
        let mut out: Vec<InterfaceRow> = inner
            .data
            .interfaces
            .iter()
            .filter(|i| {
                filter
                    .domain_id
                    .map(|d| i.provider_domain_id == d || i.object.responsible_domain_id == d)
                    .unwrap_or(true)
                    && filter.category.map(|c| i.category == c).unwrap_or(true)
                    && query
                        .as_ref()
                        .map(|q| normalize_token(&i.object.name).contains(q.as_str()))
                        .unwrap_or(true)
            })
            .map(|i| interface_row(&inner, i, now))
            .filter(|row| filter.status.map(|s| row.status == Some(s)).unwrap_or(true))
            .filter(|row| {
                filter
                    .consumer_id
                    .map(|consumer| {
                        inner.data.interface_consumers.iter().any(|c| {
                            c.consumer_object_id == consumer
                                && inner.data.interface_versions.iter().any(|v| {
                                    v.id == c.interface_version_id && v.interface_id == row.id
                                })
                        })
                    })
                    .unwrap_or(true)
            })
            .collect();
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out
    }

    pub fn interface_detail(&self, id: ObjectId) -> Option<InterfaceDetail> {
        let inner = self.inner.borrow();
        let interface = inner.find_interface(id)?.clone();
        let now = self.now();
        let mut versions: Vec<VersionSummary> = inner
            .data
            .interface_versions
            .iter()
            .filter(|v| v.interface_id == id)
            .map(|v| VersionSummary {
                version: v.clone(),
                status: version_temporal_status(v, now),
                consumer_count: inner
                    .index
                    .consumers_by_version
                    .get(&v.id)
                    .map(|s| s.len())
                    .unwrap_or(0),
            })
            .collect();
        versions.sort_by(|a, b| {
            compare_semver(&a.version.semantic_version, &b.version.semantic_version)
        });

        let mut consumers = Vec::new();
        for version in &versions {
            if let Some(set) = inner.index.consumers_by_version.get(&version.version.id) {
                for consumer_id in set {
                    if consumers.iter().any(|c: &ObjectRef| c.id == *consumer_id) {
                        continue;
                    }
                    if let Some(object) = inner.object_by_id(*consumer_id) {
                        consumers.push(ObjectRef::of(
                            object.object_type,
                            object.id,
                            object.name.clone(),
                        ));
                    }
                }
            }
        }

        let diagrams = inner
            .data
            .diagrams
            .iter()
            .filter(|d| d.layout.nodes.iter().any(|n| n.object_id == id))
            .map(|d| ObjectRef::of(ObjectType::Diagram, d.object.id, d.object.name.clone()))
            .collect();

        Some(InterfaceDetail {
            provider_domain_name: inner.domain_name(interface.provider_domain_id),
            documentation: inner
                .index
                .documentation_by_object
                .get(&id)
                .cloned()
                .unwrap_or_default(),
            interface,
            versions,
            consumers,
            diagrams,
        })
    }

    pub fn timeline(&self, id: ObjectId) -> RepoResult<Timeline> {
        let inner = self.inner.borrow();
        let interface = inner
            .find_interface(id)
            .ok_or_else(|| RepoError::not_found("Interface"))?;
        let now = self.now();
        let mut segments: Vec<TimelineSegment> = inner
            .data
            .interface_versions
            .iter()
            .filter(|v| v.interface_id == id)
            .map(|v| TimelineSegment {
                version_id: v.id,
                semantic_version: v.semantic_version.clone(),
                status: version_temporal_status(v, now),
                publication_status: v.publication_status,
                from: v.available_from,
                until: v.available_until,
                consumers: inner
                    .index
                    .consumers_by_version
                    .get(&v.id)
                    .map(|s| s.len())
                    .unwrap_or(0),
                breaking_change: v.breaking_change,
            })
            .collect();
        segments.sort_by(|a, b| compare_semver(&a.semantic_version, &b.semantic_version));

        // Chevauchements entre versions publiées.
        let publishes: Vec<&InterfaceVersion> = inner
            .data
            .interface_versions
            .iter()
            .filter(|v| {
                v.interface_id == id && v.publication_status == PublicationStatus::Published
            })
            .collect();
        let mut overlaps = Vec::new();
        for i in 0..publishes.len() {
            for j in (i + 1)..publishes.len() {
                if let (Some(af), Some(au), Some(bf), Some(bu)) = (
                    publishes[i].available_from,
                    publishes[i].available_until,
                    publishes[j].available_from,
                    publishes[j].available_until,
                ) {
                    let from = af.max(bf);
                    let until = au.min(bu);
                    if from < until {
                        overlaps.push(TimelineOverlap {
                            from,
                            until,
                            versions: vec![
                                publishes[i].semantic_version.clone(),
                                publishes[j].semantic_version.clone(),
                            ],
                        });
                    }
                }
            }
        }

        Ok(Timeline {
            interface_id: id,
            interface_name: interface.object.name.clone(),
            now,
            segments,
            overlaps,
        })
    }

    pub fn diagrams(&self, domain_id: Option<ObjectId>) -> Vec<ObjectSummary> {
        let inner = self.inner.borrow();
        let mut out: Vec<ObjectSummary> = inner
            .data
            .diagrams
            .iter()
            .filter(|d| {
                domain_id
                    .map(|x| d.object.responsible_domain_id == x)
                    .unwrap_or(true)
            })
            .filter_map(|d| inner.summarize(d.object.id))
            .collect();
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out
    }

    pub fn diagram_detail(&self, id: ObjectId) -> Option<DiagramDetail> {
        let inner = self.inner.borrow();
        let diagram = inner
            .data
            .diagrams
            .iter()
            .find(|d| d.object.id == id)?
            .clone();
        let unknown = diagram
            .layout
            .nodes
            .iter()
            .filter(|n| !inner.object_exists(n.object_id))
            .count();
        Some(DiagramDetail {
            domain_name: inner.domain_name(diagram.object.responsible_domain_id),
            node_count: diagram.layout.nodes.len(),
            unknown_nodes: unknown,
            diagram,
        })
    }

    pub fn stories(&self) -> Vec<ObjectSummary> {
        self.inner
            .borrow()
            .sorted_summaries(|t| t == ObjectType::Story)
    }

    pub fn story_detail(&self, id: ObjectId) -> Option<StoryDetail> {
        let inner = self.inner.borrow();
        let story = inner
            .data
            .stories
            .iter()
            .find(|s| s.object.id == id)?
            .clone();
        Some(StoryDetail {
            domain_name: inner.domain_name(story.object.responsible_domain_id),
            block_count: story
                .structure
                .chapters
                .iter()
                .map(|c| c.blocks.len())
                .sum(),
            story,
        })
    }

    pub fn playback(&self, id: ObjectId) -> Option<Playback> {
        let inner = self.inner.borrow();
        let story = inner.data.stories.iter().find(|s| s.object.id == id)?;
        let mut chapters = Vec::new();
        for chapter in &story.structure.chapters {
            let mut blocks = Vec::new();
            for block in &chapter.blocks {
                let mut playback = PlaybackBlock {
                    block_id: block.id().to_string(),
                    kind: "MARKDOWN".into(),
                    title: String::new(),
                    markdown_html: None,
                    diagram: None,
                    element: None,
                    interface: None,
                    transition_label: None,
                    transition_target: None,
                    focus_element_ids: vec![],
                    dimmed_element_ids: vec![],
                    zoom: archifier_core::DEFAULT_ZOOM,
                    caption: None,
                };
                match block {
                    archifier_core::StoryBlock::Markdown { markdown, .. } => {
                        playback.title = "Documentation".into();
                        playback.markdown_html = Some(archifier_core::markdown::render(markdown));
                    }
                    archifier_core::StoryBlock::Diagram {
                        diagram_id,
                        framing,
                        ..
                    } => {
                        playback.kind = "DIAGRAM".into();
                        if let Some(diagram) = inner
                            .data
                            .diagrams
                            .iter()
                            .find(|d| d.object.id == *diagram_id)
                        {
                            playback.title = diagram.object.name.clone();
                            playback.diagram = Some(diagram.clone());
                        }
                        playback.focus_element_ids = framing.focus_element_ids.clone();
                        playback.dimmed_element_ids = framing.dimmed_element_ids.clone();
                        playback.zoom = archifier_core::effective_zoom(framing);
                        playback.caption = framing.caption.clone();
                    }
                    archifier_core::StoryBlock::Element { element_id, .. } => {
                        playback.kind = "ELEMENT".into();
                        if let Some(object) = inner.object_by_id(*element_id) {
                            playback.title = object.name.clone();
                            playback.element = Some(ObjectRef::of(
                                object.object_type,
                                object.id,
                                object.name.clone(),
                            ));
                        }
                    }
                    archifier_core::StoryBlock::Interface { interface_id, .. } => {
                        playback.kind = "INTERFACE".into();
                        if let Some(interface) = inner.find_interface(*interface_id) {
                            playback.title = interface.object.name.clone();
                            playback.interface = Some(InterfaceDetail {
                                interface: interface.clone(),
                                provider_domain_name: inner
                                    .domain_name(interface.provider_domain_id),
                                versions: vec![],
                                consumers: vec![],
                                diagrams: vec![],
                                documentation: String::new(),
                            });
                        }
                    }
                    archifier_core::StoryBlock::View { view, caption, .. } => {
                        playback.kind = "VIEW".into();
                        playback.title = view.clone();
                        playback.caption = caption.clone();
                    }
                    archifier_core::StoryBlock::Transition {
                        label,
                        target_block_id,
                        ..
                    } => {
                        playback.kind = "TRANSITION".into();
                        playback.title = label.clone();
                        playback.transition_label = Some(label.clone());
                        playback.transition_target = Some(target_block_id.clone());
                    }
                }
                blocks.push(playback);
            }
            chapters.push(PlaybackChapter {
                id: chapter.id.clone(),
                title: chapter.title.clone(),
                blocks,
            });
        }
        Some(Playback {
            story_id: id,
            title: story.object.name.clone(),
            pinned_revisions: story.structure.pinned_revisions.clone(),
            pinned_versions: story.structure.pinned_versions.clone(),
            chapters,
        })
    }

    pub fn reviews(&self) -> Vec<ObjectSummary> {
        self.inner
            .borrow()
            .sorted_summaries(|t| t == ObjectType::Review)
    }

    pub fn review_detail(&self, id: ObjectId) -> Option<ReviewDetail> {
        let inner = self.inner.borrow();
        let review = inner
            .data
            .reviews
            .iter()
            .find(|r| r.object.id == id)?
            .clone();
        let now = self.now();
        let exposed: Vec<InterfaceRow> = inner
            .data
            .interfaces
            .iter()
            .filter(|i| i.provider_domain_id == review.domain_id)
            .map(|i| interface_row(&inner, i, now))
            .collect();
        let mut consumed_ids = BTreeSet::new();
        for consumer in &inner.data.interface_consumers {
            if consumer.consumer_domain_id == review.domain_id {
                if let Some(version) = inner
                    .data
                    .interface_versions
                    .iter()
                    .find(|v| v.id == consumer.interface_version_id)
                {
                    consumed_ids.insert(version.interface_id);
                }
            }
        }
        let consumed: Vec<InterfaceRow> = inner
            .data
            .interfaces
            .iter()
            .filter(|i| consumed_ids.contains(&i.object.id))
            .map(|i| interface_row(&inner, i, now))
            .collect();
        let mut anomalies = Vec::new();
        for consumer in &inner.data.interface_consumers {
            if consumer.consumer_domain_id != review.domain_id {
                continue;
            }
            if let Some(version) = inner
                .data
                .interface_versions
                .iter()
                .find(|v| v.id == consumer.interface_version_id)
            {
                if version_temporal_status(version, now) == Some(TemporalStatus::Expired) {
                    let name = inner
                        .find_interface(version.interface_id)
                        .map(|i| i.object.name.clone())
                        .unwrap_or_default();
                    anomalies.push(Anomaly {
                        code: AnomalyCode::DependencyOnExpiredInterface,
                        message: format!("{} {} est expirée.", name, version.semantic_version),
                        object_id: version.interface_id,
                        href: format!("#/view/interfaces/{}", version.interface_id),
                        detail: version.available_until.map(format_date),
                    });
                }
            }
        }
        let comments = inner
            .data
            .comments
            .iter()
            .filter(|c| c.target_object_id == id)
            .cloned()
            .collect();
        Some(ReviewDetail {
            domain_name: inner.domain_name(review.domain_id),
            review,
            anomalies,
            comments,
            exposed,
            consumed,
        })
    }

    pub fn relationship_endpoints(&self, id: ObjectId) -> Option<(ObjectId, ObjectId)> {
        let inner = self.inner.borrow();
        inner
            .data
            .relationships
            .iter()
            .find(|relationship| relationship.object.id == id)
            .map(|relationship| (relationship.source_id, relationship.target_id))
    }

    /// Relation complète, en lecture seule (résolution de type pour le rendu).
    pub fn relationship(&self, id: ObjectId) -> Option<Relationship> {
        self.inner
            .borrow()
            .data
            .relationships
            .iter()
            .find(|relationship| relationship.object.id == id)
            .cloned()
    }

    pub fn usages(&self, id: ObjectId) -> Usages {
        usages_of(&self.inner.borrow(), id)
    }

    pub fn documentation(&self, id: ObjectId) -> String {
        self.inner
            .borrow()
            .index
            .documentation_by_object
            .get(&id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn documentation_history(&self, id: ObjectId) -> Vec<RevisionMeta> {
        let inner = self.inner.borrow();
        let mut blocks: Vec<&DocumentationBlock> = inner
            .data
            .documentation
            .iter()
            .filter(|b| b.owner_object_id == id)
            .collect();
        blocks.sort_by_key(|b| std::cmp::Reverse(b.revision));
        blocks
            .into_iter()
            .map(|b| RevisionMeta {
                revision: b.revision,
                created_at: b.created_at,
                created_by: b.created_by.clone(),
                summary: format!("Début : {}", first_line(&b.markdown)),
            })
            .collect()
    }

    pub fn history(&self, id: ObjectId) -> Vec<RevisionMeta> {
        let inner = self.inner.borrow();
        let mut metas: Vec<RevisionMeta> = inner
            .data
            .audit
            .iter()
            .filter(|a| a.object_id == Some(id))
            .map(|a| RevisionMeta {
                revision: a.after_revision.unwrap_or(0),
                created_at: a.created_at,
                created_by: a.actor_id.clone(),
                summary: a.operation.clone(),
            })
            .collect();
        metas.sort_by_key(|m| std::cmp::Reverse(m.revision));
        metas.dedup_by_key(|m| m.revision);
        metas
    }

    pub fn revision_snapshot(&self, id: ObjectId, revision: Revision) -> Option<Value> {
        let inner = self.inner.borrow();
        inner
            .data
            .revisions
            .get(id.as_str())
            .and_then(|map| map.get(&revision.to_string()))
            .cloned()
    }

    pub fn revisions(&self, id: ObjectId) -> Vec<(Revision, Value)> {
        let inner = self.inner.borrow();
        inner
            .data
            .revisions
            .get(id.as_str())
            .map(|map| {
                let mut out: Vec<(Revision, Value)> = map
                    .iter()
                    .filter_map(|(k, v)| k.parse::<Revision>().ok().map(|r| (r, v.clone())))
                    .collect();
                out.sort_by_key(|(r, _)| *r);
                out
            })
            .unwrap_or_default()
    }

    /// Compare deux révisions d'un même objet, champ par champ.
    ///
    /// `from == to` produit une liste vide. Une révision ou un objet inconnu
    /// renvoie `DIFF-001` plutôt qu'un diff partiel trompeur.
    pub fn revision_diff(
        &self,
        object_id: ObjectId,
        from: Revision,
        to: Revision,
    ) -> RepoResult<RevisionDiff> {
        let inner = self.inner.borrow();
        if !inner.object_exists(object_id) {
            return Err(RepoError::new(
                "DIFF-001",
                format!("Objet inconnu : aucune révision pour « {object_id} »."),
            ));
        }
        let snapshots = inner.data.revisions.get(object_id.as_str());
        let before = snapshots.and_then(|map| map.get(&from.to_string()));
        let after = snapshots.and_then(|map| map.get(&to.to_string()));
        let (Some(before), Some(after)) = (before, after) else {
            return Err(RepoError::new(
                "DIFF-001",
                format!("Révision absente pour « {object_id} » : {from} → {to}."),
            ));
        };
        Ok(RevisionDiff {
            object_id,
            from,
            to,
            changes: if from == to {
                Vec::new()
            } else {
                diff_values(before, after)
                    .into_iter()
                    // Métadonnées volatiles affichées à part (§5.3).
                    .filter(|change| !is_volatile_revision_path(&change.path))
                    .collect()
            },
        })
    }

    /// Révisions réellement comparables : `history()` restreint aux révisions
    /// dont un snapshot existe.
    pub fn comparable_revisions(&self, id: ObjectId) -> Vec<RevisionMeta> {
        self.history(id)
            .into_iter()
            .filter(|meta| self.revision_snapshot(id, meta.revision).is_some())
            .collect()
    }

    pub fn search(&self, query: &str, types: &[ObjectFilterType]) -> Vec<SearchHit> {
        let inner = self.inner.borrow();
        let terms = tokenize(query);
        if terms.is_empty() {
            return vec![];
        }
        let mut hits: Vec<SearchHit> = Vec::new();
        for (id, (title_tokens, body_tokens)) in &inner.index.tokens {
            let Some(summary) = inner.summarize(*id) else {
                continue;
            };
            if !types.is_empty() && !types.iter().any(|t| t.matches(summary.object_type)) {
                continue;
            }
            let mut score = 0.0f32;
            for term in &terms {
                let mut matched = false;
                for token in title_tokens {
                    if token.starts_with(term.as_str()) {
                        score += 6.0;
                        matched = true;
                    } else if term.len() >= 4 && edit_distance(token, term) <= 1 {
                        score += 3.0;
                        matched = true;
                    }
                }
                for token in body_tokens {
                    if token.starts_with(term.as_str()) {
                        score += 1.5;
                        matched = true;
                    }
                }
                if !matched {
                    score = 0.0;
                    break;
                }
            }
            if score <= 0.0 {
                continue;
            }
            let excerpt = inner
                .index
                .documentation_by_object
                .get(id)
                .map(|doc| archifier_core::markdown::excerpt(doc, 120))
                .unwrap_or_default();
            hits.push(SearchHit {
                object_id: *id,
                object_type: summary.object_type,
                title: summary.name.clone(),
                excerpt,
                domain_name: summary.domain_name.clone(),
                score,
                href: format!("#/view/{}/{}", summary.object_type.route_segment(), id),
            });
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.title.cmp(&b.title))
        });
        hits.truncate(60);
        hits
    }

    pub fn all_objects(&self) -> Vec<ObjectSummary> {
        self.inner.borrow().sorted_summaries(|_| true)
    }

    pub fn palette(&self, profile_id: &str) -> Vec<PaletteGroup> {
        self.metamodel
            .profile(profile_id)
            .map(|p| p.palette())
            .unwrap_or_default()
    }

    pub fn stats(&self) -> Stats {
        let broken_links = self.broken_links().len();
        let inner = self.inner.borrow();
        let data = &inner.data;
        Stats {
            domains: data.domains.len(),
            elements: data.elements.len(),
            relationships: data.relationships.len(),
            interfaces: data.interfaces.len(),
            versions: data.interface_versions.len(),
            diagrams: data.diagrams.len(),
            stories: data.stories.len(),
            reviews: data.reviews.len(),
            documentations: data.documentation.len(),
            links: data.links.len(),
            broken_links,
            persisted_bytes: serde_json::to_string(data).map(|s| s.len()).unwrap_or(0),
        }
    }

    pub fn audit(&self, limit: usize) -> Vec<AuditEvent> {
        let inner = self.inner.borrow();
        let mut events: Vec<AuditEvent> = inner.data.audit.clone();
        events.sort_by_key(|e| std::cmp::Reverse(e.created_at));
        events.truncate(limit);
        events
    }

    pub fn comments(&self, target: ObjectId) -> Vec<Comment> {
        let inner = self.inner.borrow();
        let mut out: Vec<Comment> = inner
            .data
            .comments
            .iter()
            .filter(|c| c.target_object_id == target)
            .cloned()
            .collect();
        out.sort_by_key(|c| c.created_at);
        out
    }

    // -- commandes ----------------------------------------------------------

    /// Crée ou met à jour une identité locale simulée.
    pub fn add_user(&self, id: &str, email: &str, display_name: &str) -> RepoResult<()> {
        self.mutate(|inner| {
            let initials: String = display_name
                .split_whitespace()
                .filter_map(|part| part.chars().next())
                .take(2)
                .collect::<String>()
                .to_uppercase();
            let user = archifier_core::User {
                id: id.to_string(),
                email: email.to_string(),
                display_name: display_name.to_string(),
                initials,
            };
            match inner.data.users.iter_mut().find(|u| u.id == id) {
                Some(existing) => *existing = user,
                None => inner.data.users.push(user),
            }
            Ok(())
        })
    }

    pub fn create_domain(&self, name: &str, description: &str) -> RepoResult<ObjectId> {
        let name = name.trim().to_string();
        validate_domain_name(&name).map_err(RepoError::from_validation)?;
        let slug = archifier_core::validation::slugify(&name);
        if slug.is_empty() {
            return Err(RepoError::new(
                "DOM-002",
                "Le nom ne produit pas d'identifiant lisible.",
            ));
        }
        self.mutate(|inner| {
            if inner.index.domain_by_slug.contains_key(&slug) {
                return Err(RepoError::new(
                    "DOM-003",
                    format!("Un domaine actif utilise déjà l'identifiant « {slug} »."),
                ));
            }
            let id = self.new_id();
            let now = self.now();
            let user = self.current_user();
            let object = RepositoryObject {
                id,
                object_type: ObjectType::Domain,
                name: name.clone(),
                responsible_domain_id: id,
                lifecycle: Lifecycle::Active,
                revision: 1,
                created_at: now,
                created_by: user.clone(),
                updated_at: now,
                updated_by: user.clone(),
            };
            inner.data.domains.push(Domain {
                object,
                slug,
                description: description.to_string(),
            });
            inner.record(id, "Création du domaine", now, &user);
            Ok(id)
        })
    }

    pub fn update_domain(&self, id: ObjectId, name: &str, description: &str) -> RepoResult<()> {
        let name = name.trim().to_string();
        validate_domain_name(&name).map_err(RepoError::from_validation)?;
        self.mutate(|inner| {
            let now = self.now();
            let user = self.current_user();
            let domain = inner
                .data
                .domains
                .iter_mut()
                .find(|d| d.object.id == id)
                .ok_or_else(|| RepoError::not_found("Domaine"))?;
            domain.object.name = name;
            domain.description = description.to_string();
            domain.object.revision += 1;
            domain.object.updated_at = now;
            domain.object.updated_by = user.clone();
            domain.slug = archifier_core::validation::slugify(&domain.object.name);
            inner.record(id, "Modification du domaine", now, &user);
            Ok(())
        })
    }

    pub fn archive(&self, id: ObjectId) -> RepoResult<()> {
        self.mutate(|inner| {
            let now = self.now();
            let user = self.current_user();

            if let Some(domain) = inner.find_domain(id) {
                let blocking = inner
                    .data
                    .elements
                    .iter()
                    .filter(|e| {
                        e.object.responsible_domain_id == id && e.object.lifecycle == Lifecycle::Active
                    })
                    .map(|e| e.object.name.clone())
                    .chain(
                        inner
                            .data
                            .interfaces
                            .iter()
                            .filter(|i| i.provider_domain_id == id && i.object.is_active())
                            .map(|i| i.object.name.clone()),
                    )
                    .take(20)
                    .collect::<Vec<_>>();
                if !blocking.is_empty() {
                    return Err(RepoError::new(
                        "DOM-001",
                        format!(
                            "Impossible d'archiver « {} » : {} objet(s) actif(s) en dépendent, dont {}.",
                            domain.object.name,
                            blocking.len(),
                            blocking.join(", ")
                        ),
                    ));
                }
            }

            if let Some(element) = inner.find_element(id) {
                let active_children: Vec<String> = inner
                    .data
                    .elements
                    .iter()
                    .filter(|child| {
                        child.parent_id == Some(id)
                            && child.object.lifecycle == Lifecycle::Active
                    })
                    .map(|child| child.object.name.clone())
                    .take(20)
                    .collect();
                if !active_children.is_empty() {
                    return Err(RepoError::new(
                        "HIER-005",
                        format!(
                            "Impossible d'archiver « {} » : {} enfant(s) actif(s), dont {}.",
                            element.object.name,
                            active_children.len(),
                            active_children.join(", ")
                        ),
                    ));
                }
            }

            let object = inner
                .object_by_id(id)
                .ok_or_else(|| RepoError::not_found("Objet"))?;
            set_lifecycle(inner, id, Lifecycle::Archived, now, &user);
            inner.record(id, format!("Archivage de « {} »", object.name), now, &user);
            Ok(())
        })
    }

    pub fn restore(&self, id: ObjectId) -> RepoResult<()> {
        self.mutate(|inner| {
            let now = self.now();
            let user = self.current_user();
            set_lifecycle(inner, id, Lifecycle::Active, now, &user);
            inner.record(id, "Restauration", now, &user);
            Ok(())
        })
    }

    pub fn create_element(
        &self,
        domain_id: ObjectId,
        notation: Notation,
        class_key: &str,
        name: &str,
        properties: Map<String, Value>,
    ) -> RepoResult<ObjectId> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(RepoError::new("OBJ-002", "Le nom est obligatoire."));
        }
        let profile_id = archifier_core::default_profile_for(notation).to_string();
        let element_def = self
            .metamodel
            .validate_element(&profile_id, class_key, &properties)
            .map_err(RepoError::from_validation)?
            .clone();

        self.mutate(|inner| {
            if !inner.active_domain(domain_id) {
                return Err(RepoError::new(
                    "OBJ-003",
                    "Domaine responsable introuvable ou archivé.",
                ));
            }
            let id = self.new_id();
            let now = self.now();
            let user = self.current_user();
            let object = RepositoryObject {
                id,
                object_type: ObjectType::ModelElement,
                name: name.clone(),
                responsible_domain_id: domain_id,
                lifecycle: Lifecycle::Active,
                revision: 1,
                created_at: now,
                created_by: user.clone(),
                updated_at: now,
                updated_by: user.clone(),
            };
            validate_object(&object, true).map_err(RepoError::from_validation)?;
            inner.data.elements.push(ModelElement {
                object,
                notation,
                metamodel_version: profile_id.clone(),
                metaclass: element_def.metaclass.clone(),
                parent_id: None,
                semantic_properties: properties,
            });
            inner.record(id, format!("Création de « {name} »"), now, &user);
            Ok(id)
        })
    }

    pub fn update_element(
        &self,
        id: ObjectId,
        name: &str,
        properties: Map<String, Value>,
    ) -> RepoResult<()> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(RepoError::new("OBJ-002", "Le nom est obligatoire."));
        }
        self.mutate(|inner| {
            inner.require_active(id)?;
            let now = self.now();
            let user = self.current_user();
            let profile_id = inner
                .find_element(id)
                .map(|e| e.metamodel_version.clone())
                .ok_or_else(|| RepoError::not_found("Élément"))?;
            let metaclass = inner
                .find_element(id)
                .map(|e| e.metaclass.clone())
                .unwrap_or_default();
            self.metamodel
                .validate_element(&profile_id, &metaclass, &properties)
                .map_err(RepoError::from_validation)?;
            let element = inner
                .data
                .elements
                .iter_mut()
                .find(|e| e.object.id == id)
                .ok_or_else(|| RepoError::not_found("Élément"))?;
            element.object.name = name;
            element.semantic_properties = properties;
            element.object.revision += 1;
            element.object.updated_at = now;
            element.object.updated_by = user.clone();
            inner.record(id, "Modification de l'élément", now, &user);
            Ok(())
        })
    }

    pub fn create_relationship(
        &self,
        domain_id: ObjectId,
        source_id: ObjectId,
        target_id: ObjectId,
        relation_type: &str,
    ) -> RepoResult<ObjectId> {
        if source_id == target_id {
            return Err(RepoError::new(
                "MM-REL-002",
                "Une relation ne peut pas relier un objet à lui-même.",
            ));
        }
        let source = self.inner.borrow().find_element(source_id).cloned();
        let target = self.inner.borrow().find_element(target_id).cloned();
        let (Some(source), Some(target)) = (source, target) else {
            return Err(RepoError::new(
                "MM-REL-003",
                "Les deux extrémités doivent être des éléments du référentiel.",
            ));
        };

        let source_profile = source.metamodel_version.clone();
        let target_profile = target.metamodel_version.clone();
        let resolution = self
            .metamodel
            .resolve_relation_notation(
                Some((source_profile.as_str(), source.metaclass.as_str())),
                relation_type,
                Some((target_profile.as_str(), target.metaclass.as_str())),
            )
            .map_err(RepoError::from_validation)?;

        let (notation, profile_id) = match resolution {
            RelationResolution::SameNotation => (source.notation, source_profile.clone()),
            RelationResolution::Native => (Notation::Native, "native:1.0".to_string()),
        };

        self.mutate(|inner| {
            if !inner.active_domain(domain_id) {
                return Err(RepoError::new(
                    "OBJ-003",
                    "Domaine responsable introuvable ou archivé.",
                ));
            }
            let id = self.new_id();
            let now = self.now();
            let user = self.current_user();
            let name = format!(
                "{} → {}",
                inner
                    .object_by_id(source_id)
                    .map(|o| o.name)
                    .unwrap_or_default(),
                inner
                    .object_by_id(target_id)
                    .map(|o| o.name)
                    .unwrap_or_default()
            );
            let object = RepositoryObject {
                id,
                object_type: ObjectType::Relationship,
                name,
                responsible_domain_id: domain_id,
                lifecycle: Lifecycle::Active,
                revision: 1,
                created_at: now,
                created_by: user.clone(),
                updated_at: now,
                updated_by: user.clone(),
            };
            inner.data.relationships.push(Relationship {
                object,
                source_id,
                target_id,
                relation_type: relation_type.to_string(),
                notation,
                metamodel_version: profile_id,
            });
            inner.record(id, "Création d'une relation", now, &user);
            Ok(id)
        })
    }

    pub fn create_interface(
        &self,
        domain_id: ObjectId,
        name: &str,
        logical_key: &str,
        category: InterfaceCategory,
    ) -> RepoResult<ObjectId> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(RepoError::new("OBJ-002", "Le nom est obligatoire."));
        }
        let key = logical_key.trim().to_lowercase();
        self.mutate(|inner| {
            if !inner.active_domain(domain_id) {
                return Err(RepoError::new(
                    "OBJ-003",
                    "Domaine responsable introuvable ou archivé.",
                ));
            }
            if inner.index.interface_by_key.contains_key(&key) {
                return Err(RepoError::new(
                    "IFACE-DUP-001",
                    format!("La clé logique « {key} » est déjà utilisée."),
                ));
            }
            let id = self.new_id();
            let now = self.now();
            let user = self.current_user();
            let object = RepositoryObject {
                id,
                object_type: ObjectType::Interface,
                name: name.clone(),
                responsible_domain_id: domain_id,
                lifecycle: Lifecycle::Active,
                revision: 1,
                created_at: now,
                created_by: user.clone(),
                updated_at: now,
                updated_by: user.clone(),
            };
            let interface = Interface {
                object,
                logical_interface_key: key,
                category,
                provider_domain_id: domain_id,
                contract_type: archifier_core::ContractType::None,
                notation_bindings: vec![],
            };
            validate_interface(&interface).map_err(RepoError::from_validation)?;
            inner.data.interfaces.push(interface);
            inner.record(
                id,
                format!("Création de l'interface « {name} »"),
                now,
                &user,
            );
            Ok(id)
        })
    }

    pub fn create_interface_version(
        &self,
        interface_id: ObjectId,
        semantic_version: &str,
        available_from: Option<OffsetDateTime>,
        available_until: Option<OffsetDateTime>,
        breaking_change: bool,
        publish: bool,
    ) -> RepoResult<ObjectId> {
        let semver = semantic_version.trim().to_string();
        self.mutate(|inner| {
            inner.require_active(interface_id)?;
            let now = self.now();
            let user = self.current_user();
            let id = self.new_id();
            let version = InterfaceVersion {
                id,
                interface_id,
                semantic_version: semver.clone(),
                revision: 1,
                publication_status: if publish {
                    PublicationStatus::Published
                } else {
                    PublicationStatus::Draft
                },
                created_at: now,
                created_by: user.clone(),
                published_at: if publish { Some(now) } else { None },
                available_from,
                available_until,
                breaking_change,
                supersedes_version_id: None,
                contract_snapshot: json!({}),
                contract_format: None,
                contract_raw: None,
                contract_hash: None,
            };
            validate_interface_version(&version).map_err(RepoError::from_validation)?;
            inner.data.interface_versions.push(version);
            inner.record(id, format!("Version {semver}"), now, &user);
            Ok(id)
        })
    }

    pub fn publish_version(
        &self,
        version_id: ObjectId,
        available_from: OffsetDateTime,
        available_until: OffsetDateTime,
        breaking_change: bool,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let now = self.now();
            let user = self.current_user();
            let version = inner
                .data
                .interface_versions
                .iter_mut()
                .find(|v| v.id == version_id)
                .ok_or_else(|| RepoError::not_found("Version d'interface"))?;
            version.available_from = Some(available_from);
            version.available_until = Some(available_until);
            version.breaking_change = breaking_change;
            version.publication_status = PublicationStatus::Published;
            version.published_at = Some(now);
            version.revision += 1;
            let codes = publication_issues(version);
            if let Some(code) = codes.first() {
                let message = match *code {
                    "BR-001" => {
                        "BR-001 : une version publiée doit définir availableFrom et availableUntil."
                    }
                    "BR-002" => "BR-002 : availableUntil doit être postérieur à availableFrom.",
                    _ => "Fenêtre de disponibilité invalide.",
                };
                return Err(RepoError {
                    code: (*code).to_string(),
                    message: message.to_string(),
                    issues: vec![ValidationIssue {
                        code: (*code).to_string(),
                        severity: Severity::Error,
                        path: "/availableUntil".into(),
                        message: message.into(),
                    }],
                });
            }
            let interface_id = version.interface_id;
            let semver = version.semantic_version.clone();
            inner.record(version_id, format!("Publication {semver}"), now, &user);
            inner.record(interface_id, format!("Publication de {semver}"), now, &user);
            Ok(())
        })
    }

    pub fn add_consumer(
        &self,
        version_id: ObjectId,
        consumer_object_id: ObjectId,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let now = self.now();
            let user = self.current_user();
            let version = inner
                .data
                .interface_versions
                .iter()
                .find(|v| v.id == version_id)
                .ok_or_else(|| RepoError::not_found("Version d'interface"))?;
            if version.publication_status != PublicationStatus::Published {
                return Err(RepoError::new(
                    "IFACE-004",
                    "Seule une version publiée peut être consommée.",
                ));
            }
            let consumer = inner
                .object_by_id(consumer_object_id)
                .ok_or_else(|| RepoError::not_found("Consommateur"))?;
            if inner.data.interface_consumers.iter().any(|c| {
                c.interface_version_id == version_id && c.consumer_object_id == consumer_object_id
            }) {
                return Err(RepoError::new(
                    "IFACE-005",
                    "Cette consommation est déjà déclarée.",
                ));
            }
            let id = self.new_id();
            inner.data.interface_consumers.push(InterfaceConsumer {
                id,
                interface_version_id: version_id,
                consumer_object_id,
                consumer_domain_id: consumer.responsible_domain_id,
                declared_at: now,
                declared_by: user.clone(),
            });
            inner.record(
                consumer_object_id,
                "Déclaration de consommation",
                now,
                &user,
            );
            Ok(())
        })
    }

    pub fn create_diagram(
        &self,
        domain_id: ObjectId,
        name: &str,
        notation: DiagramNotation,
    ) -> RepoResult<ObjectId> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(RepoError::new("OBJ-002", "Le nom est obligatoire."));
        }
        self.mutate(|inner| {
            if !inner.active_domain(domain_id) {
                return Err(RepoError::new(
                    "OBJ-003",
                    "Domaine responsable introuvable ou archivé.",
                ));
            }
            let id = self.new_id();
            let now = self.now();
            let user = self.current_user();
            let object = RepositoryObject {
                id,
                object_type: ObjectType::Diagram,
                name: name.clone(),
                responsible_domain_id: domain_id,
                lifecycle: Lifecycle::Active,
                revision: 1,
                created_at: now,
                created_by: user.clone(),
                updated_at: now,
                updated_by: user.clone(),
            };
            inner.data.diagrams.push(Diagram {
                object,
                notation,
                description: String::new(),
                publication_status: archifier_core::PublicationStatus::Draft,
                published_at: None,
                layout: DiagramLayout::default(),
            });
            inner.record(id, format!("Création de la vue « {name} »"), now, &user);
            Ok(id)
        })
    }

    pub fn save_layout(&self, diagram_id: ObjectId, layout: DiagramLayout) -> RepoResult<()> {
        self.mutate(|inner| {
            inner.require_active(diagram_id)?;
            let now = self.now();
            let user = self.current_user();
            let known: HashSet<ObjectId> = inner.index.object_type.keys().copied().collect();
            let exists = |id: ObjectId| known.contains(&id);
            let diagram = inner
                .data
                .diagrams
                .iter_mut()
                .find(|d| d.object.id == diagram_id)
                .ok_or_else(|| RepoError::not_found("Diagramme"))?;
            validate_diagram(diagram, &exists).map_err(RepoError::from_validation)?;
            diagram.layout = layout;
            diagram.object.revision += 1;
            diagram.object.updated_at = now;
            diagram.object.updated_by = user.clone();
            inner.record(diagram_id, "Enregistrement de la vue", now, &user);
            Ok(())
        })
    }

    /// Écriture optimiste (C-09 §4.1) : refuse si la révision a changé.
    pub fn save_layout_expect(
        &self,
        diagram_id: ObjectId,
        layout: DiagramLayout,
        expected_revision: Revision,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            inner.require_active(diagram_id)?;
            let now = self.now();
            let user = self.current_user();
            let known: HashSet<ObjectId> = inner.index.object_type.keys().copied().collect();
            let exists = |id: ObjectId| known.contains(&id);
            let diagram = inner
                .data
                .diagrams
                .iter_mut()
                .find(|d| d.object.id == diagram_id)
                .ok_or_else(|| RepoError::not_found("Diagramme"))?;
            if diagram.object.revision != expected_revision {
                return Err(RepoError::new(
                    "REV-409",
                    format!(
                        "La vue a été modifiée ailleurs (révision {} au lieu de {}).",
                        diagram.object.revision, expected_revision
                    ),
                ));
            }
            validate_diagram(diagram, &exists).map_err(RepoError::from_validation)?;
            diagram.layout = layout;
            diagram.object.revision += 1;
            diagram.object.updated_at = now;
            diagram.object.updated_by = user.clone();
            inner.record(diagram_id, "Enregistrement de la vue", now, &user);
            Ok(())
        })
    }

    /// Couvertures de la Bibliothèque (C-09 §4.2).
    pub fn view_cards(&self, filter: &ViewFilter) -> Vec<ViewCard> {
        let inner = self.inner.borrow();
        let mut cards: Vec<ViewCard> = inner
            .data
            .diagrams
            .iter()
            .filter(|d| d.object.is_active())
            .filter(|d| {
                filter
                    .domain_id
                    .map(|id| d.object.responsible_domain_id == id)
                    .unwrap_or(true)
            })
            .filter(|d| {
                filter
                    .intent
                    .map(|intent| d.layout.intent == Some(intent))
                    .unwrap_or(true)
            })
            .filter(|d| filter.notation.map(|n| d.notation == n).unwrap_or(true))
            .filter(|d| {
                filter
                    .publication
                    .map(|p| d.publication_status == p)
                    .unwrap_or(true)
            })
            .filter(|d| {
                filter
                    .owner
                    .as_ref()
                    .map(|owner| &d.object.updated_by == owner)
                    .unwrap_or(true)
            })
            .filter(|d| {
                filter
                    .query
                    .as_ref()
                    .map(|q| {
                        let q = q.trim().to_lowercase();
                        q.is_empty() || d.object.name.to_lowercase().contains(&q)
                    })
                    .unwrap_or(true)
            })
            .map(|d| {
                let mut mini_nodes: Vec<MiniNode> = d
                    .layout
                    .nodes
                    .iter()
                    .map(|n| MiniNode {
                        x: n.x,
                        y: n.y,
                        w: n.w,
                        h: n.h,
                        kind: n.kind,
                    })
                    .collect();
                mini_nodes.sort_by(|a, b| {
                    format!("{:?}", a.kind)
                        .cmp(&format!("{:?}", b.kind))
                        .then(a.x.total_cmp(&b.x))
                        .then(a.y.total_cmp(&b.y))
                });
                mini_nodes.truncate(40);
                ViewCard {
                    id: d.object.id,
                    title: d.object.name.clone(),
                    domain_id: d.object.responsible_domain_id,
                    domain_name: inner.domain_name(d.object.responsible_domain_id),
                    intent: d.layout.intent.unwrap_or(ViewIntent::ApplicationLandscape),
                    notation: d.notation,
                    node_count: d.layout.nodes.len(),
                    draft_count: d.layout.drafts.len(),
                    publication_status: d.publication_status,
                    owner: d.object.updated_by.clone(),
                    updated_at: d.object.updated_at,
                    framing: d.layout.framing.clone(),
                    mini_nodes,
                }
            })
            .collect();
        cards.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then(a.id.as_str().cmp(b.id.as_str()))
        });
        cards
    }

    /// Renomme une interface sans toucher à sa clé logique (C-09 §4.3).
    pub fn rename_interface(&self, id: ObjectId, name: &str) -> RepoResult<()> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(RepoError::new("OBJ-002", "Le nom est obligatoire."));
        }
        self.mutate(|inner| {
            inner.require_active(id)?;
            let now = self.now();
            let user = self.current_user();
            let interface = inner
                .data
                .interfaces
                .iter_mut()
                .find(|i| i.object.id == id)
                .ok_or_else(|| RepoError::not_found("Interface"))?;
            interface.object.name = name.clone();
            interface.object.revision += 1;
            interface.object.updated_at = now;
            interface.object.updated_by = user.clone();
            inner.record(id, "Renommage de l'interface", now, &user);
            Ok(())
        })
    }

    /// Publie ou retire une vue (C-09 §4.4).
    pub fn set_diagram_publication(
        &self,
        id: ObjectId,
        status: PublicationStatus,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            inner.require_active(id)?;
            let now = self.now();
            let user = self.current_user();
            let diagram = inner
                .data
                .diagrams
                .iter_mut()
                .find(|d| d.object.id == id)
                .ok_or_else(|| RepoError::not_found("Diagramme"))?;
            diagram.publication_status = status;
            diagram.published_at = match status {
                PublicationStatus::Published => diagram.published_at.or(Some(now)),
                PublicationStatus::Withdrawn | PublicationStatus::Draft => None,
            };
            diagram.object.revision += 1;
            diagram.object.updated_at = now;
            diagram.object.updated_by = user.clone();
            inner.record(id, "Publication de la vue", now, &user);
            Ok(())
        })
    }

    /// Résout l'interface porteuse d'une version (C-09 §4.5).
    pub fn interface_of_version(&self, version: ObjectId) -> Option<ObjectId> {
        self.inner
            .borrow()
            .data
            .interface_versions
            .iter()
            .find(|v| v.id == version)
            .map(|v| v.interface_id)
    }

    /// Change les extrémités d'une relation existante, avec les mêmes règles
    /// de métamodèle que la création.
    pub fn update_relationship_endpoints(
        &self,
        id: ObjectId,
        source_id: ObjectId,
        target_id: ObjectId,
    ) -> RepoResult<()> {
        if source_id == target_id {
            return Err(RepoError::new(
                "MM-REL-002",
                "Une relation ne peut pas relier un objet à lui-même.",
            ));
        }
        let relation_type = {
            let inner = self.inner.borrow();
            let relationship = inner
                .data
                .relationships
                .iter()
                .find(|r| r.object.id == id)
                .ok_or_else(|| RepoError::not_found("Relation"))?;
            relationship.relation_type.clone()
        };
        let source = self.inner.borrow().find_element(source_id).cloned();
        let target = self.inner.borrow().find_element(target_id).cloned();
        let (Some(source), Some(target)) = (source, target) else {
            return Err(RepoError::new(
                "MM-REL-003",
                "Les deux extrémités doivent être des éléments du référentiel.",
            ));
        };
        self.metamodel
            .resolve_relation_notation(
                Some((source.metamodel_version.as_str(), source.metaclass.as_str())),
                &relation_type,
                Some((target.metamodel_version.as_str(), target.metaclass.as_str())),
            )
            .map_err(RepoError::from_validation)?;
        self.mutate(|inner| {
            inner.require_active(id)?;
            let now = self.now();
            let user = self.current_user();
            let relationship = inner
                .data
                .relationships
                .iter_mut()
                .find(|r| r.object.id == id)
                .ok_or_else(|| RepoError::not_found("Relation"))?;
            relationship.source_id = source_id;
            relationship.target_id = target_id;
            relationship.object.revision += 1;
            relationship.object.updated_at = now;
            relationship.object.updated_by = user.clone();
            inner.record(id, "Reconnexion d'une relation", now, &user);
            Ok(())
        })
    }

    pub fn add_node(
        &self,
        diagram_id: ObjectId,
        object_id: ObjectId,
        x: f64,
        y: f64,
    ) -> RepoResult<()> {
        let inner = self.inner.borrow();
        let detail = inner
            .data
            .diagrams
            .iter()
            .find(|d| d.object.id == diagram_id)
            .cloned()
            .ok_or_else(|| RepoError::not_found("Diagramme"))?;
        let object = inner
            .object_by_id(object_id)
            .ok_or_else(|| RepoError::not_found("Objet"))?;
        drop(inner);

        let mut layout = detail.layout;
        if layout.nodes.iter().any(|n| n.object_id == object_id) {
            return Ok(());
        }
        let node_id = format!("n{}", layout.nodes.len() + 1);
        let (w, h) = default_node_size(object.object_type);
        layout.nodes.push(DiagramNode {
            id: node_id,
            object_id,
            kind: match object.object_type {
                ObjectType::Interface => archifier_core::NodeKind::Interface,
                ObjectType::Diagram => archifier_core::NodeKind::DiagramRef,
                _ => archifier_core::NodeKind::Element,
            },
            version_id: None,
            x,
            y,
            w,
            h,
            layer_id: None,
            zone_id: None,
            collapsed: false,
            port: None,
            style_override: Map::new(),
        });
        self.save_layout(diagram_id, layout)
    }

    pub fn set_node_position(
        &self,
        diagram_id: ObjectId,
        node_id: &str,
        x: f64,
        y: f64,
    ) -> RepoResult<()> {
        let mut layout = self
            .inner
            .borrow()
            .data
            .diagrams
            .iter()
            .find(|d| d.object.id == diagram_id)
            .map(|d| d.layout.clone())
            .ok_or_else(|| RepoError::not_found("Diagramme"))?;
        if let Some(node) = layout.nodes.iter_mut().find(|n| n.id == node_id) {
            node.x = x;
            node.y = y;
        }
        self.save_layout(diagram_id, layout)
    }

    pub fn add_edge(&self, diagram_id: ObjectId, relationship_id: ObjectId) -> RepoResult<()> {
        let mut layout = self
            .inner
            .borrow()
            .data
            .diagrams
            .iter()
            .find(|d| d.object.id == diagram_id)
            .map(|d| d.layout.clone())
            .ok_or_else(|| RepoError::not_found("Diagramme"))?;
        if layout
            .edges
            .iter()
            .any(|e| e.relationship_id == relationship_id)
        {
            return Ok(());
        }
        layout.edges.push(DiagramEdge {
            id: format!("e{}", layout.edges.len() + 1),
            relationship_id,
            waypoints: vec![],
            label_visible: true,
            routing: Default::default(),
            layer_id: None,
        });
        self.save_layout(diagram_id, layout)
    }

    pub fn remove_node(&self, diagram_id: ObjectId, node_id: &str) -> RepoResult<()> {
        let mut layout = self
            .inner
            .borrow()
            .data
            .diagrams
            .iter()
            .find(|d| d.object.id == diagram_id)
            .map(|d| d.layout.clone())
            .ok_or_else(|| RepoError::not_found("Diagramme"))?;
        layout.nodes.retain(|n| n.id != node_id);
        self.save_layout(diagram_id, layout)
    }

    pub fn set_documentation(&self, object_id: ObjectId, markdown: &str) -> RepoResult<()> {
        self.mutate(|inner| {
            if !inner.object_exists(object_id) {
                return Err(RepoError::not_found("Objet"));
            }
            let now = self.now();
            let user = self.current_user();
            let next_revision = inner
                .data
                .documentation
                .iter()
                .filter(|b| b.owner_object_id == object_id)
                .map(|b| b.revision)
                .max()
                .unwrap_or(0)
                + 1;
            let id = self.new_id();
            inner.data.documentation.push(DocumentationBlock {
                id,
                owner_object_id: object_id,
                markdown: markdown.to_string(),
                revision: next_revision,
                created_at: now,
                created_by: user.clone(),
            });
            inner.refresh_links(object_id, markdown, now);
            inner.record(object_id, "Documentation mise à jour", now, &user);
            Ok(())
        })
    }

    pub fn create_story(&self, domain_id: ObjectId, title: &str) -> RepoResult<ObjectId> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(RepoError::new("OBJ-002", "Le titre est obligatoire."));
        }
        self.mutate(|inner| {
            if !inner.active_domain(domain_id) {
                return Err(RepoError::new(
                    "OBJ-003",
                    "Domaine responsable introuvable ou archivé.",
                ));
            }
            let id = self.new_id();
            let now = self.now();
            let user = self.current_user();
            let object = RepositoryObject {
                id,
                object_type: ObjectType::Story,
                name: title.clone(),
                responsible_domain_id: domain_id,
                lifecycle: Lifecycle::Active,
                revision: 1,
                created_at: now,
                created_by: user.clone(),
                updated_at: now,
                updated_by: user.clone(),
            };
            inner.data.stories.push(Story {
                object,
                structure: Default::default(),
            });
            inner.record(id, format!("Création de la story « {title} »"), now, &user);
            Ok(id)
        })
    }

    pub fn save_story(
        &self,
        id: ObjectId,
        structure: archifier_core::StoryStructure,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            inner.require_active(id)?;
            let now = self.now();
            let user = self.current_user();
            // Validation sur une copie : en cas d'erreur, l'état mémoire reste
            // strictement inchangé (§7.3).
            let mut candidate = inner
                .data
                .stories
                .iter()
                .find(|story| story.object.id == id)
                .cloned()
                .ok_or_else(|| RepoError::not_found("Story"))?;
            candidate.structure = structure;
            candidate.object.revision += 1;
            candidate.object.updated_at = now;
            candidate.object.updated_by = user.clone();
            validate_story(&candidate).map_err(RepoError::from_validation)?;

            let story = inner
                .data
                .stories
                .iter_mut()
                .find(|story| story.object.id == id)
                .ok_or_else(|| RepoError::not_found("Story"))?;
            *story = candidate;
            inner.record(id, "Mise à jour de la story", now, &user);
            Ok(())
        })
    }

    /// Épingle les révisions courantes des objets référencés par la Story.
    ///
    /// Les diagrammes, éléments et interfaces cités par les blocs sont figés à
    /// leur révision courante ; les versions d'interface le sont aussi
    /// (`version_id`, sinon dernière version publiée). Les références absentes
    /// du référentiel sont ignorées. Chaque révision épinglée doit posséder un
    /// snapshot, sinon `STORY-003` et aucune écriture. Retourne le nombre
    /// d'épingles de révision effectivement posées.
    pub fn pin_story_revisions(&self, story_id: ObjectId) -> RepoResult<usize> {
        let (structure, count) = {
            let inner = self.inner.borrow();
            let story = inner
                .data
                .stories
                .iter()
                .find(|story| story.object.id == story_id)
                .ok_or_else(|| RepoError::not_found("Story"))?;

            let mut object_ids = BTreeSet::new();
            let mut explicit_versions = BTreeMap::new();
            let mut latest_needed = BTreeSet::new();
            for chapter in &story.structure.chapters {
                for block in &chapter.blocks {
                    match block {
                        archifier_core::StoryBlock::Diagram { diagram_id, .. } => {
                            object_ids.insert(*diagram_id);
                        }
                        archifier_core::StoryBlock::Element { element_id, .. } => {
                            object_ids.insert(*element_id);
                        }
                        archifier_core::StoryBlock::Interface {
                            interface_id,
                            version_id,
                            ..
                        } => {
                            object_ids.insert(*interface_id);
                            match version_id {
                                Some(version_id) => {
                                    explicit_versions.insert(*interface_id, *version_id);
                                }
                                None => {
                                    latest_needed.insert(*interface_id);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            let mut pinned_revisions = BTreeMap::new();
            for object_id in object_ids {
                let Some(object) = inner.object_by_id(object_id) else {
                    continue;
                };
                let revision = object.revision;
                let snapshot_exists = inner
                    .data
                    .revisions
                    .get(object_id.as_str())
                    .and_then(|snapshots| snapshots.get(&revision.to_string()))
                    .is_some();
                if !snapshot_exists {
                    return Err(RepoError::new(
                        "STORY-003",
                        format!(
                            "Révision {revision} absente pour « {} » : épinglage impossible.",
                            object.name
                        ),
                    ));
                }
                pinned_revisions.insert(object_id, revision);
            }

            let mut pinned_versions = BTreeMap::new();
            for (interface_id, version_id) in explicit_versions {
                let belongs = inner.data.interface_versions.iter().any(|version| {
                    version.id == version_id && version.interface_id == interface_id
                });
                if belongs {
                    pinned_versions.insert(interface_id, version_id);
                }
            }
            for interface_id in latest_needed {
                if pinned_versions.contains_key(&interface_id) {
                    continue;
                }
                if let Some(version_id) = latest_published_version(&inner, interface_id) {
                    pinned_versions.insert(interface_id, version_id);
                }
            }

            let count = pinned_revisions.len();
            let mut structure = story.structure.clone();
            structure.pinned_revisions = pinned_revisions;
            structure.pinned_versions = pinned_versions;
            (structure, count)
        };
        self.save_story(story_id, structure)?;
        Ok(count)
    }

    /// Dégèle la Story en vidant révisions et versions épinglées.
    ///
    /// Sans épinglage, l'appel est un no-op qui n'écrit ni révision ni audit.
    pub fn unpin_story_revisions(&self, story_id: ObjectId) -> RepoResult<()> {
        let structure = {
            let inner = self.inner.borrow();
            inner
                .data
                .stories
                .iter()
                .find(|story| story.object.id == story_id)
                .map(|story| story.structure.clone())
                .ok_or_else(|| RepoError::not_found("Story"))?
        };
        if structure.pinned_revisions.is_empty() && structure.pinned_versions.is_empty() {
            return Ok(());
        }
        let mut structure = structure;
        structure.pinned_revisions.clear();
        structure.pinned_versions.clear();
        self.save_story(story_id, structure)
    }

    pub fn create_review(&self, domain_id: ObjectId, title: &str) -> RepoResult<ObjectId> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(RepoError::new("OBJ-002", "Le titre est obligatoire."));
        }
        self.mutate(|inner| {
            if !inner.active_domain(domain_id) {
                return Err(RepoError::new(
                    "OBJ-003",
                    "Domaine responsable introuvable ou archivé.",
                ));
            }
            let id = self.new_id();
            let now = self.now();
            let user = self.current_user();
            let object = RepositoryObject {
                id,
                object_type: ObjectType::Review,
                name: title.clone(),
                responsible_domain_id: domain_id,
                lifecycle: Lifecycle::Active,
                revision: 1,
                created_at: now,
                created_by: user.clone(),
                updated_at: now,
                updated_by: user.clone(),
            };
            inner.data.reviews.push(Review {
                object,
                domain_id,
                filter_snapshot: json!({}),
                temporal_window_start: None,
                temporal_window_end: None,
                status: ReviewStatus::Open,
                markdown_summary: String::new(),
            });
            inner.record(id, format!("Ouverture de la revue « {title} »"), now, &user);
            Ok(id)
        })
    }

    pub fn set_review_status(&self, id: ObjectId, status: ReviewStatus) -> RepoResult<()> {
        self.mutate(|inner| {
            let now = self.now();
            let user = self.current_user();
            let review = inner
                .data
                .reviews
                .iter_mut()
                .find(|r| r.object.id == id)
                .ok_or_else(|| RepoError::not_found("Revue"))?;
            review.status = status;
            review.object.revision += 1;
            review.object.updated_at = now;
            review.object.updated_by = user.clone();
            let label = review_status_label(status);
            inner.record(id, format!("Revue {label}"), now, &user);
            Ok(())
        })
    }

    pub fn add_comment(&self, target: ObjectId, markdown: &str) -> RepoResult<()> {
        if markdown.trim().is_empty() {
            return Err(RepoError::new("COMMENT-001", "Le commentaire est vide."));
        }
        self.mutate(|inner| {
            if !inner.object_exists(target) {
                return Err(RepoError::not_found("Objet"));
            }
            let now = self.now();
            let user = self.current_user();
            let id = self.new_id();
            inner.data.comments.push(Comment {
                id,
                target_object_id: target,
                author_id: user.clone(),
                markdown: markdown.to_string(),
                created_at: now,
            });
            inner.record(target, "Commentaire ajouté", now, &user);
            Ok(())
        })
    }

    /// Applique un ChangeSet validé et confirmé (FR-025).
    pub fn apply_changeset(
        &self,
        changeset: &ChangeSet,
        confirmed: bool,
    ) -> RepoResult<Vec<ObjectId>> {
        if !confirmed {
            return Err(RepoError::new(
                "CS-CONFIRM",
                "Un ChangeSet ne peut pas être appliqué sans confirmation explicite.",
            ));
        }
        let exists = {
            let inner = self.inner.borrow();
            inner
                .index
                .object_type
                .keys()
                .copied()
                .collect::<HashSet<_>>()
        };
        validate_changeset(changeset, &|id| exists.contains(&id))
            .map_err(RepoError::from_validation)?;

        let mut created = HashMap::new();
        let mut touched = Vec::new();
        let mut pending: Vec<&ChangeOperation> = changeset.operations.iter().collect();
        pending.sort_by_key(|op| match op {
            ChangeOperation::CreateElement { .. }
            | ChangeOperation::CreateInterface { .. }
            | ChangeOperation::CreateDiagram { .. }
            | ChangeOperation::CreateStory { .. } => 0,
            ChangeOperation::CreateRelationship { .. }
            | ChangeOperation::CreateInterfaceVersion { .. } => 1,
            _ => 2,
        });

        self.mutate(|inner| {
            let now = self.now();
            let user = self.current_user();
            for operation in pending {
                match operation {
                    ChangeOperation::CreateElement {
                        temp_id,
                        domain_id,
                        notation,
                        metaclass,
                        name,
                        metamodel_version,
                        properties,
                    } => {
                        if !inner.active_domain(*domain_id) {
                            return Err(RepoError::new("OBJ-003", "Domaine invalide."));
                        }
                        let profile_id = metamodel_version.clone().unwrap_or_else(|| {
                            archifier_core::default_profile_for(*notation).to_string()
                        });
                        let def = self
                            .metamodel
                            .validate_element(&profile_id, metaclass, properties)
                            .map_err(RepoError::from_validation)?
                            .clone();
                        let id = self.new_id();
                        inner.data.elements.push(ModelElement {
                            object: RepositoryObject {
                                id,
                                object_type: ObjectType::ModelElement,
                                name: name.clone(),
                                responsible_domain_id: *domain_id,
                                lifecycle: Lifecycle::Active,
                                revision: 1,
                                created_at: now,
                                created_by: user.clone(),
                                updated_at: now,
                                updated_by: user.clone(),
                            },
                            notation: *notation,
                            metamodel_version: profile_id,
                            metaclass: def.metaclass,
                            parent_id: None,
                            semantic_properties: properties.clone(),
                        });
                        inner.record(
                            id,
                            format!("Création de « {name} » (proposition)"),
                            now,
                            &user,
                        );
                        created.insert(temp_id.clone(), id);
                        touched.push(id);
                    }
                    ChangeOperation::CreateRelationship {
                        source_id,
                        target_id,
                        relation_type,
                        domain_id,
                        ..
                    } => {
                        let source = resolve_ref(source_id, &created)?;
                        let target = resolve_ref(target_id, &created)?;
                        let source_element = inner
                            .find_element(source)
                            .cloned()
                            .ok_or_else(|| RepoError::new("CS-001", "Source introuvable."))?;
                        let target_element = inner
                            .find_element(target)
                            .cloned()
                            .ok_or_else(|| RepoError::new("CS-001", "Cible introuvable."))?;
                        let resolution = self
                            .metamodel
                            .resolve_relation_notation(
                                Some((
                                    source_element.metamodel_version.as_str(),
                                    source_element.metaclass.as_str(),
                                )),
                                relation_type,
                                Some((
                                    target_element.metamodel_version.as_str(),
                                    target_element.metaclass.as_str(),
                                )),
                            )
                            .map_err(RepoError::from_validation)?;
                        let (notation, profile) = match resolution {
                            RelationResolution::SameNotation => (
                                source_element.notation,
                                source_element.metamodel_version.clone(),
                            ),
                            RelationResolution::Native => {
                                (Notation::Native, "native:1.0".to_string())
                            }
                        };
                        let id = self.new_id();
                        inner.data.relationships.push(Relationship {
                            object: RepositoryObject {
                                id,
                                object_type: ObjectType::Relationship,
                                name: format!(
                                    "{} → {}",
                                    source_element.object.name, target_element.object.name
                                ),
                                responsible_domain_id: *domain_id,
                                lifecycle: Lifecycle::Active,
                                revision: 1,
                                created_at: now,
                                created_by: user.clone(),
                                updated_at: now,
                                updated_by: user.clone(),
                            },
                            source_id: source,
                            target_id: target,
                            relation_type: relation_type.clone(),
                            notation,
                            metamodel_version: profile,
                        });
                        inner.record(id, "Création d'une relation (proposition)", now, &user);
                        touched.push(id);
                    }
                    ChangeOperation::CreateDiagram {
                        temp_id,
                        domain_id,
                        name,
                        notation,
                    } => {
                        if !inner.active_domain(*domain_id) {
                            return Err(RepoError::new("OBJ-003", "Domaine invalide."));
                        }
                        let id = self.new_id();
                        inner.data.diagrams.push(Diagram {
                            object: RepositoryObject {
                                id,
                                object_type: ObjectType::Diagram,
                                name: name.clone(),
                                responsible_domain_id: *domain_id,
                                lifecycle: Lifecycle::Active,
                                revision: 1,
                                created_at: now,
                                created_by: user.clone(),
                                updated_at: now,
                                updated_by: user.clone(),
                            },
                            notation: *notation,
                            description: String::new(),
                            publication_status: archifier_core::PublicationStatus::Draft,
                            published_at: None,
                            layout: DiagramLayout::default(),
                        });
                        inner.record(
                            id,
                            format!("Création de la vue « {name} » (proposition)"),
                            now,
                            &user,
                        );
                        created.insert(temp_id.clone(), id);
                        touched.push(id);
                    }
                    ChangeOperation::CreateStory {
                        temp_id,
                        domain_id,
                        name,
                    } => {
                        if !inner.active_domain(*domain_id) {
                            return Err(RepoError::new("OBJ-003", "Domaine invalide."));
                        }
                        let id = self.new_id();
                        inner.data.stories.push(Story {
                            object: RepositoryObject {
                                id,
                                object_type: ObjectType::Story,
                                name: name.clone(),
                                responsible_domain_id: *domain_id,
                                lifecycle: Lifecycle::Active,
                                revision: 1,
                                created_at: now,
                                created_by: user.clone(),
                                updated_at: now,
                                updated_by: user.clone(),
                            },
                            structure: Default::default(),
                        });
                        inner.record(
                            id,
                            format!("Création de la story « {name} » (proposition)"),
                            now,
                            &user,
                        );
                        created.insert(temp_id.clone(), id);
                        touched.push(id);
                    }
                    ChangeOperation::CreateInterface {
                        temp_id,
                        domain_id,
                        name,
                        logical_interface_key,
                        category,
                    } => {
                        let key = logical_interface_key.to_lowercase();
                        if inner.index.interface_by_key.contains_key(&key) {
                            return Err(RepoError::new(
                                "IFACE-DUP-001",
                                format!("La clé logique « {key} » est déjà utilisée."),
                            ));
                        }
                        let id = self.new_id();
                        inner.data.interfaces.push(Interface {
                            object: RepositoryObject {
                                id,
                                object_type: ObjectType::Interface,
                                name: name.clone(),
                                responsible_domain_id: *domain_id,
                                lifecycle: Lifecycle::Active,
                                revision: 1,
                                created_at: now,
                                created_by: user.clone(),
                                updated_at: now,
                                updated_by: user.clone(),
                            },
                            logical_interface_key: key,
                            category: *category,
                            provider_domain_id: *domain_id,
                            contract_type: archifier_core::ContractType::None,
                            notation_bindings: vec![],
                        });
                        inner.record(
                            id,
                            format!("Création de l'interface « {name} » (proposition)"),
                            now,
                            &user,
                        );
                        created.insert(temp_id.clone(), id);
                        touched.push(id);
                    }
                    ChangeOperation::CreateInterfaceVersion {
                        temp_id,
                        interface_id,
                        semantic_version,
                        available_from,
                        available_until,
                        ..
                    } => {
                        let id = self.new_id();
                        let version = InterfaceVersion {
                            id,
                            interface_id: *interface_id,
                            semantic_version: semantic_version.clone(),
                            revision: 1,
                            publication_status: PublicationStatus::Published,
                            created_at: now,
                            created_by: user.clone(),
                            published_at: Some(now),
                            available_from: Some(*available_from),
                            available_until: Some(*available_until),
                            breaking_change: false,
                            supersedes_version_id: None,
                            contract_snapshot: json!({}),
                            contract_format: None,
                            contract_raw: None,
                            contract_hash: None,
                        };
                        validate_interface_version(&version).map_err(RepoError::from_validation)?;
                        inner.data.interface_versions.push(version);
                        inner.record(
                            id,
                            format!("Version {semantic_version} (proposition)"),
                            now,
                            &user,
                        );
                        created.insert(temp_id.clone(), id);
                        touched.push(id);
                    }
                    ChangeOperation::UpdateDocumentation {
                        object_id,
                        markdown,
                    } => {
                        if !inner.object_exists(*object_id) {
                            return Err(RepoError::not_found("Objet"));
                        }
                        let next = inner
                            .data
                            .documentation
                            .iter()
                            .filter(|b| b.owner_object_id == *object_id)
                            .map(|b| b.revision)
                            .max()
                            .unwrap_or(0)
                            + 1;
                        let id = self.new_id();
                        inner.data.documentation.push(DocumentationBlock {
                            id,
                            owner_object_id: *object_id,
                            markdown: markdown.clone(),
                            revision: next,
                            created_at: now,
                            created_by: user.clone(),
                        });
                        inner.refresh_links(*object_id, markdown, now);
                        inner.record(*object_id, "Documentation (proposition)", now, &user);
                        touched.push(*object_id);
                    }
                    ChangeOperation::UpdateObject { object_id, .. } => {
                        if !inner.object_exists(*object_id) {
                            return Err(RepoError::not_found("Objet"));
                        }
                        inner.record(*object_id, "Mise à jour (proposition)", now, &user);
                        touched.push(*object_id);
                    }
                    ChangeOperation::UpdateDiagramLayout { diagram_id, patch } => {
                        if let Some(diagram) = inner
                            .data
                            .diagrams
                            .iter_mut()
                            .find(|d| d.object.id == *diagram_id)
                        {
                            if let Ok(layout) =
                                serde_json::from_value::<DiagramLayout>(patch.clone())
                            {
                                diagram.layout = layout;
                                diagram.object.revision += 1;
                            }
                        }
                        inner.record(*diagram_id, "Vue mise à jour (proposition)", now, &user);
                        touched.push(*diagram_id);
                    }
                    ChangeOperation::ArchiveObject { object_id } => {
                        set_lifecycle(inner, *object_id, Lifecycle::Archived, now, &user);
                        inner.record(*object_id, "Archivage (proposition)", now, &user);
                        touched.push(*object_id);
                    }
                }
            }
            Ok(touched.clone())
        })
    }

    /// Applique un ChangeSet en le construisant depuis une liste d'opérations,
    /// pratique pour l'IA simulée et les tests.
    pub fn apply_operations(
        &self,
        reason: &str,
        origin: ChangeOrigin,
        operations: Vec<ChangeOperation>,
        confirmed: bool,
    ) -> RepoResult<Vec<ObjectId>> {
        let changeset = ChangeSet {
            id: self.new_id(),
            origin,
            reason: reason.to_string(),
            operations,
            base_revisions: BTreeMap::new(),
            status: ChangeSetStatus::Validated,
        };
        self.apply_changeset(&changeset, confirmed)
    }

    pub fn make_changeset(
        &self,
        reason: &str,
        origin: ChangeOrigin,
        operations: Vec<ChangeOperation>,
    ) -> ChangeSet {
        ChangeSet {
            id: self.new_id(),
            origin,
            reason: reason.to_string(),
            operations,
            base_revisions: BTreeMap::new(),
            status: ChangeSetStatus::Draft,
        }
    }

    pub fn preview_changeset(&self, changeset: &ChangeSet) -> Vec<String> {
        let mut lines = Vec::new();
        for (index, op) in changeset.operations.iter().enumerate() {
            lines.push(format!("{}. {}", index + 1, op.label()));
        }
        lines
    }

    pub fn object_ref(&self, id: ObjectId) -> Option<ObjectRef> {
        let inner = self.inner.borrow();
        let object = inner.object_by_id(id)?;
        Some(ObjectRef::of(object.object_type, id, object.name))
    }

    pub fn template_choices(&self, notation: Notation) -> Vec<(String, String)> {
        self.metamodel
            .profile_for(notation)
            .map(|p| {
                p.elements
                    .iter()
                    .filter(|e| !e.is_abstract)
                    .map(|e| (e.id.clone(), e.label.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Inner {
    fn refresh_links(&mut self, object_id: ObjectId, markdown: &str, now: OffsetDateTime) {
        let uris = archifier_core::extract_uris(markdown);
        self.data.links.retain(|l| l.source_object_id != object_id);
        for uri in uris {
            let status = if self.object_exists(uri.id) {
                if self.is_active(uri.id) {
                    archifier_core::LinkStatus::Ok
                } else {
                    archifier_core::LinkStatus::Archived
                }
            } else {
                archifier_core::LinkStatus::Broken
            };
            let link_type = match uri.kind {
                archifier_core::ArchKind::Interface => archifier_core::LinkType::Consumes,
                archifier_core::ArchKind::Element => archifier_core::LinkType::Reference,
                _ => archifier_core::LinkType::Reference,
            };
            self.data.links.push(archifier_core::Link {
                id: ObjectId::from_parts(
                    now.unix_timestamp_nanos() as u64 / 1_000_000,
                    uri.id.timestamp_ms() as u128,
                ),
                source_object_id: object_id,
                target_uri: uri.to_string(),
                target_object_id: if self.object_exists(uri.id) {
                    Some(uri.id)
                } else {
                    None
                },
                link_type,
                status,
            });
        }
    }
}

fn set_lifecycle(
    inner: &mut Inner,
    id: ObjectId,
    lifecycle: Lifecycle,
    now: OffsetDateTime,
    user: &str,
) {
    if let Some(domain) = inner.data.domains.iter_mut().find(|d| d.object.id == id) {
        domain.object.lifecycle = lifecycle;
        domain.object.revision += 1;
        domain.object.updated_at = now;
        domain.object.updated_by = user.to_string();
    }
    if let Some(element) = inner.data.elements.iter_mut().find(|e| e.object.id == id) {
        element.object.lifecycle = lifecycle;
        element.object.revision += 1;
        element.object.updated_at = now;
        element.object.updated_by = user.to_string();
    }
    if let Some(rel) = inner
        .data
        .relationships
        .iter_mut()
        .find(|r| r.object.id == id)
    {
        rel.object.lifecycle = lifecycle;
        rel.object.revision += 1;
        rel.object.updated_at = now;
        rel.object.updated_by = user.to_string();
    }
    if let Some(interface) = inner.data.interfaces.iter_mut().find(|i| i.object.id == id) {
        interface.object.lifecycle = lifecycle;
        interface.object.revision += 1;
        interface.object.updated_at = now;
        interface.object.updated_by = user.to_string();
    }
    if let Some(diagram) = inner.data.diagrams.iter_mut().find(|d| d.object.id == id) {
        diagram.object.lifecycle = lifecycle;
        diagram.object.revision += 1;
        diagram.object.updated_at = now;
        diagram.object.updated_by = user.to_string();
    }
    if let Some(story) = inner.data.stories.iter_mut().find(|s| s.object.id == id) {
        story.object.lifecycle = lifecycle;
        story.object.revision += 1;
        story.object.updated_at = now;
        story.object.updated_by = user.to_string();
    }
    if let Some(review) = inner.data.reviews.iter_mut().find(|r| r.object.id == id) {
        review.object.lifecycle = lifecycle;
        review.object.revision += 1;
        review.object.updated_at = now;
        review.object.updated_by = user.to_string();
    }
}

fn resolve_ref(reference: &CsRef, created: &HashMap<String, ObjectId>) -> RepoResult<ObjectId> {
    if let Some(id) = reference.as_id() {
        return Ok(id);
    }
    created.get(&reference.0).copied().ok_or_else(|| {
        RepoError::new(
            "CS-002",
            format!("Référence « {} » non résolue.", reference.0),
        )
    })
}

fn default_node_size(object_type: ObjectType) -> (f64, f64) {
    match object_type {
        ObjectType::Interface => (132.0, 64.0),
        ObjectType::Diagram => (150.0, 90.0),
        _ => (168.0, 72.0),
    }
}

fn neighbours(
    inner: &Inner,
    id: ObjectId,
    direction: Direction,
) -> Vec<(ObjectId, String, Direction)> {
    let mut out = Vec::new();
    for relationship in &inner.data.relationships {
        if relationship.object.lifecycle != Lifecycle::Active {
            continue;
        }
        let (other, edge_direction) = if relationship.source_id == id {
            (relationship.target_id, Direction::Out)
        } else if relationship.target_id == id {
            (relationship.source_id, Direction::In)
        } else {
            continue;
        };
        if edge_direction != direction && direction != Direction::Both {
            continue;
        }
        out.push((other, relationship.relation_type.clone(), edge_direction));
    }
    for consumer in &inner.data.interface_consumers {
        if let Some(version) = inner
            .data
            .interface_versions
            .iter()
            .find(|v| v.id == consumer.interface_version_id)
        {
            let edge = if version.interface_id == id {
                // Le consommateur dépend de l'interface : arête entrante.
                Some((consumer.consumer_object_id, Direction::In))
            } else if consumer.consumer_object_id == id {
                // L'objet consomme l'interface : arête sortante.
                Some((version.interface_id, Direction::Out))
            } else {
                None
            };
            if let Some((other, edge_direction)) = edge {
                if direction == Direction::Both || direction == edge_direction {
                    out.push((other, "consumption".into(), edge_direction));
                }
            }
        }
    }
    out
}

fn usages_of(inner: &Inner, id: ObjectId) -> Usages {
    let mut diagrams = Vec::new();
    for diagram in &inner.data.diagrams {
        if diagram.layout.nodes.iter().any(|n| n.object_id == id) {
            diagrams.push(ObjectRef::of(
                ObjectType::Diagram,
                diagram.object.id,
                diagram.object.name.clone(),
            ));
        }
    }
    let mut stories = Vec::new();
    for story in &inner.data.stories {
        let mentions = story.structure.chapters.iter().any(|c| {
            c.blocks.iter().any(|b| match b {
                archifier_core::StoryBlock::Element { element_id, .. } => *element_id == id,
                archifier_core::StoryBlock::Interface { interface_id, .. } => *interface_id == id,
                archifier_core::StoryBlock::Diagram { diagram_id, .. } => *diagram_id == id,
                _ => false,
            })
        });
        if mentions {
            stories.push(ObjectRef::of(
                ObjectType::Story,
                story.object.id,
                story.object.name.clone(),
            ));
        }
    }
    let mut interfaces = Vec::new();
    for interface in &inner.data.interfaces {
        if interface
            .notation_bindings
            .iter()
            .any(|b| b.element_id == Some(id))
        {
            interfaces.push(ObjectRef::of(
                ObjectType::Interface,
                interface.object.id,
                interface.object.name.clone(),
            ));
        }
    }
    let mut reviews = Vec::new();
    for review in &inner.data.reviews {
        if review.object.responsible_domain_id
            == inner
                .object_by_id(id)
                .map(|o| o.responsible_domain_id)
                .unwrap_or(review.domain_id)
        {
            reviews.push(ObjectRef::of(
                ObjectType::Review,
                review.object.id,
                review.object.name.clone(),
            ));
        }
    }
    Usages {
        object_id: id,
        diagrams,
        stories,
        interfaces,
        reviews,
        inbound: dependency_nodes(inner, id, Direction::In),
        outbound: dependency_nodes(inner, id, Direction::Out),
    }
}

fn dependency_nodes(inner: &Inner, id: ObjectId, direction: Direction) -> Vec<DependencyNode> {
    let mut nodes: Vec<DependencyNode> = neighbours(inner, id, direction)
        .into_iter()
        .filter_map(|(other, relation, edge_direction)| {
            let object = inner.object_by_id(other)?;
            Some(DependencyNode {
                object_id: other,
                name: object.name.clone(),
                object_type: object.object_type,
                domain_id: object.responsible_domain_id,
                domain_name: inner.domain_name(object.responsible_domain_id),
                depth: 1,
                path: vec![id, other],
                relation,
                direction: edge_direction,
                lifecycle: object.lifecycle,
            })
        })
        .collect();
    nodes.sort_by(|a, b| (a.depth, a.path.as_slice()).cmp(&(b.depth, b.path.as_slice())));
    nodes
}

fn interface_row(inner: &Inner, interface: &Interface, now: OffsetDateTime) -> InterfaceRow {
    let mut versions: Vec<&InterfaceVersion> = inner
        .data
        .interface_versions
        .iter()
        .filter(|v| v.interface_id == interface.object.id)
        .collect();
    versions.sort_by(|a, b| compare_semver(&a.semantic_version, &b.semantic_version));

    let active_versions = versions
        .iter()
        .filter(|v| version_temporal_status(v, now) == Some(TemporalStatus::Active))
        .count();
    let breaking_versions = versions.iter().filter(|v| v.breaking_change).count();
    let consumers: usize = versions
        .iter()
        .map(|v| {
            inner
                .index
                .consumers_by_version
                .get(&v.id)
                .map(|s| s.len())
                .unwrap_or(0)
        })
        .sum();
    let next_change = versions
        .iter()
        .filter_map(|v| v.available_from)
        .filter(|d| *d > now)
        .min();
    let status = versions
        .iter()
        .filter_map(|v| version_temporal_status(v, now))
        .max_by_key(|s| match s {
            TemporalStatus::Active => 2,
            TemporalStatus::Scheduled => 1,
            TemporalStatus::Expired => 0,
        });

    InterfaceRow {
        id: interface.object.id,
        name: interface.object.name.clone(),
        logical_key: interface.logical_interface_key.clone(),
        category: interface.category,
        provider_domain_id: interface.provider_domain_id,
        provider_domain_name: inner.domain_name(interface.provider_domain_id),
        lifecycle: interface.object.lifecycle,
        status,
        active_versions,
        total_versions: versions.len(),
        breaking_versions,
        consumers,
        next_change,
        href: format!("#/view/interfaces/{}", interface.object.id),
    }
}

fn compare_semver(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> {
        s.split('-')
            .next()
            .unwrap_or(s)
            .split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (va, vb) = (parse(a), parse(b));
    va.cmp(&vb)
}

fn domain_links(inner: &Inner, domain_id: ObjectId, direction: Direction) -> Vec<DomainLink> {
    let mut weights: BTreeMap<ObjectId, (usize, Vec<String>)> = BTreeMap::new();
    for relationship in &inner.data.relationships {
        let source = inner.object_by_id(relationship.source_id);
        let target = inner.object_by_id(relationship.target_id);
        let (Some(source), Some(target)) = (source, target) else {
            continue;
        };
        let (from, to) = match direction {
            Direction::Out => (source, target),
            Direction::In => (target, source),
            Direction::Both => (source, target),
        };
        if from.responsible_domain_id != domain_id || to.responsible_domain_id == domain_id {
            continue;
        }
        let entry = weights
            .entry(to.responsible_domain_id)
            .or_insert((0, vec![]));
        entry.0 += 1;
        if entry.1.len() < 5 {
            entry.1.push(to.name);
        }
    }
    for consumer in &inner.data.interface_consumers {
        if consumer.consumer_domain_id != domain_id {
            continue;
        }
        if let Some(version) = inner
            .data
            .interface_versions
            .iter()
            .find(|v| v.id == consumer.interface_version_id)
        {
            if let Some(interface) = inner.find_interface(version.interface_id) {
                if interface.provider_domain_id != domain_id {
                    let entry = weights
                        .entry(interface.provider_domain_id)
                        .or_insert((0, vec![]));
                    entry.0 += 1;
                    if entry.1.len() < 5 {
                        entry.1.push(interface.object.name.clone());
                    }
                }
            }
        }
    }
    let mut out: Vec<DomainLink> = weights
        .into_iter()
        .map(|(id, (weight, via))| DomainLink {
            domain_id: id,
            domain_name: inner.domain_name(id),
            weight,
            via,
        })
        .collect();
    out.sort_by(|a, b| {
        b.weight
            .cmp(&a.weight)
            .then_with(|| a.domain_name.cmp(&b.domain_name))
    });
    out
}

fn story_mentions_domain(data: &Dataset, story: &Story, domain_id: ObjectId) -> bool {
    story.structure.chapters.iter().any(|chapter| {
        chapter.blocks.iter().any(|block| match block {
            archifier_core::StoryBlock::Element { element_id, .. } => data
                .elements
                .iter()
                .any(|e| e.object.id == *element_id && e.object.responsible_domain_id == domain_id),
            archifier_core::StoryBlock::Interface { interface_id, .. } => data
                .interfaces
                .iter()
                .any(|i| i.object.id == *interface_id && i.provider_domain_id == domain_id),
            archifier_core::StoryBlock::Diagram { diagram_id, .. } => data
                .diagrams
                .iter()
                .any(|d| d.object.id == *diagram_id && d.object.responsible_domain_id == domain_id),
            _ => false,
        })
    })
}

/// Dernière version publiée d'une interface, départagée par sémantique.
fn latest_published_version(inner: &Inner, interface_id: ObjectId) -> Option<ObjectId> {
    inner
        .data
        .interface_versions
        .iter()
        .filter(|version| {
            version.interface_id == interface_id
                && version.publication_status == PublicationStatus::Published
        })
        .max_by(|a, b| {
            a.published_at
                .cmp(&b.published_at)
                .then_with(|| compare_semver(&a.semantic_version, &b.semantic_version))
        })
        .map(|version| version.id)
}

fn format_date(date: OffsetDateTime) -> String {
    format!(
        "{:02}/{:02}/{}",
        date.day(),
        u8::from(date.month()),
        date.year()
    )
}

/// Champs exclus du diff de révisions : identité technique et date de mise à
/// jour, affichées séparément par le panneau d'historique.
fn is_volatile_revision_path(path: &str) -> bool {
    matches!(
        path,
        "/object/revision" | "/object/updated_at" | "/documentation"
    ) || path.starts_with("/documentation/")
}

fn first_line(markdown: &str) -> String {
    markdown
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| {
            let trimmed = l.trim_start_matches('#').trim();
            if trimmed.chars().count() > 60 {
                trimmed.chars().take(60).collect::<String>() + "…"
            } else {
                trimmed.to_string()
            }
        })
        .unwrap_or_else(|| "(vide)".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed;
    use std::rc::Rc;

    fn seeded() -> Repository {
        let repo = Repository::new(Rc::new(MemoryStore::new()), 1_795_000_000_000, 42);
        seed::seed_demo(&repo).expect("jeu de démonstration");
        repo
    }

    fn id_by_name(repo: &Repository, name: &str) -> ObjectId {
        repo.all_objects()
            .into_iter()
            .find(|object| object.name == name)
            .map(|object| object.id)
            .expect("objet du jeu de démonstration")
    }

    fn renamed_repo() -> (Repository, ObjectId, Revision, Revision) {
        let repo = seeded();
        let id = id_by_name(&repo, "Order Service");
        let before = repo.element(id).expect("élément").object.revision;
        repo.update_element(id, "Order Service v2", serde_json::Map::new())
            .expect("modification");
        let after = repo.element(id).expect("élément").object.revision;
        (repo, id, before, after)
    }

    #[test]
    fn revision_diff_is_field_by_field() {
        let (repo, id, before, after) = renamed_repo();
        let diff = repo.revision_diff(id, before, after).expect("diff");
        assert_eq!(diff.object_id, id);
        assert_eq!(diff.from, before);
        assert_eq!(diff.to, after);
        let name = diff
            .changes
            .iter()
            .find(|change| change.path.ends_with("/name"))
            .expect("chemin /object/name attendu");
        assert_eq!(name.after, Some(Value::String("Order Service v2".into())));
        assert!(name.before.is_some());
    }

    #[test]
    fn revision_diff_is_sorted_by_pointer() {
        let (repo, id, before, after) = renamed_repo();
        let diff = repo.revision_diff(id, before, after).expect("diff");
        let mut paths: Vec<String> = diff.changes.iter().map(|c| c.path.clone()).collect();
        let sorted = {
            let mut clone = paths.clone();
            clone.sort();
            clone
        };
        assert_eq!(paths, sorted);
        paths.dedup();
        assert_eq!(paths, sorted);
    }

    #[test]
    fn revision_diff_from_equals_to_is_empty() {
        let (repo, id, before, _after) = renamed_repo();
        let diff = repo.revision_diff(id, before, before).expect("diff");
        assert!(diff.changes.is_empty());
    }

    #[test]
    fn revision_diff_missing_revision_is_diff_001() {
        let (repo, id, before, after) = renamed_repo();
        let missing = after + 50;
        let error = repo
            .revision_diff(id, before, missing)
            .expect_err("révision absente");
        assert!(error.is("DIFF-001"), "{error}");

        let unknown = ObjectId::from_parts(1, 1);
        let error = repo
            .revision_diff(unknown, before, after)
            .expect_err("objet inconnu");
        assert!(error.is("DIFF-001"), "{error}");
    }

    fn seeded_story(repo: &Repository) -> ObjectId {
        repo.stories()
            .into_iter()
            .next()
            .expect("story du jeu de démonstration")
            .id
    }

    fn referenced_objects(repo: &Repository, story: ObjectId) -> BTreeSet<ObjectId> {
        let detail = repo.story_detail(story).expect("détail de story");
        let mut ids = BTreeSet::new();
        for chapter in &detail.story.structure.chapters {
            for block in &chapter.blocks {
                match block {
                    archifier_core::StoryBlock::Diagram { diagram_id, .. } => {
                        ids.insert(*diagram_id);
                    }
                    archifier_core::StoryBlock::Element { element_id, .. } => {
                        ids.insert(*element_id);
                    }
                    archifier_core::StoryBlock::Interface { interface_id, .. } => {
                        ids.insert(*interface_id);
                    }
                    _ => {}
                }
            }
        }
        ids
    }

    fn referenced_versions(
        repo: &Repository,
        story: ObjectId,
    ) -> BTreeMap<ObjectId, Option<ObjectId>> {
        let detail = repo.story_detail(story).expect("détail de story");
        let mut versions = BTreeMap::new();
        for chapter in &detail.story.structure.chapters {
            for block in &chapter.blocks {
                if let archifier_core::StoryBlock::Interface {
                    interface_id,
                    version_id,
                    ..
                } = block
                {
                    versions.insert(*interface_id, *version_id);
                }
            }
        }
        versions
    }

    #[test]
    fn story_pins_revision() {
        let repo = seeded();
        let story = seeded_story(&repo);
        let references = referenced_objects(&repo, story);
        let references_versions = referenced_versions(&repo, story);
        let before = repo
            .story_detail(story)
            .expect("story")
            .story
            .object
            .revision;

        let count = repo.pin_story_revisions(story).expect("épinglage");
        assert_eq!(count, references.len());
        assert!(count >= 4, "la story de démonstration référence 4 objets");

        let structure = repo.story_detail(story).expect("story").story.structure;
        let pinned = &structure.pinned_revisions;
        assert_eq!(pinned.keys().copied().collect::<BTreeSet<_>>(), references);
        for (object_id, revision) in pinned {
            let current = repo
                .object_ref(*object_id)
                .and_then(|_| {
                    repo.diagram_detail(*object_id)
                        .map(|detail| detail.diagram.object.revision)
                        .or_else(|| {
                            repo.element(*object_id)
                                .map(|element| element.object.revision)
                        })
                        .or_else(|| {
                            repo.interface_detail(*object_id)
                                .map(|detail| detail.interface.object.revision)
                        })
                })
                .expect("objet référencé");
            assert_eq!(*revision, current);
        }

        // Les versions d'interface des blocs sont figées avec les révisions.
        assert_eq!(structure.pinned_versions.len(), references_versions.len());
        for (interface_id, version_id) in &references_versions {
            match version_id {
                Some(expected) => {
                    assert_eq!(structure.pinned_versions.get(interface_id), Some(expected))
                }
                None => assert!(
                    structure.pinned_versions.contains_key(interface_id),
                    "une interface sans version explicite épingle la dernière publiée"
                ),
            }
        }

        let after = repo
            .story_detail(story)
            .expect("story")
            .story
            .object
            .revision;
        assert_eq!(after, before + 1);
        assert!(repo
            .history(story)
            .iter()
            .any(|entry| entry.revision == after && entry.summary.contains("story")));

        // Le playback expose les épingles pour le badge « Récit figé ».
        let playback = repo.playback(story).expect("playback");
        assert_eq!(playback.pinned_revisions, structure.pinned_revisions);
        assert_eq!(playback.pinned_versions, structure.pinned_versions);
    }

    #[test]
    fn story_unpin_clears_pins() {
        let repo = seeded();
        let story = seeded_story(&repo);
        assert!(repo.pin_story_revisions(story).expect("épinglage") > 0);
        repo.unpin_story_revisions(story).expect("dégel");
        let structure = repo.story_detail(story).expect("story").story.structure;
        assert!(structure.pinned_revisions.is_empty());
        assert!(structure.pinned_versions.is_empty());

        // Dégeler une story déjà vide n'écrit rien.
        let revision = repo
            .story_detail(story)
            .expect("story")
            .story
            .object
            .revision;
        repo.unpin_story_revisions(story).expect("dégel idempotent");
        assert_eq!(
            repo.story_detail(story)
                .expect("story")
                .story
                .object
                .revision,
            revision
        );
    }

    #[test]
    fn story_pin_without_snapshot_is_story_003() {
        let repo = seeded();
        let story = seeded_story(&repo);
        let reference = referenced_objects(&repo, story)
            .into_iter()
            .next()
            .expect("objet référencé");
        repo.inner
            .borrow_mut()
            .data
            .revisions
            .remove(reference.as_str());

        let error = repo
            .pin_story_revisions(story)
            .expect_err("snapshot absent");
        assert!(error.is("STORY-003"), "{error}");
        assert!(repo
            .story_detail(story)
            .expect("story")
            .story
            .structure
            .pinned_revisions
            .is_empty());
    }

    #[test]
    fn pinning_unknown_story_is_not_found() {
        let repo = seeded();
        let unknown = ObjectId::from_parts(1, 1);
        assert!(repo
            .pin_story_revisions(unknown)
            .expect_err("épinglage impossible")
            .is("NOT-FOUND"));
        assert!(repo
            .unpin_story_revisions(unknown)
            .expect_err("dégel impossible")
            .is("NOT-FOUND"));
    }
}
