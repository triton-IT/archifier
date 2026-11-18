//! Tests bloquants W1b — diff de révisions (T1-04).
//!
//! Contrat : `docs/w1b-contract.md` §1.1, §5.3 et §6. Fixture :
//! `fixtures/mvp2/export/dataset.json`, objet `Order Service v2` dont les
//! snapshots de révision 1 et 2 sont figés dans le jeu.
//!
//! Le payload comparé est le snapshot de l'objet (sans documentation) ; les
//! champs volatils `/object/revision` et `/object/updated_at` sont exclus du
//! diff et affichés à part (§5.3).
//!
//! Aucun accès au stockage : seule l'API publique `Repository::revision_diff`
//! est utilisée. `archifier_core::Revision` vaut `u64`.

use archifier_core::ObjectId;
use archifier_repo::{MemoryStore, Repository};
use serde_json::json;
use std::rc::Rc;

const NOW_MS: u64 = 1_795_000_000_000; // horloge figée
const DATASET: &str = include_str!("../../../fixtures/mvp2/export/dataset.json");

/// `Order Service v2`, objet à deux révisions dans la fixture.
const ORDER: &str = "01K8ZP4N000000000000000002";
/// ULID bien formé mais absent du référentiel.
const MISSING: &str = "01K8ZP4N00000000000000000C";

fn fixture() -> Repository {
    Repository::from_json(Rc::new(MemoryStore::new()), DATASET, NOW_MS, 42)
        .expect("fixture W1b lisible")
}

fn id(raw: &str) -> ObjectId {
    raw.parse().expect("ULID de fixture valide")
}

#[test]
fn revision_diff_is_field_by_field() {
    let repo = fixture();
    let object = id(ORDER);
    let diff = repo.revision_diff(object, 1, 2).expect("diff 1 → 2");

    assert_eq!(diff.object_id, object);
    assert_eq!(diff.from, 1);
    assert_eq!(diff.to, 2);

    // Un champ modifié = une entrée ; aucune entrée au niveau d'un conteneur.
    let paths: Vec<&str> = diff.changes.iter().map(|c| c.path.as_str()).collect();
    // `/object/revision` et `/object/updated_at` sont exclus (§5.3) ; les autres
    // métadonnées éventuelles sont tolérées ici et traitées par le test dédié.
    let content_paths: Vec<&str> = paths
        .iter()
        .copied()
        .filter(|path| {
            !matches!(
                *path,
                "/object/revision"
                    | "/object/updated_at"
                    | "/object/updated_by"
                    | "/object/created_at"
                    | "/object/created_by"
            )
        })
        .collect();
    assert_eq!(
        content_paths,
        vec![
            "/object/name",
            "/semantic_properties/owner",
            "/semantic_properties/tier",
        ],
        "un chemin JSON Pointer (RFC 6901) par champ de contenu modifié"
    );
    assert!(
        !paths.contains(&"/object"),
        "diff au niveau du conteneur objet"
    );
    assert!(
        !paths.contains(&"/semantic_properties"),
        "diff au niveau du conteneur des propriétés"
    );

    let by = |path: &str| {
        diff.changes
            .iter()
            .find(|c| c.path == path)
            .unwrap_or_else(|| panic!("champ {path} attendu dans le diff"))
    };

    assert_eq!(by("/object/name").before, Some(json!("Order Service")));
    assert_eq!(by("/object/name").after, Some(json!("Order Service v2")));

    // Un champ imbriqué est comparé à la feuille, pas au conteneur.
    assert_eq!(
        by("/semantic_properties/owner").before,
        Some(json!("Régis"))
    );
    assert_eq!(by("/semantic_properties/owner").after, Some(json!("Marie")));

    // Champ ajouté : before absent, after renseigné.
    assert_eq!(by("/semantic_properties/tier").before, None);
    assert_eq!(by("/semantic_properties/tier").after, Some(json!("1")));

    // Champ inchangé : absent du diff.
    assert!(
        !diff
            .changes
            .iter()
            .any(|c| c.path == "/semantic_properties/criticality"),
        "un champ identique ne produit aucune entrée"
    );

    // Chaque entrée porte avant/après réellement distincts.
    for change in &diff.changes {
        assert_ne!(
            change.before, change.after,
            "{} est sans changement",
            change.path
        );
        assert!(
            change.path.starts_with('/'),
            "chemin JSON Pointer absolu attendu : {}",
            change.path
        );
    }
}

#[test]
fn revision_diff_is_sorted() {
    let repo = fixture();
    let object = id(ORDER);
    let diff = repo.revision_diff(object, 1, 2).expect("diff 1 → 2");

    let paths: Vec<String> = diff.changes.iter().map(|c| c.path.clone()).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "changes triés par path");

    let unique: std::collections::BTreeSet<&String> = paths.iter().collect();
    assert_eq!(unique.len(), paths.len(), "un seul changement par chemin");

    // Révisions identiques : liste vide, pas d'erreur.
    let same = repo.revision_diff(object, 2, 2).expect("diff 2 → 2");
    assert!(same.changes.is_empty());
    assert_eq!(same.from, 2);
    assert_eq!(same.to, 2);
}

#[test]
fn revision_diff_is_immutable() {
    let repo = fixture();
    let object = id(ORDER);
    let snapshot_before = repo.export_json();
    let revisions_before = repo.revisions(object);

    let first = repo.revision_diff(object, 1, 2).expect("premier diff");
    let second = repo.revision_diff(object, 1, 2).expect("second diff");
    assert_eq!(first, second, "diff pur et reproductible");

    let reversed = repo.revision_diff(object, 2, 1).expect("diff inverse");
    assert_eq!(reversed.from, 2);
    assert_eq!(reversed.to, 1);

    assert_eq!(
        repo.revisions(object),
        revisions_before,
        "snapshots intacts"
    );
    assert_eq!(repo.export_json(), snapshot_before, "aucune écriture");
}

#[test]
fn revision_diff_excludes_volatile_metadata() {
    let repo = fixture();
    let object = id(ORDER);
    let diff = repo.revision_diff(object, 1, 2).expect("diff 1 → 2");
    let paths: Vec<&str> = diff.changes.iter().map(|c| c.path.as_str()).collect();

    // §5.3 : métadonnées affichées à part, jamais dans le tableau du diff.
    assert!(
        !paths.contains(&"/object/revision"),
        "la révision d'objet est exclue : {paths:?}"
    );
    assert!(
        !paths.contains(&"/object/updated_at"),
        "l'horodatage de mise à jour est exclu : {paths:?}"
    );

    // L'exclusion ne vide pas le diff : le contenu reste comparé.
    assert!(paths.contains(&"/object/name"), "{paths:?}");
    assert!(paths.contains(&"/semantic_properties/owner"), "{paths:?}");
}

#[test]
fn revision_diff_missing_revision_is_diff_001() {
    let repo = fixture();
    let object = id(ORDER);

    // Révision absente du stockage de snapshots.
    let error = repo.revision_diff(object, 1, 42).unwrap_err();
    assert!(error.is("DIFF-001"), "{error}");

    // `from == to` ne court-circuite pas l'existence des snapshots (§5.3).
    let error = repo.revision_diff(object, 42, 42).unwrap_err();
    assert!(error.is("DIFF-001"), "{error}");

    // Objet inconnu : même code, pas de panique.
    let error = repo.revision_diff(id(MISSING), 1, 2).unwrap_err();
    assert!(error.is("DIFF-001"), "{error}");

    // Aucune écriture : les snapshots restent consultables.
    assert_eq!(repo.revisions(object).len(), 2);
}
