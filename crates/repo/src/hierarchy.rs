//! Hiérarchie parent/enfant des éléments de modèle (T1-09).
//!
//! Seuls les `ModelElement` participent à la hiérarchie. Les règles sont
//! volontairement simples et déterministes : même notation, profondeur bornée,
//! pas de cycle. Toute écriture passe par `mutate` et laisse une révision et un
//! audit, comme les autres commandes du dépôt.

use crate::{
    Hierarchy, HierarchyNode, Inner, ObjectRef, ObjectType, RepoError, RepoResult, Repository,
};
use archifier_core::{Lifecycle, ModelElement, Notation, ObjectId};
use std::collections::HashSet;

/// Nombre maximal d'arêtes depuis la racine : au-delà, `HIER-003`.
///
/// La chaîne de cinq arêtes de la fixture W1c (six éléments, profondeur 5 pour
/// le plus profond) est légale ; un septième élément serait refusé.
pub const MAX_HIERARCHY_DEPTH: usize = 5;

impl Repository {
    /// Enfants directs et ancêtres d'un élément.
    ///
    /// Les enfants sont triés par nom puis identifiant. Les ancêtres suivent la
    /// chaîne du parent direct vers la racine, afin que l'UI puisse afficher un
    /// fil d'Ariane sans reconstruire l'ordre.
    pub fn hierarchy(&self, id: ObjectId) -> RepoResult<Hierarchy> {
        let inner = self.inner.borrow();
        let element = find_element(&inner, id)?;
        let root = ObjectRef::of(ObjectType::ModelElement, id, element.object.name.clone());
        let root_archived = element.object.lifecycle != Lifecycle::Active;
        let (parent, parent_archived) = match element
            .parent_id
            .and_then(|parent_id| inner.find_element(parent_id))
        {
            Some(parent) => (
                Some(ObjectRef::of(
                    ObjectType::ModelElement,
                    parent.object.id,
                    parent.object.name.clone(),
                )),
                parent.object.lifecycle != Lifecycle::Active,
            ),
            None => (None, false),
        };

        let mut children: Vec<HierarchyNode> = inner
            .data
            .elements
            .iter()
            .filter(|candidate| candidate.parent_id == Some(id))
            .map(|candidate| HierarchyNode {
                object: ObjectRef::of(
                    ObjectType::ModelElement,
                    candidate.object.id,
                    candidate.object.name.clone(),
                ),
                parent_id: candidate.parent_id,
                depth: depth_of(&inner, candidate.object.id).min(u8::MAX as usize) as u8,
                // Les enfants listés ont tous `id` pour parent : le drapeau
                // porte donc sur le parent direct (la racine de cette vue).
                archived_parent: root_archived,
            })
            .collect();
        children.sort_by(|a, b| {
            name_key(&a.object.name)
                .cmp(&name_key(&b.object.name))
                .then_with(|| a.object.id.cmp(&b.object.id))
        });

        let mut ancestors = Vec::new();
        for ancestor_id in ancestor_ids(&inner, id) {
            if let Some(ancestor) = inner.find_element(ancestor_id) {
                ancestors.push(ObjectRef::of(
                    ObjectType::ModelElement,
                    ancestor.object.id,
                    ancestor.object.name.clone(),
                ));
            }
        }

        Ok(Hierarchy {
            root,
            root_depth: depth_of(&inner, id).min(u8::MAX as usize) as u8,
            children,
            ancestors,
            parent,
            parent_archived,
        })
    }

