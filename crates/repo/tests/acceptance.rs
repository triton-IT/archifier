//! Scénarios d'acceptation au niveau du moteur, dérivés des spécifications
//! (Gherkin de `functional-specifications.md`) et exécutés sans navigateur.

use archifier_core::{
    ChangeOperation, DiagramNotation, InterfaceCategory, Notation, ObjectId, TemporalStatus,
};
use archifier_repo::types::{Direction, InterfaceFilter, ObjectRef};
use archifier_repo::{MemoryStore, RepoResult, Repository};
use std::rc::Rc;
use time::macros::datetime;

const NOW_MS: u64 = 1_795_000_000_000; // octobre 2026

fn repo() -> Repository {
    let store = Rc::new(MemoryStore::new());
    Repository::new(store, NOW_MS, 42)
}

fn seeded() -> Repository {
    let repo = repo();
    archifier_repo::seed::seed_demo(&repo).expect("jeu de démonstration");
    repo
}

fn domain_by_name(repo: &Repository, name: &str) -> ObjectId {
    repo.domains()
        .into_iter()
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("domaine {name} absent"))
        .id
}

#[test]
fn seed_builds_the_demo_chains() {
    let repo = seeded();
    assert_eq!(repo.domains().len(), 3);
    let stats = repo.stats();
    assert_eq!(stats.elements, 9, "{stats:?}");
    assert!(stats.relationships >= 7);
    assert_eq!(stats.interfaces, 3);
    assert_eq!(stats.diagrams, 2);
    assert_eq!(stats.stories, 1);
    assert_eq!(stats.reviews, 1);
}

#[test]
fn payment_api_has_two_active_periods_and_an_overlap() {
    let repo = seeded();
    let payment = repo
        .interfaces(&Default::default())
        .into_iter()
        .find(|i| i.name == "Payment API")
        .unwrap();
    let timeline = repo.timeline(payment.id).unwrap();
    assert_eq!(timeline.segments.len(), 2);
    assert_eq!(timeline.segments[0].status, Some(TemporalStatus::Active));
    assert_eq!(timeline.segments[1].status, Some(TemporalStatus::Scheduled));
    assert_eq!(timeline.overlaps.len(), 1, "période de migration attendue");
    assert_eq!(timeline.overlaps[0].versions.len(), 2);
}

#[test]
fn expired_consumer_is_reported_as_anomaly() {
    // Scénario « Détecter un consommateur d'une version expirée ».
    let repo = seeded();
    let commande = domain_by_name(&repo, "Commande");
    let overview = repo.domain_overview(commande).unwrap();
    let anomaly = overview
        .anomalies
        .iter()
        .find(|a| a.message.contains("Refund API"))
        .expect("anomalie DEPENDENCY_ON_EXPIRED_INTERFACE attendue");
    assert!(anomaly.href.starts_with("#/view/interfaces/"));
}

#[test]
fn scheduled_interface_is_visible_with_its_start_date() {
    // Scénario « Afficher une interface future ».
    let repo = seeded();
    let pay = domain_by_name(&repo, "Paiements");
    let overview = repo.domain_overview(pay).unwrap();
    let payment = overview
        .exposed_interfaces
        .iter()
        .find(|i| i.name == "Payment API")
        .unwrap();
    assert_eq!(payment.status, Some(TemporalStatus::Active));
    assert!(payment.next_change.is_some(), "date de début de v2 visible");
    let timeline = repo.timeline(payment.id).unwrap();
    let future = timeline
        .segments
        .iter()
        .find(|s| s.status == Some(TemporalStatus::Scheduled))
        .unwrap();
    assert!(future.from.is_some());
}

