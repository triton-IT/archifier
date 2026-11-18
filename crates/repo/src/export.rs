//! Export Markdown exposé par le dépôt (T1-05).
//!
//! Les wrappers ne font que prêter le `Dataset` courant au moteur
//! `archifier_core::export` : aucune I/O, aucune copie du stockage, aucun accès
//! direct depuis l'UI.

use crate::{RepoError, RepoResult, Repository};
use archifier_core::export::{export_domain, export_object, ExportBundle};
use archifier_core::ObjectId;

impl Repository {
    /// Exporte un domaine complet en bundle Markdown.
    pub fn export_domain_markdown(&self, domain_id: ObjectId) -> RepoResult<ExportBundle> {
        let inner = self.inner.borrow();
        export_domain(&inner.data, domain_id).map_err(export_error)
    }

    /// Exporte un objet et ses voisins directs en bundle Markdown.
    pub fn export_object_markdown(&self, object_id: ObjectId) -> RepoResult<ExportBundle> {
        let inner = self.inner.borrow();
        export_object(&inner.data, object_id).map_err(export_error)
    }
}

fn export_error(error: archifier_core::ExportError) -> RepoError {
    match error {
        archifier_core::ExportError::UnknownSubject => RepoError::new(
            "EXPORT-001",
            "Domaine ou objet inconnu : aucun export possible.",
        ),
        archifier_core::ExportError::UnreadableDiagram(detail) => {
            RepoError::new("EXPORT-002", format!("Diagramme illisible : {detail}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed;
    use crate::MemoryStore;
    use std::rc::Rc;

    const NOW_MS: u64 = 1_795_000_000_000;

    fn seeded() -> Repository {
        let repo = Repository::new(Rc::new(MemoryStore::new()), NOW_MS, 42);
        seed::seed_demo(&repo).expect("jeu de démonstration");
        repo
    }

    #[test]
    fn domain_export_wrapper_reuses_the_dataset() {
        let repo = seeded();
        let domain = repo
            .domains()
            .first()
            .map(|domain| domain.id)
            .expect("domaine");
        let bundle = repo
            .export_domain_markdown(domain)
            .expect("export du domaine");
        assert!(!bundle.files.is_empty());
        assert!(bundle.files.iter().any(|file| file.path == "index.md"));
    }

    #[test]
    fn object_export_wrapper_reuses_the_dataset() {
        let repo = seeded();
        let element = repo
            .all_objects()
            .into_iter()
            .find(|object| object.name == "Order Service")
            .expect("élément");
        let bundle = repo
            .export_object_markdown(element.id)
            .expect("export de l'objet");
        assert!(bundle
            .files
            .iter()
            .any(|file| file.path == format!("elements/{}.md", element.id)));
    }

    #[test]
    fn unknown_object_maps_to_export_001() {
        let repo = seeded();
        let error = repo
            .export_object_markdown(ObjectId::from_parts(1, 1))
            .expect_err("objet inconnu");
        assert!(error.is("EXPORT-001"), "{error}");
    }
}