    /// Rattache ou détache un élément de modèle.
    ///
    /// Erreurs : `HIER-001` (type non élément), `HIER-002` (notations
    /// différentes), `HIER-003` (profondeur > 5), `HIER-004` (cycle ou
    /// auto-parenté). Un parent archivé est accepté et signalé.
    pub fn set_parent(&self, child: ObjectId, parent: Option<ObjectId>) -> RepoResult<()> {
        self.mutate(|inner| {
            inner.require_active(child)?;
            let child_element = find_element(inner, child)?;
            let child_notation = child_element.notation;
            let child_metamodel = child_element.metamodel_version.clone();
            let child_name = child_element.object.name.clone();
            ensure_hierarchizable(child_element, &child_name)?;

            let mut archived_parent = false;
            if let Some(parent_id) = parent {
                if parent_id == child {
                    return Err(RepoError::new(
                        "HIER-004",
                        "Un élément ne peut pas être son propre parent.",
                    ));
                }
                let parent_element = find_element(inner, parent_id)?;
                ensure_hierarchizable(parent_element, &parent_element.object.name)?;
                if parent_element.notation != child_notation
                    || parent_element.metamodel_version != child_metamodel
                {
                    return Err(RepoError::new(
                        "HIER-002",
                        format!(
                            "« {} » ({}) et « {} » ({}) n'utilisent pas la même notation \
                             ou le même métamodèle.",
                            child_name,
                            notation_label(child_notation),
                            parent_element.object.name,
                            notation_label(parent_element.notation)
                        ),
                    ));
                }
                if ancestor_ids(inner, parent_id).contains(&child) {
                    return Err(RepoError::new(
                        "HIER-004",
                        format!(
                            "Rattacher « {child_name} » sous « {} » créerait un cycle.",
                            parent_element.object.name
                        ),
                    ));
                }
                let new_depth = depth_of(inner, parent_id) + 1;
                let subtree = subtree_height(inner, child);
                if new_depth + subtree > MAX_HIERARCHY_DEPTH {
                    return Err(RepoError::new(
                        "HIER-003",
                        format!(
                            "La hiérarchie est limitée à {MAX_HIERARCHY_DEPTH} arêtes depuis la \
                             racine : « {child_name} » atteindrait la profondeur {}.",
                            new_depth + subtree
                        ),
                    ));
                }
                archived_parent = parent_element.object.lifecycle != Lifecycle::Active;
            }

            let now = self.now();
            let user = self.current_user();
            let element = inner
                .data
                .elements
                .iter_mut()
                .find(|element| element.object.id == child)
                .ok_or_else(|| RepoError::not_found("Élément"))?;
            element.parent_id = parent;
            element.object.revision += 1;
            element.object.updated_at = now;
            element.object.updated_by = user.clone();
            let summary = match parent {
                Some(_) if archived_parent => {
                    format!("Changement de parent de « {child_name} » (parent archivé)")
                }
                Some(_) => format!("Changement de parent de « {child_name} »"),
                None => format!("Détachement du parent de « {child_name} »"),
            };
            inner.record(child, summary, now, &user);
            Ok(())
        })
    }
}

fn find_element(inner: &Inner, id: ObjectId) -> RepoResult<&ModelElement> {
    inner.find_element(id).ok_or_else(|| {
        if inner.object_exists(id) {
            RepoError::new(
                "HIER-001",
                "Seuls les éléments de modèle participent à la hiérarchie.",
            )
        } else {
            RepoError::not_found("Élément")
        }
    })
}

/// Refuse les notations et métaclasses explicitement exclues (§7.1).
fn ensure_hierarchizable(element: &ModelElement, name: &str) -> RepoResult<()> {
    if element.notation == Notation::Native {
        return Err(RepoError::new(
            "HIER-002",
            format!("« {name} » utilise la notation native : elle n'est pas hiérarchisable."),
        ));
    }
    if element.metaclass.eq_ignore_ascii_case("grouping") {
        return Err(RepoError::new(
            "HIER-002",
            format!("« {name} » est un Grouping : il ne participe pas à la hiérarchie."),
        ));
    }
    Ok(())
}

/// Profondeur en arêtes depuis la racine : racine = 0.
fn depth_of(inner: &Inner, id: ObjectId) -> usize {
    ancestor_ids(inner, id).len()
}

/// Ancêtres du parent direct vers la racine, avec garde anti-cycle.
fn ancestor_ids(inner: &Inner, id: ObjectId) -> Vec<ObjectId> {
    let mut out = Vec::new();
    let mut seen: HashSet<ObjectId> = HashSet::new();
    seen.insert(id);
    let mut current = inner.find_element(id).and_then(|element| element.parent_id);
    while let Some(parent_id) = current {
        if !seen.insert(parent_id) {
            break;
        }
        out.push(parent_id);
        current = inner
            .find_element(parent_id)
            .and_then(|element| element.parent_id);
    }
    out
}

/// Nombre maximal d'arêtes sous `id` (0 pour une feuille).
fn subtree_height(inner: &Inner, id: ObjectId) -> usize {
    let mut best = 0usize;
    let mut seen: HashSet<ObjectId> = HashSet::new();
    seen.insert(id);
    let mut stack = vec![(id, 0usize)];
    while let Some((current, level)) = stack.pop() {
        best = best.max(level);
        for candidate in &inner.data.elements {
            if candidate.parent_id == Some(current) && seen.insert(candidate.object.id) {
                stack.push((candidate.object.id, level + 1));
            }
        }
    }
    best
}

fn name_key(name: &str) -> String {
    name.to_lowercase()
}

