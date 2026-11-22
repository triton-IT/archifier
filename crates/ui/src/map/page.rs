//! `/map` — la Carte de l'entreprise (ADR-023/024/025, C-10).
//!
//! La projection est calculée depuis le dépôt à chaque rendu. Le SVG est une
//! représentation ; la liste des objets et le panneau latéral portent la même
//! information, sans dépendre de la couleur seule.
//!
//! Mise en page : une barre de contexte, une barre de lecture, puis la carte.
//! Les panneaux (signaux, liste, détail d'une route) sont des tiroirs ouverts à
//! la demande : aucun panneau vide ne reste affiché.

use crate::map::commands;
use crate::map::create::{CreateKind, CreateTray};
use crate::map::palette::MapPalette;
use crate::map::viewport::{
    MapGesture, MapLinkSource, MapNodeDrag, MapPan, MapSelection, MapViewport,
};
use crate::router::Route;
use crate::state::{input_value, repo, App};
use archifier_core::map::*;
use archifier_core::ObjectId;
use leptos::html::Input;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapDrawer {
    Signals,
    List,
    About,
    Ports,
}

#[component]
pub fn MapPage(app: App, query: MapQuery) -> impl IntoView {
    let open = RwSignal::new(false);
    // L'accueil ouvre la création sur le produit (ADR-024) ; l'équipe
    // accountable reste une condition du produit, pas le point d'entrée.
    let kind = RwSignal::new(CreateKind::Product);
    let selected = RwSignal::new(None::<MapSelection>);
    let gesture = RwSignal::new(None::<MapGesture>);
    let drawer = RwSignal::new(None::<MapDrawer>);
    let focus_title = RwSignal::new(false);
    let linking = RwSignal::new(None::<MapLinkSource>);
    app.map_palette.set(false);
    let initial_projection = repo().map_projection(&query);
    let viewport = RwSignal::new(MapViewport::content(&initial_projection));
    let fitted = RwSignal::new(false);
    let fit_query = query.clone();

    // Si la carte était vide au montage, le premier objet créé (ou l'exemple
    // chargé) doit cadrer la carte une fois. Ensuite, le cadrage appartient à
    // l'utilisateur : un déplacement ne le réinitialise plus.
    Effect::new(move |_| {
        let _ = app.version.get();
        if !fitted.get_untracked() && !repo().map_is_empty() {
            let projection = repo().map_projection(&fit_query);
            viewport.set(MapViewport::content(&projection));
            fitted.set(true);
        }
    });

    let route_query = query.clone();
    view! {
        <main class="page map-page">
            {move || {
                let _ = app.version.get();
                let projection = repo().map_projection(&route_query);
                let header = map_header(app, &projection, open, drawer, selected);
                let workspace = map_workspace(
                    app,
                    projection,
                    selected,
                    gesture,
                    viewport,
                    drawer,
                    focus_title,
                    linking,
                    open,
                    kind,
                );
                view! { {header} {workspace} }.into_any()
            }}
            <CreateTray app=app open=open kind=kind />
        </main>
    }
}

fn map_header(
    app: App,
    projection: &MapProjection,
    open: RwSignal<bool>,
    drawer: RwSignal<Option<MapDrawer>>,
    selected: RwSignal<Option<MapSelection>>,
) -> AnyView {
    let query = projection.query.clone();
    let level = query.level;
    let has_products = !repo().map_data().products.is_empty();
    let show_reading = has_products && !projection.nodes.is_empty();
    let signal_count = projection
        .anomalies
        .iter()
        .filter(|issue| !issue.is_completeness())
        .count();
    let completeness_count = projection
        .anomalies
        .iter()
        .filter(|issue| issue.is_completeness())
        .count();
    let has_signals = signal_count > 0;

    let crumbs = projection.breadcrumb.clone();
    let scope_title = crumbs
        .last()
        .map(|entry| entry.label.clone())
        .unwrap_or_else(|| "Carte de l'entreprise".into());
    let lens = query.primary_lens();
    let lenses: Vec<MapLens> = MapLens::ALL
        .iter()
        .copied()
        .filter(|lens| lens.available_at(level))
        .collect();
    let date_value = query
        .date
        .as_deref()
        .map(|value| value.chars().take(10).collect::<String>())
        .unwrap_or_default();
    let date_label = date_label(query.date.as_deref());
    let legend = projection.legend.clone();
    let ports_count = projection.ports.len();
    let node_count = projection.nodes.len();

    let query_lens = query.clone();
    let query_date = query.clone();
    let query_today = query.clone();
    let query_back = query.clone();
    let enterprise = MapScope::enterprise();
    // La carte n'est plus l'accueil (ADR-025). La lentille suit la lecture,
    // mais le titre reste celui de la carte.
    let show_product_reading = level == MapLevel::Enterprise && lens == MapLens::Product;

    view! {
        <header class="map-head">
            <div class="map-bar">
                <div class="map-bar-main">
                    {if level == MapLevel::Enterprise {
                        view! {
                            <div class="map-bar-heading">
                                <h1 class="map-bar-title">"Carte de l'entreprise"</h1>
                                {show_product_reading.then(|| view! {
                                    <span class="map-bar-sub">"Lecture Produits"</span>
                                })}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <h1 class="sr-only">{scope_title}</h1>
                            <nav class="map-breadcrumb" aria-label="Fil d'Ariane de la carte">
                                {{
                                    let total = crumbs.len();
                                    crumbs.into_iter().enumerate().map(|(index, entry)| {
                                        let href = entry.route.clone();
                                        view! {
                                            <a href=href on:click=move |_| selected.set(None)>{entry.label.clone()}</a>
                                            {(index + 1 < total).then(|| view! { <span aria-hidden="true">"›"</span> })}
                                        }
                                    }).collect_view()
                                }}
                            </nav>
                        }.into_any()
                    }}
                </div>
                <div class="map-bar-actions">
                    <button
                        class="map-chip map-search"
                        type="button"
                        title="Rechercher dans le référentiel (Ctrl+K)"
                        on:click=move |_| app.map_palette.update(|open| *open = !*open)
                    >
                        <span aria-hidden="true">"⌕"</span>
                        <span>"Rechercher"</span>
                        <kbd class="nav-kbd">"Ctrl K"</kbd>
                    </button>
                    {show_reading.then(|| view! {
                        <details class="map-popover map-layers">
                            <summary class=move || if has_signals { "map-chip map-chip-attention" } else { "map-chip" }>
                                "Calques"
                                {has_signals.then(|| view! {
                                    <span class="map-signal-dot" aria-hidden="true"></span>
                                })}
                            </summary>
                            <div class="map-pop map-layers-pop">
                                <section class="map-pop-section">
                                    <span class="map-pop-title">"Lecture"</span>
                                    <label class="map-lens-select">
                                        <span>"Lire par"</span>
                                        <select
                                            on:change=move |ev| {
                                                let value = input_value(&ev);
                                                if let Some(lens) = MapLens::parse(&value) {
                                                    selected.set(None);
                                                    let mut next = query_lens.clone();
                                                    next.lenses = vec![lens];
                                                    app.navigate(Route::Map(next));
                                                }
                                            }
                                        >
                                            {lenses.into_iter().map(|candidate| view! {
                                                <option
                                                    value=candidate.as_str()
                                                    selected=candidate == lens
                                                    title=format!("Lire : {}", lens_label(candidate))
                                                >
                                                    {lens_label(candidate)}
                                                </option>
                                            }).collect_view()}
                                        </select>
                                    </label>
                                    {(!legend.entries.is_empty()).then(move || view! {
                                        <ul class="legend map-legend-pop">
                                            {legend.entries.into_iter().map(|entry| view! {
                                                <li>
                                                    <span class=format!("swatch c-{}", entry.key)></span>
                                                    <span>{entry.label}</span>
                                                </li>
                                            }).collect_view()}
                                        </ul>
                                    })}
                                </section>

                                <section class="map-pop-section">
                                    <span class="map-pop-title">"Date de lecture"</span>
                                    <div class="map-date-fields">
                                        <label class="map-field">
                                            <span>"À la date du"</span>
                                            <input
                                                type="date"
                                                prop:value=date_value
                                                on:change=move |ev| {
                                                    let value = input_value(&ev);
                                                    selected.set(None);
                                                    let mut next = query_date.clone();
                                                    next.date = if value.trim().is_empty() {
                                                        None
                                                    } else {
                                                        Some(value)
                                                    };
                                                    app.navigate(Route::Map(next));
                                                }
                                            />
                                        </label>
                                        <button
                                            class="btn ghost small"
                                            type="button"
                                            on:click=move |_| {
                                                selected.set(None);
                                                let mut next = query_today.clone();
                                                next.date = None;
                                                app.navigate(Route::Map(next));
                                            }
                                        >
                                            {date_label}
                                        </button>
                                    </div>
                                </section>

                                <section class="map-pop-section">
                                    <span class="map-pop-title">"Panneaux"</span>
                                    <button
                                        type="button"
                                        on:click=move |_| open_drawer(selected, drawer, MapDrawer::Signals)
                                    >
                                        {if signal_count > 0 {
                                            format!("Signaux ({signal_count})")
                                        } else {
                                            format!("À compléter ({completeness_count})")
                                        }}
                                    </button>
                                    <button
                                        type="button"
                                        on:click=move |_| open_drawer(selected, drawer, MapDrawer::List)
                                    >
                                        {format!("Liste des objets ({node_count})")}
                                    </button>
                                    {(!projection.ports.is_empty()).then(|| view! {
                                        <button
                                            type="button"
                                            on:click=move |_| open_drawer(selected, drawer, MapDrawer::Ports)
                                        >
                                            {plural(ports_count, "port", "ports")}
                                        </button>
                                    })}
                                    <button
                                        type="button"
                                        on:click=move |_| open_drawer(selected, drawer, MapDrawer::About)
                                    >
                                        "À propos de cette carte"
                                    </button>
                                </section>

                                {(level != MapLevel::Enterprise).then(|| {
                                    let mut back = query_back.clone();
                                    back.scope = enterprise;
                                    back.level = MapLevel::Enterprise;
                                    back.focus = vec![];
                                    view! {
                                        <a
                                            class="map-pop-back"
                                            href=Route::Map(back).to_hash()
                                            on:click=move |_| selected.set(None)
                                        >
                                            "Revenir à l'entreprise"
                                        </a>
                                    }
                                })}
                            </div>
                        </details>
                    })}
                    {has_products.then(|| view! {
                        <button
                            class="btn primary small"
                            type="button"
                            on:click=move |_| open.set(true)
                        >
                            "＋ Créer"
                        </button>
                    })}
                </div>
            </div>
        </header>
    }
    .into_any()
}

