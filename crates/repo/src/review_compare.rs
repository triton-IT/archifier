//! Comparaison de deux revues d'interfaces (T1-11).
//!
//! La comparaison est en lecture seule : elle recalcule les ensembles exposés,
//! consommés et les anomalies avec exactement les mêmes filtres que
//! `review_detail`, puis expose les ajouts et retraits entre deux revues.
//! Les fenêtres temporelles des deux revues sont conservées telles quelles.

use crate::{
    interface_row, Anomaly, AnomalyCode, Inner, InterfaceRow, ObjectRef, ObjectType, RepoError,
    RepoResult, Repository, ReviewComparison, TemporalWindow,
};
use archifier_core::{version_temporal_status, ObjectId, Review, TemporalStatus};
use std::collections::{BTreeSet, HashSet};
use time::OffsetDateTime;

impl Repository {
    /// Compare les ensembles d'interfaces et d'anomalies de deux revues.
    ///
    /// `REVIEW-001` uniquement si une revue est inconnue ou si l'objet n'est
    /// pas une revue. `from == to` produit une comparaison vide ; l'ordre
    /// inverse est autorisé et échange les ajouts/retraits.
    pub fn compare_reviews(&self, from: ObjectId, to: ObjectId) -> RepoResult<ReviewComparison> {
        let inner = self.inner.borrow();
        let from_review = find_review(&inner, from)?;
        let to_review = find_review(&inner, to)?;
        let now = self.now();
        let from_sets = review_sets(&inner, from_review, now);
        let to_sets = review_sets(&inner, to_review, now);

        Ok(ReviewComparison {
            from: review_ref(from_review),
            to: review_ref(to_review),
            window_from: window_of(from_review),
            window_to: window_of(to_review),
            filter_from: from_review.filter_snapshot.clone(),
            filter_to: to_review.filter_snapshot.clone(),
            exposed_added: rows_added(&from_sets.exposed, &to_sets.exposed),
            exposed_removed: rows_added(&to_sets.exposed, &from_sets.exposed),
            exposed_changed: rows_changed(&from_sets.exposed, &to_sets.exposed),
            consumed_added: rows_added(&from_sets.consumed, &to_sets.consumed),
            consumed_removed: rows_added(&to_sets.consumed, &from_sets.consumed),
            consumed_changed: rows_changed(&from_sets.consumed, &to_sets.consumed),
            anomalies_added: anomalies_added(&from_sets.anomalies, &to_sets.anomalies),
            anomalies_removed: anomalies_added(&to_sets.anomalies, &from_sets.anomalies),
        })
    }
}

struct ReviewSets {
    exposed: Vec<InterfaceRow>,
    consumed: Vec<InterfaceRow>,
    anomalies: Vec<Anomaly>,
}

fn find_review(inner: &Inner, id: ObjectId) -> RepoResult<&Review> {
    inner
        .data
        .reviews
        .iter()
        .find(|review| review.object.id == id)
        .ok_or_else(|| RepoError::new("REVIEW-001", format!("Revue introuvable : « {id} ».")))
}

fn review_ref(review: &Review) -> ObjectRef {
    let mut object = ObjectRef::of(
        ObjectType::Review,
        review.object.id,
        review.object.name.clone(),
    );
    object.subtitle = Some(crate::review_status_label(review.status).into());
    object
}

fn window_of(review: &Review) -> Option<TemporalWindow> {
    if review.temporal_window_start.is_none() && review.temporal_window_end.is_none() {
        return None;
    }
    Some(TemporalWindow {
        start: review.temporal_window_start,
        end: review.temporal_window_end,
    })
}

