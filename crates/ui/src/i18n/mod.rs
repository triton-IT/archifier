//! Catalogue de libellés fr-FR, chargé une seule fois depuis `i18n/fr.json`.
//!
//! Aucune chaîne d'interface n'est écrite en dur dans les écrans de l'Atelier :
//! une clé manquante affiche la clé, ce qui rend le défaut visible en recette.

use std::collections::HashMap;
use std::sync::OnceLock;

static CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();

fn catalog() -> &'static HashMap<String, String> {
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../i18n/fr.json")).unwrap_or_default()
    })
}

/// Traduit une clé. Une clé absente est renvoyée telle quelle.
pub fn t(key: &str) -> String {
    catalog()
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// Traduit une clé en remplaçant les jetons `{nom}`.
pub fn tf(key: &str, args: &[(&str, String)]) -> String {
    let mut value = t(key);
    for (name, replacement) in args {
        value = value.replace(&format!("{{{name}}}"), replacement);
    }
    value
}

pub fn has(key: &str) -> bool {
    catalog().contains_key(key)
}

pub fn keys() -> Vec<String> {
    catalog().keys().cloned().collect()
}