fn open_drawer(
    selected: RwSignal<Option<MapSelection>>,
    drawer: RwSignal<Option<MapDrawer>>,
    kind: MapDrawer,
) {
    selected.set(None);
    drawer.set(Some(kind));
}

fn map_workspace(
    app: App,
    projection: MapProjection,
    selected: RwSignal<Option<MapSelection>>,
    gesture: RwSignal<Option<MapGesture>>,
    viewport: RwSignal<MapViewport>,
    drawer: RwSignal<Option<MapDrawer>>,
    focus_title: RwSignal<bool>,
    linking: RwSignal<Option<MapLinkSource>>,
    open: RwSignal<bool>,
    kind: RwSignal<CreateKind>,
) -> AnyView {
    let repo_empty = repo().map_is_empty();
    let data = repo().map_data();

    if repo_empty {
        return onboarding(app, open, kind, &data, true);
    }
    if data.products.is_empty() {
        return onboarding(app, open, kind, &data, false);
    }
    if projection.nodes.is_empty() && projection.districts.is_empty() {
        return scope_empty_state(&projection);
    }
    map_stage(
        app,
        projection,
        selected,
        gesture,
        viewport,
        drawer,
        focus_title,
        linking,
    )
}

fn map_stage(
    app: App,
    projection: MapProjection,
    selected: RwSignal<Option<MapSelection>>,
    gesture: RwSignal<Option<MapGesture>>,
    viewport: RwSignal<MapViewport>,
    drawer: RwSignal<Option<MapDrawer>>,
    focus_title: RwSignal<bool>,
    linking: RwSignal<Option<MapLinkSource>>,
) -> AnyView {
    let content = MapViewport::content(&projection);
    let canvas = map_canvas(
        app,
        projection.clone(),
        selected,
        gesture,
        viewport,
        content,
        drawer,
        linking,
        focus_title,
    );
    let inspector = map_inspector(
        app,
        projection.clone(),
        selected,
        viewport,
        focus_title,
        linking,
    );
    let palette = view! {
        <MapPalette
            app=app
            query=projection.query.clone()
            viewport=viewport
            selected=selected
            focus_title=focus_title
        />
    };
    let drawers = map_drawers(projection, drawer);
    let link_hint = move || {
        linking.get().map(|_| {
            view! {
                <div class="map-link-hint" role="status">
                    <span>"Relier : cliquez une seconde carte."</span>
                    <button
                        class="btn ghost small"
                        type="button"
                        on:click=move |_| linking.set(None)
                    >
                        "Annuler"
                    </button>
                </div>
            }
        })
    };

    view! {
        <section class="map-stage" aria-label="Carte de l'entreprise">
            {canvas}
            {inspector}
            {drawers}
            {palette}
            {link_hint}
        </section>
    }
    .into_any()
}

fn map_drawers(projection: MapProjection, drawer: RwSignal<Option<MapDrawer>>) -> AnyView {
    view! {
        {move || match drawer.get() {
            Some(MapDrawer::Signals) => signals_drawer(&projection, drawer).into_any(),
            Some(MapDrawer::List) => list_drawer(&projection, drawer).into_any(),
            Some(MapDrawer::About) => about_drawer(&projection, drawer).into_any(),
            Some(MapDrawer::Ports) => ports_drawer(&projection, drawer).into_any(),
            None => ().into_any(),
        }}
    }
    .into_any()
}

fn map_inspector(
    app: App,
    projection: MapProjection,
    selected: RwSignal<Option<MapSelection>>,
    viewport: RwSignal<MapViewport>,
    focus_title: RwSignal<bool>,
    linking: RwSignal<Option<MapLinkSource>>,
) -> AnyView {
    view! {
        {move || match selected.get() {
            Some(MapSelection::Node(id)) => {
                node_inspector(
                    app,
                    &projection,
                    &id,
                    selected,
                    viewport,
                    focus_title,
                    linking,
                )
            }
            Some(MapSelection::Edge(id)) => edge_inspector(app, &projection, &id, selected),
            None => ().into_any(),
        }}
    }
    .into_any()
}

