//! Sélection simple et cadrage (C-01, VD-05).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    None,
    Node(String),
    Edge(String),
    Zone(String),
    Draft(String),
}

impl Selection {
    pub fn node_id(&self) -> Option<&str> {
        match self {
            Self::Node(id) => Some(id),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }
}