#[test]
fn invalid_window_is_refused_with_br_002() {
    // Scénario « Refuser une période invalide ».
    let repo = repo();
    let domain = repo.create_domain("Paiements", "").unwrap();
    let interface = repo
        .create_interface(
            domain,
            "Payment API",
            "payment-api",
            InterfaceCategory::Application,
        )
        .unwrap();
    let version = repo
        .create_interface_version(interface, "1.0.0", None, None, false, false)
        .unwrap();
    let error = repo
        .publish_version(
            version,
            datetime!(2027-05-01 00:00 UTC),
            datetime!(2027-04-01 00:00 UTC),
            false,
        )
        .unwrap_err();
    assert!(error.message.contains("BR-002"), "{error}");
}

#[test]
fn reusing_an_interface_in_a_diagram_creates_no_duplicate() {
    // Scénario « Réutiliser une interface d'un autre domaine ».
    let repo = seeded();
    let pay = domain_by_name(&repo, "Paiements");
    let commande = domain_by_name(&repo, "Commande");
    let before = repo.stats().interfaces;

    let payment = repo
        .interfaces(&InterfaceFilter {
            domain_id: Some(pay),
            ..Default::default()
        })
        .into_iter()
        .find(|i| i.name == "Payment API")
        .unwrap();

    let diagram = repo
        .create_diagram(commande, "Vue Commande", DiagramNotation::Archimate)
        .unwrap();
    repo.add_node(diagram, payment.id, 40.0, 40.0).unwrap();

    assert_eq!(repo.stats().interfaces, before, "aucune interface créée");
    let detail = repo.diagram_detail(diagram).unwrap();
    assert_eq!(detail.diagram.layout.nodes.len(), 1);
    assert_eq!(detail.diagram.layout.nodes[0].object_id, payment.id);
    // Le domaine responsable affiché reste Paiements.
    let summary = repo
        .all_objects()
        .into_iter()
        .find(|o| o.id == payment.id)
        .unwrap();
    assert_eq!(summary.domain_name, "Paiements");
}

#[test]
fn cross_notation_relation_must_be_native() {
    let repo = seeded();
    let ord = domain_by_name(&repo, "Commande");
    let bpmn_task = repo
        .elements(&Default::default())
        .into_iter()
        .find(|e| e.name == "Calculer tarif")
        .unwrap();
    let archi_service = repo
        .elements(&Default::default())
        .into_iter()
        .find(|e| e.name == "Pricing Service")
        .unwrap();

    let error = repo
        .create_relationship(ord, bpmn_task.id, archi_service.id, "serving")
        .unwrap_err();
    assert!(error.is("MM-REL-003"), "{error}");

    let id = repo
        .create_relationship(ord, bpmn_task.id, archi_service.id, "implements")
        .unwrap();
    let object = repo.all_objects().into_iter().find(|o| o.id == id).unwrap();
    assert_eq!(
        object.notation,
        Some(Notation::Native),
        "relation transverse = métamodèle natif"
    );
}

#[test]
fn unsupported_metaclass_is_refused() {
    let repo = repo();
    let domain = repo.create_domain("Paiements", "").unwrap();
    let error = repo
        .create_element(
            domain,
            Notation::Archimate,
            "Junction",
            "Jonction",
            Default::default(),
        )
        .unwrap_err();
    assert!(error.is("MM-ELEM-001"), "{error}");
    assert!(error.message.contains("sous-ensemble MVP1"));
}

#[test]
fn archived_object_is_read_only() {
    let repo = seeded();
    let element = repo
        .elements(&Default::default())
        .into_iter()
        .find(|e| e.name == "Pricing API")
        .unwrap();
    repo.archive(element.id).unwrap();
    let error = repo
        .update_element(element.id, "Pricing API v2", Default::default())
        .unwrap_err();
    assert!(error.is("OBJ-007"), "{error}");
}

#[test]
fn archiving_a_domain_with_active_objects_is_refused() {
    let repo = seeded();
    let pay = domain_by_name(&repo, "Paiements");
    let error = repo.archive(pay).unwrap_err();
    assert!(error.is("DOM-001"), "{error}");
    assert!(error.message.contains("objet(s) actif(s)"));
}

