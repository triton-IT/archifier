//! Tests bloquants W1c-1 — hiérarchie parent/enfant (T1-09).
//!
//! Contrat : `docs/w1c-contract.md` §1 et §4. Fixture :
//! `fixtures/mvp2/w1c/hierarchy.json`.
//!
//! La fixture porte une chaîne BPMN de cinq arêtes
//! (`Entreprise → Couloir A → Traiter commande → Calculer tarif → Encaisser
//! → Notifier`) et une tâche libre (`Archiver commande`) qui servirait de
//! sixième niveau. Elle contient aussi un participant archivé, un composant
//! ArchiMate, un composant UML avec son port et un diagramme non hiérarchisable.
//!
//! Aucune écriture de production n'est faite ici : les tests n'utilisent que
//! l'API publique `hierarchy` / `set_parent` et les lectures existantes.

use archifier_core::{Notation, ObjectId, ObjectType};
use archifier_repo::{MemoryStore, Repository};
use std::rc::Rc;

const NOW_MS: u64 = 1_795_000_000_000; // 2026-11-18, horloge figée
const DATASET: &str = include_str!("../../../fixtures/mvp2/w1c/hierarchy.json");

const DOMAIN: &str = "01K8ZP4N000000000000000101";
/// Participant racine de la chaîne de cinq niveaux.
const ROOT: &str = "01K8ZP4N000000000000000201";
/// Couloir enfant direct, premier dans l'ordre (nom puis id).
const LANE_A: &str = "01K8ZP4N000000000000000202";
/// Second couloir de même nom : vérifie le départage par identifiant.
const LANE_A2: &str = "01K8ZP4N000000000000000203";
const LANE_B: &str = "01K8ZP4N000000000000000204";
const TASK_1: &str = "01K8ZP4N000000000000000205";
const TASK_2: &str = "01K8ZP4N000000000000000206";
const TASK_3: &str = "01K8ZP4N000000000000000207";
/// Tâche la plus profonde de la fixture : cinq ancêtres.
const DEEP_TASK: &str = "01K8ZP4N000000000000000208";
/// Tâche libre : un parent supplémentaire dépasserait la profondeur 5.
const FREE_TASK: &str = "01K8ZP4N000000000000000209";
const ARCHIVED_PARENT: &str = "01K8ZP4N00000000000000020A";
const ARCHIVED_CHILD: &str = "01K8ZP4N00000000000000020B";
const ARCHIMATE_ELEMENT: &str = "01K8ZP4N00000000000000020C";
const UML_COMPONENT: &str = "01K8ZP4N00000000000000020D";
const UML_PORT: &str = "01K8ZP4N00000000000000020E";
const DIAGRAM: &str = "01K8ZP4N00000000000000020F";
/// Participant à enfant unique : scénario archiver enfant → parent → restaurer.
const SOLO_PARENT: &str = "01K8ZP4N000000000000000210";
const SOLO_CHILD: &str = "01K8ZP4N000000000000000211";
const NATIVE_PARENT: &str = "01K8ZP4N000000000000000212";
const NATIVE_CHILD: &str = "01K8ZP4N000000000000000213";
const ARCHIMATE_GROUPING: &str = "01K8ZP4N000000000000000214";

fn fixture() -> Repository {
    Repository::from_json(Rc::new(MemoryStore::new()), DATASET, NOW_MS, 42)
        .expect("fixture W1c hiérarchie lisible")
}

fn id(raw: &str) -> ObjectId {
    raw.parse().expect("ULID de fixture valide")
}

fn bpmn_task(repo: &Repository, name: &str) -> ObjectId {
    repo.create_element(id(DOMAIN), Notation::Bpmn, "task", name, Default::default())
        .expect("tâche BPMN créée")
}

fn ancestor_ids(hierarchy: &archifier_repo::Hierarchy) -> Vec<ObjectId> {
    hierarchy.ancestors.iter().map(|a| a.id).collect()
}

