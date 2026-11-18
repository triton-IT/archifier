//! Contrats OpenAPI : parsing, empreinte canonique et modèle normalisé.
//!
//! Ce module ne fait aucune I/O et n'appelle aucune horloge. Il implémente le
//! contrat W1a [`docs/w1a-openapi-contract.md`] : lecture JSON ou YAML des
//! familles 3.0 / 3.1 / 3.2, empreinte SHA-256 calculée sur une représentation
//! canonique à clés triées, hors namespace `x-architecture-*`.
//!
//! Le document brut n'est jamais perdu : [`ParsedContract::raw`] conserve le
//! texte d'origine octet à octet, tandis que `canonical` sert uniquement à
//! l'idempotence.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::validation::is_semver;

/// Famille de contrat reconnue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractFormat {
    #[serde(rename = "OPENAPI_3_0")]
    OpenApi30,
    #[serde(rename = "OPENAPI_3_1")]
    OpenApi31,
    #[serde(rename = "OPENAPI_3_2")]
    OpenApi32,
    #[serde(rename = "ASYNCAPI_2")]
    AsyncApi2,
    #[serde(rename = "ASYNCAPI_3")]
    AsyncApi3,
}

impl ContractFormat {
    /// Libellé stable de la famille, utilisé dans le snapshot normalisé.
    pub fn document_version(self) -> &'static str {
        match self {
            Self::OpenApi30 => "3.0.0",
            Self::OpenApi31 => "3.1.0",
            Self::OpenApi32 => "3.2.0",
            Self::AsyncApi2 => "asyncapi:2",
            Self::AsyncApi3 => "asyncapi:3",
        }
    }

    /// Vrai pour une famille OpenAPI importable par le moteur W1a.
    pub fn is_openapi(self) -> bool {
        matches!(self, Self::OpenApi30 | Self::OpenApi31 | Self::OpenApi32)
    }

    /// Libellé lisible de la famille de contrat.
    pub fn label(self) -> &'static str {
        match self {
            Self::OpenApi30 => "OpenAPI 3.0",
            Self::OpenApi31 => "OpenAPI 3.1",
            Self::OpenApi32 => "OpenAPI 3.2",
            Self::AsyncApi2 => "AsyncAPI 2",
            Self::AsyncApi3 => "AsyncAPI 3",
        }
    }
}

/// Erreurs typées du parsing de contrat.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContractError {
    /// `OAS-001` : version inconnue ou famille non supportée.
    #[error("OAS-001 : version de contrat non supportée : {0}")]
    UnsupportedVersion(String),
    /// `OAS-002` : document illisible (JSON/YAML invalide, racine non objet,
    /// BOM, taille ou profondeur hors limites).
    #[error("OAS-002 : document illisible : {0}")]
    Unreadable(String),
    /// `OAS-003` : `operationId` dupliqué dans le document.
    #[error("OAS-003 : operationId dupliqué : {0}")]
    DuplicateOperationId(String),
    /// `OAS-006` : `info.version` non normalisable en SemVer.
    #[error("OAS-006 : version d'information non normalisable : {0}")]
    InvalidSemVer(String),
}

impl ContractError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedVersion(_) => "OAS-001",
            Self::Unreadable(_) => "OAS-002",
            Self::DuplicateOperationId(_) => "OAS-003",
            Self::InvalidSemVer(_) => "OAS-006",
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            Self::UnsupportedVersion(detail)
            | Self::Unreadable(detail)
            | Self::DuplicateOperationId(detail)
            | Self::InvalidSemVer(detail) => detail,
        }
    }
}

/// Opération normalisée d'un contrat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractOperation {
    pub operation_id: String,
    /// Méthode HTTP en majuscules (`GET`, `POST`, …).
    pub method: String,
    pub path: String,
    pub parameters: Vec<Value>,
    pub request_body: Option<Value>,
    pub responses: Vec<Value>,
}

