//! Jeu de démonstration : les quatre chaînes de recette de la spécification.
//!
//! Paiements publie v1 puis v2 en chevauchement, Commande consomme v1 et
//! découvre l'anomalie d'expiration ; la chaîne multi-notation relie BPMN,
//! ArchiMate et UML sans fusionner les objets ; une Story traverse les vues.

use crate::{RepoResult, Repository};
use archifier_core::{
    Chapter, Dataset, DiagramNotation, InterfaceCategory, Notation, ObjectId, StoryBlock,
    StoryStructure,
};
use serde_json::Map;
use time::Duration;

pub fn seed_demo(repo: &Repository) -> RepoResult<()> {
    if !repo.domains().is_empty() {
        return Ok(());
    }

    let now = repo.now();
    repo.add_user("u1", "claire.martin@example.org", "Claire Martin")?;
    repo.add_user("u2", "samir.benali@example.org", "Samir Benali")?;
    repo.add_user("u3", "julia.rossi@example.org", "Julia Rossi")?;
    repo.set_current_user("u1");
    let pay = repo.create_domain(
        "Paiements",
        "Encaisse, autorise et rembourse les paiements. Fournit les interfaces de paiement aux autres domaines.",
    )?;
    let ord = repo.create_domain(
        "Commande",
        "Prend la commande, calcule les tarifs et déclenche l'encaissement.",
    )?;
    let cli = repo.create_domain(
        "Données Clients",
        "Référentiel des clients, consentements et moyens de paiement enregistrés.",
    )?;

    // --- Référentiel ArchiMate -------------------------------------------
    let pricing = repo.create_element(
        pay,
        Notation::Archimate,
        "application-component",
        "Pricing",
        props(&[("owner", "Équipe Tarification"), ("criticality", "haute")]),
    )?;
    let pricing_service = repo.create_element(
        pay,
        Notation::Archimate,
        "application-service",
        "Pricing Service",
        props(&[("owner", "Équipe Tarification")]),
    )?;
    let pricing_iface = repo.create_element(
        pay,
        Notation::Archimate,
        "application-interface",
        "Pricing API",
        Map::new(),
    )?;
    let encaissement = repo.create_element(
        pay,
        Notation::Archimate,
        "business-service",
        "Encaissement",
        Map::new(),
    )?;
    let order_service = repo.create_element(
        ord,
        Notation::Archimate,
        "application-component",
        "Order Service",
        props(&[("criticality", "haute")]),
    )?;
    let order_process = repo.create_element(
        ord,
        Notation::Archimate,
        "business-process",
        "Traiter commande",
        Map::new(),
    )?;

    // --- Chaîne multi-notation : BPMN et UML -----------------------------
    let bpmn_process = repo.create_element(
        ord,
        Notation::Bpmn,
        "process",
        "Traiter commande (BPMN)",
        Map::new(),
    )?;
    let bpmn_task = repo.create_element(
        ord,
        Notation::Bpmn,
        "service-task",
        "Calculer tarif",
        Map::new(),
    )?;
    let uml_component =
        repo.create_element(pay, Notation::Uml, "component", "Pricing (UML)", Map::new())?;

    // Relations intra-notation.
    repo.create_relationship(pay, pricing, pricing_service, "serving")?;
    repo.create_relationship(pay, pricing_service, pricing_iface, "serving")?;
    repo.create_relationship(pay, encaissement, pricing_service, "serving")?;
    repo.create_relationship(ord, order_service, order_process, "serving")?;

    // Relations transverses : jamais de fusion d'objets.
    repo.create_relationship(ord, bpmn_process, order_process, "refines")?;
    repo.create_relationship(ord, bpmn_task, pricing_service, "implements")?;
    repo.create_relationship(pay, uml_component, pricing_service, "realizes")?;

    // --- Interfaces et temporalité ---------------------------------------
    let payment_api = repo.create_interface(
        pay,
        "Payment API",
        "payment-api",
        InterfaceCategory::Application,
    )?;
    let v1 = repo.create_interface_version(
        payment_api,
        "1.0.0",
        Some(now - Duration::days(180)),
        Some(now + Duration::days(60)),
        false,
        true,
    )?;
    repo.create_interface_version(
        payment_api,
        "2.0.0",
        Some(now + Duration::days(30)),
        Some(now + Duration::days(430)),
        true,
        true,
    )?;

    let refund_api = repo.create_interface(
        pay,
        "Refund API",
        "refund-api",
        InterfaceCategory::Application,
    )?;
    let refund_v1 = repo.create_interface_version(
        refund_api,
        "1.0.0",
        Some(now - Duration::days(400)),
        Some(now - Duration::days(10)),
        false,
        true,
    )?;

    let customer_api = repo.create_interface(
        cli,
        "Customer API",
        "customer-api",
        InterfaceCategory::Application,
    )?;
    let customer_v1 = repo.create_interface_version(
        customer_api,
        "1.0.0",
        Some(now - Duration::days(300)),
        Some(now + Duration::days(200)),
        false,
        true,
    )?;

    // Paiements consomme Customer API pour valider le moyen de paiement.
    repo.add_consumer(customer_v1, pricing)?;
    repo.add_consumer(v1, order_service)?;
    repo.add_consumer(customer_v1, order_service)?;
    // Anomalie volontaire : Commande consomme encore une version expirée.
    repo.add_consumer(refund_v1, order_service)?;

    // --- Diagrammes -------------------------------------------------------
    let diagram = repo.create_diagram(pay, "Paiements — applicatif", DiagramNotation::Archimate)?;
    place(
        repo,
        diagram,
        &[
            (pricing, 80.0, 120.0),
            (pricing_service, 380.0, 120.0),
            (pricing_iface, 380.0, 290.0),
            (encaissement, 80.0, 300.0),
        ],
    )?;
    add_edge_between(repo, diagram, pricing, pricing_service)?;
    add_edge_between(repo, diagram, pricing_service, pricing_iface)?;
    add_edge_between(repo, diagram, encaissement, pricing_service)?;

    let multi = repo.create_diagram(ord, "Commande — multi-notation", DiagramNotation::Free)?;
    place(
        repo,
        multi,
        &[
            (bpmn_process, 60.0, 80.0),
            (order_process, 320.0, 80.0),
            (bpmn_task, 60.0, 260.0),
            (pricing_service, 320.0, 260.0),
            (uml_component, 580.0, 260.0),
        ],
    )?;
    add_edge_between(repo, multi, bpmn_process, order_process)?;
    add_edge_between(repo, multi, bpmn_task, pricing_service)?;
    add_edge_between(repo, multi, uml_component, pricing_service)?;

    // --- Documentation ----------------------------------------------------
    repo.set_documentation(
        payment_api,
        &format!(
            "## Payment API\n\n\
             Interface d'autorisation et de capture des paiements.\n\n\
             Deux versions coexistent pendant la migration : **v1** encore consommée \
             par Commande, et **v2** en rupture de contrat.\n\n\
             Voir [Pricing Service](arch://element/{pricing_service}) et \
             [Order Service](arch://element/{order_service}).\n\n\
             Un lien volontairement cassé : [vue supprimée](arch://diagram/01ARZ3NDEKTSV4RRFFQ69G5FAV)."
        ),
    )?;
    repo.set_documentation(
        pricing_service,
        &format!(
            "Service de calcul tarifaire.\n\nRéalisé par le composant Pricing et consommé par \
             Order Service.\n\nVoir [Payment API](arch://interface/{payment_api})."
        ),
    )?;
    repo.set_documentation(
        order_service,
        "Composant de prise de commande et d'orchestration de l'encaissement.",
    )?;

    // --- Story ------------------------------------------------------------
    let story = repo.create_story(ord, "Comment une commande devient une facture")?;
    let structure = StoryStructure {
        chapters: vec![
            Chapter {
                id: "c1".into(),
                title: "1. Le processus métier".into(),
                blocks: vec![
                    StoryBlock::Markdown {
                        id: "b1".into(),
                        markdown: "La commande entre par le **processus Traiter commande**, \
                                   raffiné par sa version BPMN opérationnelle."
                            .into(),
                    },
                    StoryBlock::Diagram {
                        id: "b2".into(),
                        diagram_id: multi,
                        framing: Default::default(),
                    },
                    StoryBlock::Transition {
                        id: "b3".into(),
                        label: "Voir le calcul du tarif".into(),
                        target_block_id: "b4".into(),
                    },
                ],
            },
            Chapter {
                id: "c2".into(),
                title: "2. Le calcul du tarif".into(),
                blocks: vec![
                    StoryBlock::Element {
                        id: "b4".into(),
                        element_id: bpmn_task,
                        highlight: true,
                    },
                    StoryBlock::Markdown {
                        id: "b5".into(),
                        markdown: "La tâche BPMN **Calculer tarif** implémente le service \
                                   applicatif *Pricing Service* : deux objets distincts, reliés \
                                   par une relation native."
                            .into(),
                    },
                    StoryBlock::Element {
                        id: "b6".into(),
                        element_id: pricing_service,
                        highlight: true,
                    },
                ],
            },
            Chapter {
                id: "c3".into(),
                title: "3. L'interface et sa fenêtre de disponibilité".into(),
                blocks: vec![
                    StoryBlock::Interface {
                        id: "b7".into(),
                        interface_id: payment_api,
                        version_id: Some(v1),
                    },
                    StoryBlock::Markdown {
                        id: "b8".into(),
                        markdown: "**Payment API v1** reste consommée par Order Service alors \
                                   que v2 arrive en rupture. La revue du domaine signale \
                                   l'anomalie."
                            .into(),
                    },
                ],
            },
        ],
        pinned_revisions: Default::default(),
        pinned_versions: Default::default(),
    };
    repo.save_story(story, structure)?;

    // --- Revue ------------------------------------------------------------
    let review = repo.create_review(pay, "Revue des interfaces — T4")?;
    repo.add_comment(
        review,
        "v1 est encore consommée par Commande. La migration vers v2 est à planifier \
         avant la fin de disponibilité.",
    )?;

    debug_assert!(repo.timeline(payment_api).is_ok());
    Ok(())
}

