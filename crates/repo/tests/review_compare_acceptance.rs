//! Tests bloquants W1c-1 — comparaison de deux revues (T1-11).
//!
//! Contrat : `docs/w1c-contract.md` §3 et §4. Fixture :
//! `fixtures/mvp2/w1c/reviews.json`.
//!
//! Deux revues de domaines différents :
//!
//! | Revue | Domaine | Fenêtre | Exposées | Consommées | Anomalies |
//! |---|---|---|---|---|---|
//! | `Revue Paiements T1` | Paiements | 2026-01-01 → 2026-06-30 | Payment API | Refund API, Zebra API | Refund expirée |
//! | `Revue Commande T2` | Commande | 2026-07-01 → ∞ | — | Legacy API, Payment API | Legacy expirée |
//!
//! La comparaison `from → to` doit donc rendre : exposées retirées Payment ;
//! consommées ajoutées Legacy puis Payment ; consommées retirées Refund puis
//! Zebra ; une anomalie ajoutée (Legacy) et une retirée (Refund).
//!
//! Les ensembles sont ceux des `ReviewDetail` existants : les filtres ne sont
//! pas réinterprétés par la comparaison.

use archifier_core::{ObjectId, ObjectType};
use archifier_repo::types::{AnomalyCode, InterfaceRow};
use archifier_repo::{MemoryStore, Repository, TemporalWindow};
use std::collections::BTreeSet;
use std::rc::Rc;
use time::macros::datetime;

const NOW_MS: u64 = 1_795_000_000_000; // 2026-11-18, horloge figée
const DATASET: &str = include_str!("../../../fixtures/mvp2/w1c/reviews.json");

const DOMAIN_PAIEMENTS: &str = "01K8ZP4N000000000000000001";
const DOMAIN_COMMANDE: &str = "01K8ZP4N000000000000000002";
const PAYMENT: &str = "01K8ZP4N000000000000000010";
const REFUND: &str = "01K8ZP4N000000000000000011";
const LEGACY: &str = "01K8ZP4N000000000000000012";
const ZEBRA: &str = "01K8ZP4N000000000000000013";
/// Interface expirée dans les deux revues, mais avec une date de fin différente.
const SHARED: &str = "01K8ZP4N000000000000000014";
const FROM: &str = "01K8ZP4N000000000000000040";
const TO: &str = "01K8ZP4N000000000000000041";
/// Même domaine que `FROM`, fenêtre à fin seule et filtre différent.
const SAME_DOMAIN: &str = "01K8ZP4N000000000000000042";
/// ULID bien formé mais absent du référentiel.
const MISSING: &str = "01K8ZP4N0000000000000000ZZ";

const FILTER_FROM: &str = r#"{"view":"EXPOSED","category":"APPLICATION","status":"ACTIVE"}"#;
const FILTER_TO: &str = r#"{"view":"CONSUMED","category":"APPLICATION","status":"EXPIRED"}"#;
const FILTER_SAME_DOMAIN: &str = r#"{"view":"CONSUMED","category":"TECHNICAL","status":"ACTIVE"}"#;

fn fixture() -> Repository {
    Repository::from_json(Rc::new(MemoryStore::new()), DATASET, NOW_MS, 42)
        .expect("fixture W1c revues lisible")
}

fn id(raw: &str) -> ObjectId {
    raw.parse().expect("ULID de fixture valide")
}

fn filter(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).expect("filtre de fixture valide")
}

fn row_ids(rows: &[InterfaceRow]) -> Vec<ObjectId> {
    rows.iter().map(|row| row.id).collect()
}

/// Diff d'ensembles par identifiant, trié par nom puis identifiant (§3).
fn subtract(left: &[InterfaceRow], right: &[InterfaceRow]) -> Vec<ObjectId> {
    let excluded: BTreeSet<ObjectId> = right.iter().map(|row| row.id).collect();
    let mut rows: Vec<&InterfaceRow> = left
        .iter()
        .filter(|row| !excluded.contains(&row.id))
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    rows.into_iter().map(|row| row.id).collect()
}

