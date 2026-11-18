//! Diagnostics de documentation : liens cassés et archivés (T1-06, FR-027).
//!
//! Le diagnostic est en lecture seule : aucun lien n'est corrigé ni supprimé.
//! Le statut persisté dans `Dataset::links` n'est qu'un cache ; il est recalculé
//! à la lecture depuis le `lifecycle` courant de la cible.

use crate::types::{BrokenLink, ObjectRef};
use crate::{Inner, Repository};
use archifier_core::{ArchKind, ArchUri, Link, LinkStatus, ObjectId, ObjectType};
use std::collections::BTreeSet;

impl Repository {
    /// Liste les liens `BROKEN` et `ARCHIVED`, dédoublonnés par
    /// `(source_object_id, target_uri)` et triés par `(source.id, target_uri, link.id)`.
    pub fn broken_links(&self) -> Vec<BrokenLink> {
        let inner = self.inner.borrow();
        let mut seen: BTreeSet<(ObjectId, String)> = BTreeSet::new();
        let mut diagnostics = Vec::new();

        for stored in &inner.data.links {
            let key = (stored.source_object_id, stored.target_uri.clone());
            if !seen.insert(key) {
                continue;
            }
            let resolved = resolve_target(&inner, stored);
            let status = match resolved {
                None => LinkStatus::Broken,
                Some(target_id) if inner.is_active(target_id) => LinkStatus::Ok,
                Some(_) => LinkStatus::Archived,
            };
            if status == LinkStatus::Ok {
                continue;
            }
            diagnostics.push(diagnostic(&inner, stored, status, resolved));
        }

        diagnostics.sort_by(|a, b| {
            (a.source.id, a.link.target_uri.as_str(), a.link.id).cmp(&(
                b.source.id,
                b.link.target_uri.as_str(),
                b.link.id,
            ))
        });
        diagnostics
    }
}

fn diagnostic(
    inner: &Inner,
    stored: &Link,
    status: LinkStatus,
    target_id: Option<ObjectId>,
) -> BrokenLink {
    let source_object = inner.object_by_id(stored.source_object_id);
    let source = source_object
        .as_ref()
        .map(|object| ObjectRef::of(object.object_type, object.id, object.name.clone()))
        .unwrap_or_else(|| {
            ObjectRef::of(
                ObjectType::ModelElement,
                stored.source_object_id,
                stored.source_object_id.to_string(),
            )
        });
    let owner = source_object.as_ref().and_then(|object| {
        inner
            .find_domain(object.responsible_domain_id)
            .map(|domain| {
                ObjectRef::of(
                    ObjectType::Domain,
                    domain.object.id,
                    domain.object.name.clone(),
                )
            })
    });
    let target = target_id
        .and_then(|id| inner.object_by_id(id))
        .map(|object| ObjectRef::of(object.object_type, object.id, object.name));
    let documentation = inner
        .index
        .documentation_by_object
        .get(&stored.source_object_id)
        .cloned()
        .unwrap_or_default();

    let mut link = stored.clone();
    link.status = status;

    BrokenLink {
        code: BrokenLink::code_for(status).to_string(),
        link,
        source,
        owner,
        target,
        context: context_line(&documentation, &stored.target_uri),
    }
}

/// Retrouve l'objet visé : identifiant direct d'abord, puis URI `arch://`.
/// Une URI de version résout vers son interface.
fn resolve_target(inner: &Inner, link: &Link) -> Option<ObjectId> {
    if let Some(id) = link.target_object_id {
        if inner.object_exists(id) {
            return Some(id);
        }
    }
    let uri: ArchUri = link.target_uri.parse().ok()?;
    match uri.kind {
        ArchKind::Version => inner
            .data
            .interface_versions
            .iter()
            .find(|version| version.id == uri.id)
            .map(|version| version.interface_id)
            .filter(|id| inner.object_exists(*id)),
        _ => inner.object_exists(uri.id).then_some(uri.id),
    }
}

/// Extrait 80 caractères avant et 80 après l'occurrence de `target_uri` sur la
/// première ligne qui la contient. Les bords sont marqués par `…` s'ils sont
/// rognés ; la découpe respecte les frontières UTF-8.
fn context_line(markdown: &str, target_uri: &str) -> String {
    const MARGIN: usize = 80;
    let needle_id = target_uri.rsplit('/').next().unwrap_or(target_uri);

    for line in markdown.lines() {
        if let Some(byte) = line.find(target_uri) {
            return window_around(line, byte, target_uri, MARGIN);
        }
    }
    for line in markdown.lines() {
        if let Some(byte) = line.find(needle_id) {
            return window_around(line, byte, needle_id, MARGIN);
        }
    }
    String::new()
}

