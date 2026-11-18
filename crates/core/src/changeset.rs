//! ChangeSets : proposition, validation déterministe, application.
//!
//! Un ChangeSet ne contourne jamais les règles : il est validé par les mêmes
//! fonctions que les écritures directes, et n'est appliqué qu'après confirmation.

use crate::ids::ObjectId;
use crate::model::{DiagramNotation, InterfaceCategory, Notation, Revision};
use crate::validation::{ValidationError, ValidationIssue};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChangeOrigin {
    Ai,
    User,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChangeSetStatus {
    Draft,
    Validated,
    Applied,
    Rejected,
    Stale,
}

/// Référence à un objet : soit un identifiant réel, soit un `temp_id` créé
/// plus haut dans le même ChangeSet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectRef(pub String);

impl ObjectRef {
    pub fn is_temp(&self) -> bool {
        self.0.parse::<ObjectId>().is_err()
    }

    pub fn as_id(&self) -> Option<ObjectId> {
        self.0.parse().ok()
    }

    pub fn temp(temp_id: &str) -> Self {
        Self(temp_id.to_string())
    }

    pub fn id(id: ObjectId) -> Self {
        Self(id.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum ChangeOperation {
    CreateElement {
        temp_id: String,
        domain_id: ObjectId,
        notation: Notation,
        metaclass: String,
        name: String,
        #[serde(default)]
        metamodel_version: Option<String>,
        #[serde(default)]
        properties: Map<String, Value>,
    },
    CreateRelationship {
        temp_id: String,
        domain_id: ObjectId,
        source_id: ObjectRef,
        target_id: ObjectRef,
        relation_type: String,
    },
    CreateDiagram {
        temp_id: String,
        domain_id: ObjectId,
        name: String,
        notation: DiagramNotation,
    },
    CreateStory {
        temp_id: String,
        domain_id: ObjectId,
        name: String,
    },
    CreateInterface {
        temp_id: String,
        domain_id: ObjectId,
        name: String,
        logical_interface_key: String,
        category: InterfaceCategory,
    },
    CreateInterfaceVersion {
        temp_id: String,
        interface_id: ObjectId,
        semantic_version: String,
        #[serde(with = "time::serde::rfc3339")]
        available_from: OffsetDateTime,
        #[serde(with = "time::serde::rfc3339")]
        available_until: OffsetDateTime,
    },
    UpdateDocumentation {
        object_id: ObjectId,
        markdown: String,
    },
    UpdateObject {
        object_id: ObjectId,
        patch: Value,
    },
    UpdateDiagramLayout {
        diagram_id: ObjectId,
        patch: Value,
    },
    ArchiveObject {
        object_id: ObjectId,
    },
}

impl ChangeOperation {
    pub fn target(&self) -> Option<ObjectId> {
        match self {
            Self::CreateElement { .. }
            | Self::CreateRelationship { .. }
            | Self::CreateDiagram { .. }
            | Self::CreateStory { .. }
            | Self::CreateInterface { .. }
            | Self::CreateInterfaceVersion { .. } => None,
            Self::UpdateDocumentation { object_id, .. }
            | Self::UpdateObject { object_id, .. }
            | Self::ArchiveObject { object_id } => Some(*object_id),
            Self::UpdateDiagramLayout { diagram_id, .. } => Some(*diagram_id),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::CreateElement {
                name, metaclass, ..
            } => format!("Créer l'élément « {name} » ({metaclass})"),
            Self::CreateRelationship { relation_type, .. } => {
                format!("Créer la relation {relation_type}")
            }
            Self::CreateDiagram { name, .. } => format!("Créer le diagramme « {name} »"),
            Self::CreateStory { name, .. } => format!("Créer la story « {name} »"),
            Self::CreateInterface { name, .. } => format!("Créer l'interface « {name} »"),
            Self::CreateInterfaceVersion {
                semantic_version, ..
            } => {
                format!("Publier la version {semantic_version}")
            }
            Self::UpdateDocumentation { .. } => "Mettre à jour la documentation".into(),
            Self::UpdateObject { .. } => "Mettre à jour un objet".into(),
            Self::UpdateDiagramLayout { .. } => "Mettre à jour une vue".into(),
            Self::ArchiveObject { .. } => "Archiver un objet".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSet {
    pub id: ObjectId,
    pub origin: ChangeOrigin,
    pub reason: String,
    pub operations: Vec<ChangeOperation>,
    #[serde(default)]
    pub base_revisions: BTreeMap<ObjectId, Revision>,
    pub status: ChangeSetStatus,
}

/// Valide un ChangeSet sans rien écrire (FR-025, NFR-004).
pub fn validate_changeset(
    cs: &ChangeSet,
    exists: &dyn Fn(ObjectId) -> bool,
) -> Result<(), ValidationError> {
    let mut issues = Vec::new();
    let mut created: HashSet<&str> = HashSet::new();

    for (index, op) in cs.operations.iter().enumerate() {
        if let ChangeOperation::CreateElement { temp_id, .. }
        | ChangeOperation::CreateRelationship { temp_id, .. }
        | ChangeOperation::CreateDiagram { temp_id, .. }
        | ChangeOperation::CreateStory { temp_id, .. }
        | ChangeOperation::CreateInterface { temp_id, .. }
        | ChangeOperation::CreateInterfaceVersion { temp_id, .. } = op
        {
            if !created.insert(temp_id.as_str()) {
                issues.push(ValidationIssue::error(
                    "CS-003",
                    format!("/operations/{index}/tempId"),
                    format!("L'identifiant temporaire « {temp_id} » est déjà utilisé."),
                ));
            }
        }

        if let Some(target) = op.target() {
            check_ref(&ObjectRef::id(target), &created, exists, index, &mut issues);
        }

        match op {
            ChangeOperation::CreateRelationship {
                source_id,
                target_id,
                ..
            } => {
                check_ref(source_id, &created, exists, index, &mut issues);
                check_ref(target_id, &created, exists, index, &mut issues);
                if source_id == target_id {
                    issues.push(ValidationIssue::error(
                        "MM-REL-002",
                        format!("/operations/{index}"),
                        "Une relation ne peut pas relier un objet à lui-même.",
                    ));
                }
            }
            ChangeOperation::CreateInterfaceVersion { interface_id, .. } => {
                check_ref(
                    &ObjectRef::id(*interface_id),
                    &created,
                    exists,
                    index,
                    &mut issues,
                );
            }
            _ => {}
        }
    }

    if cs.reason.trim().is_empty() {
        issues.push(ValidationIssue::error(
            "CS-004",
            "/reason",
            "Un ChangeSet doit expliquer sa raison d'être.",
        ));
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::new(issues))
    }
}

fn check_ref(
    reference: &ObjectRef,
    created: &HashSet<&str>,
    exists: &dyn Fn(ObjectId) -> bool,
    index: usize,
    issues: &mut Vec<ValidationIssue>,
) {
    match reference.as_id() {
        Some(id) if exists(id) => {}
        Some(_) => issues.push(ValidationIssue::error(
            "CS-001",
            format!("/operations/{index}"),
            format!("L'objet {} est introuvable.", reference.0),
        )),
        None => {
            if !created.contains(reference.0.as_str()) {
                issues.push(ValidationIssue::error(
                    "CS-002",
                    format!("/operations/{index}"),
                    format!(
                        "La référence « {} » est créée après son utilisation.",
                        reference.0
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cs(ops: Vec<ChangeOperation>) -> ChangeSet {
        ChangeSet {
            id: ObjectId::from_parts(1, 1),
            origin: ChangeOrigin::Ai,
            reason: "Proposition issue de la demande utilisateur".into(),
            operations: ops,
            base_revisions: BTreeMap::new(),
            status: ChangeSetStatus::Draft,
        }
    }

    #[test]
    fn accepts_creation_chain_with_temp_ids() {
        let domain = ObjectId::from_parts(1, 2);
        let changeset = cs(vec![
            ChangeOperation::CreateElement {
                temp_id: "tmp-1".into(),
                domain_id: domain,
                notation: Notation::Archimate,
                metaclass: "ApplicationComponent".into(),
                name: "Pricing".into(),
                metamodel_version: None,
                properties: Map::new(),
            },
            ChangeOperation::CreateRelationship {
                temp_id: "tmp-2".into(),
                domain_id: domain,
                source_id: ObjectRef::temp("tmp-1"),
                target_id: ObjectRef::temp("tmp-1"),
                relation_type: "association".into(),
            },
        ]);
        let err = validate_changeset(&changeset, &|_| false).unwrap_err();
        // La chaîne est valide, seule l'auto-référence est refusée.
        assert_eq!(err.codes(), vec!["MM-REL-002"]);
    }

    #[test]
    fn cs_001_unknown_id() {
        let changeset = cs(vec![ChangeOperation::ArchiveObject {
            object_id: ObjectId::from_parts(9, 9),
        }]);
        let err = validate_changeset(&changeset, &|_| false).unwrap_err();
        assert_eq!(err.codes(), vec!["CS-001"]);
    }

    #[test]
    fn cs_002_forward_reference() {
        let domain = ObjectId::from_parts(1, 2);
        let changeset = cs(vec![ChangeOperation::CreateRelationship {
            temp_id: "tmp-2".into(),
            domain_id: domain,
            source_id: ObjectRef::temp("tmp-9"),
            target_id: ObjectRef::temp("tmp-9"),
            relation_type: "association".into(),
        }]);
        let err = validate_changeset(&changeset, &|_| false).unwrap_err();
        assert!(err.codes().contains(&"CS-002"), "{:?}", err.codes());
    }

    #[test]
    fn cs_003_duplicate_temp_id() {
        let domain = ObjectId::from_parts(1, 2);
        let mk = || ChangeOperation::CreateStory {
            temp_id: "tmp-1".into(),
            domain_id: domain,
            name: "Parcours".into(),
        };
        let changeset = cs(vec![mk(), mk()]);
        let err = validate_changeset(&changeset, &|_| false).unwrap_err();
        assert!(err.codes().contains(&"CS-003"));
    }

    #[test]
    fn serialises_with_op_tag() {
        let changeset = cs(vec![ChangeOperation::UpdateDocumentation {
            object_id: ObjectId::from_parts(1, 1),
            markdown: "# Doc".into(),
        }]);
        let value = serde_json::to_value(&changeset).unwrap();
        assert_eq!(value["operations"][0]["op"], json!("updateDocumentation"));
        let back: ChangeSet = serde_json::from_value(value).unwrap();
        assert_eq!(back.operations.len(), 1);
    }
}