#[test]
fn review_comparison_shows_filter_and_window_diff() {
    let repo = fixture();
    let snapshot = repo.export_json();

    let comparison = repo
        .compare_reviews(id(FROM), id(TO))
        .expect("comparaison de deux revues connues");

    assert_eq!(comparison.from.id, id(FROM));
    assert_eq!(comparison.from.object_type, ObjectType::Review);
    assert_eq!(comparison.from.name, "Revue Paiements T1");
    assert_eq!(comparison.from.href, format!("#/view/reviews/{}", id(FROM)));
    assert_eq!(comparison.to.id, id(TO));
    assert_eq!(comparison.to.object_type, ObjectType::Review);
    assert_eq!(comparison.to.name, "Revue Commande T2");

    // Fenêtres temporelles distinctes, borne de fin optionnelle.
    assert_eq!(
        comparison.window_from,
        Some(TemporalWindow {
            start: Some(datetime!(2026-01-01 00:00 UTC)),
            end: Some(datetime!(2026-06-30 00:00 UTC)),
        })
    );
    assert_eq!(
        comparison.window_to,
        Some(TemporalWindow {
            start: Some(datetime!(2026-07-01 00:00 UTC)),
            end: None,
        })
    );

    // Les instantanés de filtres sont conservés tels quels (§7.4).
    assert_eq!(comparison.filter_from, filter(FILTER_FROM));
    assert_eq!(comparison.filter_to, filter(FILTER_TO));

    // Aucune ligne commune n'a changé : les deux revues sont vues au même `now`.
    assert!(comparison.exposed_changed.is_empty());
    assert!(comparison.consumed_changed.is_empty());

    // Les ensembles comparés sont exactement ceux des ReviewDetail (§3).
    let detail_from = repo.review_detail(id(FROM)).expect("revue source");
    let detail_to = repo.review_detail(id(TO)).expect("revue cible");

    assert_eq!(
        row_ids(&comparison.exposed_added),
        subtract(&detail_to.exposed, &detail_from.exposed)
    );
    assert_eq!(
        row_ids(&comparison.exposed_removed),
        subtract(&detail_from.exposed, &detail_to.exposed)
    );
    assert_eq!(
        row_ids(&comparison.consumed_added),
        subtract(&detail_to.consumed, &detail_from.consumed)
    );
    assert_eq!(
        row_ids(&comparison.consumed_removed),
        subtract(&detail_from.consumed, &detail_to.consumed)
    );

    // Attendu explicite de la fixture, et tri stable par nom puis id.
    assert!(comparison.exposed_added.is_empty());
    assert_eq!(row_ids(&comparison.exposed_removed), vec![id(PAYMENT)]);
    assert_eq!(
        row_ids(&comparison.consumed_added),
        vec![id(LEGACY), id(PAYMENT)],
        "Legacy API avant Payment API"
    );
    assert_eq!(
        row_ids(&comparison.consumed_removed),
        vec![id(REFUND), id(ZEBRA)],
        "Refund API avant Zebra API"
    );
    let names: Vec<&str> = comparison
        .consumed_added
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(names, vec!["Legacy API", "Payment API"]);
    assert_eq!(
        comparison.consumed_added[1].provider_domain_id,
        id(DOMAIN_PAIEMENTS)
    );
    assert_eq!(
        comparison.consumed_removed[0].provider_domain_id,
        id("01K8ZP4N000000000000000003")
    );

    assert_eq!(repo.export_json(), snapshot, "la comparaison n'écrit pas");

    // Comparaison inverse : les listes et les fenêtres s'échangent.
    let reverse = repo
        .compare_reviews(id(TO), id(FROM))
        .expect("comparaison inverse");
    assert_eq!(
        row_ids(&reverse.consumed_added),
        row_ids(&comparison.consumed_removed)
    );
    assert_eq!(
        row_ids(&reverse.consumed_removed),
        row_ids(&comparison.consumed_added)
    );
    assert_eq!(
        row_ids(&reverse.exposed_added),
        row_ids(&comparison.exposed_removed)
    );
    assert_eq!(reverse.window_from, comparison.window_to);
    assert_eq!(reverse.window_to, comparison.window_from);
    assert_eq!(repo.export_json(), snapshot);
}

