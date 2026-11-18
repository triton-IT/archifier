//! Export Markdown d'un domaine ou d'un objet (T1-05).
//!
//! Le bundle est une valeur pure : aucun accès disque, aucune horloge. Toutes
//! les dates affichées proviennent des objets (`created_at`, `updated_at`), ce
//! qui garantit un export reproductible à révision identique.
//!
//! Arborescence produite, relative à `ExportBundle::root` :
//!
//! ```text
//! index.md
//! domains/<id>.md
//! elements/<id>.md
//! interfaces/<id>.md
//! diagrams/<id>.md
//! diagrams/<id>.svg
//! diagrams/<id>.mmd
//! stories/<id>.md
//! reviews/<id>.md
//! relationships/<id>.md
//! ```
//!
//! Les liens `arch://` sont réécrits vers ces fichiers ; quand la cible n'est
//! pas incluse dans le bundle, le lien devient du texte suivi d'une note
//! `> ⚠ lien cassé vers arch://...`, conformément à la spécification §12.2.

use crate::ids::ObjectId;
use crate::model::{
    ContractType, Dataset, Diagram, DiagramEdge, DiagramNode, Interface, InterfaceVersion,
    Lifecycle, ModelElement, ObjectType, PublicationStatus, RepositoryObject, StoryBlock,
};
use crate::uri::{ArchKind, ArchUri};
use crate::validation::slugify;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Un fichier du bundle. `path` est relatif à la racine du bundle et utilise
/// toujours le séparateur `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFile {
    pub path: String,
    pub content: String,
}

/// Ensemble de fichiers exportés pour un domaine ou un objet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportBundle {
    /// Slug du domaine ou de l'objet exporté.
    pub root: String,
    /// Fichiers triés par `path`.
    pub files: Vec<ExportFile>,
}

/// Erreur d'export, mappée par le dépôt sur `EXPORT-001`/`EXPORT-002`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    #[error("objet ou domaine inconnu")]
    UnknownSubject,
    #[error("diagramme illisible : {0}")]
    UnreadableDiagram(String),
}

impl ExportError {
    /// Code stable exposé par le dépôt (`EXPORT-001`, `EXPORT-002`).
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownSubject => "EXPORT-001",
            Self::UnreadableDiagram(_) => "EXPORT-002",
        }
    }

    pub fn is(&self, code: &str) -> bool {
        self.code() == code
    }
}

// ---------------------------------------------------------------------------
// Plan de bundle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PlannedObject {
    id: ObjectId,
    object_type: ObjectType,
    path: String,
}

#[derive(Debug, Default)]
struct Plan {
    objects: Vec<PlannedObject>,
    paths: HashMap<ObjectId, String>,
}

impl Plan {
    fn path(&self, id: ObjectId) -> Option<&str> {
        self.paths.get(&id).map(String::as_str)
    }
}

/// Exporte un domaine complet : index, page domaine, pages objets et médias.
pub fn export_domain(dataset: &Dataset, domain_id: ObjectId) -> Result<ExportBundle, ExportError> {
    let domain = dataset
        .domains
        .iter()
        .find(|domain| domain.object.id == domain_id)
        .ok_or(ExportError::UnknownSubject)?;

    let mut ids = BTreeSet::new();
    ids.insert(domain_id);
    for element in &dataset.elements {
        if element.object.responsible_domain_id == domain_id {
            ids.insert(element.object.id);
        }
    }
    for interface in &dataset.interfaces {
        if interface.object.responsible_domain_id == domain_id
            || interface.provider_domain_id == domain_id
        {
            ids.insert(interface.object.id);
        }
    }
    for diagram in &dataset.diagrams {
        if diagram.object.responsible_domain_id == domain_id {
            ids.insert(diagram.object.id);
        }
    }
    for story in &dataset.stories {
        if story.object.responsible_domain_id == domain_id {
            ids.insert(story.object.id);
        }
    }
    for review in &dataset.reviews {
        if review.object.responsible_domain_id == domain_id || review.domain_id == domain_id {
            ids.insert(review.object.id);
        }
    }

    // Pages de relations du domaine : elles rendent `arch://relationship/...`
    // résoluble, même si l'index ne liste pas de section « Relations ».
    let members: BTreeSet<ObjectId> = ids.iter().copied().collect();
    for relationship in &dataset.relationships {
        if members.contains(&relationship.source_id) && members.contains(&relationship.target_id) {
            ids.insert(relationship.object.id);
        }
    }
    close_bundle(dataset, &mut ids);

    let root = safe_root(&domain.slug, &fallback_slug(&domain.object.name, domain_id));
    let id_line = format!("Domaine `{}` · `{domain_id}`", md_text(&root));
    build_bundle(
        dataset,
        &root,
        &ids,
        &domain.object.name,
        &domain.description,
        !domain.object.is_active(),
        &id_line,
    )
}

/// Exporte un objet et sa fermeture de voisins directs.
pub fn export_object(dataset: &Dataset, object_id: ObjectId) -> Result<ExportBundle, ExportError> {
    let object = find_object(dataset, object_id).ok_or(ExportError::UnknownSubject)?;

    let mut ids = BTreeSet::new();
    ids.insert(object_id);
    for (neighbour, _) in direct_neighbours(dataset, object_id) {
        ids.insert(neighbour);
    }
    // Relations incidentes : la page de relation fait partie de la fermeture.
    for relationship in &dataset.relationships {
        if relationship.source_id == object_id || relationship.target_id == object_id {
            ids.insert(relationship.object.id);
        }
    }
    if let Some(diagram) = dataset
        .diagrams
        .iter()
        .find(|diagram| diagram.object.id == object_id)
    {
        for node in &diagram.layout.nodes {
            ids.insert(node.object_id);
        }
    }
    if let Some(story) = dataset
        .stories
        .iter()
        .find(|story| story.object.id == object_id)
    {
        for chapter in &story.structure.chapters {
            for block in &chapter.blocks {
                if let Some(referenced) = block_object(block) {
                    ids.insert(referenced);
                }
            }
        }
    }

    close_bundle(dataset, &mut ids);

    let root = fallback_slug(&object.name, object_id);
    let id_line = format!("Objet `{}` · `{object_id}`", md_text(&root));
    build_bundle(
        dataset,
        &root,
        &ids,
        &object.name,
        "",
        !object.is_active(),
        &id_line,
    )
}

/// Ferme le bundle : liens documentaires, nœuds de diagrammes, blocs de récits,
/// extrémités de relations et domaines propriétaires. Les objets hors domaine
/// sont conservés avec une page (stub), pour qu'aucun lien ne soit mort.
fn close_bundle(dataset: &Dataset, ids: &mut BTreeSet<ObjectId>) {
    loop {
        let before = ids.len();
        let current: Vec<ObjectId> = ids.iter().copied().collect();

        for id in &current {
            for uri in crate::uri::extract_uris(&current_documentation(dataset, *id)) {
                if let Some(target) = resolve_uri_target(dataset, &uri) {
                    if find_object(dataset, target).is_some() {
                        ids.insert(target);
                    }
                }
            }
        }
        for id in &current {
            if let Some(diagram) = dataset
                .diagrams
                .iter()
                .find(|diagram| diagram.object.id == *id)
            {
                for node in &diagram.layout.nodes {
                    ids.insert(node.object_id);
                }
            }
            if let Some(story) = dataset.stories.iter().find(|story| story.object.id == *id) {
                for chapter in &story.structure.chapters {
                    for block in &chapter.blocks {
                        if let Some(referenced) = block_object(block) {
                            ids.insert(referenced);
                        }
                    }
                }
            }
            if let Some(relationship) = dataset
                .relationships
                .iter()
                .find(|relationship| relationship.object.id == *id)
            {
                ids.insert(relationship.source_id);
                ids.insert(relationship.target_id);
            }
            if let Some(interface) = dataset
                .interfaces
                .iter()
                .find(|interface| interface.object.id == *id)
            {
                for consumer in &dataset.interface_consumers {
                    if dataset.interface_versions.iter().any(|version| {
                        version.id == consumer.interface_version_id
                            && version.interface_id == interface.object.id
                    }) {
                        ids.insert(consumer.consumer_object_id);
                    }
                }
            }
        }

        ids.retain(|id| find_object(dataset, *id).is_some());
        expand_owners(dataset, ids);
        if ids.len() == before {
            break;
        }
    }
}