fn window_around(line: &str, start_byte: usize, needle: &str, margin: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    let start = line[..start_byte].chars().count();
    let end = start + needle.chars().count();
    let from = start.saturating_sub(margin);
    let to = (end + margin).min(chars.len());

    let mut out = String::new();
    if from > 0 {
        out.push('…');
    }
    out.extend(chars[from..to].iter());
    if to < chars.len() {
        out.push('…');
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed;
    use crate::{MemoryStore, Repository};
    use std::rc::Rc;

    const NOW_MS: u64 = 1_795_000_000_000;

    fn seeded() -> Repository {
        let repo = Repository::new(Rc::new(MemoryStore::new()), NOW_MS, 42);
        seed::seed_demo(&repo).expect("jeu de démonstration");
        repo
    }

    fn id_by_name(repo: &Repository, name: &str) -> ObjectId {
        repo.all_objects()
            .into_iter()
            .find(|object| object.name == name)
            .map(|object| object.id)
            .unwrap_or_else(|| panic!("objet {name} absent"))
    }

    #[test]
    fn broken_link_diagnostic_has_context_and_owner() {
        let repo = seeded();
        let diagnostics = repo.broken_links();
        let broken = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.link.status == LinkStatus::Broken)
            .expect("lien cassé du jeu de démonstration attendu");
        assert_eq!(broken.source.name, "Payment API");
        let owner = broken.owner.as_ref().expect("domaine propriétaire");
        assert_eq!(owner.object_type, ObjectType::Domain);
        assert_eq!(owner.name, "Paiements");
        assert!(broken.target.is_none());
        assert!(!broken.context.is_empty(), "contexte documentaire attendu");
        assert!(
            broken.context.contains("arch://diagram/"),
            "{}",
            broken.context
        );
        assert_eq!(broken.code, "DOC-LINK-001");
    }

    #[test]
    fn broken_link_diagnostic_includes_archived() {
        let repo = seeded();
        let pricing = id_by_name(&repo, "Pricing Service");
        repo.archive(pricing).expect("archivage");
        let diagnostics = repo.broken_links();
        let archived = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.link.status == LinkStatus::Archived)
            .expect("lien archivé attendu");
        assert_eq!(
            archived.target.as_ref().map(|target| target.id),
            Some(pricing)
        );
        assert!(
            archived.context.contains("Pricing Service"),
            "{}",
            archived.context
        );
        assert_eq!(archived.code, "DOC-LINK-002");
    }

    #[test]
    fn broken_links_are_deduplicated_sorted_and_read_only() {
        let repo = seeded();
        let before = repo.stats().links;
        let diagnostics = repo.broken_links();
        let keys: BTreeSet<(ObjectId, String)> = diagnostics
            .iter()
            .map(|entry| (entry.source.id, entry.link.target_uri.clone()))
            .collect();
        assert_eq!(keys.len(), diagnostics.len(), "dédoublonné");
        let mut sorted = diagnostics.clone();
        sorted.sort_by(|a, b| {
            (a.source.id, a.link.target_uri.as_str(), a.link.id).cmp(&(
                b.source.id,
                b.link.target_uri.as_str(),
                b.link.id,
            ))
        });
        assert_eq!(diagnostics, sorted);
        assert_eq!(
            repo.stats().links,
            before,
            "aucun lien ne doit être modifié"
        );
        assert_eq!(repo.stats().broken_links, diagnostics.len());
    }

    #[test]
    fn context_is_a_window_around_the_uri() {
        let uri = "arch://element/01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let line = format!("{}{uri}{}", "x".repeat(200), "y".repeat(200));
        let context = context_line(&line, uri);
        assert!(context.starts_with('…'), "{context}");
        assert!(context.ends_with('…'), "{context}");
        assert!(context.contains(uri), "{context}");
        assert_eq!(context.chars().filter(|c| *c == 'x').count(), 80);
        assert_eq!(context.chars().filter(|c| *c == 'y').count(), 80);
    }
}