#[test]
fn review_comparison_same_domain_exposes_window_and_filter_diff() {
    let repo = fixture();
    let comparison = repo
        .compare_reviews(id(FROM), id(SAME_DOMAIN))
        .expect("deux revues du même domaine");

    // Même domaine : aucun ajout ni retrait d'interface.
    assert!(comparison.exposed_added.is_empty());
    assert!(comparison.exposed_removed.is_empty());
    assert!(comparison.consumed_added.is_empty());
    assert!(comparison.consumed_removed.is_empty());
    assert!(comparison.exposed_changed.is_empty());
    assert!(comparison.consumed_changed.is_empty());

    // Fenêtres différentes, dont une fenêtre à fin seule.
    assert_eq!(
        comparison.window_from,
        Some(TemporalWindow {
            start: Some(datetime!(2026-01-01 00:00 UTC)),
            end: Some(datetime!(2026-06-30 00:00 UTC)),
        })
    );
    assert_eq!(
        comparison.window_to,
        Some(TemporalWindow {
            start: None,
            end: Some(datetime!(2026-09-30 00:00 UTC)),
        }),
        "une fenêtre avec seulement une fin est conservée"
    );
    assert_ne!(comparison.window_from, comparison.window_to);

    // Filtres différents, exposés bruts pour le sélecteur UI.
    assert_eq!(comparison.filter_from, filter(FILTER_FROM));
    assert_eq!(comparison.filter_to, filter(FILTER_SAME_DOMAIN));
    assert_ne!(comparison.filter_from, comparison.filter_to);

    // Même domaine et même `now` : mêmes anomalies des deux côtés.
    assert!(comparison.anomalies_added.is_empty());
    assert!(comparison.anomalies_removed.is_empty());
}

