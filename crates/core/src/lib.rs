//! Archifier Core — moteur de domaine sans I/O.
//!
//! Ce crate est compilé à l'identique côté serveur (lot 2) et dans le bundle
//! WebAssembly du front. Il ne doit jamais dépendre de `sqlx`, `axum`, `leptos`
//! ni d'une horloge implicite.

pub mod catalog;
pub mod changeset;
pub mod contract_diff;
pub mod export;
pub mod history_diff;
pub mod ids;
pub mod map;
pub mod markdown;
pub mod model;
pub mod openapi;
pub mod story;
pub mod temporal;
pub mod uri;
pub mod validation;
pub mod view_command;

pub use catalog::{
    product_catalog, ProductCard, ProductCatalog, ProductGap, ProductQuery, ProductSort,
};
pub use changeset::{ChangeOperation, ChangeOrigin, ChangeSet, ChangeSetStatus, ObjectRef};
pub use contract_diff::*;
pub use export::{export_domain, export_object, ExportBundle, ExportError, ExportFile};
pub use history_diff::{diff_values, FieldDifference};
pub use ids::{IdGenerator, ObjectId};
pub use model::*;
pub use openapi::*;
pub use story::{
    apply_framing, effective_zoom, frame_layout, FramedDiagram, FramedNode, DEFAULT_ZOOM,
    DIMMED_OPACITY, MAX_ZOOM, MIN_ZOOM,
};
pub use temporal::{is_effective_at, temporal_status, version_temporal_status};
pub use uri::{extract_uris, ArchKind, ArchUri};
pub use validation::{
    slugify, validate_diagram, validate_domain_name, validate_interface,
    validate_interface_version, validate_object, validate_story, Severity, ValidationError,
    ValidationIssue,
};
pub use view_command::{
    apply as apply_view_command, category_from, default_intent_for, logical_key,
    migrate_layout_v1_to_v2, validate_layout_shape, CommandError, CommandReceipt, CommandWarning,
    EndPoint, MigrationNote, ObjectKind, PropertyBag, RepoEffect, Target, ViewCommand,
    ViewCommandBody, ViewWorld,
};

pub const APP_NAME: &str = "Archifier";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Profil de métamodèle par défaut pour chaque notation.
pub fn default_profile_for(notation: Notation) -> &'static str {
    match notation {
        Notation::Archimate => "archimate:3.2",
        Notation::Bpmn => "bpmn:2.0.2-core",
        Notation::Uml => "uml:2.5.1-static",
        Notation::Native => "native:1.0",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profiles_are_named() {
        assert_eq!(default_profile_for(Notation::Archimate), "archimate:3.2");
        assert_eq!(default_profile_for(Notation::Bpmn), "bpmn:2.0.2-core");
        assert_eq!(default_profile_for(Notation::Uml), "uml:2.5.1-static");
        assert_eq!(default_profile_for(Notation::Native), "native:1.0");
    }
}