/// Mêmes filtres que `review_detail` : interfaces exposées et consommées du
/// domaine de la revue, anomalie d'expiration des versions consommées.
fn review_sets(inner: &Inner, review: &Review, now: OffsetDateTime) -> ReviewSets {
    let mut exposed: Vec<InterfaceRow> = inner
        .data
        .interfaces
        .iter()
        .filter(|interface| interface.provider_domain_id == review.domain_id)
        .map(|interface| interface_row(inner, interface, now))
        .collect();
    sort_rows(&mut exposed);

    let mut consumed_ids = BTreeSet::new();
    for consumer in &inner.data.interface_consumers {
        if consumer.consumer_domain_id != review.domain_id {
            continue;
        }
        if let Some(version) = inner
            .data
            .interface_versions
            .iter()
            .find(|version| version.id == consumer.interface_version_id)
        {
            consumed_ids.insert(version.interface_id);
        }
    }
    let mut consumed: Vec<InterfaceRow> = inner
        .data
        .interfaces
        .iter()
        .filter(|interface| consumed_ids.contains(&interface.object.id))
        .map(|interface| interface_row(inner, interface, now))
        .collect();
    sort_rows(&mut consumed);

    let mut anomalies = Vec::new();
    for consumer in &inner.data.interface_consumers {
        if consumer.consumer_domain_id != review.domain_id {
            continue;
        }
        if let Some(version) = inner
            .data
            .interface_versions
            .iter()
            .find(|version| version.id == consumer.interface_version_id)
        {
            if version_temporal_status(version, now) == Some(TemporalStatus::Expired) {
                let name = inner
                    .find_interface(version.interface_id)
                    .map(|interface| interface.object.name.clone())
                    .unwrap_or_default();
                anomalies.push(Anomaly {
                    code: AnomalyCode::DependencyOnExpiredInterface,
                    message: format!("{} {} est expirée.", name, version.semantic_version),
                    object_id: version.interface_id,
                    href: format!("#/view/interfaces/{}", version.interface_id),
                    detail: version.available_until.map(format_date),
                });
            }
        }
    }
    sort_anomalies(&mut anomalies);

    ReviewSets {
        exposed,
        consumed,
        anomalies,
    }
}

fn rows_added(reference: &[InterfaceRow], candidate: &[InterfaceRow]) -> Vec<InterfaceRow> {
    let known: HashSet<ObjectId> = reference.iter().map(|row| row.id).collect();
    let mut out: Vec<InterfaceRow> = candidate
        .iter()
        .filter(|row| !known.contains(&row.id))
        .cloned()
        .collect();
    sort_rows(&mut out);
    out
}

