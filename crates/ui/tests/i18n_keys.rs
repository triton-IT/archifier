//! VD-25 : aucune chaîne d'interface en dur.
//!
//! Le test lit les sources et vérifie que chaque clé littérale passée à `t()`
//! ou `tf()` existe dans `i18n/fr.json`. Les clés construites dynamiquement
//! (`format!("layer.{}")`) sont couvertes par les tests de rendu.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}

fn literal_keys(source: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for pattern in ["t(\"", "tf(\""] {
        let mut offset = 0;
        while let Some(index) = source[offset..].find(pattern) {
            let absolute = offset + index;
            let boundary = absolute == 0
                || !source[..absolute]
                    .chars()
                    .next_back()
                    .map(|c| c.is_alphanumeric() || c == '_')
                    .unwrap_or(false);
            let after = &source[absolute + pattern.len()..];
            if boundary {
                if let Some(end) = after.find('"') {
                    keys.push(after[..end].to_string());
                }
            }
            offset = absolute + pattern.len();
        }
    }
    keys
}

#[test]
fn every_literal_key_exists_in_catalog() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog_path = root.join("../../i18n/fr.json");
    let catalog: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&catalog_path).unwrap()).unwrap();
    let keys: BTreeSet<String> = catalog.as_object().unwrap().keys().cloned().collect();

    let mut missing = Vec::new();
    for file in rust_files(&root.join("src")) {
        let text = fs::read_to_string(&file).unwrap_or_default();
        for key in literal_keys(&text) {
            if !keys.contains(&key) {
                missing.push(format!("{}: {key}", file.display()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "clés i18n manquantes :\n{}",
        missing.join("\n")
    );
}

#[test]
fn every_error_code_has_a_catalog_key() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog_path = root.join("../../i18n/fr.json");
    let catalog: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&catalog_path).unwrap()).unwrap();
    let keys: BTreeSet<String> = catalog.as_object().unwrap().keys().cloned().collect();
    let mut codes = BTreeSet::new();
    for file in rust_files(&root.join("src")) {
        let text = fs::read_to_string(&file).unwrap_or_default();
        for code in [
            "DSG-001",
            "DSG-002",
            "DSG-003",
            "DSG-004",
            "DSG-005",
            "REV-409",
            "CMD-001",
            "CMD-002",
            "DGM-000",
            "DGM-001",
            "DGM-002",
            "DGM-007",
            "DGM-008",
            "DGM-009",
            "DGM-012",
            "DGM-013",
            "DGM-014",
            "LINK-002",
            "LINK-003",
            "MM-ELEM-001",
            "MM-PROP-001",
            "MM-REL-002",
            "MM-REL-003",
            "OBJ-002",
        ] {
            if text.contains(code) {
                codes.insert(code.to_string());
            }
        }
    }
    for code in codes {
        let key = format!("error.{code}");
        assert!(keys.contains(&key), "clé d'erreur absente : {key}");
    }
}

#[test]
fn seven_intents_have_label_tagline_and_empty_state() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog_path = root.join("../../i18n/fr.json");
    let catalog: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&catalog_path).unwrap()).unwrap();
    let keys: BTreeSet<String> = catalog.as_object().unwrap().keys().cloned().collect();
    for intent in [
        "context",
        "application_landscape",
        "process",
        "deployment",
        "capability",
        "migration",
        "interface_review",
    ] {
        for suffix in ["label", "tagline", "empty"] {
            let key = format!("intent.{intent}.{suffix}");
            assert!(keys.contains(&key), "clé d'intention absente : {key}");
        }
    }
}

#[test]
fn templates_and_layers_have_labels() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog_path = root.join("../../i18n/fr.json");
    let catalog: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&catalog_path).unwrap()).unwrap();
    let keys: BTreeSet<String> = catalog.as_object().unwrap().keys().cloned().collect();
    for key in [
        "layer.business",
        "layer.application",
        "layer.technology",
        "layer.motivation",
        "layer.implementation",
        "layer.process",
        "layer.flow",
        "layer.data",
        "layer.structure",
        "layer.deployment",
        "template.application_landscape.title",
        "template.zone.supplier_domain",
    ] {
        assert!(keys.contains(key), "clé de libellé absente : {key}");
    }
}
