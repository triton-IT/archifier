//! URIs internes `arch://<kind>/<ulid>` (FR-017).

use crate::ids::ObjectId;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchKind {
    Domain,
    Element,
    Relationship,
    Interface,
    Version,
    Diagram,
    Story,
    Review,
}

impl ArchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Domain => "domain",
            Self::Element => "element",
            Self::Relationship => "relationship",
            Self::Interface => "interface",
            Self::Version => "version",
            Self::Diagram => "diagram",
            Self::Story => "story",
            Self::Review => "review",
        }
    }

    /// Segment de route applicatif.
    pub fn route_segment(self) -> &'static str {
        match self {
            Self::Domain => "domains",
            Self::Element => "elements",
            Self::Relationship => "relationships",
            Self::Interface => "interfaces",
            Self::Version => "versions",
            Self::Diagram => "diagrams",
            Self::Story => "stories",
            Self::Review => "reviews",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "domain" => Self::Domain,
            "element" => Self::Element,
            "relationship" => Self::Relationship,
            "interface" => Self::Interface,
            "version" => Self::Version,
            "diagram" => Self::Diagram,
            "story" => Self::Story,
            "review" => Self::Review,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchUri {
    pub kind: ArchKind,
    pub id: ObjectId,
}

impl ArchUri {
    pub fn new(kind: ArchKind, id: ObjectId) -> Self {
        Self { kind, id }
    }

    /// Route applicative vers l'objet.
    pub fn route(&self) -> String {
        format!("#/view/{}/{}", self.kind.route_segment(), self.id)
    }
}

impl fmt::Display for ArchUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "arch://{}/{}", self.kind.as_str(), self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("lien interne invalide : {0}")]
pub struct UriError(pub String);

impl FromStr for ArchUri {
    type Err = UriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rest = s
            .strip_prefix("arch://")
            .ok_or_else(|| UriError(format!("préfixe arch:// absent dans « {s} »")))?;
        let (kind, id) = rest
            .split_once('/')
            .ok_or_else(|| UriError(format!("forme attendue arch://<kind>/<id>, reçu « {s} »")))?;
        let kind = ArchKind::parse(kind)
            .ok_or_else(|| UriError(format!("type d'objet inconnu « {kind} »")))?;
        let id = ObjectId::from_str(id).map_err(|e| UriError(e.to_string()))?;
        Ok(Self { kind, id })
    }
}

/// Extrait toutes les URIs `arch://` d'un texte Markdown.
pub fn extract_uris(md: &str) -> Vec<ArchUri> {
    let mut found = Vec::new();
    let bytes = md.as_bytes();
    let needle = b"arch://";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let start = i;
            let mut end = i + needle.len();
            while end < bytes.len() {
                let c = bytes[end];
                let stop = c.is_ascii_whitespace()
                    || matches!(
                        c,
                        b')' | b']' | b'<' | b'>' | b'"' | b'\'' | b'`' | b',' | b';'
                    );
                if stop {
                    break;
                }
                end += 1;
            }
            let candidate = &md[start..end];
            // On retire une éventuelle ponctuation finale collée.
            let trimmed = candidate.trim_end_matches(['.', ':', '!', '?']);
            if let Ok(uri) = ArchUri::from_str(trimmed) {
                found.push(uri);
            }
            i = end;
        } else {
            i += 1;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    #[test]
    fn parses_canonical_form() {
        let uri: ArchUri = format!("arch://element/{ID}").parse().unwrap();
        assert_eq!(uri.kind, ArchKind::Element);
        assert_eq!(format!("{uri}"), format!("arch://element/{ID}"));
    }

    #[test]
    fn accepts_other_cases_and_routes() {
        let uri: ArchUri = format!("arch://Interface/{ID}").parse().unwrap();
        assert_eq!(uri.kind, ArchKind::Interface);
        assert_eq!(uri.route(), format!("#/view/interfaces/{ID}"));
    }

    #[test]
    fn rejects_unknown_kind_and_trailing_slash() {
        assert!(format!("arch://gadget/{ID}").parse::<ArchUri>().is_err());
        assert!(format!("arch://element/{ID}/x").parse::<ArchUri>().is_err());
        assert!("pas-une-uri".parse::<ArchUri>().is_err());
    }

    #[test]
    fn extracts_from_markdown() {
        let md = format!(
            "Voir [Customer API](arch://interface/{ID}) et arch://diagram/{ID}.\nPuis arch://bogus/x."
        );
        let uris = extract_uris(&md);
        assert_eq!(uris.len(), 2);
        assert_eq!(uris[0].kind, ArchKind::Interface);
        assert_eq!(uris[1].kind, ArchKind::Diagram);
    }
}
