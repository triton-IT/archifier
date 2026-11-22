//! Routes de l'accueil, de la carte, de l'Atelier, de la Bibliothèque et du Contrôle
//! (C-06, C-10).

pub mod atelier;
pub mod control;
pub mod design;
pub mod library;

use crate::components::{notation_label, relation_label};
use crate::focus::{
    build_focus_context, focus_context_card, focus_layout, panel, FocusGraph, FocusLink, FocusNode,
};
use crate::i18n::t;
use crate::router::{Route, ViewLink};
use crate::state::{repo, App};
use leptos::prelude::*;

pub fn render(app: App, route: Route) -> AnyView {
    match route {
        Route::Home(query) => {
            let query = query.clone();
            view! {
                {move || {
                    let _ = app.version.get();
                    view! { <crate::catalog::ProductCatalogPage app=app query=query.clone() /> }
                }}
            }
            .into_any()
        }
        Route::Map(query) => view! { <crate::map::MapPage app=app query=query /> }.into_any(),
        // Les pages d'objet relisent le dépôt à chaque écriture : une action
        // inline (rattacher une application, publier une version) met à jour
        // la page sans rechargement ni navigation forcée.
        Route::Product { id } => view! {
            {move || { let _ = app.version.get(); view! { <crate::map::ProductPage app=app id=id /> } }}
        }
        .into_any(),
        Route::Team { id } => view! {
            {move || { let _ = app.version.get(); view! { <crate::map::TeamPage app=app id=id /> } }}
        }
        .into_any(),
        Route::ValueStream { id } => view! {
            {move || { let _ = app.version.get(); view! { <crate::map::ValueStreamPage app=app id=id /> } }}
        }
        .into_any(),
        Route::ValueStreamStage {
            value_stream,
            stage,
        } => view! {
            {move || {
                let _ = app.version.get();
                view! { <crate::map::ValueStreamStagePage app=app value_stream=value_stream stage=stage /> }
            }}
        }
        .into_any(),
        Route::Capability { id } => view! {
            {move || { let _ = app.version.get(); view! { <crate::map::CapabilityPage app=app id=id /> } }}
        }
        .into_any(),
        Route::Application { id } => view! {
            {move || { let _ = app.version.get(); view! { <crate::map::ApplicationPage app=app id=id /> } }}
        }
        .into_any(),
        Route::ApplicationComponent {
            application,
            component,
        } => view! {
            {move || {
                let _ = app.version.get();
                view! { <crate::map::ComponentPage app=app application=application component=component /> }
            }}
        }
        .into_any(),
        Route::Infrastructure { id } => view! {
            {move || { let _ = app.version.get(); view! { <crate::map::InfrastructurePage app=app id=id /> } }}
        }
        .into_any(),
        Route::MapInterface { id } => view! {
            {move || { let _ = app.version.get(); view! { <crate::map::MapInterfacePage app=app id=id /> } }}
        }
        .into_any(),
        Route::BusinessDomain { id } => view! {
            {move || { let _ = app.version.get(); view! { <crate::map::BusinessDomainPage app=app id=id /> } }}
        }
        .into_any(),
        Route::Atelier => view! { <atelier::Atelier app=app initial_intent=None /> }.into_any(),
        Route::ViewNew { intent } => view! {
            <atelier::Atelier app=app initial_intent=intent />
        }
        .into_any(),
        Route::ViewEditor { id, link } => design::Design(app, id, false, link),
        Route::ViewReader { id, link } => design::Design(app, id, true, link),
        Route::ViewAt { at, view } => {
            let id = view
                .or_else(|| crate::state::read_last_view().map(|entry| entry.view_id))
                .or_else(|| repo().diagrams(None).first().map(|d| d.id));
            match id {
                Some(id) => design::Design(
                    app,
                    id,
                    true,
                    ViewLink {
                        t: Some(at),
                        ..Default::default()
                    },
                ),
                None => not_found("view/at".into()),
            }
        }
        Route::Library { tab } => view! { <library::Library app=app tab=tab /> }.into_any(),
        Route::DomainSummary { id } => view! {
            <library::DomainSummary app=app id=id />
        }
        .into_any(),
        Route::Control { tab } => view! { <control::Control app=app tab=tab /> }.into_any(),
        Route::Element { id } => {
            view! { <crate::views::element::ElementView app=app id=id /> }.into_any()
        }
        Route::Relationship { id } => relationship_fiche(id),
        Route::Interface { id, version: _ } => {
            view! { <crate::views::interface::InterfaceView app=app id=id /> }.into_any()
        }
        Route::Version { id } => match repo().interface_of_version(id) {
            Some(interface) => view! {
                <crate::views::interface::InterfaceView app=app id=interface />
            }
            .into_any(),
            None => not_found("version".into()),
        },
        Route::Story { id } => {
            view! { <crate::views::story::StoryView app=app id=id /> }.into_any()
        }
        Route::Review { id } => {
            view! { <crate::views::review::ReviewView app=app id=id /> }.into_any()
        }
        Route::Graph { id } => {
            view! { <crate::views::graph::GraphView app=app id=id /> }.into_any()
        }
        Route::Search { q } => view! { <crate::views::search::Search app=app q=q /> }.into_any(),
        Route::Settings => view! { <crate::views::settings::Settings app=app /> }.into_any(),
        Route::NotFound(message) => not_found(message),
    }
}

