//! Diff de contrats OpenAPI et classification des ruptures (W1a).
//!
//! Les changements suivent les codes du contrat W1a §5. Le diff travaille sur
//! le modèle normalisé (`InterfaceVersion.contract_snapshot`), ce qui permet de
//! comparer une nouvelle analyse à une version déjà stockée sans relire le
//! document brut d'origine.
//!
//! Interprétation des codes structurels : le contrat ne prévoit pas de code
//! dédié aux évolutions compatibles de structure (opération ajoutée, champ
//! optionnel retiré). Elles sont donc rapportées sous `OAS-021` avec un
//! message explicite, et les restrictions de schéma sous `OAS-014`.

use crate::openapi::ParsedContract;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiffClass {
    Compatible,
    Breaking,
    Unknown,
}

impl DiffClass {
    fn rank(self) -> u8 {
        match self {
            Self::Compatible => 0,
            Self::Unknown => 1,
            Self::Breaking => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractChange {
    /// `OAS-010`, `OAS-012`, …
    pub code: String,
    /// Chemin stable dans le document normalisé, ex. `/paths/~1customers/get`.
    pub path: String,
    pub message: String,
    pub class: DiffClass,
}

impl ContractChange {
    pub fn new(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
        class: DiffClass,
    ) -> Self {
        Self {
            code: code.into(),
            path: path.into(),
            message: message.into(),
            class,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractDiff {
    pub changes: Vec<ContractChange>,
    pub classification: DiffClass,
    pub requires_confirmation: bool,
}

impl ContractDiff {
    /// Agrège des changements : tri stable, dédoublonnage, classification
    /// dominante (`BREAKING` > `UNKNOWN` > `COMPATIBLE`).
    pub fn from_changes(mut changes: Vec<ContractChange>) -> Self {
        changes.sort_by(|a, b| (&a.path, &a.code, &a.message).cmp(&(&b.path, &b.code, &b.message)));
        changes.dedup();
        let mut classification = DiffClass::Compatible;
        for change in &changes {
            if change.class.rank() > classification.rank() {
                classification = change.class;
            }
        }
        Self {
            requires_confirmation: classification == DiffClass::Breaking
                || classification == DiffClass::Unknown,
            classification,
            changes,
        }
    }

    /// Diff sans changement : compatible, aucune confirmation.
    pub fn unchanged() -> Self {
        Self::from_changes(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// Diff entre deux contrats analysés.
pub fn contract_diff(old: &ParsedContract, new: &ParsedContract) -> ContractDiff {
    diff_snapshots(&old.snapshot(), &new.snapshot())
}

/// Diff entre deux modèles normalisés (snapshots de version).
pub fn diff_snapshots(old: &Value, new: &Value) -> ContractDiff {
    let mut changes = Vec::new();

    compare_operations(old, new, &mut changes);
    compare_schemas(old, new, &mut changes);
    compare_metadata(old, new, &mut changes);
    compare_extensions(old, new, &mut changes);
    report_unresolved_refs(new, &mut changes);

    ContractDiff::from_changes(changes)
}

// ---------------------------------------------------------------------------
// Opérations
// ---------------------------------------------------------------------------

type OperationKey = (String, String);

fn operations_of(snapshot: &Value) -> BTreeMap<OperationKey, &Value> {
    let mut operations = BTreeMap::new();
    if let Some(list) = snapshot.get("operations").and_then(Value::as_array) {
        for operation in list {
            let path = operation.get("path").and_then(Value::as_str).unwrap_or("");
            let method = operation
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("");
            if path.is_empty() || method.is_empty() {
                continue;
            }
            operations.insert((path.to_string(), method.to_ascii_uppercase()), operation);
        }
    }
    operations
}

fn escape_pointer(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

fn operation_path(key: &OperationKey) -> String {
    format!(
        "/paths/{}/{}",
        escape_pointer(&key.0),
        key.1.to_ascii_lowercase()
    )
}

fn compare_operations(old: &Value, new: &Value, changes: &mut Vec<ContractChange>) {
    let old_operations = operations_of(old);
    let new_operations = operations_of(new);

    for (key, old_operation) in &old_operations {
        let path = operation_path(key);
        let Some(new_operation) = new_operations.get(key) else {
            changes.push(ContractChange::new(
                "OAS-010",
                path.clone(),
                format!("Opération {} {} supprimée.", key.1, key.0),
                DiffClass::Breaking,
            ));
            continue;
        };

        let old_id = old_operation
            .get("operation_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        let new_id = new_operation
            .get("operation_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        if old_id != new_id {
            changes.push(ContractChange::new(
                "OAS-016",
                path.clone(),
                format!("operationId renommé de « {old_id} » à « {new_id} »."),
                DiffClass::Breaking,
            ));
        }

        compare_parameters(old_operation, new_operation, &path, changes);
        compare_responses(old_operation, new_operation, &path, changes);
        compare_request_body(old_operation, new_operation, &path, changes);
    }

    for key in new_operations.keys() {
        if !old_operations.contains_key(key) {
            changes.push(ContractChange::new(
                "OAS-021",
                operation_path(key),
                format!("Opération {} {} ajoutée.", key.1, key.0),
                DiffClass::Compatible,
            ));
        }
    }
}

fn parameters_of(operation: &Value) -> BTreeMap<(String, String), &Value> {
    let mut parameters = BTreeMap::new();
    if let Some(list) = operation.get("parameters").and_then(Value::as_array) {
        for parameter in list {
            let name = parameter.get("name").and_then(Value::as_str).unwrap_or("");
            let location = parameter.get("in").and_then(Value::as_str).unwrap_or("");
            if name.is_empty() {
                continue;
            }
            parameters.insert((name.to_string(), location.to_string()), parameter);
        }
    }
    parameters
}

fn parameter_required(parameter: &Value) -> bool {
    parameter
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn parameter_type(parameter: &Value) -> Option<String> {
    parameter
        .get("schema")
        .and_then(|schema| schema.get("type"))
        .or_else(|| parameter.get("type"))
        .map(type_label)
}

fn type_label(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("|"),
        other => other.to_string(),
    }
}

fn compare_parameters(
    old_operation: &Value,
    new_operation: &Value,
    path: &str,
    changes: &mut Vec<ContractChange>,
) {
    let old_parameters = parameters_of(old_operation);
    let new_parameters = parameters_of(new_operation);

    for ((name, location), old_parameter) in &old_parameters {
        let parameter_path = format!("{path}/parameters/{}", escape_pointer(name));
        let Some(new_parameter) = new_parameters.get(&(name.clone(), location.clone())) else {
            changes.push(ContractChange::new(
                "OAS-015",
                parameter_path,
                format!("Paramètre « {name} » ({location}) supprimé."),
                DiffClass::Breaking,
            ));
            continue;
        };

        if parameter_required(old_parameter) != parameter_required(new_parameter) {
            if parameter_required(new_parameter) {
                changes.push(ContractChange::new(
                    "OAS-013",
                    parameter_path.clone(),
                    format!("Paramètre « {name} » ({location}) devenu requis."),
                    DiffClass::Breaking,
                ));
            } else {
                changes.push(ContractChange::new(
                    "OAS-021",
                    parameter_path.clone(),
                    format!("Paramètre « {name} » ({location}) devenu optionnel."),
                    DiffClass::Compatible,
                ));
            }
        }

        if parameter_type(old_parameter) != parameter_type(new_parameter) {
            changes.push(ContractChange::new(
                "OAS-012",
                parameter_path.clone(),
                format!("Type du paramètre « {name} » ({location}) modifié."),
                DiffClass::Breaking,
            ));
        }

        if let (Some(old_schema), Some(new_schema)) =
            (old_parameter.get("schema"), new_parameter.get("schema"))
        {
            compare_schema(
                &format!("{parameter_path}/schema"),
                old_schema,
                new_schema,
                changes,
                0,
            );
        }
    }

    for ((name, location), new_parameter) in &new_parameters {
        if old_parameters.contains_key(&(name.clone(), location.clone())) {
            continue;
        }
        let parameter_path = format!("{path}/parameters/{}", escape_pointer(name));
        if parameter_required(new_parameter) {
            changes.push(ContractChange::new(
                "OAS-013",
                parameter_path,
                format!("Paramètre requis « {name} » ({location}) ajouté."),
                DiffClass::Breaking,
            ));
        } else {
            changes.push(ContractChange::new(
                "OAS-021",
                parameter_path,
                format!("Paramètre optionnel « {name} » ({location}) ajouté."),
                DiffClass::Compatible,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Réponses et corps de requête
// ---------------------------------------------------------------------------

fn responses_of(operation: &Value) -> BTreeMap<String, &Value> {
    let mut responses = BTreeMap::new();
    if let Some(list) = operation.get("responses").and_then(Value::as_array) {
        for response in list {
            let status = response.get("status").and_then(Value::as_str).unwrap_or("");
            if !status.is_empty() {
                responses.insert(status.to_string(), response);
            }
        }
    }
    responses
}

fn media_schemas(value: &Value) -> BTreeMap<String, &Value> {
    let mut schemas = BTreeMap::new();
    if let Some(content) = value.get("content").and_then(Value::as_object) {
        for (media_type, media) in content {
            if let Some(schema) = media.get("schema") {
                schemas.insert(media_type.clone(), schema);
            }
        }
    }
    schemas
}

fn compare_responses(
    old_operation: &Value,
    new_operation: &Value,
    path: &str,
    changes: &mut Vec<ContractChange>,
) {
    let old_responses = responses_of(old_operation);
    let new_responses = responses_of(new_operation);

    for (status, old_response) in &old_responses {
        let response_path = format!("{path}/responses/{status}");
        let Some(new_response) = new_responses.get(status) else {
            changes.push(ContractChange::new(
                "OAS-011",
                response_path,
                format!("Réponse {status} supprimée."),
                DiffClass::Breaking,
            ));
            continue;
        };

        let old_ref = old_response.get("schema_ref").and_then(Value::as_str);
        let new_ref = new_response.get("schema_ref").and_then(Value::as_str);
        if old_ref != new_ref {
            if new_ref.is_none() {
                changes.push(ContractChange::new(
                    "OAS-014",
                    response_path,
                    format!("Schéma de réponse {status} retiré : contrat restreint."),
                    DiffClass::Breaking,
                ));
            } else {
                changes.push(ContractChange::new(
                    "OAS-012",
                    response_path,
                    format!("Schéma de réponse {status} modifié."),
                    DiffClass::Breaking,
                ));
            }
        }
    }

    for status in new_responses.keys() {
        if !old_responses.contains_key(status) {
            changes.push(ContractChange::new(
                "OAS-021",
                format!("{path}/responses/{status}"),
                format!("Réponse {status} ajoutée."),
                DiffClass::Compatible,
            ));
        }
    }
}

fn compare_media(
    path: &str,
    old_container: &Value,
    new_container: &Value,
    changes: &mut Vec<ContractChange>,
) {
    let old_schemas = media_schemas(old_container);
    let new_schemas = media_schemas(new_container);

    for (media_type, old_schema) in &old_schemas {
        let media_path = format!("{path}/content/{}", escape_pointer(media_type));
        let Some(new_schema) = new_schemas.get(media_type) else {
            changes.push(ContractChange::new(
                "OAS-014",
                media_path,
                format!("Schéma de réponse restreint : type de média {media_type} retiré."),
                DiffClass::Breaking,
            ));
            continue;
        };
        compare_schema(
            &format!("{media_path}/schema"),
            old_schema,
            new_schema,
            changes,
            0,
        );
    }

    for media_type in new_schemas.keys() {
        if !old_schemas.contains_key(media_type) {
            changes.push(ContractChange::new(
                "OAS-021",
                format!("{path}/content/{}", escape_pointer(media_type)),
                format!("Type de média {media_type} ajouté."),
                DiffClass::Compatible,
            ));
        }
    }
}

fn compare_request_body(
    old_operation: &Value,
    new_operation: &Value,
    path: &str,
    changes: &mut Vec<ContractChange>,
) {
    let body_path = format!("{path}/requestBody");
    match (
        old_operation
            .get("request_body")
            .filter(|body| !body.is_null()),
        new_operation
            .get("request_body")
            .filter(|body| !body.is_null()),
    ) {
        (None, None) | (None, Some(_)) => {}
        (Some(_), None) => changes.push(ContractChange::new(
            "OAS-014",
            body_path,
            "Corps de requête retiré.".to_string(),
            DiffClass::Breaking,
        )),
        (Some(old_body), Some(new_body)) => {
            let old_required = old_body
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let new_required = new_body
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !old_required && new_required {
                changes.push(ContractChange::new(
                    "OAS-013",
                    body_path.clone(),
                    "Corps de requête devenu obligatoire.".to_string(),
                    DiffClass::Breaking,
                ));
            } else if old_required && !new_required {
                changes.push(ContractChange::new(
                    "OAS-021",
                    body_path.clone(),
                    "Corps de requête devenu optionnel.".to_string(),
                    DiffClass::Compatible,
                ));
            }
            compare_media(&body_path, old_body, new_body, changes);
        }
    }
}

// ---------------------------------------------------------------------------
// Schémas
// ---------------------------------------------------------------------------

fn schemas_of(snapshot: &Value) -> Map<String, Value> {
    snapshot
        .get("schemas")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn compare_schemas(old: &Value, new: &Value, changes: &mut Vec<ContractChange>) {
    let old_schemas = schemas_of(old);
    let new_schemas = schemas_of(new);

    for (name, old_schema) in &old_schemas {
        let path = format!("/components/schemas/{}", escape_pointer(name));
        let Some(new_schema) = new_schemas.get(name) else {
            changes.push(ContractChange::new(
                "OAS-014",
                path,
                format!("Schéma « {name} » supprimé."),
                DiffClass::Breaking,
            ));
            continue;
        };
        compare_schema(&path, old_schema, new_schema, changes, 0);
    }

    for name in new_schemas.keys() {
        if !old_schemas.contains_key(name) {
            changes.push(ContractChange::new(
                "OAS-021",
                format!("/components/schemas/{}", escape_pointer(name)),
                format!("Schéma « {name} » ajouté."),
                DiffClass::Compatible,
            ));
        }
    }
}

fn required_names(schema: &Value) -> BTreeSet<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|required| {
            required
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn enum_values(schema: &Value) -> Option<BTreeSet<String>> {
    schema
        .get("enum")
        .and_then(Value::as_array)
        .map(|values| values.iter().map(Value::to_string).collect())
}

fn compare_schema(
    path: &str,
    old: &Value,
    new: &Value,
    changes: &mut Vec<ContractChange>,
    depth: usize,
) {
    if depth > 32 {
        return;
    }
    let (Some(old_object), Some(new_object)) = (old.as_object(), new.as_object()) else {
        if old != new {
            changes.push(ContractChange::new(
                "OAS-012",
                path.to_string(),
                "Schéma modifié.".to_string(),
                DiffClass::Breaking,
            ));
        }
        return;
    };

    let old_ref = old_object.get("$ref").and_then(Value::as_str).unwrap_or("");
    let new_ref = new_object.get("$ref").and_then(Value::as_str).unwrap_or("");
    if old_ref != new_ref {
        changes.push(ContractChange::new(
            "OAS-012",
            format!("{path}/$ref"),
            format!("Schéma référencé modifié : « {old_ref} » → « {new_ref} »."),
            DiffClass::Breaking,
        ));
    }

    let old_type = old_object.get("type").map(type_label).unwrap_or_default();
    let new_type = new_object.get("type").map(type_label).unwrap_or_default();
    if old_type != new_type {
        changes.push(ContractChange::new(
            "OAS-012",
            format!("{path}/type"),
            format!("Type de schéma modifié : « {old_type} » → « {new_type} »."),
            DiffClass::Breaking,
        ));
    }

    let old_format = old_object
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("");
    let new_format = new_object
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("");
    if old_format != new_format {
        changes.push(ContractChange::new(
            "OAS-012",
            format!("{path}/format"),
            format!("Format de schéma modifié : « {old_format} » → « {new_format} »."),
            DiffClass::Breaking,
        ));
    }

    if old_object.get("description") != new_object.get("description") {
        changes.push(ContractChange::new(
            "OAS-020",
            format!("{path}/description"),
            "Description de schéma modifiée.".to_string(),
            DiffClass::Compatible,
        ));
    }

    let old_required = required_names(old);
    let new_required = required_names(new);
    let properties = match (
        old_object.get("properties").and_then(Value::as_object),
        new_object.get("properties").and_then(Value::as_object),
    ) {
        (Some(old_properties), Some(new_properties)) => Some((old_properties, new_properties)),
        _ => None,
    };

    if let Some((old_properties, new_properties)) = properties {
        for (name, old_property) in old_properties {
            let property_path = format!("{path}/properties/{}", escape_pointer(name));
            match new_properties.get(name) {
                None => {
                    if old_required.contains(name) {
                        changes.push(ContractChange::new(
                            "OAS-014",
                            property_path,
                            format!("Champ requis « {name} » supprimé."),
                            DiffClass::Breaking,
                        ));
                    } else {
                        changes.push(ContractChange::new(
                            "OAS-021",
                            property_path,
                            format!("Champ optionnel « {name} » supprimé."),
                            DiffClass::Compatible,
                        ));
                    }
                }
                Some(new_property) => {
                    compare_schema(
                        &property_path,
                        old_property,
                        new_property,
                        changes,
                        depth + 1,
                    );
                }
            }
        }
        for (name, _) in new_properties {
            if old_properties.contains_key(name) {
                continue;
            }
            let property_path = format!("{path}/properties/{}", escape_pointer(name));
            if new_required.contains(name) {
                changes.push(ContractChange::new(
                    "OAS-013",
                    property_path,
                    format!("Champ requis « {name} » ajouté."),
                    DiffClass::Breaking,
                ));
            } else {
                changes.push(ContractChange::new(
                    "OAS-021",
                    property_path,
                    format!("Champ optionnel « {name} » ajouté."),
                    DiffClass::Compatible,
                ));
            }
        }
    }

    for name in new_required.difference(&old_required) {
        if properties
            .as_ref()
            .map(|(old_properties, _)| old_properties.contains_key(name))
            .unwrap_or(false)
        {
            changes.push(ContractChange::new(
                "OAS-013",
                format!("{path}/required/{}", escape_pointer(name)),
                format!("Champ « {name} » devenu requis."),
                DiffClass::Breaking,
            ));
        }
    }
    for name in old_required.difference(&new_required) {
        if properties
            .as_ref()
            .map(|(_, new_properties)| new_properties.contains_key(name))
            .unwrap_or(false)
        {
            changes.push(ContractChange::new(
                "OAS-021",
                format!("{path}/required/{}", escape_pointer(name)),
                format!("Champ « {name} » devenu optionnel."),
                DiffClass::Compatible,
            ));
        }
    }

    if let (Some(old_enum), Some(new_enum)) = (enum_values(old), enum_values(new)) {
        for value in old_enum.difference(&new_enum) {
            changes.push(ContractChange::new(
                "OAS-014",
                format!("{path}/enum"),
                format!("Valeur d'énumération {value} retirée."),
                DiffClass::Breaking,
            ));
        }
        for value in new_enum.difference(&old_enum) {
            changes.push(ContractChange::new(
                "OAS-021",
                format!("{path}/enum"),
                format!("Valeur d'énumération {value} ajoutée."),
                DiffClass::Compatible,
            ));
        }
    }

    if let (Some(old_items), Some(new_items)) = (
        old_object.get("items").filter(|value| value.is_object()),
        new_object.get("items").filter(|value| value.is_object()),
    ) {
        compare_schema(
            &format!("{path}/items"),
            old_items,
            new_items,
            changes,
            depth + 1,
        );
    }

    let old_additional = old_object.get("additionalProperties");
    let new_additional = new_object.get("additionalProperties");
    if new_additional.and_then(Value::as_bool) == Some(false)
        && old_additional.and_then(Value::as_bool) != Some(false)
    {
        changes.push(ContractChange::new(
            "OAS-014",
            format!("{path}/additionalProperties"),
            "Propriétés additionnelles interdites.".to_string(),
            DiffClass::Breaking,
        ));
    }

    if !old_object.contains_key("not") && new_object.contains_key("not") {
        changes.push(ContractChange::new(
            "OAS-014",
            format!("{path}/not"),
            "Contrainte d'exclusion ajoutée.".to_string(),
            DiffClass::Breaking,
        ));
    }

    for keyword in ["anyOf", "oneOf", "allOf"] {
        if let (Some(old_variants), Some(new_variants)) = (
            old_object.get(keyword).and_then(Value::as_array),
            new_object.get(keyword).and_then(Value::as_array),
        ) {
            if new_variants.len() < old_variants.len() {
                changes.push(ContractChange::new(
                    "OAS-014",
                    format!("{path}/{keyword}"),
                    format!("Variantes {keyword} retirées : schéma restreint."),
                    DiffClass::Breaking,
                ));
            } else if new_variants.len() > old_variants.len() {
                changes.push(ContractChange::new(
                    "OAS-021",
                    format!("{path}/{keyword}"),
                    format!("Variantes {keyword} ajoutées."),
                    DiffClass::Compatible,
                ));
            }
            for (index, (old_variant, new_variant)) in
                old_variants.iter().zip(new_variants.iter()).enumerate()
            {
                compare_schema(
                    &format!("{path}/{keyword}/{index}"),
                    old_variant,
                    new_variant,
                    changes,
                    depth + 1,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Extensions et références
// ---------------------------------------------------------------------------

fn compare_metadata(old: &Value, new: &Value, changes: &mut Vec<ContractChange>) {
    let old_servers = old.get("servers").cloned().unwrap_or(Value::Null);
    let new_servers = new.get("servers").cloned().unwrap_or(Value::Null);
    if old_servers != new_servers {
        changes.push(ContractChange::new(
            "OAS-032",
            "/servers",
            "Serveurs du contrat modifiés.".to_string(),
            DiffClass::Unknown,
        ));
    }
    let old_security = old.get("security_schemes").cloned().unwrap_or(Value::Null);
    let new_security = new.get("security_schemes").cloned().unwrap_or(Value::Null);
    if old_security != new_security {
        changes.push(ContractChange::new(
            "OAS-032",
            "/security_schemes",
            "Schémas de sécurité modifiés.".to_string(),
            DiffClass::Unknown,
        ));
    }
}

fn compare_extensions(old: &Value, new: &Value, changes: &mut Vec<ContractChange>) {
    let empty = Map::new();
    let old_extensions = old
        .get("extensions")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let new_extensions = new
        .get("extensions")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let keys: BTreeSet<&String> = old_extensions.keys().chain(new_extensions.keys()).collect();

    for key in keys {
        match (old_extensions.get(key), new_extensions.get(key)) {
            (Some(old_value), Some(new_value)) if old_value == new_value => {}
            (Some(_), Some(_)) => changes.push(ContractChange::new(
                "OAS-031",
                key.clone(),
                "Extension modifiée.".to_string(),
                DiffClass::Unknown,
            )),
            (Some(_), None) => changes.push(ContractChange::new(
                "OAS-031",
                key.clone(),
                "Extension supprimée.".to_string(),
                DiffClass::Unknown,
            )),
            (None, Some(_)) => changes.push(ContractChange::new(
                "OAS-031",
                key.clone(),
                "Extension ajoutée.".to_string(),
                DiffClass::Unknown,
            )),
            (None, None) => {}
        }
    }
}

fn collect_refs(value: &Value, path: &str, refs: &mut Vec<(String, String)>) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                refs.push((reference.to_string(), path.to_string()));
            }
            if let Some(reference) = object.get("schema_ref").and_then(Value::as_str) {
                refs.push((reference.to_string(), path.to_string()));
            }
            for (key, item) in object {
                if key == "$ref" {
                    continue;
                }
                collect_refs(item, &format!("{path}/{}", escape_pointer(key)), refs);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_refs(item, &format!("{path}/{index}"), refs);
            }
        }
        _ => {}
    }
}

fn report_unresolved_refs(snapshot: &Value, changes: &mut Vec<ContractChange>) {
    let schemas = schemas_of(snapshot);
    let mut refs = Vec::new();
    for (name, schema) in &schemas {
        collect_refs(schema, &format!("/components/schemas/{name}"), &mut refs);
    }
    if let Some(operations) = snapshot.get("operations").and_then(Value::as_array) {
        for operation in operations {
            let path = operation.get("path").and_then(Value::as_str).unwrap_or("");
            let method = operation
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("");
            collect_refs(
                operation,
                &format!(
                    "/paths/{}/{}",
                    escape_pointer(path),
                    method.to_ascii_lowercase()
                ),
                &mut refs,
            );
        }
    }

    let mut reported = BTreeSet::new();
    for (reference, path) in refs {
        if !reference.starts_with('#') {
            // Les `$ref` externes ne sont jamais récupérés (§10.6).
            continue;
        }
        let Some(name) = reference.strip_prefix("#/components/schemas/") else {
            continue;
        };
        let name = name.split('/').next().unwrap_or(name);
        if schemas.contains_key(name) || !reported.insert(reference.clone()) {
            continue;
        }
        changes.push(ContractChange::new(
            "OAS-030",
            path,
            format!("Référence interne non résolue : {reference}."),
            DiffClass::Unknown,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::parse_contract;

    fn document(paths: &str, extra: &str) -> String {
        format!(
            r#"{{"openapi":"3.1.0","info":{{"title":"Customer API","version":"1.0.0","description":"Clients"}},"paths":{{{paths}}}{extra}}}"#
        )
    }

    fn base_paths() -> &'static str {
        r##""/customers": {
            "get": {
              "operationId": "listCustomers",
              "parameters": [],
              "responses": { "200": { "description": "ok", "content": { "application/json": { "schema": { "type": "object" } } } } }
            }
          }"##
    }

    fn diff(old: &str, new: &str) -> ContractDiff {
        contract_diff(&parse_contract(old).unwrap(), &parse_contract(new).unwrap())
    }

    #[test]
    fn removing_an_operation_is_breaking() {
        let old = document(base_paths(), "");
        let new = document("", "");
        let result = diff(&old, &new);
        assert_eq!(result.classification, DiffClass::Breaking);
        assert!(result.requires_confirmation);
        assert!(result.changes.iter().any(|change| change.code == "OAS-010"));
    }

    #[test]
    fn adding_an_operation_is_compatible() {
        let old = document("", "");
        let new = document(base_paths(), "");
        let result = diff(&old, &new);
        assert_eq!(result.classification, DiffClass::Compatible);
        assert!(!result.requires_confirmation);
        assert!(result.changes.iter().any(|change| change.code == "OAS-021"));
    }

    #[test]
    fn adding_a_required_parameter_is_breaking() {
        let old = document(base_paths(), "");
        let new = document(
            r##""/customers": {
                "get": {
                  "operationId": "listCustomers",
                  "parameters": [{ "name": "tenant", "in": "query", "required": true, "schema": { "type": "string" } }],
                  "responses": { "200": { "description": "ok", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Customer" } } } } }
                }
              }"##,
            "",
        );
        let result = diff(&old, &new);
        assert_eq!(result.classification, DiffClass::Breaking);
        assert!(result.changes.iter().any(|change| change.code == "OAS-013"));
    }

    #[test]
    fn description_change_is_compatible() {
        let old = document(
            base_paths(),
            r#","components":{"schemas":{"Customer":{"type":"object","description":"old"}}}"#,
        );
        let new = document(
            base_paths(),
            r#","components":{"schemas":{"Customer":{"type":"object","description":"new"}}}"#,
        );
        let result = diff(&old, &new);
        assert_eq!(result.classification, DiffClass::Compatible);
        assert!(result.changes.iter().any(|change| change.code == "OAS-020"));
    }

    #[test]
    fn changing_a_property_type_is_breaking() {
        let old = document(
            base_paths(),
            r#","components":{"schemas":{"Customer":{"type":"object","properties":{"id":{"type":"string"}}}}}"#,
        );
        let new = document(
            base_paths(),
            r#","components":{"schemas":{"Customer":{"type":"object","properties":{"id":{"type":"integer"}}}}}"#,
        );
        let result = diff(&old, &new);
        assert_eq!(result.classification, DiffClass::Breaking);
        assert!(result.changes.iter().any(|change| change.code == "OAS-012"));
    }

    #[test]
    fn modified_extension_is_unknown() {
        let old = document(base_paths(), r#","x-internal-owner":"team-a""#);
        let new = document(base_paths(), r#","x-internal-owner":"team-b""#);
        let result = diff(&old, &new);
        assert_eq!(result.classification, DiffClass::Unknown);
        assert!(result.requires_confirmation);
        assert!(result.changes.iter().any(|change| change.code == "OAS-031"));
    }

    #[test]
    fn unresolved_ref_is_reported_as_unknown() {
        let new = document(
            r##""/customers": {
                "get": {
                  "operationId": "listCustomers",
                  "responses": { "200": { "description": "ok", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Missing" } } } } }
                }
              }"##,
            "",
        );
        let result = diff(&document("", ""), &new);
        assert!(result.changes.iter().any(|change| change.code == "OAS-030"));
    }

    #[test]
    fn identical_documents_have_no_changes() {
        let document = document(base_paths(), "");
        let result = diff(&document, &document);
        assert!(result.is_empty());
        assert_eq!(result.classification, DiffClass::Compatible);
    }
}