/// Lignes présentes des deux côtés mais différentes : la version `candidate`
/// est exposée, avec un tri identique aux ajouts et retraits.
fn rows_changed(reference: &[InterfaceRow], candidate: &[InterfaceRow]) -> Vec<InterfaceRow> {
    let mut out: Vec<InterfaceRow> = candidate
        .iter()
        .filter(|row| {
            reference
                .iter()
                .find(|known| known.id == row.id)
                .map(|known| known != *row)
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    sort_rows(&mut out);
    out
}

fn anomalies_added(reference: &[Anomaly], candidate: &[Anomaly]) -> Vec<Anomaly> {
    let mut out: Vec<Anomaly> = candidate
        .iter()
        .filter(|candidate| !reference.iter().any(|known| same_anomaly(known, candidate)))
        .cloned()
        .collect();
    sort_anomalies(&mut out);
    out
}

fn same_anomaly(a: &Anomaly, b: &Anomaly) -> bool {
    a.code == b.code && a.object_id == b.object_id && a.detail == b.detail
}

fn sort_rows(rows: &mut [InterfaceRow]) {
    rows.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn sort_anomalies(anomalies: &mut [Anomaly]) {
    anomalies.sort_by(|a, b| {
        a.code
            .as_str()
            .cmp(b.code.as_str())
            .then_with(|| a.object_id.cmp(&b.object_id))
            .then_with(|| a.message.cmp(&b.message))
    });
}

fn format_date(date: OffsetDateTime) -> String {
    format!(
        "{:02}/{:02}/{}",
        date.day(),
        u8::from(date.month()),
        date.year()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{seed, InterfaceFilter, MemoryStore};
    use std::rc::Rc;
    use time::macros::datetime;

    fn seeded() -> Repository {
        let repo = Repository::new(Rc::new(MemoryStore::new()), 1_795_000_000_000, 42);
        seed::seed_demo(&repo).expect("jeu de démonstration");
        repo
    }

    fn domain_id(repo: &Repository, name: &str) -> ObjectId {
        repo.domains()
            .into_iter()
            .find(|domain| domain.name == name)
            .expect("domaine du jeu de démonstration")
            .id
    }

    fn review_in(repo: &Repository, domain: ObjectId, title: &str) -> ObjectId {
        repo.create_review(domain, title).expect("revue")
    }

    fn interface_id(repo: &Repository, name: &str) -> ObjectId {
        repo.interfaces(&InterfaceFilter::default())
            .into_iter()
            .find(|row| row.name == name)
            .expect("interface du jeu de démonstration")
            .id
    }

    fn names(rows: &[InterfaceRow]) -> Vec<String> {
        rows.iter().map(|row| row.name.clone()).collect()
    }

    #[test]
    fn review_comparison_shows_filter_and_window_diff() {
        let repo = seeded();
        let pay = domain_id(&repo, "Paiements");
        let ord = domain_id(&repo, "Commande");
        let from = review_in(&repo, pay, "Paiements — départ");
        let to = review_in(&repo, ord, "Commande — arrivée");

        let start = datetime!(2026-10-01 0:00 UTC);
        let end = datetime!(2026-12-31 0:00 UTC);
        set_window(&repo, from, Some(start), Some(end));
        set_window(&repo, to, Some(start), None);

        let comparison = repo.compare_reviews(from, to).expect("comparaison");
        assert_eq!(comparison.from.id, from);
        assert_eq!(comparison.to.id, to);
        assert_eq!(
            comparison.window_from,
            Some(TemporalWindow {
                start: Some(start),
                end: Some(end),
            })
        );
        assert_eq!(
            comparison.window_to,
            Some(TemporalWindow {
                start: Some(start),
                end: None,
            })
        );

        assert_eq!(
            names(&comparison.exposed_removed),
            vec!["Payment API", "Refund API"]
        );
        assert!(comparison.exposed_added.is_empty());
        assert_eq!(
            names(&comparison.consumed_added),
            vec!["Payment API", "Refund API"]
        );
        assert!(comparison.consumed_removed.is_empty());
    }

    #[test]
    fn review_comparison_lists_anomalies() {
        let repo = seeded();
        let pay = domain_id(&repo, "Paiements");
        let ord = domain_id(&repo, "Commande");
        let from = review_in(&repo, pay, "Paiements");
        let to = review_in(&repo, ord, "Commande");
        let refund = interface_id(&repo, "Refund API");

        let comparison = repo.compare_reviews(from, to).expect("comparaison");
        assert_eq!(comparison.anomalies_added.len(), 1);
        assert_eq!(
            comparison.anomalies_added[0].code,
            AnomalyCode::DependencyOnExpiredInterface
        );
        assert_eq!(comparison.anomalies_added[0].object_id, refund);
        assert!(comparison.anomalies_removed.is_empty());

        // L'ordre inverse est autorisé : ajouts et retraits s'échangent.
        let reverse = repo
            .compare_reviews(to, from)
            .expect("comparaison inverse autorisée");
        assert!(reverse.anomalies_added.is_empty());
        assert_eq!(reverse.anomalies_removed.len(), 1);
        assert_eq!(reverse.anomalies_removed[0].object_id, refund);
    }

    #[test]
    fn review_comparison_rejects_unknown_review() {
        let repo = seeded();
        let pay = domain_id(&repo, "Paiements");
        let known = review_in(&repo, pay, "Connue");
        let unknown = ObjectId::from_parts(1, 9);

        let error = repo
            .compare_reviews(known, unknown)
            .expect_err("revue inconnue");
        assert!(error.is("REVIEW-001"), "{error}");

        let error = repo
            .compare_reviews(pay, known)
            .expect_err("objet qui n'est pas une revue");
        assert!(error.is("REVIEW-001"), "{error}");
    }

    #[test]
    fn same_review_is_empty_and_reverse_is_allowed() {
        let repo = seeded();
        let pay = domain_id(&repo, "Paiements");
        let ord = domain_id(&repo, "Commande");
        let first = review_in(&repo, pay, "Première");
        let second = review_in(&repo, ord, "Seconde");

        let same = repo
            .compare_reviews(first, first)
            .expect("même revue : comparaison vide");
        assert!(same.exposed_added.is_empty());
        assert!(same.exposed_removed.is_empty());
        assert!(same.exposed_changed.is_empty());
        assert!(same.consumed_added.is_empty());
        assert!(same.consumed_removed.is_empty());
        assert!(same.consumed_changed.is_empty());
        assert!(same.anomalies_added.is_empty());
        assert!(same.anomalies_removed.is_empty());

        let forward = repo.compare_reviews(first, second).expect("ordre direct");
        let reverse = repo
            .compare_reviews(second, first)
            .expect("ordre inverse autorisé");
        assert_eq!(
            reverse
                .consumed_added
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            forward
                .consumed_removed
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            reverse
                .consumed_removed
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            forward
                .consumed_added
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(reverse.window_from, forward.window_to);
        assert_eq!(reverse.window_to, forward.window_from);
    }

    #[test]
    fn comparison_rows_are_sorted_by_name() {
        let repo = seeded();
        let pay = domain_id(&repo, "Paiements");
        let ord = domain_id(&repo, "Commande");
        let from = review_in(&repo, pay, "Départ");
        let to = review_in(&repo, ord, "Arrivée");
        let comparison = repo.compare_reviews(from, to).expect("comparaison");
        let mut sorted = names(&comparison.exposed_removed);
        sorted.sort();
        assert_eq!(names(&comparison.exposed_removed), sorted);
    }

    /// Vérifie l'implémentation contre la fixture d'acceptation W1c-1.
    #[test]
    fn review_comparison_matches_w1c_fixture() {
        const DATASET: &str = include_str!("../../../fixtures/mvp2/w1c/reviews.json");
        let repo =
            Repository::from_json(Rc::new(MemoryStore::new()), DATASET, 1_795_000_000_000, 42)
                .expect("fixture W1c revues lisible");
        let from: ObjectId = "01K8ZP4N000000000000000040".parse().expect("ULID");
        let to: ObjectId = "01K8ZP4N000000000000000041".parse().expect("ULID");
        let payment: ObjectId = "01K8ZP4N000000000000000010".parse().expect("ULID");
        let refund: ObjectId = "01K8ZP4N000000000000000011".parse().expect("ULID");
        let legacy: ObjectId = "01K8ZP4N000000000000000012".parse().expect("ULID");
        let zebra: ObjectId = "01K8ZP4N000000000000000013".parse().expect("ULID");
        let shared: ObjectId = "01K8ZP4N000000000000000014".parse().expect("ULID");

        let snapshot = repo.export_json();
        let comparison = repo.compare_reviews(from, to).expect("comparaison");

        assert_eq!(
            comparison.window_from,
            Some(TemporalWindow {
                start: Some(datetime!(2026-01-01 0:00 UTC)),
                end: Some(datetime!(2026-06-30 0:00 UTC)),
            })
        );
        assert_eq!(
            comparison.window_to,
            Some(TemporalWindow {
                start: Some(datetime!(2026-07-01 0:00 UTC)),
                end: None,
            })
        );
        assert_eq!(
            comparison
                .exposed_removed
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            vec![payment]
        );
        assert_eq!(
            comparison
                .consumed_added
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            vec![legacy, payment]
        );
        assert_eq!(
            comparison
                .consumed_removed
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            vec![refund, zebra]
        );
        // Shared API est consommée des deux côtés mais dans deux versions
        // expirées différentes : l'identité `(code, object_id, detail)` ne
        // masque donc ni l'ajout ni le retrait (§7.4).
        assert_eq!(comparison.anomalies_added.len(), 2);
        assert_eq!(
            comparison
                .anomalies_added
                .iter()
                .map(|anomaly| anomaly.object_id)
                .collect::<Vec<_>>(),
            vec![legacy, shared]
        );
        assert_eq!(comparison.anomalies_removed.len(), 2);
        assert_eq!(
            comparison
                .anomalies_removed
                .iter()
                .map(|anomaly| anomaly.object_id)
                .collect::<Vec<_>>(),
            vec![refund, shared]
        );
        let added_shared = comparison
            .anomalies_added
            .iter()
            .find(|anomaly| anomaly.object_id == shared)
            .expect("anomalie Shared ajoutée");
        let removed_shared = comparison
            .anomalies_removed
            .iter()
            .find(|anomaly| anomaly.object_id == shared)
            .expect("anomalie Shared retirée");
        assert_ne!(added_shared.detail, removed_shared.detail);
        assert!(
            comparison.exposed_changed.is_empty() && comparison.consumed_changed.is_empty(),
            "aucune ligne commune ne diffère à now"
        );
        assert_eq!(
            comparison.filter_from,
            repo.review_detail(from)
                .expect("détail")
                .review
                .filter_snapshot
        );
        assert_eq!(
            comparison.filter_to,
            repo.review_detail(to)
                .expect("détail")
                .review
                .filter_snapshot
        );
        assert_eq!(repo.export_json(), snapshot, "la comparaison n'écrit pas");
    }

    fn set_window(
        repo: &Repository,
        review_id: ObjectId,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
    ) {
        let mut inner = repo.inner.borrow_mut();
        let review = inner
            .data
            .reviews
            .iter_mut()
            .find(|review| review.object.id == review_id)
            .expect("revue de test");
        review.temporal_window_start = start;
        review.temporal_window_end = end;
    }
}