/// Contrat analysé, prêt pour l'aperçu d'import.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedContract {
    pub format: ContractFormat,
    /// Texte d'origine, conservé octet à octet.
    pub raw: String,
    /// JSON canonique à clés triées, sans `x-architecture-*`.
    pub canonical: String,
    /// SHA-256 hexadécimal de `canonical`.
    pub hash: String,
    pub title: String,
    pub version: String,
    pub description: String,
    pub servers: Vec<String>,
    pub security: Vec<String>,
    pub operations: Vec<ContractOperation>,
    pub schemas: Map<String, Value>,
}

/// Clé d'extension exclue de l'empreinte d'idempotence.
pub const ARCHITECTURE_EXTENSION_PREFIX: &str = "x-architecture";

impl ParsedContract {
    /// Valeur texte d'une extension de premier niveau du document.
    ///
    /// La lecture repart du document brut : `canonical` exclut volontairement
    /// le namespace `x-architecture-*`, qui porte l'identité d'export.
    pub fn extension(&self, key: &str) -> Option<String> {
        let document = parse_document(&self.raw).ok()?;
        document
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    /// Modèle normalisé stocké dans `InterfaceVersion.contract_snapshot`.
    ///
    /// DTO exact du contrat W1a §10.3 : `openapi_version`, `servers`,
    /// `security_schemes`, `operations`, `schemas`, `extensions`. Le document
    /// brut, le canonique et l'empreinte restent hors du snapshot.
    pub fn snapshot(&self) -> Value {
        let operations: Vec<Value> = self
            .operations
            .iter()
            .map(|operation| {
                json!({
                    "operation_id": operation.operation_id,
                    "method": operation.method,
                    "path": operation.path,
                    "parameters": operation.parameters,
                    "request_body": operation.request_body,
                    "responses": snapshot_responses(&operation.responses),
                })
            })
            .collect();

        let extensions = parse_document(&self.raw)
            .ok()
            .map(|document| root_extensions(&document))
            .unwrap_or_default();

        json!({
            "openapi_version": self.format.document_version(),
            "servers": self.servers,
            "security_schemes": self.security,
            "operations": operations,
            "schemas": self.schemas,
            "extensions": extensions,
        })
    }
}

// ---------------------------------------------------------------------------
// Entrée : JSON ou YAML
// ---------------------------------------------------------------------------

/// Plafond d'entrée du contrat W1a §10.6.
pub const MAX_CONTRACT_BYTES: usize = 2 * 1024 * 1024;
/// Profondeur maximale acceptée du document.
pub const MAX_CONTRACT_DEPTH: usize = 128;

fn parse_document(raw: &str) -> Result<Value, ContractError> {
    if raw.trim().is_empty() {
        return Err(ContractError::Unreadable("le document est vide".into()));
    }
    if raw.len() > MAX_CONTRACT_BYTES {
        return Err(ContractError::Unreadable(format!(
            "document de {} octets : plafond de {MAX_CONTRACT_BYTES} octets dépassé",
            raw.len()
        )));
    }
    if raw.starts_with('\u{feff}') {
        return Err(ContractError::Unreadable(
            "BOM UTF-8 refusé : le document doit être un texte UTF-8 sans marque d'ordre".into(),
        ));
    }
    let parsed = if let Ok(value) = serde_json::from_str::<Value>(raw) {
        value
    } else {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(raw)
            .map_err(|error| ContractError::Unreadable(format!("YAML invalide : {error}")))?;
        yaml_to_json(&yaml)?
    };
    if value_depth(&parsed, 0) > MAX_CONTRACT_DEPTH {
        return Err(ContractError::Unreadable(format!(
            "profondeur supérieure à {MAX_CONTRACT_DEPTH}"
        )));
    }
    Ok(parsed)
}

fn value_depth(value: &Value, depth: usize) -> usize {
    match value {
        Value::Object(object) => object
            .values()
            .map(|item| value_depth(item, depth + 1))
            .max()
            .unwrap_or(depth),
        Value::Array(items) => items
            .iter()
            .map(|item| value_depth(item, depth + 1))
            .max()
            .unwrap_or(depth),
        _ => depth,
    }
}

fn yaml_to_json(value: &serde_yaml_ng::Value) -> Result<Value, ContractError> {
    use serde_yaml_ng::Value as Yaml;
    match value {
        Yaml::Null => Ok(Value::Null),
        Yaml::Bool(flag) => Ok(Value::Bool(*flag)),
        Yaml::Number(number) => {
            if let Some(unsigned) = number.as_u64() {
                Ok(Value::Number(unsigned.into()))
            } else if let Some(signed) = number.as_i64() {
                Ok(Value::Number(signed.into()))
            } else if let Some(float) = number.as_f64() {
                serde_json::Number::from_f64(float)
                    .map(Value::Number)
                    .ok_or_else(|| {
                        ContractError::Unreadable(format!(
                            "nombre YAML non représentable en JSON : {number}"
                        ))
                    })
            } else {
                Err(ContractError::Unreadable(
                    "nombre YAML non représentable en JSON".into(),
                ))
            }
        }
        Yaml::String(text) => Ok(Value::String(text.clone())),
        Yaml::Sequence(items) => items
            .iter()
            .map(yaml_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Yaml::Mapping(mapping) => {
            let mut object = Map::new();
            for (key, item) in mapping.iter() {
                let key = match key {
                    Yaml::String(text) => text.clone(),
                    Yaml::Number(number) => number.to_string(),
                    Yaml::Bool(flag) => flag.to_string(),
                    _ => {
                        return Err(ContractError::Unreadable(
                            "clé YAML non textuelle dans un objet".into(),
                        ))
                    }
                };
                object.insert(key, yaml_to_json(item)?);
            }
            Ok(Value::Object(object))
        }
        Yaml::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}

fn detect_format(root: &Map<String, Value>) -> Result<ContractFormat, ContractError> {
    if let Some(declared) = root.get("openapi").and_then(Value::as_str) {
        let mut parts = declared.trim().split('.');
        let major = parts.next().unwrap_or("");
        let minor = parts.next().unwrap_or("");
        return match (major, minor) {
            ("3", "0") => Ok(ContractFormat::OpenApi30),
            ("3", "1") => Ok(ContractFormat::OpenApi31),
            ("3", "2") => Ok(ContractFormat::OpenApi32),
            _ => Err(ContractError::UnsupportedVersion(format!(
                "OpenAPI {declared} (3.0, 3.1 et 3.2 attendues)"
            ))),
        };
    }
    if let Some(declared) = root.get("asyncapi").and_then(Value::as_str) {
        return Err(ContractError::UnsupportedVersion(format!(
            "AsyncAPI {declared} : l'import de contrats asynchrones n'est pas supporté"
        )));
    }
    Err(ContractError::UnsupportedVersion(
        "champ « openapi » absent".into(),
    ))
}

// ---------------------------------------------------------------------------
// Parsing du modèle normalisé
// ---------------------------------------------------------------------------

const HTTP_METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// Alias public exigé par le contrat d'intégration §9.9.
pub type OpenApiError = ContractError;

/// Analyse un document OpenAPI (JSON ou YAML) — nom public du contrat §9.9.
pub fn parse_openapi(raw: &str) -> Result<ParsedContract, OpenApiError> {
    parse_contract(raw)
}

/// Analyse un document JSON ou YAML.
pub fn parse_contract(raw: &str) -> Result<ParsedContract, ContractError> {
    parse_contract_with_warnings(raw).map(|(contract, _)| contract)
}

/// Analyse un document et renvoie les avertissements non bloquants.
///
/// Avertissements produits ici :
/// - `OAS-WARN-001` : `operationId` absent, identifiant généré.
pub fn parse_contract_with_warnings(
    raw: &str,
) -> Result<(ParsedContract, Vec<String>), ContractError> {
    let document = parse_document(raw)?;
    let root = document.as_object().ok_or_else(|| {
        ContractError::Unreadable("la racine du document doit être un objet".into())
    })?;
    let format = detect_format(root)?;

    let info = root.get("info").and_then(Value::as_object);
    let title = text_of(info.and_then(|info| info.get("title")));
    let version = text_of(info.and_then(|info| info.get("version")));
    let description = text_of(info.and_then(|info| info.get("description")));

    let servers = root
        .get("servers")
        .and_then(Value::as_array)
        .map(|servers| {
            servers
                .iter()
                .filter_map(|server| {
                    server.as_str().map(str::to_string).or_else(|| {
                        server
                            .get("url")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                })
                .map(|url| url.trim().to_string())
                .filter(|url| !url.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let security = collect_security(root);

    let mut warnings = Vec::new();
    let operations = collect_operations(root, &mut warnings)?;
    check_unique_operation_ids(&operations)?;

    let schemas = root
        .get("components")
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let canonical = canonical_fingerprint(&document);
    let hash = sha256_hex(canonical.as_bytes());

    Ok((
        ParsedContract {
            format,
            raw: raw.to_string(),
            canonical,
            hash,
            title,
            version,
            description,
            servers,
            security,
            operations,
            schemas,
        },
        warnings,
    ))
}

fn text_of(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string()
}

fn collect_security(root: &Map<String, Value>) -> Vec<String> {
    let mut names = BTreeSet::new();
    if let Some(requirements) = root.get("security").and_then(Value::as_array) {
        for requirement in requirements {
            if let Some(requirement) = requirement.as_object() {
                for name in requirement.keys() {
                    names.insert(name.clone());
                }
            }
        }
    }
    if let Some(schemes) = root
        .get("components")
        .and_then(|components| components.get("securitySchemes"))
        .and_then(Value::as_object)
    {
        for name in schemes.keys() {
            names.insert(name.clone());
        }
    }
    names.into_iter().collect()
}

fn collect_operations(
    root: &Map<String, Value>,
    warnings: &mut Vec<String>,
) -> Result<Vec<ContractOperation>, ContractError> {
    let mut operations = Vec::new();
    let Some(paths) = root.get("paths").and_then(Value::as_object) else {
        return Ok(operations);
    };

    for (path, path_item) in paths {
        let Some(path_item) = path_item.as_object() else {
            continue;
        };
        let path_parameters = path_item.get("parameters");
        for method in HTTP_METHODS {
            let Some(operation) = path_item.get(method).and_then(Value::as_object) else {
                continue;
            };
            let declared_id = operation
                .get("operationId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let operation_id = match declared_id {
                Some(id) => id,
                None => {
                    let generated = generated_operation_id(method, path);
                    warnings.push(format!(
                        "OAS-WARN-001 : operationId absent pour {} {} ; « {generated} » généré.",
                        method.to_ascii_uppercase(),
                        path
                    ));
                    generated
                }
            };

            let parameters = merge_parameters(path_parameters, operation.get("parameters"));
            let request_body = operation.get("requestBody").cloned();
            let responses = normalize_responses(operation.get("responses"));

            operations.push(ContractOperation {
                operation_id,
                method: method.to_ascii_uppercase(),
                path: path.clone(),
                parameters,
                request_body,
                responses,
            });
        }
    }

    operations.sort_by(|a, b| (&a.path, &a.method).cmp(&(&b.path, &b.method)));
    warnings.sort();
    warnings.dedup();
    Ok(operations)
}

fn check_unique_operation_ids(operations: &[ContractOperation]) -> Result<(), ContractError> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for operation in operations {
        if !seen.insert(operation.operation_id.as_str()) {
            return Err(ContractError::DuplicateOperationId(format!(
                "« {} » apparaît plusieurs fois",
                operation.operation_id
            )));
        }
    }
    Ok(())
}

/// Identifiant généré quand `operationId` est absent : `method_path`.
///
/// Les segments sont séparés par `_`, les accolades de paramètres de chemin
/// sont retirées : `GET /customers/{id}` → `get_customers_id`.
pub fn generated_operation_id(method: &str, path: &str) -> String {
    let mut out = method.to_ascii_lowercase();
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        let segment = segment.trim_matches(|c| c == '{' || c == '}');
        let normalized: String = segment
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let normalized = normalized.trim_matches('_');
        if !normalized.is_empty() {
            out.push('_');
            out.push_str(normalized);
        }
    }
    out
}

fn parameter_key(value: &Value) -> Option<(String, String)> {
    let object = value.as_object()?;
    let name = object.get("name")?.as_str()?.to_string();
    let location = object
        .get("in")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some((name, location))
}

fn merge_parameters(path_level: Option<&Value>, operation_level: Option<&Value>) -> Vec<Value> {
    let mut merged: Vec<Value> = Vec::new();
    for parameter in path_level
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            operation_level
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        )
    {
        match parameter_key(parameter) {
            Some(key) => {
                if let Some(position) = merged
                    .iter()
                    .position(|existing| parameter_key(existing).as_ref() == Some(&key))
                {
                    merged[position] = parameter.clone();
                } else {
                    merged.push(parameter.clone());
                }
            }
            None => merged.push(parameter.clone()),
        }
    }
    merged
}

fn normalize_responses(responses: Option<&Value>) -> Vec<Value> {
    let Some(responses) = responses.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut statuses: Vec<&String> = responses.keys().collect();
    statuses.sort();
    let mut normalized = Vec::with_capacity(statuses.len());
    for status in statuses {
        let Some(value) = responses.get(status) else {
            continue;
        };
        let mut entry = match value {
            Value::Object(object) => object.clone(),
            other => {
                let mut object = Map::new();
                object.insert("description".into(), other.clone());
                object
            }
        };
        entry.insert("status".into(), Value::String(status.clone()));
        normalized.push(Value::Object(entry));
    }
    normalized
}

// ---------------------------------------------------------------------------
// Canonicalisation et empreinte
// ---------------------------------------------------------------------------

/// SHA-256 hexadécimal minuscule.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Représentation d'empreinte : JSON compact à clés triées, sans
/// `x-architecture-*` à aucun niveau.
pub fn canonical_fingerprint(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out, false, 0, true);
    out
}

/// JSON canonique compact à clés triées, extensions comprises.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out, false, 0, false);
    out
}

/// JSON canonique indenté sur deux espaces, extensions comprises.
///
/// Utilisé par l'export pour garantir des octets reproductibles.
pub fn canonical_json_pretty(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out, true, 0, false);
    out
}

fn write_canonical(
    value: &Value,
    out: &mut String,
    pretty: bool,
    depth: usize,
    exclude_architecture: bool,
) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => out.push_str(&number.to_string()),
        Value::String(text) => {
            out.push_str(&serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string()));
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if pretty {
                    out.push('\n');
                    push_indent(out, depth + 1);
                }
                write_canonical(item, out, pretty, depth + 1, exclude_architecture);
            }
            if pretty && !items.is_empty() {
                out.push('\n');
                push_indent(out, depth);
            }
            out.push(']');
        }
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            let keys: Vec<&String> = keys
                .into_iter()
                .filter(|key| {
                    !(exclude_architecture && key.starts_with(ARCHITECTURE_EXTENSION_PREFIX))
                })
                .collect();
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if pretty {
                    out.push('\n');
                    push_indent(out, depth + 1);
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                if pretty {
                    out.push(' ');
                }
                if let Some(item) = object.get(*key) {
                    write_canonical(item, out, pretty, depth + 1, exclude_architecture);
                }
            }
            if pretty && !keys.is_empty() {
                out.push('\n');
                push_indent(out, depth);
            }
            out.push('}');
        }
    }
}