#[test]
fn hierarchy_depth_is_limited_to_five() {
    let repo = fixture();
    let deepest = id(DEEP_TASK);

    // Cinq ancêtres sont acceptés et relus par `hierarchy`.
    let hierarchy = repo
        .hierarchy(deepest)
        .expect("hiérarchie de la tâche profonde");
    assert_eq!(
        hierarchy.ancestors.len(),
        5,
        "cinq niveaux au-dessus de la tâche la plus profonde : {:?}",
        ancestor_ids(&hierarchy)
    );
    let expected = [ROOT, LANE_A, TASK_1, TASK_2, TASK_3];
    let actual = ancestor_ids(&hierarchy);
    for raw in expected {
        assert!(
            actual.contains(&id(raw)),
            "ancêtre {raw} absent de {actual:?}"
        );
    }
    assert!(hierarchy
        .ancestors
        .iter()
        .all(|a| a.object_type == ObjectType::ModelElement));

    // Un sixième niveau est refusé et n'écrit rien.
    let snapshot = repo.export_json();
    let error = repo
        .set_parent(id(FREE_TASK), Some(deepest))
        .expect_err("profondeur 6 refusée");
    assert!(error.is("HIER-003"), "{error}");
    assert_eq!(repo.export_json(), snapshot, "refus sans écriture");
    assert_eq!(
        repo.element(id(FREE_TASK)).expect("tâche libre").parent_id,
        None,
        "le parent refusé n'est pas mémorisé"
    );

    // Profondeur = nombre d'arêtes : la racine vaut 0, ses enfants directs 1.
    let root_view = repo.hierarchy(id(ROOT)).expect("racine");
    assert!(
        root_view.ancestors.is_empty(),
        "la racine n'a pas d'ancêtre"
    );
    assert!(root_view.parent.is_none());
    assert!(!root_view.parent_archived);
    let child_depths: Vec<u8> = root_view.children.iter().map(|child| child.depth).collect();
    assert_eq!(
        child_depths,
        vec![1, 1, 1],
        "racine depth = 0, enfants directs depth = 1"
    );

    // Le nœud le plus profond a exactement cinq arêtes depuis la racine.
    let deep_view = repo.hierarchy(id(TASK_3)).expect("parent du nœud profond");
    let deep_node = deep_view
        .children
        .iter()
        .find(|node| node.object.id == deepest)
        .expect("nœud profond listé comme enfant direct");
    assert_eq!(deep_node.depth, 5, "cinq arêtes depuis la racine");
    assert_eq!(deep_node.parent_id, Some(id(TASK_3)));
}

#[test]
fn hierarchy_cycle_is_refused() {
    let repo = fixture();
    let snapshot = repo.export_json();

    // Fermer la chaîne sur elle-même : la racine deviendrait son propre descendant.
    let error = repo
        .set_parent(id(ROOT), Some(id(DEEP_TASK)))
        .expect_err("cycle refusé");
    assert!(error.is("HIER-004"), "{error}");

    // Cycle court : Encaisser → Notifier existe déjà, l'inverser crée un cycle.
    let error = repo
        .set_parent(id(TASK_3), Some(id(DEEP_TASK)))
        .expect_err("cycle court refusé");
    assert!(error.is("HIER-004"), "{error}");

    assert_eq!(repo.export_json(), snapshot, "aucune écriture partielle");
    assert_eq!(
        repo.element(id(ROOT)).expect("racine").parent_id,
        None,
        "le parent de la racine reste inchangé"
    );
}