fn node_inspector(
    app: App,
    projection: &MapProjection,
    node_id: &str,
    selected: RwSignal<Option<MapSelection>>,
    viewport: RwSignal<MapViewport>,
    focus_title: RwSignal<bool>,
    linking: RwSignal<Option<MapLinkSource>>,
) -> AnyView {
    let Some(node) = projection.node(node_id) else {
        return ().into_any();
    };
    let node = node.clone();
    let level = projection.query.level;
    let has_anchor = repo()
        .map_data()
        .anchors
        .iter()
        .any(|anchor| anchor.object_id == node.object_id && anchor.level == level);
    let kind = node.object_kind.label();
    let original_title = node.label.clone();
    let name_value = RwSignal::new(node.label.clone());
    let name_ref: NodeRef<Input> = NodeRef::new();
    let sublabel = node.sublabel.clone().unwrap_or_default();
    let status = node.status.clone().map(|value| status_label(&value));
    let badges = node.badges.join(" · ");
    let route = node.route.clone();
    let object_id = node.object_id;
    let node_kind = node.object_kind;
    let node_x = node.x;
    let node_y = node.y;
    let node_w = node.w;
    let node_h = node.h;
    let link_node_id = node.id.clone();

    let focus = focus_title;
    Effect::new(move |_| {
        if focus.get() {
            if let Some(input) = name_ref.get() {
                let _ = input.focus();
                let _ = input.select();
            }
            focus.set(false);
        }
    });

    let close = move |_| selected.set(None);
    let recenter = move |_| {
        viewport.update(|viewport| {
            *viewport = viewport.centered_on(node_x + node_w / 2.0, node_y + node_h / 2.0);
        });
    };
    let clear_anchor = move |_| commands::clear_anchor(app, object_id, level);
    let link = move |_| {
        linking.set(Some(MapLinkSource {
            node_id: link_node_id.clone(),
            object_id,
            kind: node_kind,
            level,
        }));
    };
    let change_title = original_title.clone();
    let key_title = original_title.clone();

    view! {
        <aside class="map-inspector" aria-label="Objet sélectionné">
            <header class="map-inspector-head">
                <span class="map-inspector-kind">{kind}</span>
                <button
                    class="icon-btn"
                    type="button"
                    aria-label="Fermer le panneau"
                    on:click=close
                >
                    "×"
                </button>
            </header>
            <div class="map-inspector-body">
                <input
                    class="map-inspector-title"
                    node_ref=name_ref
                    aria-label="Nom de l'objet"
                    prop:value=move || name_value.get()
                    on:input=move |ev| name_value.set(input_value(&ev))
                    on:change=move |_| {
                        let value = name_value.get();
                        if value.trim() != change_title {
                            commands::rename_object(app, object_id, &value);
                        }
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            let value = name_value.get();
                            if value.trim() != key_title {
                                commands::rename_object(app, object_id, &value);
                            }
                        }
                    }
                />
                {(!sublabel.is_empty()).then(|| view! {
                    <p class="muted small">{sublabel}</p>
                })}
                <div class="map-inspector-tags">
                    {status.map(|status| view! { <span class="badge">{status}</span> })}
                    {(!badges.is_empty()).then(|| view! {
                        <span class="badge ghost">{badges}</span>
                    })}
                    <span class="badge ghost">
                        {if has_anchor { "Ancre manuelle" } else { "Placement auto" }}
                    </span>
                </div>
                <p class="muted small">
                    "Renommez ici, glissez la forme pour la déplacer. La position est enregistrée au relâchement."
                </p>
                <div class="map-inspector-actions">
                    <a class="btn small" href=route>"Ouvrir la fiche"</a>
                    <button class="btn small" type="button" on:click=link>
                        "Relier"
                    </button>
                    <button class="btn small" type="button" on:click=recenter>
                        "Recentrer"
                    </button>
                    {has_anchor.then(|| view! {
                        <button class="btn ghost small" type="button" on:click=clear_anchor>
                            "Retirer l'ancre"
                        </button>
                    })}
                </div>
            </div>
        </aside>
    }
    .into_any()
}

