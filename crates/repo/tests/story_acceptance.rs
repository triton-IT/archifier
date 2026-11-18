//! Tests bloquants W1c-1 — cadrage Story et épinglage (T1-10).
//!
//! Contrat : `docs/w1c-contract.md` §2 et §4. Fixture :
//! `fixtures/mvp2/w1c/story-framing.json`.
//!
//! Le récit référence trois objets uniques (diagramme, élément, interface) à
//! des révisions distinctes (`2`, `3`, `4`). Un second bloc élément répète le
//! même identifiant : l'épinglage doit dédupliquer. Le bloc diagramme porte un
//! cadrage `zoom = 2.0`, `focus = [Order Service]`, `dim = [Pricing Service]`
//! et une légende.
//!
//! API de cadrage attendue côté noyau (`crates/core/src/story.rs`, §5) :
//!
//! ```text
//! pub struct FramedNode {
//!     pub object_id: ObjectId,
//!     pub highlighted: bool,
//!     pub opacity: f32,
//! }
//! pub struct FramedDiagram {
//!     pub zoom: f32,
//!     pub nodes: Vec<FramedNode>,
//!     pub caption: Option<String>,
//! }
//! pub fn apply_framing(framing: &DiagramFraming, layout: &DiagramLayout) -> FramedDiagram;
//! ```
//!
//! Aucun accès au stockage : seules l'API publique du dépôt et le helper pur du
//! noyau sont utilisés.

use archifier_core::story::apply_framing;
use archifier_core::{DiagramFraming, ObjectId, StoryBlock};
use archifier_repo::{MemoryStore, Repository};
use std::collections::BTreeMap;
use std::rc::Rc;

const NOW_MS: u64 = 1_795_000_000_000; // 2026-11-18, horloge figée
const DATASET: &str = include_str!("../../../fixtures/mvp2/w1c/story-framing.json");

const DOMAIN: &str = "01K8ZP4N000000000000000301";
const ELEMENT: &str = "01K8ZP4N000000000000000401";
const DIMMED_ELEMENT: &str = "01K8ZP4N000000000000000402";
const INTERFACE: &str = "01K8ZP4N000000000000000403";
const DIAGRAM: &str = "01K8ZP4N000000000000000404";
const STORY: &str = "01K8ZP4N000000000000000405";
const VERSION: &str = "01K8ZP4N000000000000000406";
/// Récit qui référence un élément sans instantané de révision.
const STORY_MISSING_SNAPSHOT: &str = "01K8ZP4N000000000000000407";
/// Récit dont le bloc interface ne fixe aucune version.
const STORY_WITHOUT_VERSION: &str = "01K8ZP4N000000000000000408";
/// Dernière version publiée de `Payment API`.
const LATEST_PUBLISHED: &str = "01K8ZP4N000000000000000409";
/// Version brouillon : ne doit jamais être épinglée par défaut.
const DRAFT_VERSION: &str = "01K8ZP4N000000000000000410";

fn fixture() -> Repository {
    Repository::from_json(Rc::new(MemoryStore::new()), DATASET, NOW_MS, 7)
        .expect("fixture W1c story lisible")
}

fn id(raw: &str) -> ObjectId {
    raw.parse().expect("ULID de fixture valide")
}

fn pins(repo: &Repository, story: ObjectId) -> BTreeMap<ObjectId, u64> {
    repo.story_detail(story)
        .expect("story lisible")
        .story
        .structure
        .pinned_revisions
}

fn pinned_versions(repo: &Repository, story: ObjectId) -> BTreeMap<ObjectId, ObjectId> {
    repo.story_detail(story)
        .expect("story lisible")
        .story
        .structure
        .pinned_versions
}