fn build_bundle(
    dataset: &Dataset,
    root: &str,
    ids: &BTreeSet<ObjectId>,
    title: &str,
    description: &str,
    archived: bool,
    id_line: &str,
) -> Result<ExportBundle, ExportError> {
    let plan = build_plan(dataset, ids);
    let mut files = vec![normalize_file(
        "index.md".to_string(),
        index_page(dataset, &plan, title, description, archived, id_line),
    )];

    for planned in &plan.objects {
        files.push(normalize_file(
            planned.path.clone(),
            render_object_page(dataset, &plan, planned.id, &planned.path),
        ));
        if planned.object_type == ObjectType::Diagram {
            if let Some(diagram) = dataset
                .diagrams
                .iter()
                .find(|diagram| diagram.object.id == planned.id)
            {
                // §5.7 : un diagramme sans nœud ne produit ni SVG ni Mermaid.
                if diagram.layout.nodes.is_empty() {
                    continue;
                }
                let base = format!("diagrams/{}", planned.id);
                files.push(normalize_file(
                    format!("{base}.svg"),
                    diagram_svg(dataset, diagram)?,
                ));
                files.push(normalize_file(
                    format!("{base}.mmd"),
                    diagram_mermaid(dataset, diagram),
                ));
            }
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ExportBundle {
        root: root.to_string(),
        files,
    })
}

fn build_plan(dataset: &Dataset, ids: &BTreeSet<ObjectId>) -> Plan {
    let mut objects = Vec::new();
    for id in ids {
        if let Some(object) = find_object(dataset, *id) {
            objects.push(PlannedObject {
                id: *id,
                object_type: object.object_type,
                path: page_path(object.object_type, *id),
            });
        }
    }
    objects.sort_by_key(|object| (object.id, object.object_type));
    let mut paths = HashMap::new();
    for object in &objects {
        paths.insert(object.id, object.path.clone());
    }
    Plan { objects, paths }
}

fn page_path(object_type: ObjectType, id: ObjectId) -> String {
    format!("{}/{id}.md", object_type.route_segment())
}

fn fallback_slug(name: &str, id: ObjectId) -> String {
    let slug = slugify(name);
    if slug.is_empty() {
        id.to_string()
    } else {
        slug
    }
}

/// Racine sûre : jamais `.`, `..`, ni séparateur de chemin.
fn safe_root(candidate: &str, fallback: &str) -> String {
    let trimmed = candidate.trim();
    let unsafe_segment = trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\');
    if unsafe_segment {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

/// UTF-8 sans BOM, retours à la ligne `\n`, saut de ligne final.
fn normalize_file(path: String, content: String) -> ExportFile {
    let mut text = content.replace("\r\n", "\n").replace('\r', "\n");
    if let Some(stripped) = text.strip_prefix('\u{feff}') {
        text = stripped.to_string();
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    ExportFile {
        path,
        content: text,
    }
}

/// Ajoute les domaines propriétaires (et fournisseurs) des objets déjà inclus,
/// pour que les liens « Domaine » des pages ne soient jamais morts.
fn expand_owners(dataset: &Dataset, ids: &mut BTreeSet<ObjectId>) {
    loop {
        let mut wanted = Vec::new();
        for id in ids.iter() {
            if let Some(object) = find_object(dataset, *id) {
                wanted.push(object.responsible_domain_id);
            }
            if let Some(interface) = dataset
                .interfaces
                .iter()
                .find(|interface| interface.object.id == *id)
            {
                wanted.push(interface.provider_domain_id);
            }
            if let Some(review) = dataset
                .reviews
                .iter()
                .find(|review| review.object.id == *id)
            {
                wanted.push(review.domain_id);
            }
        }
        let before = ids.len();
        for domain_id in wanted {
            if dataset
                .domains
                .iter()
                .any(|domain| domain.object.id == domain_id)
            {
                ids.insert(domain_id);
            }
        }
        if ids.len() == before {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Index
// ---------------------------------------------------------------------------

fn index_page(
    dataset: &Dataset,
    plan: &Plan,
    title: &str,
    description: &str,
    archived: bool,
    id_line: &str,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# {}\n", md_text(title));
    let _ = writeln!(out, "> {id_line}\n");
    if archived {
        out.push_str("> **Archivé** — consultation seule, dernière révision\n\n");
    }
    if !description.trim().is_empty() {
        out.push_str(description.trim());
        out.push_str("\n\n");
    }
    if plan.objects.is_empty() {
        return out;
    }

    for (object_type, section) in [
        (ObjectType::Domain, "## Domaines"),
        (ObjectType::ModelElement, "## Éléments"),
        (ObjectType::Interface, "## Interfaces"),
        (ObjectType::Diagram, "## Diagrammes"),
        (ObjectType::Story, "## Histoires"),
        (ObjectType::Review, "## Revues"),
    ] {
        let mut members: Vec<&PlannedObject> = plan
            .objects
            .iter()
            .filter(|member| member.object_type == object_type)
            .collect();
        if members.is_empty() {
            continue;
        }
        members.sort_by_key(|member| member.id);
        let _ = writeln!(out, "{section}\n");
        for member in members {
            let name = find_object(dataset, member.id)
                .map(|object| object.name.clone())
                .unwrap_or_else(|| member.id.to_string());
            let _ = writeln!(
                out,
                "- [{}]({})",
                md_text(&name),
                relative_path("index.md", &member.path)
            );
        }
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Page d'objet
// ---------------------------------------------------------------------------

fn render_object_page(dataset: &Dataset, plan: &Plan, id: ObjectId, path: &str) -> String {
    let Some(object) = find_object(dataset, id) else {
        return String::new();
    };
    let mut out = String::new();
    let _ = writeln!(out, "# {}\n", md_text(&object.name));
    if !object.is_active() {
        out.push_str("> **Archivé** — consultation seule, dernière révision\n\n");
    }

    out.push_str("## Identification\n\n| Champ | Valeur |\n|---|---|\n");
    meta_row(
        &mut out,
        "Type",
        &format!(
            "{} (`{}`)",
            kind_label(object.object_type),
            object.object_type.as_str()
        ),
    );
    meta_row(&mut out, "Identifiant", &format!("`{id}`"));
    meta_row(
        &mut out,
        "Domaine",
        &link_to(
            plan,
            path,
            object.responsible_domain_id,
            &domain_name(dataset, object.responsible_domain_id),
        ),
    );
    meta_row(
        &mut out,
        "Cycle de vie",
        match object.lifecycle {
            Lifecycle::Active => "Actif",
            Lifecycle::Archived => "Archivé",
        },
    );
    meta_row(&mut out, "Révision", &object.revision.to_string());
    meta_row(&mut out, "Créé le", &format_date(object.created_at));
    meta_row(&mut out, "Créé par", &format!("`{}`", object.created_by));
    meta_row(&mut out, "Modifié le", &format_date(object.updated_at));
    meta_row(&mut out, "Modifié par", &format!("`{}`", object.updated_by));
    out.push('\n');

    match object.object_type {
        ObjectType::Domain => render_domain_properties(dataset, plan, path, id, &mut out),
        ObjectType::ModelElement => render_element_properties(dataset, plan, path, id, &mut out),
        ObjectType::Interface => render_interface_properties(dataset, plan, path, id, &mut out),
        ObjectType::Diagram => render_diagram_properties(dataset, plan, path, id, &mut out),
        ObjectType::Story => render_story_properties(dataset, plan, path, id, &mut out),
        ObjectType::Review => render_review_properties(dataset, plan, path, id, &mut out),
        ObjectType::Relationship => {
            render_relationship_properties(dataset, plan, path, id, &mut out)
        }
    }

    let documentation = current_documentation(dataset, id);
    if !documentation.trim().is_empty() {
        out.push_str("## Documentation\n\n");
        out.push_str(&rewrite_links(dataset, plan, path, &documentation));
        out.push('\n');
    }

    out.push_str("## Relations et voisins\n\n");
    let neighbours = direct_neighbours(dataset, id);
    if neighbours.is_empty() {
        out.push_str("_Aucun voisin direct._\n");
    } else {
        for (neighbour, relation) in neighbours {
            let name = find_object(dataset, neighbour)
                .map(|object| object.name.clone())
                .unwrap_or_else(|| neighbour.to_string());
            let _ = writeln!(
                out,
                "- {} — {}",
                link_to(plan, path, neighbour, &name),
                relation
            );
        }
    }
    out
}

fn render_domain_properties(
    dataset: &Dataset,
    _plan: &Plan,
    _path: &str,
    id: ObjectId,
    out: &mut String,
) {
    if let Some(domain) = dataset.domains.iter().find(|domain| domain.object.id == id) {
        out.push_str("## Propriétés\n\n");
        meta_row(out, "Slug", &format!("`{}`", domain.slug));
        out.push('\n');
        if !domain.description.trim().is_empty() {
            out.push_str("## Description\n\n");
            out.push_str(domain.description.trim());
            out.push_str("\n\n");
        }
        out.push_str("## Contenu\n\n| Objet | Nombre |\n|---|---|\n");
        meta_row(
            out,
            "Éléments",
            &dataset
                .elements
                .iter()
                .filter(|element| element.object.responsible_domain_id == id)
                .count()
                .to_string(),
        );
        meta_row(
            out,
            "Interfaces",
            &dataset
                .interfaces
                .iter()
                .filter(|interface| interface.provider_domain_id == id)
                .count()
                .to_string(),
        );
        meta_row(
            out,
            "Diagrammes",
            &dataset
                .diagrams
                .iter()
                .filter(|diagram| diagram.object.responsible_domain_id == id)
                .count()
                .to_string(),
        );
        meta_row(
            out,
            "Récits",
            &dataset
                .stories
                .iter()
                .filter(|story| story.object.responsible_domain_id == id)
                .count()
                .to_string(),
        );
        meta_row(
            out,
            "Revues",
            &dataset
                .reviews
                .iter()
                .filter(|review| review.domain_id == id)
                .count()
                .to_string(),
        );
        out.push('\n');
    }
}

fn render_element_properties(
    dataset: &Dataset,
    plan: &Plan,
    path: &str,
    id: ObjectId,
    out: &mut String,
) {
    let Some(element): Option<&ModelElement> = dataset
        .elements
        .iter()
        .find(|element| element.object.id == id)
    else {
        return;
    };
    out.push_str("## Propriétés\n\n");
    meta_row(out, "Notation", notation_label(element.notation.as_str()));
    meta_row(out, "Métaclasse", &format!("`{}`", element.metaclass));
    meta_row(
        out,
        "Version de métamodèle",
        &format!("`{}`", element.metamodel_version),
    );
    if let Some(parent_id) = element.parent_id {
        let parent_name = find_object(dataset, parent_id)
            .map(|object| object.name.clone())
            .unwrap_or_else(|| parent_id.to_string());
        meta_row(
            out,
            "Élément parent",
            &link_to(plan, path, parent_id, &parent_name),
        );
    }
    out.push('\n');

    if !element.semantic_properties.is_empty() {
        out.push_str("### Propriétés sémantiques\n\n| Propriété | Valeur |\n|---|---|\n");
        let mut keys: Vec<&String> = element.semantic_properties.keys().collect();
        keys.sort();
        for key in keys {
            let value = element
                .semantic_properties
                .get(key)
                .map(render_json_value)
                .unwrap_or_default();
            meta_row(out, key, &value);
        }
        out.push('\n');
    }
    render_diagrams_using(dataset, plan, path, id, out);
}

fn render_interface_properties(
    dataset: &Dataset,
    plan: &Plan,
    path: &str,
    id: ObjectId,
    out: &mut String,
) {
    let Some(interface): Option<&Interface> = dataset
        .interfaces
        .iter()
        .find(|interface| interface.object.id == id)
    else {
        return;
    };
    out.push_str("## Propriétés\n\n");
    meta_row(
        out,
        "Clé logique",
        &format!("`{}`", interface.logical_interface_key),
    );
    meta_row(out, "Catégorie", interface.category.label());
    meta_row(
        out,
        "Domaine fournisseur",
        &link_to(
            plan,
            path,
            interface.provider_domain_id,
            &domain_name(dataset, interface.provider_domain_id),
        ),
    );
    meta_row(
        out,
        "Type de contrat",
        contract_label(interface.contract_type),
    );
    out.push('\n');

    let mut versions: Vec<&InterfaceVersion> = dataset
        .interface_versions
        .iter()
        .filter(|version| version.interface_id == id)
        .collect();
    versions.sort_by(|a, b| {
        a.semantic_version
            .cmp(&b.semantic_version)
            .then_with(|| a.id.cmp(&b.id))
    });
    if !versions.is_empty() {
        out.push_str("## Versions\n\n");
        out.push_str(
            "| Version | Publication | Rupture | Début | Fin | Créée le |\n|---|---|---|---|---|---|\n",
        );
        for version in &versions {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} | {} | {} |",
                md_text(&version.semantic_version),
                publication_label(version.publication_status),
                if version.breaking_change {
                    "oui"
                } else {
                    "non"
                },
                version
                    .available_from
                    .map(format_date)
                    .unwrap_or_else(|| "—".into()),
                version
                    .available_until
                    .map(format_date)
                    .unwrap_or_else(|| "—".into()),
                format_date(version.created_at),
            );
        }
        out.push('\n');
    }

    if versions
        .iter()
        .any(|version| version.contract_format.is_some() || version.contract_hash.is_some())
    {
        out.push_str("## Contrats\n\n| Version | Format | Hash |\n|---|---|---|\n");
        for version in &versions {
            if version.contract_format.is_none() && version.contract_hash.is_none() {
                continue;
            }
            let _ = writeln!(
                out,
                "| `{}` | {} | {} |",
                md_text(&version.semantic_version),
                version
                    .contract_format
                    .map(contract_format_label)
                    .unwrap_or("—"),
                version
                    .contract_hash
                    .as_deref()
                    .map(|hash| format!("`{}`", md_text(hash)))
                    .unwrap_or_else(|| "—".to_string())
            );
        }
        out.push('\n');
    }

    if !interface.notation_bindings.is_empty() {
        out.push_str("## Rattachements\n\n");
        for binding in &interface.notation_bindings {
            let _ = writeln!(
                out,
                "- {} · `{}`{}",
                notation_label(binding.notation.as_str()),
                binding.metaclass,
                binding
                    .element_id
                    .map(|element_id| {
                        let name = find_object(dataset, element_id)
                            .map(|object| object.name.clone())
                            .unwrap_or_else(|| element_id.to_string());
                        format!(" — {}", link_to(plan, path, element_id, &name))
                    })
                    .unwrap_or_default()
            );
        }
        out.push('\n');
    }

    let mut consumers: BTreeSet<ObjectId> = BTreeSet::new();
    for version in &versions {
        for consumer in &dataset.interface_consumers {
            if consumer.interface_version_id == version.id {
                consumers.insert(consumer.consumer_object_id);
            }
        }
    }
    if !consumers.is_empty() {
        out.push_str("## Consommateurs\n\n");
        for consumer in consumers {
            let name = find_object(dataset, consumer)
                .map(|object| object.name.clone())
                .unwrap_or_else(|| consumer.to_string());
            let _ = writeln!(out, "- {}", link_to(plan, path, consumer, &name));
        }
        out.push('\n');
    }
    render_diagrams_using(dataset, plan, path, id, out);
}

fn render_diagrams_using(
    dataset: &Dataset,
    plan: &Plan,
    path: &str,
    id: ObjectId,
    out: &mut String,
) {
    let mut diagrams: Vec<&crate::model::Diagram> = dataset
        .diagrams
        .iter()
        .filter(|diagram| diagram.layout.nodes.iter().any(|node| node.object_id == id))
        .collect();
    diagrams.sort_by_key(|diagram| diagram.object.id);
    if diagrams.is_empty() {
        return;
    }
    out.push_str("## Diagrammes\n\n");
    for diagram in diagrams {
        let _ = writeln!(
            out,
            "- {}",
            link_to(plan, path, diagram.object.id, &diagram.object.name)
        );
    }
    out.push('\n');
}

fn render_diagram_properties(
    dataset: &Dataset,
    plan: &Plan,
    path: &str,
    id: ObjectId,
    out: &mut String,
) {
    let Some(diagram) = dataset
        .diagrams
        .iter()
        .find(|diagram| diagram.object.id == id)
    else {
        return;
    };
    out.push_str("## Propriétés\n\n");
    meta_row(out, "Notation", diagram_notation_label(diagram));
    meta_row(out, "Nœuds", &diagram.layout.nodes.len().to_string());
    meta_row(out, "Arêtes", &diagram.layout.edges.len().to_string());
    out.push('\n');

    if !diagram.description.trim().is_empty() {
        out.push_str("## Description\n\n");
        out.push_str(&rewrite_links(dataset, plan, path, &diagram.description));
        out.push('\n');
    }

    let svg_path = relative_path(path, &format!("diagrams/{id}.svg"));
    let mmd_path = relative_path(path, &format!("diagrams/{id}.mmd"));
    out.push_str("## Rendu\n\n");
    if diagram.layout.nodes.is_empty() {
        out.push_str("_Diagramme vide : aucun SVG ni Mermaid produit._\n\n");
    } else {
        let _ = writeln!(
            out,
            "[Voir le SVG]({svg_path}) · [Source Mermaid]({mmd_path})\n"
        );
        let _ = writeln!(out, "![{}]({svg_path})\n", md_text(&diagram.object.name));
        out.push_str("```mermaid\n");
        out.push_str(&diagram_mermaid(dataset, diagram));
        out.push_str("```\n\n");
    }

    let mut nodes: Vec<&DiagramNode> = diagram.layout.nodes.iter().collect();
    nodes.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.object_id.cmp(&b.object_id)));
    if !nodes.is_empty() {
        out.push_str("## Nœuds\n\n| Nœud | Objet | Type |\n|---|---|---|\n");
        for node in nodes {
            let name = find_object(dataset, node.object_id)
                .map(|object| object.name.clone())
                .unwrap_or_else(|| node.object_id.to_string());
            let _ = writeln!(
                out,
                "| `{}` | {} | `{}` |",
                md_text(&node.id),
                link_to(plan, path, node.object_id, &name),
                node_kind_label(node)
            );
        }
        out.push('\n');
    }

    let mut edges: Vec<&DiagramEdge> = diagram.layout.edges.iter().collect();
    edges.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then_with(|| a.relationship_id.cmp(&b.relationship_id))
    });
    if !edges.is_empty() {
        out.push_str("## Arêtes\n\n| Arête | Relation | Source | Cible |\n|---|---|---|---|\n");
        for edge in edges {
            if let Some(relationship) = dataset
                .relationships
                .iter()
                .find(|relationship| relationship.object.id == edge.relationship_id)
            {
                let source_name = find_object(dataset, relationship.source_id)
                    .map(|object| object.name.clone())
                    .unwrap_or_else(|| relationship.source_id.to_string());
                let target_name = find_object(dataset, relationship.target_id)
                    .map(|object| object.name.clone())
                    .unwrap_or_else(|| relationship.target_id.to_string());
                let _ = writeln!(
                    out,
                    "| `{}` | `{}` | {} | {} |",
                    md_text(&edge.id),
                    md_text(&relationship.relation_type),
                    link_to(plan, path, relationship.source_id, &source_name),
                    link_to(plan, path, relationship.target_id, &target_name)
                );
            } else {
                let _ = writeln!(
                    out,
                    "| `{}` | _relation introuvable_ | — | — |",
                    md_text(&edge.id)
                );
            }
        }
        out.push('\n');
    }
}

fn render_story_properties(
    dataset: &Dataset,
    plan: &Plan,
    path: &str,
    id: ObjectId,
    out: &mut String,
) {
    let Some(story) = dataset.stories.iter().find(|story| story.object.id == id) else {
        return;
    };
    out.push_str("## Chapitres\n\n");
    if story.structure.chapters.is_empty() {
        out.push_str("_Aucun chapitre._\n\n");
    }
    for (index, chapter) in story.structure.chapters.iter().enumerate() {
        let _ = writeln!(out, "### {}. {}\n", index + 1, md_text(&chapter.title));
        if chapter.blocks.is_empty() {
            out.push_str("_Chapitre vide._\n\n");
            continue;
        }
        for block in &chapter.blocks {
            match block_object(block) {
                Some(referenced) => {
                    let name = find_object(dataset, referenced)
                        .map(|object| object.name.clone())
                        .unwrap_or_else(|| referenced.to_string());
                    let _ = writeln!(
                        out,
                        "- {} : {}",
                        block_kind_label(block),
                        link_to(plan, path, referenced, &name)
                    );
                }
                None => {
                    let _ = writeln!(
                        out,
                        "- {} : {}",
                        block_kind_label(block),
                        block_summary(block)
                    );
                }
            }
        }
        out.push('\n');
    }

    if !story.structure.pinned_revisions.is_empty() {
        out.push_str("## Révisions épinglées\n\n");
        for (object_id, revision) in &story.structure.pinned_revisions {
            let name = find_object(dataset, *object_id)
                .map(|object| object.name.clone())
                .unwrap_or_else(|| object_id.to_string());
            let _ = writeln!(
                out,
                "- {} — révision {}",
                link_to(plan, path, *object_id, &name),
                revision
            );
        }
        out.push('\n');
    }
}

fn render_review_properties(
    dataset: &Dataset,
    plan: &Plan,
    path: &str,
    id: ObjectId,
    out: &mut String,
) {
    let Some(review) = dataset.reviews.iter().find(|review| review.object.id == id) else {
        return;
    };
    out.push_str("## Propriétés\n\n");
    meta_row(
        out,
        "Statut",
        match review.status {
            crate::model::ReviewStatus::Open => "Ouverte",
            crate::model::ReviewStatus::Closed => "Clôturée",
        },
    );
    meta_row(
        out,
        "Domaine",
        &link_to(
            plan,
            path,
            review.domain_id,
            &domain_name(dataset, review.domain_id),
        ),
    );
    meta_row(
        out,
        "Fenêtre",
        &format!(
            "{} → {}",
            review
                .temporal_window_start
                .map(format_date)
                .unwrap_or_else(|| "—".into()),
            review
                .temporal_window_end
                .map(format_date)
                .unwrap_or_else(|| "—".into())
        ),
    );
    out.push('\n');

    if !review.markdown_summary.trim().is_empty() {
        out.push_str("## Résumé\n\n");
        out.push_str(&rewrite_links(
            dataset,
            plan,
            path,
            &review.markdown_summary,
        ));
        out.push('\n');
    }
}

fn render_relationship_properties(
    dataset: &Dataset,
    plan: &Plan,
    path: &str,
    id: ObjectId,
    out: &mut String,
) {
    let Some(relationship) = dataset
        .relationships
        .iter()
        .find(|relationship| relationship.object.id == id)
    else {
        return;
    };
    out.push_str("## Propriétés\n\n");
    meta_row(
        out,
        "Type de relation",
        &format!("`{}`", relationship.relation_type),
    );
    meta_row(
        out,
        "Notation",
        notation_label(relationship.notation.as_str()),
    );
    let source_name = find_object(dataset, relationship.source_id)
        .map(|object| object.name.clone())
        .unwrap_or_else(|| relationship.source_id.to_string());
    let target_name = find_object(dataset, relationship.target_id)
        .map(|object| object.name.clone())
        .unwrap_or_else(|| relationship.target_id.to_string());
    meta_row(
        out,
        "Source",
        &link_to(plan, path, relationship.source_id, &source_name),
    );
    meta_row(
        out,
        "Cible",
        &link_to(plan, path, relationship.target_id, &target_name),
    );
    out.push('\n');
}

// ---------------------------------------------------------------------------
// SVG et Mermaid
// ---------------------------------------------------------------------------

fn diagram_svg(dataset: &Dataset, diagram: &Diagram) -> Result<String, ExportError> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for node in &diagram.layout.nodes {
        if !node.x.is_finite()
            || !node.y.is_finite()
            || !node.w.is_finite()
            || !node.h.is_finite()
            || node.w <= 0.0
            || node.h <= 0.0
        {
            return Err(ExportError::UnreadableDiagram(format!(
                "« {} » : layout absent ou géométrie invalide",
                diagram.object.name
            )));
        }
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
        max_x = max_x.max(node.x + node.w);
        max_y = max_y.max(node.y + node.h);
    }

    // bbox = union des rectangles + padding 8 (§5.7). viewport, annotations et
    // waypoints ne participent pas à la bbox.
    let padding = 8.0;
    let (view_x, view_y, width, height) = (
        min_x - padding,
        min_y - padding,
        (max_x - min_x).max(1.0) + padding * 2.0,
        (max_y - min_y).max(1.0) + padding * 2.0,
    );

    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg width="{w}" height="{h}" viewBox="{vx} {vy} {vw} {vh}" role="img" aria-label="{label}">"#,
        w = coord(width),
        h = coord(height),
        vx = coord(view_x),
        vy = coord(view_y),
        vw = coord(width),
        vh = coord(height),
        label = xml_escape(&diagram.object.name),
    );
    let _ = writeln!(out, "  <title>{}</title>", xml_escape(&diagram.object.name));

    for edge in &diagram.layout.edges {
        let Some((x1, y1, x2, y2)) = edge_points(dataset, diagram, edge) else {
            continue;
        };
        let _ = writeln!(
            out,
            r##"  <line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#5b6470" stroke-width="1.5" />"##,
            x1 = coord(x1),
            y1 = coord(y1),
            x2 = coord(x2),
            y2 = coord(y2),
        );
    }

    for node in &diagram.layout.nodes {
        let label = find_object(dataset, node.object_id)
            .map(|object| object.name.as_str())
            .unwrap_or("Objet supprimé");
        let _ = writeln!(
            out,
            r##"  <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="6" ry="6" fill="#ffffff" stroke="#3b4252" stroke-width="1.5" />"##,
            x = coord(node.x),
            y = coord(node.y),
            w = coord(node.w),
            h = coord(node.h),
        );
        let _ = writeln!(
            out,
            r##"  <text x="{x}" y="{y}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="12" fill="#1f2430">{label}</text>"##,
            x = coord(node.x + node.w / 2.0),
            y = coord(node.y + node.h / 2.0),
            label = xml_escape(label),
        );
    }
    out.push_str("</svg>\n");
    Ok(out)
}

fn edge_points(
    dataset: &Dataset,
    diagram: &Diagram,
    edge: &DiagramEdge,
) -> Option<(f64, f64, f64, f64)> {
    if let (Some(first), Some(last)) = (edge.waypoints.first(), edge.waypoints.last()) {
        return Some((first.x, first.y, last.x, last.y));
    }
    let relationship = dataset
        .relationships
        .iter()
        .find(|relationship| relationship.object.id == edge.relationship_id)?;
    let source = diagram
        .layout
        .nodes
        .iter()
        .find(|node| node.object_id == relationship.source_id)?;
    let target = diagram
        .layout
        .nodes
        .iter()
        .find(|node| node.object_id == relationship.target_id)?;
    Some((
        source.x + source.w / 2.0,
        source.y + source.h / 2.0,
        target.x + target.w / 2.0,
        target.y + target.h / 2.0,
    ))
}

fn diagram_mermaid(dataset: &Dataset, diagram: &Diagram) -> String {
    let mut out = String::from("flowchart LR\n");
    for (index, node) in diagram.layout.nodes.iter().enumerate() {
        let label = find_object(dataset, node.object_id)
            .map(|object| object.name.as_str())
            .unwrap_or("Objet supprimé");
        let _ = writeln!(out, "  n{index}[\"{}\"]", mermaid_escape(label));
    }

    for edge in &diagram.layout.edges {
        let Some(relationship) = dataset
            .relationships
            .iter()
            .find(|relationship| relationship.object.id == edge.relationship_id)
        else {
            continue;
        };
        // Premier nœud dont `object_id` correspond, dans l'ordre du tableau.
        let source = diagram
            .layout
            .nodes
            .iter()
            .position(|node| node.object_id == relationship.source_id);
        let target = diagram
            .layout
            .nodes
            .iter()
            .position(|node| node.object_id == relationship.target_id);
        if let (Some(source), Some(target)) = (source, target) {
            let _ = writeln!(
                out,
                "  n{source} -->|{}| n{target}",
                mermaid_escape(&relationship.relation_type)
            );
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Réécriture des liens `arch://`
// ---------------------------------------------------------------------------

fn rewrite_links(dataset: &Dataset, plan: &Plan, from_path: &str, markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len() + 32);
    let mut remaining = markdown;
    let mut broken: BTreeSet<String> = BTreeSet::new();

    loop {
        let Some(index) = remaining.find("arch://") else {
            out.push_str(remaining);
            break;
        };
        let before = &remaining[..index];
        let after = &remaining[index..];

        if let Some(stripped) = before.strip_suffix("](") {
            if let Some(close) = after.find(')') {
                if let Some(open) = stripped.rfind('[') {
                    let label = &stripped[open + 1..];
                    let raw_uri = &after[..close];
                    out.push_str(&remaining[..open]);
                    match resolve_uri(dataset, plan, from_path, raw_uri) {
                        Some(link) => {
                            let _ = write!(out, "[{label}]({link})");
                        }
                        None => {
                            out.push_str(label);
                            broken.insert(raw_uri.to_string());
                        }
                    }
                    remaining = &after[close + 1..];
                    continue;
                }
            }
        }

        let end = after
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(
                        character,
                        ')' | ']' | '<' | '>' | '"' | '\'' | '`' | ',' | ';'
                    )
            })
            .unwrap_or(after.len());
        let raw_uri = after[..end].trim_end_matches(['.', ':', '!', '?']);
        out.push_str(&remaining[..index]);
        match resolve_uri(dataset, plan, from_path, raw_uri) {
            Some(link) => {
                let label = uri_label(dataset, raw_uri);
                let _ = write!(out, "[{label}]({link})");
            }
            None => {
                out.push_str(raw_uri);
                broken.insert(raw_uri.to_string());
            }
        }
        remaining = &after[raw_uri.len()..];
    }

    if !broken.is_empty() {
        out.push_str("\n\n");
        for uri in broken {
            let _ = writeln!(out, "> ⚠ lien cassé vers {uri}");
        }
    }
    out
}

fn resolve_uri(dataset: &Dataset, plan: &Plan, from_path: &str, raw_uri: &str) -> Option<String> {
    let uri: ArchUri = raw_uri.parse().ok()?;
    let target_id = resolve_uri_target(dataset, &uri)?;
    find_object(dataset, target_id)?;
    let target_path = plan.path(target_id)?;
    let mut link = relative_path(from_path, target_path);
    if uri.kind == ArchKind::Version {
        if let Some(version) = dataset
            .interface_versions
            .iter()
            .find(|version| version.id == uri.id)
        {
            let _ = write!(link, "#v{}", version.semantic_version);
        }
    }
    Some(link)
}

/// Résout l'objet réellement visé par une URI. Une URI de version pointe vers
/// la page de son interface ; une relation possède sa propre page.
fn resolve_uri_target(dataset: &Dataset, uri: &ArchUri) -> Option<ObjectId> {
    match uri.kind {
        ArchKind::Version => dataset
            .interface_versions
            .iter()
            .find(|version| version.id == uri.id)
            .map(|version| version.interface_id),
        _ => Some(uri.id),
    }
}

fn uri_label(dataset: &Dataset, raw_uri: &str) -> String {
    raw_uri
        .parse::<ArchUri>()
        .ok()
        .and_then(|uri| resolve_uri_target(dataset, &uri))
        .and_then(|id| find_object(dataset, id))
        .map(|object| md_text(&object.name))
        .unwrap_or_else(|| raw_uri.to_string())
}

/// Calcule un lien relatif entre deux chemins du bundle (séparateur `/`).
fn relative_path(from_path: &str, to_path: &str) -> String {
    let from: Vec<&str> = from_path.split('/').collect();
    let to: Vec<&str> = to_path.split('/').collect();
    let from_dir = &from[..from.len().saturating_sub(1)];
    let mut common = 0;
    while common < from_dir.len() && common + 1 < to.len() && from_dir[common] == to[common] {
        common += 1;
    }
    let mut out = String::new();
    for _ in common..from_dir.len() {
        out.push_str("../");
    }
    out.push_str(&to[common..].join("/"));
    out
}

fn link_to(plan: &Plan, from_path: &str, id: ObjectId, label: &str) -> String {
    match plan.path(id) {
        Some(target) => format!("[{}]({})", md_text(label), relative_path(from_path, target)),
        None => md_text(label),
    }
}

// ---------------------------------------------------------------------------
// Voisinage et documentation
// ---------------------------------------------------------------------------

fn direct_neighbours(dataset: &Dataset, id: ObjectId) -> Vec<(ObjectId, String)> {
    let mut out = Vec::new();
    for relationship in &dataset.relationships {
        if relationship.source_id == id && relationship.target_id != id {
            out.push((
                relationship.target_id,
                format!("{} (sortante)", relationship.relation_type),
            ));
        } else if relationship.target_id == id && relationship.source_id != id {
            out.push((
                relationship.source_id,
                format!("{} (entrante)", relationship.relation_type),
            ));
        }
    }
    for consumer in &dataset.interface_consumers {
        let Some(version) = dataset
            .interface_versions
            .iter()
            .find(|version| version.id == consumer.interface_version_id)
        else {
            continue;
        };
        if version.interface_id == id && consumer.consumer_object_id != id {
            out.push((consumer.consumer_object_id, "consommation".to_string()));
        }
        if consumer.consumer_object_id == id && version.interface_id != id {
            out.push((version.interface_id, "consommation".to_string()));
        }
    }
    for diagram in &dataset.diagrams {
        if diagram.object.id != id && diagram.layout.nodes.iter().any(|node| node.object_id == id) {
            out.push((diagram.object.id, "représenté dans".to_string()));
        }
    }
    for story in &dataset.stories {
        if story.object.id != id
            && story.structure.chapters.iter().any(|chapter| {
                chapter
                    .blocks
                    .iter()
                    .any(|block| block_object(block) == Some(id))
            })
        {
            out.push((story.object.id, "cité dans".to_string()));
        }
    }
    for interface in &dataset.interfaces {
        if interface.object.id != id
            && interface
                .notation_bindings
                .iter()
                .any(|binding| binding.element_id == Some(id))
        {
            out.push((interface.object.id, "rattachée à".to_string()));
        }
    }
    out.sort_by(|a, b| (a.0, a.1.as_str()).cmp(&(b.0, b.1.as_str())));
    out.dedup();
    out
}

fn current_documentation(dataset: &Dataset, id: ObjectId) -> String {
    dataset
        .documentation
        .iter()
        .filter(|block| block.owner_object_id == id)
        .max_by_key(|block| block.revision)
        .map(|block| block.markdown.clone())
        .unwrap_or_default()
}

fn block_object(block: &StoryBlock) -> Option<ObjectId> {
    match block {
        StoryBlock::Diagram { diagram_id, .. } => Some(*diagram_id),
        StoryBlock::Element { element_id, .. } => Some(*element_id),
        StoryBlock::Interface { interface_id, .. } => Some(*interface_id),
        StoryBlock::Markdown { .. } | StoryBlock::View { .. } | StoryBlock::Transition { .. } => {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Accès au Dataset
// ---------------------------------------------------------------------------

fn find_object(dataset: &Dataset, id: ObjectId) -> Option<&RepositoryObject> {
    if let Some(domain) = dataset.domains.iter().find(|domain| domain.object.id == id) {
        return Some(&domain.object);
    }
    if let Some(element) = dataset
        .elements
        .iter()
        .find(|element| element.object.id == id)
    {
        return Some(&element.object);
    }
    if let Some(relationship) = dataset
        .relationships
        .iter()
        .find(|relationship| relationship.object.id == id)
    {
        return Some(&relationship.object);
    }
    if let Some(interface) = dataset
        .interfaces
        .iter()
        .find(|interface| interface.object.id == id)
    {
        return Some(&interface.object);
    }
    if let Some(diagram) = dataset
        .diagrams
        .iter()
        .find(|diagram| diagram.object.id == id)
    {
        return Some(&diagram.object);
    }
    if let Some(story) = dataset.stories.iter().find(|story| story.object.id == id) {
        return Some(&story.object);
    }
    if let Some(review) = dataset.reviews.iter().find(|review| review.object.id == id) {
        return Some(&review.object);
    }
    None
}

fn domain_name(dataset: &Dataset, id: ObjectId) -> String {
    dataset
        .domains
        .iter()
        .find(|domain| domain.object.id == id)
        .map(|domain| domain.object.name.clone())
        .unwrap_or_else(|| "—".to_string())
}

// ---------------------------------------------------------------------------
// Libellés et formatage
// ---------------------------------------------------------------------------

fn kind_label(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::Domain => "Domaine",
        ObjectType::ModelElement => "Élément",
        ObjectType::Interface => "Interface",
        ObjectType::Diagram => "Diagramme",
        ObjectType::Story => "Récit",
        ObjectType::Review => "Revue",
        ObjectType::Relationship => "Relation",
    }
}

fn notation_label(notation: &str) -> &'static str {
    match notation {
        "ARCHIMATE" => "ArchiMate",
        "BPMN" => "BPMN",
        "UML" => "UML",
        "NATIVE" => "Natif",
        _ => "Inconnue",
    }
}

fn diagram_notation_label(diagram: &Diagram) -> &'static str {
    match diagram.notation {
        crate::model::DiagramNotation::Archimate => "ArchiMate",
        crate::model::DiagramNotation::Bpmn => "BPMN",
        crate::model::DiagramNotation::Uml => "UML",
        crate::model::DiagramNotation::Free => "Libre",
    }
}

fn contract_label(contract_type: ContractType) -> &'static str {
    match contract_type {
        ContractType::None => "Aucun",
        ContractType::OpenApi => "OpenAPI",
        ContractType::Event => "Événement",
        ContractType::Other => "Autre",
    }
}