fn edge_inspector(
    app: App,
    projection: &MapProjection,
    edge_id: &str,
    selected: RwSignal<Option<MapSelection>>,
) -> AnyView {
    let Some(edge) = projection.edges.iter().find(|edge| edge.id == edge_id) else {
        return ().into_any();
    };
    let edge = edge.clone();
    let data = repo().map_data();
    let source = projection
        .node(&edge.source_node_id)
        .map(|node| node.label.clone())
        .unwrap_or_else(|| edge.source_node_id.clone());
    let target = projection
        .node(&edge.target_node_id)
        .map(|node| node.label.clone())
        .unwrap_or_else(|| edge.target_node_id.clone());
    let interfaces: Vec<MapInterface> = edge
        .interface_ids
        .iter()
        .filter_map(|id| data.interface(*id).cloned())
        .collect();
    let single_interface = if interfaces.len() == 1 {
        interfaces.first().cloned()
    } else {
        None
    };
    let has_single_interface = single_interface.is_some();
    let first_interface_href = edge
        .interface_ids
        .first()
        .map(|id| format!("#/interfaces/{id}"));
    let panel_title = if edge.aggregated {
        "Route agrégée"
    } else {
        "Interface"
    };
    let interface_count = plural(edge.count, "interface", "interfaces");
    let version_count = plural(edge.interface_version_ids.len(), "version", "versions");
    let status = temporal_label(edge.temporal_status.as_deref());
    let close = move |_| selected.set(None);

    let single_editor = single_interface.map(|interface| {
        let interface_id = interface.id;
        let name = interface.name.clone();
        let value = RwSignal::new(name.clone());
        let change_name = name.clone();
        let key_name = name.clone();
        let version = data.versions_of(interface_id).first().cloned();
        let version_label = version
            .as_ref()
            .map(|version| version.semantic_version.clone())
            .unwrap_or_else(|| "sans version".into());
        let publication = version
            .as_ref()
            .map(|version| version.publication_status.label())
            .unwrap_or("Sans version");
        let draft = version
            .as_ref()
            .map(|version| version.publication_status == PublicationState::Draft)
            .unwrap_or(false);
        view! {
            <label class="map-field">
                <span>"Nom de l'interface"</span>
                <input
                    prop:value=move || value.get()
                    aria-label="Nom de l'interface"
                    on:input=move |ev| value.set(input_value(&ev))
                    on:change=move |_| {
                        let next = value.get();
                        if next.trim() != change_name {
                            commands::rename_object(app, interface_id, &next);
                        }
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            let next = value.get();
                            if next.trim() != key_name {
                                commands::rename_object(app, interface_id, &next);
                            }
                        }
                    }
                />
            </label>
            <div class="map-inspector-tags">
                <span class="badge">{version_label}</span>
                <span class=if draft { "badge warn" } else { "badge" }>{publication}</span>
            </div>
            {draft.then(|| view! {
                <p class="draft-note">
                    "Version brouillon : complétez le contrat, l'exposition et les consommateurs dans la fiche."
                </p>
            })}
        }
        .into_any()
    });

    view! {
        <aside class="map-inspector" aria-label=panel_title>
            <header class="map-inspector-head">
                <span class="map-inspector-kind">"Route"</span>
                <button
                    class="icon-btn"
                    type="button"
                    aria-label="Fermer le panneau"
                    on:click=close
                >
                    "×"
                </button>
            </header>
            <div class="map-inspector-body">
                <h2>{panel_title}</h2>
                <p class="map-inspector-route">
                    <strong>{source}</strong>
                    " → "
                    <strong>{target}</strong>
                </p>
                <p class="muted small">
                    {format!("{interface_count} · {version_count} · état {status}")}
                </p>
                {single_editor}
                {if has_single_interface {
                    ().into_any()
                } else if interfaces.is_empty() {
                    view! {
                        <p class="muted small">
                            "Aucune interface déclarée sur cette route. C'est un signal, pas une erreur."
                        </p>
                    }.into_any()
                } else {
                    view! {
                        <ul class="map-inspector-list">
                            {interfaces.into_iter().map(|interface| {
                                let versions = data.versions_of(interface.id);
                                let version = versions.first();
                                view! {
                                    <li>
                                        <a href=format!("#/interfaces/{}", interface.id)>
                                            {interface.name.clone()}
                                        </a>
                                        <span class="muted small">
                                            {format!("{} · {}", interface.level.label(), interface.exposure.label())}
                                        </span>
                                        <span class="badge">
                                            {version
                                                .map(|version| version.semantic_version.clone())
                                                .unwrap_or_else(|| "sans version".into())}
                                        </span>
                                        {version.map(|version| view! {
                                            <span class="badge ghost">{version.publication_status.label()}</span>
                                        })}
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }}
                {first_interface_href.map(|href| view! {
                    <a class="btn small" href=href>"Ouvrir la première interface"</a>
                })}
            </div>
        </aside>
    }
    .into_any()
}

fn signals_drawer(projection: &MapProjection, drawer: RwSignal<Option<MapDrawer>>) -> AnyView {
    let (signals, completeness): (Vec<MapAnomaly>, Vec<MapAnomaly>) = projection
        .anomalies
        .iter()
        .cloned()
        .partition(|issue| !issue.is_completeness());
    let close = move |_| drawer.set(None);
    let title = if !signals.is_empty() {
        format!("Signaux ({})", signals.len())
    } else {
        format!("À compléter ({})", completeness.len())
    };
    let intro = if signals.is_empty() {
        "Une carte en construction : ces points ne sont pas des erreurs, ils indiquent ce qui reste à relier."
    } else {
        "Ce qui mérite une décision. Les manques de complétude sont séparés : ils décrivent une carte en construction, pas une erreur."
    };

    view! {
        <aside class="map-drawer" aria-label="Points d'attention de la carte">
            <header class="map-drawer-head">
                <h2>{title}</h2>
                <button class="icon-btn" type="button" aria-label="Fermer le panneau" on:click=close>"×"</button>
            </header>
            <div class="map-drawer-body">
                <p class="muted small">{intro}</p>
                {(!signals.is_empty()).then(|| view! {
                    <section class="map-drawer-section">
                        <h3>"À arbitrer"</h3>
                        <ul class="anomaly-list">
                            {signals.into_iter().map(|issue| {
                                let href = anomaly_href(&issue);
                                view! {
                                    <li class=if issue.blocking { "blocking" } else { "" }>
                                        <span>{issue.message.clone()}</span>
                                        <span class="map-code">{issue.code.clone()}</span>
                                        {href.map(|href| view! {
                                            <a class="small" href=href>"Voir l'objet"</a>
                                        })}
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    </section>
                })}
                {(!completeness.is_empty()).then(|| view! {
                    <section class="map-drawer-section">
                        <h3>"À compléter"</h3>
                        <ul class="anomaly-list">
                            {completeness.into_iter().map(|issue| {
                                let href = anomaly_href(&issue);
                                view! {
                                    <li>
                                        <span class="muted small">{issue.message.clone()}</span>
                                        <span class="map-code">{issue.code.clone()}</span>
                                        {href.map(|href| view! {
                                            <a class="small" href=href>"Voir l'objet"</a>
                                        })}
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    </section>
                })}
            </div>
        </aside>
    }
    .into_any()
}

fn list_drawer(projection: &MapProjection, drawer: RwSignal<Option<MapDrawer>>) -> AnyView {
    let nodes = projection.nodes.clone();
    let close = move |_| drawer.set(None);
    view! {
        <aside class="map-drawer" aria-label="Liste des objets de la carte">
            <header class="map-drawer-head">
                <h2>{format!("Objets de la carte ({})", nodes.len())}</h2>
                <button class="icon-btn" type="button" aria-label="Fermer le panneau" on:click=close>"×"</button>
            </header>
            <div class="map-drawer-body">
                <ul class="map-list">
                    {nodes.into_iter().map(|node| view! {
                        <li>
                            <a href=node.route.clone()>{node.label.clone()}</a>
                            <span class="muted small">
                                {format!("{} · {}", node.object_kind.label(), node.sublabel.clone().unwrap_or_default())}
                            </span>
                            {node.status.clone().map(|status| view! { <span class="badge">{status_label(&status)}</span> })}
                        </li>
                    }).collect_view()}
                </ul>
            </div>
        </aside>
    }
    .into_any()
}

fn ports_drawer(projection: &MapProjection, drawer: RwSignal<Option<MapDrawer>>) -> AnyView {
    let ports = projection.ports.clone();
    let close = move |_| drawer.set(None);
    view! {
        <aside class="map-drawer" aria-label="Ports exposés">
            <header class="map-drawer-head">
                <h2>{format!("Ports exposés ({})", ports.len())}</h2>
                <button class="icon-btn" type="button" aria-label="Fermer le panneau" on:click=close>"×"</button>
            </header>
            <div class="map-drawer-body">
                <ul class="map-list">
                    {ports.into_iter().map(|port| view! {
                        <li>
                            <a href=port.route.clone()>{port.name.clone()}</a>
                            <span class="muted small">
                                {format!("{} · {} · {}", port.level.label(), port.exposure.label(), plural(port.consumer_count, "consommateur", "consommateurs"))}
                            </span>
                            <span class="badge">{port.latest_version.clone().unwrap_or_else(|| "sans version".into())}</span>
                        </li>
                    }).collect_view()}
                </ul>
            </div>
        </aside>
    }
    .into_any()
}

fn about_drawer(projection: &MapProjection, drawer: RwSignal<Option<MapDrawer>>) -> AnyView {
    let scope = projection
        .breadcrumb
        .last()
        .map(|entry| entry.label.clone())
        .unwrap_or_else(|| "Entreprise".into());
    let counts = projection.counts.clone();
    let close = move |_| drawer.set(None);
    view! {
        <aside class="map-drawer" aria-label="À propos de cette carte">
            <header class="map-drawer-head">
                <h2>"À propos de cette carte"</h2>
                <button class="icon-btn" type="button" aria-label="Fermer le panneau" on:click=close>"×"</button>
            </header>
            <div class="map-drawer-body">
                <dl class="facts">
                    <div class="fact"><dt>"Périmètre"</dt><dd>{scope}</dd></div>
                    <div class="fact"><dt>"Niveau"</dt><dd>{projection.query.level.label()}</dd></div>
                    <div class="fact"><dt>"Lecture"</dt><dd>{lens_label(projection.query.primary_lens())}</dd></div>
                    <div class="fact"><dt>"Date"</dt><dd>{date_label(projection.query.date.as_deref())}</dd></div>
                    <div class="fact"><dt>"Quartiers"</dt><dd>{counts.districts}</dd></div>
                    <div class="fact"><dt>"Objets"</dt><dd>{counts.nodes}</dd></div>
                    <div class="fact"><dt>"Routes"</dt><dd>{counts.edges}</dd></div>
                    <div class="fact"><dt>"Dont agrégées"</dt><dd>{counts.aggregated_edges}</dd></div>
                    <div class="fact"><dt>"Révision"</dt><dd>{projection.source_revision}</dd></div>
                </dl>
                <p class="muted small">
                    "Un lien profond restaure le périmètre, le niveau, la lentille et la date. La géographie reste stable : changer de lecture ne déplace aucun objet."
                </p>
            </div>
        </aside>
    }
    .into_any()
}

fn onboarding(
    app: App,
    open: RwSignal<bool>,
    kind: RwSignal<CreateKind>,
    data: &MapDataset,
    repo_empty: bool,
) -> AnyView {
    let has_team = !data.teams.is_empty();
    let has_product = !data.products.is_empty();
    let has_value_stream = !data.value_streams.is_empty();
    let team_count = data.teams.len();
    let product_count = data.products.len();
    let team_name = data
        .teams
        .first()
        .map(|team| team.name.clone())
        .unwrap_or_default();
    let product_name = data
        .products
        .first()
        .map(|product| product.name.clone())
        .unwrap_or_default();
    let stream_name = data
        .value_streams
        .first()
        .map(|stream| stream.name.clone())
        .unwrap_or_default();
    let team_href = data
        .teams
        .first()
        .map(|team| Route::Team { id: team.id }.to_hash());
    let product_href = data
        .products
        .first()
        .map(|product| Route::Product { id: product.id }.to_hash());
    let stream_href = data
        .value_streams
        .first()
        .map(|stream| Route::ValueStream { id: stream.id }.to_hash());
    let team_value = if team_count > 1 {
        format!("{team_name} + {}", team_count - 1)
    } else {
        team_name.clone()
    };
    let product_value = if product_count > 1 {
        format!("{product_name} + {}", product_count - 1)
    } else {
        product_name.clone()
    };

    let title = if has_product {
        "Votre carte est prête."
    } else {
        "Aucun produit à cartographier."
    };
    let lede = if has_product {
        format!(
            "{} sur la carte. Ouvrez-en un pour voir ses applications, ses interfaces et ses dépendances.",
            plural(product_count, "produit", "produits")
        )
    } else if has_team {
        format!(
            "L'équipe « {team_name} » est prête. Ajoutez un produit pour le voir apparaître sur la carte, ou repassez par le portefeuille."
        )
    } else {
        "Le portefeuille de produits est l'accueil. Créez un produit pour commencer, puis ouvrez sa carte.".to_string()
    };

    let create = move |value: CreateKind| {
        kind.set(value);
        open.set(true);
    };

    view! {
        <section class="map-onboarding" aria-label="Première carte">
            <div class="map-onboarding-main">
                <h1>{title}</h1>
                <p class="map-onboarding-lede">{lede}</p>

                <ol class="map-steps">
                    <li class=if has_product { "map-step done" } else { "map-step current" }>
                        <span class="map-step-marker" aria-hidden="true">
                            {if has_product { "✓" } else { "1" }}
                        </span>
                        <div class="map-step-body">
                            <strong>"Un produit"</strong>
                            <span>"Ce qui est offert, et à qui. Le bâtiment de la carte."</span>
                        </div>
                        {if has_product {
                            view! {
                                {product_href.map(|href| view! {
                                    <a class="map-step-value" href=href>{product_value.clone()}</a>
                                })}
                            }.into_any()
                        } else {
                            view! {
                                <button class="btn primary" type="button" on:click=move |_| create(CreateKind::Product)>
                                    "Créer un produit"
                                </button>
                            }.into_any()
                        }}
                    </li>

                    <li class=if has_team { "map-step done" } else { "map-step" }>
                        <span class="map-step-marker" aria-hidden="true">
                            {if has_team { "✓" } else { "2" }}
                        </span>
                        <div class="map-step-body">
                            <strong>"Une équipe"</strong>
                            <span>"Qui est responsable du produit ?"</span>
                        </div>
                        {if has_team {
                            view! {
                                {team_href.map(|href| view! {
                                    <a class="map-step-value" href=href>{team_value.clone()}</a>
                                })}
                            }.into_any()
                        } else {
                            view! {
                                <button class="btn" type="button" on:click=move |_| create(CreateKind::Team)>
                                    "Créer une équipe"
                                </button>
                            }.into_any()
                        }}
                    </li>

                    <li class=if has_value_stream { "map-step done" } else { "map-step" }>
                        <span class="map-step-marker" aria-hidden="true">
                            {if has_value_stream { "✓" } else { "3" }}
                        </span>
                        <div class="map-step-body">
                            <strong>"Un value stream"</strong>
                            <span>"Le parcours qui porte le produit. Facultatif."</span>
                        </div>
                        {if has_value_stream {
                            view! {
                                {stream_href.map(|href| view! {
                                    <a class="map-step-value" href=href>{stream_name.clone()}</a>
                                })}
                            }.into_any()
                        } else {
                            view! {
                                <button class="btn ghost" type="button" on:click=move |_| create(CreateKind::ValueStream)>
                                    "Créer un value stream"
                                </button>
                            }.into_any()
                        }}
                    </li>
                </ol>

                <div class="map-onboarding-foot">
                    <a class="btn ghost small" href="#/">"Ouvrir le portefeuille de produits"</a>
                </div>

                {repo_empty.then(|| view! {
                    <div class="map-onboarding-foot">
                        <span>"Voir une carte avant la vôtre :"</span>
                        <button
                            class="btn ghost small"
                            type="button"
                            on:click=move |_| {
                                match app.run(repo().load_demo_map()) {
                                    Some(true) => app.notify("Référentiel d'exemple chargé : explorez la carte, puis remplacez-le par vos objets."),
                                    Some(false) => app.notify("La carte contient déjà des objets : l'exemple n'a pas été chargé."),
                                    None => {}
                                }
                            }
                        >
                            "Explorer le référentiel d'exemple"
                        </button>
                    </div>
                })}
            </div>

            <aside class="map-teaching" aria-label="Comment se lit une carte">
                <h2>"Comment se lit une carte ?"</h2>
                <div class="teaching-map" aria-hidden="true">
                    <div class="teaching-district">
                        <span class="teaching-district-name">"value stream"</span>
                        <div class="teaching-nodes">
                            <span class="teaching-node"></span>
                            <span class="teaching-node"></span>
                        </div>
                    </div>
                    <div class="teaching-platform"></div>
                    <span class="teaching-route"></span>
                </div>
                <ul class="teaching-legend">
                    <li><strong>"Quartier"</strong><span>"Un value stream ou un portefeuille."</span></li>
                    <li><strong>"Bâtiment"</strong><span>"Un produit, avec son équipe et son cycle de vie."</span></li>
                    <li><strong>"Route"</strong><span>"Une interface entre deux produits."</span></li>
                </ul>
            </aside>
        </section>
    }
    .into_any()
}

fn scope_empty_state(projection: &MapProjection) -> AnyView {
    let level = projection.query.level.label();
    let scope = projection
        .breadcrumb
        .last()
        .map(|entry| entry.label.clone())
        .unwrap_or_else(|| "ce périmètre".into());
    view! {
        <section class="map-onboarding map-emptyscope" aria-label="Périmètre vide">
            <div class="map-onboarding-main">
                <h1>"Aucun objet à ce niveau."</h1>
                <p class="map-onboarding-lede">
                    {format!("« {scope} » ne contient pas d'objet au niveau {level}. Revenez à la carte de l'entreprise ou ouvrez un objet compatible.")}
                </p>
                <div class="actions">
                    <a class="btn primary" href="#/map">"Revenir à la carte de l'entreprise"</a>
                </div>
            </div>
        </section>
    }
    .into_any()
}

fn map_canvas(
    app: App,
    projection: MapProjection,
    selected: RwSignal<Option<MapSelection>>,
    gesture: RwSignal<Option<MapGesture>>,
    viewport: RwSignal<MapViewport>,
    content: MapViewport,
    drawer: RwSignal<Option<MapDrawer>>,
    linking: RwSignal<Option<MapLinkSource>>,
    focus_title: RwSignal<bool>,
) -> AnyView {
    let level = projection.query.level;
    let query = projection.query.clone();
    let districts = projection.districts.clone();
    let edges = projection.edges.clone();
    let nodes = projection.nodes.clone();

    let on_wheel = move |ev: web_sys::WheelEvent| {
        if ev.ctrl_key() || ev.meta_key() {
            return;
        }
        let Some(svg) = svg_from_target(ev.current_target()) else {
            return;
        };
        ev.prevent_default();
        let rect = svg.get_bounding_client_rect();
        let current = viewport.get_untracked();
        let (rel_x, rel_y) = current.relative_in(
            rect.width(),
            rect.height(),
            ev.client_x() as f64,
            ev.client_y() as f64,
            rect.left(),
            rect.top(),
        );
        let factor = if ev.delta_y() > 0.0 { 0.9 } else { 1.1 };
        viewport.set(current.zoomed(factor, rel_x, rel_y));
    };

    let on_background_down = move |ev: web_sys::PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        selected.set(None);
        linking.set(None);
        app.map_palette.set(false);
        let Some(svg) = svg_from_target(ev.current_target()) else {
            return;
        };
        let rect = svg.get_bounding_client_rect();
        let current = viewport.get_untracked();
        let scale = current.scale_in(rect.width(), rect.height());
        let _ = svg.set_pointer_capture(ev.pointer_id());
        gesture.set(Some(MapGesture::Pan(MapPan {
            start_client_x: ev.client_x() as f64,
            start_client_y: ev.client_y() as f64,
            origin_x: current.x,
            origin_y: current.y,
            scale,
        })));
    };

    let on_move = move |ev: web_sys::PointerEvent| {
        let Some(current) = gesture.get_untracked() else {
            return;
        };
        match current {
            MapGesture::Pan(pan) => {
                let dx = (ev.client_x() as f64 - pan.start_client_x) / pan.scale;
                let dy = (ev.client_y() as f64 - pan.start_client_y) / pan.scale;
                viewport.update(|viewport| {
                    let start = MapViewport {
                        x: pan.origin_x,
                        y: pan.origin_y,
                        ..*viewport
                    };
                    *viewport = start.panned(dx, dy);
                });
            }
            MapGesture::Node(mut drag) => {
                let dx = (ev.client_x() as f64 - drag.start_client_x) / drag.scale;
                let dy = (ev.client_y() as f64 - drag.start_client_y) / drag.scale;
                if dx.abs() + dy.abs() > 1.0 {
                    drag.moved = true;
                }
                drag.x = drag.origin_x + dx;
                drag.y = drag.origin_y + dy;
                gesture.set(Some(MapGesture::Node(drag)));
            }
        }
    };

    let on_up = move |ev: web_sys::PointerEvent| {
        let Some(current) = gesture.get_untracked() else {
            return;
        };
        gesture.set(None);
        if let Some(svg) = svg_from_target(ev.current_target()) {
            let _ = svg.release_pointer_capture(ev.pointer_id());
        }
        if let MapGesture::Node(drag) = current {
            if drag.moved {
                commands::move_anchor(app, drag.object_id, drag.level, drag.x, drag.y);
            }
        }
    };

    let on_cancel = move |_ev: web_sys::PointerEvent| {
        gesture.set(None);
    };

    let on_background_double_click = move |ev: web_sys::MouseEvent| {
        let Some(svg) = svg_from_target(ev.current_target()) else {
            return;
        };
        let Some(kind) = default_create_kind(level, &repo().map_data()) else {
            app.notify("Ce niveau se construit depuis le panneau Créer.");
            return;
        };
        let rect = svg.get_bounding_client_rect();
        let current = viewport.get_untracked();
        let (world_x, world_y) = current.world_at(
            rect.width(),
            rect.height(),
            ev.client_x() as f64,
            ev.client_y() as f64,
            rect.left(),
            rect.top(),
        );
        let x = world_x - 79.0;
        let y = world_y - 32.0;
        let Some(object_id) = commands::create_at(app, kind, &query, x, y) else {
            return;
        };
        app.map_palette.set(false);
        linking.set(None);
        let projection = repo().map_projection(&query);
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
    };

    let on_key = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" {
            selected.set(None);
            gesture.set(None);
        }
    };

    let zoom_in = move |_| viewport.update(|viewport| *viewport = viewport.zoomed_centered(1.2));
    let zoom_out = move |_| viewport.update(|viewport| *viewport = viewport.zoomed_centered(0.8));
    let fit = move |_| viewport.set(content);

    view! {
        <div class="map-canvas">
            <svg
                class="map-svg"
                role="group"
                aria-label="Carte de l'entreprise : quartiers, produits et routes"
                preserveAspectRatio="xMidYMid meet"
                viewBox=move || viewport.get().view_box()
                tabindex="0"
                on:wheel=on_wheel
                on:pointerdown=on_background_down
                on:pointermove=on_move
                on:pointerup=on_up
                on:pointercancel=on_cancel
                on:dblclick=on_background_double_click
                on:keydown=on_key
            >
                <defs>
                    <marker id="map-arrow" viewBox="0 0 10 10" refX="9" refY="5"
                        markerWidth="7" markerHeight="7" orient="auto-start-reverse">
                        <path d="M 0 0 L 10 5 L 0 10 z" class="map-arrow-head"></path>
                    </marker>
                </defs>

                {districts.into_iter().map(|district| {
                    let route = district_route(&district, level);
                    let classes = format!(
                        "map-district d-{}{}",
                        district.color_index,
                        if route.is_some() { " clickable" } else { "" }
                    );
                    if let Some(route) = route {
                        let label = format!(
                            "{} — {} — ouvrir le quartier",
                            district.label,
                            district.node_ids.len()
                        );
                        let click_route = route.clone();
                        let key_route = route;
                        view! {
                            <g
                                class=classes
                                role="link"
                                tabindex="0"
                                aria-label=label
                                on:pointerdown=move |ev: web_sys::PointerEvent| ev.stop_propagation()
                                on:dblclick=move |ev: web_sys::MouseEvent| ev.stop_propagation()
                                on:click=move |_| navigate_to(&click_route)
                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                    if ev.key() == "Enter" || ev.key() == " " {
                                        ev.prevent_default();
                                        navigate_to(&key_route);
                                    }
                                }
                            >
                                {district_shape(&district)}
                            </g>
                        }.into_any()
                    } else {
                        view! {
                            <g class=classes>
                                {district_shape(&district)}
                            </g>
                        }.into_any()
                    }
                }).collect_view()}

                {move || {
                    let dragged = match gesture.get() {
                        Some(MapGesture::Node(drag)) => Some(drag),
                        _ => None,
                    };
                    let mut current_nodes = nodes.clone();
                    if let Some(drag) = dragged.as_ref() {
                        if let Some(node) = current_nodes.iter_mut().find(|node| node.id == drag.node_id) {
                            node.x = drag.x;
                            node.y = drag.y;
                        }
                    }
                    view! {
                        {edges.iter().cloned().map(|edge| {
                            map_edge_view(&current_nodes, edge, selected, drawer)
                        }).collect_view()}
                        {current_nodes.into_iter().map(|node| {
                            map_node_view(app, node, level, selected, gesture, viewport, drawer, linking)
                        }).collect_view()}
                    }
                    .into_any()
                }}
            </svg>

            <div class="map-zoom" role="group" aria-label="Zoom de la carte">
                <button type="button" aria-label="Dézoomer" on:click=zoom_out>"−"</button>
                <span class="map-zoom-label">
                    {move || format!("{:.0} %", viewport.get().zoom_percent(content))}
                </span>
                <button type="button" aria-label="Zoomer" on:click=zoom_in>"+"</button>
                <span class="map-zoom-sep"></span>
                <button class="map-zoom-fit" type="button" on:click=fit>"Ajuster"</button>
            </div>

            <p class="map-gesture-hint" aria-hidden="true">
                "Glisser pour déplacer · Molette pour zoomer · Double-clic pour entrer"
            </p>
        </div>
    }
    .into_any()
}

fn map_node_view(
    app: App,
    node: MapNode,
    level: MapLevel,
    selected: RwSignal<Option<MapSelection>>,
    gesture: RwSignal<Option<MapGesture>>,
    viewport: RwSignal<MapViewport>,
    drawer: RwSignal<Option<MapDrawer>>,
    linking: RwSignal<Option<MapLinkSource>>,
) -> AnyView {
    let label = node.label.clone();
    let kind = node.object_kind.label().to_string();
    let badge = node.badges.join(" · ");
    let visible_label = truncate(&node.label, 22);
    let visible_sub = truncate(node.sublabel.as_deref().unwrap_or(""), 24);
    let visible_badge = truncate(&badge, 28);
    let aria = [
        visible_label.clone(),
        visible_sub.clone(),
        visible_badge.clone(),
        kind.clone(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" — ");

    let color_key = node.color_key.clone();
    let focused = node.focused;
    let terminal = !node.expandable;
    let selected_id = node.id.clone();
    let drag_id = node.id.clone();
    let aria_id = node.id.clone();
    let link_class_id = node.id.clone();
    let class = move || {
        let mut out = format!("map-node c-{}", color_key);
        if focused {
            out.push_str(" focused");
        }
        if terminal {
            out.push_str(" terminal");
        }
        if matches!(selected.get(), Some(MapSelection::Node(ref id)) if id == &selected_id) {
            out.push_str(" selected");
        }
        if matches!(gesture.get(), Some(MapGesture::Node(ref drag)) if drag.node_id == drag_id) {
            out.push_str(" dragging");
        }
        match linking.get() {
            Some(source) if source.node_id == link_class_id => out.push_str(" link-source"),
            Some(_) => out.push_str(" link-candidate"),
            None => {}
        }
        out
    };

    let object_id = node.object_id;
    let node_kind = node.object_kind;
    let origin_x = node.x;
    let origin_y = node.y;

    let link_at_down_id = node.id.clone();
    let on_down_id = node.id.clone();
    let on_down = move |ev: web_sys::PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        if let Some(source) = linking.get_untracked() {
            if source.node_id != link_at_down_id {
                ev.stop_propagation();
                ev.prevent_default();
                complete_link(app, source, object_id, node_kind, linking);
                return;
            }
            linking.set(None);
        }
        ev.stop_propagation();
        selected.set(Some(MapSelection::Node(on_down_id.clone())));
        drawer.set(None);
        let Some(svg) = svg_from_target(ev.current_target()) else {
            return;
        };
        let rect = svg.get_bounding_client_rect();
        let current = viewport.get_untracked();
        let scale = current.scale_in(rect.width(), rect.height());
        let _ = svg.set_pointer_capture(ev.pointer_id());
        gesture.set(Some(MapGesture::Node(MapNodeDrag {
            node_id: on_down_id.clone(),
            object_id,
            level,
            origin_x,
            origin_y,
            start_client_x: ev.client_x() as f64,
            start_client_y: ev.client_y() as f64,
            scale,
            x: origin_x,
            y: origin_y,
            moved: false,
        })));
    };

    let route = enter_node(&node, level);
    let on_double_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        navigate_to(&route);
    };

    let key_id = node.id.clone();
    let key_route = enter_node(&node, level);
    let on_key = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "Enter" => {
            ev.prevent_default();
            if let Some(source) = linking.get_untracked() {
                if source.node_id != key_id {
                    complete_link(app, source, object_id, node_kind, linking);
                    return;
                }
                linking.set(None);
            }
            navigate_to(&key_route);
        }
        " " => {
            ev.prevent_default();
            selected.set(Some(MapSelection::Node(key_id.clone())));
            drawer.set(None);
        }
        "Escape" => {
            selected.set(None);
            linking.set(None);
        }
        _ => {}
    };

    let handle_id = node.id.clone();
    let on_handle = move |ev: web_sys::PointerEvent| {
        ev.stop_propagation();
        ev.prevent_default();
        linking.set(Some(MapLinkSource {
            node_id: handle_id.clone(),
            object_id,
            kind: node_kind,
            level,
        }));
    };

    let data_id = node.id.clone();
    let handle_x = node.x + node.w;
    let handle_y = node.y + node.h / 2.0;

    view! {
        <g
            class=class
            role="link"
            tabindex="0"
            data-node-id=data_id
            aria-label=aria
            aria-current=move || {
                if matches!(selected.get(), Some(MapSelection::Node(ref id)) if id == &aria_id) {
                    "true"
                } else {
                    "false"
                }
            }
            on:pointerdown=on_down
            on:dblclick=on_double_click
            on:keydown=on_key
        >
            <rect x=node.x y=node.y width=node.w height=node.h rx="10"></rect>
            <text x=node.x + 12.0 y=node.y + 24.0 class="map-node-label">{visible_label}</text>
            <text x=node.x + 12.0 y=node.y + 42.0 class="map-node-sub">{visible_sub}</text>
            {(!visible_badge.is_empty()).then(|| view! {
                <text x=node.x + 12.0 y=node.y + 56.0 class="map-node-badge">{visible_badge}</text>
            })}
            <circle
                class="map-node-link-handle"
                cx=handle_x
                cy=handle_y
                r="6"
                on:pointerdown=on_handle
            ></circle>
            <title>{format!("{} — {}", label, kind)}</title>
        </g>
    }
    .into_any()
}

fn complete_link(
    app: App,
    source: MapLinkSource,
    target_object_id: ObjectId,
    target_kind: MapObjectKind,
    linking: RwSignal<Option<MapLinkSource>>,
) {
    let interface_level = match (source.kind, target_kind) {
        (MapObjectKind::Product, MapObjectKind::Product) => Some(InterfaceLevel::Business),
        (MapObjectKind::Application, MapObjectKind::Application) => {
            Some(InterfaceLevel::Application)
        }
        _ => None,
    };
    match interface_level {
        Some(level) => {
            commands::link_objects(app, source.object_id, target_object_id, level);
            linking.set(None);
        }
        None => {
            app.notify("Reliez deux produits ou deux applications pour créer une interface.");
        }
    }
}

fn default_create_kind(level: MapLevel, data: &MapDataset) -> Option<CreateKind> {
    let has_team = !data.teams.is_empty();
    match level {
        MapLevel::Enterprise | MapLevel::Portfolio => Some(if has_team {
            CreateKind::Product
        } else {
            CreateKind::Team
        }),
        MapLevel::Product => Some(if has_team {
            CreateKind::Application
        } else {
            CreateKind::Team
        }),
        MapLevel::Application => None,
        MapLevel::Infrastructure => Some(CreateKind::Node),
    }
}

fn map_edge_view(
    nodes: &[MapNode],
    edge: MapEdge,
    selected: RwSignal<Option<MapSelection>>,
    drawer: RwSignal<Option<MapDrawer>>,
) -> AnyView {
    let Some(source) = nodes.iter().find(|node| node.id == edge.source_node_id) else {
        return ().into_any();
    };
    let Some(target) = nodes.iter().find(|node| node.id == edge.target_node_id) else {
        return ().into_any();
    };

    let sx = source.x + source.w;
    let sy = source.y + source.h / 2.0;
    let tx = target.x;
    let ty = target.y + target.h / 2.0;
    let mx = (sx + tx) / 2.0;
    let path = format!("M {sx} {sy} C {mx} {sy}, {mx} {ty}, {tx} {ty}");

    let edge_id = edge.id.clone();
    let selected_id = edge.id.clone();
    let key_id = edge.id.clone();
    let aggregated = edge.aggregated;
    let temporal = edge.temporal_status.clone();
    let class = move || {
        let mut out = String::from("map-edge");
        if aggregated {
            out.push_str(" aggregated");
        }
        match temporal.as_deref() {
            Some("SCHEDULED") => out.push_str(" scheduled"),
            Some("EXPIRED") => out.push_str(" expired"),
            _ => {}
        }
        if matches!(selected.get(), Some(MapSelection::Edge(ref id)) if id == &selected_id) {
            out.push_str(" selected");
        }
        out
    };

    let title = edge.label.clone().unwrap_or_else(|| {
        format!(
            "{} {}",
            plural(edge.count, "interface", "interfaces"),
            if edge.aggregated { "agrégée" } else { "" }
        )
    });
    let aria = format!(
        "Ouvrir {} de {} vers {}",
        if edge.aggregated {
            "la route agrégée"
        } else {
            "l'interface"
        },
        source.label,
        target.label
    );

    let on_down = move |ev: web_sys::PointerEvent| {
        ev.stop_propagation();
        selected.set(Some(MapSelection::Edge(edge_id.clone())));
        drawer.set(None);
    };
    let on_key = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" || ev.key() == " " {
            ev.prevent_default();
            selected.set(Some(MapSelection::Edge(key_id.clone())));
            drawer.set(None);
        }
    };

    let data_id = edge.id.clone();

    view! {
        <g
            class=class
            role="button"
            tabindex="0"
            data-edge-id=data_id
            aria-label=aria
            on:pointerdown=on_down
            on:dblclick=move |ev: web_sys::MouseEvent| ev.stop_propagation()
            on:keydown=on_key
        >
            <path class="map-edge-hit" d=path.clone()></path>
            <path class="map-edge-line" d=path marker-end="url(#map-arrow)"></path>
            <title>{title}</title>
        </g>
    }
    .into_any()
}

fn svg_from_target(target: Option<web_sys::EventTarget>) -> Option<web_sys::Element> {
    let target = target?;
    let element: web_sys::Element = target.dyn_into().ok()?;
    element.closest("svg").ok().flatten()
}

fn anomaly_href(issue: &MapAnomaly) -> Option<String> {
    if issue.code.starts_with("MAP-") {
        None
    } else {
        issue.object_id.and_then(map_route_for)
    }
}

fn map_route_for(object_id: ObjectId) -> Option<String> {
    repo()
        .map_inventory()
        .into_iter()
        .find(|hit| hit.object_id == object_id)
        .map(|hit| hit.route)
}

fn navigate_to(route: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_hash(route.trim_start_matches('#'));
    }
}

/// Double-clic (ou Entrée) : on entre dans l'objet, on n'ouvre pas sa fiche.
/// Un produit mène à la carte du produit, une application à la carte de
/// l'application ; les autres objets gardent leur page.
fn enter_node(node: &MapNode, level: MapLevel) -> String {
    match (level, node.object_kind) {
        (MapLevel::Enterprise | MapLevel::Portfolio, MapObjectKind::Product) => Route::Map(
            MapQuery::default()
                .at_scope(MapScope::product(node.object_id))
                .level(MapLevel::Product),
        )
        .to_hash(),
        (MapLevel::Product, MapObjectKind::Application) => Route::Map(
            MapQuery::default()
                .at_scope(MapScope::application(node.object_id))
                .level(MapLevel::Application),
        )
        .to_hash(),
        _ => node.route.clone(),
    }
}

fn district_route(district: &MapDistrict, level: MapLevel) -> Option<String> {
    match (level, district.kind) {
        (MapLevel::Enterprise, DistrictKind::ValueStream) => district.object_id.map(|id| {
            Route::Map(
                MapQuery::default()
                    .at_scope(MapScope::value_stream(id))
                    .level(MapLevel::Portfolio),
            )
            .to_hash()
        }),
        (MapLevel::Portfolio, DistrictKind::ValueStream) => district
            .object_id
            .map(|id| Route::ValueStream { id }.to_hash()),
        (MapLevel::Product, DistrictKind::ProductGroup) => {
            district.object_id.map(|id| Route::Product { id }.to_hash())
        }
        (MapLevel::Application, DistrictKind::ProductGroup) => district
            .object_id
            .map(|id| Route::Application { id }.to_hash()),
        _ => None,
    }
}

fn district_shape(district: &MapDistrict) -> AnyView {
    view! {
        <rect x=district.x y=district.y width=district.w height=district.h rx="14"></rect>
        <text x=district.x + 20.0 y=district.y + 24.0 class="map-district-label">{district.label.clone()}</text>
        <text x=district.x + district.w - 20.0 y=district.y + 24.0 class="map-district-count" text-anchor="end">
            {format!("{}", district.node_ids.len())}
        </text>
    }
    .into_any()
}

fn lens_label(lens: MapLens) -> &'static str {
    match lens {
        MapLens::Value => "Parcours de valeur",
        MapLens::Capability => "Capacité",
        MapLens::Product => "Cycle de vie",
        MapLens::Organization => "Équipe",
        MapLens::Application => "Applicatif",
        MapLens::Interface => "Interface",
        MapLens::Technology => "Technologie",
        MapLens::Data => "Données",
        MapLens::Time => "Temps",
        MapLens::Risk => "Risque",
    }
}

fn date_label(date: Option<&str>) -> String {
    match date {
        Some(value) => {
            let short: String = value.chars().take(10).collect();
            format!("Au {}", short_date(&short))
        }
        None => "Aujourd'hui".into(),
    }
}

fn short_date(value: &str) -> String {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() == 3 {
        format!("{}/{}/{}", parts[2], parts[1], parts[0])
    } else {
        value.to_string()
    }
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count <= 1 { one } else { many })
}

fn status_label(status: &str) -> String {
    if let Some(lifecycle) = LifecycleState::parse(status) {
        lifecycle.label().to_string()
    } else {
        temporal_label(Some(status))
    }
}

fn temporal_label(status: Option<&str>) -> String {
    match status {
        Some("SCHEDULED") => "Planifiée",
        Some("EXPIRED") => "Expirée",
        Some("ACTIVE") | None => "Active",
        Some(other) => other,
    }
    .to_string()
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
