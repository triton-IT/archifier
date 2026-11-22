//! `/` — Portefeuille de produits (ADR-025).
//!
//! L'accueil est une lecture filtrable du référentiel, pas la carte. Chaque
//! carte produit résume les relations utiles (équipe, value stream, capacités,
//! applications, interfaces) et ouvre soit la fiche produit, soit sa carte L2.

use crate::map::create::{CreateKind, CreateTray};
use crate::router::Route;
use crate::state::{input_value, parse_object_id, repo, App};
use archifier_core::catalog::*;
use archifier_core::map::*;
use leptos::prelude::*;

#[component]
pub fn ProductCatalogPage(app: App, query: ProductQuery) -> impl IntoView {
    let open = RwSignal::new(false);
    let kind = RwSignal::new(CreateKind::Product);
    let data = repo().map_data();
    let catalog = product_catalog(&data, &query);

    let teams: Vec<(String, String)> = data
        .teams
        .iter()
        .map(|team| (team.id.to_string(), team.name.clone()))
        .collect();
    let value_streams: Vec<(String, String)> = data
        .value_streams
        .iter()
        .map(|stream| (stream.id.to_string(), stream.name.clone()))
        .collect();
    let capabilities: Vec<(String, String)> = data
        .capabilities
        .iter()
        .map(|capability| (capability.id.to_string(), capability.name.clone()))
        .collect();
    let applications: Vec<(String, String)> = data
        .applications
        .iter()
        .map(|application| (application.id.to_string(), application.name.clone()))
        .collect();

    let total = catalog.total;
    let filtered = catalog.filtered;
    let active = query.active_filter_count();

    let query_search = query.clone();
    let query_team = query.clone();
    let query_kind = query.clone();
    let query_lifecycle = query.clone();
    let query_criticality = query.clone();
    let query_stream = query.clone();
    let query_capability = query.clone();
    let query_application = query.clone();
    let query_level = query.clone();
    let query_exposure = query.clone();
    let query_gap = query.clone();
    let query_sort = query.clone();

    let body = if data.products.is_empty() {
        catalog_onboarding(app, open)
    } else if catalog.items.is_empty() {
        view! {
            <section class="catalog-empty" aria-label="Aucun produit">
                <h2>"Aucun produit ne correspond à ces filtres."</h2>
                <p class="muted">"Élargissez un critère ou repartez du portefeuille complet."</p>
                <a class="btn primary" href="#/">"Réinitialiser les filtres"</a>
            </section>
        }
        .into_any()
    } else {
        view! {
            <section class="catalog-grid" aria-label="Produits">
                {catalog.items.into_iter().map(product_card).collect_view()}
            </section>
        }
        .into_any()
    };

    view! {
        <main class="page catalog-page">
            <header class="catalog-head">
                <div class="catalog-head-main">
                    <p class="catalog-eyebrow">"Portefeuille de produits"</p>
                    <h1>"Produits"</h1>
                    <p class="muted catalog-count">
                        {format!(
                            "{} · {} affichés",
                            plural(total, "produit", "produits"),
                            filtered
                        )}
                    </p>
                </div>
                <div class="catalog-actions">
                    <a class="btn ghost" href="#/map">"Carte de l'entreprise"</a>
                    <button class="btn primary" type="button" on:click=move |_| open.set(true)>
                        "＋ Créer un produit"
                    </button>
                </div>
            </header>

            <section class="catalog-filters" aria-label="Filtres produits">
                <label class="catalog-search">
                    <span class="sr-only">"Rechercher un produit"</span>
                    <input
                        type="search"
                        placeholder="Rechercher un produit, une équipe, une capacité, une application…"
                        prop:value=query.search.clone()
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_search.clone();
                            next.search = value;
                            app.navigate(Route::Home(next));
                        }
                    />
                </label>

                <label class="catalog-filter">
                    <span>"Équipe"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_team.clone();
                            next.team_ids = parse_object_id(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.team_ids.is_empty()>"Toutes les équipes"</option>
                        {teams.into_iter().map(|(id, label)| {
                            let selected = query.team_ids.first().map(|value| value.to_string()).as_deref() == Some(id.as_str());
                            view! { <option value=id selected=selected>{label}</option> }
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"Type"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_kind.clone();
                            next.kinds = ProductKind::parse(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.kinds.is_empty()>"Tous les types"</option>
                        {ProductKind::ALL.iter().map(|value| view! {
                            <option value=(*value).as_str() selected=query.kinds.contains(value)>
                                {(*value).label()}
                            </option>
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"Cycle de vie"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_lifecycle.clone();
                            next.lifecycles = LifecycleState::parse(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.lifecycles.is_empty()>"Tous les cycles"</option>
                        {LifecycleState::ALL.iter().map(|value| view! {
                            <option value=(*value).as_str() selected=query.lifecycles.contains(value)>
                                {(*value).label()}
                            </option>
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"Criticité"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_criticality.clone();
                            next.criticalities = Criticality::parse(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.criticalities.is_empty()>"Toutes"</option>
                        {Criticality::ALL.iter().map(|value| view! {
                            <option value=(*value).as_str() selected=query.criticalities.contains(value)>
                                {(*value).label()}
                            </option>
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"Value stream"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_stream.clone();
                            next.value_stream_ids = parse_object_id(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.value_stream_ids.is_empty()>"Tous les value streams"</option>
                        {value_streams.into_iter().map(|(id, label)| {
                            let selected = query.value_stream_ids.first().map(|value| value.to_string()).as_deref() == Some(id.as_str());
                            view! { <option value=id selected=selected>{label}</option> }
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"Capacité"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_capability.clone();
                            next.capability_ids = parse_object_id(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.capability_ids.is_empty()>"Toutes les capacités"</option>
                        {capabilities.into_iter().map(|(id, label)| {
                            let selected = query.capability_ids.first().map(|value| value.to_string()).as_deref() == Some(id.as_str());
                            view! { <option value=id selected=selected>{label}</option> }
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"Application"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_application.clone();
                            next.application_ids = parse_object_id(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.application_ids.is_empty()>"Toutes les applications"</option>
                        {applications.into_iter().map(|(id, label)| {
                            let selected = query.application_ids.first().map(|value| value.to_string()).as_deref() == Some(id.as_str());
                            view! { <option value=id selected=selected>{label}</option> }
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"Niveau d'interface"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_level.clone();
                            next.interface_levels = InterfaceLevel::parse(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.interface_levels.is_empty()>"Tous les niveaux"</option>
                        {InterfaceLevel::ALL.iter().map(|value| view! {
                            <option value=(*value).as_str() selected=query.interface_levels.contains(value)>
                                {(*value).label()}
                            </option>
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"Exposition"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_exposure.clone();
                            next.exposures = InterfaceExposure::parse(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.exposures.is_empty()>"Toutes les expositions"</option>
                        {InterfaceExposure::ALL.iter().map(|value| view! {
                            <option value=(*value).as_str() selected=query.exposures.contains(value)>
                                {(*value).label()}
                            </option>
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"À compléter"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_gap.clone();
                            next.gaps = ProductGap::parse(&value).into_iter().collect();
                            app.navigate(Route::Home(next));
                        }
                    >
                        <option value="" selected=query.gaps.is_empty()>"Aucun filtre de complétude"</option>
                        {ProductGap::ALL.iter().map(|value| view! {
                            <option value=(*value).as_str() selected=query.gaps.contains(value)>
                                {(*value).label()}
                            </option>
                        }).collect_view()}
                    </select>
                </label>

                <label class="catalog-filter">
                    <span>"Trier par"</span>
                    <select
                        on:change=move |ev| {
                            let value = input_value(&ev);
                            let mut next = query_sort.clone();
                            next.sort = ProductSort::parse(&value).unwrap_or_default();
                            app.navigate(Route::Home(next));
                        }
                    >
                        {ProductSort::ALL.iter().map(|value| view! {
                            <option value=(*value).as_str() selected=query.sort == *value>
                                {(*value).label()}
                            </option>
                        }).collect_view()}
                    </select>
                </label>
            </section>

            <div class="catalog-summary">
                <span>{if active == 0 {
                    "Tous les produits du référentiel".to_string()
                } else {
                    format!("{} filtre(s) actif(s)", active)
                }}</span>
                {(!query.is_default()).then(|| view! {
                    <a class="btn ghost small" href="#/">"Réinitialiser"</a>
                })}
            </div>

            {body}

            <CreateTray app=app open=open kind=kind />
        </main>
    }
}

fn catalog_onboarding(app: App, open: RwSignal<bool>) -> AnyView {
    view! {
        <section class="catalog-empty" aria-label="Premier produit">
            <h2>"Votre portefeuille est vide."</h2>
            <p class="muted">
                "Le produit est le point d'entrée : créez-le, puis rattachez-lui une équipe, des capacités, des applications et des interfaces."
            </p>
            <div class="actions">
                <button class="btn primary" type="button" on:click=move |_| open.set(true)>
                    "Créer un produit"
                </button>
                <button
                    class="btn ghost"
                    type="button"
                    on:click=move |_| {
                        match app.run(repo().load_demo_map()) {
                            Some(true) => app.notify("Référentiel d'exemple chargé : explorez le portefeuille, puis remplacez-le par vos produits."),
                            Some(false) => app.notify("Le référentiel contient déjà des objets : l'exemple n'a pas été chargé."),
                            None => {}
                        }
                    }
                >
                    "Explorer le référentiel d'exemple"
                </button>
                <a class="btn ghost" href="#/map">"Voir la carte"</a>
            </div>
        </section>
    }
    .into_any()
}

fn product_card(card: ProductCard) -> AnyView {
    let href = Route::Product { id: card.id }.to_hash();
    let map = Route::Map(
        MapQuery::map_default()
            .at_scope(MapScope::product(card.id))
            .level(MapLevel::Product),
    )
    .to_hash();
    let team = card
        .team_name
        .clone()
        .unwrap_or_else(|| "Sans équipe".into());
    let stream = card
        .value_stream_name
        .clone()
        .unwrap_or_else(|| "Hors value stream".into());
    let gaps = card
        .gaps
        .iter()
        .map(|gap| (*gap).label())
        .collect::<Vec<_>>()
        .join(" · ");

    view! {
        <article class="product-card">
            <a class="product-card-main" href=href.clone()>
                <header class="product-card-head">
                    <h2>{card.name.clone()}</h2>
                    <span class="badge cat">{card.kind.label()}</span>
                </header>
                <p class="product-card-meta">
                    <span>{team}</span>
                    <span aria-hidden="true">"·"</span>
                    <span>{stream}</span>
                </p>
                <p class="product-card-status">
                    <span class="badge">{card.lifecycle.label()}</span>
                    <span class="badge">{card.criticality.label()}</span>
                    {if card.anomaly_count > 0 {
                        view! {
                            <span class="product-card-signal">
                                {format!("{} signal(s)", card.anomaly_count)}
                            </span>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                </p>
                {(!card.value_proposition.trim().is_empty()).then(|| view! {
                    <p class="product-card-value">{card.value_proposition.clone()}</p>
                })}
                <ul class="product-card-stats">
                    <li>{format!("{} capacités", card.capability_count)}</li>
                    <li>{format!("{} applications", card.application_count)}</li>
                    <li>{format!("{} interfaces exposées", card.exposed_interface_count)}</li>
                    <li>{format!("{} interfaces consommées", card.consumed_interface_count)}</li>
                </ul>
                {(!gaps.is_empty()).then(|| view! {
                    <p class="product-card-gaps">{format!("À compléter : {gaps}")}</p>
                })}
            </a>
            <footer class="product-card-actions">
                <a href=map>"Voir sur la carte"</a>
                <a href=href>"Ouvrir la fiche"</a>
            </footer>
        </article>
    }
    .into_any()
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count <= 1 { one } else { many })
}
