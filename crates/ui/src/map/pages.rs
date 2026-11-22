//! Pages d'objet de la carte : produit, équipe, value stream, capacité,
//! application, interface, infrastructure, domaine métier.
//!
//! Chaque page est une lecture contextualisée du même référentiel ; elle ne
//! duplique jamais la carte et renvoie toujours vers son niveau de zoom.
//!
//! Toutes les fiches partagent le même gabarit : un contexte système visuel au
//! centre, les propriétés réparties dans des panneaux latéraux. Seul le graphe
//! central change avec le type d'objet.

use crate::focus::{
    build_focus_context, focus_context_card, focus_layout, panel, ContextSide, ContextVisual,
    FocusGraph, FocusLink, FocusNode,
};
use crate::map::create::{
    ApplicationCreate, BusinessDomainCreate, CapabilityCreate, ComponentCreate, CreateMenu,
    CreateSection, InterfaceCreate, NodeCreate, ProductCreate, StageCreate, TeamCreate,
    ValueStreamCreate,
};
use crate::router::Route;
use crate::state::{input_value, repo, App};
use archifier_core::map::*;
use archifier_core::ObjectId;
use leptos::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

fn object_not_found(kind: &str) -> AnyView {
    view! {
        <main class="page">
            <div class="callout error">
                <strong>{format!("{kind} introuvable")}</strong>
                <p>"L'objet n'existe pas dans ce référentiel. Il a peut-être été supprimé ou le lien est incomplet."</p>
                <a class="btn primary" href="#/map">"Revenir à la carte"</a>
            </div>
        </main>
    }
    .into_any()
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count <= 1 { one } else { many })
}

fn map_href(scope: MapScope, level: MapLevel) -> String {
    Route::Map(MapQuery::default().at_scope(scope).level(level)).to_hash()
}

