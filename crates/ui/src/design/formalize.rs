//! Formalisation progressive : `SKETCH → FORMALIZED → VALID` (C-04).
//!
//! L'inspecteur ne construit jamais un objet : il construit une commande.

use archifier_core::{CommandError, DraftShape, ShapeKind, ViewCommandBody};
use serde_json::{Map, Value};

/// Assemble la commande `Formalize` à partir des choix de l'inspecteur.
///
/// `metaclass` et `domain` vides produisent `DSG-001`, avec le champ manquant.
pub fn formalize_body(
    draft: &DraftShape,
    metaclass: Option<String>,
    domain: Option<archifier_core::ObjectId>,
    version: Option<String>,
    new_object_id: archifier_core::ObjectId,
    new_version_id: Option<archifier_core::ObjectId>,
) -> Result<ViewCommandBody, CommandError> {
    let metaclass = metaclass.unwrap_or_default().trim().to_string();
    if metaclass.is_empty() {
        return Err(CommandError {
            code: "DSG-001".into(),
            path: "/metaclass".into(),
            message_key: "error.DSG-001".into(),
            detail: Some("METACLASS_REQUIRED".into()),
        });
    }
    let domain = domain.ok_or_else(|| CommandError {
        code: "DSG-001".into(),
        path: "/domain".into(),
        message_key: "error.DSG-001".into(),
        detail: Some("DOMAIN_REQUIRED".into()),
    })?;

    let mut properties = Map::new();
    if draft.shape == ShapeKind::Port {
        if let Some(version) = version.filter(|v| !v.trim().is_empty()) {
            properties.insert("version".into(), Value::String(version));
        }
        if draft.interface_id.is_some() {
            return Ok(ViewCommandBody::Formalize {
                draft_id: draft.id.clone(),
                metaclass,
                domain_id: domain,
                properties,
                new_object_id: None,
                new_version_id: None,
            });
        }
        let new_version_id = new_version_id.ok_or_else(|| CommandError {
            code: "DSG-004".into(),
            path: "/version".into(),
            message_key: "error.DSG-004".into(),
            detail: Some("VERSION_ID_REQUIRED".into()),
        })?;
        return Ok(ViewCommandBody::Formalize {
            draft_id: draft.id.clone(),
            metaclass,
            domain_id: domain,
            properties,
            new_object_id: Some(new_object_id),
            new_version_id: Some(new_version_id),
        });
    }

    Ok(ViewCommandBody::Formalize {
        draft_id: draft.id.clone(),
        metaclass,
        domain_id: domain,
        properties,
        new_object_id: Some(new_object_id),
        new_version_id: None,
    })
}
