//! Routage par fragment d'URL (`#/design/01J…`), conformément à ADR-012.
//!
//! Les chemins de la spécification (§15.2) sont des chemins logiques ; la
//! forme réelle est `#/<chemin>`. Les alias (`/model/diagrams/:id`,
//! `/view/domains/:id`) restent résolus sans redirection d'historique.

use archifier_core::catalog::{ProductGap, ProductQuery, ProductSort};
use archifier_core::map::{
    Criticality, InterfaceExposure, InterfaceLevel, LifecycleState, MapQuery, ProductKind,
};
use archifier_core::ObjectId;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryTab {
    Views,
    Products,
    Teams,
    ValueStreams,
    Capabilities,
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
    /// Portefeuille de produits, accueil `/` (ADR-025).
    Home(ProductQuery),
    /// Carte de l'entreprise et navigation par zoom (ADR-023/025).
    Map(MapQuery),
    Product {
        id: ObjectId,
    },
    Team {
        id: ObjectId,
    },
    ValueStream {
        id: ObjectId,
    },
    ValueStreamStage {
        value_stream: ObjectId,
        stage: ObjectId,
    },
    Capability {
        id: ObjectId,
    },
    Application {
        id: ObjectId,
    },
    ApplicationComponent {
        application: ObjectId,
        component: ObjectId,
    },
    Infrastructure {
        id: ObjectId,
    },
    MapInterface {
        id: ObjectId,
    },
    BusinessDomain {
        id: ObjectId,
    },
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
        [] => Route::Home(product_query_from_params(&query)),
        ["atelier"] => Route::Atelier,
        ["map"] | ["map", "enterprise"] => Route::Map(MapQuery::from_params(
            query
                .get("scope")
                .map(String::as_str)
                .unwrap_or("ENTERPRISE"),
            query
                .get("level")
                .map(String::as_str)
                .unwrap_or("ENTERPRISE"),
            query.get("lens").map(String::as_str).unwrap_or("PRODUCT"),
            query.get("date").map(String::as_str),
            query.get("focus").map(String::as_str).unwrap_or_default(),
            query.get("mode").map(String::as_str).unwrap_or("read"),
        )),
        ["products"] => Route::Home(product_query_from_params(&query)),
        ["products", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::Product { id },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["teams", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::Team { id },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["value-streams", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::ValueStream { id },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["value-streams", value_stream, "stages", stage] => {
            match (ObjectId::from_str(value_stream), ObjectId::from_str(stage)) {
                (Ok(value_stream), Ok(stage)) => Route::ValueStreamStage {
                    value_stream,
                    stage,
                },
                _ => Route::NotFound(hash.to_string()),
            }
        }
        ["capabilities", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::Capability { id },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["applications", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::Application { id },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["applications", application, "components", component] => match (
            ObjectId::from_str(application),
            ObjectId::from_str(component),
        ) {
            (Ok(application), Ok(component)) => Route::ApplicationComponent {
                application,
                component,
            },
            _ => Route::NotFound(hash.to_string()),
        },
        ["infrastructure", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::Infrastructure { id },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["interfaces", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::MapInterface { id },
            Err(_) => Route::NotFound(hash.to_string()),
        },
        ["business-domains", id] | ["domains", id] => match ObjectId::from_str(id) {
            Ok(id) => Route::BusinessDomain { id },
            Err(_) => Route::NotFound(hash.to_string()),
        },
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
        ["library", "products"] => Route::Library {
            tab: LibraryTab::Products,
        },
        ["library", "teams"] => Route::Library {
            tab: LibraryTab::Teams,
        },
        ["library", "value-streams"] => Route::Library {
            tab: LibraryTab::ValueStreams,
        },
        ["library", "capabilities"] => Route::Library {
            tab: LibraryTab::Capabilities,
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
            Ok(id) => Route::BusinessDomain { id },
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

fn product_query_from_params(query: &BTreeMap<String, String>) -> ProductQuery {
    ProductQuery {
        search: query.get("q").cloned().unwrap_or_default(),
        team_ids: parse_ids(query.get("team")),
        kinds: parse_values(query.get("kind"), ProductKind::parse),
        lifecycles: parse_values(query.get("lifecycle"), LifecycleState::parse),
        criticalities: parse_values(query.get("criticality"), Criticality::parse),
        value_stream_ids: parse_ids(query.get("vs")),
        capability_ids: parse_ids(query.get("capability")),
        application_ids: parse_ids(query.get("application")),
        interface_levels: parse_values(query.get("ilevel"), InterfaceLevel::parse),
        exposures: parse_values(query.get("exposure"), InterfaceExposure::parse),
        gaps: parse_values(query.get("gap"), ProductGap::parse),
        sort: query
            .get("sort")
            .and_then(|value| ProductSort::parse(value))
            .unwrap_or_default(),
    }
}

fn parse_ids(value: Option<&String>) -> Vec<ObjectId> {
    value
        .map(|value| {
            value
                .split(',')
                .filter_map(|part| ObjectId::from_str(part.trim()).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_values<T>(value: Option<&String>, parse: impl Fn(&str) -> Option<T>) -> Vec<T> {
    value
        .map(|value| {
            value
                .split(',')
                .filter_map(|part| parse(part.trim()))
                .collect()
        })
        .unwrap_or_default()
}

fn product_query_params(query: &ProductQuery) -> Vec<String> {
    let mut parts = Vec::new();
    if !query.search.trim().is_empty() {
        parts.push(format!("q={}", encode(query.search.trim())));
    }
    if !query.team_ids.is_empty() {
        parts.push(format!("team={}", join_ids(&query.team_ids)));
    }
    if !query.kinds.is_empty() {
        parts.push(format!(
            "kind={}",
            join_values(&query.kinds, |value| value.as_str())
        ));
    }
    if !query.lifecycles.is_empty() {
        parts.push(format!(
            "lifecycle={}",
            join_values(&query.lifecycles, |value| value.as_str())
        ));
    }
    if !query.criticalities.is_empty() {
        parts.push(format!(
            "criticality={}",
            join_values(&query.criticalities, |value| value.as_str())
        ));
    }
    if !query.value_stream_ids.is_empty() {
        parts.push(format!("vs={}", join_ids(&query.value_stream_ids)));
    }
    if !query.capability_ids.is_empty() {
        parts.push(format!("capability={}", join_ids(&query.capability_ids)));
    }
    if !query.application_ids.is_empty() {
        parts.push(format!("application={}", join_ids(&query.application_ids)));
    }
    if !query.interface_levels.is_empty() {
        parts.push(format!(
            "ilevel={}",
            join_values(&query.interface_levels, |value| value.as_str())
        ));
    }
    if !query.exposures.is_empty() {
        parts.push(format!(
            "exposure={}",
            join_values(&query.exposures, |value| value.as_str())
        ));
    }
    if !query.gaps.is_empty() {
        parts.push(format!(
            "gap={}",
            join_values(&query.gaps, |value| value.as_str())
        ));
    }
    if query.sort != ProductSort::Name {
        parts.push(format!("sort={}", query.sort.as_str()));
    }
    parts
}

fn join_ids(ids: &[ObjectId]) -> String {
    ids.iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn join_values<T: Copy>(values: &[T], as_str: impl Fn(T) -> &'static str) -> String {
    values
        .iter()
        .map(|value| as_str(*value))
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_query(query: &str) -> BTreeMap<String, String> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        let key = decode(key);
        let value = decode(value);
        // `lens` est répétable dans C-10 ; on conserve toutes les valeurs sous
        // une forme qui reste lisible dans un lien partagé.
        if key == "lens" && map.contains_key(&key) {
            if let Some(existing) = map.get_mut(&key) {
                if !existing.is_empty() {
                    existing.push(',');
                }
                existing.push_str(&value);
            }
        } else {
            map.insert(key, value);
        }
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
            Self::Home(query) => {
                let parts = product_query_params(query);
                if parts.is_empty() {
                    "#/".into()
                } else {
                    format!("#/?{}", parts.join("&"))
                }
            }
            Self::Map(query) => {
                if query == &MapQuery::map_default() {
                    "#/map".into()
                } else {
                    format!("#/map?{}", query.to_query_string())
                }
            }
            Self::Product { id } => format!("#/products/{id}"),
            Self::Team { id } => format!("#/teams/{id}"),
            Self::ValueStream { id } => format!("#/value-streams/{id}"),
            Self::ValueStreamStage {
                value_stream,
                stage,
            } => {
                format!("#/value-streams/{value_stream}/stages/{stage}")
            }
            Self::Capability { id } => format!("#/capabilities/{id}"),
            Self::Application { id } => format!("#/applications/{id}"),
            Self::ApplicationComponent {
                application,
                component,
            } => format!("#/applications/{application}/components/{component}"),
            Self::Infrastructure { id } => format!("#/infrastructure/{id}"),
            Self::MapInterface { id } => format!("#/interfaces/{id}"),
            Self::BusinessDomain { id } => format!("#/business-domains/{id}"),
            Self::Atelier => "#/atelier".into(),
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
                LibraryTab::Products => "#/library/products".into(),
                LibraryTab::Teams => "#/library/teams".into(),
                LibraryTab::ValueStreams => "#/library/value-streams".into(),
                LibraryTab::Capabilities => "#/library/capabilities".into(),
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

    pub fn is_products(&self) -> bool {
        matches!(self, Self::Home(_) | Self::Product { .. })
    }

    pub fn is_map(&self) -> bool {
        matches!(
            self,
            Self::Map(_)
                | Self::Team { .. }
                | Self::ValueStream { .. }
                | Self::ValueStreamStage { .. }
                | Self::Capability { .. }
                | Self::Application { .. }
                | Self::ApplicationComponent { .. }
                | Self::Infrastructure { .. }
                | Self::MapInterface { .. }
                | Self::BusinessDomain { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archifier_core::map::{MapLens, MapLevel};

    const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    fn oid() -> ObjectId {
        ObjectId::from_str(ID).unwrap()
    }

    #[test]
    fn home_opens_product_catalog() {
        assert_eq!(parse_hash(""), Route::Home(ProductQuery::home()));
        assert_eq!(parse_hash("#/"), Route::Home(ProductQuery::home()));
        assert_eq!(parse_hash("#/atelier"), Route::Atelier);
    }

    #[test]
    fn map_opens_enterprise_map() {
        for hash in ["#/map", "#/map/enterprise"] {
            match parse_hash(hash) {
                Route::Map(query) => {
                    assert_eq!(
                        query.scope.kind,
                        archifier_core::map::MapScopeKind::Enterprise
                    );
                    assert_eq!(query.level, MapLevel::Enterprise);
                    assert_eq!(query.primary_lens(), MapLens::Product);
                }
                other => panic!("route inattendue pour {hash:?} : {other:?}"),
            }
        }
    }

    #[test]
    fn home_deep_link_restores_product_filters() {
        let route = parse_hash(&format!(
            "#/?team={ID}&kind=PLATFORM&lifecycle=RUN&criticality=HIGH&sort=CRITICALITY"
        ));
        match route {
            Route::Home(query) => {
                assert_eq!(query.team_ids, vec![oid()]);
                assert_eq!(query.kinds, vec![ProductKind::Platform]);
                assert_eq!(query.lifecycles, vec![LifecycleState::Run]);
                assert_eq!(query.criticalities, vec![Criticality::High]);
                assert_eq!(query.sort, ProductSort::Criticality);
            }
            other => panic!("route inattendue : {other:?}"),
        }
    }

    #[test]
    fn map_deep_link_restores_context() {
        let route = parse_hash(&format!(
            "#/map?scope=PRODUCT:{}&level=PRODUCT&lens=INTERFACE&lens=TIME&date=2026-11-19&focus={}",
            ID, ID
        ));
        match route {
            Route::Map(query) => {
                assert_eq!(query.scope.kind, archifier_core::map::MapScopeKind::Product);
                assert_eq!(query.level, MapLevel::Product);
                assert_eq!(query.lenses, vec![MapLens::Interface, MapLens::Time]);
                assert_eq!(query.date.as_deref(), Some("2026-11-19"));
                assert_eq!(query.focus, vec![oid()]);
            }
            other => panic!("route inattendue : {other:?}"),
        }
    }

    #[test]
    fn map_entity_routes_parse() {
        assert_eq!(
            parse_hash(&format!("#/products/{ID}")),
            Route::Product { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/teams/{ID}")),
            Route::Team { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/value-streams/{ID}")),
            Route::ValueStream { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/applications/{ID}")),
            Route::Application { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/infrastructure/{ID}")),
            Route::Infrastructure { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/interfaces/{ID}")),
            Route::MapInterface { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/capabilities/{ID}")),
            Route::Capability { id: oid() }
        );
    }

    #[test]
    fn legacy_domain_routes_still_resolve() {
        assert_eq!(
            parse_hash(&format!("#/library/domains/{ID}")),
            Route::BusinessDomain { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/view/domains/{ID}")),
            Route::BusinessDomain { id: oid() }
        );
        assert_eq!(
            parse_hash(&format!("#/business-domains/{ID}")),
            Route::BusinessDomain { id: oid() }
        );
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
    fn library_map_tabs_parse() {
        for (raw, tab) in [
            ("#/library/products", LibraryTab::Products),
            ("#/library/teams", LibraryTab::Teams),
            ("#/library/value-streams", LibraryTab::ValueStreams),
            ("#/library/capabilities", LibraryTab::Capabilities),
        ] {
            assert_eq!(parse_hash(raw), Route::Library { tab });
        }
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
            Route::Home(ProductQuery::home()),
            {
                let mut query = ProductQuery::home();
                query.team_ids = vec![oid()];
                query.kinds = vec![ProductKind::Platform];
                query.sort = ProductSort::Criticality;
                Route::Home(query)
            },
            Route::Map(MapQuery::enterprise()),
            Route::Product { id: oid() },
            Route::Team { id: oid() },
            Route::ValueStream { id: oid() },
            Route::ValueStreamStage {
                value_stream: oid(),
                stage: oid(),
            },
            Route::Capability { id: oid() },
            Route::Application { id: oid() },
            Route::ApplicationComponent {
                application: oid(),
                component: oid(),
            },
            Route::Infrastructure { id: oid() },
            Route::MapInterface { id: oid() },
            Route::BusinessDomain { id: oid() },
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
