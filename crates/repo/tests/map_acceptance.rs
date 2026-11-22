//! Acceptation de la couche native v0.4 : commandes, projection, persistance.
//!
//! Ces tests remplacent les tests « accueil = Atelier » du MVP3, contredits par
//! ADR-023. Ils prouvent les règles nommées de C-10 sans dépendre du rendu.

use archifier_core::map::*;
use archifier_repo::map::*;
use archifier_repo::{MemoryStore, Repository};
use std::rc::Rc;

fn repo() -> Repository {
    Repository::new(Rc::new(MemoryStore::new()), 1_760_000_000_000, 42)
}

fn seed_minimal(repo: &Repository) -> (archifier_core::ObjectId, archifier_core::ObjectId) {
    let team = repo
        .map_create_team(NewTeam {
            name: "Équipe Souscription".into(),
            topology: TeamTopology::StreamAligned,
            mission: "Posséder le parcours.".into(),
        })
        .unwrap();
    let value_stream = repo
        .map_create_value_stream(NewValueStream {
            name: "Souscrire".into(),
            value_proposition: "Un contrat en dix minutes.".into(),
            stages: vec!["Choisir".into(), "Payer".into()],
            owner_team_id: Some(team),
        })
        .unwrap();
    (team, value_stream)
}

#[test]
fn first_run_creates_product_not_domain() {
    let repo = repo();
    assert!(repo.map_is_empty());
    let (team, value_stream) = seed_minimal(&repo);
    let product = repo
        .map_create_product(NewProduct {
            name: "Parcours Souscription".into(),
            kind: ProductKind::Customer,
            criticality: Criticality::Critical,
            lifecycle: LifecycleState::Discovery,
            accountable_team_id: Some(team),
            primary_value_stream_id: Some(value_stream),
            ..Default::default()
        })
        .unwrap();
    let data = repo.map_data();
    assert_eq!(data.products.len(), 1);
    assert!(
        data.domains.is_empty(),
        "aucun domaine n'est créé au premier lancement"
    );
    assert_eq!(data.products[0].id, product);
    let projection = repo.map_projection(&MapQuery::enterprise());
    assert!(projection
        .nodes
        .iter()
        .any(|node| node.object_id == product));
}