#[test]
fn review_comparison_lists_anomalies() {
    let repo = fixture();
    let snapshot = repo.export_json();
    let comparison = repo
        .compare_reviews(id(FROM), id(TO))
        .expect("comparaison de deux revues connues");

    assert_eq!(
        comparison.anomalies_added.len(),
        2,
        "Legacy et Shared côté cible : {:?}",
        comparison.anomalies_added
    );
    let added_legacy = comparison
        .anomalies_added
        .iter()
        .find(|anomaly| anomaly.object_id == id(LEGACY))
        .expect("Legacy expirée");
    assert_eq!(added_legacy.code, AnomalyCode::DependencyOnExpiredInterface);
    assert_eq!(
        added_legacy.href,
        format!("#/view/interfaces/{}", id(LEGACY))
    );
    assert!(added_legacy.message.contains("Legacy API"));
    assert!(added_legacy.detail.is_some());
    let added_shared = comparison
        .anomalies_added
        .iter()
        .find(|anomaly| anomaly.object_id == id(SHARED))
        .expect("Shared expirée côté cible");
    assert!(added_shared.detail.is_some());

    assert_eq!(
        comparison.anomalies_removed.len(),
        2,
        "Refund et Shared côté source : {:?}",
        comparison.anomalies_removed
    );
    let removed_refund = comparison
        .anomalies_removed
        .iter()
        .find(|anomaly| anomaly.object_id == id(REFUND))
        .expect("Refund expirée");
    assert!(removed_refund.detail.is_some());
    let removed_shared = comparison
        .anomalies_removed
        .iter()
        .find(|anomaly| anomaly.object_id == id(SHARED))
        .expect("Shared expirée côté source");
    assert!(removed_shared.detail.is_some());

    // L'identité inclut `detail` : deux versions expirées de Shared API ne se
    // masquent pas, la même interface apparaît des deux côtés avec deux dates.
    assert_ne!(added_shared.detail, removed_shared.detail);
    let keys: BTreeSet<(String, ObjectId, String)> = comparison
        .anomalies_added
        .iter()
        .chain(comparison.anomalies_removed.iter())
        .map(|anomaly| {
            (
                anomaly.code.as_str().to_string(),
                anomaly.object_id,
                anomaly.detail.clone().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(keys.len(), 4, "aucune anomalie masquée par la clé courte");
    assert_eq!(
        keys.iter()
            .filter(|(_, object, _)| *object == id(SHARED))
            .count(),
        2,
        "Shared API présente dans les deux sens avec deux détails"
    );

    // Les anomalies viennent bien des ReviewDetail, pas d'un recalcul divergent.
    let detail_from = repo.review_detail(id(FROM)).expect("revue source");
    let detail_to = repo.review_detail(id(TO)).expect("revue cible");
    assert_eq!(detail_from.anomalies.len(), 2);
    assert!(detail_from
        .anomalies
        .iter()
        .any(|a| a.object_id == id(REFUND)));
    assert!(detail_from
        .anomalies
        .iter()
        .any(|a| a.object_id == id(SHARED)));
    assert_eq!(detail_to.anomalies.len(), 2);
    assert!(detail_to
        .anomalies
        .iter()
        .any(|a| a.object_id == id(LEGACY)));
    assert!(detail_to
        .anomalies
        .iter()
        .any(|a| a.object_id == id(SHARED)));

    // La comparaison inverse échange ajouts et retraits.
    let reverse = repo
        .compare_reviews(id(TO), id(FROM))
        .expect("comparaison inverse");
    assert_eq!(reverse.anomalies_added.len(), 2);
    let reverse_added_refund = reverse
        .anomalies_added
        .iter()
        .find(|a| a.object_id == id(REFUND))
        .expect("Refund côté source");
    assert!(reverse_added_refund.detail.is_some());
    let reverse_added_shared = reverse
        .anomalies_added
        .iter()
        .find(|a| a.object_id == id(SHARED))
        .expect("Shared côté source");
    assert_eq!(
        reverse_added_shared.detail, removed_shared.detail,
        "le détail de la version source est conservé"
    );
    assert_eq!(reverse.anomalies_removed.len(), 2);
    assert!(reverse
        .anomalies_removed
        .iter()
        .any(|a| a.object_id == id(LEGACY)));
    let reverse_removed_shared = reverse
        .anomalies_removed
        .iter()
        .find(|a| a.object_id == id(SHARED))
        .expect("Shared côté cible");
    assert_eq!(
        reverse_removed_shared.detail, added_shared.detail,
        "le détail de la version cible est conservé"
    );

    assert_eq!(repo.export_json(), snapshot, "aucune écriture");
}

#[test]
fn review_comparison_rejects_unknown_review() {
    let repo = fixture();
    let snapshot = repo.export_json();

    let error = repo
        .compare_reviews(id(FROM), id(MISSING))
        .expect_err("revue cible inconnue");
    assert!(error.is("REVIEW-001"), "{error}");

    let error = repo
        .compare_reviews(id(MISSING), id(TO))
        .expect_err("revue source inconnue");
    assert!(error.is("REVIEW-001"), "{error}");

    let error = repo
        .compare_reviews(id(MISSING), id(MISSING))
        .expect_err("deux revues inconnues");
    assert!(error.is("REVIEW-001"), "{error}");

    // Un objet existant qui n'est pas une revue est aussi une revue inconnue,
    // des deux côtés de la comparaison.
    let error = repo
        .compare_reviews(id(DOMAIN_COMMANDE), id(TO))
        .expect_err("objet non-revue en source");
    assert!(error.is("REVIEW-001"), "{error}");

    let error = repo
        .compare_reviews(id(FROM), id(DOMAIN_COMMANDE))
        .expect_err("objet non-revue en cible");
    assert!(error.is("REVIEW-001"), "{error}");

    assert_eq!(repo.export_json(), snapshot, "refus sans écriture");
}