fn not_found(message: String) -> AnyView {
    view! {
        <main class="page">
            <h1>{t("state.not_found.title")}</h1>
            <p class="muted">{message}</p>
            <div class="actions">
                <a class="btn primary" href="#/">{t("state.not_found.atelier")}</a>
                <a class="btn ghost" href="#/search">{t("state.not_found.search")}</a>
            </div>
        </main>
    }
    .into_any()
}

fn relationship_fiche(id: archifier_core::ObjectId) -> AnyView {
    let Some(relationship) = repo().relationship(id) else {
        return not_found(format!("{}: {id}", t("state.not_found.title")));
    };
    let source = repo().object_ref(relationship.source_id);
    let target = repo().object_ref(relationship.target_id);
    let source_name = source
        .as_ref()
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "?".into());
    let source_href = source
        .as_ref()
        .map(|r| r.href.clone())
        .unwrap_or_else(|| "#/".into());
    let target_name = target
        .as_ref()
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "?".into());
    let target_href = target
        .as_ref()
        .map(|r| r.href.clone())
        .unwrap_or_else(|| "#/".into());
    let display_type = relation_label(relationship.notation, &relationship.relation_type);

    let left = FocusNode {
        id: relationship.source_id,
        label: source_name.clone(),
        sublabel: "Source".into(),
        color_key: "external".into(),
        route: source_href.clone(),
    };
    let right = FocusNode {
        id: relationship.target_id,
        label: target_name.clone(),
        sublabel: "Cible".into(),
        color_key: "external".into(),
        route: target_href.clone(),
    };
    let context = build_focus_context(FocusGraph {
        center_id: relationship.object.id,
        center_label: relationship.object.name.clone(),
        center_meta: display_type.clone(),
        center_badge: Some("relation".into()),
        empty_label: None,
        inside: Vec::new(),
        left_label: Some("SOURCE".into()),
        left: vec![left],
        right_label: Some("CIBLE".into()),
        right: vec![right],
        links: vec![
            FocusLink {
                from: relationship.source_id,
                to: relationship.object.id,
                label: display_type.clone(),
            },
            FocusLink {
                from: relationship.object.id,
                to: relationship.target_id,
                label: display_type.clone(),
            },
        ],
        center_legend: "Relation".into(),
        inside_legend: None,
        around_legend: Some("Objet lié".into()),
    });
    let context_card = focus_context_card(
        format!(
            "{} · révision {}",
            display_type, relationship.object.revision
        ),
        context,
        None,
    );

    let left_panels = vec![panel(
        "Extrémités",
        view! {
            <dl class="facts">
                <div class="fact">
                    <dt>"Source"</dt>
                    <dd><a href=source_href>{source_name}</a></dd>
                </div>
                <div class="fact">
                    <dt>"Cible"</dt>
                    <dd><a href=target_href>{target_name}</a></dd>
                </div>
            </dl>
        }
        .into_any(),
    )];

    let right_panels = vec![panel(
        "Propriétés",
        view! {
            <dl class="facts">
                <div class="fact">
                    <dt>"Type"</dt>
                    <dd>{display_type.clone()}</dd>
                </div>
                <div class="fact">
                    <dt>"Notation"</dt>
                    <dd>{notation_label(relationship.notation)}</dd>
                </div>
                <div class="fact">
                    <dt>"Révision"</dt>
                    <dd>{relationship.object.revision.to_string()}</dd>
                </div>
            </dl>
        }
        .into_any(),
    )];

    view! {
        <main class="page entity-page entity-focus-page">
            <header class="page-head">
                <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                    <a href="#/map">"Entreprise"</a><span>"›"</span>
                    <span aria-current="page">{relationship.object.name.clone()}</span>
                </nav>
                <div class="entity-title">
                    <div>
                        <h1>{relationship.object.name.clone()}</h1>
                        <p class="muted">{display_type}</p>
                    </div>
                </div>
            </header>
            {focus_layout(left_panels, context_card, right_panels)}
        </main>
    }
    .into_any()
}