fn props(pairs: &[(&str, &str)]) -> Map<String, serde_json::Value> {
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert(
            (*key).to_string(),
            serde_json::Value::String((*value).to_string()),
        );
    }
    map
}

fn place(repo: &Repository, diagram: ObjectId, nodes: &[(ObjectId, f64, f64)]) -> RepoResult<()> {
    for (object_id, x, y) in nodes {
        repo.add_node(diagram, *object_id, *x, *y)?;
    }
    Ok(())
}

fn add_edge_between(
    repo: &Repository,
    diagram: ObjectId,
    source: ObjectId,
    target: ObjectId,
) -> RepoResult<()> {
    let detail = repo
        .element_detail(source)
        .ok_or_else(|| crate::RepoError::new("SEED-001", "Élément source introuvable"))?;
    if let Some(view) = detail.outgoing.iter().find(|r| r.other_id == target) {
        return repo.add_edge(diagram, view.id);
    }
    Ok(())
}

/// Reconnaît le jeu de démonstration historique (identités `@example.org` et
/// domaines livrés par la démo). Au démarrage, un tel snapshot n'est jamais
/// rechargé : l'application repart sur un référentiel vierge.
pub fn is_demo_snapshot(json: &str) -> bool {
    let Ok(data) = serde_json::from_str::<Dataset>(json) else {
        return false;
    };
    let demo_user = data
        .users
        .iter()
        .any(|user| user.email.ends_with("@example.org"));
    let demo_domains = data
        .domains
        .iter()
        .filter(|domain| {
            matches!(
                domain.object.name.as_str(),
                "Paiements" | "Commande" | "Données Clients"
            )
        })
        .count();
    demo_user && demo_domains >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryStore;
    use std::rc::Rc;

    fn repo() -> Repository {
        Repository::new(Rc::new(MemoryStore::new()), 1_700_000_000_000, 42)
    }

    #[test]
    fn demo_snapshot_is_recognized() {
        let repo = repo();
        seed_demo(&repo).expect("jeu de démonstration");
        assert!(is_demo_snapshot(&repo.export_json()));
    }

    #[test]
    fn empty_snapshot_is_not_a_demo_snapshot() {
        let repo = repo();
        assert!(!is_demo_snapshot(&repo.export_json()));
    }
}