#[test]
fn archived_parent_is_reported() {
    let repo = fixture();

    // Fiche d'un enfant feuille : le parent archivé est visible directement.
    let leaf = repo
        .hierarchy(id(ARCHIVED_CHILD))
        .expect("hiérarchie du couloir");
    assert!(leaf.children.is_empty(), "l'enfant est bien une feuille");
    assert_eq!(
        leaf.parent.as_ref().map(|parent| parent.id),
        Some(id(ARCHIVED_PARENT)),
        "parent direct exposé sur la fiche enfant"
    );
    assert!(
        leaf.parent_archived,
        "parent_archived doit allumer le badge depuis la fiche enfant feuille"
    );
    assert!(
        ancestor_ids(&leaf).contains(&id(ARCHIVED_PARENT)),
        "parent archivé absent des ancêtres : {:?}",
        ancestor_ids(&leaf)
    );

    // Vue depuis le parent archivé : l'enfant direct porte aussi le drapeau.
    let hierarchy = repo
        .hierarchy(id(ARCHIVED_PARENT))
        .expect("hiérarchie du participant archivé");
    let child = hierarchy
        .children
        .iter()
        .find(|node| node.object.id == id(ARCHIVED_CHILD))
        .expect("couloir enfant présent");
    assert_eq!(child.parent_id, Some(id(ARCHIVED_PARENT)));
    assert!(
        child.archived_parent,
        "archived_parent doit signaler le parent archivé"
    );

    // Un parent archivé reste acceptable pour un nouvel enfant.
    let fresh = bpmn_task(&repo, "Tâche partenaire");
    repo.set_parent(fresh, Some(id(ARCHIVED_PARENT)))
        .expect("parent archivé accepté");
    let refreshed = repo
        .hierarchy(id(ARCHIVED_PARENT))
        .expect("hiérarchie après rattachement");
    let node = refreshed
        .children
        .iter()
        .find(|node| node.object.id == fresh)
        .expect("nouvel enfant listé");
    assert!(node.archived_parent, "le drapeau suit le parent archivé");
}

#[test]
fn archive_parent_with_active_child_is_hier_005() {
    let repo = fixture();

    // Le parent a un enfant actif : archivage refusé.
    let error = repo
        .archive(id(SOLO_PARENT))
        .expect_err("parent avec enfant actif");
    assert!(error.is("HIER-005"), "{error}");
    assert!(
        repo.element(id(SOLO_PARENT))
            .expect("parent")
            .object
            .is_active(),
        "le parent reste actif"
    );
    assert!(
        repo.element(id(SOLO_CHILD))
            .expect("enfant")
            .object
            .is_active(),
        "l'enfant reste actif"
    );

    // Scénario imposé : archiver l'enfant, puis le parent, puis restaurer l'enfant.
    repo.archive(id(SOLO_CHILD)).expect("archivage de l'enfant");
    repo.archive(id(SOLO_PARENT)).expect("archivage du parent");
    let leaf = repo.hierarchy(id(SOLO_CHILD)).expect("fiche enfant");
    assert!(leaf.children.is_empty());
    assert_eq!(
        leaf.parent.as_ref().map(|parent| parent.id),
        Some(id(SOLO_PARENT))
    );
    assert!(
        leaf.parent_archived,
        "le badge reste visible depuis l'enfant feuille"
    );

    repo.restore(id(SOLO_CHILD))
        .expect("restauration de l'enfant");
    let restored = repo.element(id(SOLO_CHILD)).expect("enfant restauré");
    assert!(restored.object.is_active());
    let leaf = repo
        .hierarchy(id(SOLO_CHILD))
        .expect("fiche après restauration");
    assert!(
        leaf.parent_archived,
        "restaurer l'enfant ne restaure pas le parent"
    );
}

#[test]
fn hierarchy_excludes_native_and_grouping() {
    let repo = fixture();

    // Notation Native : jamais hiérarchisable, même entre éléments natifs.
    let error = repo
        .set_parent(id(NATIVE_CHILD), Some(id(NATIVE_PARENT)))
        .expect_err("Native non hiérarchisable");
    assert!(error.is("HIER-002"), "{error}");

    // Grouping ArchiMate exclu malgré une notation identique.
    let error = repo
        .set_parent(id(ARCHIMATE_GROUPING), Some(id(ARCHIMATE_ELEMENT)))
        .expect_err("Grouping exclu");
    assert!(error.is("HIER-002"), "{error}");
}