#[test]
fn story_pins_revision() {
    let repo = fixture();
    let story = id(STORY);

    let before = repo.story_detail(story).expect("story").story;
    assert!(
        before.structure.pinned_revisions.is_empty(),
        "récit non épinglé au départ"
    );

    let pinned = repo.pin_story_revisions(story).expect("épinglage");
    assert_eq!(
        pinned, 3,
        "diagramme + élément + interface, doublon dédupliqué"
    );

    let after = repo.story_detail(story).expect("story après épinglage");
    let after_revision = after.story.object.revision;
    let stored = after.story.structure.pinned_revisions;
    assert_eq!(stored.len(), 3, "seuls les objets référencés sont épinglés");
    assert_eq!(
        stored.get(&id(DIAGRAM)),
        Some(&2),
        "révision courante du diagramme"
    );
    assert_eq!(
        stored.get(&id(ELEMENT)),
        Some(&3),
        "révision courante de l'élément"
    );
    assert_eq!(
        stored.get(&id(INTERFACE)),
        Some(&4),
        "révision courante de l'interface"
    );
    assert!(
        !stored.contains_key(&id(DIMMED_ELEMENT)),
        "un nœud de diagramme n'est pas une référence de Story"
    );
    assert!(
        after_revision > before.object.revision,
        "l'épinglage passe par save_story et crée une révision"
    );

    // La version explicite du bloc interface est figée à part.
    let versions = pinned_versions(&repo, story);
    assert_eq!(versions.len(), 1);
    assert_eq!(
        versions.get(&id(INTERFACE)),
        Some(&id(VERSION)),
        "version_id du bloc interface"
    );

    // `playback` expose les deux cartes pour le badge « Récit figé ».
    let playback = repo.playback(story).expect("playback");
    assert_eq!(playback.pinned_revisions, stored);
    assert_eq!(playback.pinned_versions, versions);

    // Ré-épingler après modification fige la nouvelle révision courante.
    repo.update_element(id(ELEMENT), "Order Service v2", Default::default())
        .expect("élément modifié");
    let revised = repo.element(id(ELEMENT)).expect("élément").object.revision;
    assert!(revised > 3);
    let pinned_again = repo.pin_story_revisions(story).expect("second épinglage");
    assert_eq!(pinned_again, 3);
    let stored = pins(&repo, story);
    assert_eq!(stored.get(&id(ELEMENT)), Some(&revised));
    assert_eq!(stored.get(&id(DIAGRAM)), Some(&2), "inchangé");
    assert_eq!(stored.get(&id(INTERFACE)), Some(&4), "inchangé");
}

#[test]
fn story_unpin_clears_pins() {
    let repo = fixture();
    let story = id(STORY);

    repo.pin_story_revisions(story).expect("épinglage initial");
    assert_eq!(pins(&repo, story).len(), 3);
    assert_eq!(pinned_versions(&repo, story).len(), 1);
    let pinned_revision = repo
        .story_detail(story)
        .expect("story")
        .story
        .object
        .revision;

    repo.unpin_story_revisions(story).expect("dégelage");
    assert!(pins(&repo, story).is_empty(), "pinned_revisions est vidé");
    assert!(
        pinned_versions(&repo, story).is_empty(),
        "pinned_versions est vidé aussi"
    );
    let after = repo.story_detail(story).expect("story après dégel");
    assert!(
        after.story.object.revision > pinned_revision,
        "le dégel des épingles passe par save_story et crée une révision"
    );

    // Second dégel : aucune écriture, donc aucune nouvelle révision.
    let revision = after.story.object.revision;
    let snapshot = repo.export_json();
    repo.unpin_story_revisions(story)
        .expect("dégel idempotent sans écriture");
    assert!(pins(&repo, story).is_empty());
    assert!(pinned_versions(&repo, story).is_empty());
    assert_eq!(
        repo.story_detail(story)
            .expect("story")
            .story
            .object
            .revision,
        revision,
        "un dégel sans épingle ne crée pas de révision"
    );
    assert_eq!(repo.export_json(), snapshot, "aucune écriture");
}