fn contract_format_label(format: crate::openapi::ContractFormat) -> &'static str {
    use crate::openapi::ContractFormat;
    match format {
        ContractFormat::OpenApi30 => "OpenAPI 3.0",
        ContractFormat::OpenApi31 => "OpenAPI 3.1",
        ContractFormat::OpenApi32 => "OpenAPI 3.2",
        ContractFormat::AsyncApi2 => "AsyncAPI 2",
        ContractFormat::AsyncApi3 => "AsyncAPI 3",
    }
}

fn publication_label(status: PublicationStatus) -> &'static str {
    match status {
        PublicationStatus::Draft => "Brouillon",
        PublicationStatus::Published => "Publiée",
        PublicationStatus::Withdrawn => "Retirée",
    }
}

fn node_kind_label(node: &DiagramNode) -> &'static str {
    match node.kind {
        crate::model::NodeKind::Element => "ELEMENT",
        crate::model::NodeKind::Interface => "INTERFACE",
        crate::model::NodeKind::DiagramRef => "DIAGRAM_REF",
        crate::model::NodeKind::Group => "GROUP",
    }
}

fn block_kind_label(block: &StoryBlock) -> &'static str {
    match block {
        StoryBlock::Markdown { .. } => "markdown",
        StoryBlock::Diagram { .. } => "diagramme",
        StoryBlock::View { .. } => "vue",
        StoryBlock::Element { .. } => "élément",
        StoryBlock::Interface { .. } => "interface",
        StoryBlock::Transition { .. } => "transition",
    }
}

