//! Erreurs typées du moteur collaboratif.
//!
//! Aucune fonction publique du crate ne panique : les échecs de transaction, de
//! décodage et d'intégration Yrs sont convertis en variantes de [`CollabError`].

use thiserror::Error;

/// Erreur du moteur de collaboration.
#[derive(Debug, Error)]
pub enum CollabError {
    /// Une transaction exclusive est déjà ouverte sur le document Yrs.
    #[error("transaction CRDT indisponible : {0}")]
    Transaction(#[from] yrs::TransactionAcqError),

    /// Le binaire Yrs v1 (mise à jour ou vecteur d'état) est illisible.
    #[error("binaire Yrs v1 illisible : {0}")]
    Decode(#[from] yrs::encoding::read::Error),

    /// La mise à jour est décodable mais ne s'intègre pas au document.
    #[error("mise à jour Yrs non intégrable : {0}")]
    Apply(#[from] yrs::error::UpdateError),

    /// Indice de texte hors bornes (compté en unités UTF-16, comme l'UI).
    #[error("indice {index} hors bornes pour un texte de {len} unités UTF-16")]
    IndexOutOfBounds {
        /// Indice demandé.
        index: u32,
        /// Longueur courante du texte.
        len: u32,
    },

    /// L'indice tombe au milieu d'une paire de substitution UTF-16.
    #[error("indice {index} situé au milieu d'une paire de substitution UTF-16")]
    InvalidBoundary {
        /// Indice demandé.
        index: u32,
    },
}