#[test]
fn ids_and_history_survive_a_reload() {
    // NFR-010 : une sauvegarde préserve les identifiants et l'historique.
    let repo = seeded();
    let snapshot = repo.export_json();
    let element = repo
        .elements(&Default::default())
        .into_iter()
        .next()
        .unwrap();

    let store = Rc::new(MemoryStore::new());
    let restored = Repository::from_json(store, &snapshot, NOW_MS, 7).unwrap();
    let again = restored.element(element.id).expect("même identifiant");
    assert_eq!(again.object.name, element.name);
    assert!(!restored.revisions(element.id).is_empty());
}

#[test]
fn every_write_is_persisted() {
    let store = Rc::new(MemoryStore::new());
    let repo = Repository::new(store.clone(), NOW_MS, 1);
    repo.create_domain("Paiements", "").unwrap();
    assert_eq!(store.save_count(), 1);
    assert!(store.snapshot().unwrap().contains("Paiements"));
}

#[test]
fn changeset_cannot_be_applied_without_confirmation() {
    // Scénario « Interdire une modification IA silencieuse ».
    let repo = seeded();
    let ord = domain_by_name(&repo, "Commande");
    let before = repo.stats().elements;
    let changeset = repo.make_changeset(
        "Ajoute la dépendance entre A et B",
        archifier_core::ChangeOrigin::Ai,
        vec![ChangeOperation::CreateElement {
            temp_id: "tmp-1".into(),
            domain_id: ord,
            notation: Notation::Archimate,
            metaclass: "ApplicationComponent".into(),
            name: "Facturation".into(),
            metamodel_version: None,
            properties: Default::default(),
        }],
    );
    let error = repo.apply_changeset(&changeset, false).unwrap_err();
    assert!(error.is("CS-CONFIRM"), "{error}");
    assert_eq!(repo.stats().elements, before, "le référentiel est inchangé");

    let created = repo.apply_changeset(&changeset, true).unwrap();
    assert_eq!(created.len(), 1);
    assert_eq!(repo.stats().elements, before + 1);
}

#[test]
fn changeset_with_unknown_id_is_refused() {
    let repo = seeded();
    let changeset = repo.make_changeset(
        "archive un objet inexistant",
        archifier_core::ChangeOrigin::Ai,
        vec![ChangeOperation::ArchiveObject {
            object_id: ObjectId::from_parts(1, 999),
        }],
    );
    let error = repo.apply_changeset(&changeset, true).unwrap_err();
    assert!(error.is("CS-001"), "{error}");
}

#[test]
fn search_finds_documentation_and_ignores_accents() {
    let repo = seeded();
    let hits = repo.search("solvabilité", &[]);
    // Le mot n'existe pas dans le jeu : aucune réponse inventée.
    assert!(hits.is_empty());

    let hits = repo.search("encaissement", &[]);
    assert!(!hits.is_empty());
    let hits = repo.search("ENCAISSEMENT", &[]);
    assert!(!hits.is_empty(), "la casse ne doit pas compter");
}

#[test]
fn broken_documentation_links_are_detected() {
    let repo = seeded();
    let links = repo.documentation(payment_api(&repo));
    assert!(links.contains("arch://"));
    let stats = repo.stats();
    assert!(
        stats.broken_links >= 1,
        "le lien volontairement cassé est signalé"
    );
}

#[test]
fn dependencies_are_bounded_and_direction_aware() {
    let repo = seeded();
    let element = repo
        .elements(&Default::default())
        .into_iter()
        .find(|e| e.name == "Pricing Service")
        .unwrap();
    let out = repo.dependencies(element.id, Direction::Out, 3).unwrap();
    assert!(!out.nodes.is_empty());
    assert!(out.nodes.iter().all(|n| n.depth <= 3));
    let deep = repo.dependencies(element.id, Direction::Out, 99).unwrap();
    assert!(
        deep.nodes.iter().all(|n| n.depth <= 6),
        "profondeur bornée à 6"
    );
}