fn fact(label: &str, value: String) -> AnyView {
    view! {
        <div class="fact">
            <dt>{label.to_string()}</dt>
            <dd>{value}</dd>
        </div>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Contexte visuel des fiches
// ---------------------------------------------------------------------------

fn lifecycle_color_key(lifecycle: LifecycleState) -> &'static str {
    match lifecycle {
        LifecycleState::Run => "run",
        LifecycleState::Build | LifecycleState::Discovery | LifecycleState::Idea => "build",
        LifecycleState::Sunset | LifecycleState::Retired => "sunset",
    }
}

fn context_object(data: &MapDataset, id: ObjectId) -> Option<(String, String, String)> {
    if let Some(application) = data.application(id) {
        return Some((
            application.name.clone(),
            format!(
                "{} · {}",
                application.kind.label(),
                application.lifecycle.label()
            ),
            format!("#/applications/{id}"),
        ));
    }
    if let Some(product) = data.product(id) {
        return Some((
            product.name.clone(),
            format!("Produit · {}", product.kind.label()),
            format!("#/products/{id}"),
        ));
    }
    if let Some(stream) = data.value_stream(id) {
        return Some((
            stream.name.clone(),
            "Value stream".into(),
            format!("#/value-streams/{id}"),
        ));
    }
    if let Some(component) = data.component(id) {
        return Some((
            component.name.clone(),
            format!("Composant · {}", component.kind.label()),
            format!(
                "#/applications/{}/components/{id}",
                component.application_id
            ),
        ));
    }
    if let Some(team) = data.team(id) {
        return Some((
            team.name.clone(),
            format!("Équipe · {}", team.topology.label()),
            format!("#/teams/{id}"),
        ));
    }
    if let Some(capability) = data.capability(id) {
        return Some((
            capability.name.clone(),
            format!("Capacité · niveau {}", capability.level),
            format!("#/capabilities/{id}"),
        ));
    }
    if let Some(stage) = data.stage(id) {
        return Some((
            stage.name.clone(),
            format!("Étape {}", stage.order),
            format!("#/value-streams/{}/stages/{id}", stage.value_stream_id),
        ));
    }
    if let Some(node) = data.node(id) {
        return Some((
            node.name.clone(),
            node.kind.label().into(),
            format!("#/infrastructure/{id}"),
        ));
    }
    if let Some(interface) = data.interface(id) {
        return Some((
            interface.name.clone(),
            format!("Interface · {}", interface.level.label()),
            format!("#/interfaces/{id}"),
        ));
    }
    if let Some(domain) = data.domain(id) {
        return Some((
            domain.name.clone(),
            "Domaine métier".into(),
            format!("#/business-domains/{id}"),
        ));
    }
    None
}

fn focus_node(data: &MapDataset, id: ObjectId, color_key: &str) -> Option<FocusNode> {
    context_object(data, id).map(|(label, sublabel, route)| FocusNode {
        id,
        label,
        sublabel,
        color_key: color_key.into(),
        route,
    })
}

fn build_product_context(data: &MapDataset, product: &Product) -> ContextVisual {
    let mut applications: Vec<&Application> = data
        .applications
        .iter()
        .filter(|application| product.application_ids.contains(&application.id))
        .collect();
    applications.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

    let own_applications: BTreeSet<ObjectId> = product.application_ids.iter().copied().collect();
    let resolve = |object_id: ObjectId| -> Option<ObjectId> {
        if object_id == product.id || own_applications.contains(&object_id) {
            return Some(object_id);
        }
        if let Some(application) = data.application_of_object(object_id) {
            return Some(application.id);
        }
        if data.product(object_id).is_some()
            || data.node(object_id).is_some()
            || data.component(object_id).is_some()
        {
            return Some(object_id);
        }
        None
    };

    let mut external_sides: BTreeMap<ObjectId, ContextSide> = BTreeMap::new();
    let mut links: BTreeMap<(ObjectId, ObjectId), BTreeSet<String>> = BTreeMap::new();
    for consumption in &data.consumptions {
        let Some(version) = data.interface_version(consumption.interface_version_id) else {
            continue;
        };
        let Some(interface) = data.interface(version.interface_id) else {
            continue;
        };
        let Some(provider) = resolve(interface.provider_object_id) else {
            continue;
        };
        let Some(consumer) = resolve(consumption.consumer_object_id) else {
            continue;
        };
        if provider == consumer {
            continue;
        }
        let provider_inside = provider == product.id || own_applications.contains(&provider);
        let consumer_inside = consumer == product.id || own_applications.contains(&consumer);
        if !provider_inside && !consumer_inside {
            continue;
        }
        if !provider_inside {
            external_sides.entry(provider).or_insert(ContextSide::Left);
        }
        if !consumer_inside {
            external_sides.entry(consumer).or_insert(ContextSide::Right);
        }
        links
            .entry((provider, consumer))
            .or_default()
            .insert(interface.name.clone());
    }

    let inside: Vec<FocusNode> = applications
        .iter()
        .map(|application| FocusNode {
            id: application.id,
            label: application.name.clone(),
            sublabel: format!(
                "{} · {}",
                application.kind.label(),
                application.lifecycle.label()
            ),
            color_key: lifecycle_color_key(application.lifecycle).into(),
            route: format!("#/applications/{}", application.id),
        })
        .collect();

    // Au-delà des applications, le contexte produit relie aussi les objets
    // d'appartenance et de responsabilité : value stream, équipe, domaines et
    // capacités. Sans eux, la fiche donne l'impression d'un produit isolé.
    let mut left: Vec<FocusNode> = Vec::new();
    let mut right: Vec<FocusNode> = Vec::new();
    let mut placed: BTreeSet<ObjectId> = BTreeSet::new();

    if let Some(stream_id) = product.primary_value_stream_id {
        if placed.insert(stream_id) {
            if let Some(node) = focus_node(data, stream_id, "external") {
                left.push(node);
            }
        }
        links
            .entry((stream_id, product.id))
            .or_default()
            .insert("value stream principal".into());
    }

    if let Some(team_id) = product.accountable_team_id {
        if placed.insert(team_id) {
            if let Some(node) = focus_node(data, team_id, "external") {
                left.push(node);
            }
        }
        links
            .entry((team_id, product.id))
            .or_default()
            .insert("responsable".into());
    }

    for team_id in &product.contributing_team_ids {
        if placed.insert(*team_id) {
            if let Some(node) = focus_node(data, *team_id, "external") {
                left.push(node);
            }
        }
        links
            .entry((*team_id, product.id))
            .or_default()
            .insert("contribue".into());
    }

    let mut domain_nodes: Vec<FocusNode> = product
        .business_domain_ids
        .iter()
        .filter_map(|domain_id| focus_node(data, *domain_id, "external"))
        .collect();
    domain_nodes.sort_by(|a, b| a.label.cmp(&b.label));
    for node in domain_nodes {
        if placed.insert(node.id) {
            links
                .entry((node.id, product.id))
                .or_default()
                .insert("classifie".into());
            left.push(node);
        }
    }

    let mut capability_nodes: Vec<FocusNode> = product
        .capability_ids
        .iter()
        .filter_map(|capability_id| focus_node(data, *capability_id, "external"))
        .collect();
    capability_nodes.sort_by(|a, b| a.label.cmp(&b.label));
    for node in capability_nodes {
        if placed.insert(node.id) {
            links
                .entry((product.id, node.id))
                .or_default()
                .insert("réalise".into());
            right.push(node);
        }
    }

    let mut left_ids: Vec<ObjectId> = external_sides
        .iter()
        .filter(|(_, side)| **side == ContextSide::Left)
        .map(|(id, _)| *id)
        .collect();
    let mut right_ids: Vec<ObjectId> = external_sides
        .iter()
        .filter(|(_, side)| **side == ContextSide::Right)
        .map(|(id, _)| *id)
        .collect();
    left_ids.sort_by_key(|id| {
        data.object_label(*id)
            .map(|(_, label)| label)
            .unwrap_or_default()
    });
    right_ids.sort_by_key(|id| {
        data.object_label(*id)
            .map(|(_, label)| label)
            .unwrap_or_default()
    });
    for id in left_ids {
        if placed.insert(id) {
            if let Some(node) = focus_node(data, id, "external") {
                left.push(node);
            }
        }
    }
    for id in right_ids {
        if placed.insert(id) {
            if let Some(node) = focus_node(data, id, "external") {
                right.push(node);
            }
        }
    }

    let port_count = data
        .interfaces
        .iter()
        .filter(|interface| {
            interface.provider_product_id == Some(product.id)
                || interface.provider_object_id == product.id
                || (product
                    .application_ids
                    .contains(&interface.provider_object_id)
                    && interface.exposure == InterfaceExposure::Product)
        })
        .count();
    let links: Vec<FocusLink> = links
        .into_iter()
        .map(|((from, to), labels)| FocusLink {
            from,
            to,
            label: labels.into_iter().collect::<Vec<_>>().join(", "),
        })
        .collect();

    build_focus_context(FocusGraph {
        center_id: product.id,
        center_label: product.name.clone(),
        center_meta: format!(
            "Produit {} · {} · {}",
            product.kind.label(),
            product.lifecycle.label(),
            product.criticality.label()
        ),
        center_badge: (port_count > 0).then(|| plural(port_count, "port", "ports")),
        empty_label: Some("Aucune application rattachée".into()),
        inside,
        left_label: (!left.is_empty()).then_some("VALUE STREAM, ÉQUIPE & AMONT".into()),
        left,
        right_label: (!right.is_empty()).then_some("CAPACITÉS & AVAL".into()),
        right,
        links,
        center_legend: "Frontière du produit".into(),
        inside_legend: Some("Application".into()),
        around_legend: Some("Objet lié".into()),
    })
}

// ---------------------------------------------------------------------------
// Produit (L2)
// ---------------------------------------------------------------------------

#[component]
pub fn ProductPage(app: App, id: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    let Some(product) = data.product(id).cloned() else {
        return object_not_found("Produit");
    };
    let team = product
        .accountable_team_id
        .and_then(|team_id| data.team(team_id));
    let value_stream = product
        .primary_value_stream_id
        .and_then(|stream_id| data.value_stream(stream_id));
    let applications = data.applications_of_product(id);
    let capabilities: Vec<Capability> = product
        .capability_ids
        .iter()
        .filter_map(|capability_id| data.capability(*capability_id).cloned())
        .collect();
    let interfaces: Vec<MapInterface> = data
        .interfaces
        .iter()
        .filter(|interface| {
            interface.provider_product_id == Some(id)
                || interface.provider_object_id == id
                || (product
                    .application_ids
                    .contains(&interface.provider_object_id)
                    && interface.exposure == InterfaceExposure::Product)
        })
        .cloned()
        .collect();
    let domain_rows: Vec<(ObjectId, String)> = product
        .business_domain_ids
        .iter()
        .filter_map(|domain_id| {
            data.domain(*domain_id)
                .map(|domain| (domain.id, domain.name.clone()))
        })
        .collect();
    let team_name = team
        .map(|team| team.name.clone())
        .unwrap_or_else(|| "Aucune équipe accountable".into());
    let team_link = team.map(|team| (team.id, team.name.clone()));
    let vs_name = value_stream
        .map(|stream| stream.name.clone())
        .unwrap_or_else(|| "Hors value stream".into());
    let vs_href = value_stream
        .map(|stream| format!("#/value-streams/{}", stream.id))
        .unwrap_or_else(|| "#/map".into());
    let app_links: Vec<(ObjectId, String, String, String)> = applications
        .iter()
        .map(|application| {
            (
                application.id,
                application.name.clone(),
                application.kind.label().to_string(),
                application.lifecycle.label().to_string(),
            )
        })
        .collect();
    let capability_rows: Vec<(ObjectId, String, String)> = capabilities
        .iter()
        .map(|capability| {
            (
                capability.id,
                capability.name.clone(),
                capability.criticality.label().to_string(),
            )
        })
        .collect();
    let interface_rows: Vec<(ObjectId, String, String, Option<String>, usize)> = interfaces
        .iter()
        .map(|interface| {
            (
                interface.id,
                interface.name.clone(),
                format!(
                    "{} · {}",
                    interface.level.label(),
                    interface.exposure.label()
                ),
                data.latest_version(interface.id)
                    .map(|version| version.semantic_version.clone()),
                data.consumptions_of_interface(interface.id).len(),
            )
        })
        .collect();
    // Un produit est concerné par ses propres signaux et par ceux des
    // applications qu'il utilise : un couplage d'application est un risque
    // produit, même si l'objet porteur est l'application.
    let anomalies: Vec<MapAnomaly> = dataset_anomalies(&data)
        .into_iter()
        .filter(|issue| {
            issue.object_id == Some(id)
                || issue
                    .object_id
                    .is_some_and(|object_id| product.application_ids.contains(&object_id))
        })
        .collect();
    let (completeness, signals): (Vec<MapAnomaly>, Vec<MapAnomaly>) = anomalies
        .into_iter()
        .partition(|issue| issue.is_completeness());
    let available_apps: Vec<(String, String)> = data
        .applications
        .iter()
        .filter(|application| !product.application_ids.contains(&application.id))
        .map(|application| (application.id.to_string(), application.name.clone()))
        .collect();
    let available_value_streams: Vec<(String, String)> = data
        .value_streams
        .iter()
        .filter(|stream| product.primary_value_stream_id != Some(stream.id))
        .map(|stream| (stream.id.to_string(), stream.name.clone()))
        .collect();
    let available_capabilities: Vec<(String, String)> = data
        .capabilities
        .iter()
        .filter(|capability| !product.capability_ids.contains(&capability.id))
        .map(|capability| (capability.id.to_string(), capability.name.clone()))
        .collect();
    let has_value_stream = product.primary_value_stream_id.is_some();
    let has_capability = !product.capability_ids.is_empty();
    let has_application = !product.application_ids.is_empty();
    let no_available_apps = available_apps.is_empty();
    let no_available_value_streams = available_value_streams.is_empty();
    let no_available_capabilities = available_capabilities.is_empty();

    let port_count = interfaces.len();
    let context = build_product_context(&data, &product);
    let context_summary = format!(
        "{} · {} · {}",
        plural(context.inside_count(), "application", "applications"),
        plural(context.around_count(), "objet lié", "objets liés"),
        plural(port_count, "port exposé", "ports exposés")
    );
    let product_map_href = map_href(MapScope::product(id), MapLevel::Product);
    let enterprise_map_href = map_href(MapScope::enterprise(), MapLevel::Enterprise);
    let context_card = focus_context_card(
        context_summary,
        context,
        Some((product_map_href.clone(), "Carte L2".into())),
    );

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <a href=vs_href>{vs_name.clone()}</a><span>"›"</span>
                    <span aria-current="page">{product.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{product.name.clone()}</h1>
                        <p class="muted">
                            {format!(
                                "Produit {} · {} · {}",
                                product.kind.label(),
                                product.lifecycle.label(),
                                product.criticality.label()
                            )}
                        </p>
                    </div>
                    <div class="actions">
                        <a class="btn primary" href=product_map_href.clone()>"Voir la carte L2"</a>
                        <a class="btn ghost" href=enterprise_map_href.clone()>"Carte de l'entreprise"</a>
                    </div>
                </div>
            </header>

            <div class="entity-focus">
                <aside class="entity-rail entity-rail-left" aria-label="Responsabilité et complétude">
                    <section class="card">
                        <header class="card-head"><h2>"Responsabilité et valeur"</h2></header>
                        <div class="card-body">
                            <dl class="facts">
                                {fact("Équipe accountable", team_name.clone())}
                                {team_link.map(|(team_id, name)| view! {
                                    <div class="fact">
                                        <dt>"Page équipe"</dt>
                                        <dd><a href=format!("#/teams/{team_id}")>{name}</a></dd>
                                    </div>
                                })}
                                {fact("Value stream principal", vs_name.clone())}
                                <div class="fact">
                                    <dt>"Domaine métier (classification)"</dt>
                                    <dd>
                                        {if domain_rows.is_empty() {
                                            view! { <span class="muted">"Aucun"</span> }.into_any()
                                        } else {
                                            view! {
                                                {domain_rows.into_iter().map(|(domain_id, name)| view! {
                                                    <a class="badge" href=format!("#/business-domains/{domain_id}")>{name}</a>
                                                }).collect_view()}
                                            }.into_any()
                                        }}
                                    </dd>
                                </div>
                                {fact(
                                    "Proposition de valeur",
                                    if product.value_proposition.trim().is_empty() {
                                        "À documenter".into()
                                    } else {
                                        product.value_proposition.clone()
                                    }
                                )}
                            </dl>
                            {(!has_value_stream && !no_available_value_streams).then(|| view! {
                                <ValueStreamAttach app=app product=id options=available_value_streams />
                            })}
                            {(!has_value_stream && no_available_value_streams).then(|| view! {
                                <a class="btn small" href="#/map">
                                    "Créer un value stream depuis la carte"
                                </a>
                            })}
                        </div>
                    </section>

                    <section class="card">
                        <header class="card-head"><h2>"Complétude"</h2></header>
                        <div class="card-body">
                            <ul class="detail-list">
                                <li>
                                    <strong>{if has_value_stream { "✔" } else { "○" }}</strong>
                                    <span class="muted small">" Value stream principal"</span>
                                </li>
                                <li>
                                    <strong>{if has_capability { "✔" } else { "○" }}</strong>
                                    <span class="muted small">" Capacité réalisée"</span>
                                </li>
                                <li>
                                    <strong>{if has_application { "✔" } else { "○" }}</strong>
                                    <span class="muted small">" Application utilisée"</span>
                                </li>
                            </ul>
                            {(!completeness.is_empty()).then(|| view! {
                                <ul class="anomaly-list">
                                    {completeness.into_iter().map(|issue| view! {
                                        <li>
                                            <span class="badge">{issue.code}</span>
                                            <span class="muted small">{issue.message}</span>
                                        </li>
                                    }).collect_view()}
                                </ul>
                            })}
                        </div>
                    </section>

                </aside>

                {context_card}

                <aside class="entity-rail entity-rail-right" aria-label="Applications, capacités et interfaces">
                    <CreateMenu>
                        <CreateSection
                            title="Équipe accountable".to_string()
                            description="La nouvelle équipe devient responsable de ce produit.".to_string()
                        >
                            <TeamCreate app=app product=id />
                        </CreateSection>
                        <CreateSection
                            title="Application".to_string()
                            description="L'application est créée puis rattachée à ce produit comme relation d'usage.".to_string()
                        >
                            <ApplicationCreate app=app product=id />
                        </CreateSection>
                        <CreateSection
                            title="Capacité".to_string()
                            description="La capacité est créée puis déclarée réalisée par ce produit.".to_string()
                        >
                            <CapabilityCreate app=app product=id />
                        </CreateSection>
                        <CreateSection
                            title="Value stream".to_string()
                            description="Le value stream devient le périmètre principal de ce produit.".to_string()
                        >
                            <ValueStreamCreate app=app product=id />
                        </CreateSection>
                        <CreateSection
                            title="Interface".to_string()
                            description="L'interface est déclarée comme port exposé par ce produit.".to_string()
                        >
                            <InterfaceCreate app=app provider=id level=InterfaceLevel::Application />
                        </CreateSection>
                        <CreateSection
                            title="Domaine métier".to_string()
                            description="Le domaine classe ce produit ; il ne le possède pas.".to_string()
                        >
                            <BusinessDomainCreate app=app product=id />
                        </CreateSection>
                    </CreateMenu>

                    <section class="card">
                        <header class="card-head"><h2>"Applications utilisées"</h2></header>
                        <div class="card-body">
                            {if app_links.is_empty() {
                                view! {
                                    <p class="empty">
                                        "Un produit en découverte peut ne pas encore avoir d'application."
                                    </p>
                                }.into_any()
                            } else {
                                view! {
                                    <ul class="detail-list">
                                        {app_links.into_iter().map(|(app_id, name, kind, lifecycle)| view! {
                                            <li>
                                                <a href=format!("#/applications/{app_id}")>{name}</a>
                                                <span class="muted small">{format!("{kind} · {lifecycle}")}</span>
                                            </li>
                                        }).collect_view()}
                                    </ul>
                                }.into_any()
                            }}
                            {(!no_available_apps).then(|| view! {
                                <AddApplication app=app product=id available=available_apps />
                            })}
                            {(!has_application && no_available_apps).then(|| view! {
                                <a class="btn small" href="#/map">
                                    "Créer une application depuis la carte"
                                </a>
                            })}
                            <p class="muted small">
                                "Le produit utilise des applications ; il ne les possède pas. Une application partagée est un couplage à arbitrer, pas une possession du produit."
                            </p>
                        </div>
                    </section>

                    <section class="card">
                        <header class="card-head"><h2>"Capacités réalisées"</h2></header>
                        <div class="card-body">
                            {if capability_rows.is_empty() {
                                view! {
                                    <p class="empty">"Aucune capacité réalisée pour l'instant."</p>
                                }.into_any()
                            } else {
                                view! {
                                    <ul class="detail-list">
                                        {capability_rows.into_iter().map(|(capability_id, name, criticality)| view! {
                                            <li>
                                                <a href=format!("#/capabilities/{capability_id}")>{name}</a>
                                                <span class="muted small">{criticality}</span>
                                            </li>
                                        }).collect_view()}
                                    </ul>
                                }.into_any()
                            }}
                            {(!no_available_capabilities).then(|| view! {
                                <CapabilityAttach app=app product=id options=available_capabilities />
                            })}
                        </div>
                    </section>

                    <section class="card">
                        <header class="card-head"><h2>"Ports et interfaces"</h2></header>
                        <div class="card-body">
                            {if interface_rows.is_empty() {
                                view! { <p class="empty">"Aucune interface exposée."</p> }.into_any()
                            } else {
                                view! {
                                    <ul class="detail-list">
                                        {interface_rows.into_iter().map(|(interface_id, name, detail, version, consumers)| view! {
                                            <li>
                                                <a href=format!("#/interfaces/{interface_id}")>{name}</a>
                                                <span class="muted small">{detail}</span>
                                                <span class="badge">
                                                    {version.unwrap_or_else(|| "sans version".into())}
                                                </span>
                                                <span class="muted small">
                                                    {plural(consumers, "consommateur", "consommateurs")}
                                                </span>
                                            </li>
                                        }).collect_view()}
                                    </ul>
                                }.into_any()
                            }}
                        </div>
                    </section>

                    <section class="card">
                        <header class="card-head"><h2>"Signaux du produit"</h2></header>
                        <div class="card-body">
                            {if signals.is_empty() {
                                view! { <p class="empty">"Aucun signal sur ce produit."</p> }.into_any()
                            } else {
                                view! {
                                    <ul class="anomaly-list">
                                        {signals.into_iter().map(|issue| view! {
                                            <li class=if issue.blocking { "blocking" } else { "" }>
                                                <span class="badge warn">{issue.code}</span>
                                                <span>{issue.message}</span>
                                            </li>
                                        }).collect_view()}
                                    </ul>
                                }.into_any()
                            }}
                        </div>
                    </section>
                </aside>
            </div>
        </main>
    }.into_any()
}

#[component]
fn AddApplication(app: App, product: ObjectId, available: Vec<(String, String)>) -> impl IntoView {
    let value = RwSignal::new(
        available
            .first()
            .map(|(id, _)| id.clone())
            .unwrap_or_default(),
    );
    let choices = available.clone();
    view! {
        <form class="inline-form" on:submit=move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            if let Some(application_id) = crate::state::parse_object_id(&value.get()) {
                if app.run(repo().map_add_application_to_product(product, application_id)).is_some() {
                    app.notify("Application rattachée au produit (relation d'usage).");
                }
            }
        }>
            <select aria-label="Application à rattacher au produit" on:change=move |ev| value.set(input_value(&ev))>
                {choices.into_iter().map(|(id, label)| view! {
                    <option value=id>{label}</option>
                }).collect_view()}
            </select>
            <button class="btn small" type="submit">"Rattacher au produit"</button>
        </form>
    }
}

