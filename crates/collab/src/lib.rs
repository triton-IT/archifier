//! Moteur de collaboration CRDT headless.
//!
//! Ce crate ne dépend ni de l'interface ni d'IndexedDB. Il porte les documents
//! Yrs, les transports et la présence. Le spike T7-00 vérifie les budgets
//! d'encodage, de décodage et de convergence avant tout branchement UI.
//!
//! # Périmètre du spike T7-00
//!
//! Le crate n'expose pour l'instant qu'un document mono-texte
//! ([`TextDocument`]) : c'est le plus petit support qui prouve les quatre
//! portes de sortie de l'ADR-014 (convergence, taille d'une mise à jour,
//! latence de décodage, empreinte WASM). Les cartes, tableaux et sous-documents
//! du schéma `technical-specifications-lot2.md` §3 viendront après le feu vert.
//!
//! Voir `crates/collab/README.md` pour les commandes et les mesures.

pub mod error;
pub mod text_doc;

pub use error::CollabError;
pub use text_doc::TextDocument;

/// Version du schéma CRDT décrite dans `technical-specifications-lot2.md` §3.
pub const COLLAB_SCHEMA: u32 = 1;
