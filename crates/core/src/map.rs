//! Cible v0.4 — ontologie produit et projection de carte (C-10, ADR-023).
//!
//! Ce module est volontairement pur : aucun accès disque, aucune horloge
//! implicite, aucune dépendance UI. Il fixe ce qui doit être vrai du modèle et
//! de la carte, indépendamment du rendu.
//!
//! Deux vocabulaires coexistent et ne fusionnent jamais :
//! - `Product`, `Team`, `ValueStream`… forment la couche **native** de
//!   responsabilité et de navigation ;
//! - les éléments ArchiMate/BPMN/UML restent la couche de notation.
//!
//! La géographie de la carte est calculée à partir des objets et des ancres ;
//! une lentille ne change que l'encodage, jamais `x`/`y`.

use crate::ids::ObjectId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

// ---------------------------------------------------------------------------
// Énumérations opposables (C-10 §3)
// ---------------------------------------------------------------------------

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $text:literal => $label:literal),+ $(,)? } default $default:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),+
        }

        impl Default for $name {
            fn default() -> Self {
                Self::$default
            }
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn label(self) -> &'static str {
                match self {
                    $(Self::$variant => $label),+
                }
            }

            pub fn parse(value: &str) -> Option<Self> {
                match value.trim().to_ascii_uppercase().as_str() {
                    $($text => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

string_enum!(ProductKind {
    Customer => "CUSTOMER" => "Client",
    Internal => "INTERNAL" => "Interne",
    Platform => "PLATFORM" => "Plateforme",
    Data => "DATA" => "Données",
    Partner => "PARTNER" => "Partenaire",
} default Internal);

string_enum!(LifecycleState {
    Idea => "IDEA" => "Idée",
    Discovery => "DISCOVERY" => "Découverte",
    Build => "BUILD" => "Construction",
    Run => "RUN" => "En run",
    Sunset => "SUNSET" => "En retrait",
    Retired => "RETIRED" => "Retiré",
} default Run);

string_enum!(Criticality {
    Critical => "CRITICAL" => "Critique",
    High => "HIGH" => "Élevée",
    Medium => "MEDIUM" => "Moyenne",
    Low => "LOW" => "Faible",
} default Medium);

string_enum!(TeamTopology {
    StreamAligned => "STREAM_ALIGNED" => "Stream-aligned",
    Platform => "PLATFORM" => "Plateforme",
    Enabling => "ENABLING" => "Enabling",
    ComplicatedSubsystem => "COMPLICATED_SUBSYSTEM" => "Sous-système compliqué",
    Other => "OTHER" => "Autre",
} default StreamAligned);

string_enum!(TeamInteractionMode {
    Collaboration => "COLLABORATION" => "Collaboration",
    XAsAService => "X_AS_A_SERVICE" => "X-as-a-Service",
    Facilitation => "FACILITATION" => "Facilitation",
} default Collaboration);

string_enum!(ApplicationKind {
    Internal => "INTERNAL" => "Interne",
    Cots => "COTS" => "COTS",
    Saas => "SAAS" => "SaaS",
    Platform => "PLATFORM" => "Plateforme",
    Legacy => "LEGACY" => "Héritage",
    External => "EXTERNAL" => "Externe",
} default Internal);

string_enum!(InterfaceLevel {
    Business => "BUSINESS" => "Métier",
    Application => "APPLICATION" => "Applicatif",
    Infrastructure => "INFRASTRUCTURE" => "Infrastructure",
} default Business);

string_enum!(InterfaceCategory {
    Business => "BUSINESS" => "Métier",
    Application => "APPLICATION" => "Applicatif",
    Technical => "TECHNICAL" => "Technique",
} default Business);

string_enum!(InterfaceKind {
    Api => "API" => "API",
    Event => "EVENT" => "Événement",
    File => "FILE" => "Fichier",
    Batch => "BATCH" => "Batch",
    Ui => "UI" => "UI",
    Database => "DATABASE" => "Base de données",
    Network => "NETWORK" => "Réseau",
    Other => "OTHER" => "Autre",
} default Api);

string_enum!(InterfaceExposure {
    Internal => "INTERNAL" => "Interne",
    Product => "PRODUCT" => "Produit",
    Partner => "PARTNER" => "Partenaire",
    Public => "PUBLIC" => "Public",
} default Internal);

string_enum!(DataClassification {
    Public => "PUBLIC" => "Public",
    Internal => "INTERNAL" => "Interne",
    Confidential => "CONFIDENTIAL" => "Confidentiel",
    Secret => "SECRET" => "Secret",
} default Internal);

string_enum!(MapLevel {
    Enterprise => "ENTERPRISE" => "Entreprise",
    Portfolio => "PORTFOLIO" => "Portefeuille",
    Product => "PRODUCT" => "Produit",
    Application => "APPLICATION" => "Application",
    Infrastructure => "INFRASTRUCTURE" => "Infrastructure",
} default Enterprise);

string_enum!(MapLens {
    Value => "VALUE" => "Valeur",
    Capability => "CAPABILITY" => "Capacité",
    Product => "PRODUCT" => "Produit",
    Organization => "ORGANIZATION" => "Organisation",
    Application => "APPLICATION" => "Applicatif",
    Interface => "INTERFACE" => "Interface",
    Technology => "TECHNOLOGY" => "Technologie",
    Data => "DATA" => "Données",
    Time => "TIME" => "Temps",
    Risk => "RISK" => "Risque",
} default Product);

string_enum!(MapScopeKind {
    Enterprise => "ENTERPRISE" => "Entreprise",
    ValueStream => "VALUE_STREAM" => "Value stream",
    Portfolio => "PORTFOLIO" => "Portefeuille",
    Product => "PRODUCT" => "Produit",
    Application => "APPLICATION" => "Application",
    Node => "NODE" => "Nœud",
} default Enterprise);

string_enum!(MapEdgeKind {
    ProductInterface => "PRODUCT_INTERFACE" => "Interface produit",
    ApplicationInterface => "APPLICATION_INTERFACE" => "Interface applicative",
    ComponentInterface => "COMPONENT_INTERFACE" => "Interface composant",
    TechnicalInterface => "TECHNICAL_INTERFACE" => "Interface technique",
    Deployment => "DEPLOYMENT" => "Déploiement",
    ValueContribution => "VALUE_CONTRIBUTION" => "Contribution de valeur",
    CapabilitySupport => "CAPABILITY_SUPPORT" => "Support de capacité",
    DataFlow => "DATA_FLOW" => "Flux de données",
} default ProductInterface);

string_enum!(MapMode {
    Read => "READ" => "Lecture",
    Edit => "EDIT" => "Édition",
} default Read);

string_enum!(TechnologyNodeKind {
    Platform => "PLATFORM" => "Plateforme",
    Server => "SERVER" => "Serveur",
    Cluster => "CLUSTER" => "Cluster",
    Network => "NETWORK" => "Réseau",
    Database => "DATABASE" => "Base de données",
    Environment => "ENVIRONMENT" => "Environnement",
    Other => "OTHER" => "Autre",
} default Server);

string_enum!(Environment {
    Dev => "DEV" => "Dev",
    Test => "TEST" => "Test",
    Staging => "STAGING" => "Recette",
    Prod => "PROD" => "Production",
    Dr => "DR" => "PRA",
} default Prod);

string_enum!(ComponentKind {
    Component => "COMPONENT" => "Composant",
    Service => "SERVICE" => "Service",
    Batch => "BATCH" => "Batch",
    Adapter => "ADAPTER" => "Adaptateur",
    Database => "DATABASE" => "Base de données",
    Ui => "UI" => "UI",
} default Component);

string_enum!(PortfolioKind {
    ValueStream => "VALUE_STREAM" => "Value stream",
    BusinessUnit => "BUSINESS_UNIT" => "Unité métier",
    Theme => "THEME" => "Thème",
    PlatformGroup => "PLATFORM_GROUP" => "Groupe de plateformes",
    ProductLine => "PRODUCT_LINE" => "Ligne de produits",
} default Theme);

string_enum!(InitiativeKind {
    Migration => "MIGRATION" => "Migration",
    Program => "PROGRAM" => "Programme",
    Wave => "WAVE" => "Vague",
    Project => "PROJECT" => "Projet",
} default Program);

string_enum!(PublicationState {
    Draft => "DRAFT" => "Brouillon",
    Published => "PUBLISHED" => "Publiée",
    Withdrawn => "WITHDRAWN" => "Retirée",
} default Draft);

string_enum!(ContractType {
    OpenApi => "OPENAPI" => "OpenAPI",
    AsyncApi => "ASYNCAPI" => "AsyncAPI",
    Wsdl => "WSDL" => "WSDL",
    File => "FILE" => "Fichier",
    Other => "OTHER" => "Autre",
} default Other);

string_enum!(MapObjectKind {
    Product => "PRODUCT" => "Produit",
    Application => "APPLICATION" => "Application",
    Component => "COMPONENT" => "Composant",
    Node => "NODE" => "Nœud",
    Capability => "CAPABILITY" => "Capacité",
    Team => "TEAM" => "Équipe",
    Stage => "STAGE" => "Étape",
    Interface => "INTERFACE" => "Interface",
} default Product);

string_enum!(DistrictKind {
    ValueStream => "VALUE_STREAM" => "Value stream",
    Portfolio => "PORTFOLIO" => "Portefeuille",
    BusinessUnit => "BUSINESS_UNIT" => "Unité métier",
    ProductGroup => "PRODUCT_GROUP" => "Groupe de produits",
    PlatformGroup => "PLATFORM_GROUP" => "Groupe de plateformes",
    Environment => "ENVIRONMENT" => "Environnement",
    Unassigned => "UNASSIGNED" => "Non rattaché",
} default ValueStream);
impl MapLevel {
    pub fn rank(self) -> u8 {
        match self {
            Self::Enterprise => 0,
            Self::Portfolio => 1,
            Self::Product => 2,
            Self::Application => 3,
            Self::Infrastructure => 4,
        }
    }

    pub fn next(self) -> Option<Self> {
        match self {
            Self::Enterprise => Some(Self::Portfolio),
            Self::Portfolio => Some(Self::Product),
            Self::Product => Some(Self::Application),
            Self::Application => Some(Self::Infrastructure),
            Self::Infrastructure => None,
        }
    }

    pub fn previous(self) -> Option<Self> {
        match self {
            Self::Enterprise => None,
            Self::Portfolio => Some(Self::Enterprise),
            Self::Product => Some(Self::Portfolio),
            Self::Application => Some(Self::Product),
            Self::Infrastructure => Some(Self::Application),
        }
    }

    /// Le niveau le plus proche compatible avec un périmètre (`MAP-002`).
    pub fn closest_for(scope: MapScopeKind) -> Self {
        match scope {
            MapScopeKind::Enterprise => Self::Enterprise,
            MapScopeKind::ValueStream | MapScopeKind::Portfolio => Self::Portfolio,
            MapScopeKind::Product => Self::Product,
            MapScopeKind::Application => Self::Application,
            MapScopeKind::Node => Self::Infrastructure,
        }
    }

    pub fn compatible_with(self, scope: MapScopeKind) -> bool {
        // Un niveau n'est proposé que si le périmètre porte l'objet racine de
        // ce niveau. On descend d'un niveau en cliquant un objet, jamais en
        // changeant d'onglet sur un périmètre incompatible.
        match scope {
            MapScopeKind::Enterprise => matches!(self, Self::Enterprise | Self::Portfolio),
            // Un value stream ou un portefeuille ne porte pas d'identité de
            // produit : le niveau Produit exigerait un scope PRODUCT. Le zoom
            // vers une fiche produit se fait par clic sur un nœud, jamais par
            // un onglet de niveau.
            MapScopeKind::ValueStream | MapScopeKind::Portfolio => self == Self::Portfolio,
            MapScopeKind::Product => self == Self::Product,
            MapScopeKind::Application => self == Self::Application,
            MapScopeKind::Node => self == Self::Infrastructure,
        }
    }
}

impl MapLens {
    pub fn default_for(level: MapLevel) -> Self {
        match level {
            MapLevel::Enterprise | MapLevel::Portfolio => Self::Product,
            MapLevel::Product | MapLevel::Application => Self::Interface,
            MapLevel::Infrastructure => Self::Technology,
        }
    }

    pub fn available_at(self, level: MapLevel) -> bool {
        match self {
            Self::Value | Self::Capability | Self::Product | Self::Organization | Self::Risk => {
                matches!(
                    level,
                    MapLevel::Enterprise | MapLevel::Portfolio | MapLevel::Product
                )
            }
            Self::Application => matches!(
                level,
                MapLevel::Portfolio | MapLevel::Product | MapLevel::Application
            ),
            Self::Interface => true,
            Self::Technology => matches!(
                level,
                MapLevel::Product | MapLevel::Application | MapLevel::Infrastructure
            ),
            Self::Data => matches!(
                level,
                MapLevel::Portfolio
                    | MapLevel::Product
                    | MapLevel::Application
                    | MapLevel::Infrastructure
            ),
            Self::Time => true,
        }
    }

    pub fn unavailable_reason(self, level: MapLevel) -> Option<String> {
        if self.available_at(level) {
            None
        } else {
            Some(format!(
                "La lentille {} n'est pas disponible au niveau {}.",
                self.as_str(),
                level.as_str()
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Périmètre et requête
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapScope {
    pub kind: MapScopeKind,
    pub id: Option<ObjectId>,
}

impl Default for MapScope {
    fn default() -> Self {
        Self {
            kind: MapScopeKind::Enterprise,
            id: None,
        }
    }
}

impl MapScope {
    pub fn enterprise() -> Self {
        Self::default()
    }

    pub fn value_stream(id: ObjectId) -> Self {
        Self {
            kind: MapScopeKind::ValueStream,
            id: Some(id),
        }
    }

    pub fn portfolio(id: ObjectId) -> Self {
        Self {
            kind: MapScopeKind::Portfolio,
            id: Some(id),
        }
    }

    pub fn product(id: ObjectId) -> Self {
        Self {
            kind: MapScopeKind::Product,
            id: Some(id),
        }
    }

    pub fn application(id: ObjectId) -> Self {
        Self {
            kind: MapScopeKind::Application,
            id: Some(id),
        }
    }

    pub fn node(id: ObjectId) -> Self {
        Self {
            kind: MapScopeKind::Node,
            id: Some(id),
        }
    }

    /// Forme courte utilisée dans les liens : `ENTERPRISE` ou `KIND:id`.
    pub fn to_param(self) -> String {
        match (self.kind, self.id) {
            (MapScopeKind::Enterprise, _) => "ENTERPRISE".into(),
            (kind, Some(id)) => format!("{}:{id}", kind.as_str()),
            (kind, None) => kind.as_str().into(),
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.is_empty() {
            return Some(Self::enterprise());
        }
        if let Some((kind, id)) = value.split_once(':') {
            let kind = MapScopeKind::parse(kind)?;
            let id = id.parse::<ObjectId>().ok()?;
            if kind == MapScopeKind::Enterprise {
                return None;
            }
            return Some(Self { kind, id: Some(id) });
        }
        match MapScopeKind::parse(value) {
            Some(MapScopeKind::Enterprise) => Some(Self::enterprise()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapFilter {
    pub product_kinds: Vec<ProductKind>,
    pub lifecycles: Vec<LifecycleState>,
    pub criticalities: Vec<Criticality>,
    pub team_ids: Vec<ObjectId>,
    pub interface_levels: Vec<InterfaceLevel>,
    pub interface_kinds: Vec<InterfaceKind>,
    pub exposures: Vec<InterfaceExposure>,
    pub include_external: bool,
    pub include_expired: bool,
}

impl Default for MapFilter {
    fn default() -> Self {
        Self {
            product_kinds: vec![],
            lifecycles: vec![],
            criticalities: vec![],
            team_ids: vec![],
            interface_levels: vec![],
            interface_kinds: vec![],
            exposures: vec![],
            include_external: true,
            include_expired: false,
        }
    }
}

impl MapFilter {
    pub fn is_empty(&self) -> bool {
        self.product_kinds.is_empty()
            && self.lifecycles.is_empty()
            && self.criticalities.is_empty()
            && self.team_ids.is_empty()
            && self.interface_levels.is_empty()
            && self.interface_kinds.is_empty()
            && self.exposures.is_empty()
            && self.include_external
            && !self.include_expired
    }

    fn accepts_product(&self, product: &Product) -> bool {
        (self.product_kinds.is_empty() || self.product_kinds.contains(&product.kind))
            && (self.lifecycles.is_empty() || self.lifecycles.contains(&product.lifecycle))
            && (self.criticalities.is_empty() || self.criticalities.contains(&product.criticality))
            && (self.team_ids.is_empty()
                || product
                    .accountable_team_id
                    .is_some_and(|id| self.team_ids.contains(&id)))
    }

    fn accepts_interface(&self, interface: &MapInterface) -> bool {
        (self.interface_levels.is_empty() || self.interface_levels.contains(&interface.level))
            && (self.interface_kinds.is_empty() || self.interface_kinds.contains(&interface.kind))
            && (self.exposures.is_empty() || self.exposures.contains(&interface.exposure))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapQuery {
    pub scope: MapScope,
    pub level: MapLevel,
    pub lenses: Vec<MapLens>,
    pub date: Option<String>,
    pub focus: Vec<ObjectId>,
    pub filters: MapFilter,
    pub mode: MapMode,
}

impl Default for MapQuery {
    fn default() -> Self {
        Self {
            scope: MapScope::enterprise(),
            level: MapLevel::Enterprise,
            lenses: vec![MapLens::Product],
            date: None,
            focus: vec![],
            filters: MapFilter::default(),
            mode: MapMode::Read,
        }
    }
}

impl MapQuery {
    /// Requête par défaut de la carte (`/map`) : l'entreprise lue par ses produits.
    ///
    /// La carte n'est plus l'accueil depuis ADR-025 : l'accueil est le catalogue
    /// produits. Un lien profond qui précise `scope`, `level` ou `lens` reste
    /// prioritaire.
    pub fn map_default() -> Self {
        Self {
            scope: MapScope::enterprise(),
            level: MapLevel::Enterprise,
            lenses: vec![MapLens::Product],
            date: None,
            focus: vec![],
            filters: MapFilter::default(),
            mode: MapMode::Read,
        }
    }

    pub fn enterprise() -> Self {
        Self::default()
    }

    pub fn level(mut self, level: MapLevel) -> Self {
        self.level = level;
        self.lenses = vec![MapLens::default_for(level)];
        self
    }

    pub fn at_scope(mut self, scope: MapScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_lens(mut self, lens: MapLens) -> Self {
        if !self.lenses.contains(&lens) {
            self.lenses.push(lens);
        }
        self
    }

    pub fn with_date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }

    pub fn with_focus(mut self, focus: Vec<ObjectId>) -> Self {
        self.focus = focus;
        self
    }

    pub fn edit(mut self) -> Self {
        self.mode = MapMode::Edit;
        self
    }

    pub fn primary_lens(&self) -> MapLens {
        self.lenses
            .first()
            .copied()
            .unwrap_or_else(|| MapLens::default_for(self.level))
    }

    /// Normalise la requête et retourne les codes de correction (`MAP-00x`).
    ///
    /// Aucune erreur n'est fatale : un périmètre inconnu retombe sur
    /// l'entreprise, un niveau incompatible sur le niveau le plus proche, une
    /// lentille indisponible est retirée. Le lien reste toujours exploitable.
    pub fn normalized(&self, data: &MapDataset) -> (MapQuery, Vec<MapAnomaly>) {
        let mut anomalies = Vec::new();
        let mut query = self.clone();

        let scope_ok = match query.scope.kind {
            MapScopeKind::Enterprise => true,
            MapScopeKind::ValueStream => query
                .scope
                .id
                .is_some_and(|id| data.value_streams.iter().any(|v| v.id == id)),
            MapScopeKind::Portfolio => query
                .scope
                .id
                .is_some_and(|id| data.portfolios.iter().any(|p| p.id == id)),
            MapScopeKind::Product => query
                .scope
                .id
                .is_some_and(|id| data.products.iter().any(|p| p.id == id)),
            MapScopeKind::Application => query
                .scope
                .id
                .is_some_and(|id| data.applications.iter().any(|p| p.id == id)),
            MapScopeKind::Node => query
                .scope
                .id
                .is_some_and(|id| data.nodes.iter().any(|p| p.id == id)),
        };
        if !scope_ok {
            anomalies.push(MapAnomaly::new(
                "MAP-001",
                "Périmètre inconnu : retour à la carte de l'entreprise.",
                false,
                query.scope.id,
            ));
            query.scope = MapScope::enterprise();
        }

        if !query.level.compatible_with(query.scope.kind) {
            let closest = MapLevel::closest_for(query.scope.kind);
            anomalies.push(MapAnomaly::new(
                "MAP-002",
                format!(
                    "Niveau {} incompatible avec le périmètre : niveau {} appliqué.",
                    query.level.as_str(),
                    closest.as_str()
                ),
                false,
                query.scope.id,
            ));
            query.level = closest;
        }

        if query.lenses.is_empty() {
            query.lenses = vec![MapLens::default_for(query.level)];
        }
        let mut kept = Vec::new();
        for lens in query.lenses {
            if lens.available_at(query.level) {
                if !kept.contains(&lens) {
                    kept.push(lens);
                }
            } else {
                anomalies.push(MapAnomaly::new(
                    "MAP-004",
                    lens.unavailable_reason(query.level).unwrap_or_default(),
                    false,
                    None,
                ));
            }
        }
        if kept.is_empty() {
            kept.push(MapLens::default_for(query.level));
        }
        query.lenses = kept;

        if let Some(date) = &query.date {
            if !is_valid_date(date) {
                anomalies.push(MapAnomaly::new(
                    "MAP-006",
                    format!("Date illisible « {date} » : carte sans curseur temporel."),
                    false,
                    None,
                ));
                query.date = None;
            }
        }

        (query, anomalies)
    }

    /// Construit une requête depuis les paramètres d'un lien profond.
    pub fn from_params(
        scope: &str,
        level: &str,
        lens: &str,
        date: Option<&str>,
        focus: &str,
        mode: &str,
    ) -> Self {
        let scope = MapScope::parse(scope).unwrap_or_default();
        let level = MapLevel::parse(level).unwrap_or_else(|| MapLevel::closest_for(scope.kind));
        let lenses: Vec<MapLens> = lens
            .split(',')
            .filter_map(|value| MapLens::parse(value))
            .collect();
        let lenses = if lenses.is_empty() {
            vec![MapLens::default_for(level)]
        } else {
            lenses
        };
        let focus = focus
            .split(',')
            .filter_map(|value| value.trim().parse::<ObjectId>().ok())
            .collect();
        Self {
            scope,
            level,
            lenses,
            date: date.map(|value| value.to_string()),
            focus,
            filters: MapFilter::default(),
            mode: if mode.eq_ignore_ascii_case("edit") {
                MapMode::Edit
            } else {
                MapMode::Read
            },
        }
    }

    /// Forme canonique du lien profond (`MAP-009`).
    pub fn to_query_string(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!("scope={}", self.scope.to_param()));
        parts.push(format!("level={}", self.level.as_str()));
        parts.push(format!(
            "lens={}",
            self.lenses
                .iter()
                .map(|lens| lens.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ));
        if let Some(date) = &self.date {
            parts.push(format!("date={}", encode(date)));
        }
        if !self.focus.is_empty() {
            parts.push(format!(
                "focus={}",
                self.focus
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if self.mode == MapMode::Edit {
            parts.push("mode=edit".into());
        }
        parts.join("&")
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

/// Date utilisable par le curseur : `AAAA-MM-JJ` ou RFC 3339.
pub fn is_valid_date(value: &str) -> bool {
    let value = value.trim();
    if value.len() < 10 {
        return false;
    }
    let head = &value[..10];
    let mut parts = head.split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<i32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    (1..=12).contains(&month) && (1..=31).contains(&day) && (1900..=2400).contains(&year)
}

fn semver_key(value: &str) -> (u64, u64, u64) {
    let mut parts = value.split('.');
    (
        parts.next().and_then(|part| part.parse().ok()).unwrap_or(0),
        parts.next().and_then(|part| part.parse().ok()).unwrap_or(0),
        parts.next().and_then(|part| part.parse().ok()).unwrap_or(0),
    )
}

fn date_only(value: &str) -> &str {
    let value = value.trim();
    if value.len() >= 10 {
        &value[..10]
    } else {
        value
    }
}

// ---------------------------------------------------------------------------
// Objets natifs (C-10 §4)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub id: ObjectId,
    pub name: String,
    pub kind: ProductKind,
    pub lifecycle: LifecycleState,
    pub criticality: Criticality,
    pub parent_product_id: Option<ObjectId>,
    pub portfolio_ids: Vec<ObjectId>,
    pub accountable_team_id: Option<ObjectId>,
    pub contributing_team_ids: Vec<ObjectId>,
    pub primary_value_stream_id: Option<ObjectId>,
    pub primary_portfolio_id: Option<ObjectId>,
    pub business_domain_ids: Vec<ObjectId>,
    pub value_proposition: String,
    pub capability_ids: Vec<ObjectId>,
    pub application_ids: Vec<ObjectId>,
    pub revision: u64,
}

impl Product {
    /// Identifiant de quartier d'accueil à L0 (`PRD-033`).
    pub fn home_district(&self) -> Option<ObjectId> {
        self.primary_value_stream_id.or(self.primary_portfolio_id)
    }

    pub fn is_platform(&self) -> bool {
        self.kind == ProductKind::Platform
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Team {
    pub id: ObjectId,
    pub name: String,
    pub topology: TeamTopology,
    pub mission: String,
    pub interaction_modes: Vec<TeamInteractionMode>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueStream {
    pub id: ObjectId,
    pub name: String,
    pub value_proposition: String,
    pub stage_ids: Vec<ObjectId>,
    pub owner_team_id: Option<ObjectId>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueStreamStage {
    pub id: ObjectId,
    pub value_stream_id: ObjectId,
    pub name: String,
    pub order: u32,
    pub value_item: String,
    pub capability_ids: Vec<ObjectId>,
    pub product_ids: Vec<ObjectId>,
    pub process_ids: Vec<ObjectId>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    pub id: ObjectId,
    pub name: String,
    pub parent_id: Option<ObjectId>,
    pub level: u32,
    pub maturity: Option<u8>,
    pub criticality: Criticality,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Application {
    pub id: ObjectId,
    pub name: String,
    pub kind: ApplicationKind,
    pub lifecycle: LifecycleState,
    pub criticality: Criticality,
    pub operating_team_id: Option<ObjectId>,
    pub component_ids: Vec<ObjectId>,
    pub data_object_ids: Vec<ObjectId>,
    pub node_ids: Vec<ObjectId>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationComponent {
    pub id: ObjectId,
    pub application_id: ObjectId,
    pub name: String,
    pub kind: ComponentKind,
    pub interface_ids: Vec<ObjectId>,
    pub data_object_ids: Vec<ObjectId>,
    pub revision: u64,
}

/// Interface native de la carte.
///
/// Le type historique `Interface` du MVP3 reste le binding de métamodèle ; ce
/// type-ci porte le niveau de navigation, l'exposition et le fournisseur
/// polymorphe exigés par C-10.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapInterface {
    pub id: ObjectId,
    pub name: String,
    pub level: InterfaceLevel,
    pub category: InterfaceCategory,
    pub kind: InterfaceKind,
    pub provider_object_id: ObjectId,
    pub provider_product_id: Option<ObjectId>,
    pub exposure: InterfaceExposure,
    pub criticality: Criticality,
    pub data_classification: Option<DataClassification>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapInterfaceVersion {
    pub id: ObjectId,
    pub interface_id: ObjectId,
    pub semantic_version: String,
    pub publication_status: PublicationState,
    pub available_from: Option<String>,
    pub available_until: Option<String>,
    pub breaking_change: bool,
    pub contract_type: Option<ContractType>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceConsumption {
    pub id: ObjectId,
    pub interface_version_id: ObjectId,
    pub consumer_object_id: ObjectId,
    pub criticality: Criticality,
    pub since: Option<String>,
    pub until: Option<String>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnologyNode {
    pub id: ObjectId,
    pub name: String,
    pub kind: TechnologyNodeKind,
    pub environment: Option<Environment>,
    pub provider_team_id: Option<ObjectId>,
    pub parent_node_id: Option<ObjectId>,
    pub application_ids: Vec<ObjectId>,
    pub lifecycle: LifecycleState,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusinessDomain {
    pub id: ObjectId,
    pub name: String,
    pub description: String,
    pub parent_id: Option<ObjectId>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portfolio {
    pub id: ObjectId,
    pub name: String,
    pub kind: PortfolioKind,
    pub product_ids: Vec<ObjectId>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Initiative {
    pub id: ObjectId,
    pub name: String,
    pub kind: InitiativeKind,
    pub start: String,
    pub end: Option<String>,
    pub product_ids: Vec<ObjectId>,
    pub application_ids: Vec<ObjectId>,
    pub interface_ids: Vec<ObjectId>,
    pub revision: u64,
}

/// Ancre persistée par objet et par niveau (`MAP-020`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapAnchor {
    pub object_id: ObjectId,
    pub level: MapLevel,
    pub district_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub pinned: bool,
    pub updated_at: String,
    pub revision: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MapDataset {
    #[serde(default)]
    pub products: Vec<Product>,
    #[serde(default)]
    pub teams: Vec<Team>,
    #[serde(default)]
    pub value_streams: Vec<ValueStream>,
    #[serde(default)]
    pub stages: Vec<ValueStreamStage>,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub applications: Vec<Application>,
    #[serde(default)]
    pub components: Vec<ApplicationComponent>,
    #[serde(default)]
    pub interfaces: Vec<MapInterface>,
    #[serde(default)]
    pub versions: Vec<MapInterfaceVersion>,
    #[serde(default)]
    pub consumptions: Vec<InterfaceConsumption>,
    #[serde(default)]
    pub nodes: Vec<TechnologyNode>,
    #[serde(default)]
    pub domains: Vec<BusinessDomain>,
    #[serde(default)]
    pub portfolios: Vec<Portfolio>,
    #[serde(default)]
    pub initiatives: Vec<Initiative>,
    #[serde(default)]
    pub anchors: Vec<MapAnchor>,
    #[serde(default)]
    pub revision: u64,
}

impl MapDataset {
    pub fn is_empty(&self) -> bool {
        self.products.is_empty()
            && self.teams.is_empty()
            && self.value_streams.is_empty()
            && self.stages.is_empty()
            && self.capabilities.is_empty()
            && self.applications.is_empty()
            && self.components.is_empty()
            && self.interfaces.is_empty()
            && self.versions.is_empty()
            && self.consumptions.is_empty()
            && self.nodes.is_empty()
            && self.domains.is_empty()
            && self.portfolios.is_empty()
            && self.initiatives.is_empty()
    }

    pub fn product(&self, id: ObjectId) -> Option<&Product> {
        self.products.iter().find(|p| p.id == id)
    }

    pub fn team(&self, id: ObjectId) -> Option<&Team> {
        self.teams.iter().find(|t| t.id == id)
    }

    pub fn value_stream(&self, id: ObjectId) -> Option<&ValueStream> {
        self.value_streams.iter().find(|v| v.id == id)
    }

    pub fn stage(&self, id: ObjectId) -> Option<&ValueStreamStage> {
        self.stages.iter().find(|s| s.id == id)
    }

    pub fn capability(&self, id: ObjectId) -> Option<&Capability> {
        self.capabilities.iter().find(|c| c.id == id)
    }

    pub fn application(&self, id: ObjectId) -> Option<&Application> {
        self.applications.iter().find(|a| a.id == id)
    }

    pub fn component(&self, id: ObjectId) -> Option<&ApplicationComponent> {
        self.components.iter().find(|c| c.id == id)
    }

    pub fn interface(&self, id: ObjectId) -> Option<&MapInterface> {
        self.interfaces.iter().find(|i| i.id == id)
    }

    pub fn interface_version(&self, id: ObjectId) -> Option<&MapInterfaceVersion> {
        self.versions.iter().find(|v| v.id == id)
    }

    pub fn node(&self, id: ObjectId) -> Option<&TechnologyNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn domain(&self, id: ObjectId) -> Option<&BusinessDomain> {
        self.domains.iter().find(|d| d.id == id)
    }

    pub fn portfolio(&self, id: ObjectId) -> Option<&Portfolio> {
        self.portfolios.iter().find(|p| p.id == id)
    }

    pub fn versions_of(&self, interface_id: ObjectId) -> Vec<&MapInterfaceVersion> {
        let mut versions: Vec<&MapInterfaceVersion> = self
            .versions
            .iter()
            .filter(|v| v.interface_id == interface_id)
            .collect();
        versions
            .sort_by(|a, b| semver_key(&b.semantic_version).cmp(&semver_key(&a.semantic_version)));
        versions
    }

    pub fn latest_version(&self, interface_id: ObjectId) -> Option<&MapInterfaceVersion> {
        self.versions_of(interface_id).into_iter().next()
    }

    pub fn consumptions_of_interface(&self, interface_id: ObjectId) -> Vec<&InterfaceConsumption> {
        let version_ids: BTreeSet<ObjectId> = self
            .versions
            .iter()
            .filter(|v| v.interface_id == interface_id)
            .map(|v| v.id)
            .collect();
        self.consumptions
            .iter()
            .filter(|c| version_ids.contains(&c.interface_version_id))
            .collect()
    }

    pub fn applications_of_product(&self, product_id: ObjectId) -> Vec<&Application> {
        let Some(product) = self.product(product_id) else {
            return vec![];
        };
        self.applications
            .iter()
            .filter(|app| product.application_ids.contains(&app.id))
            .collect()
    }

    pub fn products_using_application(&self, application_id: ObjectId) -> Vec<&Product> {
        self.products
            .iter()
            .filter(|product| product.application_ids.contains(&application_id))
            .collect()
    }

    pub fn products_of_team(&self, team_id: ObjectId) -> Vec<&Product> {
        self.products
            .iter()
            .filter(|product| product.accountable_team_id == Some(team_id))
            .collect()
    }

    pub fn applications_of_team(&self, team_id: ObjectId) -> Vec<&Application> {
        self.applications
            .iter()
            .filter(|app| app.operating_team_id == Some(team_id))
            .collect()
    }

    pub fn stages_of(&self, value_stream_id: ObjectId) -> Vec<&ValueStreamStage> {
        let mut stages: Vec<&ValueStreamStage> = self
            .stages
            .iter()
            .filter(|stage| stage.value_stream_id == value_stream_id)
            .collect();
        stages.sort_by_key(|stage| stage.order);
        stages
    }

    pub fn interface_provider_label(&self, interface: &MapInterface) -> String {
        self.object_label(interface.provider_object_id)
            .map(|(_, label)| label)
            .unwrap_or_else(|| "Fournisseur inconnu".into())
    }

    /// Nom lisible et type natif d'un objet adressable.
    pub fn object_label(&self, id: ObjectId) -> Option<(MapObjectKind, String)> {
        if let Some(product) = self.product(id) {
            return Some((MapObjectKind::Product, product.name.clone()));
        }
        if let Some(application) = self.application(id) {
            return Some((MapObjectKind::Application, application.name.clone()));
        }
        if let Some(component) = self.component(id) {
            return Some((MapObjectKind::Component, component.name.clone()));
        }
        if let Some(node) = self.node(id) {
            return Some((MapObjectKind::Node, node.name.clone()));
        }
        if let Some(capability) = self.capability(id) {
            return Some((MapObjectKind::Capability, capability.name.clone()));
        }
        if let Some(team) = self.team(id) {
            return Some((MapObjectKind::Team, team.name.clone()));
        }
        if let Some(stage) = self.stage(id) {
            return Some((MapObjectKind::Stage, stage.name.clone()));
        }
        if let Some(interface) = self.interface(id) {
            return Some((MapObjectKind::Interface, interface.name.clone()));
        }
        None
    }

    pub fn product_owning_object(&self, id: ObjectId) -> Option<&Product> {
        if let Some(product) = self.product(id) {
            return Some(product);
        }
        if let Some(application) = self.application(id) {
            return self
                .products
                .iter()
                .find(|product| product.application_ids.contains(&application.id));
        }
        if let Some(component) = self.component(id) {
            let application = self.application(component.application_id)?;
            return self
                .products
                .iter()
                .find(|product| product.application_ids.contains(&application.id));
        }
        None
    }

    pub fn application_of_object(&self, id: ObjectId) -> Option<&Application> {
        if let Some(application) = self.application(id) {
            return Some(application);
        }
        self.component(id)
            .and_then(|component| self.application(component.application_id))
    }

    pub fn node_of_object(&self, id: ObjectId) -> Option<&TechnologyNode> {
        if let Some(node) = self.node(id) {
            return Some(node);
        }
        self.application(id).and_then(|application| {
            application
                .node_ids
                .first()
                .and_then(|node_id| self.node(*node_id))
        })
    }

    pub fn add_revision(&mut self, revision: u64) {
        self.revision = revision;
    }
}

// ---------------------------------------------------------------------------
// Règles et anomalies
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapAnomaly {
    pub code: String,
    pub message: String,
    pub blocking: bool,
    pub object_id: Option<ObjectId>,
}

impl MapAnomaly {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        blocking: bool,
        object_id: Option<ObjectId>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            blocking,
            object_id,
        }
    }

    /// Un signal de complétude décrit un produit encore jeune, pas une
    /// violation de règle. Il ne doit pas être présenté comme une anomalie.
    pub fn is_completeness(&self) -> bool {
        matches!(
            self.code.as_str(),
            "PRD-002" | "PRD-003" | "PRD-032" | "PRD-033"
        )
    }
}

/// Toutes les anomalies de modèle, triées par code puis objet pour rester
/// déterministes (`MAP-012`).
pub fn dataset_anomalies(data: &MapDataset) -> Vec<MapAnomaly> {
    let mut out = Vec::new();

    for product in &data.products {
        if let Some(issue) = product_issue(data, product) {
            out.push(issue);
        }
        if product.kind == ProductKind::Platform {
            let exposed = data.interfaces.iter().any(|interface| {
                interface.provider_product_id == Some(product.id)
                    && interface.exposure != InterfaceExposure::Internal
            });
            if !exposed {
                out.push(MapAnomaly::new(
                    "PRD-007",
                    format!(
                        "Le produit plateforme « {} » n'expose aucune interface aux autres produits.",
                        product.name
                    ),
                    false,
                    Some(product.id),
                ));
            }
        }
        if product.application_ids.is_empty() {
            out.push(MapAnomaly::new(
                "PRD-032",
                format!(
                    "Le produit « {} » n'utilise aucune application.",
                    product.name
                ),
                false,
                Some(product.id),
            ));
        }
        if product.capability_ids.is_empty() {
            out.push(MapAnomaly::new(
                "PRD-003",
                format!(
                    "Le produit « {} » ne réalise aucune capacité.",
                    product.name
                ),
                false,
                Some(product.id),
            ));
        }
        if !product.is_platform() {
            let contributes = data.stages.iter().any(|stage| {
                stage.product_ids.contains(&product.id)
                    || product
                        .primary_value_stream_id
                        .is_some_and(|stream| stream == stage.value_stream_id)
            });
            if !contributes {
                out.push(MapAnomaly::new(
                    "PRD-002",
                    format!(
                        "Le produit « {} » ne contribue à aucune étape de value stream.",
                        product.name
                    ),
                    false,
                    Some(product.id),
                ));
            }
        }
        if product.home_district().is_none() && !product.is_platform() {
            out.push(MapAnomaly::new(
                "PRD-033",
                format!(
                    "Le produit « {} » n'a pas de value stream principal : il est placé dans « Non rattaché ».",
                    product.name
                ),
                false,
                Some(product.id),
            ));
        }
    }

    for application in &data.applications {
        if application.operating_team_id.is_none() {
            out.push(MapAnomaly::new(
                "PRD-009",
                format!(
                    "L'application « {} » n'a pas d'équipe opératrice.",
                    application.name
                ),
                true,
                Some(application.id),
            ));
        }
        if application.node_ids.is_empty() {
            out.push(MapAnomaly::new(
                "PRD-022",
                format!(
                    "L'application « {} » n'est déployée sur aucun nœud.",
                    application.name
                ),
                false,
                Some(application.id),
            ));
        }
        if data.products_using_application(application.id).len() > 1 {
            // Signal de couplage assumé (`PRD-006`), jamais une anomalie bloquante.
            out.push(MapAnomaly::new(
                "PRD-006",
                format!(
                    "L'application « {} » est partagée par plusieurs produits ; c'est un couplage à arbitrer.",
                    application.name
                ),
                false,
                Some(application.id),
            ));
        }
    }

    for interface in &data.interfaces {
        if data.interface(interface.provider_object_id).is_some()
            || data.object_label(interface.provider_object_id).is_none()
        {
            out.push(MapAnomaly::new(
                "PRD-024",
                format!(
                    "L'interface « {} » n'a pas de fournisseur résolu.",
                    interface.name
                ),
                true,
                Some(interface.id),
            ));
        }
        if data.versions_of(interface.id).is_empty() {
            out.push(MapAnomaly::new(
                "PRD-025",
                format!(
                    "L'interface « {} » n'a aucune version : elle ne peut pas être publiée.",
                    interface.name
                ),
                true,
                Some(interface.id),
            ));
        }
        if interface.level == InterfaceLevel::Infrastructure
            && matches!(
                interface.exposure,
                InterfaceExposure::Partner | InterfaceExposure::Public
            )
        {
            out.push(MapAnomaly::new(
                "PRD-029",
                format!(
                    "L'interface d'infrastructure « {} » ne peut pas être exposée au partenaire ou au public.",
                    interface.name
                ),
                true,
                Some(interface.id),
            ));
        }
    }

    for consumption in &data.consumptions {
        if data
            .interface_version(consumption.interface_version_id)
            .is_none()
        {
            out.push(MapAnomaly::new(
                "PRD-026",
                "Une consommation référence une version d'interface inexistante.",
                true,
                None,
            ));
        }
    }

    for team in &data.teams {
        if team.topology == TeamTopology::Enabling && !data.applications_of_team(team.id).is_empty()
        {
            out.push(MapAnomaly::new(
                "PRD-011",
                format!(
                    "L'équipe enabling « {} » opère une application en propre.",
                    team.name
                ),
                false,
                Some(team.id),
            ));
        }
    }

    for capability in &data.capabilities {
        let realized = data
            .products
            .iter()
            .any(|product| product.capability_ids.contains(&capability.id));
        if !realized {
            out.push(MapAnomaly::new(
                "PRD-018",
                format!(
                    "La capacité « {} » n'est réalisée par aucun produit.",
                    capability.name
                ),
                false,
                Some(capability.id),
            ));
        }
    }

    out.sort_by(|a, b| {
        a.code
            .cmp(&b.code)
            .then_with(|| a.object_id.cmp(&b.object_id))
            .then_with(|| a.message.cmp(&b.message))
    });
    out
}

/// Anomalie principale d'un produit, ou `None` s'il est publiable.
pub fn product_issue(data: &MapDataset, product: &Product) -> Option<MapAnomaly> {
    match product.accountable_team_id {
        Some(team_id) if data.team(team_id).is_some() => None,
        _ => Some(MapAnomaly::new(
            "PRD-001",
            format!(
                "Le produit « {} » n'a pas exactement une équipe accountable.",
                product.name
            ),
            true,
            Some(product.id),
        )),
    }
}

pub fn validate_product(data: &MapDataset, product: &Product) -> Result<(), MapAnomaly> {
    if let Some(issue) = product_issue(data, product) {
        return Err(issue);
    }
    if let Some(parent) = product.parent_product_id {
        if data.product(parent).is_none() {
            return Err(MapAnomaly::new(
                "PRD-004",
                "Le produit parent n'existe pas dans le référentiel.",
                true,
                Some(product.id),
            ));
        }
        let mut depth = 1;
        let mut cursor = data.product(parent);
        while let Some(current) = cursor {
            depth += 1;
            if depth > 3 {
                return Err(MapAnomaly::new(
                    "PRD-004",
                    "La hiérarchie de produits dépasse trois niveaux.",
                    true,
                    Some(product.id),
                ));
            }
            cursor = current.parent_product_id.and_then(|id| data.product(id));
        }
    }
    Ok(())
}

pub fn validate_application(
    data: &MapDataset,
    application: &Application,
) -> Result<(), MapAnomaly> {
    match application.operating_team_id {
        Some(team_id) if data.team(team_id).is_some() => Ok(()),
        _ => Err(MapAnomaly::new(
            "PRD-009",
            format!(
                "L'application « {} » doit avoir exactement une équipe opératrice.",
                application.name
            ),
            true,
            Some(application.id),
        )),
    }
}

/// Une interface a exactement un fournisseur (`PRD-024`) et au moins une
/// version pour être publiée (`PRD-025`).
pub fn validate_interface(
    data: &MapDataset,
    interface: &MapInterface,
    publish: bool,
) -> Result<(), MapAnomaly> {
    if data.object_label(interface.provider_object_id).is_none() {
        return Err(MapAnomaly::new(
            "PRD-024",
            format!(
                "L'interface « {} » n'a pas de fournisseur résolu.",
                interface.name
            ),
            true,
            Some(interface.id),
        ));
    }
    if interface.level == InterfaceLevel::Infrastructure
        && matches!(
            interface.exposure,
            InterfaceExposure::Partner | InterfaceExposure::Public
        )
    {
        return Err(MapAnomaly::new(
            "PRD-029",
            "Une interface d'infrastructure ne peut pas être exposée au partenaire ou au public.",
            true,
            Some(interface.id),
        ));
    }
    if publish && data.versions_of(interface.id).is_empty() {
        return Err(MapAnomaly::new(
            "PRD-025",
            format!(
                "L'interface « {} » ne peut pas être publiée sans version.",
                interface.name
            ),
            true,
            Some(interface.id),
        ));
    }
    Ok(())
}

/// `PRD-026` : une consommation référence toujours une version précise.
pub fn validate_consumption(
    data: &MapDataset,
    version_id: ObjectId,
    consumer_id: ObjectId,
) -> Result<(), MapAnomaly> {
    let Some(version) = data.interface_version(version_id) else {
        return Err(MapAnomaly::new(
            "PRD-026",
            "La consommation doit référencer une version d'interface existante, jamais l'interface nue.",
            true,
            None,
        ));
    };
    if data.interface(version.interface_id).is_none() {
        return Err(MapAnomaly::new(
            "PRD-026",
            "La version consommée n'appartient à aucune interface.",
            true,
            None,
        ));
    }
    if data.object_label(consumer_id).is_none() {
        return Err(MapAnomaly::new(
            "PRD-026",
            "Le consommateur de l'interface n'existe pas dans le référentiel.",
            true,
            None,
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Projection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreadcrumbEntry {
    pub label: String,
    pub level: MapLevel,
    pub scope: MapScope,
    pub route: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapDistrict {
    pub id: String,
    pub kind: DistrictKind,
    pub object_id: Option<ObjectId>,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub color_index: u8,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapNode {
    pub id: String,
    pub object_id: ObjectId,
    pub object_kind: MapObjectKind,
    pub label: String,
    pub sublabel: Option<String>,
    pub status: Option<String>,
    pub criticality: Option<Criticality>,
    pub parent_node_id: Option<String>,
    pub district_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub aggregate_count: Option<usize>,
    pub expandable: bool,
    pub color_key: String,
    pub route: String,
    pub badges: Vec<String>,
    pub focused: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapEdge {
    pub id: String,
    pub kind: MapEdgeKind,
    pub source_node_id: String,
    pub target_node_id: String,
    pub label: Option<String>,
    pub interface_ids: Vec<ObjectId>,
    pub interface_version_ids: Vec<ObjectId>,
    pub count: usize,
    pub criticality: Option<Criticality>,
    pub temporal_status: Option<String>,
    pub aggregated: bool,
    pub expandable: bool,
    pub strategic: bool,
}

/// Port affiché sur la frontière d'un produit (`PRD-028`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapPort {
    pub interface_id: ObjectId,
    pub name: String,
    pub level: InterfaceLevel,
    pub exposure: InterfaceExposure,
    pub latest_version: Option<String>,
    pub consumer_count: usize,
    pub route: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegendEntry {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapLegend {
    pub lens: MapLens,
    pub title: String,
    pub entries: Vec<LegendEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MapCounts {
    pub districts: usize,
    pub nodes: usize,
    pub edges: usize,
    pub aggregated_edges: usize,
    pub interfaces: usize,
    pub anomalies: usize,
    pub hidden: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapProjection {
    pub query: MapQuery,
    pub breadcrumb: Vec<BreadcrumbEntry>,
    pub districts: Vec<MapDistrict>,
    pub nodes: Vec<MapNode>,
    pub edges: Vec<MapEdge>,
    pub ports: Vec<MapPort>,
    pub legend: MapLegend,
    pub counts: MapCounts,
    pub anomalies: Vec<MapAnomaly>,
    pub source_revision: u64,
}

impl MapProjection {
    pub fn node(&self, id: &str) -> Option<&MapNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn objects_in_scope(&self) -> Vec<ObjectId> {
        self.nodes.iter().map(|node| node.object_id).collect()
    }
}

const NODE_W: f64 = 158.0;
const NODE_H: f64 = 64.0;
const NODE_GAP: f64 = 18.0;
const DISTRICT_PAD: f64 = 22.0;
const DISTRICT_HEADER: f64 = 34.0;
const DISTRICT_COLS: usize = 2;
const NODE_COLS: usize = 3;

struct Layout {
    nodes: Vec<MapNode>,
    districts: Vec<MapDistrict>,
}

fn push_node(
    layout: &mut Layout,
    district: &mut MapDistrict,
    data: &MapDataset,
    query: &MapQuery,
    object_id: ObjectId,
    kind: MapObjectKind,
    label: String,
    sublabel: Option<String>,
    status: Option<String>,
    criticality: Option<Criticality>,
    index: usize,
) {
    let col = index % NODE_COLS;
    let row = index / NODE_COLS;
    let x = district.x + DISTRICT_PAD + col as f64 * (NODE_W + NODE_GAP);
    let y = district.y + DISTRICT_HEADER + row as f64 * (NODE_H + NODE_GAP);
    let mut node = MapNode {
        id: format!("{}:{}", kind.as_str().to_ascii_lowercase(), object_id),
        object_id,
        object_kind: kind,
        label,
        sublabel,
        status,
        criticality,
        parent_node_id: None,
        district_id: Some(district.id.clone()),
        x,
        y,
        w: NODE_W,
        h: NODE_H,
        aggregate_count: None,
        expandable: true,
        color_key: color_key(data, query, kind, object_id),
        route: object_route(kind, object_id),
        badges: badges(data, query, kind, object_id),
        focused: query.focus.contains(&object_id),
    };
    if node.sublabel.is_none() {
        node.sublabel = Some(kind_label(kind).into());
    }
    district.node_ids.push(node.id.clone());
    layout.nodes.push(node);
}

fn district_height(node_count: usize) -> f64 {
    let rows = if node_count == 0 {
        1
    } else {
        node_count.div_ceil(NODE_COLS)
    };
    DISTRICT_HEADER + rows as f64 * (NODE_H + NODE_GAP) - NODE_GAP + DISTRICT_PAD
}

fn add_district(
    layout: &mut Layout,
    index: usize,
    kind: DistrictKind,
    object_id: Option<ObjectId>,
    label: String,
) -> usize {
    let col = index % DISTRICT_COLS;
    let row = index / DISTRICT_COLS;
    let x = 40.0 + col as f64 * (NODE_COLS as f64 * (NODE_W + NODE_GAP) + DISTRICT_PAD);
    let y = 40.0 + row as f64 * 300.0;
    // La hauteur réelle est ajustée après le placement des nœuds.
    let district = MapDistrict {
        id: format!("district-{index}"),
        kind,
        object_id,
        label,
        x,
        y,
        w: NODE_COLS as f64 * (NODE_W + NODE_GAP) + DISTRICT_PAD * 2.0 - NODE_GAP,
        h: district_height(0),
        color_index: (index % 6) as u8,
        node_ids: vec![],
    };
    layout.districts.push(district);
    layout.districts.len() - 1
}

fn finish_district(layout: &mut Layout, index: usize) {
    let count = layout.districts[index].node_ids.len();
    layout.districts[index].h = district_height(count);
}

fn color_key(
    data: &MapDataset,
    query: &MapQuery,
    kind: MapObjectKind,
    object_id: ObjectId,
) -> String {
    let lens = query.primary_lens();
    match lens {
        MapLens::Product | MapLens::Value | MapLens::Time => {
            if let Some(product) = data.product(object_id) {
                return lifecycle_key(product.lifecycle).into();
            }
            if let Some(application) = data.application(object_id) {
                return lifecycle_key(application.lifecycle).into();
            }
            if let Some(node) = data.node(object_id) {
                return lifecycle_key(node.lifecycle).into();
            }
            "neutral".into()
        }
        MapLens::Organization => {
            let team = data
                .product(object_id)
                .and_then(|p| p.accountable_team_id)
                .or_else(|| {
                    data.application(object_id)
                        .and_then(|a| a.operating_team_id)
                })
                .or_else(|| data.node(object_id).and_then(|n| n.provider_team_id));
            match team {
                Some(id) => format!("team-{}", hash_index(id.as_str(), 6)),
                None => "neutral".into(),
            }
        }
        MapLens::Interface => {
            let count = data
                .interfaces
                .iter()
                .filter(|interface| {
                    interface.provider_object_id == object_id
                        || data
                            .product_owning_object(interface.provider_object_id)
                            .is_some_and(|p| p.id == object_id)
                })
                .count();
            match count {
                0 => "none".into(),
                1..=2 => "few".into(),
                3..=5 => "several".into(),
                _ => "many".into(),
            }
        }
        MapLens::Capability => {
            let count = data
                .product(object_id)
                .map(|p| p.capability_ids.len())
                .unwrap_or(0);
            match count {
                0 => "none".into(),
                1..=2 => "few".into(),
                3..=5 => "several".into(),
                _ => "many".into(),
            }
        }
        MapLens::Application => match data.application(object_id).map(|a| a.kind) {
            Some(ApplicationKind::Legacy) => "legacy".into(),
            Some(ApplicationKind::Saas | ApplicationKind::Cots) => "package".into(),
            Some(ApplicationKind::External) => "external".into(),
            Some(_) => "internal".into(),
            None => "neutral".into(),
        },
        MapLens::Technology => match kind {
            MapObjectKind::Node => data
                .node(object_id)
                .map(|node| match node.kind {
                    TechnologyNodeKind::Cluster => "cluster",
                    TechnologyNodeKind::Network => "network",
                    TechnologyNodeKind::Database => "database",
                    TechnologyNodeKind::Platform => "platform",
                    _ => "server",
                })
                .unwrap_or("neutral")
                .into(),
            _ => "neutral".into(),
        },
        MapLens::Risk => {
            let criticality = data
                .product(object_id)
                .map(|p| p.criticality)
                .or_else(|| data.application(object_id).map(|a| a.criticality))
                .or_else(|| data.capability(object_id).map(|c| c.criticality));
            criticality
                .map(|value| value.as_str().to_ascii_lowercase())
                .unwrap_or_else(|| "none".into())
        }
        MapLens::Data => {
            let classification = data
                .interfaces
                .iter()
                .find(|interface| interface.provider_object_id == object_id)
                .and_then(|interface| interface.data_classification);
            classification
                .map(|value| value.as_str().to_ascii_lowercase())
                .unwrap_or_else(|| "none".into())
        }
    }
}

fn lifecycle_key(lifecycle: LifecycleState) -> &'static str {
    match lifecycle {
        LifecycleState::Run => "run",
        LifecycleState::Build | LifecycleState::Discovery | LifecycleState::Idea => "build",
        LifecycleState::Sunset | LifecycleState::Retired => "sunset",
    }
}

fn hash_index(value: &str, modulo: u8) -> u8 {
    let mut hash: u32 = 2_166_136_261;
    for byte in value.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }
    (hash % modulo as u32) as u8
}

fn kind_label(kind: MapObjectKind) -> &'static str {
    match kind {
        MapObjectKind::Product => "Produit",
        MapObjectKind::Application => "Application",
        MapObjectKind::Component => "Composant",
        MapObjectKind::Node => "Nœud",
        MapObjectKind::Capability => "Capacité",
        MapObjectKind::Team => "Équipe",
        MapObjectKind::Stage => "Étape",
        MapObjectKind::Interface => "Interface",
    }
}

fn object_route(kind: MapObjectKind, id: ObjectId) -> String {
    match kind {
        MapObjectKind::Product => format!("#/products/{id}"),
        MapObjectKind::Application => format!("#/applications/{id}"),
        MapObjectKind::Component => format!("#/applications/{id}"),
        MapObjectKind::Node => format!("#/infrastructure/{id}"),
        MapObjectKind::Capability => format!("#/capabilities/{id}"),
        MapObjectKind::Team => format!("#/teams/{id}"),
        MapObjectKind::Stage => format!("#/map"),
        MapObjectKind::Interface => format!("#/interfaces/{id}"),
    }
}

fn badges(data: &MapDataset, query: &MapQuery, kind: MapObjectKind, id: ObjectId) -> Vec<String> {
    if query.primary_lens() == MapLens::Interface {
        let count = data
            .interfaces
            .iter()
            .filter(|interface| {
                interface.provider_object_id == id
                    || data
                        .product_owning_object(interface.provider_object_id)
                        .is_some_and(|product| product.id == id)
            })
            .count();
        return if count == 0 {
            vec!["aucune interface".into()]
        } else {
            vec![format!("{count} interface(s)")]
        };
    }
    match kind {
        MapObjectKind::Product => {
            let mut badges = Vec::new();
            if let Some(product) = data.product(id) {
                if !product.application_ids.is_empty() {
                    badges.push(format!("{} app.", product.application_ids.len()));
                }
                let interfaces = data
                    .interfaces
                    .iter()
                    .filter(|interface| {
                        interface.provider_product_id == Some(id)
                            || interface.provider_object_id == id
                    })
                    .count();
                if interfaces > 0 {
                    badges.push(format!("{interfaces} if."));
                }
                let consumers = data.products_using_application(id).len().saturating_sub(1);
                if consumers > 0 {
                    badges.push(format!("partagé ×{}", consumers + 1));
                }
            }
            badges
        }
        MapObjectKind::Application => {
            let mut count = 0;
            if let Some(app) = data.application(id) {
                count = data.products_using_application(app.id).len();
            }
            if count > 1 {
                vec![format!("{count} produits")]
            } else {
                vec![]
            }
        }
        MapObjectKind::Node => data
            .node(id)
            .map(|node| {
                if node.application_ids.is_empty() {
                    vec![]
                } else {
                    vec![format!("{} app.", node.application_ids.len())]
                }
            })
            .unwrap_or_default(),
        _ => vec![],
    }
}

fn object_status(data: &MapDataset, kind: MapObjectKind, id: ObjectId) -> Option<String> {
    let lifecycle = match kind {
        MapObjectKind::Product => data.product(id).map(|p| p.lifecycle),
        MapObjectKind::Application => data.application(id).map(|a| a.lifecycle),
        MapObjectKind::Node => data.node(id).map(|n| n.lifecycle),
        _ => None,
    };
    lifecycle.map(|value| value.as_str().to_string())
}

fn scope_label(data: &MapDataset, scope: MapScope) -> String {
    match (scope.kind, scope.id) {
        (MapScopeKind::Enterprise, _) => "Entreprise".into(),
        (MapScopeKind::ValueStream, Some(id)) => data
            .value_stream(id)
            .map(|v| v.name.clone())
            .unwrap_or_else(|| "Value stream".into()),
        (MapScopeKind::Portfolio, Some(id)) => data
            .portfolio(id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Portefeuille".into()),
        (MapScopeKind::Product, Some(id)) => data
            .product(id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Produit".into()),
        (MapScopeKind::Application, Some(id)) => data
            .application(id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Application".into()),
        (MapScopeKind::Node, Some(id)) => data
            .node(id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "Infrastructure".into()),
        _ => "Entreprise".into(),
    }
}

fn breadcrumb(data: &MapDataset, query: &MapQuery) -> Vec<BreadcrumbEntry> {
    let mut entries = vec![BreadcrumbEntry {
        label: "Entreprise".into(),
        level: MapLevel::Enterprise,
        scope: MapScope::enterprise(),
        route: "#/".into(),
    }];

    if query.scope.kind == MapScopeKind::Enterprise {
        return entries;
    }

    // Un produit se situe d'abord dans son value stream principal : le fil
    // d'Ariane raconte le parcours, il n'empile pas des niveaux techniques.
    if query.scope.kind == MapScopeKind::Product {
        if let Some(stream) = query
            .scope
            .id
            .and_then(|id| data.product(id))
            .and_then(|product| product.primary_value_stream_id)
            .and_then(|id| data.value_stream(id))
        {
            entries.push(BreadcrumbEntry {
                label: stream.name.clone(),
                level: MapLevel::Portfolio,
                scope: MapScope::value_stream(stream.id),
                route: format!(
                    "#/map?{}",
                    MapQuery::default()
                        .at_scope(MapScope::value_stream(stream.id))
                        .level(MapLevel::Portfolio)
                        .to_query_string()
                ),
            });
        }
    }

    entries.push(BreadcrumbEntry {
        label: scope_label(data, query.scope),
        level: MapLevel::closest_for(query.scope.kind),
        scope: query.scope,
        route: format!(
            "#/map?{}",
            MapQuery::default()
                .at_scope(query.scope)
                .level(MapLevel::closest_for(query.scope.kind))
                .to_query_string()
        ),
    });

    entries
}

fn legend(lens: MapLens) -> MapLegend {
    let entries = match lens {
        MapLens::Product | MapLens::Value | MapLens::Time => vec![
            ("run", "En run"),
            ("build", "En construction"),
            ("sunset", "En retrait"),
        ],
        MapLens::Organization => vec![
            ("team-0", "Équipe A"),
            ("team-1", "Équipe B"),
            ("team-2", "Équipe C"),
            ("neutral", "Sans équipe"),
        ],
        MapLens::Interface => vec![
            ("none", "Aucune interface"),
            ("few", "1–2 interfaces"),
            ("several", "3–5 interfaces"),
            ("many", "6 interfaces et plus"),
        ],
        MapLens::Capability => vec![
            ("none", "Aucune capacité"),
            ("few", "1–2 capacités"),
            ("several", "3–5 capacités"),
            ("many", "6 capacités et plus"),
        ],
        MapLens::Application => vec![
            ("internal", "Application interne"),
            ("package", "COTS / SaaS"),
            ("legacy", "Héritage"),
            ("external", "Externe"),
        ],
        MapLens::Technology => vec![
            ("server", "Serveur"),
            ("cluster", "Cluster"),
            ("database", "Base de données"),
            ("network", "Réseau"),
            ("platform", "Plateforme"),
        ],
        MapLens::Risk => vec![
            ("critical", "Critique"),
            ("high", "Élevé"),
            ("medium", "Moyen"),
            ("low", "Faible"),
        ],
        MapLens::Data => vec![
            ("public", "Public"),
            ("internal", "Interne"),
            ("confidential", "Confidentiel"),
            ("secret", "Secret"),
        ],
    };
    MapLegend {
        lens,
        title: format!("Lentille {}", lens.label()),
        entries: entries
            .into_iter()
            .map(|(key, label)| LegendEntry {
                key: key.into(),
                label: label.into(),
            })
            .collect(),
    }
}

/// Projette la carte. Déterministe : mêmes données + même requête ⇒ même
/// projection (`MAP-012`).
pub fn project(data: &MapDataset, query: &MapQuery) -> MapProjection {
    let (query, mut anomalies) = query.normalized(data);
    let mut layout = Layout {
        nodes: vec![],
        districts: vec![],
    };

    match query.level {
        MapLevel::Enterprise => project_enterprise(data, &query, &mut layout),
        MapLevel::Portfolio => project_portfolio(data, &query, &mut layout),
        MapLevel::Product => project_product(data, &query, &mut layout),
        MapLevel::Application => project_application(data, &query, &mut layout),
        MapLevel::Infrastructure => project_infrastructure(data, &query, &mut layout),
    }

    // Une ancre l'emporte toujours sur l'heuristique, sans changer le quartier.
    let anchors: HashMap<(ObjectId, MapLevel), &MapAnchor> = data
        .anchors
        .iter()
        .map(|anchor| ((anchor.object_id, anchor.level), anchor))
        .collect();
    for node in &mut layout.nodes {
        if let Some(anchor) = anchors.get(&(node.object_id, query.level)) {
            node.x = anchor.x;
            node.y = anchor.y;
            if anchor.district_id.is_some() {
                node.district_id = anchor.district_id.clone();
            }
        }
    }

    let visible: BTreeSet<String> = layout.nodes.iter().map(|node| node.id.clone()).collect();
    let edges = build_edges(data, &query, &visible);
    let ports = if query.level == MapLevel::Product {
        query
            .scope
            .id
            .map(|id| product_ports(data, id))
            .unwrap_or_default()
    } else {
        vec![]
    };

    let dataset_anomalies = dataset_anomalies(data);
    anomalies.extend(dataset_anomalies.iter().cloned());
    anomalies.sort_by(|a, b| {
        a.code
            .cmp(&b.code)
            .then_with(|| a.object_id.cmp(&b.object_id))
    });
    anomalies.dedup_by(|a, b| a.code == b.code && a.object_id == b.object_id);
    let anomaly_count = anomalies.len();
    let hidden = data
        .products
        .len()
        .saturating_sub(
            layout
                .nodes
                .iter()
                .filter(|n| n.object_kind == MapObjectKind::Product)
                .count(),
        )
        .max(0);

    let counts = MapCounts {
        districts: layout.districts.len(),
        nodes: layout.nodes.len(),
        edges: edges.len(),
        aggregated_edges: edges.iter().filter(|edge| edge.aggregated).count(),
        interfaces: data.interfaces.len(),
        anomalies: anomaly_count,
        hidden,
    };

    MapProjection {
        breadcrumb: breadcrumb(data, &query),
        districts: layout.districts,
        nodes: layout.nodes,
        edges,
        ports,
        legend: legend(query.primary_lens()),
        counts,
        anomalies,
        source_revision: data.revision,
        query,
    }
}

fn visible_products<'a>(data: &'a MapDataset, filter: &MapFilter) -> Vec<&'a Product> {
    let mut products: Vec<&Product> = data
        .products
        .iter()
        .filter(|product| filter.accepts_product(product))
        .collect();
    products.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    products
}

fn push_product_node(
    data: &MapDataset,
    query: &MapQuery,
    layout: &mut Layout,
    district_index: usize,
    product: &Product,
    index: usize,
) {
    let mut district = layout.districts[district_index].clone();
    push_node(
        layout,
        &mut district,
        data,
        query,
        product.id,
        MapObjectKind::Product,
        product.name.clone(),
        data.team(product.accountable_team_id.unwrap_or(product.id))
            .map(|team| team.name.clone())
            .or_else(|| Some("Sans équipe accountable".into())),
        object_status(data, MapObjectKind::Product, product.id),
        Some(product.criticality),
        index,
    );
    layout.districts[district_index] = district;
}

fn project_enterprise(data: &MapDataset, query: &MapQuery, layout: &mut Layout) {
    let products = visible_products(data, &query.filters);
    let platforms: Vec<&Product> = products
        .iter()
        .copied()
        .filter(|product| product.is_platform())
        .collect();
    let value_streams: Vec<&ValueStream> = {
        let mut list: Vec<&ValueStream> = data.value_streams.iter().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        list
    };

    let mut district_index = 0;
    let mut placed: BTreeSet<ObjectId> = BTreeSet::new();
    for value_stream in &value_streams {
        let members: Vec<&Product> = products
            .iter()
            .copied()
            .filter(|product| {
                !product.is_platform() && product.primary_value_stream_id == Some(value_stream.id)
            })
            .collect();
        if members.is_empty() {
            continue;
        }
        let index = add_district(
            layout,
            district_index,
            DistrictKind::ValueStream,
            Some(value_stream.id),
            value_stream.name.clone(),
        );
        for (i, product) in members.iter().enumerate() {
            push_product_node(data, query, layout, index, product, i);
            placed.insert(product.id);
        }
        finish_district(layout, index);
        district_index += 1;
    }

    if !platforms.is_empty() {
        let index = add_district(
            layout,
            district_index,
            DistrictKind::PlatformGroup,
            None,
            "Plateformes partagées".into(),
        );
        for (i, product) in platforms.iter().enumerate() {
            push_product_node(data, query, layout, index, product, i);
            placed.insert(product.id);
        }
        finish_district(layout, index);
        district_index += 1;
    }

    let unassigned: Vec<&Product> = products
        .iter()
        .copied()
        .filter(|product| !placed.contains(&product.id))
        .collect();
    if !unassigned.is_empty() {
        let index = add_district(
            layout,
            district_index,
            DistrictKind::Unassigned,
            None,
            "Non rattaché".into(),
        );
        for (i, product) in unassigned.iter().enumerate() {
            push_product_node(data, query, layout, index, product, i);
        }
        finish_district(layout, index);
    }
}

fn project_portfolio(data: &MapDataset, query: &MapQuery, layout: &mut Layout) {
    let products = visible_products(data, &query.filters);
    let (scope_label, members): (String, Vec<&Product>) = match query.scope.kind {
        MapScopeKind::ValueStream => {
            let id = query.scope.id;
            (
                scope_label(data, query.scope),
                products
                    .iter()
                    .copied()
                    .filter(|product| {
                        !product.is_platform() && product.primary_value_stream_id == id
                    })
                    .collect(),
            )
        }
        MapScopeKind::Portfolio => {
            let id = query.scope.id;
            (
                scope_label(data, query.scope),
                products
                    .iter()
                    .copied()
                    .filter(|product| match id {
                        Some(id) => {
                            product.primary_portfolio_id == Some(id)
                                || product.portfolio_ids.contains(&id)
                        }
                        None => false,
                    })
                    .collect(),
            )
        }
        _ => (
            "Entreprise".into(),
            products
                .iter()
                .copied()
                .filter(|product| !product.is_platform())
                .collect(),
        ),
    };

    let index = add_district(
        layout,
        0,
        DistrictKind::ValueStream,
        query.scope.id,
        scope_label.clone(),
    );
    for (i, product) in members.iter().enumerate() {
        push_product_node(data, query, layout, index, product, i);
    }
    finish_district(layout, index);

    let platforms: Vec<&Product> = products
        .iter()
        .copied()
        .filter(|product| product.is_platform())
        .collect();
    if !platforms.is_empty() {
        let index = add_district(
            layout,
            1,
            DistrictKind::PlatformGroup,
            None,
            "Plateformes partagées".into(),
        );
        for (i, product) in platforms.iter().enumerate() {
            push_product_node(data, query, layout, index, product, i);
        }
        finish_district(layout, index);
    }
}

fn project_product(data: &MapDataset, query: &MapQuery, layout: &mut Layout) {
    let Some(product_id) = query.scope.id else {
        return;
    };
    let Some(product) = data.product(product_id) else {
        return;
    };

    let index = add_district(
        layout,
        0,
        DistrictKind::ProductGroup,
        Some(product.id),
        product.name.clone(),
    );
    let own_applications: Vec<ObjectId> = product.application_ids.clone();
    let mut applications: Vec<&Application> = data
        .applications
        .iter()
        .filter(|application| own_applications.contains(&application.id))
        .collect();
    applications.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

    // Dépendances externes : une application hors produit reste visible,
    // atténuée, pour montrer les interfaces entrantes et sortantes (MAP-007).
    let mut external: Vec<&Application> = Vec::new();
    for consumption in &data.consumptions {
        let Some(version) = data.interface_version(consumption.interface_version_id) else {
            continue;
        };
        let Some(interface) = data.interface(version.interface_id) else {
            continue;
        };
        let provider = data.application_of_object(interface.provider_object_id);
        let consumer = data.application_of_object(consumption.consumer_object_id);
        match (provider, consumer) {
            (Some(provider), Some(consumer)) => {
                let provider_inside = own_applications.contains(&provider.id);
                let consumer_inside = own_applications.contains(&consumer.id);
                if provider_inside && !consumer_inside && !external.contains(&consumer) {
                    external.push(consumer);
                }
                if consumer_inside && !provider_inside && !external.contains(&provider) {
                    external.push(provider);
                }
            }
            _ => {}
        }
    }
    external.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

    if applications.is_empty() {
        // État vide explicite : la frontière produit existe même sans application.
        let mut district = layout.districts[index].clone();
        push_node(
            layout,
            &mut district,
            data,
            query,
            product.id,
            MapObjectKind::Product,
            "Aucune application".into(),
            Some("Ajouter une application au produit".into()),
            Some("EMPTY".into()),
            Some(product.criticality),
            0,
        );
        layout.districts[index] = district;
        layout.nodes.last_mut().map(|node| {
            node.expandable = false;
            node.color_key = "none".into();
        });
    } else {
        for (i, application) in applications.iter().enumerate() {
            let mut district = layout.districts[index].clone();
            push_node(
                layout,
                &mut district,
                data,
                query,
                application.id,
                MapObjectKind::Application,
                application.name.clone(),
                data.team(application.operating_team_id.unwrap_or(application.id))
                    .map(|team| team.name.clone()),
                object_status(data, MapObjectKind::Application, application.id),
                Some(application.criticality),
                i,
            );
            layout.districts[index] = district;
        }
    }
    finish_district(layout, index);

    if !external.is_empty() && query.filters.include_external {
        let index = add_district(
            layout,
            1,
            DistrictKind::ProductGroup,
            None,
            "Dépendances externes".into(),
        );
        for (i, application) in external.iter().enumerate() {
            let mut district = layout.districts[index].clone();
            push_node(
                layout,
                &mut district,
                data,
                query,
                application.id,
                MapObjectKind::Application,
                application.name.clone(),
                data.team(application.operating_team_id.unwrap_or(application.id))
                    .map(|team| team.name.clone()),
                object_status(data, MapObjectKind::Application, application.id),
                Some(application.criticality),
                i,
            );
            layout.districts[index] = district;
        }
        finish_district(layout, index);
    }
}

fn project_application(data: &MapDataset, query: &MapQuery, layout: &mut Layout) {
    let Some(application_id) = query.scope.id else {
        return;
    };
    let Some(application) = data.application(application_id) else {
        return;
    };
    let index = add_district(
        layout,
        0,
        DistrictKind::ProductGroup,
        Some(application.id),
        application.name.clone(),
    );
    let mut components: Vec<&ApplicationComponent> = data
        .components
        .iter()
        .filter(|component| component.application_id == application.id)
        .collect();
    components.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    if components.is_empty() {
        // Application feuille (COTS/SaaS) : la frontière reste visible même
        // sans composant, sinon la page L3 serait un écran blanc.
        let mut district = layout.districts[index].clone();
        push_node(
            layout,
            &mut district,
            data,
            query,
            application.id,
            MapObjectKind::Application,
            application.name.clone(),
            Some("Application feuille · aucun composant".into()),
            object_status(data, MapObjectKind::Application, application.id),
            Some(application.criticality),
            0,
        );
        layout.districts[index] = district;
        if let Some(node) = layout.nodes.last_mut() {
            node.color_key = "package".into();
            node.expandable = false;
        }
        finish_district(layout, index);
        return;
    }
    for (i, component) in components.iter().enumerate() {
        let mut district = layout.districts[index].clone();
        push_node(
            layout,
            &mut district,
            data,
            query,
            component.id,
            MapObjectKind::Component,
            component.name.clone(),
            Some(component.kind.label().into()),
            None,
            None,
            i,
        );
        layout.districts[index] = district;
    }
    finish_district(layout, index);
}

fn project_infrastructure(data: &MapDataset, query: &MapQuery, layout: &mut Layout) {
    let scoped = query.scope.id.and_then(|id| data.node(id));
    let mut nodes: Vec<&TechnologyNode> = data
        .nodes
        .iter()
        .filter(|node| match scoped {
            Some(scoped) => {
                node.id == scoped.id
                    || (node.environment.is_some() && node.environment == scoped.environment)
            }
            None => true,
        })
        .collect();
    nodes.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    let mut environments: Vec<(String, String, Vec<&TechnologyNode>)> = Vec::new();
    for node in nodes {
        let (key, label) = match node.environment {
            Some(environment) => (
                environment.as_str().to_string(),
                environment.label().to_string(),
            ),
            None => ("NONE".into(), "Sans environnement".into()),
        };
        if let Some((_, _, members)) = environments.iter_mut().find(|(k, _, _)| *k == key) {
            members.push(node);
        } else {
            environments.push((key, label, vec![node]));
        }
    }
    for (i, (_, label, members)) in environments.iter().enumerate() {
        let index = add_district(
            layout,
            i,
            DistrictKind::Environment,
            None,
            format!("Environnement {label}"),
        );
        for (j, node) in members.iter().enumerate() {
            let mut district = layout.districts[index].clone();
            push_node(
                layout,
                &mut district,
                data,
                query,
                node.id,
                MapObjectKind::Node,
                node.name.clone(),
                Some(node.kind.label().into()),
                object_status(data, MapObjectKind::Node, node.id),
                None,
                j,
            );
            layout.districts[index] = district;
        }
        finish_district(layout, index);
    }
}

fn product_ports(data: &MapDataset, product_id: ObjectId) -> Vec<MapPort> {
    let Some(product) = data.product(product_id) else {
        return vec![];
    };
    let mut interfaces: Vec<&MapInterface> = data
        .interfaces
        .iter()
        .filter(|interface| {
            interface.provider_product_id == Some(product_id)
                || interface.provider_object_id == product_id
                || (product
                    .application_ids
                    .contains(&interface.provider_object_id)
                    && interface.exposure == InterfaceExposure::Product)
        })
        .collect();
    interfaces.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    interfaces
        .into_iter()
        .map(|interface| MapPort {
            interface_id: interface.id,
            name: interface.name.clone(),
            level: interface.level,
            exposure: interface.exposure,
            latest_version: data
                .latest_version(interface.id)
                .map(|version| version.semantic_version.clone()),
            consumer_count: data.consumptions_of_interface(interface.id).len(),
            route: format!("#/interfaces/{}", interface.id),
        })
        .collect()
}

fn build_edges(data: &MapDataset, query: &MapQuery, visible: &BTreeSet<String>) -> Vec<MapEdge> {
    #[derive(Default)]
    struct Accumulator {
        kind: MapEdgeKind,
        interface_ids: BTreeSet<ObjectId>,
        version_ids: BTreeSet<ObjectId>,
        criticality: Option<Criticality>,
        temporal: Option<String>,
    }

    let mut accumulators: BTreeMap<(String, String), Accumulator> = BTreeMap::new();
    let date = query.date.as_deref();

    for consumption in &data.consumptions {
        let Some(version) = data.interface_version(consumption.interface_version_id) else {
            continue;
        };
        let Some(interface) = data.interface(version.interface_id) else {
            continue;
        };
        if !query.filters.accepts_interface(interface) {
            continue;
        }
        let temporal = temporal_status(version, date);
        if temporal.as_deref() == Some("EXPIRED") && !query.filters.include_expired {
            continue;
        }

        let provider_objects = provider_objects_for(data, query.level, interface);
        let consumer_objects =
            consumer_objects_for(data, query.level, consumption.consumer_object_id);
        for source_id in &provider_objects {
            let source = node_key(query.level, *source_id);
            if !visible.contains(&source) {
                continue;
            }
            for target_id in &consumer_objects {
                let target = node_key(query.level, *target_id);
                if !visible.contains(&target) || source == target {
                    continue;
                }
                let entry = accumulators
                    .entry((source.clone(), target.clone()))
                    .or_insert_with(|| Accumulator {
                        kind: edge_kind(query.level, interface.level),
                        interface_ids: BTreeSet::new(),
                        version_ids: BTreeSet::new(),
                        criticality: None,
                        temporal: None,
                    });
                entry.interface_ids.insert(interface.id);
                entry.version_ids.insert(version.id);
                entry.criticality = Some(max_criticality(
                    entry.criticality.unwrap_or(Criticality::Low),
                    max_criticality(interface.criticality, consumption.criticality),
                ));
                let candidate = temporal.clone();
                if entry.temporal.as_deref() != Some("EXPIRED") {
                    entry.temporal = match (entry.temporal.take(), candidate) {
                        (Some(current), Some(next)) => Some(most_constraining(&current, &next)),
                        (current, next) => current.or(next),
                    };
                }
            }
        }
    }

    accumulators
        .into_iter()
        .zip(0..)
        .map(|(((source, target), acc), index)| {
            let count = acc.interface_ids.len();
            MapEdge {
                id: format!("edge-{index}-{}", count),
                kind: acc.kind,
                source_node_id: source.clone(),
                target_node_id: target,
                label: if count > 1 {
                    Some(format!("{count} interfaces"))
                } else {
                    None
                },
                interface_ids: acc.interface_ids.into_iter().collect(),
                interface_version_ids: acc.version_ids.into_iter().collect(),
                count,
                criticality: acc.criticality,
                temporal_status: acc.temporal,
                aggregated: count > 1,
                expandable: true,
                strategic: false,
            }
        })
        .collect()
}

fn node_key(level: MapLevel, id: ObjectId) -> String {
    match level {
        MapLevel::Enterprise | MapLevel::Portfolio => format!("product:{id}"),
        MapLevel::Product => format!("application:{id}"),
        MapLevel::Application => format!("component:{id}"),
        MapLevel::Infrastructure => format!("node:{id}"),
    }
}

fn products_of_object(data: &MapDataset, id: ObjectId) -> Vec<ObjectId> {
    if let Some(product) = data.product(id) {
        return vec![product.id];
    }
    if let Some(application) = data.application(id) {
        return data
            .products_using_application(application.id)
            .into_iter()
            .map(|product| product.id)
            .collect();
    }
    if let Some(component) = data.component(id) {
        if let Some(application) = data.application(component.application_id) {
            return data
                .products_using_application(application.id)
                .into_iter()
                .map(|product| product.id)
                .collect();
        }
    }
    vec![]
}

fn provider_objects_for(
    data: &MapDataset,
    level: MapLevel,
    interface: &MapInterface,
) -> Vec<ObjectId> {
    match level {
        MapLevel::Enterprise | MapLevel::Portfolio => {
            if let Some(product_id) = interface.provider_product_id {
                if data.product(product_id).is_some() {
                    return vec![product_id];
                }
            }
            products_of_object(data, interface.provider_object_id)
        }
        MapLevel::Product => data
            .application_of_object(interface.provider_object_id)
            .map(|application| vec![application.id])
            .unwrap_or_default(),
        MapLevel::Application => data
            .component(interface.provider_object_id)
            .map(|component| vec![component.id])
            .unwrap_or_default(),
        MapLevel::Infrastructure => data
            .node_of_object(interface.provider_object_id)
            .map(|node| vec![node.id])
            .unwrap_or_default(),
    }
}

fn consumer_objects_for(data: &MapDataset, level: MapLevel, id: ObjectId) -> Vec<ObjectId> {
    match level {
        MapLevel::Enterprise | MapLevel::Portfolio => products_of_object(data, id),
        MapLevel::Product => data
            .application_of_object(id)
            .map(|application| vec![application.id])
            .unwrap_or_default(),
        MapLevel::Application => data
            .component(id)
            .map(|component| vec![component.id])
            .unwrap_or_default(),
        MapLevel::Infrastructure => data
            .node_of_object(id)
            .map(|node| vec![node.id])
            .unwrap_or_default(),
    }
}

fn edge_kind(level: MapLevel, interface_level: InterfaceLevel) -> MapEdgeKind {
    match level {
        MapLevel::Enterprise | MapLevel::Portfolio => MapEdgeKind::ProductInterface,
        MapLevel::Product => MapEdgeKind::ApplicationInterface,
        MapLevel::Application => MapEdgeKind::ComponentInterface,
        MapLevel::Infrastructure => {
            if interface_level == InterfaceLevel::Infrastructure {
                MapEdgeKind::TechnicalInterface
            } else {
                MapEdgeKind::Deployment
            }
        }
    }
}

fn max_criticality(a: Criticality, b: Criticality) -> Criticality {
    let rank = |value: Criticality| match value {
        Criticality::Critical => 4,
        Criticality::High => 3,
        Criticality::Medium => 2,
        Criticality::Low => 1,
    };
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

fn most_constraining(a: &str, b: &str) -> String {
    let rank = |value: &str| match value {
        "EXPIRED" => 3,
        "SCHEDULED" => 2,
        "ACTIVE" => 1,
        _ => 0,
    };
    if rank(a) >= rank(b) {
        a.into()
    } else {
        b.into()
    }
}

/// État temporel d'une version à la date demandée (`MAP-008`).
pub fn temporal_status(version: &MapInterfaceVersion, date: Option<&str>) -> Option<String> {
    let Some(date) = date else {
        return Some("ACTIVE".into());
    };
    let day = date_only(date);
    if let Some(from) = &version.available_from {
        if day < date_only(from) {
            return Some("SCHEDULED".into());
        }
    }
    if let Some(until) = &version.available_until {
        if day > date_only(until) {
            return Some("EXPIRED".into());
        }
    }
    Some("ACTIVE".into())
}

// ---------------------------------------------------------------------------
// Recherche et helpers d'interface
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapSearchHit {
    pub object_id: ObjectId,
    pub kind: MapObjectKind,
    pub label: String,
    pub detail: String,
    pub route: String,
}

pub fn search(data: &MapDataset, query: &str) -> Vec<MapSearchHit> {
    let needle = query.trim().to_lowercase();
    if needle.len() < 2 {
        return vec![];
    }
    let mut hits = Vec::new();
    for product in &data.products {
        if product.name.to_lowercase().contains(&needle) {
            hits.push(MapSearchHit {
                object_id: product.id,
                kind: MapObjectKind::Product,
                label: product.name.clone(),
                detail: format!(
                    "Produit · {} · {}",
                    product.kind.label(),
                    product.lifecycle.label()
                ),
                route: object_route(MapObjectKind::Product, product.id),
            });
        }
    }
    for team in &data.teams {
        if team.name.to_lowercase().contains(&needle) {
            hits.push(MapSearchHit {
                object_id: team.id,
                kind: MapObjectKind::Team,
                label: team.name.clone(),
                detail: format!("Équipe · {}", team.topology.label()),
                route: object_route(MapObjectKind::Team, team.id),
            });
        }
    }
    for application in &data.applications {
        if application.name.to_lowercase().contains(&needle) {
            hits.push(MapSearchHit {
                object_id: application.id,
                kind: MapObjectKind::Application,
                label: application.name.clone(),
                detail: format!(
                    "Application · {} · {}",
                    application.kind.label(),
                    application.lifecycle.label()
                ),
                route: object_route(MapObjectKind::Application, application.id),
            });
        }
    }
    for capability in &data.capabilities {
        if capability.name.to_lowercase().contains(&needle) {
            hits.push(MapSearchHit {
                object_id: capability.id,
                kind: MapObjectKind::Capability,
                label: capability.name.clone(),
                detail: format!("Capacité · {}", capability.criticality.label()),
                route: object_route(MapObjectKind::Capability, capability.id),
            });
        }
    }
    for node in &data.nodes {
        if node.name.to_lowercase().contains(&needle) {
            let environment = node
                .environment
                .map(|value| value.label().to_string())
                .unwrap_or_else(|| "Environnement non renseigné".into());
            hits.push(MapSearchHit {
                object_id: node.id,
                kind: MapObjectKind::Node,
                label: node.name.clone(),
                detail: format!("Nœud · {} · {}", node.kind.label(), environment),
                route: object_route(MapObjectKind::Node, node.id),
            });
        }
    }
    for interface in &data.interfaces {
        if interface.name.to_lowercase().contains(&needle) {
            hits.push(MapSearchHit {
                object_id: interface.id,
                kind: MapObjectKind::Interface,
                label: interface.name.clone(),
                detail: format!(
                    "Interface · {} · {} · {}",
                    interface.level.label(),
                    interface.exposure.label(),
                    interface.criticality.label()
                ),
                route: object_route(MapObjectKind::Interface, interface.id),
            });
        }
    }
    for value_stream in &data.value_streams {
        if value_stream.name.to_lowercase().contains(&needle) {
            hits.push(MapSearchHit {
                object_id: value_stream.id,
                kind: MapObjectKind::Stage,
                label: value_stream.name.clone(),
                detail: {
                    let stages = data
                        .stages
                        .iter()
                        .filter(|stage| stage.value_stream_id == value_stream.id)
                        .count();
                    format!("Value stream · {stages} étape(s)")
                },
                route: format!("#/value-streams/{}", value_stream.id),
            });
        }
    }
    hits.sort_by(|a, b| a.label.cmp(&b.label));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{product_catalog, ProductGap, ProductQuery, ProductSort};

    fn id(n: u8) -> ObjectId {
        ObjectId::from_parts(1_700_000_000_000 + n as u64, n as u128)
    }

    fn fixture() -> MapDataset {
        let team_a = Team {
            id: id(1),
            name: "Équipe Souscription".into(),
            topology: TeamTopology::StreamAligned,
            mission: "Posséder le parcours de souscription.".into(),
            interaction_modes: vec![],
            revision: 1,
        };
        let team_b = Team {
            id: id(2),
            name: "Équipe Paiements".into(),
            topology: TeamTopology::Platform,
            mission: "Fournir le hub de paiement.".into(),
            interaction_modes: vec![],
            revision: 1,
        };
        let value_stream = ValueStream {
            id: id(3),
            name: "Souscrire un contrat".into(),
            value_proposition: "Un contrat souscrit en moins de dix minutes.".into(),
            stage_ids: vec![id(4), id(5)],
            owner_team_id: Some(team_a.id),
            revision: 1,
        };
        let stages = vec![
            ValueStreamStage {
                id: id(4),
                value_stream_id: value_stream.id,
                name: "Choisir une offre".into(),
                order: 1,
                value_item: "Une offre adaptée".into(),
                capability_ids: vec![id(9)],
                product_ids: vec![id(6)],
                process_ids: vec![],
                revision: 1,
            },
            ValueStreamStage {
                id: id(5),
                value_stream_id: value_stream.id,
                name: "Payer".into(),
                order: 2,
                value_item: "Un paiement encaissé".into(),
                capability_ids: vec![id(10)],
                product_ids: vec![id(6)],
                process_ids: vec![],
                revision: 1,
            },
        ];
        let product = Product {
            id: id(6),
            name: "Parcours Souscription".into(),
            kind: ProductKind::Customer,
            lifecycle: LifecycleState::Run,
            criticality: Criticality::Critical,
            parent_product_id: None,
            portfolio_ids: vec![],
            accountable_team_id: Some(team_a.id),
            contributing_team_ids: vec![team_b.id],
            primary_value_stream_id: Some(value_stream.id),
            primary_portfolio_id: None,
            business_domain_ids: vec![id(11)],
            value_proposition: "Souscrire sans friction.".into(),
            capability_ids: vec![id(9), id(10)],
            application_ids: vec![id(7), id(8)],
            revision: 1,
        };
        let platform = Product {
            id: id(12),
            name: "Hub Paiements".into(),
            kind: ProductKind::Platform,
            lifecycle: LifecycleState::Run,
            criticality: Criticality::High,
            parent_product_id: None,
            portfolio_ids: vec![],
            accountable_team_id: Some(team_b.id),
            contributing_team_ids: vec![],
            primary_value_stream_id: None,
            primary_portfolio_id: None,
            business_domain_ids: vec![],
            value_proposition: "Encaisser pour tous les produits.".into(),
            capability_ids: vec![id(10)],
            application_ids: vec![id(8)],
            revision: 1,
        };
        let applications = vec![
            Application {
                id: id(7),
                name: "Portail Souscription".into(),
                kind: ApplicationKind::Internal,
                lifecycle: LifecycleState::Run,
                criticality: Criticality::High,
                operating_team_id: Some(team_a.id),
                component_ids: vec![id(13)],
                data_object_ids: vec![],
                node_ids: vec![id(16)],
                revision: 1,
            },
            Application {
                id: id(8),
                name: "Payment Hub".into(),
                kind: ApplicationKind::Platform,
                lifecycle: LifecycleState::Run,
                criticality: Criticality::Critical,
                operating_team_id: Some(team_b.id),
                component_ids: vec![],
                data_object_ids: vec![],
                node_ids: vec![id(15)],
                revision: 1,
            },
        ];
        let components = vec![ApplicationComponent {
            id: id(13),
            application_id: id(7),
            name: "Formulaire".into(),
            kind: ComponentKind::Ui,
            interface_ids: vec![id(14)],
            data_object_ids: vec![],
            revision: 1,
        }];
        let interfaces = vec![
            MapInterface {
                id: id(14),
                name: "Payment API".into(),
                level: InterfaceLevel::Application,
                category: InterfaceCategory::Application,
                kind: InterfaceKind::Api,
                provider_object_id: id(8),
                provider_product_id: Some(platform.id),
                exposure: InterfaceExposure::Product,
                criticality: Criticality::Critical,
                data_classification: Some(DataClassification::Confidential),
                revision: 1,
            },
            MapInterface {
                id: id(17),
                name: "Portail API".into(),
                level: InterfaceLevel::Application,
                category: InterfaceCategory::Application,
                kind: InterfaceKind::Api,
                provider_object_id: id(7),
                provider_product_id: Some(product.id),
                exposure: InterfaceExposure::Product,
                criticality: Criticality::High,
                data_classification: Some(DataClassification::Internal),
                revision: 1,
            },
        ];
        let versions = vec![
            MapInterfaceVersion {
                id: id(18),
                interface_id: id(14),
                semantic_version: "2.0.0".into(),
                publication_status: PublicationState::Published,
                available_from: Some("2026-01-01".into()),
                available_until: None,
                breaking_change: true,
                contract_type: Some(ContractType::OpenApi),
                revision: 1,
            },
            MapInterfaceVersion {
                id: id(19),
                interface_id: id(17),
                semantic_version: "1.0.0".into(),
                publication_status: PublicationState::Published,
                available_from: Some("2025-06-01".into()),
                available_until: Some("2027-01-01".into()),
                breaking_change: false,
                contract_type: None,
                revision: 1,
            },
        ];
        let consumptions = vec![InterfaceConsumption {
            id: id(20),
            interface_version_id: id(18),
            consumer_object_id: id(7),
            criticality: Criticality::Critical,
            since: None,
            until: None,
            revision: 1,
        }];
        let nodes = vec![
            TechnologyNode {
                id: id(15),
                name: "Kubernetes Prod".into(),
                kind: TechnologyNodeKind::Cluster,
                environment: Some(Environment::Prod),
                provider_team_id: Some(team_b.id),
                parent_node_id: None,
                application_ids: vec![id(8)],
                lifecycle: LifecycleState::Run,
                revision: 1,
            },
            TechnologyNode {
                id: id(16),
                name: "Cluster Souscription".into(),
                kind: TechnologyNodeKind::Cluster,
                environment: Some(Environment::Prod),
                provider_team_id: Some(team_a.id),
                parent_node_id: None,
                application_ids: vec![id(7)],
                lifecycle: LifecycleState::Run,
                revision: 1,
            },
        ];
        let capabilities = vec![
            Capability {
                id: id(9),
                name: "Tarification".into(),
                parent_id: None,
                level: 1,
                maturity: Some(4),
                criticality: Criticality::High,
                revision: 1,
            },
            Capability {
                id: id(10),
                name: "Encaissement".into(),
                parent_id: None,
                level: 1,
                maturity: Some(3),
                criticality: Criticality::Critical,
                revision: 1,
            },
        ];
        MapDataset {
            products: vec![product, platform],
            teams: vec![team_a, team_b],
            value_streams: vec![value_stream],
            stages,
            capabilities,
            applications,
            components,
            interfaces,
            versions,
            consumptions,
            nodes,
            domains: vec![BusinessDomain {
                id: id(11),
                name: "Assurance".into(),
                description: "Classification de connaissance.".into(),
                parent_id: None,
                revision: 1,
            }],
            portfolios: vec![],
            initiatives: vec![],
            anchors: vec![],
            revision: 7,
        }
    }

    #[test]
    fn product_requires_accountable_team() {
        let data = fixture();
        let mut product = data.products[0].clone();
        product.accountable_team_id = None;
        assert_eq!(
            product_issue(&data, &product).map(|issue| issue.code),
            Some("PRD-001".into())
        );
        assert!(validate_product(&data, &product).is_err());
        assert!(validate_product(&data, &data.products[0]).is_ok());
    }

    #[test]
    fn application_is_used_by_products_not_owned() {
        let data = fixture();
        let app = &data.applications[1];
        let users = data.products_using_application(app.id);
        assert_eq!(users.len(), 2, "le hub paiement sert deux produits");
        // Aucun champ de possession n'existe sur l'application : la relation
        // n'est lisible que depuis les produits utilisateurs.
        let shared = dataset_anomalies(&data)
            .into_iter()
            .any(|issue| issue.code == "PRD-006" && issue.object_id == Some(app.id));
        assert!(shared, "l'application partagée est signalée comme couplage");
    }

    #[test]
    fn domain_is_classification_not_owner() {
        let data = fixture();
        let product = &data.products[0];
        assert_eq!(product.business_domain_ids, vec![id(11)]);
        // Une équipe reste la seule responsable : le domaine n'a aucune méthode
        // de possession et ne peut pas satisfaire `PRD-001`.
        let mut orphan = product.clone();
        orphan.accountable_team_id = None;
        orphan.business_domain_ids = vec![id(11)];
        assert_eq!(
            product_issue(&data, &orphan).map(|issue| issue.code),
            Some("PRD-001".into())
        );
    }

    #[test]
    fn interface_consumption_requires_version() {
        let data = fixture();
        assert!(validate_consumption(&data, id(99), id(7)).is_err());
        assert!(validate_consumption(&data, id(18), id(7)).is_ok());
    }

    #[test]
    fn map_opens_enterprise() {
        let data = fixture();
        let projection = project(&data, &MapQuery::enterprise());
        assert_eq!(projection.query.level, MapLevel::Enterprise);
        assert_eq!(projection.query.scope.kind, MapScopeKind::Enterprise);
        assert!(projection
            .nodes
            .iter()
            .all(|node| node.object_kind == MapObjectKind::Product));
        assert!(projection
            .nodes
            .iter()
            .any(|node| node.label == "Parcours Souscription"));
    }

    #[test]
    fn map_opens_on_products() {
        let projection = project(&fixture(), &MapQuery::map_default());
        assert_eq!(projection.query.scope.kind, MapScopeKind::Enterprise);
        assert_eq!(projection.query.level, MapLevel::Enterprise);
        assert_eq!(projection.query.primary_lens(), MapLens::Product);
        assert!(!projection.nodes.is_empty());
    }

    #[test]
    fn map_default_lens_is_product() {
        assert_eq!(MapLens::default_for(MapLevel::Enterprise), MapLens::Product);
        assert_eq!(MapLens::default_for(MapLevel::Portfolio), MapLens::Product);
        assert_eq!(MapQuery::map_default().primary_lens(), MapLens::Product);
    }

    #[test]
    fn map_products_are_the_only_nodes_at_l0() {
        let projection = project(&fixture(), &MapQuery::map_default());
        assert!(projection
            .nodes
            .iter()
            .all(|node| node.object_kind == MapObjectKind::Product));
    }

    // -----------------------------------------------------------------------
    // Catalogue produits (ADR-025)
    // -----------------------------------------------------------------------

    #[test]
    fn catalog_lists_all_products() {
        let catalog = product_catalog(&fixture(), &ProductQuery::home());
        assert_eq!(catalog.total, 2);
        assert_eq!(catalog.filtered, 2);
    }

    #[test]
    fn catalog_filters_by_team() {
        let mut query = ProductQuery::home();
        query.team_ids = vec![id(2)];
        let catalog = product_catalog(&fixture(), &query);
        assert_eq!(catalog.filtered, 1);
        assert_eq!(catalog.items[0].name, "Hub Paiements");
    }

    #[test]
    fn catalog_filters_by_value_stream() {
        let mut query = ProductQuery::home();
        query.value_stream_ids = vec![id(3)];
        let catalog = product_catalog(&fixture(), &query);
        assert_eq!(catalog.filtered, 1);
        assert_eq!(catalog.items[0].name, "Parcours Souscription");
    }

    #[test]
    fn catalog_filters_by_capability() {
        let mut query = ProductQuery::home();
        query.capability_ids = vec![id(10)];
        let catalog = product_catalog(&fixture(), &query);
        assert_eq!(catalog.filtered, 2);
    }

    #[test]
    fn catalog_filters_by_application() {
        let mut query = ProductQuery::home();
        query.application_ids = vec![id(8)];
        let catalog = product_catalog(&fixture(), &query);
        assert_eq!(catalog.filtered, 2);
    }

    #[test]
    fn catalog_filter_or_within_facet() {
        let mut query = ProductQuery::home();
        query.kinds = vec![ProductKind::Customer, ProductKind::Platform];
        let catalog = product_catalog(&fixture(), &query);
        assert_eq!(catalog.filtered, 2);
    }

    #[test]
    fn catalog_filter_and_across_facets() {
        let mut query = ProductQuery::home();
        query.team_ids = vec![id(2)];
        query.kinds = vec![ProductKind::Platform];
        let catalog = product_catalog(&fixture(), &query);
        assert_eq!(catalog.filtered, 1);
        assert_eq!(catalog.items[0].name, "Hub Paiements");
    }

    #[test]
    fn catalog_gap_without_value_stream() {
        let mut query = ProductQuery::home();
        query.gaps = vec![ProductGap::NoValueStream];
        let catalog = product_catalog(&fixture(), &query);
        assert_eq!(catalog.filtered, 1);
        assert_eq!(catalog.items[0].name, "Hub Paiements");
    }

    #[test]
    fn catalog_card_counts_relations() {
        let catalog = product_catalog(&fixture(), &ProductQuery::home());
        let product = catalog
            .items
            .iter()
            .find(|item| item.name == "Parcours Souscription")
            .unwrap();
        assert_eq!(product.capability_count, 2);
        assert_eq!(product.application_count, 2);
        assert_eq!(product.exposed_interface_count, 2);
        assert_eq!(product.consumed_interface_count, 1);
    }

    #[test]
    fn catalog_sorts_by_criticality() {
        let mut query = ProductQuery::home();
        query.sort = ProductSort::Criticality;
        let catalog = product_catalog(&fixture(), &query);
        assert_eq!(catalog.items[0].name, "Parcours Souscription");
    }

    #[test]
    fn catalog_search_matches_related_objects() {
        let mut query = ProductQuery::home();
        query.search = "Portail API".into();
        let catalog = product_catalog(&fixture(), &query);
        assert_eq!(catalog.filtered, 1);
        assert_eq!(catalog.items[0].name, "Parcours Souscription");
    }

    #[test]
    fn map_does_not_show_components_at_enterprise_level() {
        let data = fixture();
        let projection = project(&data, &MapQuery::enterprise());
        assert!(!projection
            .nodes
            .iter()
            .any(|node| node.object_kind == MapObjectKind::Component));
    }

    #[test]
    fn map_keeps_geometry_across_lenses() {
        let data = fixture();
        let base = MapQuery::enterprise();
        let a = project(&data, &base.clone().with_lens(MapLens::Product));
        let b = project(&data, &base.with_lens(MapLens::Organization));
        assert_eq!(a.nodes.len(), b.nodes.len());
        for (left, right) in a.nodes.iter().zip(b.nodes.iter()) {
            assert_eq!(left.id, right.id);
            assert_eq!(
                (left.x, left.y, left.w, left.h),
                (right.x, right.y, right.w, right.h)
            );
        }
    }

    #[test]
    fn map_zooms_enterprise_to_infrastructure() {
        let data = fixture();
        let enterprise = project(&data, &MapQuery::enterprise());
        assert_eq!(enterprise.query.level, MapLevel::Enterprise);

        let product = project(
            &data,
            &MapQuery::default()
                .at_scope(MapScope::product(id(6)))
                .level(MapLevel::Product),
        );
        assert!(product
            .nodes
            .iter()
            .any(|node| node.object_kind == MapObjectKind::Application));

        let application = project(
            &data,
            &MapQuery::default()
                .at_scope(MapScope::application(id(7)))
                .level(MapLevel::Application),
        );
        assert!(application
            .nodes
            .iter()
            .any(|node| node.object_kind == MapObjectKind::Component));

        let infrastructure = project(
            &data,
            &MapQuery::default()
                .at_scope(MapScope::node(id(15)))
                .level(MapLevel::Infrastructure),
        );
        assert!(infrastructure
            .nodes
            .iter()
            .any(|node| node.object_kind == MapObjectKind::Node));
    }

    #[test]
    fn map_aggregate_edge_expands_to_interfaces() {
        let data = fixture();
        let projection = project(&data, &MapQuery::enterprise());
        let edge = projection.edges.first().expect("une route attendue");
        assert!(edge.expandable);
        assert_eq!(edge.interface_ids, vec![id(14)]);
        assert_eq!(edge.count, 1);
        assert!(!edge.interface_version_ids.is_empty());
    }

    #[test]
    fn map_temporal_cursor_does_not_mutate_layout() {
        let data = fixture();
        let base = MapQuery::enterprise();
        let today = project(&data, &base.clone().with_date("2026-11-19"));
        let past = project(&data, &base.with_date("2024-01-01"));
        for (left, right) in today.nodes.iter().zip(past.nodes.iter()) {
            assert_eq!((left.x, left.y), (right.x, right.y));
        }
        assert_eq!(
            past.edges[0].temporal_status.as_deref(),
            Some("SCHEDULED"),
            "en 2024 la version n'est pas encore disponible"
        );
        assert_eq!(
            today.edges[0].temporal_status.as_deref(),
            Some("ACTIVE"),
            "à la date courante la version est active"
        );
    }

    #[test]
    fn map_deep_link_restores_context() {
        let query = MapQuery::from_params(
            &format!("PRODUCT:{}", id(6)),
            "PRODUCT",
            "INTERFACE,TIME",
            Some("2026-11-19"),
            &id(6).to_string(),
            "read",
        );
        let projection = project(&fixture(), &query);
        assert_eq!(projection.query.scope.kind, MapScopeKind::Product);
        assert_eq!(projection.query.level, MapLevel::Product);
        assert_eq!(
            projection.query.lenses,
            vec![MapLens::Interface, MapLens::Time]
        );
        let restored = MapQuery::from_params(
            &projection.query.scope.to_param(),
            projection.query.level.as_str(),
            "INTERFACE,TIME",
            projection.query.date.as_deref(),
            &id(6).to_string(),
            "read",
        );
        assert_eq!(restored, projection.query);
    }

    #[test]
    fn map_rejects_unavailable_lens_without_moving_objects() {
        let data = fixture();
        let query = MapQuery::enterprise().with_lens(MapLens::Technology);
        let projection = project(&data, &query);
        assert_eq!(projection.query.lenses, vec![MapLens::Product]);
        assert!(projection
            .anomalies
            .iter()
            .any(|issue| issue.code == "MAP-004"));
    }

    #[test]
    fn map_unknown_scope_falls_back_to_enterprise() {
        let data = fixture();
        let query = MapQuery::default().at_scope(MapScope::product(id(99)));
        let projection = project(&data, &query);
        assert_eq!(projection.query.scope.kind, MapScopeKind::Enterprise);
        assert!(projection
            .anomalies
            .iter()
            .any(|issue| issue.code == "MAP-001"));
    }

    #[test]
    fn projection_is_deterministic() {
        let data = fixture();
        let query = MapQuery::enterprise();
        let a = project(&data, &query);
        let b = project(&data, &query);
        assert_eq!(a, b);
    }

    #[test]
    fn completeness_signals_are_not_anomalies() {
        assert!(MapAnomaly::new("PRD-033", "", false, None).is_completeness());
        assert!(MapAnomaly::new("PRD-003", "", false, None).is_completeness());
        assert!(!MapAnomaly::new("PRD-006", "", false, None).is_completeness());
        assert!(!MapAnomaly::new("MAP-002", "", false, None).is_completeness());
    }

    #[test]
    fn value_stream_scope_rejects_product_level() {
        assert!(!MapLevel::Product.compatible_with(MapScopeKind::ValueStream));
        assert_eq!(
            MapLevel::closest_for(MapScopeKind::ValueStream),
            MapLevel::Portfolio
        );
    }

    #[test]
    fn value_stream_product_level_is_corrected_to_portfolio() {
        let data = fixture();
        let query = MapQuery::default()
            .at_scope(MapScope::value_stream(id(3)))
            .level(MapLevel::Product);
        let projection = project(&data, &query);
        assert_eq!(projection.query.level, MapLevel::Portfolio);
        assert!(projection
            .anomalies
            .iter()
            .any(|issue| issue.code == "MAP-002"));
        assert!(!projection.nodes.is_empty());
    }

    #[test]
    fn anchors_override_heuristic_without_changing_lens() {
        let mut data = fixture();
        data.anchors.push(MapAnchor {
            object_id: id(6),
            level: MapLevel::Enterprise,
            district_id: None,
            x: 11.0,
            y: 22.0,
            pinned: true,
            updated_at: "2026-11-19T00:00:00Z".into(),
            revision: 1,
        });
        let projection = project(&data, &MapQuery::enterprise());
        let node = projection
            .nodes
            .iter()
            .find(|node| node.object_id == id(6))
            .expect("produit visible");
        assert_eq!((node.x, node.y), (11.0, 22.0));
    }
}
