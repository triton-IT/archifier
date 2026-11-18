//! Tests d'architecture de la frontière UI ↔ dépôt ↔ stockage.
//!
//! Ils sont volontairement basés sur une lecture des sources : le compilateur ne
//! peut pas distinguer un accès IndexedDB caché dans une vue d'un appel normal.

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

fn source(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn ui_views_do_not_touch_storage() {
    let views = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/views");
    let forbidden = [
        "indexed_db",
        "local_storage",
        "IdbStore",
        "SnapshotStore",
        "CollabStore",
        "MemoryStore",
        "Repository::new",
        "archifier_repo::Repository",
    ];
    for file in rust_files(&views) {
        let text = source(&file);
        for needle in forbidden {
            assert!(
                !text.contains(needle),
                "{} contient un accès au stockage interdit : {needle}",
                file.display()
            );
        }
    }
}

#[test]
fn idb_store_is_only_known_by_boot() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in rust_files(&src) {
        let text = source(&file);
        if text.contains("IdbStore") {
            assert!(
                file.ends_with("boot.rs"),
                "IdbStore ne doit apparaître que dans boot.rs, trouvé dans {}",
                file.display()
            );
        }
    }
}

#[test]
fn memory_fallback_is_documented_exception() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in rust_files(&src) {
        let text = source(&file);
        if text.contains("MemoryStore") {
            let allowed = file.ends_with("boot.rs") || file.ends_with("state.rs");
            assert!(
                allowed,
                "MemoryStore n'est autorisé que dans boot.rs (racine de composition) \
                 et state.rs (repli anti-panic documenté), trouvé dans {}",
                file.display()
            );
        }
    }
}

#[test]
fn canvas_never_writes_layout_directly() {
    let canvas = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/canvas");
    let forbidden = [
        "save_layout",
        "create_element",
        "create_relationship",
        "repo()",
    ];
    for file in rust_files(&canvas) {
        let text = source(&file);
        for needle in forbidden {
            assert!(
                !text.contains(needle),
                "{} : le canvas ne doit jamais ?crire le layout ni lire le d?p?t ({needle})",
                file.display()
            );
        }
    }
}

#[test]
fn inspector_never_writes_repository() {
    let inspector = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/inspector");
    let forbidden = [
        "save_layout",
        "create_element",
        "create_relationship",
        "create_interface",
        "set_diagram_publication",
        "update_element(",
        "update_domain(",
        "rename_interface(",
    ];
    for file in rust_files(&inspector) {
        let text = source(&file);
        for needle in forbidden {
            assert!(
                !text.contains(needle),
                "{} : l'inspecteur doit passer par le bus, pas par le d?p?t ({needle})",
                file.display()
            );
        }
    }
}

#[test]
fn every_layout_write_goes_through_commands() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in rust_files(&src) {
        if file.ends_with("design/commands.rs") {
            continue;
        }
        let text = source(&file);
        assert!(
            !text.contains(".save_layout(") && !text.contains(".save_layout_expect("),
            "{} ?crit le layout hors de design/commands.rs",
            file.display()
        );
    }
}

#[test]
fn repository_surface_is_frozen() {
    let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../repo/src/lib.rs");
    let text = source(&lib);
    for method in [
        "pub fn save_layout_expect",
        "pub fn view_cards",
        "pub fn rename_interface",
        "pub fn set_diagram_publication",
        "pub fn interface_of_version",
        "pub fn update_relationship_endpoints",
    ] {
        assert!(text.contains(method), "m?thode additive absente : {method}");
    }
}

#[test]
fn atelier_has_no_home_route() {
    let router = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/router.rs");
    let text = source(&router);
    assert!(
        !text.contains("Route::Home"),
        "la route Home ne doit plus exister"
    );
    assert!(
        text.contains("Route::Atelier"),
        "la route Atelier doit exister"
    );
}
