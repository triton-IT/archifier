//! Document texte Yrs, indexé en unités UTF-16.
//!
//! Ce module est le seul endroit du crate qui parle directement à Yrs. Il expose
//! un sous-ensemble volontairement étroit, suffisant pour le spike T7-00 :
//! insertion, lecture, encodage/décodage binaire v1 et vecteurs d'état.
//!
//! Les indices de texte sont comptés en **unités UTF-16** (`OffsetKind::Utf16`),
//! c'est-à-dire l'unité de `String.prototype.length` et des sélections DOM. La
//! conversion depuis les indices byte/`char` de Rust reste centralisée ici.

use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{
    Doc, GetString, OffsetKind, Options, ReadTxn, RootRef, StateVector, Text, TextRef, Transact,
    Update,
};

use crate::error::CollabError;

/// Document collaboratif mono-texte (Y.Text racine).
///
/// Un seul réplica par instance : `client_id` doit être unique parmi les
/// réplicas actifs du même document, sinon l'état CRDT est corrompu.
#[derive(Debug)]
pub struct TextDocument {
    doc: Doc,
    text: TextRef,
}

impl TextDocument {
    /// Crée un document vide avec un réplica identifié par `client_id`, et un
    /// `Y.Text` racine nommé `root`.
    pub fn new(client_id: u64, root: &str) -> Result<Self, CollabError> {
        let options = Options {
            client_id,
            offset_kind: OffsetKind::Utf16,
            ..Options::default()
        };
        let doc = Doc::with_options(options);
        let text = {
            let mut txn = doc.try_transact_mut()?;
            TextRef::root(root).get_or_create(&mut txn)
        };
        Ok(Self { doc, text })
    }

    /// Identifiant du réplica local.
    pub fn client_id(&self) -> u64 {
        self.doc.client_id()
    }

    /// Longueur visible du texte, en unités UTF-16.
    pub fn len(&self) -> Result<u32, CollabError> {
        let txn = self.doc.try_transact()?;
        Ok(self.text.len(&txn))
    }

    /// Vrai si le texte est vide.
    pub fn is_empty(&self) -> Result<bool, CollabError> {
        Ok(self.len()? == 0)
    }

    /// Contenu courant, sans attributs de mise en forme.
    pub fn content(&self) -> Result<String, CollabError> {
        let txn = self.doc.try_transact()?;
        Ok(self.text.get_string(&txn))
    }

    /// Insère `chunk` à l'indice `index` (unités UTF-16).
    ///
    /// Renvoie [`CollabError::IndexOutOfBounds`] plutôt que de laisser Yrs
    /// paniquer sur un indice supérieur à la longueur courante.
    pub fn insert(&self, index: u32, chunk: &str) -> Result<(), CollabError> {
        let len = self.len()?;
        if index > len {
            return Err(CollabError::IndexOutOfBounds { index, len });
        }
        let content = self.content()?;
        if !is_utf16_boundary(&content, index) {
            return Err(CollabError::InvalidBoundary { index });
        }
        let mut txn = self.doc.try_transact_mut()?;
        self.text.insert(&mut txn, index, chunk);
        drop(txn);
        Ok(())
    }

    /// Vecteur d'état local encodé en binaire Yrs v1.
    pub fn state_vector_v1(&self) -> Result<Vec<u8>, CollabError> {
        let txn = self.doc.try_transact()?;
        Ok(txn.state_vector().encode_v1())
    }

    /// Mise à jour contenant tout ce que le pair distant n'a pas encore vu,
    /// d'après son vecteur d'état encodé en v1.
    ///
    /// C'est le chemin utilisé pour propager une frappe.
    pub fn diff_v1(&self, remote_state_vector: &[u8]) -> Result<Vec<u8>, CollabError> {
        let sv = StateVector::decode_v1(remote_state_vector)?;
        let txn = self.doc.try_transact()?;
        Ok(txn.encode_diff_v1(&sv))
    }

    /// Mise à jour contenant l'intégralité de l'état du document (v1).
    ///
    /// Sert à l'initialisation d'un réplica ou à la persistance.
    pub fn full_update_v1(&self) -> Result<Vec<u8>, CollabError> {
        let txn = self.doc.try_transact()?;
        Ok(txn.encode_state_as_update_v1(&StateVector::default()))
    }

    /// Décode et applique une mise à jour binaire v1.
    ///
    /// Les blocs déjà connus sont ignorés par Yrs : l'opération est idempotente.
    pub fn apply_v1(&self, update: &[u8]) -> Result<(), CollabError> {
        let decoded = Update::decode_v1(update)?;
        let mut txn = self.doc.try_transact_mut()?;
        txn.apply_update(decoded)?;
        drop(txn);
        Ok(())
    }
}

/// Vrai si `index` (unités UTF-16) tombe sur une frontière de caractère.
fn is_utf16_boundary(text: &str, index: u32) -> bool {
    if index == 0 {
        return true;
    }
    let mut units = 0u32;
    for ch in text.chars() {
        if units == index {
            return true;
        }
        units += ch.len_utf16() as u32;
        if units > index {
            return false;
        }
    }
    units == index
}
