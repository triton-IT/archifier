//! Tests bloquants W1b — liens cassés (T1-06) et graphe de dépendances (T1-07).
//!
//! Contrat : `docs/w1b-contract.md` §1.3, §1.4, §5.4, §5.9 et §6 ;
//! `technical-specifications.md` §10.2. Fixture :
//! `fixtures/mvp2/export/dataset.json` — documentation de `Order Service v2`
//! avec un lien valide, un lien cassé **répété deux fois** et un lien vers une
//! cible archivée après écriture du lien (cache `OK` périmé) ; chaîne de huit
//! relations `serving` et relation `Facturation → Legacy Billing` (cible archivée).
//!
//! Le test de route `graph_route_parses` vit désormais dans
//! `crates/ui/src/router.rs` (§5.9) : il est à ajouter par le dev UI, ce crate
//! n'étant pas accessible ni modifiable depuis ce périmètre.

use archifier_core::{Lifecycle, LinkStatus, ObjectId, ObjectType};
use archifier_repo::types::Direction;
use archifier_repo::{MemoryStore, Repository};
use std::collections::BTreeSet;
use std::rc::Rc;

const NOW_MS: u64 = 1_795_000_000_000; // horloge figée
const DATASET: &str = include_str!("../../../fixtures/mvp2/export/dataset.json");

const DOMAIN: &str = "01K8ZP4N000000000000000001";
const ORDER: &str = "01K8ZP4N000000000000000002";
const PRICING: &str = "01K8ZP4N000000000000000003";
const TARIF: &str = "01K8ZP4N000000000000000004";
const GRID: &str = "01K8ZP4N000000000000000005";
const ARCHIVED: &str = "01K8ZP4N00000000000000000B";
const MISSING: &str = "01K8ZP4N00000000000000000C";
const FACTURATION: &str = "01K8ZP4N00000000000000000X";

fn fixture() -> Repository {
    Repository::from_json(Rc::new(MemoryStore::new()), DATASET, NOW_MS, 42)
        .expect("fixture W1b lisible")
}

fn id(raw: &str) -> ObjectId {
    raw.parse().expect("ULID de fixture valide")
}

fn uri(identifier: &str) -> String {
    format!("arch://element/{identifier}")
}

#[test]
fn broken_link_diagnostic_has_context_and_owner() {
    let repo = fixture();
    let entries = repo.broken_links();

    let broken = entries
        .iter()
        .find(|entry| entry.link.status == LinkStatus::Broken)
        .expect("lien cassé signalé");

    // Source, propriétaire (Option depuis §5.4) et cible absente.
    assert_eq!(broken.source.id, id(ORDER));
    assert_eq!(broken.source.object_type, ObjectType::ModelElement);
    let owner = broken
        .owner
        .as_ref()
        .expect("domaine responsable de la source");
    assert_eq!(owner.id, id(DOMAIN));
    assert_eq!(owner.object_type, ObjectType::Domain);
    assert_eq!(owner.name, "Paiements");
    assert_eq!(
        owner.href,
        format!("#/view/domains/{}", id(DOMAIN)),
        "accès au propriétaire depuis le diagnostic"
    );
    assert_eq!(broken.link.target_uri, uri(MISSING));
    assert!(broken.target.is_none(), "cible absente du référentiel");
    assert!(broken.link.target_object_id.is_none());

    // §5.4 : fenêtre de 80 caractères avant + 80 après, sur la ligne courante.
    assert!(
        broken.context.contains(MISSING),
        "extrait centré sur l'URI : {}",
        broken.context
    );
    assert!(
        !broken.context.contains('\n'),
        "extrait = une seule ligne Markdown : {}",
        broken.context
    );
    let bound = 80 + broken.link.target_uri.chars().count() + 80 + 2; // + « … »
    assert!(
        broken.context.chars().count() <= bound,
        "extrait borné ({} caractères pour une borne de {bound})",
        broken.context.chars().count()
    );
    assert!(
        broken.context.contains("Lien cassé volontaire"),
        "contenu avant l'occurrence conservé : {}",
        broken.context
    );

    // Tri §5.4 : (source.id, target_uri, link.id) ; les liens sains sont exclus.
    let pairs: Vec<(ObjectId, String, ObjectId)> = entries
        .iter()
        .map(|entry| {
            (
                entry.source.id,
                entry.link.target_uri.clone(),
                entry.link.id,
            )
        })
        .collect();
    assert!(
        pairs.windows(2).all(|window| window[0] <= window[1]),
        "tri par (source.id, target_uri, link.id) : {pairs:?}"
    );
    assert!(
        !entries
            .iter()
            .any(|entry| entry.link.status == LinkStatus::Ok),
        "un lien sain n'est pas un diagnostic"
    );
}

