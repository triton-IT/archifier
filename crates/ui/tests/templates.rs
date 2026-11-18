//! C-05 : les sept modèles de vue sont valides, sans objet, et instanciables.

use archifier_core::{ObjectId, ShapeKind, ViewIntent};
use archifier_ui::templates;

const DOMAIN: ObjectId = ObjectId::from_parts(10, 1);

#[test]
fn seven_templates_load_and_match_intents() {
    for intent in ViewIntent::ALL {
        let raw = templates::raw(intent);
        let value: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(value["schema"], 1, "schéma de {}", intent.key());
        assert_eq!(
            value["intent"].as_str(),
            Some(intent.key()),
            "intention de {}",
            intent.key()
        );
        assert!(
            !value["layers"].as_array().unwrap().is_empty(),
            "bandes absentes de {}",
            intent.key()
        );
    }
}

#[test]
fn template_contains_no_object_reference() {
    for intent in ViewIntent::ALL {
        let raw = templates::raw(intent);
        assert!(!raw.contains("object_id"), "{}", intent.key());
        assert!(!raw.contains("relationship_id"), "{}", intent.key());
        assert!(!raw.contains("interface_id"), "{}", intent.key());
        assert!(!raw.contains("version_id"), "{}", intent.key());
        assert!(!raw.contains("\"domain_id\""), "{}", intent.key());
        assert!(!raw.contains("#0"), "{}", intent.key());
    }
}

#[test]
fn template_instantiation_creates_no_object_and_keeps_sketches() {
    for intent in ViewIntent::ALL {
        let layout = templates::layout(intent, DOMAIN);
        assert_eq!(layout.schema, 2);
        assert_eq!(layout.intent, Some(intent));
        assert!(layout.nodes.is_empty(), "{}", intent.key());
        assert!(layout.edges.is_empty(), "{}", intent.key());
        assert!(
            !layout.layers.is_empty(),
            "bandes absentes de {}",
            intent.key()
        );
        for zone in &layout.zones {
            if zone.kind == archifier_core::ZoneKind::Domain {
                assert_eq!(zone.domain_id, Some(DOMAIN));
            }
        }
        for draft in &layout.drafts {
            assert_eq!(draft.status, archifier_core::DraftStatus::Sketch);
            assert!(matches!(
                draft.shape,
                ShapeKind::Rect | ShapeKind::Port | ShapeKind::Rounded | ShapeKind::Sticky
            ));
        }
    }
}

#[test]
fn interface_review_has_ports_and_migration_has_clock() {
    let review = templates::layout(ViewIntent::InterfaceReview, DOMAIN);
    assert!(review.zones.iter().any(|zone| zone.boundary));
    assert!(review
        .drafts
        .iter()
        .any(|draft| draft.shape == ShapeKind::Port));
    let migration = templates::layout(ViewIntent::Migration, DOMAIN);
    assert_eq!(migration.intent, Some(ViewIntent::Migration));
}

#[test]
fn template_labels_are_resolved_not_keys() {
    let layout = templates::layout(ViewIntent::ApplicationLandscape, DOMAIN);
    for layer in &layout.layers {
        assert!(!layer.label.starts_with("layer."), "libellé non résolu");
    }
    for draft in &layout.drafts {
        assert!(
            !draft.label.starts_with("template."),
            "libellé non résolu : {}",
            draft.label
        );
    }
}