#[component]
fn ValueStreamAttach(app: App, product: ObjectId, options: Vec<(String, String)>) -> impl IntoView {
    let value = RwSignal::new(
        options
            .first()
            .map(|(id, _)| id.clone())
            .unwrap_or_default(),
    );
    let choices = options;
    view! {
        <form class="inline-form" on:submit=move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            if let Some(value_stream_id) = crate::state::parse_object_id(&value.get()) {
                if app.run(repo().map_set_product_value_stream(product, Some(value_stream_id))).is_some() {
                    app.notify("Value stream rattaché au produit.");
                }
            }
        }>
            <select aria-label="Value stream à rattacher au produit" on:change=move |ev| value.set(input_value(&ev))>
                {choices.into_iter().map(|(id, label)| view! {
                    <option value=id>{label}</option>
                }).collect_view()}
            </select>
            <button class="btn small" type="submit">"Rattacher le value stream"</button>
        </form>
    }
}

#[component]
fn CapabilityAttach(app: App, product: ObjectId, options: Vec<(String, String)>) -> impl IntoView {
    let value = RwSignal::new(
        options
            .first()
            .map(|(id, _)| id.clone())
            .unwrap_or_default(),
    );
    let choices = options;
    view! {
        <form class="inline-form" on:submit=move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            if let Some(capability_id) = crate::state::parse_object_id(&value.get()) {
                if app.run(repo().map_add_capability_to_product(product, capability_id)).is_some() {
                    app.notify("Capacité rattachée au produit.");
                }
            }
        }>
            <select aria-label="Capacité à rattacher au produit" on:change=move |ev| value.set(input_value(&ev))>
                {choices.into_iter().map(|(id, label)| view! {
                    <option value=id>{label}</option>
                }).collect_view()}
            </select>
            <button class="btn small" type="submit">"Rattacher la capacité"</button>
        </form>
    }
}

// ---------------------------------------------------------------------------
// Équipe
// ---------------------------------------------------------------------------

