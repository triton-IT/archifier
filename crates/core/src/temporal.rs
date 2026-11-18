//! Règles temporelles BR-001 à BR-012.
//!
//! Les fenêtres sont des intervalles **semi-ouverts** `[available_from, available_until)` :
//! une interface qui se termine le 1er janvier n'est plus active à cet instant.

use crate::model::{InterfaceVersion, PublicationStatus, TemporalStatus};
use time::OffsetDateTime;

/// Statut temporel d'une version, si sa fenêtre est définie.
pub fn temporal_status(
    from: Option<OffsetDateTime>,
    until: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> Option<TemporalStatus> {
    let (from, until) = (from?, until?);
    if now < from {
        Some(TemporalStatus::Scheduled)
    } else if now < until {
        Some(TemporalStatus::Active)
    } else {
        Some(TemporalStatus::Expired)
    }
}

pub fn version_temporal_status(
    v: &InterfaceVersion,
    now: OffsetDateTime,
) -> Option<TemporalStatus> {
    temporal_status(v.available_from, v.available_until, now)
}

/// Une version est « effective » à `now` si sa fenêtre la couvre et qu'elle
/// n'est plus en brouillon.
pub fn is_effective_at(v: &InterfaceVersion, now: OffsetDateTime) -> bool {
    if v.publication_status == PublicationStatus::Draft {
        return false;
    }
    matches!(
        version_temporal_status(v, now),
        Some(TemporalStatus::Active)
    )
}

/// Codes de règle violés par une publication (BR-001, BR-002).
pub fn publication_issues(v: &InterfaceVersion) -> Vec<&'static str> {
    let (from, until) = match (v.available_from, v.available_until) {
        (Some(f), Some(u)) => (f, u),
        _ => return vec!["BR-001"],
    };
    if until <= from {
        return vec!["BR-002"];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ObjectId;
    use serde_json::json;
    use time::macros::datetime;

    fn version() -> InterfaceVersion {
        InterfaceVersion {
            id: ObjectId::from_parts(1, 1),
            interface_id: ObjectId::from_parts(1, 2),
            semantic_version: "1.0.0".into(),
            revision: 1,
            publication_status: PublicationStatus::Published,
            created_at: datetime!(2026-01-01 00:00 UTC),
            created_by: "u".into(),
            published_at: Some(datetime!(2026-01-01 00:00 UTC)),
            available_from: Some(datetime!(2026-01-01 00:00 UTC)),
            available_until: Some(datetime!(2027-01-01 00:00 UTC)),
            breaking_change: false,
            supersedes_version_id: None,
            contract_snapshot: json!({}),
            contract_format: None,
            contract_raw: None,
            contract_hash: None,
        }
    }

    #[test]
    fn br_003_future_is_scheduled() {
        assert_eq!(
            version_temporal_status(&version(), datetime!(2025-01-01 00:00 UTC)),
            Some(TemporalStatus::Scheduled)
        );
    }

    #[test]
    fn br_005_active_inside_window() {
        assert_eq!(
            version_temporal_status(&version(), datetime!(2026-06-01 00:00 UTC)),
            Some(TemporalStatus::Active)
        );
    }

    #[test]
    fn br_006_expired_at_upper_bound() {
        // Intervalle semi-ouvert : l'instant de fin est déjà expiré.
        assert_eq!(
            version_temporal_status(&version(), datetime!(2027-01-01 00:00 UTC)),
            Some(TemporalStatus::Expired)
        );
    }

    #[test]
    fn br_002_rejects_inverted_window() {
        let mut v = version();
        v.available_from = Some(datetime!(2027-06-01 00:00 UTC));
        v.available_until = Some(datetime!(2026-06-01 00:00 UTC));
        assert_eq!(publication_issues(&v), vec!["BR-002"]);
    }

    #[test]
    fn br_001_requires_both_bounds() {
        let mut v = version();
        v.available_from = None;
        v.available_until = None;
        assert_eq!(publication_issues(&v), vec!["BR-001"]);
    }

    #[test]
    fn draft_is_never_effective() {
        let mut v = version();
        v.publication_status = PublicationStatus::Draft;
        assert!(!is_effective_at(&v, datetime!(2026-06-01 00:00 UTC)));
    }

    #[test]
    fn overlap_period_is_representable() {
        // BR-009 : v1 active jusqu'en juin, v2 à partir de mars.
        let mut v1 = version();
        v1.available_until = Some(datetime!(2027-06-01 00:00 UTC));
        let mut v2 = version();
        v2.semantic_version = "2.0.0".into();
        v2.available_from = Some(datetime!(2027-03-01 00:00 UTC));
        v2.available_until = Some(datetime!(2028-03-01 00:00 UTC));

        let during = datetime!(2027-04-01 00:00 UTC);
        assert!(is_effective_at(&v1, during));
        assert!(is_effective_at(&v2, during));
    }
}
