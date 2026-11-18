//! Outil actif et équivalents clavier (VD-05, §14.4.4).

use crate::i18n::t;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Element,
    Relation,
    Zone,
    Annotation,
    Frame,
}

impl Tool {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Select => "design.tool.select",
            Self::Element => "design.tool.element",
            Self::Relation => "design.tool.relation",
            Self::Zone => "design.tool.zone",
            Self::Annotation => "design.tool.annotation",
            Self::Frame => "design.tool.frame",
        }
    }

    pub fn label(self) -> String {
        t(self.label_key())
    }

    /// Raccourci simple (sans modificateur), insensible à la casse.
    pub fn from_key(key: &str) -> Option<Self> {
        match key.to_ascii_lowercase().as_str() {
            "v" => Some(Self::Select),
            "e" => Some(Self::Element),
            "r" => Some(Self::Relation),
            "z" => Some(Self::Zone),
            "n" => Some(Self::Annotation),
            "f" => Some(Self::Frame),
            _ => None,
        }
    }

    pub fn all() -> [Self; 6] {
        [
            Self::Select,
            Self::Element,
            Self::Relation,
            Self::Zone,
            Self::Annotation,
            Self::Frame,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_a_shortcut() {
        assert_eq!(Tool::from_key("v"), Some(Tool::Select));
        assert_eq!(Tool::from_key("E"), Some(Tool::Element));
        assert_eq!(Tool::from_key("r"), Some(Tool::Relation));
        assert_eq!(Tool::from_key("z"), Some(Tool::Zone));
        assert_eq!(Tool::from_key("n"), Some(Tool::Annotation));
        assert_eq!(Tool::from_key("f"), Some(Tool::Frame));
        assert_eq!(Tool::from_key("x"), None);
    }

    #[test]
    fn every_tool_has_a_label() {
        for tool in Tool::all() {
            assert!(!tool.label().is_empty());
            assert_ne!(tool.label(), tool.label_key());
        }
    }
}