#[test]
fn documentation_is_versioned() {
    let repo = seeded();
    let element = repo
        .elements(&Default::default())
        .into_iter()
        .find(|e| e.name == "Order Service")
        .unwrap();
    repo.set_documentation(element.id, "Version 2 de la documentation.")
        .unwrap();
    let history = repo.documentation_history(element.id);
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].revision, 2, "la plus récente d'abord");
}

fn payment_api(repo: &Repository) -> ObjectId {
    repo.interfaces(&Default::default())
        .into_iter()
        .find(|i| i.name == "Payment API")
        .unwrap()
        .id
}

#[test]
fn interface_detail_shows_consumers_and_versions() {
    let repo = seeded();
    let detail = repo.interface_detail(payment_api(&repo)).unwrap();
    assert_eq!(detail.versions.len(), 2);
    assert_eq!(detail.provider_domain_name, "Paiements");
    assert!(detail.consumers.iter().any(|c| c.name == "Order Service"));
}

#[test]
fn story_playback_resolves_every_block() {
    let repo = seeded();
    let story = repo.stories().into_iter().next().unwrap();
    let playback = repo.playback(story.id).unwrap();
    assert_eq!(playback.chapters.len(), 3);
    let kinds: Vec<&str> = playback
        .chapters
        .iter()
        .flat_map(|c| c.blocks.iter())
        .map(|b| b.kind.as_str())
        .collect();
    assert!(kinds.contains(&"DIAGRAM"));
    assert!(kinds.contains(&"ELEMENT"));
    assert!(kinds.contains(&"INTERFACE"));
    assert!(kinds.contains(&"TRANSITION"));
}

#[test]
fn review_lists_exposed_and_consumed_interfaces() {
    let repo = seeded();
    let review = repo.reviews().into_iter().next().unwrap();
    let detail = repo.review_detail(review.id).unwrap();
    assert!(!detail.exposed.is_empty());
    assert!(!detail.consumed.is_empty());
    assert!(!detail.comments.is_empty());
}

#[test]
fn failed_persistence_does_not_lose_the_write() {
    let store = Rc::new(archifier_repo::FailingStore);
    let repo = Repository::new(store, NOW_MS, 1);
    let id: RepoResult<ObjectId> = repo.create_domain("Paiements", "");
    assert!(id.is_ok(), "l'écriture en mémoire réussit");
    assert!(
        repo.persist_error().is_some(),
        "l'échec de persistance est signalé"
    );
    assert_eq!(repo.domains().len(), 1);
}

#[test]
fn archived_interface_stays_referenceable() {
    let repo = seeded();
    let id = payment_api(&repo);
    repo.archive(id).unwrap();
    let detail = repo.interface_detail(id).expect("toujours résoluble");
    assert!(!detail.interface.object.is_active());
    let summary = repo.all_objects().into_iter().find(|o| o.id == id).unwrap();
    assert_eq!(summary.lifecycle, archifier_core::Lifecycle::Archived);
}

#[test]
fn object_ref_hrefs_point_to_view_routes() {
    let repo = seeded();
    let element = repo
        .elements(&Default::default())
        .into_iter()
        .next()
        .unwrap();
    let reference: ObjectRef = repo.object_ref(element.id).unwrap();
    assert!(reference.href.starts_with("#/view/elements/"));
}

#[test]
fn timeline_is_computed_for_an_arbitrary_date() {
    let repo = seeded();
    let id = payment_api(&repo);
    let detail = repo.interface_detail(id).unwrap();
    // Les deux versions doivent être actives dans la fenêtre de migration.
    let overlap = repo
        .timeline(id)
        .unwrap()
        .overlaps
        .first()
        .cloned()
        .unwrap();
    repo.set_now_ms(overlap.from.unix_timestamp() as u64 * 1000 + 86_400_000);
    let refreshed = repo.interface_detail(id).unwrap();
    let active = refreshed
        .versions
        .iter()
        .filter(|v| v.status == Some(TemporalStatus::Active))
        .count();
    assert_eq!(active, 2, "v1 et v2 actives pendant la migration");
    assert_eq!(detail.versions.len(), 2);
}
