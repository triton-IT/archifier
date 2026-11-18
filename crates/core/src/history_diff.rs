//! Diff champ par champ entre deux instantanés JSON (T1-04).
//!
//! Les chemins suivent la RFC 6901 (JSON Pointer) : `~` est échappé en `~0`
//! et `/` en `~1`. Le diff est purement structurel, sans horloge ni I/O, ce qui
//! le rend identique côté serveur et côté bundle WebAssembly.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

/// Une différence atomique entre deux instantanés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDifference {
    /// Pointeur JSON RFC 6901 vers le champ modifié.
    pub path: String,
    /// Valeur avant, `None` si le champ a été ajouté.
    pub before: Option<Value>,
    /// Valeur après, `None` si le champ a été supprimé.
    pub after: Option<Value>,
}

/// Compare deux documents JSON et renvoie les différences triées par `path`.
///
/// Deux documents égaux produisent une liste vide. Les objets sont comparés
/// clé par clé et les tableaux indice par indice ; un tableau ajouté ou retiré
/// produit une différence par élément manquant, ce qui reste lisible dans le
/// tableau `path / avant / après` de l'UI.
pub fn diff_values(before: &Value, after: &Value) -> Vec<FieldDifference> {
    let mut differences = Vec::new();
    if before != after {
        walk("", before, after, &mut differences);
    }
    differences.sort_by(|a, b| a.path.cmp(&b.path));
    differences
}

fn walk(prefix: &str, before: &Value, after: &Value, out: &mut Vec<FieldDifference>) {
    if before == after {
        return;
    }
    match (before, after) {
        (Value::Object(left), Value::Object(right)) => {
            walk_objects(prefix, left, right, out);
        }
        (Value::Array(left), Value::Array(right)) => {
            walk_arrays(prefix, left, right, out);
        }
        _ => out.push(FieldDifference {
            path: prefix.to_string(),
            before: Some(before.clone()),
            after: Some(after.clone()),
        }),
    }
}

fn walk_objects(
    prefix: &str,
    left: &Map<String, Value>,
    right: &Map<String, Value>,
    out: &mut Vec<FieldDifference>,
) {
    let keys: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
    for key in keys {
        let path = format!("{prefix}/{}", escape_pointer(key));
        match (left.get(key), right.get(key)) {
            (Some(before), Some(after)) => walk(&path, before, after, out),
            (Some(before), None) => out.push(FieldDifference {
                path,
                before: Some(before.clone()),
                after: None,
            }),
            (None, Some(after)) => out.push(FieldDifference {
                path,
                before: None,
                after: Some(after.clone()),
            }),
            (None, None) => {}
        }
    }
}

fn walk_arrays(prefix: &str, left: &[Value], right: &[Value], out: &mut Vec<FieldDifference>) {
    let length = left.len().max(right.len());
    for index in 0..length {
        let path = format!("{prefix}/{index}");
        match (left.get(index), right.get(index)) {
            (Some(before), Some(after)) => walk(&path, before, after, out),
            (Some(before), None) => out.push(FieldDifference {
                path,
                before: Some(before.clone()),
                after: None,
            }),
            (None, Some(after)) => out.push(FieldDifference {
                path,
                before: None,
                after: Some(after.clone()),
            }),
            (None, None) => {}
        }
    }
}

/// Échappement RFC 6901 : `~` puis `/`, dans cet ordre.
fn escape_pointer(raw: &str) -> String {
    raw.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn diff_is_field_by_field() {
        let before = json!({ "name": "Order", "properties": { "tier": "gold", "owner": "ops" } });
        let after = json!({ "name": "Order", "properties": { "tier": "silver", "owner": "ops" } });
        let changes = diff_values(&before, &after);
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert_eq!(changes[0].path, "/properties/tier");
        assert_eq!(changes[0].before, Some(json!("gold")));
        assert_eq!(changes[0].after, Some(json!("silver")));
    }

    #[test]
    fn diff_reports_additions_and_removals() {
        let before = json!({ "a": 1, "b": 2 });
        let after = json!({ "a": 1, "c": 3 });
        let changes = diff_values(&before, &after);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].path, "/b");
        assert_eq!(changes[0].before, Some(json!(2)));
        assert_eq!(changes[0].after, None);
        assert_eq!(changes[1].path, "/c");
        assert_eq!(changes[1].before, None);
        assert_eq!(changes[1].after, Some(json!(3)));
    }

    #[test]
    fn diff_is_sorted_by_path() {
        let before = json!({ "z": 1, "a": 1, "m": { "y": 1, "b": 1 } });
        let after = json!({ "z": 2, "a": 2, "m": { "y": 2, "b": 2 } });
        let paths: Vec<String> = diff_values(&before, &after)
            .into_iter()
            .map(|change| change.path)
            .collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
        assert_eq!(
            paths,
            vec!["/a", "/m/b", "/m/y", "/z"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn identical_values_have_no_difference() {
        let value = json!({ "a": [1, 2, { "b": true }] });
        assert!(diff_values(&value, &value).is_empty());
    }

    #[test]
    fn arrays_compare_index_by_index() {
        let changes = diff_values(&json!([1, 2, 3]), &json!([1, 9]));
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].path, "/1");
        assert_eq!(changes[0].before, Some(json!(2)));
        assert_eq!(changes[0].after, Some(json!(9)));
        assert_eq!(changes[1].path, "/2");
        assert_eq!(changes[1].before, Some(json!(3)));
        assert_eq!(changes[1].after, None);
    }

    #[test]
    fn pointers_escape_slashes_and_tildes() {
        let before = json!({ "a/b": 1, "c~d": 1 });
        let after = json!({ "a/b": 2, "c~d": 2 });
        let paths: Vec<String> = diff_values(&before, &after)
            .into_iter()
            .map(|change| change.path)
            .collect();
        assert_eq!(paths, vec!["/a~1b", "/c~0d"]);
    }
}