fn push_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn root_extensions(document: &Value) -> Map<String, Value> {
    let mut extensions = Map::new();
    if let Some(object) = document.as_object() {
        for (key, value) in object {
            if key.starts_with('x') && !key.starts_with(ARCHITECTURE_EXTENSION_PREFIX) {
                extensions.insert(key.clone(), value.clone());
            }
        }
    }
    extensions
}

fn snapshot_responses(responses: &[Value]) -> Vec<Value> {
    let mut out = Vec::with_capacity(responses.len());
    for response in responses {
        let status = response.get("status").and_then(Value::as_str).unwrap_or("");
        if status.is_empty() {
            continue;
        }
        let schema_ref = response
            .get("content")
            .and_then(Value::as_object)
            .and_then(|content| {
                let mut media_types: Vec<&String> = content.keys().collect();
                media_types.sort();
                media_types.iter().find_map(|media_type| {
                    content
                        .get(*media_type)
                        .and_then(|media| media.get("schema"))
                        .and_then(|schema| schema.get("$ref"))
                        .and_then(Value::as_str)
                        .map(|reference| Value::String(reference.to_string()))
                })
            })
            .unwrap_or(Value::Null);
        out.push(json!({ "status": status, "schema_ref": schema_ref }));
    }
    out
}

/// Normalise `info.version` en SemVer (`1.0` → `1.0.0`, `1` → `1.0.0`).
pub fn normalize_semver(declared: &str) -> Result<String, OpenApiError> {
    let trimmed = declared.trim();
    if is_semver(trimmed) {
        return Ok(trimmed.to_string());
    }
    let without_prefix = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    let parts: Vec<&str> = without_prefix.split('.').collect();
    let numeric = !parts.is_empty()
        && parts.len() <= 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    if numeric {
        let mut padded: Vec<&str> = parts;
        while padded.len() < 3 {
            padded.push("0");
        }
        let candidate = padded.join(".");
        if is_semver(&candidate) {
            return Ok(candidate);
        }
    }
    Err(OpenApiError::InvalidSemVer(format!(
        "info.version « {declared} » n'est pas normalisable en SemVer"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON_30: &str = r##"{
      "openapi": "3.0.3",
      "info": { "title": "Customer API", "version": "1.4.0", "description": "Clients" },
      "servers": [{ "url": "https://api.example.com/v1" }],
      "paths": {
        "/customers/{id}": {
          "get": {
            "operationId": "getCustomer",
            "parameters": [
              { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
            ],
            "responses": {
              "200": { "description": "ok", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Customer" } } } }
            }
          }
        }
      },
      "components": { "schemas": { "Customer": { "type": "object", "properties": { "id": { "type": "string" } } } } },
      "security": [{ "oauth2": [] }]
    }"##;

    #[test]
    fn parses_json_openapi_3_0() {
        let contract = parse_contract(JSON_30).unwrap();
        assert_eq!(contract.format, ContractFormat::OpenApi30);
        assert_eq!(contract.title, "Customer API");
        assert_eq!(contract.version, "1.4.0");
        assert_eq!(contract.servers, vec!["https://api.example.com/v1"]);
        assert_eq!(contract.security, vec!["oauth2"]);
        assert_eq!(contract.operations.len(), 1);
        assert_eq!(contract.operations[0].operation_id, "getCustomer");
        assert_eq!(contract.operations[0].method, "GET");
        assert_eq!(contract.operations[0].path, "/customers/{id}");
        assert_eq!(contract.operations[0].responses[0]["status"], "200");
        assert!(contract.schemas.contains_key("Customer"));
        assert_eq!(contract.hash.len(), 64);
    }

    #[test]
    fn parses_yaml_3_1_and_3_2() {
        let yaml_31 = r#"
openapi: 3.1.0
info:
  title: Orders
  version: "2.0.0"
paths: {}
"#;
        let contract = parse_contract(yaml_31).unwrap();
        assert_eq!(contract.format, ContractFormat::OpenApi31);
        assert_eq!(contract.raw, yaml_31);

        let yaml_32 = r#"
openapi: 3.2.0
info:
  title: Billing
  version: "0.1.0"
paths: {}
"#;
        assert_eq!(
            parse_contract(yaml_32).unwrap().format,
            ContractFormat::OpenApi32
        );
    }

    #[test]
    fn rejects_unsupported_version_and_illegible_document() {
        let unsupported = parse_contract(r#"{"openapi":"2.0","info":{},"paths":{}}"#).unwrap_err();
        assert_eq!(unsupported.code(), "OAS-001");
        let missing = parse_contract(r#"{"info":{},"paths":{}}"#).unwrap_err();
        assert_eq!(missing.code(), "OAS-001");
        let broken = parse_contract("openapi: 3.0.3\npaths: [unclosed").unwrap_err();
        assert_eq!(broken.code(), "OAS-002");
        let scalar = parse_contract("42").unwrap_err();
        assert_eq!(scalar.code(), "OAS-002");
        let asyncapi =
            parse_contract(r#"{"asyncapi":"2.6.0","info":{},"channels":{}}"#).unwrap_err();
        assert_eq!(asyncapi.code(), "OAS-001");
    }

    #[test]
    fn hash_ignores_key_order_and_architecture_extensions() {
        let first = r#"
openapi: 3.0.3
x-architecture-version-id: 01ARZ3NDEKTSV4RRFFQ69G5FAV
info:
  title: Customer API
  version: "1.0.0"
paths: {}
"#;
        let second = r#"{"openapi":"3.0.3","info":{"version":"1.0.0","title":"Customer API"},"paths":{},"x-architecture-version-id":"01ARZ3NDEKTSV4RRFFQ69G5FAV"}"#;
        let a = parse_contract(first).unwrap();
        let b = parse_contract(second).unwrap();
        assert_eq!(a.hash, b.hash);
        assert!(!a.canonical.contains("x-architecture"));
        assert!(a.canonical.starts_with(r#"{"info":"#));

        let changed = parse_contract(
            &second.replace("01ARZ3NDEKTSV4RRFFQ69G5FAV", "01BX5ZZKBKACTAV9WEVGEMMVRZ"),
        )
        .unwrap();
        assert_eq!(
            a.hash, changed.hash,
            "l'identité d'export ne change pas l'empreinte"
        );
    }

    #[test]
    fn raw_document_is_preserved_byte_for_byte() {
        let raw =
            "openapi: 3.0.3\r\ninfo:\r\n  title: CRLF\r\n  version: \"1.0.0\"\r\npaths: {}\r\n";
        let contract = parse_contract(raw).unwrap();
        assert_eq!(contract.raw, raw);
    }

    #[test]
    fn operations_are_sorted_and_generated_ids_are_warned() {
        let raw = r#"
openapi: 3.1.0
info: { title: Opened, version: "1.0.0" }
paths:
  /b:
    post: { responses: { "204": { description: done } } }
  /a:
    get: { responses: { "200": { description: ok } } }
"#;
        let (contract, warnings) = parse_contract_with_warnings(raw).unwrap();
        let keys: Vec<(&str, &str)> = contract
            .operations
            .iter()
            .map(|operation| (operation.path.as_str(), operation.method.as_str()))
            .collect();
        assert_eq!(keys, vec![("/a", "GET"), ("/b", "POST")]);
        assert_eq!(contract.operations[0].operation_id, "get_a");
        assert_eq!(contract.operations[1].operation_id, "post_b");
        assert_eq!(warnings.len(), 2);
        assert!(warnings
            .iter()
            .all(|warning| warning.contains("OAS-WARN-001")));
    }

    #[test]
    fn snapshot_contains_normalized_model_and_non_architecture_extensions() {
        let raw = r#"
openapi: 3.0.3
info: { title: Customer API, version: "1.0.0" }
paths: {}
x-internal-owner: platform
x-architecture-version-id: 01ARZ3NDEKTSV4RRFFQ69G5FAV
"#;
        let contract = parse_contract(raw).unwrap();
        let snapshot = contract.snapshot();
        assert_eq!(snapshot["openapi_version"], "3.0.0");
        assert_eq!(snapshot["extensions"]["x-internal-owner"], "platform");
        assert!(snapshot["extensions"]
            .get("x-architecture-version-id")
            .is_none());
        assert!(snapshot.get("title").is_none());
        assert_eq!(
            contract.extension("x-architecture-version-id").as_deref(),
            Some("01ARZ3NDEKTSV4RRFFQ69G5FAV")
        );
    }

    #[test]
    fn snapshot_responses_are_reduced_to_status_and_schema_ref() {
        let contract = parse_contract(JSON_30).unwrap();
        let snapshot = contract.snapshot();
        let response = &snapshot["operations"][0]["responses"][0];
        assert_eq!(response["status"], "200");
        assert_eq!(response["schema_ref"], "#/components/schemas/Customer");
    }

    #[test]
    fn semver_is_normalized_or_rejected() {
        assert_eq!(normalize_semver("1.0.0").unwrap(), "1.0.0");
        assert_eq!(normalize_semver("1.0").unwrap(), "1.0.0");
        assert_eq!(normalize_semver("1").unwrap(), "1.0.0");
        assert_eq!(normalize_semver("v2.3").unwrap(), "2.3.0");
        let error = normalize_semver("2026-11-18").unwrap_err();
        assert_eq!(error.code(), "OAS-006");
    }

    #[test]
    fn duplicate_operation_ids_are_rejected() {
        let raw = r#"
openapi: 3.0.3
info: { title: Dup, version: "1.0.0" }
paths:
  /a:
    get: { operationId: same, responses: { "200": { description: ok } } }
  /b:
    get: { operationId: same, responses: { "200": { description: ok } } }
"#;
        let error = parse_contract(raw).unwrap_err();
        assert_eq!(error.code(), "OAS-003");
    }

    #[test]
    fn bom_and_oversized_documents_are_rejected() {
        let bom = format!("\u{feff}{JSON_30}");
        assert_eq!(parse_contract(&bom).unwrap_err().code(), "OAS-002");
        let oversized = format!("{}\n{}", JSON_30, "x".repeat(MAX_CONTRACT_BYTES));
        assert_eq!(parse_contract(&oversized).unwrap_err().code(), "OAS-002");
    }

    #[test]
    fn duplicate_yaml_keys_are_rejected() {
        let raw =
            "openapi: 3.0.3\ninfo:\n  title: A\n  title: B\n  version: \"1.0.0\"\npaths: {}\n";
        let error = parse_contract(raw).unwrap_err();
        assert_eq!(error.code(), "OAS-002");
    }

    #[test]
    fn contract_format_serde_names_are_explicit() {
        assert_eq!(
            serde_json::to_string(&ContractFormat::OpenApi31).unwrap(),
            "\"OPENAPI_3_1\""
        );
        assert_eq!(
            serde_json::to_string(&ContractFormat::AsyncApi2).unwrap(),
            "\"ASYNCAPI_2\""
        );
    }

    #[test]
    fn canonical_pretty_is_stable_and_sorted() {
        let value = json!({ "b": [1, 2], "a": { "d": true, "c": null } });
        let pretty = canonical_json_pretty(&value);
        assert_eq!(
            pretty,
            "{\n  \"a\": {\n    \"c\": null,\n    \"d\": true\n  },\n  \"b\": [\n    1,\n    2\n  ]\n}"
        );
        assert_eq!(pretty, canonical_json_pretty(&value));
        assert_eq!(
            canonical_json(&value),
            r#"{"a":{"c":null,"d":true},"b":[1,2]}"#
        );
    }
}
