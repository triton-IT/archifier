//! Routage par fragment d'URL (`#/design/01J…`), conformément à ADR-012.
//!
//! Les chemins de la spécification (§15.2) sont des chemins logiques ; la
//! forme réelle est `#/<chemin>`. Les alias (`/model/diagrams/:id`,
//! `/view/domains/:id`) restent résolus sans redirection d'historique.

use archifier_core::ObjectId;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryTab {
    Views,
    Domains,
    Objects,
    Interfaces,
    Stories,
    Reviews,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlTab {
    Anomalies,
    Inventory,
    Coverage,
    Exports,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewLink {
    pub zoom: Option<f64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub focus: Vec<String>,
    pub dim: bool,
    pub caption: Option<String>,
    pub t: Option<String>,
}

impl ViewLink {
    pub fn is_empty(&self) -> bool {
        self.zoom.is_none()
            && self.x.is_none()
            && self.y.is_none()
            && self.focus.is_empty()
            && !self.dim
            && self.caption.is_none()
            && self.t.is_none()
    }

    pub fn to_query(&self) -> String {
        let mut parts = Vec::new();
        if let Some(zoom) = self.zoom {
            parts.push(format!("zoom={zoom}"));
        }
        if let Some(x) = self.x {
            parts.push(format!("x={x}"));
        }
        if let Some(y) = self.y {
            parts.push(format!("y={y}"));
        }
        if !self.focus.is_empty() {
            let mut focus = self.focus.clone();
            focus.sort();
            focus.dedup();
            parts.push(format!("focus={}", focus.join(",")));
        }
        if self.dim {
            parts.push("dim=1".into());
        }
        if let Some(caption) = &self.caption {
            if !caption.is_empty() {
                parts.push(format!("caption={}", encode(caption)));
            }
        }
        if let Some(t) = &self.t {
            parts.push(format!("t={}", encode(t)));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("?{}", parts.join("&"))
        }
    }

    fn from_query(query: &BTreeMap<String, String>) -> Self {
        let zoom = query.get("zoom").and_then(|v| v.parse().ok());
        let x = query.get("x").and_then(|v| v.parse().ok());
        let y = query.get("y").and_then(|v| v.parse().ok());
        let focus = query
            .get("focus")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let dim = query
            .get("dim")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false);
        let caption = query.get("caption").map(|v| decode(v));
        let t = query.get("t").map(|v| decode(v));
        Self {
            zoom,
            x,
            y,
            focus,
            dim,
            caption,
            t,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Atelier,
    ViewNew {
        intent: Option<archifier_core::ViewIntent>,
    },
    ViewEditor {
        id: ObjectId,
        link: ViewLink,
    },
    ViewReader {
        id: ObjectId,
        link: ViewLink,
    },
    Library {
        tab: LibraryTab,
    },
    DomainSummary {
        id: ObjectId,
    },
    Search {
        q: String,
    },
    Control {
        tab: ControlTab,
    },
    Element {
        id: ObjectId,
    },
    Relationship {
        id: ObjectId,
    },
    Interface {
        id: ObjectId,
        version: Option<ObjectId>,
    },
    Version {
        id: ObjectId,
    },
    Story {
        id: ObjectId,
    },
    Review {
        id: ObjectId,
    },
    ViewAt {
        at: String,
        view: Option<ObjectId>,
    },
    Graph {
        id: ObjectId,
    },
    Settings,
    NotFound(String),
}

pub fn current_hash() -> String {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default()
}

pub fn parse_hash(hash: &str) -> Route {
    let raw = hash.trim_start_matches('#');
    let (path, query) = match raw.split_once('?') {
        Some((path, query)) => (path, parse_query(query)),
        None => (raw, BTreeMap::new()),
    };
    let path = path.trim_start_matches('/');
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let link = ViewLink::from_query(&query);

    match segments.as_slice() {
        [] => Route::Atelier,
        ["design", "new"] => Route::ViewNew {
            intent: query
                .get("intent")
                .and_then(|value| archifier_core::ViewIntent::parse(value)),
        },
        ["design", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::ViewEditor { id, link },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["model", "diagrams", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::ViewEditor { id, link },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["view", "diagrams", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::ViewReader { id, link },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["library"] | ["library", "views"] => Route::Library {
            tab: LibraryTab::Views,
        },
        ["library", "domains"] => Route::Library {
            tab: LibraryTab::Domains,
        },
        ["library", "objects"] => Route::Library {
            tab: LibraryTab::Objects,
        },
        ["library", "interfaces"] => Route::Library {
            tab: LibraryTab::Interfaces,
        },
        ["library", "stories"] => Route::Library {
            tab: LibraryTab::Stories,
        },
        ["library", "reviews"] => Route::Library {
            tab: LibraryTab::Reviews,
        },
        ["library", "domains", id] | ["view", "domains", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::DomainSummary { id },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["search"] => Route::Search {
            q: query.get("q").cloned().unwrap_or_default(),
        },
        ["control"] => Route::Control {
            tab: ControlTab::Anomalies,
        },
        ["control", "inventory"] => Route::Control {
            tab: ControlTab::Inventory,
        },
        ["control", "coverage"] => Route::Control {
            tab: ControlTab::Coverage,
        },
        ["control", "exports"] => Route::Control {
            tab: ControlTab::Exports,
        },
        ["view", "elements", id] => parse_object(hash, id).map_or_else(
            || Route::NotFound(hash.to_string()),
            |id| Route::Element { id },
        ),
        ["view", "relationships", id] => parse_object(hash, id).map_or_else(
            || Route::NotFound(hash.to_string()),
            |id| Route::Relationship { id },
        ),
        ["view", "interfaces", id] => parse_object(hash, id).map_or_else(
            || Route::NotFound(hash.to_string()),
            |id| Route::Interface { id, version: None },
        ),
        ["view", "versions", id] => parse_object(hash, id).map_or_else(
            || Route::NotFound(hash.to_string()),
            |id| Route::Version { id },
        ),
        ["view", "stories", id] => parse_object(hash, id).map_or_else(
            || Route::NotFound(hash.to_string()),
            |id| Route::Story { id },
        ),
        ["view", "reviews", id] => parse_object(hash, id).map_or_else(
            || Route::NotFound(hash.to_string()),
            |id| Route::Review { id },
        ),
        ["view", "at", at] => Route::ViewAt {
            at: decode(at),
            view: query
                .get("view")
                .and_then(|value| ObjectId::from_str(value).ok()),
        },
        ["graph", id] => parse_object(hash, id).map_or_else(
            || Route::NotFound(hash.to_string()),
            |id| Route::Graph { id },
        ),
        ["settings"] => Route::Settings,
        _ => Route::NotFound(hash.to_string()),
    }
}

fn parse_object(_hash: &str, value: &str) -> Option<ObjectId> {
    ObjectId::from_str(value).ok()
}

fn parse_query(query: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        map.insert(decode(key), decode(value));
    }
    map
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

impl Route {
    pub fn to_hash(&self) -> String {
        match self {
            Self::Atelier => "#/".into(),
            Self::ViewNew { intent } => match intent {
                Some(intent) => format!("#/design/new?intent={}", intent.key()),
                None => "#/design/new".into(),
            },
            Self::ViewEditor { id, link } => format!("#/design/{id}{}", link.to_query()),
            Self::ViewReader { id, link } => {
                format!("#/view/diagrams/{id}{}", link.to_query())
            }
            Self::Library { tab } => match tab {
                LibraryTab::Views => "#/library".into(),
                LibraryTab::Domains => "#/library/domains".into(),
                LibraryTab::Objects => "#/library/objects".into(),
                LibraryTab::Interfaces => "#/library/interfaces".into(),
                LibraryTab::Stories => "#/library/stories".into(),
                LibraryTab::Reviews => "#/library/reviews".into(),
            },
            Self::DomainSummary { id } => format!("#/library/domains/{id}"),
            Self::Search { q } => {
                if q.is_empty() {
                    "#/search".into()
                } else {
                    format!("#/search?q={}", encode(q))
                }
            }
            Self::Control { tab } => match tab {
                ControlTab::Anomalies => "#/control".into(),
                ControlTab::Inventory => "#/control/inventory".into(),
                ControlTab::Coverage => "#/control/coverage".into(),
                ControlTab::Exports => "#/control/exports".into(),
            },
            Self::Element { id } => format!("#/view/elements/{id}"),
            Self::Relationship { id } => format!("#/view/relationships/{id}"),
            Self::Interface { id, version } => match version {
                Some(version) => format!("#/view/interfaces/{id}?version={version}"),
                None => format!("#/view/interfaces/{id}"),
            },
            Self::Version { id } => format!("#/view/versions/{id}"),
            Self::Story { id } => format!("#/view/stories/{id}"),
            Self::Review { id } => format!("#/view/reviews/{id}"),
            Self::ViewAt { at, view } => match view {
                Some(view) => format!("#/view/at/{}?view={view}", encode(at)),
                None => format!("#/view/at/{}", encode(at)),
            },
            Self::Graph { id } => format!("#/graph/{id}"),
            Self::Settings => "#/settings".into(),
            Self::NotFound(_) => "#/".into(),
        }
    }

    /// Libellé de l'entrée de navigation active.
    pub fn is_atelier(&self) -> bool {
        matches!(
            self,
            Self::Atelier
                | Self::ViewNew { .. }
                | Self::ViewEditor { .. }
                | Self::ViewReader { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    fn oid() -> ObjectId {
        ObjectId::from_str(ID).unwrap()
    }

    #[test]
    fn home_parses_as_atelier() {
        assert_eq!(parse_hash(""), Route::Atelier);
        assert_eq!(parse_hash("#/"), Route::Atelier);
    }

    #[test]
    fn design_routes_parse() {
        assert_eq!(
            parse_hash(&format!("#/design/{ID}")),
            Route::ViewEditor {
                id: oid(),
                link: ViewLink::default()
            }
        );
        assert_eq!(
            parse_hash(&format!("#/view/diagrams/{ID}")),
            Route::ViewReader {
                id: oid(),
                link: ViewLink::default()
            }
        );
        assert_eq!(
            parse_hash("#/design/new?intent=APPLICATION_LANDSCAPE"),
            Route::ViewNew {
                intent: Some(archifier_core::ViewIntent::ApplicationLandscape)
            }
        );
    }

    #[test]
    fn model_diagrams_alias_maps_to_design() {
        assert_eq!(
            parse_hash(&format!("#/model/diagrams/{ID}")),
            Route::ViewEditor {
                id: oid(),
                link: ViewLink::default()
            }
        );
    }

    #[test]
    fn view_domains_alias_maps_to_library_domain() {
        assert_eq!(
            parse_hash(&format!("#/view/domains/{ID}")),
            Route::DomainSummary { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/library/domains/{ID}")),
            Route::DomainSummary { id: oid() }
        );
    }

    #[test]
    fn relationship_and_version_routes_parse() {
        assert_eq!(
            parse_hash(&format!("#/view/relationships/{ID}")),
            Route::Relationship { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/view/versions/{ID}")),
            Route::Version { id: oid() }
        );
    }

    #[test]
    fn share_link_roundtrip_is_reproducible() {
        let link = ViewLink {
            zoom: Some(0.9),
            x: Some(120.0),
            y: Some(40.0),
            focus: vec!["n2".into(), "n1".into(), "n2".into()],
            dim: true,
            caption: Some("Migration v1".into()),
            t: Some("2027-08-01T00:00:00Z".into()),
        };
        let route = Route::ViewReader { id: oid(), link };
        assert_eq!(
            route.to_hash(),
            format!(
                "#/view/diagrams/{ID}?zoom=0.9&x=120&y=40&focus=n1,n2&dim=1&caption=Migration%20v1&t=2027-08-01T00%3A00%3A00Z"
            )
        );
        assert_eq!(parse_hash(&route.to_hash()).to_hash(), route.to_hash());
    }

    #[test]
    fn unknown_routes_are_not_found() {
        assert!(matches!(parse_hash("#/nope"), Route::NotFound(_)));
        assert!(matches!(
            parse_hash("#/view/gadgets/01"),
            Route::NotFound(_)
        ));
    }

    #[test]
    fn library_and_control_routes() {
        assert_eq!(
            parse_hash("#/library"),
            Route::Library {
                tab: LibraryTab::Views
            }
        );
        assert_eq!(
            parse_hash("#/control/coverage"),
            Route::Control {
                tab: ControlTab::Coverage
            }
        );
    }

    #[test]
    fn hash_roundtrip_for_each_route() {
        for route in [
            Route::Atelier,
            Route::ViewNew { intent: None },
            Route::ViewEditor {
                id: oid(),
                link: ViewLink::default(),
            },
            Route::ViewReader {
                id: oid(),
                link: ViewLink::default(),
            },
            Route::Library {
                tab: LibraryTab::Views,
            },
            Route::DomainSummary { id: oid() },
            Route::Search { q: "api".into() },
            Route::Control {
                tab: ControlTab::Inventory,
            },
            Route::Element { id: oid() },
            Route::Relationship { id: oid() },
            Route::Interface {
                id: oid(),
                version: None,
            },
            Route::Version { id: oid() },
            Route::Story { id: oid() },
            Route::Review { id: oid() },
            Route::Graph { id: oid() },
            Route::Settings,
        ] {
            assert_eq!(
                parse_hash(&route.to_hash()),
                route,
                "aller-retour {}",
                route.to_hash()
            );
        }
    }
}