#[test]
fn story_framing_applies_zoom_focus_dim() {
    let repo = fixture();
    let story = repo.story_detail(id(STORY)).expect("story").story;
    let framing = story
        .structure
        .chapters
        .iter()
        .flat_map(|chapter| chapter.blocks.iter())
        .find_map(|block| match block {
            StoryBlock::Diagram { framing, .. } => Some(framing.clone()),
            _ => None,
        })
        .expect("bloc diagramme avec cadrage");
    assert_eq!(framing.zoom, Some(2.0));
    assert_eq!(framing.caption.as_deref(), Some("Vue ciblée"));

    let layout = repo
        .diagram_detail(id(DIAGRAM))
        .expect("diagramme")
        .diagram
        .layout;
    let source = layout.clone();

    let framed = apply_framing(&framing, &layout);
    assert_eq!(framed.zoom, 2.0, "le zoom est appliqué à l'échelle");
    assert_eq!(framed.caption.as_deref(), Some("Vue ciblée"));

    let focused = framed
        .nodes
        .iter()
        .find(|node| node.object_id == id(ELEMENT))
        .expect("nœud en focus");
    assert!(focused.highlighted, "l'élément en focus est mis en avant");
    assert!(!focused.dimmed);
    assert_eq!(focused.opacity, 1.0);
    assert_eq!(focused.x, 80.0, "x multiplié par le zoom");
    assert_eq!(focused.y, 80.0);
    assert_eq!(focused.w, 360.0);
    assert_eq!(focused.h, 160.0);
    assert_eq!(focused.zoom, 2.0);

    let dimmed = framed
        .nodes
        .iter()
        .find(|node| node.object_id == id(DIMMED_ELEMENT))
        .expect("nœud atténué");
    assert!(!dimmed.highlighted);
    assert!(dimmed.dimmed);
    assert_eq!(
        dimmed.opacity, 0.35,
        "opacité d'atténuation imposée par §7.5"
    );

    // Le layout source n'est jamais muté.
    assert_eq!(layout, source, "aucun impact sur le layout source");

    // Cadrage par défaut : zoom 1, aucun focus, aucune atténuation.
    let plain = apply_framing(&DiagramFraming::default(), &source);
    assert_eq!(plain.zoom, 1.0);
    assert!(plain.caption.is_none());
    assert!(plain
        .nodes
        .iter()
        .all(|node| !node.highlighted && node.opacity == 1.0));

    // Bornes 0.5–3.0.
    for (requested, expected) in [(0.1_f32, 0.5_f32), (9.0, 3.0), (0.5, 0.5), (3.0, 3.0)] {
        let zoomed = apply_framing(
            &DiagramFraming {
                zoom: Some(requested),
                ..Default::default()
            },
            &source,
        );
        assert_eq!(
            zoomed.zoom, expected,
            "zoom {requested} doit être borné à {expected}"
        );
    }
}

#[test]
fn story_framing_fit_is_computed_before_zoom() {
    let repo = fixture();
    let layout = repo
        .diagram_detail(id(DIAGRAM))
        .expect("diagramme")
        .diagram
        .layout;
    let framed = apply_framing(
        &DiagramFraming {
            zoom: Some(2.0),
            ..Default::default()
        },
        &layout,
    );

    let bbox_width = |nodes: &[(f64, f64)]| {
        let min = nodes.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
        let max = nodes
            .iter()
            .map(|(x, w)| x + w)
            .fold(f64::NEG_INFINITY, f64::max);
        max - min
    };
    let source: Vec<(f64, f64)> = layout.nodes.iter().map(|node| (node.x, node.w)).collect();
    let scaled: Vec<(f64, f64)> = framed.nodes.iter().map(|node| (node.x, node.w)).collect();
    let source_width = bbox_width(&source);
    let framed_width = bbox_width(&scaled);
    assert!(
        (framed_width - source_width * 2.0).abs() < 1e-9,
        "le helper livre des coordonnées déjà zoomées"
    );

    // Invariant UI §7.5 : le fit se calcule sur le layout NON zoomé, puis le
    // zoom s'applique. Fitter le layout zoomé annulerait le zoom.
    let viewport_width = source_width; // fit théorique = 1.0 sur le layout source
    let fit_before_zoom = viewport_width / source_width;
    let fit_after_zoom = viewport_width / framed_width;
    assert!((fit_before_zoom * f64::from(framed.zoom) - 2.0).abs() < 1e-9);
    assert!((fit_after_zoom - fit_before_zoom / f64::from(framed.zoom)).abs() < 1e-9);
}

