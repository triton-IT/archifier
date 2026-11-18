//! C-09 §4 : méthodes additives de l'Atelier (save_layout_expect, view_cards,
//! rename_interface, set_diagram_publication, interface_of_version).

use archifier_core::{
    DiagramLayout, DiagramNotation, InterfaceCategory, PublicationStatus, ViewIntent, ViewScope,
};
use archifier_repo::{MemoryStore, Repository, ViewFilter};
use std::rc::Rc;

const NOW: u64 = 1_765_000_000_000;

fn repo() -> Repository {
    Repository::new(Rc::new(MemoryStore::new()), NOW, 42)
}

fn setup() -> (Repository, archifier_core::ObjectId) {
    let repository = repo();
    let domain = repository.create_domain("Paiements", "Paiements").unwrap();
    let diagram = repository
        .create_diagram(domain, "Paysage", DiagramNotation::Archimate)
        .unwrap();
    let layout = DiagramLayout {
        schema: 2,
        intent: Some(ViewIntent::ApplicationLandscape),
        scope: ViewScope {
            kind: archifier_core::ScopeKind::Domain,
            domain_id: Some(domain),
            domain_ids: vec![],
        },
        ..Default::default()
    };
    repository.save_layout(diagram, layout).unwrap();
    (repository, diagram)
}

#[test]
fn save_layout_expect_writes_on_matching_revision() {
    let (repository, diagram) = setup();
    let revision = repository
        .diagram_detail(diagram)
        .unwrap()
        .diagram
        .object
        .revision;
    let mut layout = repository.diagram_detail(diagram).unwrap().diagram.layout;
    layout.grid.size = 16;
    repository
        .save_layout_expect(diagram, layout.clone(), revision)
        .unwrap();
    let saved = repository.diagram_detail(diagram).unwrap();
    assert_eq!(saved.diagram.layout.grid.size, 16);
    assert_eq!(saved.diagram.object.revision, revision + 1);
}

#[test]
fn save_layout_expect_returns_409_without_write() {
    let (repository, diagram) = setup();
    let revision = repository
        .diagram_detail(diagram)
        .unwrap()
        .diagram
        .object
        .revision;
    let mut layout = repository.diagram_detail(diagram).unwrap().diagram.layout;
    layout.grid.size = 16;
    repository
        .save_layout_expect(diagram, layout.clone(), revision)
        .unwrap();
    let error = repository
        .save_layout_expect(diagram, layout, revision)
        .unwrap_err();
    assert_eq!(error.code, "REV-409");
    let saved = repository.diagram_detail(diagram).unwrap();
    assert_eq!(saved.diagram.object.revision, revision + 1);
    assert_eq!(saved.diagram.layout.grid.size, 16);
}

#[test]
fn publish_diagram_sets_status_and_date() {
    let (repository, diagram) = setup();
    repository
        .set_diagram_publication(diagram, PublicationStatus::Published)
        .unwrap();
    let detail = repository.diagram_detail(diagram).unwrap();
    assert_eq!(
        detail.diagram.publication_status,
        PublicationStatus::Published
    );
    assert!(detail.diagram.published_at.is_some());
}

#[test]
fn withdraw_diagram_clears_published_at() {
    let (repository, diagram) = setup();
    repository
        .set_diagram_publication(diagram, PublicationStatus::Published)
        .unwrap();
    repository
        .set_diagram_publication(diagram, PublicationStatus::Withdrawn)
        .unwrap();
    let detail = repository.diagram_detail(diagram).unwrap();
    assert_eq!(
        detail.diagram.publication_status,
        PublicationStatus::Withdrawn
    );
    assert!(detail.diagram.published_at.is_none());
}

#[test]
fn rename_interface_keeps_logical_key_and_versions() {
    let repository = repo();
    let domain = repository.create_domain("Paiements", "Paiements").unwrap();
    let interface = repository
        .create_interface(
            domain,
            "Payment API",
            "payment-api",
            InterfaceCategory::Application,
        )
        .unwrap();
    repository
        .create_interface_version(interface, "1.0.0", None, None, false, false)
        .unwrap();
    let before = repository.interface_detail(interface).unwrap();
    repository
        .rename_interface(interface, "Paiement API")
        .unwrap();
    let after = repository.interface_detail(interface).unwrap();
    assert_eq!(after.interface.object.name, "Paiement API");
    assert_eq!(
        after.interface.logical_interface_key,
        before.interface.logical_interface_key
    );
    assert_eq!(after.versions.len(), before.versions.len());
}

#[test]
fn interface_of_version_resolves_owner() {
    let repository = repo();
    let domain = repository.create_domain("Paiements", "Paiements").unwrap();
    let interface = repository
        .create_interface(
            domain,
            "Payment API",
            "payment-api",
            InterfaceCategory::Application,
        )
        .unwrap();
    let version = repository
        .create_interface_version(interface, "1.0.0", None, None, false, false)
        .unwrap();
    assert_eq!(repository.interface_of_version(version), Some(interface));
    assert_eq!(
        repository.interface_of_version(archifier_core::ObjectId::from_parts(1, 9)),
        None
    );
}

#[test]
fn view_cards_are_sorted_and_filtered() {
    let (repository, diagram) = setup();
    repository
        .set_diagram_publication(diagram, PublicationStatus::Published)
        .unwrap();
    let cards = repository.view_cards(&ViewFilter {
        intent: Some(ViewIntent::ApplicationLandscape),
        publication: Some(PublicationStatus::Published),
        ..Default::default()
    });
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].id, diagram);
    assert_eq!(cards[0].intent, ViewIntent::ApplicationLandscape);
    assert_eq!(cards[0].domain_name, "Paiements");
    let none = repository.view_cards(&ViewFilter {
        intent: Some(ViewIntent::Process),
        ..Default::default()
    });
    assert!(none.is_empty());
}

#[test]
fn view_cards_vignette_is_bounded() {
    let (repository, diagram) = setup();
    let domain = repository
        .diagram_detail(diagram)
        .unwrap()
        .diagram
        .object
        .responsible_domain_id;
    let mut layout = repository.diagram_detail(diagram).unwrap().diagram.layout;
    for index in 0..60u128 {
        let object = repository
            .create_element(
                domain,
                archifier_core::Notation::Archimate,
                "ApplicationComponent",
                &format!("Composant {index}"),
                Default::default(),
            )
            .unwrap();
        layout.nodes.push(archifier_core::DiagramNode {
            id: format!("n{index}"),
            object_id: object,
            kind: archifier_core::NodeKind::Element,
            version_id: None,
            x: (index % 10) as f64 * 100.0,
            y: (index / 10) as f64 * 80.0,
            w: 80.0,
            h: 40.0,
            layer_id: None,
            zone_id: None,
            collapsed: false,
            port: None,
            style_override: Default::default(),
        });
    }
    repository.save_layout(diagram, layout).unwrap();
    let cards = repository.view_cards(&ViewFilter::default());
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].node_count, 60);
    assert_eq!(cards[0].mini_nodes.len(), 40);
}