#[test]
fn hierarchy_requires_same_notation() {
    let repo = fixture();

    // Enfant ArchiMate sous un participant BPMN.
    let error = repo
        .set_parent(id(ARCHIMATE_ELEMENT), Some(id(ROOT)))
        .expect_err("notations différentes refusées");
    assert!(error.is("HIER-002"), "{error}");

    // Port UML sous un participant BPMN.
    let error = repo
        .set_parent(id(UML_PORT), Some(id(ROOT)))
        .expect_err("notations différentes refusées");
    assert!(error.is("HIER-002"), "{error}");

    // Un diagramme n'est pas un ModelElement : il ne peut pas avoir de parent.
    let error = repo
        .set_parent(id(DIAGRAM), Some(id(ROOT)))
        .expect_err("type non hiérarchisable refusé");
    assert!(error.is("HIER-001"), "{error}");

    // Un diagramme n'est pas non plus un parent acceptable.
    let error = repo
        .set_parent(id(FREE_TASK), Some(id(DIAGRAM)))
        .expect_err("parent non hiérarchisable refusé");
    assert!(error.is("HIER-001"), "{error}");

    // Même notation : BPMN entre eux, UML entre eux.
    repo.set_parent(id(FREE_TASK), Some(id(LANE_B)))
        .expect("couloir BPMN sous participant BPMN");
    repo.set_parent(id(UML_PORT), Some(id(UML_COMPONENT)))
        .expect("port UML sous composant UML");
    assert_eq!(
        repo.element(id(UML_PORT)).expect("port").parent_id,
        Some(id(UML_COMPONENT))
    );
}

#[test]
fn hierarchy_lists_direct_children_sorted_by_name_then_id() {
    let repo = fixture();
    let hierarchy = repo.hierarchy(id(ROOT)).expect("racine BPMN");

    let ids: Vec<ObjectId> = hierarchy.children.iter().map(|c| c.object.id).collect();
    assert_eq!(
        ids,
        vec![id(LANE_A), id(LANE_A2), id(LANE_B)],
        "nom puis identifiant, enfants directs seulement"
    );
    let names: Vec<&str> = hierarchy
        .children
        .iter()
        .map(|c| c.object.name.as_str())
        .collect();
    assert_eq!(names, vec!["Couloir A", "Couloir A", "Couloir B"]);
    assert!(hierarchy
        .children
        .iter()
        .all(|child| child.parent_id == Some(id(ROOT))));
    assert_eq!(hierarchy.children[0].depth, hierarchy.children[1].depth);
}

#[test]
fn hierarchy_self_parent_is_refused() {
    let repo = fixture();
    let snapshot = repo.export_json();

    for object in [ROOT, LANE_A, DEEP_TASK] {
        let error = repo
            .set_parent(id(object), Some(id(object)))
            .expect_err("auto-parenté refusé");
        assert!(error.is("HIER-004"), "{error}");
        assert_eq!(
            repo.element(id(object)).expect("élément").parent_id,
            if object == ROOT {
                None
            } else {
                Some(id(if object == LANE_A { ROOT } else { TASK_3 }))
            }
        );
    }
    assert_eq!(repo.export_json(), snapshot);
}

#[test]
fn hierarchy_set_parent_is_versioned() {
    let repo = fixture();
    let before = repo.element(id(FREE_TASK)).expect("tâche libre");
    assert_eq!(before.parent_id, None);
    let revisions_before = repo.revisions(id(FREE_TASK)).len();
    let history_before = repo.history(id(FREE_TASK)).len();

    repo.set_parent(id(FREE_TASK), Some(id(LANE_B)))
        .expect("rattachement accepté");

    let after = repo.element(id(FREE_TASK)).expect("tâche rattachée");
    assert_eq!(after.parent_id, Some(id(LANE_B)));
    assert_eq!(after.object.revision, before.object.revision + 1);
    assert_eq!(
        after.object.updated_by,
        repo.current_user(),
        "updated_by suit l'utilisateur courant"
    );
    assert!(
        repo.history(id(FREE_TASK)).len() > history_before,
        "un audit est créé"
    );
    assert!(
        repo.revisions(id(FREE_TASK)).len() > revisions_before,
        "une révision est créée"
    );
    assert!(repo
        .revisions(id(FREE_TASK))
        .iter()
        .any(|(revision, _)| *revision == after.object.revision));
}
