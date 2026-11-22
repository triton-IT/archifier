//! Palette de création et de recherche de la carte (Ctrl/⌘ K).
//!
//! Elle ne quitte jamais le canevas : chercher un objet le sélectionne s'il est
//! visible, sinon ouvre sa fiche. Créer pose l'objet au centre de la vue.

use crate::map::commands;
use crate::map::create::CreateKind;
use crate::map::viewport::{MapSelection, MapViewport};
use crate::state::{input_value, repo, App};
use archifier_core::map::*;
use leptos::html::Input;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

const CARD_W: f64 = 158.0;
const CARD_H: f64 = 64.0;

fn quick_create_kinds(level: MapLevel) -> Vec<CreateKind> {
    match level {
        MapLevel::Enterprise | MapLevel::Portfolio => {
            vec![CreateKind::Product, CreateKind::Team]
        }
        MapLevel::Product => vec![CreateKind::Application, CreateKind::Team],
        MapLevel::Application => vec![CreateKind::Team],
        MapLevel::Infrastructure => vec![CreateKind::Node, CreateKind::Team],
    }
}

fn creation_title(kind: CreateKind) -> &'static str {
    match kind {
        CreateKind::Product => "Créer un produit",
        CreateKind::Team => "Créer une équipe",
        CreateKind::Application => "Créer une application",
        CreateKind::Capability => "Créer une capacité",
        CreateKind::Node => "Créer un nœud",
        CreateKind::ValueStream => "Créer un value stream",
        CreateKind::Interface => "Créer une interface",
    }
}

fn creation_detail(kind: CreateKind) -> &'static str {
    match kind {
        CreateKind::Product => "Une unité de valeur, posée au centre de la carte.",
        CreateKind::Team => "Qui possède et opère.",
        CreateKind::Application => "Un système utilisé par le produit.",
        CreateKind::Capability => "Ce que l'entreprise sait faire.",
        CreateKind::Node => "Où cela tourne.",
        CreateKind::ValueStream => "Un parcours de bout en bout.",
        CreateKind::Interface => "Passez par « Relier » sur deux cartes.",
    }
}

fn create_from_palette(
    app: App,
    query: &MapQuery,
    viewport: RwSignal<MapViewport>,
    selected: RwSignal<Option<MapSelection>>,
    focus_title: RwSignal<bool>,
    kind: CreateKind,
) {
    app.map_palette.set(false);
    let viewport = viewport.get_untracked();
    let x = viewport.x + viewport.w / 2.0 - CARD_W / 2.0;
    let y = viewport.y + viewport.h / 2.0 - CARD_H / 2.0;
    let Some(object_id) = commands::create_at(app, kind, query, x, y) else {
        return;
    };
    let projection = repo().map_projection(query);
    if let Some(node) = projection
        .nodes
        .iter()
        .find(|node| node.object_id == object_id)
    {
        selected.set(Some(MapSelection::Node(node.id.clone())));
        focus_title.set(true);
    } else {
        app.notify("Objet créé — retrouvez-le dans la recherche.");
    }
}

fn go_to_hit(
    app: App,
    query: &MapQuery,
    selected: RwSignal<Option<MapSelection>>,
    hit: MapSearchHit,
) {
    app.map_palette.set(false);
    let projection = repo().map_projection(query);
    if let Some(node) = projection
        .nodes
        .iter()
        .find(|node| node.object_id == hit.object_id)
    {
        selected.set(Some(MapSelection::Node(node.id.clone())));
    } else if let Some(window) = web_sys::window() {
        let _ = window
            .location()
            .set_hash(hit.route.trim_start_matches('#'));
    }
}

#[component]
pub fn MapPalette(
    app: App,
    query: MapQuery,
    viewport: RwSignal<MapViewport>,
    selected: RwSignal<Option<MapSelection>>,
    focus_title: RwSignal<bool>,
) -> impl IntoView {
    let text = RwSignal::new(String::new());
    let input_ref: NodeRef<Input> = NodeRef::new();

    Effect::new(move |_| {
        if app.map_palette.get() {
            if let Some(input) = input_ref.get() {
                let _ = input.focus();
            }
        }
    });

    let close = move || {
        app.map_palette.set(false);
        text.set(String::new());
    };

    let on_key = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" {
            ev.prevent_default();
            ev.stop_propagation();
            close();
            return;
        }
        if ev.key() == "Enter" {
            ev.prevent_default();
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Ok(Some(item)) = document.query_selector(".map-palette-item") {
                        if let Some(button) = item.dyn_into::<web_sys::HtmlElement>().ok() {
                            let _ = button.click();
                        }
                    }
                }
            }
        }
    };

    view! {
        {move || app.map_palette.get().then(|| {
            let needle = text.get();
            let searching = needle.trim().chars().count() >= 2;
            let creates = if searching { Vec::new() } else { quick_create_kinds(query.level) };
            let results = if searching { repo().map_search(&needle) } else { Vec::new() };
            let has_results = !results.is_empty();
            let no_results = searching && results.is_empty();
            let query_create = query.clone();
            let query_search = query.clone();
            view! {
                <aside
                    class="map-palette"
                    role="dialog"
                    aria-label="Créer ou rechercher"
                    on:pointerdown=move |ev: web_sys::PointerEvent| ev.stop_propagation()
                >
                    <div class="map-palette-head">
                        <input
                            class="map-palette-input"
                            node_ref=input_ref
                            placeholder="Créer un objet ou aller à…"
                            aria-label="Créer un objet ou aller à"
                            autocomplete="off"
                            prop:value=move || text.get()
                            on:input=move |ev| text.set(input_value(&ev))
                            on:keydown=on_key
                        />
                        <button
                            class="icon-btn"
                            type="button"
                            aria-label="Fermer la palette"
                            on:click=move |_| close()
                        >
                            "×"
                        </button>
                    </div>

                    <div class="map-palette-body">
                        {(!creates.is_empty()).then(|| view! {
                            <div class="map-palette-group">"Créer"</div>
                        })}
                        {creates.into_iter().map(|kind| {
                            let q = query_create.clone();
                            view! {
                                <button
                                    class="map-palette-item"
                                    type="button"
                                    on:click=move |_| {
                                        create_from_palette(app, &q, viewport, selected, focus_title, kind)
                                    }
                                >
                                    <span class="map-palette-icon" aria-hidden="true">"＋"</span>
                                    <span class="map-palette-text">
                                        <strong>{creation_title(kind)}</strong>
                                        <small>{creation_detail(kind)}</small>
                                    </span>
                                </button>
                            }
                        }).collect_view()}

                        {has_results.then(|| view! {
                            <div class="map-palette-group">"Aller à"</div>
                        })}
                        {results.into_iter().map(|hit| {
                            let q = query_search.clone();
                            let hit_for_click = hit.clone();
                            view! {
                                <button
                                    class="map-palette-item"
                                    type="button"
                                    on:click=move |_| {
                                        go_to_hit(app, &q, selected, hit_for_click.clone())
                                    }
                                >
                                    <span class="map-palette-icon" aria-hidden="true">"→"</span>
                                    <span class="map-palette-text">
                                        <strong>{hit.label.clone()}</strong>
                                        <small>{hit.detail.clone()}</small>
                                    </span>
                                </button>
                            }
                        }).collect_view()}

                        {no_results.then(|| view! {
                            <p class="map-palette-empty">
                                "Aucun objet. Effacez la recherche pour créer."
                            </p>
                        })}
                    </div>

                    <div class="map-palette-foot">
                        <span>"↵ premier résultat"</span>
                        <span>"esc fermer"</span>
                    </div>
                </aside>
            }
        })}
    }
}