#[component]
pub fn TeamPage(app: App, id: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    let Some(team) = data.team(id).cloned() else {
        return object_not_found("Équipe");
    };
    let products = data.products_of_team(id);
    let applications = data.applications_of_team(id);
    let product_rows: Vec<(ObjectId, String, String)> = products
        .iter()
        .map(|product| {
            (
                product.id,
                product.name.clone(),
                format!("{} · {}", product.kind.label(), product.lifecycle.label()),
            )
        })
        .collect();
    let app_rows: Vec<(ObjectId, String, String)> = applications
        .iter()
        .map(|application| {
            (
                application.id,
                application.name.clone(),
                format!(
                    "{} · {}",
                    application.kind.label(),
                    application.lifecycle.label()
                ),
            )
        })
        .collect();
    let product_ids: Vec<ObjectId> = products.iter().map(|product| product.id).collect();
    let application_ids: Vec<ObjectId> = applications
        .iter()
        .map(|application| application.id)
        .collect();
    let provided_rows: Vec<(ObjectId, String)> = data
        .interfaces
        .iter()
        .filter(|interface| {
            interface
                .provider_product_id
                .is_some_and(|id| product_ids.contains(&id))
                || application_ids.contains(&interface.provider_object_id)
        })
        .map(|interface| (interface.id, interface.name.clone()))
        .collect();
    let consumed_rows: Vec<(ObjectId, String)> = data
        .consumptions
        .iter()
        .filter_map(|consumption| {
            if application_ids.contains(&consumption.consumer_object_id)
                || product_ids.contains(&consumption.consumer_object_id)
            {
                data.interface_version(consumption.interface_version_id)
                    .and_then(|version| data.interface(version.interface_id))
                    .map(|interface| (interface.id, interface.name.clone()))
            } else {
                None
            }
        })
        .collect();

    let left: Vec<FocusNode> = products
        .iter()
        .filter_map(|product| focus_node(&data, product.id, "external"))
        .collect();
    let right: Vec<FocusNode> = applications
        .iter()
        .filter_map(|application| focus_node(&data, application.id, "external"))
        .collect();
    let mut links: Vec<FocusLink> = products
        .iter()
        .map(|product| FocusLink {
            from: id,
            to: product.id,
            label: "possède".into(),
        })
        .collect();
    links.extend(applications.iter().map(|application| FocusLink {
        from: id,
        to: application.id,
        label: "opère".into(),
    }));
    let context = build_focus_context(FocusGraph {
        center_id: id,
        center_label: team.name.clone(),
        center_meta: format!("Équipe {}", team.topology.label()),
        center_badge: Some(plural(
            products.len() + applications.len(),
            "objet",
            "objets",
        )),
        empty_label: None,
        inside: Vec::new(),
        left_label: (!left.is_empty()).then_some("PRODUITS POSSÉDÉS".into()),
        left,
        right_label: (!right.is_empty()).then_some("APPLICATIONS OPÉRÉES".into()),
        right,
        links,
        center_legend: "Équipe".into(),
        inside_legend: None,
        around_legend: Some("Objet de l'équipe".into()),
    });
    let product_count = product_rows.len();
    let app_count = app_rows.len();
    let context_summary = format!(
        "{} · {} · {}",
        plural(product_count, "produit", "produits"),
        plural(app_count, "application", "applications"),
        plural(
            provided_rows.len() + consumed_rows.len(),
            "interface",
            "interfaces"
        )
    );

    let left_panels = vec![
        panel(
            "Mission",
            view! {
                <p>
                    {if team.mission.trim().is_empty() {
                        "Mission à documenter.".into()
                    } else {
                        team.mission.clone()
                    }}
                </p>
                <dl class="facts">
                    {fact("Type d'équipe", team.topology.label().into())}
                    {fact(
                        "Modes d'interaction",
                        if team.interaction_modes.is_empty() {
                            "Non renseigné".into()
                        } else {
                            team.interaction_modes
                                .iter()
                                .map(|mode| mode.label())
                                .collect::<Vec<_>>()
                                .join(", ")
                        }
                    )}
                </dl>
            }
            .into_any(),
        ),
        panel(
            "Produits possédés",
            if product_rows.is_empty() {
                view! {
                    <p class="empty">"Aucun produit possédé."</p>
                    <a class="btn small" href="#/">"Créer un produit"</a>
                }
                .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {product_rows.into_iter().map(|(product_id, name, detail)| view! {
                            <li>
                                <a href=format!("#/products/{product_id}")>{name}</a>
                                <span class="muted small">{detail}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
    ];

    let right_panels = vec![
        panel(
            "Applications opérées",
            if app_rows.is_empty() {
                view! { <p class="empty">"Aucune application opérée."</p> }.into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {app_rows.into_iter().map(|(app_id, name, detail)| view! {
                            <li>
                                <a href=format!("#/applications/{app_id}")>{name}</a>
                                <span class="muted small">{detail}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        panel(
            "Interfaces fournies",
            if provided_rows.is_empty() {
                view! { <p class="empty">"Cette équipe ne fournit encore aucune interface."</p> }
                    .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {provided_rows.into_iter().map(|(interface_id, name)| view! {
                            <li><a href=format!("#/interfaces/{interface_id}")>{name}</a></li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        panel(
            "Interfaces consommées",
            if consumed_rows.is_empty() {
                view! { <p class="empty">"Cette équipe ne consomme encore aucune interface."</p> }
                    .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {consumed_rows.into_iter().map(|(interface_id, name)| view! {
                            <li><a href=format!("#/interfaces/{interface_id}")>{name}</a></li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        view! {
            <CreateMenu>
                <CreateSection
                    title="Produit".to_string()
                    description="Le produit est créé avec cette équipe comme équipe accountable.".to_string()
                >
                    <ProductCreate app=app team=id />
                </CreateSection>
                <CreateSection
                    title="Value stream".to_string()
                    description="Le value stream est créé avec cette équipe comme propriétaire.".to_string()
                >
                    <ValueStreamCreate app=app team=id />
                </CreateSection>
                <CreateSection
                    title="Application".to_string()
                    description="L'application est créée avec cette équipe comme équipe opératrice.".to_string()
                >
                    <ApplicationCreate app=app team=id />
                </CreateSection>
                <CreateSection
                    title="Nœud d'infrastructure".to_string()
                    description="Le nœud est créé avec cette équipe comme fournisseur.".to_string()
                >
                    <NodeCreate app=app team=id />
                </CreateSection>
            </CreateMenu>
        }
        .into_any(),
    ];

    let context_card = focus_context_card(context_summary, context, None);

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <span aria-current="page">{team.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{team.name.clone()}</h1>
                        <p class="muted">
                            {format!(
                                "Équipe {} · {} · {}",
                                team.topology.label(),
                                plural(product_count, "produit", "produits"),
                                plural(app_count, "application", "applications")
                            )}
                        </p>
                    </div>
                    <a class="btn ghost" href="#/map">"Carte de l'entreprise"</a>
                </div>
            </header>
            {focus_layout(left_panels, context_card, right_panels)}
        </main>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Value stream et étape
// ---------------------------------------------------------------------------

#[component]
pub fn ValueStreamPage(app: App, id: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    let Some(stream) = data.value_stream(id).cloned() else {
        return object_not_found("Value stream");
    };
    let stages = data.stages_of(id);
    let stage_rows: Vec<(ObjectId, String, u32, String, Vec<String>, Vec<String>)> = stages
        .iter()
        .map(|stage| {
            let products: Vec<String> = stage
                .product_ids
                .iter()
                .filter_map(|product_id| {
                    data.product(*product_id)
                        .map(|product| product.name.clone())
                })
                .collect();
            let capabilities: Vec<String> = stage
                .capability_ids
                .iter()
                .filter_map(|capability_id| {
                    data.capability(*capability_id)
                        .map(|capability| capability.name.clone())
                })
                .collect();
            (
                stage.id,
                stage.name.clone(),
                stage.order,
                stage.value_item.clone(),
                products,
                capabilities,
            )
        })
        .collect();
    let products: Vec<(ObjectId, String)> = data
        .products
        .iter()
        .filter(|product| product.primary_value_stream_id == Some(id))
        .map(|product| (product.id, product.name.clone()))
        .collect();

    let inside: Vec<FocusNode> = stages
        .iter()
        .map(|stage| FocusNode {
            id: stage.id,
            label: stage.name.clone(),
            sublabel: format!("Étape {} · {}", stage.order, stage.value_item),
            color_key: "build".into(),
            route: format!("#/value-streams/{id}/stages/{}", stage.id),
        })
        .collect();
    let mut product_ids: BTreeSet<ObjectId> = stages
        .iter()
        .flat_map(|stage| stage.product_ids.iter().copied())
        .collect();
    product_ids.extend(products.iter().map(|(product_id, _)| *product_id));
    let capability_ids: BTreeSet<ObjectId> = stages
        .iter()
        .flat_map(|stage| stage.capability_ids.iter().copied())
        .collect();
    let left: Vec<FocusNode> = product_ids
        .iter()
        .filter_map(|product_id| focus_node(&data, *product_id, "external"))
        .collect();
    let right: Vec<FocusNode> = capability_ids
        .iter()
        .filter_map(|capability_id| focus_node(&data, *capability_id, "external"))
        .collect();
    let mut link_map: BTreeMap<(ObjectId, ObjectId), BTreeSet<String>> = BTreeMap::new();
    for stage in &stages {
        for product_id in &stage.product_ids {
            link_map
                .entry((stage.id, *product_id))
                .or_default()
                .insert("contribue".into());
        }
        for capability_id in &stage.capability_ids {
            link_map
                .entry((stage.id, *capability_id))
                .or_default()
                .insert("capacité".into());
        }
    }
    for (product_id, _) in &products {
        link_map
            .entry((id, *product_id))
            .or_default()
            .insert("regroupe".into());
    }
    let links: Vec<FocusLink> = link_map
        .into_iter()
        .map(|((from, to), labels)| FocusLink {
            from,
            to,
            label: labels.into_iter().collect::<Vec<_>>().join(", "),
        })
        .collect();
    let context = build_focus_context(FocusGraph {
        center_id: id,
        center_label: stream.name.clone(),
        center_meta: "Value stream".into(),
        center_badge: Some(plural(stages.len(), "étape", "étapes")),
        empty_label: Some("Aucune étape ordonnée".into()),
        inside,
        left_label: (!left.is_empty()).then_some("PRODUITS CONTRIBUTEURS".into()),
        left,
        right_label: (!right.is_empty()).then_some("CAPACITÉS HABILITANTES".into()),
        right,
        links,
        center_legend: "Value stream".into(),
        inside_legend: Some("Étape".into()),
        around_legend: Some("Contribution".into()),
    });
    let stage_count = stage_rows.len();
    let product_count = product_ids.len();
    let capability_count = capability_ids.len();
    let context_summary = format!(
        "{} · {} · {}",
        plural(stage_count, "étape", "étapes"),
        plural(product_count, "produit", "produits"),
        plural(capability_count, "capacité", "capacités")
    );
    let stream_map_href = map_href(MapScope::value_stream(id), MapLevel::Portfolio);

    let left_panels = vec![
        panel(
            "Proposition de valeur",
            view! {
                <p>
                    {if stream.value_proposition.trim().is_empty() {
                        "À documenter.".into()
                    } else {
                        stream.value_proposition.clone()
                    }}
                </p>
            }
            .into_any(),
        ),
        panel(
            "Produits contributeurs",
            if products.is_empty() {
                view! { <p class="empty">"Aucun produit rattaché à ce value stream."</p> }
                    .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {products.into_iter().map(|(product_id, name)| view! {
                            <li><a href=format!("#/products/{product_id}")>{name}</a></li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
    ];

    let right_panels = vec![
        panel(
            "Étapes ordonnées",
            if stage_rows.is_empty() {
                view! { <p class="empty">"Ce value stream n'a pas encore d'étape."</p> }.into_any()
            } else {
                view! {
                    <ol class="stage-list">
                        {stage_rows.into_iter().map(|(stage_id, name, order, value, stage_products, stage_capabilities)| view! {
                            <li>
                                <a class="stage-name" href=format!("#/value-streams/{id}/stages/{stage_id}")>
                                    <span class="stage-order">{order.to_string()}</span>
                                    <strong>{name}</strong>
                                </a>
                                <span class="muted small">{value}</span>
                                <div class="stage-meta">
                                    {stage_products.into_iter().map(|product| view! { <span class="badge">{product}</span> }).collect_view()}
                                    {stage_capabilities.into_iter().map(|capability| view! { <span class="badge cat">{capability}</span> }).collect_view()}
                                </div>
                            </li>
                        }).collect_view()}
                    </ol>
                }
                .into_any()
            },
        ),
        view! {
            <CreateMenu>
                <CreateSection
                    title="Produit".to_string()
                    description="Le produit est créé avec ce value stream comme périmètre principal.".to_string()
                >
                    <ProductCreate app=app value_stream=id />
                </CreateSection>
                <CreateSection
                    title="Étape".to_string()
                    description="L'étape est ajoutée à la fin de ce value stream.".to_string()
                >
                    <StageCreate app=app value_stream=id />
                </CreateSection>
                <CreateSection
                    title="Capacité".to_string()
                    description="La capacité est créée puis rattachée à une étape de ce value stream.".to_string()
                >
                    <CapabilityCreate app=app value_stream=id />
                </CreateSection>
                <CreateSection
                    title="Équipe propriétaire".to_string()
                    description="L'équipe créée devient propriétaire de ce value stream.".to_string()
                >
                    <TeamCreate app=app value_stream=id />
                </CreateSection>
            </CreateMenu>
        }
        .into_any(),
    ];

    let context_card = focus_context_card(
        context_summary,
        context,
        Some((stream_map_href.clone(), "Voir le quartier L1".into())),
    );

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <span aria-current="page">{stream.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{stream.name.clone()}</h1>
                        <p class="muted">
                            {format!(
                                "Value stream · {} · {}",
                                plural(stage_count, "étape", "étapes"),
                                plural(product_count, "produit", "produits")
                            )}
                        </p>
                    </div>
                    <a class="btn primary" href=stream_map_href.clone()>"Voir le quartier L1"</a>
                </div>
            </header>
            {focus_layout(left_panels, context_card, right_panels)}
        </main>
    }
    .into_any()
}

#[component]
pub fn ValueStreamStagePage(app: App, value_stream: ObjectId, stage: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    let Some(stage) = data.stage(stage).cloned() else {
        return object_not_found("Étape");
    };
    let Some(stream) = data.value_stream(value_stream).cloned() else {
        return object_not_found("Value stream");
    };
    let products: Vec<(ObjectId, String)> = stage
        .product_ids
        .iter()
        .filter_map(|id| {
            data.product(*id)
                .map(|product| (product.id, product.name.clone()))
        })
        .collect();
    let capabilities: Vec<(ObjectId, String)> = stage
        .capability_ids
        .iter()
        .filter_map(|id| {
            data.capability(*id)
                .map(|capability| (capability.id, capability.name.clone()))
        })
        .collect();

    let left: Vec<FocusNode> = products
        .iter()
        .filter_map(|(product_id, _)| focus_node(&data, *product_id, "external"))
        .collect();
    let right: Vec<FocusNode> = capabilities
        .iter()
        .filter_map(|(capability_id, _)| focus_node(&data, *capability_id, "external"))
        .collect();
    let mut links: Vec<FocusLink> = products
        .iter()
        .map(|(product_id, _)| FocusLink {
            from: stage.id,
            to: *product_id,
            label: "contribue".into(),
        })
        .collect();
    links.extend(capabilities.iter().map(|(capability_id, _)| FocusLink {
        from: stage.id,
        to: *capability_id,
        label: "capacité".into(),
    }));
    let context = build_focus_context(FocusGraph {
        center_id: stage.id,
        center_label: stage.name.clone(),
        center_meta: format!("Étape {} · {}", stage.order, stage.value_item),
        center_badge: Some(format!("étape {}", stage.order)),
        empty_label: None,
        inside: Vec::new(),
        left_label: (!left.is_empty()).then_some("PRODUITS CONTRIBUTEURS".into()),
        left,
        right_label: (!right.is_empty()).then_some("CAPACITÉS HABILITANTES".into()),
        right,
        links,
        center_legend: "Étape".into(),
        inside_legend: None,
        around_legend: Some("Contribution".into()),
    });
    let product_count = products.len();
    let capability_count = capabilities.len();
    let process_count = stage.process_ids.len();
    let context_summary = format!(
        "{} · {} · {}",
        plural(product_count, "produit", "produits"),
        plural(capability_count, "capacité", "capacités"),
        plural(process_count, "processus BPMN", "processus BPMN")
    );
    let stream_href = format!("#/value-streams/{value_stream}");

    let left_panels = vec![panel(
        "Produits contributeurs",
        if products.is_empty() {
            view! { <p class="empty">"Aucun produit ne contribue à cette étape."</p> }.into_any()
        } else {
            view! {
                <ul class="detail-list">
                    {products.into_iter().map(|(product_id, name)| view! {
                        <li><a href=format!("#/products/{product_id}")>{name}</a></li>
                    }).collect_view()}
                </ul>
            }
            .into_any()
        },
    )];

    let right_panels = vec![
        panel(
            "Capacités",
            if capabilities.is_empty() {
                view! { <p class="empty">"Aucune capacité habilitante."</p> }.into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {capabilities.into_iter().map(|(capability_id, name)| view! {
                            <li><a href=format!("#/capabilities/{capability_id}")>{name}</a></li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        view! {
            <CreateMenu>
                <CreateSection
                    title="Produit".to_string()
                    description="Le produit est créé dans ce value stream et rattaché à cette étape.".to_string()
                >
                    <ProductCreate app=app value_stream=value_stream stage=stage.id />
                </CreateSection>
                <CreateSection
                    title="Capacité".to_string()
                    description="La capacité est créée puis rattachée à cette étape.".to_string()
                >
                    <CapabilityCreate app=app stage=stage.id />
                </CreateSection>
            </CreateMenu>
        }
        .into_any(),
        panel(
            "Traçabilité BPMN",
            if stage.process_ids.is_empty() {
                view! { <p class="empty">"Aucun processus BPMN n'est rattaché à cette étape."</p> }
                    .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {stage.process_ids.into_iter().map(|process_id| view! {
                            <li><a href=format!("#/view/elements/{process_id}")>"Ouvrir le processus BPMN"</a></li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
    ];

    let context_card = focus_context_card(
        context_summary,
        context,
        Some((stream_href.clone(), "Voir le value stream".into())),
    );

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <a href=stream_href>{stream.name.clone()}</a><span>"›"</span>
                    <span aria-current="page">{stage.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{stage.name.clone()}</h1>
                        <p class="muted">{format!("Étape {} · {}", stage.order, stage.value_item)}</p>
                    </div>
                </div>
            </header>
            {focus_layout(left_panels, context_card, right_panels)}
        </main>
    }.into_any()
}

// ---------------------------------------------------------------------------
// Capacité
// ---------------------------------------------------------------------------

#[component]
pub fn CapabilityPage(app: App, id: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    let Some(capability) = data.capability(id).cloned() else {
        return object_not_found("Capacité");
    };
    let products: Vec<&Product> = data
        .products
        .iter()
        .filter(|product| product.capability_ids.contains(&id))
        .collect();
    let applications: Vec<&Application> = data
        .applications
        .iter()
        .filter(|application| {
            products
                .iter()
                .any(|product| product.application_ids.contains(&application.id))
        })
        .collect();
    let stages: Vec<&ValueStreamStage> = data
        .stages
        .iter()
        .filter(|stage| stage.capability_ids.contains(&id))
        .collect();
    let product_rows: Vec<(ObjectId, String)> = products
        .iter()
        .map(|product| (product.id, product.name.clone()))
        .collect();
    let application_rows: Vec<(ObjectId, String)> = applications
        .iter()
        .map(|application| (application.id, application.name.clone()))
        .collect();
    let stage_rows: Vec<(ObjectId, ObjectId, String)> = stages
        .iter()
        .map(|stage| (stage.value_stream_id, stage.id, stage.name.clone()))
        .collect();

    let left: Vec<FocusNode> = products
        .iter()
        .filter_map(|product| focus_node(&data, product.id, "external"))
        .collect();
    let right: Vec<FocusNode> = applications
        .iter()
        .filter_map(|application| focus_node(&data, application.id, "external"))
        .collect();
    let mut links: Vec<FocusLink> = products
        .iter()
        .map(|product| FocusLink {
            from: id,
            to: product.id,
            label: "réalisée par".into(),
        })
        .collect();
    links.extend(applications.iter().map(|application| FocusLink {
        from: id,
        to: application.id,
        label: "supportée par".into(),
    }));
    let context = build_focus_context(FocusGraph {
        center_id: id,
        center_label: capability.name.clone(),
        center_meta: format!(
            "{} · maturité {}",
            capability.criticality.label(),
            capability
                .maturity
                .map(|value| value.to_string())
                .unwrap_or_else(|| "non mesurée".into())
        ),
        center_badge: Some(format!("niveau {}", capability.level)),
        empty_label: None,
        inside: Vec::new(),
        left_label: (!left.is_empty()).then_some("PRODUITS RÉALISATEURS".into()),
        left,
        right_label: (!right.is_empty()).then_some("APPLICATIONS DE SUPPORT".into()),
        right,
        links,
        center_legend: "Capacité".into(),
        inside_legend: None,
        around_legend: Some("Objet lié".into()),
    });
    let product_count = product_rows.len();
    let application_count = application_rows.len();
    let stage_count = stage_rows.len();
    let context_summary = format!(
        "{} · {} · {}",
        plural(product_count, "produit", "produits"),
        plural(application_count, "application", "applications"),
        plural(stage_count, "étape", "étapes")
    );

    let left_panels = vec![
        panel(
            "Réalisée par",
            if product_rows.is_empty() {
                view! { <p class="empty">"Aucun produit réalisateur."</p> }.into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {product_rows.into_iter().map(|(product_id, name)| view! {
                            <li><a href=format!("#/products/{product_id}")>{name}</a></li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        panel(
            "Étapes de value stream",
            if stage_rows.is_empty() {
                view! { <p class="empty">"Aucune étape ne mobilise cette capacité."</p> }.into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {stage_rows.into_iter().map(|(value_stream_id, stage_id, name)| view! {
                            <li>
                                <a href=format!("#/value-streams/{value_stream_id}/stages/{stage_id}")>{name}</a>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
    ];

    let right_panels = vec![
        panel(
            "Supportée par",
        if application_rows.is_empty() {
            view! { <p class="empty">"Aucune application de support."</p> }.into_any()
        } else {
            view! {
                <ul class="detail-list">
                    {application_rows.into_iter().map(|(application_id, name)| view! {
                        <li><a href=format!("#/applications/{application_id}")>{name}</a></li>
                    }).collect_view()}
                </ul>
            }
            .into_any()
        },
    ),
        view! {
            <CreateMenu>
                <CreateSection
                    title="Produit".to_string()
                    description="Le produit est créé puis déclare réaliser cette capacité.".to_string()
                >
                    <ProductCreate app=app capability=id />
                </CreateSection>
                <CreateSection
                    title="Capacité enfant".to_string()
                    description="La capacité est créée au niveau suivant, sous cette capacité.".to_string()
                >
                    <CapabilityCreate app=app parent=id />
                </CreateSection>
            </CreateMenu>
        }
        .into_any(),
    ];

    let context_card = focus_context_card(context_summary, context, None);

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <span aria-current="page">{capability.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{capability.name.clone()}</h1>
                        <p class="muted">
                            {format!(
                                "Capacité de niveau {} · {} · maturité {}",
                                capability.level,
                                capability.criticality.label(),
                                capability
                                    .maturity
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "non mesurée".into())
                            )}
                        </p>
                    </div>
                    <a class="btn ghost" href="#/map">"Carte de l'entreprise"</a>
                </div>
            </header>
            {focus_layout(left_panels, context_card, right_panels)}
        </main>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Application (L3) et composant
// ---------------------------------------------------------------------------

#[component]
pub fn ApplicationPage(app: App, id: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    let Some(application) = data.application(id).cloned() else {
        return object_not_found("Application");
    };
    let team = application
        .operating_team_id
        .and_then(|team_id| data.team(team_id));
    let products = data.products_using_application(id);
    let components: Vec<ApplicationComponent> = data
        .components
        .iter()
        .filter(|item| item.application_id == id)
        .cloned()
        .collect();
    let interfaces: Vec<MapInterface> = data
        .interfaces
        .iter()
        .filter(|interface| {
            interface.provider_object_id == id
                || components
                    .iter()
                    .any(|component| component.interface_ids.contains(&interface.id))
        })
        .cloned()
        .collect();
    let nodes: Vec<TechnologyNode> = application
        .node_ids
        .iter()
        .filter_map(|node_id| data.node(*node_id).cloned())
        .collect();
    let available_nodes: Vec<(String, String)> = data
        .nodes
        .iter()
        .filter(|node| !application.node_ids.contains(&node.id))
        .map(|node| (node.id.to_string(), node.name.clone()))
        .collect();
    let anomalies: Vec<MapAnomaly> = dataset_anomalies(&data)
        .into_iter()
        .filter(|issue| issue.object_id == Some(id))
        .collect();
    let team_name = team
        .map(|team| team.name.clone())
        .unwrap_or_else(|| "Aucune équipe opératrice".into());
    let used_by = if products.is_empty() {
        "Aucun produit".to_string()
    } else {
        products
            .iter()
            .map(|product| product.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let node_names = if nodes.is_empty() {
        "Aucun déploiement".to_string()
    } else {
        nodes
            .iter()
            .map(|node| node.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let is_leaf_application = matches!(
        application.kind,
        ApplicationKind::Cots | ApplicationKind::Saas | ApplicationKind::External
    );
    let product_rows: Vec<(ObjectId, String, String)> = products
        .iter()
        .map(|product| {
            (
                product.id,
                product.name.clone(),
                format!("{} · {}", product.kind.label(), product.lifecycle.label()),
            )
        })
        .collect();
    let component_rows: Vec<(ObjectId, String, String)> = components
        .iter()
        .map(|component| {
            (
                component.id,
                component.name.clone(),
                component.kind.label().to_string(),
            )
        })
        .collect();

    // Contexte système : l'application au centre, ses composants à l'intérieur,
    // les produits servis, déploiements et consommateurs autour.
    let inside: Vec<FocusNode> = components
        .iter()
        .map(|component| FocusNode {
            id: component.id,
            label: component.name.clone(),
            sublabel: component.kind.label().to_string(),
            color_key: "package".into(),
            route: format!("#/applications/{id}/components/{}", component.id),
        })
        .collect();
    let mut left: Vec<FocusNode> = Vec::new();
    let mut right: Vec<FocusNode> = Vec::new();
    let mut seen_left = BTreeSet::new();
    let mut seen_right = BTreeSet::new();
    let mut link_map: BTreeMap<(ObjectId, ObjectId), BTreeSet<String>> = BTreeMap::new();

    for product in &products {
        if seen_left.insert(product.id) {
            if let Some(node) = focus_node(&data, product.id, "external") {
                left.push(node);
            }
        }
        link_map
            .entry((id, product.id))
            .or_default()
            .insert("utilise".into());
    }
    for node in &nodes {
        if seen_right.insert(node.id) {
            if let Some(focus) = focus_node(&data, node.id, "external") {
                right.push(focus);
            }
        }
        link_map
            .entry((id, node.id))
            .or_default()
            .insert("déployée sur".into());
    }
    for interface in &interfaces {
        for consumption in data.consumptions_of_interface(interface.id) {
            let consumer = consumption.consumer_object_id;
            if consumer == id {
                continue;
            }
            if !seen_left.contains(&consumer) && seen_right.insert(consumer) {
                if let Some(focus) = focus_node(&data, consumer, "external") {
                    right.push(focus);
                }
            }
            link_map
                .entry((id, consumer))
                .or_default()
                .insert(interface.name.clone());
        }
    }
    let own_objects: BTreeSet<ObjectId> = std::iter::once(id)
        .chain(components.iter().map(|component| component.id))
        .collect();
    for consumption in &data.consumptions {
        if !own_objects.contains(&consumption.consumer_object_id) {
            continue;
        }
        let Some(version) = data.interface_version(consumption.interface_version_id) else {
            continue;
        };
        let Some(interface) = data.interface(version.interface_id) else {
            continue;
        };
        let provider = interface.provider_object_id;
        if provider == id || components.iter().any(|item| item.id == provider) {
            continue;
        }
        if !seen_right.contains(&provider) && seen_left.insert(provider) {
            if let Some(focus) = focus_node(&data, provider, "external") {
                left.push(focus);
            }
        }
        link_map
            .entry((provider, id))
            .or_default()
            .insert(interface.name.clone());
    }
    let links: Vec<FocusLink> = link_map
        .into_iter()
        .map(|((from, to), labels)| FocusLink {
            from,
            to,
            label: labels.into_iter().collect::<Vec<_>>().join(", "),
        })
        .collect();
    let context = build_focus_context(FocusGraph {
        center_id: id,
        center_label: application.name.clone(),
        center_meta: format!(
            "Application {} · {} · {}",
            application.kind.label(),
            application.lifecycle.label(),
            application.criticality.label()
        ),
        center_badge: Some(if is_leaf_application {
            "application feuille".into()
        } else {
            plural(components.len(), "composant", "composants")
        }),
        empty_label: Some(if is_leaf_application {
            "Application feuille : aucun composant".into()
        } else {
            "Aucun composant rattaché".into()
        }),
        inside,
        left_label: (!left.is_empty()).then_some("PRODUITS & AMONT".into()),
        left,
        right_label: (!right.is_empty()).then_some("AVAL & DÉPLOIEMENT".into()),
        right,
        links,
        center_legend: "Frontière de l'application".into(),
        inside_legend: Some("Composant".into()),
        around_legend: Some("Objet lié".into()),
    });
    let context_summary = format!(
        "{} · {} · {}",
        plural(components.len(), "composant", "composants"),
        plural(product_rows.len(), "produit servi", "produits servis"),
        plural(interfaces.len(), "interface fournie", "interfaces fournies")
    );
    let application_map_href = map_href(MapScope::application(id), MapLevel::Application);

    let left_panels = vec![
        panel(
            "Exploitation",
            view! {
                <dl class="facts">
                    {fact("Équipe opératrice", team_name)}
                    {fact("Utilisée par", used_by)}
                    {fact("Nœuds", node_names)}
                </dl>
            }
            .into_any(),
        ),
        panel(
            "Produits servis",
            if product_rows.is_empty() {
                view! { <p class="empty">"Aucun produit n'utilise encore cette application."</p> }
                    .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {product_rows.into_iter().map(|(product_id, name, detail)| view! {
                            <li>
                                <a href=format!("#/products/{product_id}")>{name}</a>
                                <span class="muted small">{detail}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        panel(
            "Signaux",
            if anomalies.is_empty() {
                view! { <p class="empty">"Aucun signal sur cette application."</p> }.into_any()
            } else {
                view! {
                    <ul class="anomaly-list">
                        {anomalies.into_iter().map(|issue| view! {
                            <li class=if issue.blocking { "blocking" } else { "" }>
                                <span class="badge warn">{issue.code}</span>
                                <span>{issue.message}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
    ];

    let right_panels = vec![
        view! {
            <CreateMenu>
                <CreateSection
                    title="Composant".to_string()
                    description="Le composant est ajouté à la frontière de cette application.".to_string()
                >
                    <ComponentCreate app=app application=id />
                </CreateSection>
                <CreateSection
                    title="Interface".to_string()
                    description="L'interface est déclarée comme port fourni par cette application.".to_string()
                >
                    <InterfaceCreate app=app provider=id level=InterfaceLevel::Application />
                </CreateSection>
                <CreateSection
                    title="Produit".to_string()
                    description="Le produit est créé puis rattaché à cette application comme relation d'usage.".to_string()
                >
                    <ProductCreate app=app application=id />
                </CreateSection>
                <CreateSection
                    title="Nœud de déploiement".to_string()
                    description="Le nœud est créé puis cette application y est déployée.".to_string()
                >
                    <NodeCreate app=app application=id />
                </CreateSection>
                <CreateSection
                    title="Équipe opératrice".to_string()
                    description="L'équipe créée devient immédiatement opératrice de cette application.".to_string()
                >
                    <TeamCreate app=app application=id />
                </CreateSection>
            </CreateMenu>
        }
        .into_any(),
        panel(
            "Composants",
            view! {
                {if component_rows.is_empty() {
                    let message = if is_leaf_application {
                        "Application feuille assumée : aucun composant n'est attendu à ce niveau."
                    } else {
                        "Cette application n'a pas encore de composant. Ajoutez le premier, ou déclarez-la comme application feuille."
                    };
                    view! { <p class="empty">{message}</p> }.into_any()
                } else {
                    view! {
                        <ul class="detail-list">
                            {component_rows.into_iter().map(|(component_id, name, kind)| view! {
                                <li>
                                    <a href=format!("#/applications/{id}/components/{component_id}")>{name}</a>
                                    <span class="muted small">{kind}</span>
                                </li>
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }}
            }
            .into_any(),
        ),
        panel(
            "Interfaces fournies",
            if interfaces.is_empty() {
                view! {
                    <p class="empty">
                        "Cette application ne fournit encore aucune interface. Déclarez-la comme application feuille ou exposez une interface."
                    </p>
                }
                .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {interfaces.into_iter().map(|interface| view! {
                            <li>
                                <a href=format!("#/interfaces/{}", interface.id)>{interface.name.clone()}</a>
                                <span class="muted small">
                                    {format!("{} · {}", interface.level.label(), interface.exposure.label())}
                                </span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        panel(
            "Déploiement",
            view! {
                {if nodes.is_empty() {
                    view! { <p class="empty">"Aucun nœud de déploiement."</p> }.into_any()
                } else {
                    view! {
                        <ul class="detail-list">
                            {nodes.into_iter().map(|node| view! {
                                <li>
                                    <a href=format!("#/infrastructure/{}", node.id)>{node.name.clone()}</a>
                                    <span class="muted small">{node.kind.label()}</span>
                                </li>
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }}
                {(!available_nodes.is_empty()).then(|| view! {
                    <DeployApp app=app application=id nodes=available_nodes />
                })}
            }
            .into_any(),
        ),
    ];

    let context_card = focus_context_card(
        context_summary,
        context,
        Some((application_map_href.clone(), "Carte L3".into())),
    );

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <span aria-current="page">{application.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{application.name.clone()}</h1>
                        <p class="muted">
                            {format!(
                                "Application {} · {} · {}",
                                application.kind.label(),
                                application.lifecycle.label(),
                                application.criticality.label()
                            )}
                        </p>
                    </div>
                    <div class="actions">
                        <a class="btn primary" href=application_map_href.clone()>"Voir la carte L3"</a>
                        <a class="btn ghost" href="#/map">"Carte de l'entreprise"</a>
                    </div>
                </div>
            </header>
            {focus_layout(left_panels, context_card, right_panels)}
        </main>
    }.into_any()
}

// ---------------------------------------------------------------------------
// Composant applicatif
// ---------------------------------------------------------------------------

#[component]
pub fn ComponentPage(app: App, application: ObjectId, component: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    let Some(application) = data.application(application).cloned() else {
        return object_not_found("Application");
    };
    let Some(component) = data
        .component(component)
        .filter(|item| item.application_id == application.id)
        .cloned()
    else {
        return object_not_found("Composant");
    };
    let products = data.products_using_application(application.id);
    let interfaces: Vec<MapInterface> = data
        .interfaces
        .iter()
        .filter(|interface| {
            interface.provider_object_id == component.id
                || component.interface_ids.contains(&interface.id)
        })
        .cloned()
        .collect();
    let nodes: Vec<TechnologyNode> = application
        .node_ids
        .iter()
        .filter_map(|node_id| data.node(*node_id).cloned())
        .collect();

    let product_rows: Vec<(ObjectId, String, String)> = products
        .iter()
        .map(|product| {
            (
                product.id,
                product.name.clone(),
                format!("{} · {}", product.kind.label(), product.lifecycle.label()),
            )
        })
        .collect();
    let interface_rows: Vec<(ObjectId, String, String)> = interfaces
        .iter()
        .map(|interface| {
            (
                interface.id,
                interface.name.clone(),
                format!(
                    "{} · {}",
                    interface.level.label(),
                    interface.exposure.label()
                ),
            )
        })
        .collect();
    let node_rows: Vec<(ObjectId, String, String)> = nodes
        .iter()
        .map(|node| {
            (
                node.id,
                node.name.clone(),
                node.environment
                    .map(|environment| environment.label().to_string())
                    .unwrap_or_else(|| node.kind.label().to_string()),
            )
        })
        .collect();

    let application_href = format!("#/applications/{}", application.id);
    let application_map_href =
        map_href(MapScope::application(application.id), MapLevel::Application);

    // Contexte système : le composant au centre, son application d'appartenance
    // en amont, les interfaces qu'il fournit en aval.
    let left: Vec<FocusNode> = focus_node(&data, application.id, "external")
        .into_iter()
        .collect();
    let right: Vec<FocusNode> = interfaces
        .iter()
        .filter_map(|interface| focus_node(&data, interface.id, "external"))
        .collect();
    let mut links = vec![FocusLink {
        from: application.id,
        to: component.id,
        label: "compose".into(),
    }];
    links.extend(interfaces.iter().map(|interface| FocusLink {
        from: component.id,
        to: interface.id,
        label: "expose".into(),
    }));

    let context = build_focus_context(FocusGraph {
        center_id: component.id,
        center_label: component.name.clone(),
        center_meta: format!(
            "Composant {} · {}",
            component.kind.label(),
            application.name
        ),
        center_badge: Some(plural(interfaces.len(), "interface", "interfaces")),
        empty_label: None,
        inside: Vec::new(),
        left_label: Some("APPLICATION".into()),
        left,
        right_label: (!right.is_empty()).then_some("INTERFACES FOURNIES".into()),
        right,
        links,
        center_legend: "Composant".into(),
        inside_legend: None,
        around_legend: Some("Objet lié".into()),
    });
    let context_summary = format!(
        "{} · {} · {}",
        plural(interfaces.len(), "interface", "interfaces"),
        plural(product_rows.len(), "produit servi", "produits servis"),
        plural(node_rows.len(), "nœud", "nœuds")
    );

    let left_panels = vec![
        panel(
            "Appartenance",
            view! {
                <dl class="facts">
                    <div class="fact">
                        <dt>"Application"</dt>
                        <dd><a href=application_href.clone()>{application.name.clone()}</a></dd>
                    </div>
                    {fact("Type de composant", component.kind.label().to_string())}
                    {fact("Révision", component.revision.to_string())}
                </dl>
            }
            .into_any(),
        ),
        panel(
            "Produits servis",
            if product_rows.is_empty() {
                view! { <p class="empty">"Aucun produit n'utilise cette application."</p> }
                    .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {product_rows.into_iter().map(|(product_id, name, detail)| view! {
                            <li>
                                <a href=format!("#/products/{product_id}")>{name}</a>
                                <span class="muted small">{detail}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
    ];

    let right_panels = vec![
        panel(
            "Interfaces fournies",
            if interface_rows.is_empty() {
                view! { <p class="empty">"Ce composant ne fournit aucune interface."</p> }
                    .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {interface_rows.into_iter().map(|(interface_id, name, detail)| view! {
                            <li>
                                <a href=format!("#/interfaces/{interface_id}")>{name}</a>
                                <span class="muted small">{detail}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        panel(
            "Déploiement",
            if node_rows.is_empty() {
                view! { <p class="empty">"Aucun nœud de déploiement pour l'application."</p> }
                    .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {node_rows.into_iter().map(|(node_id, name, detail)| view! {
                            <li>
                                <a href=format!("#/infrastructure/{node_id}")>{name}</a>
                                <span class="muted small">{detail}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        view! {
            <CreateMenu>
                <CreateSection
                    title="Interface".to_string()
                    description="L'interface est déclarée comme port fourni par ce composant.".to_string()
                >
                    <InterfaceCreate app=app provider=component.id level=InterfaceLevel::Application />
                </CreateSection>
                <CreateSection
                    title="Composant voisin".to_string()
                    description="Le composant est créé dans la même application que celui-ci.".to_string()
                >
                    <ComponentCreate app=app application=application.id />
                </CreateSection>
                <CreateSection
                    title="Produit".to_string()
                    description="Le produit est créé puis rattaché à l'application qui porte ce composant.".to_string()
                >
                    <ProductCreate app=app application=application.id />
                </CreateSection>
                <CreateSection
                    title="Nœud de déploiement".to_string()
                    description="Le nœud est créé puis l'application qui porte ce composant y est déployée.".to_string()
                >
                    <NodeCreate app=app application=application.id />
                </CreateSection>
            </CreateMenu>
        }
        .into_any(),
    ];

    let context_card = focus_context_card(
        context_summary,
        context,
        Some((application_map_href.clone(), "Carte L3".into())),
    );

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <a href=application_href.clone()>{application.name.clone()}</a><span>"›"</span>
                    <span aria-current="page">{component.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{component.name.clone()}</h1>
                        <p class="muted">
                            {format!(
                                "Composant {} · {}",
                                component.kind.label(),
                                application.name
                            )}
                        </p>
                    </div>
                    <div class="actions">
                        <a class="btn primary" href=application_map_href.clone()>"Voir la carte L3"</a>
                        <a class="btn ghost" href=application_href.clone()>"Ouvrir l'application"</a>
                    </div>
                </div>
            </header>
            {focus_layout(left_panels, context_card, right_panels)}
        </main>
    }.into_any()
}

#[component]
fn DeployApp(app: App, application: ObjectId, nodes: Vec<(String, String)>) -> impl IntoView {
    let value = RwSignal::new(nodes.first().map(|(id, _)| id.clone()).unwrap_or_default());
    let choices = nodes.clone();
    view! {
        <form class="inline-form" on:submit=move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            if let Some(node_id) = crate::state::parse_object_id(&value.get()) {
                if app.run(repo().map_deploy_application(application, node_id)).is_some() {
                    app.notify("Application déployée sur le nœud.");
                }
            }
        }>
            <select aria-label="Nœud de déploiement" on:change=move |ev| value.set(input_value(&ev))>
                {choices.into_iter().map(|(id, label)| view! {
                    <option value=id>{label}</option>
                }).collect_view()}
            </select>
            <button class="btn small" type="submit">"Déployer sur ce nœud"</button>
        </form>
    }
}

// ---------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------

#[component]
pub fn MapInterfacePage(app: App, id: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    let Some(interface) = data.interface(id).cloned() else {
        return object_not_found("Interface");
    };
    let provider = data.object_label(interface.provider_object_id);
    let provider_name = provider
        .as_ref()
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| "Fournisseur inconnu".into());
    let versions = data.versions_of(id);
    let latest_version = versions.first().cloned();
    let contract = latest_version
        .as_ref()
        .and_then(|version| version.contract_type)
        .map(|kind| kind.label().to_string())
        .unwrap_or_else(|| "Aucun contrat".into());
    let consumptions = data.consumptions_of_interface(id);
    let consumer_ids: Vec<ObjectId> = consumptions
        .iter()
        .map(|consumption| consumption.consumer_object_id)
        .collect();
    let mut candidates: Vec<(String, String)> = Vec::new();
    for product in &data.products {
        if product.id != interface.provider_object_id && !consumer_ids.contains(&product.id) {
            candidates.push((
                product.id.to_string(),
                format!("Produit · {}", product.name),
            ));
        }
    }
    for application in &data.applications {
        if application.id != interface.provider_object_id && !consumer_ids.contains(&application.id)
        {
            candidates.push((
                application.id.to_string(),
                format!("Application · {}", application.name),
            ));
        }
    }
    for component in &data.components {
        if component.id != interface.provider_object_id && !consumer_ids.contains(&component.id) {
            candidates.push((
                component.id.to_string(),
                format!("Composant · {}", component.name),
            ));
        }
    }
    for node in &data.nodes {
        if node.id != interface.provider_object_id && !consumer_ids.contains(&node.id) {
            candidates.push((node.id.to_string(), format!("Nœud · {}", node.name)));
        }
    }
    let consumers: Vec<(String, String, String)> = consumptions
        .iter()
        .map(|consumption| {
            let (kind, label) = data
                .object_label(consumption.consumer_object_id)
                .unwrap_or((MapObjectKind::Product, "Consommateur inconnu".into()));
            (
                label,
                kind.label().to_string(),
                consumption.criticality.label().to_string(),
            )
        })
        .collect();
    let rows: Vec<(String, String, String, String, bool)> = versions
        .iter()
        .map(|version| {
            (
                version.semantic_version.clone(),
                version.publication_status.label().to_string(),
                version.available_from.clone().unwrap_or_else(|| "—".into()),
                version
                    .available_until
                    .clone()
                    .unwrap_or_else(|| "—".into()),
                version.breaking_change,
            )
        })
        .collect();
    let interface_id = interface.id;
    let next_version = rows
        .first()
        .map(|(version, _, _, _, _)| bump_minor(version))
        .unwrap_or_else(|| "1.0.0".into());
    let consumer_version = latest_version.as_ref().map(|version| version.id);

    let left: Vec<FocusNode> = focus_node(&data, interface.provider_object_id, "external")
        .into_iter()
        .collect();
    let right: Vec<FocusNode> = consumer_ids
        .iter()
        .filter_map(|consumer_id| focus_node(&data, *consumer_id, "external"))
        .collect();
    let mut links = vec![FocusLink {
        from: interface.provider_object_id,
        to: id,
        label: "fournit".into(),
    }];
    links.extend(consumer_ids.iter().map(|consumer_id| FocusLink {
        from: id,
        to: *consumer_id,
        label: "consomme".into(),
    }));
    let context = build_focus_context(FocusGraph {
        center_id: id,
        center_label: interface.name.clone(),
        center_meta: format!(
            "Interface {} · {} · {}",
            interface.level.label(),
            interface.kind.label(),
            interface.exposure.label()
        ),
        center_badge: Some(
            latest_version
                .as_ref()
                .map(|version| format!("v{}", version.semantic_version))
                .unwrap_or_else(|| "sans version".into()),
        ),
        empty_label: None,
        inside: Vec::new(),
        left_label: (!left.is_empty()).then_some("FOURNISSEUR".into()),
        left,
        right_label: (!right.is_empty()).then_some("CONSOMMATEURS".into()),
        right,
        links,
        center_legend: "Interface".into(),
        inside_legend: None,
        around_legend: Some("Objet lié".into()),
    });
    let version_count = rows.len();
    let consumer_count = consumers.len();
    let context_summary = format!(
        "{} · {} · {}",
        plural(version_count, "version", "versions"),
        plural(consumer_count, "consommateur", "consommateurs"),
        contract
    );

    let left_panels = vec![
        panel(
            "Contrat",
            view! {
                <dl class="facts">
                    {fact("Fournisseur", provider_name)}
                    {fact("Contrat", contract.clone())}
                    {fact(
                        "Classification des données",
                        interface
                            .data_classification
                            .map(|value| value.label().to_string())
                            .unwrap_or_else(|| "Non renseignée".into())
                    )}
                    {fact("Consommateurs", plural(consumer_count, "consommateur", "consommateurs"))}
                </dl>
            }
            .into_any(),
        ),
        panel(
            "Versions",
            view! {
                {if rows.is_empty() {
                    view! {
                        <p class="empty">
                            "Aucune version publiée. Publiez une première version avant de déclarer un consommateur."
                        </p>
                    }.into_any()
                } else {
                    view! {
                        <ul class="detail-list">
                            {rows.into_iter().map(|(version, status, from, until, breaking)| view! {
                                <li>
                                    <strong>{version}</strong>
                                    <span class="muted small">{format!("{status} · {from} → {until}")}</span>
                                    {breaking.then(|| view! { <span class="badge warn">"rupture"</span> })}
                                </li>
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }}
                <VersionCreate app=app interface=interface_id suggested=next_version />
            }
            .into_any(),
        ),
    ];

    let right_panels = vec![panel(
        "Consommateurs",
        view! {
            {if consumers.is_empty() {
                view! { <p class="empty">"Aucun consommateur versionné."</p> }.into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {consumers.into_iter().map(|(label, kind, criticality)| view! {
                            <li>
                                <strong>{label}</strong>
                                <span class="muted small">{format!("{kind} · {criticality}")}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
            {if let Some(version) = latest_version {
                if !candidates.is_empty() {
                    view! {
                        <ConsumptionCreate
                            app=app
                            version=version.id
                            candidates=candidates
                        />
                    }.into_any()
                } else {
                    ().into_any()
                }
            } else {
                view! {
                    <p class="muted small">"Publiez une version avant d'ajouter un consommateur."</p>
                }.into_any()
            }}
            <p class="muted small">
                "Une consommation référence toujours une version précise. Une rupture crée une nouvelle version ; les consommateurs existants restent attachés à l'ancienne."
            </p>
        }
        .into_any(),
    ),
    view! {
        <CreateMenu>
            <CreateSection
                title="Produit consommateur".to_string()
                description="Le produit est créé puis rattaché à la dernière version de cette interface.".to_string()
            >
                {consumer_version
                    .map(|version| view! { <ProductCreate app=app consume_version=version /> }.into_any())
                    .unwrap_or_else(|| view! {
                        <p class="empty">"Publiez une version avant de créer un consommateur."</p>
                    }.into_any())}
            </CreateSection>
            <CreateSection
                title="Application consommatrice".to_string()
                description="L'application est créée puis rattachée à la dernière version de cette interface.".to_string()
            >
                {consumer_version
                    .map(|version| view! { <ApplicationCreate app=app consume_version=version /> }.into_any())
                    .unwrap_or_else(|| view! {
                        <p class="empty">"Publiez une version avant de créer un consommateur."</p>
                    }.into_any())}
            </CreateSection>
            <CreateSection
                title="Composant consommateur".to_string()
                description="Le composant est créé puis rattaché à la dernière version de cette interface.".to_string()
            >
                {consumer_version
                    .map(|version| view! { <ComponentCreate app=app consume_version=version /> }.into_any())
                    .unwrap_or_else(|| view! {
                        <p class="empty">"Publiez une version avant de créer un consommateur."</p>
                    }.into_any())}
            </CreateSection>
            <CreateSection
                title="Nœud consommateur".to_string()
                description="Le nœud est créé puis rattaché à la dernière version de cette interface.".to_string()
            >
                {consumer_version
                    .map(|version| view! { <NodeCreate app=app consume_version=version /> }.into_any())
                    .unwrap_or_else(|| view! {
                        <p class="empty">"Publiez une version avant de créer un consommateur."</p>
                    }.into_any())}
            </CreateSection>
        </CreateMenu>
    }
    .into_any(),
    ];

    let context_card = focus_context_card(context_summary, context, None);

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <span aria-current="page">{interface.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{interface.name.clone()}</h1>
                        <p class="muted">
                            {format!(
                                "Interface {} · {} · {} · {}",
                                interface.level.label(),
                                interface.kind.label(),
                                interface.exposure.label(),
                                interface.criticality.label()
                            )}
                        </p>
                    </div>
                </div>
            </header>
            {focus_layout(left_panels, context_card, right_panels)}
        </main>
    }
    .into_any()
}

#[component]
fn ConsumptionCreate(
    app: App,
    version: ObjectId,
    candidates: Vec<(String, String)>,
) -> impl IntoView {
    let value = RwSignal::new(
        candidates
            .first()
            .map(|(id, _)| id.clone())
            .unwrap_or_default(),
    );
    let choices = candidates;
    view! {
        <form class="inline-form" on:submit=move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            if let Some(consumer_id) = crate::state::parse_object_id(&value.get()) {
                if app.run(repo().map_add_interface_consumption(
                    version,
                    consumer_id,
                    Criticality::Medium,
                )).is_some() {
                    app.notify("Consommateur ajouté à cette version.");
                }
            }
        }>
            <select aria-label="Consommateur à rattacher à cette version" on:change=move |ev| value.set(input_value(&ev))>
                {choices.into_iter().map(|(id, label)| view! {
                    <option value=id>{label}</option>
                }).collect_view()}
            </select>
            <button class="btn small" type="submit">"Ajouter le consommateur"</button>
        </form>
    }
}

#[component]
fn VersionCreate(app: App, interface: ObjectId, suggested: String) -> impl IntoView {
    let version = RwSignal::new(suggested);
    let breaking = RwSignal::new(false);
    let from = RwSignal::new("2026-11-19".to_string());
    view! {
        <form class="inline-form" on:submit=move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            if app.run(repo().map_create_interface_version(archifier_repo::map::NewVersion {
                interface_id: interface,
                semantic_version: version.get(),
                available_from: Some(from.get()),
                available_until: None,
                breaking_change: breaking.get(),
            })).is_some() {
                app.notify("Nouvelle version publiée.");
            }
        }>
            <input aria-label="Version" prop:value=move || version.get()
                on:input=move |ev| version.set(input_value(&ev)) />
            <input aria-label="Disponible à partir du" type="date" prop:value=move || from.get()
                on:change=move |ev| from.set(input_value(&ev)) />
            <label class="inline-check">
                <input type="checkbox" on:change=move |ev| breaking.set(crate::state::is_checked(&ev)) />
                "Rupture"
            </label>
            <button class="btn small" type="submit">"Publier la version"</button>
        </form>
    }
}

fn bump_minor(version: &str) -> String {
    let mut parts = version.split('.');
    let major = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or(1);
    let minor = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or(0);
    format!("{major}.{}.0", minor + 1)
}

// ---------------------------------------------------------------------------
// Infrastructure
// ---------------------------------------------------------------------------

#[component]
pub fn InfrastructurePage(app: App, id: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    let Some(node) = data.node(id).cloned() else {
        return object_not_found("Nœud");
    };
    let team = node.provider_team_id.and_then(|team_id| data.team(team_id));
    let applications: Vec<&Application> = data
        .applications
        .iter()
        .filter(|application| application.node_ids.contains(&id))
        .collect();
    let interfaces: Vec<&MapInterface> = data
        .interfaces
        .iter()
        .filter(|interface| {
            interface.provider_object_id == id && interface.level == InterfaceLevel::Infrastructure
        })
        .collect();
    let team_name = team
        .map(|team| team.name.clone())
        .unwrap_or_else(|| "Non renseignée".into());
    let application_rows: Vec<(ObjectId, String, String)> = applications
        .iter()
        .map(|application| {
            (
                application.id,
                application.name.clone(),
                application.kind.label().to_string(),
            )
        })
        .collect();
    let interface_rows: Vec<(ObjectId, String)> = interfaces
        .iter()
        .map(|interface| (interface.id, interface.name.clone()))
        .collect();

    let inside: Vec<FocusNode> = applications
        .iter()
        .map(|application| FocusNode {
            id: application.id,
            label: application.name.clone(),
            sublabel: application.kind.label().to_string(),
            color_key: lifecycle_color_key(application.lifecycle).into(),
            route: format!("#/applications/{}", application.id),
        })
        .collect();
    let left: Vec<FocusNode> = node
        .provider_team_id
        .and_then(|team_id| focus_node(&data, team_id, "external"))
        .into_iter()
        .collect();
    let right: Vec<FocusNode> = interfaces
        .iter()
        .filter_map(|interface| focus_node(&data, interface.id, "external"))
        .collect();
    let mut links = Vec::new();
    if let Some(team_id) = node.provider_team_id {
        links.push(FocusLink {
            from: team_id,
            to: id,
            label: "fournit".into(),
        });
    }
    links.extend(interfaces.iter().map(|interface| FocusLink {
        from: id,
        to: interface.id,
        label: "expose".into(),
    }));
    let context = build_focus_context(FocusGraph {
        center_id: id,
        center_label: node.name.clone(),
        center_meta: format!(
            "Nœud {} · {} · {}",
            node.kind.label(),
            node.environment
                .map(|value| value.label().to_string())
                .unwrap_or_else(|| "sans environnement".into()),
            node.lifecycle.label()
        ),
        center_badge: Some(plural(applications.len(), "application", "applications")),
        empty_label: Some("Aucune application déployée".into()),
        inside,
        left_label: (!left.is_empty()).then_some("ÉQUIPE FOURNISSEUR".into()),
        left,
        right_label: (!right.is_empty()).then_some("INTERFACES TECHNIQUES".into()),
        right,
        links,
        center_legend: "Nœud d'infrastructure".into(),
        inside_legend: Some("Application déployée".into()),
        around_legend: Some("Objet lié".into()),
    });
    let application_count = application_rows.len();
    let interface_count = interface_rows.len();
    let context_summary = format!(
        "{} · {} · {}",
        plural(application_count, "application", "applications"),
        plural(
            interface_count,
            "interface technique",
            "interfaces techniques"
        ),
        node.environment
            .map(|value| value.label().to_string())
            .unwrap_or_else(|| "sans environnement".into())
    );
    let node_map_href = map_href(MapScope::node(id), MapLevel::Infrastructure);

    let left_panels = vec![
        panel(
            "Exploitation",
            view! {
                <dl class="facts">
                    {fact("Équipe fournisseur", team_name)}
                    {fact("Applications déployées", application_count.to_string())}
                </dl>
            }
            .into_any(),
        ),
        panel(
            "Interfaces techniques",
            if interface_rows.is_empty() {
                view! { <p class="empty">"Aucune interface technique exposée par ce nœud."</p> }
                    .into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {interface_rows.into_iter().map(|(interface_id, name)| view! {
                            <li><a href=format!("#/interfaces/{interface_id}")>{name}</a></li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
    ];

    let right_panels = vec![
        panel(
            "Applications déployées",
            if application_rows.is_empty() {
                view! { <p class="empty">"Aucune application déployée sur ce nœud."</p> }.into_any()
            } else {
                view! {
                    <ul class="detail-list">
                        {application_rows.into_iter().map(|(application_id, name, kind)| view! {
                            <li>
                                <a href=format!("#/applications/{application_id}")>{name}</a>
                                <span class="muted small">{kind}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            },
        ),
        view! {
            <CreateMenu>
                <CreateSection
                    title="Équipe fournisseur".to_string()
                    description="L'équipe créée devient fournisseur de ce nœud.".to_string()
                >
                    <TeamCreate app=app node=id />
                </CreateSection>
                <CreateSection
                    title="Application déployée".to_string()
                    description="L'application est créée puis déployée sur ce nœud.".to_string()
                >
                    <ApplicationCreate app=app node=id />
                </CreateSection>
                <CreateSection
                    title="Interface technique".to_string()
                    description="L'interface est déclarée comme port d'infrastructure exposé par ce nœud.".to_string()
                >
                    <InterfaceCreate app=app provider=id level=InterfaceLevel::Infrastructure />
                </CreateSection>
            </CreateMenu>
        }
        .into_any(),
    ];

    let context_card = focus_context_card(
        context_summary,
        context,
        Some((node_map_href.clone(), "Carte L4".into())),
    );

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <span aria-current="page">{node.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{node.name.clone()}</h1>
                        <p class="muted">
                            {format!(
                                "Nœud {} · {} · {}",
                                node.kind.label(),
                                node.environment
                                    .map(|value| value.label().to_string())
                                    .unwrap_or_else(|| "sans environnement".into()),
                                node.lifecycle.label()
                            )}
                        </p>
                    </div>
                    <a class="btn primary" href=node_map_href.clone()>"Voir la carte L4"</a>
                </div>
            </header>
            {focus_layout(left_panels, context_card, right_panels)}
        </main>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Domaine métier (classification, jamais propriétaire)
// ---------------------------------------------------------------------------

#[component]
pub fn BusinessDomainPage(app: App, id: ObjectId) -> impl IntoView {
    let _ = app.version.get();
    let data = repo().map_data();
    if let Some(domain) = data.domain(id).cloned() {
        let products: Vec<&Product> = data
            .products
            .iter()
            .filter(|product| product.business_domain_ids.contains(&id))
            .collect();
        let product_rows: Vec<(ObjectId, String)> = products
            .iter()
            .map(|product| (product.id, product.name.clone()))
            .collect();
        let team_ids: BTreeSet<ObjectId> = products
            .iter()
            .filter_map(|product| product.accountable_team_id)
            .collect();
        let team_rows: Vec<(ObjectId, String)> = team_ids
            .iter()
            .filter_map(|team_id| {
                data.team(*team_id)
                    .map(|team| (*team_id, team.name.clone()))
            })
            .collect();

        let left: Vec<FocusNode> = products
            .iter()
            .filter_map(|product| focus_node(&data, product.id, "external"))
            .collect();
        let right: Vec<FocusNode> = team_ids
            .iter()
            .filter_map(|team_id| focus_node(&data, *team_id, "external"))
            .collect();
        let mut links: Vec<FocusLink> = products
            .iter()
            .map(|product| FocusLink {
                from: id,
                to: product.id,
                label: "classifie".into(),
            })
            .collect();
        links.extend(team_ids.iter().map(|team_id| FocusLink {
            from: id,
            to: *team_id,
            label: "responsable".into(),
        }));
        let context = build_focus_context(FocusGraph {
            center_id: id,
            center_label: domain.name.clone(),
            center_meta: "Domaine métier · classification".into(),
            center_badge: Some(plural(products.len(), "produit", "produits")),
            empty_label: None,
            inside: Vec::new(),
            left_label: (!left.is_empty()).then_some("PRODUITS CLASSÉS".into()),
            left,
            right_label: (!right.is_empty()).then_some("ÉQUIPES RESPONSABLES".into()),
            right,
            links,
            center_legend: "Domaine métier".into(),
            inside_legend: None,
            around_legend: Some("Objet lié".into()),
        });
        let product_count = product_rows.len();
        let team_count = team_rows.len();
        let context_summary = format!(
            "{} · {}",
            plural(product_count, "produit classé", "produits classés"),
            plural(team_count, "équipe responsable", "équipes responsables")
        );

        let left_panels = vec![
            panel(
                "Classification",
                view! {
                    <p>
                        {if domain.description.trim().is_empty() {
                            "Description à documenter.".into()
                        } else {
                            domain.description.clone()
                        }}
                    </p>
                    <p class="muted small">
                        "Un domaine métier classe ; il ne possède rien. La responsabilité reste portée par une équipe."
                    </p>
                }
                .into_any(),
            ),
            panel(
                "Produits classés",
                if product_rows.is_empty() {
                    view! { <p class="empty">"Aucun produit classé dans ce domaine."</p> }.into_any()
                } else {
                    view! {
                        <ul class="detail-list">
                            {product_rows.into_iter().map(|(product_id, name)| view! {
                                <li><a href=format!("#/products/{product_id}")>{name}</a></li>
                            }).collect_view()}
                        </ul>
                    }
                    .into_any()
                },
            ),
        ];

        let right_panels = vec![
            panel(
                "Équipes responsables",
                if team_rows.is_empty() {
                    view! { <p class="empty">"Aucune équipe responsable identifiée."</p> }
                        .into_any()
                } else {
                    view! {
                        <ul class="detail-list">
                            {team_rows.into_iter().map(|(team_id, name)| view! {
                                <li><a href=format!("#/teams/{team_id}")>{name}</a></li>
                            }).collect_view()}
                        </ul>
                    }
                    .into_any()
                },
            ),
            view! {
                <CreateMenu>
                    <CreateSection
                        title="Produit classé".to_string()
                        description="Le produit est créé puis classé dans ce domaine. Le domaine ne le possède pas.".to_string()
                    >
                        <ProductCreate app=app domain=id />
                    </CreateSection>
                    <CreateSection
                        title="Sous-domaine".to_string()
                        description="Le domaine est créé comme enfant de celui-ci.".to_string()
                    >
                        <BusinessDomainCreate app=app parent=id />
                    </CreateSection>
                </CreateMenu>
            }
            .into_any(),
        ];

        let context_card = focus_context_card(context_summary, context, None);

        return view! {
            <main class="page entity-page entity-focus-page">
                <header class="page-head">
                    <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                        <a href="#/map">"Entreprise"</a><span>"›"</span>
                        <span aria-current="page">{domain.name.clone()}</span>
                    </nav>
                    <div class="entity-title">
                        <div>
                            <h1>{domain.name.clone()}</h1>
                            <p class="muted">"Domaine métier · classification de connaissance"</p>
                        </div>
                        <a class="btn ghost" href="#/map">"Carte de l'entreprise"</a>
                    </div>
                </header>
                {focus_layout(left_panels, context_card, right_panels)}
            </main>
        }
        .into_any();
    }
    // Compatibilité additive : un domaine du MVP3 reste lisible et ne perd ni
    // son nom ni son historique. Il n'est pas converti en produit.
    if let Some(domain) = repo().domain(id) {
        let name = domain.object.name.clone();
        let description = domain.description.clone();
        let overview = repo().domain_overview(id).ok();

        let mut left: Vec<FocusNode> = Vec::new();
        let mut right: Vec<FocusNode> = Vec::new();
        let mut links: Vec<FocusLink> = Vec::new();
        if let Some(overview) = &overview {
            for row in &overview.exposed_interfaces {
                left.push(FocusNode {
                    id: row.id,
                    label: row.name.clone(),
                    sublabel: format!("Interface · {}", row.category.label()),
                    color_key: "external".into(),
                    route: row.href.clone(),
                });
                links.push(FocusLink {
                    from: id,
                    to: row.id,
                    label: "expose".into(),
                });
            }
            for row in &overview.consumed_interfaces {
                right.push(FocusNode {
                    id: row.id,
                    label: row.name.clone(),
                    sublabel: format!("Interface · {}", row.category.label()),
                    color_key: "external".into(),
                    route: row.href.clone(),
                });
                links.push(FocusLink {
                    from: row.id,
                    to: id,
                    label: "consomme".into(),
                });
            }
        }
        let context = build_focus_context(FocusGraph {
            center_id: id,
            center_label: name.clone(),
            center_meta: "Domaine historique (MVP3)".into(),
            center_badge: Some(if left.is_empty() && right.is_empty() {
                "classification".into()
            } else {
                format!("{} interface(s)", left.len() + right.len())
            }),
            empty_label: None,
            inside: Vec::new(),
            left_label: (!left.is_empty()).then_some("INTERFACES EXPOSÉES".into()),
            left,
            right_label: (!right.is_empty()).then_some("INTERFACES CONSOMMÉES".into()),
            right,
            links,
            center_legend: "Domaine historique".into(),
            inside_legend: None,
            around_legend: Some("Interface".into()),
        });
        let context_summary = format!(
            "{} · {}",
            if context.around_count() <= 1 {
                format!("{} interface liée", context.around_count())
            } else {
                format!("{} interfaces liées", context.around_count())
            },
            "migration additive"
        );
        let context_card = focus_context_card(context_summary, context, None);

        let left_panels = vec![panel(
            "Migration guidée",
            view! {
                <p>{description}</p>
                <p class="muted small">
                    "Ce domaine n'a pas été converti automatiquement : une équipe accountable et un produit restent à créer explicitement. Aucune donnée n'a été supprimée."
                </p>
                <a class="btn primary" href="#/">"Créer le produit qui reprend ce périmètre"</a>
            }
            .into_any(),
        )];

        let right_panels = if let Some(overview) = overview {
            vec![
                panel(
                    "Interfaces exposées",
                    if overview.exposed_interfaces.is_empty() {
                        view! { <p class="empty">"Aucune interface exposée."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="detail-list">
                                {overview.exposed_interfaces.into_iter().map(|row| view! {
                                    <li><a href=format!("#/view/interfaces/{}", row.id)>{row.name.clone()}</a></li>
                                }).collect_view()}
                            </ul>
                        }
                        .into_any()
                    },
                ),
                panel(
                    "Interfaces consommées",
                    if overview.consumed_interfaces.is_empty() {
                        view! { <p class="empty">"Aucune interface consommée."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="detail-list">
                                {overview.consumed_interfaces.into_iter().map(|row| view! {
                                    <li><a href=format!("#/view/interfaces/{}", row.id)>{row.name.clone()}</a></li>
                                }).collect_view()}
                            </ul>
                        }
                        .into_any()
                    },
                ),
            ]
        } else {
            vec![]
        };

        return view! {
            <main class="page entity-page entity-focus-page">
                <header class="page-head">
                    <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                        <a href="#/map">"Entreprise"</a><span>"›"</span>
                        <span aria-current="page">{name.clone()}</span>
                    </nav>
                    <div class="entity-title">
                        <div>
                            <h1>{name.clone()}</h1>
                            <p class="muted">"Domaine historique (MVP3) — conservé comme classification, migration additive"</p>
                        </div>
                        <a class="btn ghost" href="#/library">"Bibliothèque"</a>
                    </div>
                </header>
                {focus_layout(left_panels, context_card, right_panels)}
            </main>
        }
        .into_any();
    }
    object_not_found("Domaine métier")
}
