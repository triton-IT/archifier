//! Catalogue produits — accueil `/` (ADR-025).
//!
//! Le catalogue est une **lecture du même référentiel** que la carte : il ne
//! duplique aucun objet et ne crée aucune relation. Il répond à « quels
//! produits avons-nous, et lesquels correspondent à ces critères ? ».
//!
//! Les filtres portent à la fois sur les attributs du produit et sur ses
//! relations (équipe, value stream, capacité, application, interface). Ils se
//! combinent en **OU à l'intérieur d'une facette** et en **ET entre facettes**.

use crate::ids::ObjectId;
use crate::map::{
    dataset_anomalies, Criticality, InterfaceExposure, InterfaceLevel, LifecycleState, MapDataset,
    MapInterface, Product, ProductKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProductSort {
    Name,
    Criticality,
    Lifecycle,
    Team,
}

impl Default for ProductSort {
    fn default() -> Self {
        Self::Name
    }
}

impl ProductSort {
    pub const ALL: &'static [Self] = &[Self::Name, Self::Criticality, Self::Lifecycle, Self::Team];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Name => "NAME",
            Self::Criticality => "CRITICALITY",
            Self::Lifecycle => "LIFECYCLE",
            Self::Team => "TEAM",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Nom",
            Self::Criticality => "Criticité",
            Self::Lifecycle => "Cycle de vie",
            Self::Team => "Équipe",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "" | "NAME" | "NOM" => Some(Self::Name),
            "CRITICALITY" | "CRITICITE" | "CRITICITÉ" => Some(Self::Criticality),
            "LIFECYCLE" | "CYCLE" => Some(Self::Lifecycle),
            "TEAM" | "EQUIPE" | "ÉQUIPE" => Some(Self::Team),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProductGap {
    NoTeam,
    NoValueStream,
    NoCapability,
    NoApplication,
}

impl ProductGap {
    pub const ALL: &'static [Self] = &[
        Self::NoTeam,
        Self::NoValueStream,
        Self::NoCapability,
        Self::NoApplication,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoTeam => "NO_TEAM",
            Self::NoValueStream => "NO_VALUE_STREAM",
            Self::NoCapability => "NO_CAPABILITY",
            Self::NoApplication => "NO_APPLICATION",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NoTeam => "Sans équipe accountable",
            Self::NoValueStream => "Sans value stream",
            Self::NoCapability => "Sans capacité",
            Self::NoApplication => "Sans application",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "NO_TEAM" | "SANS_EQUIPE" | "SANS_ÉQUIPE" => Some(Self::NoTeam),
            "NO_VALUE_STREAM" | "SANS_VALUE_STREAM" => Some(Self::NoValueStream),
            "NO_CAPABILITY" | "SANS_CAPACITE" | "SANS_CAPACITÉ" => Some(Self::NoCapability),
            "NO_APPLICATION" | "SANS_APPLICATION" => Some(Self::NoApplication),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductQuery {
    pub search: String,
    pub team_ids: Vec<ObjectId>,
    pub kinds: Vec<ProductKind>,
    pub lifecycles: Vec<LifecycleState>,
    pub criticalities: Vec<Criticality>,
    pub value_stream_ids: Vec<ObjectId>,
    pub capability_ids: Vec<ObjectId>,
    pub application_ids: Vec<ObjectId>,
    pub interface_levels: Vec<InterfaceLevel>,
    pub exposures: Vec<InterfaceExposure>,
    pub gaps: Vec<ProductGap>,
    pub sort: ProductSort,
}

impl Default for ProductQuery {
    fn default() -> Self {
        Self {
            search: String::new(),
            team_ids: Vec::new(),
            kinds: Vec::new(),
            lifecycles: Vec::new(),
            criticalities: Vec::new(),
            value_stream_ids: Vec::new(),
            capability_ids: Vec::new(),
            application_ids: Vec::new(),
            interface_levels: Vec::new(),
            exposures: Vec::new(),
            gaps: Vec::new(),
            sort: ProductSort::Name,
        }
    }
}

impl ProductQuery {
    /// Requête d'accueil : tous les produits, sans filtre.
    pub fn home() -> Self {
        Self::default()
    }

    pub fn is_default(&self) -> bool {
        self.search.trim().is_empty()
            && self.team_ids.is_empty()
            && self.kinds.is_empty()
            && self.lifecycles.is_empty()
            && self.criticalities.is_empty()
            && self.value_stream_ids.is_empty()
            && self.capability_ids.is_empty()
            && self.application_ids.is_empty()
            && self.interface_levels.is_empty()
            && self.exposures.is_empty()
            && self.gaps.is_empty()
            && self.sort == ProductSort::Name
    }

    /// Nombre de facettes actives, pour le libellé « N filtres ».
    pub fn active_filter_count(&self) -> usize {
        let mut count = 0;
        if !self.search.trim().is_empty() {
            count += 1;
        }
        for empty in [
            self.team_ids.is_empty(),
            self.kinds.is_empty(),
            self.lifecycles.is_empty(),
            self.criticalities.is_empty(),
            self.value_stream_ids.is_empty(),
            self.capability_ids.is_empty(),
            self.application_ids.is_empty(),
            self.interface_levels.is_empty(),
            self.exposures.is_empty(),
            self.gaps.is_empty(),
        ] {
            if !empty {
                count += 1;
            }
        }
        count
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductCard {
    pub id: ObjectId,
    pub name: String,
    pub kind: ProductKind,
    pub lifecycle: LifecycleState,
    pub criticality: Criticality,
    pub value_proposition: String,
    pub team_id: Option<ObjectId>,
    pub team_name: Option<String>,
    pub value_stream_id: Option<ObjectId>,
    pub value_stream_name: Option<String>,
    pub capability_count: usize,
    pub application_count: usize,
    pub exposed_interface_count: usize,
    pub consumed_interface_count: usize,
    pub anomaly_count: usize,
    pub gaps: Vec<ProductGap>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductCatalog {
    pub items: Vec<ProductCard>,
    pub total: usize,
    pub filtered: usize,
}

pub fn product_catalog(data: &MapDataset, query: &ProductQuery) -> ProductCatalog {
    let anomalies = dataset_anomalies(data);
    let mut items = Vec::new();

    for product in &data.products {
        let value_stream_ids = value_stream_ids_of(data, product);
        let interfaces = linked_interfaces(data, product);
        let gaps = product_gaps(data, product, &value_stream_ids);
        if !matches_query(data, product, query, &value_stream_ids, &interfaces, &gaps) {
            continue;
        }

        let team = product.accountable_team_id.and_then(|id| data.team(id));
        let value_stream = product
            .primary_value_stream_id
            .and_then(|id| data.value_stream(id))
            .or_else(|| {
                value_stream_ids
                    .iter()
                    .find_map(|id| data.value_stream(*id))
            });
        let anomaly_count = anomalies
            .iter()
            .filter(|issue| {
                issue.object_id == Some(product.id)
                    || issue
                        .object_id
                        .is_some_and(|id| product.application_ids.contains(&id))
            })
            .count();

        items.push(ProductCard {
            id: product.id,
            name: product.name.clone(),
            kind: product.kind,
            lifecycle: product.lifecycle,
            criticality: product.criticality,
            value_proposition: product.value_proposition.clone(),
            team_id: product.accountable_team_id,
            team_name: team.map(|team| team.name.clone()),
            value_stream_id: value_stream.map(|stream| stream.id),
            value_stream_name: value_stream.map(|stream| stream.name.clone()),
            capability_count: product.capability_ids.len(),
            application_count: product.application_ids.len(),
            exposed_interface_count: interfaces
                .iter()
                .filter(|interface| is_exposed(product, interface))
                .count(),
            consumed_interface_count: interfaces
                .iter()
                .filter(|interface| is_consumed(data, product, interface))
                .count(),
            anomaly_count,
            gaps,
        });
    }

    sort_items(&mut items, query.sort);
    ProductCatalog {
        total: data.products.len(),
        filtered: items.len(),
        items,
    }
}

fn matches_query(
    data: &MapDataset,
    product: &Product,
    query: &ProductQuery,
    value_stream_ids: &BTreeSet<ObjectId>,
    interfaces: &[&MapInterface],
    gaps: &[ProductGap],
) -> bool {
    if !query.kinds.is_empty() && !query.kinds.contains(&product.kind) {
        return false;
    }
    if !query.lifecycles.is_empty() && !query.lifecycles.contains(&product.lifecycle) {
        return false;
    }
    if !query.criticalities.is_empty() && !query.criticalities.contains(&product.criticality) {
        return false;
    }
    if !query.team_ids.is_empty()
        && !product
            .accountable_team_id
            .is_some_and(|id| query.team_ids.contains(&id))
    {
        return false;
    }
    if !query.value_stream_ids.is_empty()
        && !query
            .value_stream_ids
            .iter()
            .any(|id| value_stream_ids.contains(id))
    {
        return false;
    }
    if !query.capability_ids.is_empty()
        && !query
            .capability_ids
            .iter()
            .any(|id| product.capability_ids.contains(id))
    {
        return false;
    }
    if !query.application_ids.is_empty()
        && !query
            .application_ids
            .iter()
            .any(|id| product.application_ids.contains(id))
    {
        return false;
    }
    if !query.interface_levels.is_empty()
        && !interfaces
            .iter()
            .any(|interface| query.interface_levels.contains(&interface.level))
    {
        return false;
    }
    if !query.exposures.is_empty()
        && !interfaces.iter().any(|interface| {
            is_exposed(product, interface) && query.exposures.contains(&interface.exposure)
        })
    {
        return false;
    }
    if !query.gaps.is_empty() && !query.gaps.iter().any(|gap| gaps.contains(gap)) {
        return false;
    }

    let needle = query.search.trim().to_lowercase();
    if !needle.is_empty() {
        let mut haystack = String::new();
        haystack.push_str(&product.name);
        haystack.push('\n');
        haystack.push_str(&product.value_proposition);
        if let Some(team) = product.accountable_team_id.and_then(|id| data.team(id)) {
            haystack.push('\n');
            haystack.push_str(&team.name);
        }
        if let Some(stream) = product
            .primary_value_stream_id
            .and_then(|id| data.value_stream(id))
        {
            haystack.push('\n');
            haystack.push_str(&stream.name);
        }
        for capability in product
            .capability_ids
            .iter()
            .filter_map(|id| data.capability(*id))
        {
            haystack.push('\n');
            haystack.push_str(&capability.name);
        }
        for application in product
            .application_ids
            .iter()
            .filter_map(|id| data.application(*id))
        {
            haystack.push('\n');
            haystack.push_str(&application.name);
        }
        for interface in interfaces {
            haystack.push('\n');
            haystack.push_str(&interface.name);
        }
        if !haystack.to_lowercase().contains(&needle) {
            return false;
        }
    }

    true
}

fn value_stream_ids_of(data: &MapDataset, product: &Product) -> BTreeSet<ObjectId> {
    let mut ids = BTreeSet::new();
    if let Some(id) = product.primary_value_stream_id {
        if data.value_stream(id).is_some() {
            ids.insert(id);
        }
    }
    for stage in &data.stages {
        if stage.product_ids.contains(&product.id)
            && data.value_stream(stage.value_stream_id).is_some()
        {
            ids.insert(stage.value_stream_id);
        }
    }
    ids
}

fn product_gaps(
    data: &MapDataset,
    product: &Product,
    value_stream_ids: &BTreeSet<ObjectId>,
) -> Vec<ProductGap> {
    let mut gaps = Vec::new();
    if product
        .accountable_team_id
        .and_then(|id| data.team(id))
        .is_none()
    {
        gaps.push(ProductGap::NoTeam);
    }
    if value_stream_ids.is_empty() {
        gaps.push(ProductGap::NoValueStream);
    }
    if product.capability_ids.is_empty() {
        gaps.push(ProductGap::NoCapability);
    }
    if product.application_ids.is_empty() {
        gaps.push(ProductGap::NoApplication);
    }
    gaps
}

fn is_exposed(product: &Product, interface: &MapInterface) -> bool {
    interface.provider_object_id == product.id
        || interface.provider_product_id == Some(product.id)
        || (product
            .application_ids
            .contains(&interface.provider_object_id)
            && interface.exposure == InterfaceExposure::Product)
}

fn is_consumed(data: &MapDataset, product: &Product, interface: &MapInterface) -> bool {
    let version_ids: BTreeSet<ObjectId> = data
        .versions
        .iter()
        .filter(|version| version.interface_id == interface.id)
        .map(|version| version.id)
        .collect();
    if version_ids.is_empty() {
        return false;
    }
    data.consumptions.iter().any(|consumption| {
        version_ids.contains(&consumption.interface_version_id)
            && (consumption.consumer_object_id == product.id
                || product
                    .application_ids
                    .contains(&consumption.consumer_object_id))
    })
}

fn linked_interfaces<'a>(data: &'a MapDataset, product: &'a Product) -> Vec<&'a MapInterface> {
    data.interfaces
        .iter()
        .filter(|interface| is_exposed(product, interface) || is_consumed(data, product, interface))
        .collect()
}

fn sort_items(items: &mut [ProductCard], sort: ProductSort) {
    items.sort_by(|left, right| {
        let primary = match sort {
            ProductSort::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
            ProductSort::Criticality => {
                criticality_rank(left.criticality).cmp(&criticality_rank(right.criticality))
            }
            ProductSort::Lifecycle => {
                lifecycle_rank(left.lifecycle).cmp(&lifecycle_rank(right.lifecycle))
            }
            ProductSort::Team => left
                .team_name
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&right.team_name.as_deref().unwrap_or("").to_lowercase()),
        };
        primary.then_with(|| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then(left.id.cmp(&right.id))
        })
    });
}

fn criticality_rank(value: Criticality) -> u8 {
    match value {
        Criticality::Critical => 0,
        Criticality::High => 1,
        Criticality::Medium => 2,
        Criticality::Low => 3,
    }
}

fn lifecycle_rank(value: LifecycleState) -> u8 {
    match value {
        LifecycleState::Idea => 0,
        LifecycleState::Discovery => 1,
        LifecycleState::Build => 2,
        LifecycleState::Run => 3,
        LifecycleState::Sunset => 4,
        LifecycleState::Retired => 5,
    }
}
