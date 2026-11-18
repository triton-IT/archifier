//! Tests de la frontière de stockage : import persistant et reset explicite.

use archifier_repo::{MemoryStore, Repository};
use std::rc::Rc;

const NOW_MS: u64 = 1_795_000_000_000;

#[test]
fn imported_snapshot_is_persisted() {
    let source_store = Rc::new(MemoryStore::new());
    let source = Repository::new(source_store, NOW_MS, 42);
    source.create_domain("Importé", "description").unwrap();
    let snapshot = source.export_json();

    let target_store = Rc::new(MemoryStore::new());
    Repository::from_json(target_store.clone(), &snapshot, NOW_MS, 42).unwrap();

    let persisted = target_store
        .snapshot()
        .expect("l'import doit persister le snapshot");
    assert!(persisted.contains("Importé"));

    let reloaded = Repository::from_json(Rc::new(MemoryStore::new()), &persisted, NOW_MS, 42)
        .expect("le snapshot persisté doit être relisible");
    assert!(reloaded.domains().iter().any(|d| d.name == "Importé"));
}

#[test]
fn reset_storage_clears_memory_store_and_repository() {
    let store = Rc::new(MemoryStore::new());
    let repo = Repository::new(store.clone(), NOW_MS, 42);
    repo.create_domain("À effacer", "").unwrap();
    assert!(store.snapshot().is_some());

    repo.reset_storage().expect("reset mémoire");

    assert!(store.snapshot().is_none());
    assert!(repo.domains().is_empty());
}