fn notation_label(notation: Notation) -> &'static str {
    match notation {
        Notation::Archimate => "ArchiMate",
        Notation::Bpmn => "BPMN",
        Notation::Uml => "UML",
        Notation::Native => "native",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryStore, Repository};
    use archifier_core::{Lifecycle, Notation, RepositoryObject};
    use serde_json::Map;
    use std::rc::Rc;

    fn repo_with_domain() -> (Repository, ObjectId) {
        let repo = Repository::new(Rc::new(MemoryStore::new()), 1_795_000_000_000, 7);
        let domain = repo
            .create_domain("Hiérarchie", "Domaine de test")
            .expect("domaine");
        (repo, domain)
    }

    /// Élément brut, pour tester les notations/métaclasses que l'API de
    /// création valide par ailleurs (native, Grouping, métamodèle différent).
    fn raw_element(
        repo: &Repository,
        domain: ObjectId,
        notation: Notation,
        metamodel_version: &str,
        metaclass: &str,
        name: &str,
    ) -> ObjectId {
        let id = repo.new_id();
        let now = repo.now();
        repo.inner.borrow_mut().data.elements.push(ModelElement {
            object: RepositoryObject {
                id,
                object_type: ObjectType::ModelElement,
                name: name.into(),
                responsible_domain_id: domain,
                lifecycle: Lifecycle::Active,
                revision: 1,
                created_at: now,
                created_by: "u1".into(),
                updated_at: now,
                updated_by: "u1".into(),
            },
            notation,
            metamodel_version: metamodel_version.into(),
            metaclass: metaclass.into(),
            parent_id: None,
            semantic_properties: Map::new(),
        });
        id
    }

    fn element(
        repo: &Repository,
        domain: ObjectId,
        notation: Notation,
        class_key: &str,
        name: &str,
    ) -> ObjectId {
        repo.create_element(domain, notation, class_key, name, Map::new())
            .expect("élément")
    }

    #[test]
    fn hierarchy_depth_is_limited_to_five() {
        let (repo, domain) = repo_with_domain();
        // Six éléments, cinq arêtes, cinq ancêtres pour le plus profond.
        let chain: Vec<ObjectId> = (0..6)
            .map(|index| {
                element(
                    &repo,
                    domain,
                    Notation::Archimate,
                    "application-component",
                    &format!("Niveau {index}"),
                )
            })
            .collect();
        for index in 1..chain.len() {
            repo.set_parent(chain[index], Some(chain[index - 1]))
                .expect("rattachement dans la limite");
        }
        assert_eq!(repo.hierarchy(chain[5]).expect("vue").ancestors.len(), 5);
        assert_eq!(repo.hierarchy(chain[5]).expect("vue").root_depth, 5);
        assert_eq!(repo.hierarchy(chain[0]).expect("vue").root_depth, 0);

        let seventh = element(
            &repo,
            domain,
            Notation::Archimate,
            "application-component",
            "Niveau 6",
        );
        let error = repo
            .set_parent(seventh, Some(chain[5]))
            .expect_err("profondeur refusée");
        assert!(error.is("HIER-003"), "{error}");
        assert!(repo.hierarchy(chain[5]).expect("vue").children.is_empty());
    }

    #[test]
    fn hierarchy_cycle_is_refused() {
        let (repo, domain) = repo_with_domain();
        let a = element(&repo, domain, Notation::Bpmn, "service-task", "A");
        let b = element(&repo, domain, Notation::Bpmn, "service-task", "B");
        let c = element(&repo, domain, Notation::Bpmn, "service-task", "C");
        repo.set_parent(b, Some(a)).expect("b sous a");
        repo.set_parent(c, Some(b)).expect("c sous b");

        let cycle = repo.set_parent(a, Some(c)).expect_err("cycle refusé");
        assert!(cycle.is("HIER-004"), "{cycle}");

        let auto = repo
            .set_parent(a, Some(a))
            .expect_err("auto-parenté refusé");
        assert!(auto.is("HIER-004"), "{auto}");
        assert!(repo.element(a).expect("élément").parent_id.is_none());
    }

    #[test]
    fn archived_parent_is_reported() {
        let (repo, domain) = repo_with_domain();
        let parent = element(
            &repo,
            domain,
            Notation::Uml,
            "component",
            "Composant parent",
        );
        let child = element(
            &repo,
            domain,
            Notation::Uml,
            "component",
            "Composant enfant",
        );
        repo.set_parent(child, Some(parent)).expect("rattachement");
        // Un parent avec enfant actif ne peut pas être archivé (§7.2).
        let blocked = repo
            .archive(parent)
            .expect_err("archivage bloqué par l'enfant actif");
        assert!(blocked.is("HIER-005"), "{blocked}");
        repo.archive(child).expect("archivage de l'enfant");
        repo.archive(parent).expect("archivage du parent");
        repo.restore(child).expect("restauration de l'enfant");

        let view = repo.hierarchy(parent).expect("vue");
        let node = view
            .children
            .iter()
            .find(|node| node.object.id == child)
            .expect("enfant listé");
        assert!(node.archived_parent);

        // La fiche de l'enfant expose aussi le parent archivé.
        let from_child = repo.hierarchy(child).expect("vue enfant");
        assert_eq!(
            from_child.parent.as_ref().map(|parent| parent.id),
            Some(parent)
        );
        assert!(from_child.parent_archived);
        assert_eq!(from_child.ancestors.len(), 1);

        // Un parent archivé reste accepté pour un nouvel enfant.
        let sibling = element(
            &repo,
            domain,
            Notation::Uml,
            "component",
            "Composant tardif",
        );
        repo.set_parent(sibling, Some(parent))
            .expect("parent archivé accepté");
        let view = repo.hierarchy(parent).expect("vue");
        assert_eq!(view.children.len(), 2);
        assert!(view.children.iter().all(|node| node.archived_parent));
    }

    #[test]
    fn hierarchy_requires_same_notation() {
        let (repo, domain) = repo_with_domain();
        let archimate = element(
            &repo,
            domain,
            Notation::Archimate,
            "application-component",
            "Composant ArchiMate",
        );
        let bpmn = element(&repo, domain, Notation::Bpmn, "service-task", "Tâche BPMN");
        let error = repo
            .set_parent(archimate, Some(bpmn))
            .expect_err("notations différentes");
        assert!(error.is("HIER-002"), "{error}");
    }

    #[test]
    fn set_parent_records_revision_and_audit() {
        let (repo, domain) = repo_with_domain();
        let parent = element(
            &repo,
            domain,
            Notation::Archimate,
            "application-component",
            "Parent",
        );
        let child = element(
            &repo,
            domain,
            Notation::Archimate,
            "application-component",
            "Enfant",
        );
        let before = repo.element(child).expect("élément").object.revision;
        repo.set_parent(child, Some(parent)).expect("rattachement");
        let after = repo.element(child).expect("élément").object.revision;
        assert_eq!(after, before + 1);
        assert!(repo
            .history(child)
            .iter()
            .any(|entry| entry.revision == after && entry.summary.contains("parent")));
    }

    #[test]
    fn children_are_sorted_by_name() {
        let (repo, domain) = repo_with_domain();
        let parent = element(
            &repo,
            domain,
            Notation::Archimate,
            "application-component",
            "Parent",
        );
        for name in ["Zêta", "alpha", "Bêta"] {
            let child = element(
                &repo,
                domain,
                Notation::Archimate,
                "application-component",
                name,
            );
            repo.set_parent(child, Some(parent)).expect("rattachement");
        }
        let names: Vec<String> = repo
            .hierarchy(parent)
            .expect("vue")
            .children
            .into_iter()
            .map(|node| node.object.name)
            .collect();
        assert_eq!(names, vec!["alpha", "Bêta", "Zêta"]);
    }

    #[test]
    fn native_and_grouping_are_not_hierarchizable() {
        let (repo, domain) = repo_with_domain();
        let native = raw_element(
            &repo,
            domain,
            Notation::Native,
            "native:1.0",
            "Native",
            "Relation native",
        );
        let archimate = element(
            &repo,
            domain,
            Notation::Archimate,
            "application-component",
            "Composant",
        );
        let grouping = raw_element(
            &repo,
            domain,
            Notation::Archimate,
            "archimate:3.2",
            "Grouping",
            "Regroupement",
        );

        let error = repo
            .set_parent(native, Some(archimate))
            .expect_err("notation native refusée");
        assert!(error.is("HIER-002"), "{error}");
        let error = repo
            .set_parent(archimate, Some(native))
            .expect_err("parent natif refusé");
        assert!(error.is("HIER-002"), "{error}");

        let error = repo
            .set_parent(grouping, Some(archimate))
            .expect_err("Grouping refusé");
        assert!(error.is("HIER-002"), "{error}");
        let error = repo
            .set_parent(archimate, Some(grouping))
            .expect_err("Grouping parent refusé");
        assert!(error.is("HIER-002"), "{error}");
    }

    #[test]
    fn different_metamodel_version_is_refused() {
        let (repo, domain) = repo_with_domain();
        let parent = raw_element(
            &repo,
            domain,
            Notation::Archimate,
            "archimate:3.1",
            "ApplicationComponent",
            "Parent 3.1",
        );
        let child = element(
            &repo,
            domain,
            Notation::Archimate,
            "application-component",
            "Enfant 3.2",
        );
        let error = repo
            .set_parent(child, Some(parent))
            .expect_err("métamodèles différents");
        assert!(error.is("HIER-002"), "{error}");
    }
}