#[test]
fn story_pins_interface_version() {
    let repo = fixture();

    // Bloc avec `version_id` explicite.
    repo.pin_story_revisions(id(STORY)).expect("épinglage");
    let versions = pinned_versions(&repo, id(STORY));
    assert_eq!(versions.get(&id(INTERFACE)), Some(&id(VERSION)));

    // Bloc sans version : dernière version publiée, jamais le brouillon.
    let pinned = repo
        .pin_story_revisions(id(STORY_WITHOUT_VERSION))
        .expect("épinglage sans version explicite");
    assert_eq!(pinned, 1);
    let versions = pinned_versions(&repo, id(STORY_WITHOUT_VERSION));
    assert_eq!(
        versions.get(&id(INTERFACE)),
        Some(&id(LATEST_PUBLISHED)),
        "2.0.0 publiée préférée à 3.0.0 brouillon"
    );
    assert_ne!(versions.get(&id(INTERFACE)), Some(&id(DRAFT_VERSION)));
    assert_eq!(
        pins(&repo, id(STORY_WITHOUT_VERSION)).get(&id(INTERFACE)),
        Some(&4)
    );
}

#[test]
fn story_pin_missing_snapshot_is_story_003() {
    let repo = fixture();
    let story = id(STORY_MISSING_SNAPSHOT);
    let before = repo.story_detail(story).expect("story").story;
    let snapshot = repo.export_json();

    let error = repo
        .pin_story_revisions(story)
        .expect_err("instantané de révision absent");
    assert!(error.is("STORY-003"), "{error}");

    let after = repo.story_detail(story).expect("story").story;
    assert!(after.structure.pinned_revisions.is_empty());
    assert!(after.structure.pinned_versions.is_empty());
    assert_eq!(after.structure, before.structure, "structure inchangée");
    assert_eq!(
        after.object.revision, before.object.revision,
        "aucune révision créée"
    );
    assert_eq!(repo.export_json(), snapshot, "aucune écriture");
}

#[test]
fn save_story_invalid_is_atomic() {
    let repo = fixture();
    let story = id(STORY);
    let before = repo.story_detail(story).expect("story").story;
    let snapshot = repo.export_json();

    let mut invalid = before.structure.clone();
    invalid.chapters[0].blocks.push(StoryBlock::Transition {
        id: "b-invalide".into(),
        label: "Cible absente".into(),
        target_block_id: "bloc-absent".into(),
    });

    let error = repo
        .save_story(story, invalid)
        .expect_err("transition vers un bloc absent");
    assert!(error.is("STORY-002"), "{error}");

    let after = repo.story_detail(story).expect("story après refus").story;
    assert_eq!(after.structure, before.structure, "structure inchangée");
    assert_eq!(
        after.object.revision, before.object.revision,
        "révision inchangée"
    );
    assert_eq!(repo.export_json(), snapshot, "aucune écriture");
}

#[test]
fn story_pin_without_references_returns_zero() {
    let repo = fixture();
    let story = repo
        .create_story(id(DOMAIN), "Récit sans objet")
        .expect("story créée");

    assert_eq!(
        repo.pin_story_revisions(story).expect("épinglage vide"),
        0,
        "aucun objet référencé"
    );
    assert!(pins(&repo, story).is_empty());
}

#[test]
fn story_unpin_is_idempotent_on_empty_pins() {
    let repo = fixture();
    let story = repo
        .create_story(id(DOMAIN), "Récit jamais figé")
        .expect("story créée");
    let revision = repo
        .story_detail(story)
        .expect("story")
        .story
        .object
        .revision;
    let snapshot = repo.export_json();

    repo.unpin_story_revisions(story)
        .expect("dégeler un récit non figé reste permis");
    assert!(pins(&repo, story).is_empty());
    assert!(pinned_versions(&repo, story).is_empty());
    assert_eq!(
        repo.story_detail(story)
            .expect("story")
            .story
            .object
            .revision,
        revision,
        "no-op sans révision"
    );
    assert_eq!(repo.export_json(), snapshot, "aucune écriture");
}