fn block_summary(block: &StoryBlock) -> String {
    match block {
        StoryBlock::Markdown { markdown, .. } => truncate(
            markdown
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .trim_start_matches('#')
                .trim(),
            80,
        ),
        StoryBlock::Diagram { diagram_id, .. } => format!("diagramme `{diagram_id}`"),
        StoryBlock::View { view, .. } => format!("vue `{}`", md_text(view)),
        StoryBlock::Element { element_id, .. } => format!("élément `{element_id}`"),
        StoryBlock::Interface { interface_id, .. } => format!("interface `{interface_id}`"),
        StoryBlock::Transition { label, .. } => format!("transition « {} »", md_text(label)),
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn format_date(date: OffsetDateTime) -> String {
    date.format(&Rfc3339)
        .unwrap_or_else(|_| date.unix_timestamp().to_string())
}

fn render_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn meta_row(out: &mut String, label: &str, value: &str) {
    let _ = writeln!(out, "| {} | {} |", table_cell(label), table_cell(value));
}

fn table_cell(value: &str) -> String {
    value.replace('\n', " ").replace('|', "\\|")
}

/// Échappe un texte destiné à un libellé Markdown simple.
fn md_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn mermaid_escape(value: &str) -> String {
    value.replace('"', "'").replace('|', "/").replace('\n', " ")
}

/// Coordonnée SVG à une décimale (§5.7).
fn coord(value: f64) -> String {
    if !value.is_finite() {
        return "0.0".to_string();
    }
    format!("{value:.1}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Chapter, DiagramEdge, DiagramLayout, DiagramNode, DiagramNotation, Domain,
        InterfaceCategory, NodeKind, Point, Review, ReviewStatus, Story, StoryStructure, Viewport,
    };
    use serde_json::json;
    use time::macros::datetime;

    fn oid(entropy: u128) -> ObjectId {
        ObjectId::from_parts(1_700_000_000_000, entropy)
    }

    fn date() -> OffsetDateTime {
        datetime!(2026-10-01 09:00:00 UTC)
    }

    fn object(
        id: ObjectId,
        object_type: ObjectType,
        name: &str,
        domain: ObjectId,
    ) -> RepositoryObject {
        RepositoryObject {
            id,
            object_type,
            name: name.to_string(),
            responsible_domain_id: domain,
            lifecycle: Lifecycle::Active,
            revision: 1,
            created_at: date(),
            created_by: "u1".to_string(),
            updated_at: date(),
            updated_by: "u1".to_string(),
        }
    }

    struct Fixture {
        dataset: Dataset,
        domain: ObjectId,
        order: ObjectId,
        pricing: ObjectId,
        payment: ObjectId,
        diagram: ObjectId,
    }

    fn fixture() -> Fixture {
        let domain = oid(1);
        let order = oid(2);
        let pricing = oid(3);
        let payment = oid(4);
        let relationship = oid(5);
        let diagram = oid(6);
        let story = oid(7);
        let review = oid(8);

        let mut dataset = Dataset::default();
        dataset.domains.push(Domain {
            object: object(domain, ObjectType::Domain, "Paiements", domain),
            slug: "paiements".to_string(),
            description: "Domaine de démonstration.".to_string(),
        });
        dataset.elements.push(ModelElement {
            object: object(order, ObjectType::ModelElement, "Order Service", domain),
            notation: crate::model::Notation::Archimate,
            metamodel_version: "archimate:3.2".to_string(),
            metaclass: "ApplicationComponent".to_string(),
            parent_id: None,
            semantic_properties: serde_json::Map::new(),
        });
        dataset.elements.push(ModelElement {
            object: object(pricing, ObjectType::ModelElement, "Pricing Service", domain),
            notation: crate::model::Notation::Archimate,
            metamodel_version: "archimate:3.2".to_string(),
            metaclass: "ApplicationService".to_string(),
            parent_id: None,
            semantic_properties: json!({ "tier": "gold" })
                .as_object()
                .cloned()
                .unwrap_or_default(),
        });
        dataset.interfaces.push(Interface {
            object: object(payment, ObjectType::Interface, "Payment API", domain),
            logical_interface_key: "payment-api".to_string(),
            category: InterfaceCategory::Application,
            provider_domain_id: domain,
            contract_type: ContractType::OpenApi,
            notation_bindings: vec![],
        });
        dataset.relationships.push(crate::model::Relationship {
            object: object(
                relationship,
                ObjectType::Relationship,
                "Order uses Pricing",
                domain,
            ),
            source_id: order,
            target_id: pricing,
            relation_type: "SERVING".to_string(),
            notation: crate::model::Notation::Archimate,
            metamodel_version: "archimate:3.2".to_string(),
        });
        dataset.diagrams.push(Diagram {
            object: object(diagram, ObjectType::Diagram, "Chaîne de commande", domain),
            notation: DiagramNotation::Archimate,
            description: "Vue de la chaîne.".to_string(),
            publication_status: crate::model::PublicationStatus::Draft,
            published_at: None,
            layout: DiagramLayout {
                schema: 1,
                viewport: Viewport::default(),
                nodes: vec![
                    DiagramNode {
                        id: "n1".to_string(),
                        object_id: order,
                        kind: NodeKind::Element,
                        version_id: None,
                        x: 20.0,
                        y: 40.0,
                        w: 160.0,
                        h: 72.0,
                        layer_id: None,
                        zone_id: None,
                        collapsed: false,
                        port: None,
                        style_override: serde_json::Map::new(),
                    },
                    DiagramNode {
                        id: "n2".to_string(),
                        object_id: pricing,
                        kind: NodeKind::Element,
                        version_id: None,
                        x: 260.0,
                        y: 40.0,
                        w: 160.0,
                        h: 72.0,
                        layer_id: None,
                        zone_id: None,
                        collapsed: false,
                        port: None,
                        style_override: serde_json::Map::new(),
                    },
                ],
                edges: vec![DiagramEdge {
                    id: "e1".to_string(),
                    relationship_id: relationship,
                    waypoints: vec![Point { x: 180.0, y: 76.0 }, Point { x: 260.0, y: 76.0 }],
                    label_visible: true,
                    routing: Default::default(),
                    layer_id: None,
                }],
                ..Default::default()
            },
        });
        dataset.stories.push(Story {
            object: object(story, ObjectType::Story, "Parcours de commande", domain),
            structure: StoryStructure {
                chapters: vec![Chapter {
                    id: "c1".to_string(),
                    title: "Commande".to_string(),
                    blocks: vec![StoryBlock::Diagram {
                        id: "b1".to_string(),
                        diagram_id: diagram,
                        framing: Default::default(),
                    }],
                }],
                pinned_revisions: Default::default(),
                pinned_versions: Default::default(),
            },
        });
        dataset.reviews.push(Review {
            object: object(review, ObjectType::Review, "Revue Paiements", domain),
            domain_id: domain,
            filter_snapshot: json!({}),
            temporal_window_start: None,
            temporal_window_end: None,
            status: ReviewStatus::Open,
            markdown_summary: "Tout est conforme.".to_string(),
        });
        dataset
            .documentation
            .push(crate::model::DocumentationBlock {
                id: oid(9),
                owner_object_id: order,
                markdown: format!(
                    "Voir [Pricing](arch://element/{pricing}) et [absent](arch://element/{}).",
                    oid(99)
                ),
                revision: 1,
                created_at: date(),
                created_by: "u1".to_string(),
            });

        Fixture {
            dataset,
            domain,
            order,
            pricing,
            payment,
            diagram,
        }
    }

    fn files_by_path(bundle: &ExportBundle) -> HashMap<String, String> {
        bundle
            .files
            .iter()
            .map(|file| (file.path.clone(), file.content.clone()))
            .collect()
    }

    fn md_links(content: &str) -> Vec<String> {
        let mut links = Vec::new();
        let mut remaining = content;
        while let Some(index) = remaining.find("](") {
            let start = index + 2;
            let Some(end) = remaining[start..].find(')') else {
                break;
            };
            links.push(remaining[start..start + end].to_string());
            remaining = &remaining[start + end + 1..];
        }
        links
    }

    fn resolve_relative(from_path: &str, link: &str) -> String {
        let mut parts: Vec<&str> = from_path.split('/').collect();
        parts.pop();
        for segment in link.split('/') {
            match segment {
                ".." => {
                    parts.pop();
                }
                "." => {}
                other => parts.push(other),
            }
        }
        parts.join("/")
    }

    #[test]
    fn domain_export_has_index_and_all_object_pages() {
        let fixture = fixture();
        let bundle = export_domain(&fixture.dataset, fixture.domain).expect("export");
        let paths = files_by_path(&bundle);
        assert_eq!(bundle.root, "paiements");
        assert!(paths.contains_key("index.md"));
        assert!(paths.contains_key(&format!("domains/{}.md", fixture.domain)));
        assert!(paths.contains_key(&format!("elements/{}.md", fixture.order)));
        assert!(paths.contains_key(&format!("interfaces/{}.md", fixture.payment)));
        assert!(paths.contains_key(&format!("diagrams/{}.md", fixture.diagram)));
        assert!(paths.contains_key(&format!("stories/{}.md", oid(7))));
        assert!(paths.contains_key(&format!("reviews/{}.md", oid(8))));
        let index = &paths["index.md"];
        assert!(index.contains("[Order Service]"));
        assert!(index.contains("elements/"));
        assert!(index.contains("## Éléments"), "{index}");
        assert!(paths.contains_key(&format!("relationships/{}.md", oid(5))));
    }

    #[test]
    fn domain_export_is_reproducible_and_sorted() {
        let fixture = fixture();
        let first = export_domain(&fixture.dataset, fixture.domain).expect("export");
        let second = export_domain(&fixture.dataset, fixture.domain).expect("export");
        assert_eq!(first, second);
        let mut sorted = first.files.clone();
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(first.files, sorted);
        for file in &first.files {
            assert!(
                !file.content.contains("> Généré le"),
                "aucune date de génération : {}",
                file.path
            );
        }
    }

    #[test]
    fn markdown_export_has_valid_relative_links() {
        let fixture = fixture();
        let bundle = export_domain(&fixture.dataset, fixture.domain).expect("export");
        let paths: BTreeSet<&str> = bundle.files.iter().map(|file| file.path.as_str()).collect();
        for file in &bundle.files {
            if !file.path.ends_with(".md") {
                continue;
            }
            for link in md_links(&file.content) {
                assert!(
                    !link.contains("arch://"),
                    "lien arch:// généré dans {} : {link}",
                    file.path
                );
                if link.starts_with('#') || link.contains("://") {
                    continue;
                }
                let resolved = resolve_relative(&file.path, &link);
                assert!(
                    paths.contains(resolved.as_str()),
                    "lien mort dans {} : {link} → {resolved}",
                    file.path
                );
            }
        }
    }

    #[test]
    fn markdown_export_includes_svg_and_mermaid() {
        let fixture = fixture();
        let bundle = export_domain(&fixture.dataset, fixture.domain).expect("export");
        let paths = files_by_path(&bundle);
        let svg = &paths[&format!("diagrams/{}.svg", fixture.diagram)];
        let mmd = &paths[&format!("diagrams/{}.mmd", fixture.diagram)];
        assert!(svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains("viewBox="), "{svg}");
        assert!(svg.contains("<rect"), "{svg}");
        assert!(svg.contains("<line"), "{svg}");
        assert!(!svg.contains("<script"), "{svg}");
        assert!(!svg.contains("<image"), "{svg}");
        assert!(mmd.starts_with("flowchart LR"), "{mmd}");
        assert!(mmd.contains("n0[\"Order Service\"]"), "{mmd}");
        assert!(mmd.contains("-->|SERVING|"), "{mmd}");
        let page = &paths[&format!("diagrams/{}.md", fixture.diagram)];
        assert!(page.contains("```mermaid"), "{page}");
        assert!(page.contains(&format!("{}.svg", fixture.diagram)));
    }

    #[test]
    fn markdown_export_includes_neighbors_for_object() {
        let fixture = fixture();
        let bundle = export_object(&fixture.dataset, fixture.order).expect("export");
        let paths = files_by_path(&bundle);
        assert!(paths.contains_key(&format!("elements/{}.md", fixture.order)));
        assert!(
            paths.contains_key(&format!("elements/{}.md", fixture.pricing)),
            "voisin direct attendu"
        );
        let page = &paths[&format!("elements/{}.md", fixture.order)];
        assert!(page.contains(&format!("{}.md", fixture.pricing)), "{page}");
        assert!(page.contains("lien cassé"), "{page}");
    }

    #[test]
    fn unknown_domain_or_object_is_export_001() {
        let fixture = fixture();
        let missing = oid(500);
        let domain_error = export_domain(&fixture.dataset, missing).expect_err("domaine inconnu");
        assert!(domain_error.is("EXPORT-001"), "{domain_error}");
        let object_error = export_object(&fixture.dataset, missing).expect_err("objet inconnu");
        assert!(object_error.is("EXPORT-001"), "{object_error}");
    }

    #[test]
    fn domain_without_objects_still_has_valid_index() {
        let domain = oid(42);
        let mut dataset = Dataset::default();
        dataset.domains.push(Domain {
            object: object(domain, ObjectType::Domain, "Vide", domain),
            slug: "vide".to_string(),
            description: String::new(),
        });
        let bundle = export_domain(&dataset, domain).expect("export");
        let index = &files_by_path(&bundle)["index.md"];
        assert!(index.starts_with("# Vide"));
        assert!(index.contains("> Domaine `vide`"), "{index}");
    }

    #[test]
    fn archived_object_gets_a_read_only_banner() {
        let mut fixture = fixture();
        if let Some(element) = fixture
            .dataset
            .elements
            .iter_mut()
            .find(|element| element.object.id == fixture.order)
        {
            element.object.lifecycle = Lifecycle::Archived;
        }
        let bundle = export_object(&fixture.dataset, fixture.order).expect("export");
        let paths = files_by_path(&bundle);
        let page = &paths[&format!("elements/{}.md", fixture.order)];
        assert!(page.to_lowercase().contains("archiv"), "{page}");
    }

    #[test]
    fn empty_diagram_produces_no_media_and_no_dead_links() {
        let mut fixture = fixture();
        if let Some(diagram) = fixture
            .dataset
            .diagrams
            .iter_mut()
            .find(|diagram| diagram.object.id == fixture.diagram)
        {
            diagram.layout.nodes.clear();
            diagram.layout.edges.clear();
        }
        let bundle = export_domain(&fixture.dataset, fixture.domain).expect("export");
        let paths = files_by_path(&bundle);
        assert!(!paths.contains_key(&format!("diagrams/{}.svg", fixture.diagram)));
        assert!(!paths.contains_key(&format!("diagrams/{}.mmd", fixture.diagram)));
        let page = &paths[&format!("diagrams/{}.md", fixture.diagram)];
        assert!(page.contains("Diagramme vide"), "{page}");
        assert!(!page.contains(".svg"), "{page}");
    }

    #[test]
    fn invalid_layout_is_export_002() {
        let mut fixture = fixture();
        if let Some(diagram) = fixture
            .dataset
            .diagrams
            .iter_mut()
            .find(|diagram| diagram.object.id == fixture.diagram)
        {
            diagram.layout.nodes[0].w = f64::NAN;
        }
        let error = export_domain(&fixture.dataset, fixture.domain).expect_err("layout illisible");
        assert!(error.is("EXPORT-002"), "{error}");
    }
}
