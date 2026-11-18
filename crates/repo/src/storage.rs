//! Persistance : le dépôt reste la source de vérité en mémoire, le stockage
//! n'est qu'un journal durable. Une seule écriture = une seule opération
//! atomique, ce qui rend le rollback trivial.

use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    #[error("stockage indisponible : {0}")]
    Unavailable(String),
    #[error("quota de stockage dépassé")]
    QuotaExceeded,
    #[error("données illisibles : {0}")]
    Corrupted(String),
}

pub trait SnapshotStore {
    /// Écrit un snapshot complet. Doit être atomique : soit tout, soit rien.
    fn save(&self, json: &str) -> Result<(), StoreError>;

    /// Efface le stockage persistant. Utilisé par le reset explicite des réglages.
    /// L'implémentation mémoire vide son contenu ; IndexedDB ferme puis supprime
    /// la base. L'appelant doit recharger l'application ensuite.
    fn reset(&self) -> Result<(), StoreError> {
        Ok(())
    }

    /// Dernière erreur asynchrone observée (IndexedDB), à poller par l'UI.
    fn take_error(&self) -> Option<String> {
        None
    }

    /// Renseigné quand le navigateur signale une éviction ou une base vide
    /// alors qu'on attendait des données.
    fn eviction_suspected(&self) -> bool {
        false
    }
}

/// Stockage en mémoire : tests natifs et repli si le navigateur refuse IndexedDB.
#[derive(Debug, Default)]
pub struct MemoryStore {
    snapshot: RefCell<Option<String>>,
    saves: RefCell<u32>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_snapshot(json: impl Into<String>) -> Self {
        let store = Self::new();
        *store.snapshot.borrow_mut() = Some(json.into());
        store
    }

    pub fn snapshot(&self) -> Option<String> {
        self.snapshot.borrow().clone()
    }

    pub fn save_count(&self) -> u32 {
        *self.saves.borrow()
    }
}

impl SnapshotStore for MemoryStore {
    fn save(&self, json: &str) -> Result<(), StoreError> {
        *self.snapshot.borrow_mut() = Some(json.to_string());
        *self.saves.borrow_mut() += 1;
        Ok(())
    }

    fn reset(&self) -> Result<(), StoreError> {
        *self.snapshot.borrow_mut() = None;
        Ok(())
    }
}

/// Stockage qui échoue : utilisé pour prouver qu'une écriture ratée n'altère
/// pas l'état du référentiel.
#[derive(Debug, Default)]
pub struct FailingStore;

impl SnapshotStore for FailingStore {
    fn save(&self, _json: &str) -> Result<(), StoreError> {
        Err(StoreError::QuotaExceeded)
    }
}