#[test]
fn product_requires_accountable_team() {
    let repo = repo();
    let error = repo
        .map_create_product(NewProduct {
            name: "Orphelin".into(),
            kind: ProductKind::Internal,
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(error.code, "PRD-001");
    assert!(repo.map_data().products.is_empty());
}

#[test]
fn application_is_used_by_products_not_owned() {
    let repo = repo();
    let (team, _) = seed_minimal(&repo);
    let application = repo
        .map_create_application(NewApplication {
            name: "Payment Hub".into(),
            kind: ApplicationKind::Platform,
            criticality: Criticality::Critical,
            lifecycle: LifecycleState::Run,
            operating_team_id: Some(team),
            node_ids: vec![],
        })
        .unwrap();
    let first = repo
        .map_create_product(NewProduct {
            name: "Produit A".into(),
            accountable_team_id: Some(team),
            application_ids: vec![application],
            ..Default::default()
        })
        .unwrap();
    let second = repo
        .map_create_product(NewProduct {
            name: "Produit B".into(),
            accountable_team_id: Some(team),
            application_ids: vec![application],
            ..Default::default()
        })
        .unwrap();

    let data = repo.map_data();
    let app = data.application(application).unwrap();
    assert!(!app.name.is_empty());
    let users: Vec<_> = data
        .products_using_application(application)
        .into_iter()
        .map(|product| product.id)
        .collect();
    assert_eq!(users, vec![first, second]);
    let shared = repo
        .map_anomalies()
        .into_iter()
        .any(|issue| issue.code == "PRD-006" && issue.object_id == Some(application));
    assert!(shared, "une application partagée reste un couplage signalé");
}

#[test]
fn interface_consumption_requires_version() {
    let repo = repo();
    let (team, _) = seed_minimal(&repo);
    let application = repo
        .map_create_application(NewApplication {
            name: "Portail".into(),
            operating_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    let error = repo
        .map_add_interface_consumption(
            archifier_core::ObjectId::from_parts(1, 999),
            application,
            Criticality::High,
        )
        .unwrap_err();
    assert_eq!(error.code, "PRD-026");
}

#[test]
fn map_keeps_geometry_across_lenses() {
    let repo = repo();
    seed_minimal(&repo);
    let _ = repo.load_demo_map().unwrap();
    let base = MapQuery::enterprise();
    let product = repo.map_projection(&base.clone().with_lens(MapLens::Product));
    let organization = repo.map_projection(&base.with_lens(MapLens::Organization));
    assert_eq!(product.nodes.len(), organization.nodes.len());
    for (left, right) in product.nodes.iter().zip(organization.nodes.iter()) {
        assert_eq!(left.id, right.id);
        assert_eq!(
            (left.x, left.y, left.w, left.h),
            (right.x, right.y, right.w, right.h)
        );
    }
}

#[test]
fn map_zooms_from_enterprise_to_infrastructure() {
    let repo = repo();
    let _ = repo.load_demo_map().unwrap();
    let data = repo.map_data();
    let product = data.products.first().unwrap();
    let application = data
        .applications
        .iter()
        .find(|application| {
            product.application_ids.contains(&application.id)
                && !application.component_ids.is_empty()
        })
        .unwrap();
    let node = data
        .nodes
        .iter()
        .find(|node| application.node_ids.contains(&node.id))
        .unwrap();

    let enterprise = repo.map_projection(&MapQuery::enterprise());
    assert_eq!(enterprise.query.level, MapLevel::Enterprise);

    let product_map = repo.map_projection(
        &MapQuery::default()
            .at_scope(MapScope::product(product.id))
            .level(MapLevel::Product),
    );
    assert!(product_map
        .nodes
        .iter()
        .any(|node| node.object_kind == MapObjectKind::Application));

    let application_map = repo.map_projection(
        &MapQuery::default()
            .at_scope(MapScope::application(application.id))
            .level(MapLevel::Application),
    );
    assert!(!application_map.nodes.is_empty());

    let infrastructure = repo.map_projection(
        &MapQuery::default()
            .at_scope(MapScope::node(node.id))
            .level(MapLevel::Infrastructure),
    );
    assert_eq!(infrastructure.query.level, MapLevel::Infrastructure);
    assert!(!infrastructure.nodes.is_empty());
}

#[test]
fn map_deep_link_restores_context() {
    let repo = repo();
    let _ = repo.load_demo_map().unwrap();
    let data = repo.map_data();
    let product = data.products.first().unwrap().id;
    let query = MapQuery::from_params(
        &format!("PRODUCT:{product}"),
        "PRODUCT",
        "INTERFACE,TIME",
        Some("2026-11-19"),
        &product.to_string(),
        "read",
    );
    let projection = repo.map_projection(&query);
    assert_eq!(projection.query.scope.kind, MapScopeKind::Product);
    assert_eq!(projection.query.level, MapLevel::Product);
    assert_eq!(
        projection.query.lenses,
        vec![MapLens::Interface, MapLens::Time]
    );
    assert_eq!(projection.query.date.as_deref(), Some("2026-11-19"));
}

#[test]
fn map_commands_survive_reload() {
    let repo = repo();
    let (team, stream) = seed_minimal(&repo);
    let product = repo
        .map_create_product(NewProduct {
            name: "Parcours".into(),
            accountable_team_id: Some(team),
            primary_value_stream_id: Some(stream),
            ..Default::default()
        })
        .unwrap();
    let application = repo
        .map_create_application(NewApplication {
            name: "Portail".into(),
            operating_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    repo.map_create_interface(NewInterface {
        name: "Portail API".into(),
        level: InterfaceLevel::Business,
        kind: InterfaceKind::Api,
        exposure: InterfaceExposure::Product,
        criticality: Criticality::High,
        provider_object_id: product,
        provider_product_id: Some(product),
        data_classification: Some(DataClassification::Confidential),
    })
    .unwrap();
    let interface = repo.map_data().interfaces[0].id;
    repo.map_create_interface_version(NewVersion {
        interface_id: interface,
        semantic_version: "1.0.0".into(),
        available_from: Some("2026-01-01".into()),
        available_until: None,
        breaking_change: false,
    })
    .unwrap();
    repo.map_add_application_to_product(product, application)
        .unwrap();
    repo.map_set_anchor(product, MapLevel::Enterprise, 42.0, 84.0)
        .unwrap();

    let json = repo.export_json();
    let reloaded =
        Repository::from_json(Rc::new(MemoryStore::new()), &json, 1_760_000_000_000, 42).unwrap();
    let data = reloaded.map_data();
    assert_eq!(data.products.len(), 1);
    assert_eq!(data.applications.len(), 1);
    assert_eq!(data.interfaces.len(), 1);
    assert_eq!(data.versions.len(), 1);
    assert_eq!(data.anchors.len(), 1);
    let projection = reloaded.map_projection(&MapQuery::enterprise());
    let node = projection
        .nodes
        .iter()
        .find(|node| node.object_id == product)
        .unwrap();
    assert_eq!((node.x, node.y), (42.0, 84.0));
}

#[test]
fn map_aggregate_edge_expands_to_interfaces() {
    let repo = repo();
    let (team, stream) = seed_minimal(&repo);
    let provider = repo
        .map_create_product(NewProduct {
            name: "Fournisseur".into(),
            kind: ProductKind::Platform,
            accountable_team_id: Some(team),
            primary_value_stream_id: Some(stream),
            ..Default::default()
        })
        .unwrap();
    let consumer = repo
        .map_create_product(NewProduct {
            name: "Consommateur".into(),
            accountable_team_id: Some(team),
            primary_value_stream_id: Some(stream),
            ..Default::default()
        })
        .unwrap();
    for (name, version) in [("API A", "1.0.0"), ("API B", "2.0.0")] {
        let interface = repo
            .map_create_interface(NewInterface {
                name: name.into(),
                level: InterfaceLevel::Business,
                kind: InterfaceKind::Api,
                exposure: InterfaceExposure::Product,
                criticality: Criticality::High,
                provider_object_id: provider,
                provider_product_id: Some(provider),
                data_classification: None,
            })
            .unwrap();
        let version_id = repo
            .map_create_interface_version(NewVersion {
                interface_id: interface,
                semantic_version: version.into(),
                available_from: Some("2026-01-01".into()),
                available_until: None,
                breaking_change: false,
            })
            .unwrap();
        repo.map_add_interface_consumption(version_id, consumer, Criticality::High)
            .unwrap();
    }
    let projection = repo.map_projection(&MapQuery::enterprise());
    let edge = projection
        .edges
        .iter()
        .find(|edge| edge.count == 2)
        .expect("une route agrégée");
    assert!(edge.aggregated);
    assert!(edge.expandable);
    assert_eq!(edge.interface_ids.len(), 2);
    assert_eq!(edge.interface_version_ids.len(), 2);
}

#[test]
fn product_value_stream_can_be_attached_after_creation() {
    let repo = repo();
    let (team, value_stream) = seed_minimal(&repo);
    let product = repo
        .map_create_product(NewProduct {
            name: "Produit sans périmètre".into(),
            kind: ProductKind::Customer,
            criticality: Criticality::Medium,
            lifecycle: LifecycleState::Discovery,
            accountable_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    assert!(repo
        .map_anomalies()
        .iter()
        .any(|issue| issue.code == "PRD-033" && issue.is_completeness()));

    repo.map_set_product_value_stream(product, Some(value_stream))
        .unwrap();
    assert!(!repo
        .map_anomalies()
        .iter()
        .any(|issue| issue.code == "PRD-033"));
    assert_eq!(
        repo.map_data()
            .product(product)
            .and_then(|product| product.primary_value_stream_id),
        Some(value_stream)
    );
}

#[test]
fn native_inventory_covers_capabilities_and_nodes() {
    let repo = repo();
    let (team, value_stream) = seed_minimal(&repo);
    let _product = repo
        .map_create_product(NewProduct {
            name: "Parcours Souscription".into(),
            kind: ProductKind::Customer,
            criticality: Criticality::High,
            lifecycle: LifecycleState::Run,
            accountable_team_id: Some(team),
            primary_value_stream_id: Some(value_stream),
            ..Default::default()
        })
        .unwrap();
    let capability = repo
        .map_create_capability(NewCapability {
            name: "Tarification".into(),
            parent_id: None,
            criticality: Criticality::High,
        })
        .unwrap();
    let node = repo
        .map_create_node(NewNode {
            name: "Kubernetes Production".into(),
            kind: TechnologyNodeKind::Cluster,
            environment: Environment::Prod,
            provider_team_id: Some(team),
        })
        .unwrap();

    let inventory = repo.map_inventory();
    assert!(inventory
        .iter()
        .any(|hit| hit.object_id == capability && hit.kind == MapObjectKind::Capability));
    assert!(inventory
        .iter()
        .any(|hit| hit.object_id == node && hit.kind == MapObjectKind::Node));

    let hits = repo.map_search("Tarification");
    assert!(hits.iter().any(|hit| hit.object_id == capability));
}

#[test]
fn domain_is_classification_not_owner() {
    let repo = repo();
    let _ = repo.load_demo_map().unwrap();
    let data = repo.map_data();
    assert!(!data.domains.is_empty());
    for product in &data.products {
        assert!(
            product.accountable_team_id.is_some(),
            "la responsabilité vient de l'équipe, jamais du domaine"
        );
    }
    let projection = repo.map_projection(&MapQuery::enterprise());
    assert!(projection
        .nodes
        .iter()
        .all(|node| node.object_kind == MapObjectKind::Product));
}

#[test]
fn map_rename_object_updates_the_label() {
    let repo = repo();
    let (team, _) = seed_minimal(&repo);
    let product = repo
        .map_create_product(NewProduct {
            name: "Produit A".into(),
            accountable_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    repo.map_rename_object(product, "Produit renomme").unwrap();
    let data = repo.map_data();
    let renamed = data.product(product).unwrap();
    assert_eq!(renamed.name, "Produit renomme");
    assert_eq!(renamed.revision, 2);
    assert_eq!(
        repo.map_rename_object(product, "   ").unwrap_err().code,
        "OBJ-001"
    );
}

#[test]
fn stage_creation_and_links_are_idempotent() {
    let repo = repo();
    let (team, value_stream) = seed_minimal(&repo);
    let product = repo
        .map_create_product(NewProduct {
            name: "Produit lié".into(),
            accountable_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    let capability = repo
        .map_create_capability(NewCapability {
            name: "Capacité liée".into(),
            ..Default::default()
        })
        .unwrap();
    let stage = repo
        .map_create_stage(NewStage {
            value_stream_id: value_stream,
            name: "Étape liée".into(),
            value_item: "Une valeur".into(),
        })
        .unwrap();
    repo.map_add_product_to_stage(stage, product).unwrap();
    repo.map_add_product_to_stage(stage, product).unwrap();
    repo.map_add_capability_to_stage(stage, capability).unwrap();
    repo.map_add_capability_to_stage(stage, capability).unwrap();

    let data = repo.map_data();
    let created = data.stage(stage).unwrap();
    assert_eq!(created.order, 3);
    assert_eq!(created.product_ids, vec![product]);
    assert_eq!(created.capability_ids, vec![capability]);
    assert!(data
        .value_stream(value_stream)
        .unwrap()
        .stage_ids
        .contains(&stage));
}

#[test]
fn owners_can_be_created_then_linked() {
    let repo = repo();
    let (team, value_stream) = seed_minimal(&repo);
    let product = repo
        .map_create_product(NewProduct {
            name: "Produit à rattacher".into(),
            accountable_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    let application = repo
        .map_create_application(NewApplication {
            name: "Application à rattacher".into(),
            operating_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    let node = repo
        .map_create_node(NewNode {
            name: "Nœud à rattacher".into(),
            ..Default::default()
        })
        .unwrap();
    let new_team = repo
        .map_create_team(NewTeam {
            name: "Nouvelle équipe".into(),
            ..Default::default()
        })
        .unwrap();

    repo.map_set_product_accountable_team(product, Some(new_team))
        .unwrap();
    repo.map_set_application_operating_team(application, Some(new_team))
        .unwrap();
    repo.map_set_value_stream_owner(value_stream, Some(new_team))
        .unwrap();
    repo.map_set_node_provider_team(node, Some(new_team))
        .unwrap();

    let data = repo.map_data();
    assert_eq!(
        data.product(product).unwrap().accountable_team_id,
        Some(new_team)
    );
    assert_eq!(
        data.application(application).unwrap().operating_team_id,
        Some(new_team)
    );
    assert_eq!(
        data.value_stream(value_stream).unwrap().owner_team_id,
        Some(new_team)
    );
    assert_eq!(data.node(node).unwrap().provider_team_id, Some(new_team));
}

#[test]
fn domain_creation_classifies_product_without_ownership() {
    let repo = repo();
    let (team, _) = seed_minimal(&repo);
    let product = repo
        .map_create_product(NewProduct {
            name: "Produit classé".into(),
            accountable_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    let domain = repo
        .map_create_business_domain(NewDomain {
            name: "Domaine lié".into(),
            description: "Classification".into(),
            parent_id: None,
        })
        .unwrap();
    repo.map_add_product_to_domain(product, domain).unwrap();
    repo.map_add_product_to_domain(product, domain).unwrap();

    let data = repo.map_data();
    assert_eq!(
        data.product(product).unwrap().business_domain_ids,
        vec![domain]
    );
    assert_eq!(
        data.product(product).unwrap().accountable_team_id,
        Some(team)
    );
}

#[test]
fn interface_provided_by_component_is_tracked_on_the_component() {
    let repo = repo();
    let (team, _) = seed_minimal(&repo);
    let application = repo
        .map_create_application(NewApplication {
            name: "Application composée".into(),
            operating_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    let component = repo
        .map_create_component(NewComponent {
            application_id: application,
            name: "Composant fournisseur".into(),
            kind: ComponentKind::Component,
        })
        .unwrap();
    let interface = repo
        .map_create_interface(NewInterface {
            name: "API du composant".into(),
            level: InterfaceLevel::Application,
            kind: InterfaceKind::Api,
            exposure: InterfaceExposure::Internal,
            criticality: Criticality::High,
            provider_object_id: component,
            provider_product_id: None,
            data_classification: None,
        })
        .unwrap();

    let data = repo.map_data();
    assert!(data
        .component(component)
        .unwrap()
        .interface_ids
        .contains(&interface));
}

#[test]
fn map_link_creates_a_draft_interface_and_consumption() {
    let repo = repo();
    let (team, _) = seed_minimal(&repo);
    let provider = repo
        .map_create_product(NewProduct {
            name: "Produit fournisseur".into(),
            accountable_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    let consumer = repo
        .map_create_product(NewProduct {
            name: "Produit consommateur".into(),
            accountable_team_id: Some(team),
            ..Default::default()
        })
        .unwrap();
    let interface = repo
        .map_create_interface_between(provider, consumer, InterfaceLevel::Business)
        .unwrap();
    let data = repo.map_data();
    let linked = data.interface(interface).unwrap();
    assert_eq!(linked.provider_object_id, provider);
    let versions = data.versions_of(interface);
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].publication_status, PublicationState::Draft);
    assert!(data.consumptions.iter().any(|consumption| {
        consumption.interface_version_id == versions[0].id
            && consumption.consumer_object_id == consumer
    }));
}
