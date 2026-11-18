//! État d'enregistrement et entrées d'historique (C-01 §6, VD-04).

use archifier_core::DiagramLayout;

#[derive(Debug, Clone, PartialEq)]
pub enum SaveState {
    Saved { revision: u64 },
    Saving,
    Dirty,
    Conflict,
    Error { code: String, message: String },
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub before: DiagramLayout,
    pub after: DiagramLayout,
    pub revision_before: u64,
    pub revision_after: u64,
}

pub const MAX_HISTORY: usize = 100;