#[test]
fn broken_link_diagnostic_includes_archived() {
    let repo = fixture();
    let entries = repo.broken_links();

    let archived = entries
        .iter()
        .find(|entry| entry.link.status == LinkStatus::Archived)
        .expect("lien archivé signalé");

    assert_eq!(archived.source.id, id(ORDER));
    assert_eq!(
        archived.owner.as_ref().map(|owner| owner.id),
        Some(id(DOMAIN))
    );
    assert_eq!(archived.link.target_uri, uri(ARCHIVED));
    assert_eq!(
        archived.target.as_ref().map(|target| target.id),
        Some(id(ARCHIVED)),
        "la cible existe mais est archivée"
    );
    assert_eq!(archived.link.target_object_id, Some(id(ARCHIVED)));
    assert!(
        archived.context.contains(ARCHIVED),
        "extrait autour de l'URI archivée : {}",
        archived.context
    );

    // La cible reste lisible, en lecture seule : aucune correction automatique.
    let object = repo.element(id(ARCHIVED)).expect("objet archivé conservé");
    assert_eq!(object.object.lifecycle, Lifecycle::Archived);

    // §5.4 : le statut persisté était `OK` quand le lien a été écrit ; le
    // diagnostic recalcule à la lecture depuis le lifecycle courant.
    let cached = serde_json::from_str::<serde_json::Value>(DATASET).expect("fixture JSON lisible");
    let cached_status = cached["links"]
        .as_array()
        .expect("liens de fixture")
        .iter()
        .find(|link| link["target_uri"] == serde_json::json!(uri(ARCHIVED)))
        .map(|link| link["status"].as_str().unwrap_or("").to_string())
        .expect("lien archivé dans le cache");
    assert_eq!(cached_status, "OK", "cache périmé dans la fixture");

    assert_eq!(entries.len(), 2, "un cassé + un archivé, rien d'autre");

    // §5.4 : `stats().broken_links` doit utiliser le même recalcul.
    assert_eq!(
        repo.stats().broken_links,
        entries.len(),
        "statistique alignée sur le diagnostic recalculé"
    );
}

#[test]
fn broken_link_diagnostic_deduplicates() {
    let repo = fixture();
    let entries = repo.broken_links();

    // La documentation cite deux fois la même URI cassée, sur deux lignes.
    let matches: Vec<_> = entries
        .iter()
        .filter(|entry| entry.source.id == id(ORDER) && entry.link.target_uri == uri(MISSING))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "dédoublonnage par (source_object_id, target_uri)"
    );

    let entry = matches[0];
    assert!(
        entry.context.contains("Lien cassé volontaire"),
        "contexte = première occurrence : {}",
        entry.context
    );
    assert!(
        !entry.context.contains("Deuxième mention"),
        "la seconde occurrence ne remplace pas la première : {}",
        entry.context
    );

    // Le doublon ne gonfle ni la liste ni la statistique.
    assert_eq!(entries.len(), 2);
    assert_eq!(repo.stats().broken_links, 2);
}

