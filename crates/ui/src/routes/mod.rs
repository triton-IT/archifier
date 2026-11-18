//! Routes de l'Atelier, de la Bibliothèque et du Contrôle (C-06).

pub mod atelier;
pub mod control;
pub mod design;
pub mod library;

use crate::components::{notation_label, relation_label};
use crate::i18n::t;
use crate::router::{Route, ViewLink};
use crate::state::{repo, App};
use leptos::prelude::*;

pub fn render(app: App, route: Route) -> AnyView {
    match route {
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
    view! {
        <main class="page">
            <header class="page-head">
                <h1>{relationship.object.name.clone()}</h1>
                <p class="muted">{display_type}</p>
            </header>
            <section class="card">
                <dl class="facts">
                    <dt>"Source"</dt><dd><a href=source_href>{source_name}</a></dd>
                    <dt>"Cible"</dt><dd><a href=target_href>{target_name}</a></dd>
                    <dt>"Notation"</dt><dd>{notation_label(relationship.notation)}</dd>
                    <dt>"Révision"</dt><dd>{relationship.object.revision.to_string()}</dd>
                </dl>
            </section>
        </main>
    }
    .into_any()
}