#[test]
fn dependency_graph_shows_paths_and_depth() {
    let repo = fixture();
    let root = id(ORDER);
    let graph = repo
        .dependencies(root, Direction::Out, 3)
        .expect("graphe sortant");

    assert_eq!(graph.root, root);

    let node = |target: ObjectId| {
        graph
            .nodes
            .iter()
            .find(|node| node.object_id == target)
            .unwrap_or_else(|| panic!("nœud {target} attendu dans le graphe"))
    };

    let neighbor = node(id(PRICING));
    assert_eq!(neighbor.depth, 1);
    assert_eq!(
        neighbor.path,
        vec![root, id(PRICING)],
        "chemin depuis la racine"
    );
    assert_eq!(neighbor.relation, "serving");
    assert_eq!(neighbor.domain_name, "Paiements");
    assert_eq!(neighbor.lifecycle, Lifecycle::Active);
    assert_eq!(neighbor.direction, Direction::Out);

    let distant = node(id(GRID));
    assert_eq!(distant.depth, 3);
    assert_eq!(
        distant.path,
        vec![root, id(PRICING), id(TARIF), id(GRID)],
        "chemin complet, pas seulement l'arête"
    );

    assert!(graph.nodes.iter().all(|node| (1..=3).contains(&node.depth)));
    assert!(graph
        .nodes
        .iter()
        .all(|node| node.path.first() == Some(&root) && node.path.last() == Some(&node.object_id)));
    assert!(graph
        .nodes
        .iter()
        .all(|node| node.path.len() == node.depth + 1));

    let unique: BTreeSet<ObjectId> = graph.nodes.iter().map(|node| node.object_id).collect();
    assert_eq!(unique.len(), graph.nodes.len(), "aucun nœud dupliqué");

    // §5.9 : tri (depth, path).
    let keys: Vec<(usize, Vec<ObjectId>)> = graph
        .nodes
        .iter()
        .map(|node| (node.depth, node.path.clone()))
        .collect();
    assert!(
        keys.windows(2).all(|window| window[0] <= window[1]),
        "nœuds triés par (depth, path) : {keys:?}"
    );

    // La direction inverse la jointure.
    let inbound = repo
        .dependencies(id(PRICING), Direction::In, 1)
        .expect("graphe entrant");
    assert!(inbound.nodes.iter().any(|node| node.object_id == root));
    assert!(inbound
        .nodes
        .iter()
        .all(|node| node.direction == Direction::In));
}

#[test]
fn dependency_graph_is_bounded_to_six() {
    let repo = fixture();
    let root = id(ORDER);

    let deep = repo
        .dependencies(root, Direction::Out, 99)
        .expect("profondeur hors bornes acceptée puis ramenée à 6");
    assert!(
        deep.nodes.iter().all(|node| node.depth <= 6),
        "profondeur maximale 6 : {:?}",
        deep.nodes.iter().map(|node| node.depth).max()
    );
    assert_eq!(
        deep.nodes.iter().map(|node| node.depth).max(),
        Some(6),
        "la chaîne de huit relations est coupée au sixième niveau"
    );
    assert_eq!(deep.nodes.len(), 6, "un nœud par niveau, sans duplication");
    assert!(deep.truncated, "borne atteinte ⇒ truncated signalé");

    let shallow = repo
        .dependencies(root, Direction::Out, 0)
        .expect("profondeur basse ramenée à 1");
    assert_eq!(shallow.nodes.len(), 1);
    assert_eq!(shallow.nodes[0].object_id, id(PRICING));
    assert!(shallow.nodes.iter().all(|node| node.depth == 1));
}

#[test]
fn dependency_graph_both_direction_is_labelled() {
    let repo = fixture();
    let graph = repo
        .dependencies(id(PRICING), Direction::Both, 1)
        .expect("graphe bidirectionnel");

    let inbound = graph
        .nodes
        .iter()
        .find(|node| node.object_id == id(ORDER))
        .expect("prédécesseur inclus");
    assert_eq!(inbound.direction, Direction::In, "A → Pricing : entrant");

    let outbound = graph
        .nodes
        .iter()
        .find(|node| node.object_id == id(TARIF))
        .expect("successeur inclus");
    assert_eq!(outbound.direction, Direction::Out, "Pricing → C : sortant");

    assert!(
        graph
            .nodes
            .iter()
            .all(|node| node.path.first() == Some(&id(PRICING))),
        "chaque chemin part de la racine"
    );
    let mut sorted = graph.nodes.clone();
    sorted.sort_by(|a, b| (a.depth, &a.path).cmp(&(b.depth, &b.path)));
    assert_eq!(
        graph
            .nodes
            .iter()
            .map(|node| node.object_id)
            .collect::<Vec<_>>(),
        sorted.iter().map(|node| node.object_id).collect::<Vec<_>>(),
        "BOTH = union triée par (depth, path)"
    );
}

#[test]
fn dependency_graph_archived_target_is_flagged() {
    let repo = fixture();
    let graph = repo
        .dependencies(id(FACTURATION), Direction::Out, 1)
        .expect("graphe sortant de Facturation");

    let archived = graph
        .nodes
        .iter()
        .find(|node| node.object_id == id(ARCHIVED))
        .expect("cible archivée incluse");
    assert_eq!(
        archived.lifecycle,
        Lifecycle::Archived,
        "§5.9 : la cible archivée est signalée par son lifecycle"
    );
    assert_eq!(archived.direction, Direction::Out);
    assert_eq!(archived.depth, 1);
    assert_eq!(archived.path, vec![id(FACTURATION), id(ARCHIVED)]);
}
