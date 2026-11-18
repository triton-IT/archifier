//! `/library` et `/library/domains/:id` — espace secondaire (VD-15).

use crate::i18n::t;
use crate::router::{LibraryTab, Route};
use crate::state::{input_value, repo, App};
use archifier_repo::{ObjectFilterType, ViewFilter};
use leptos::prelude::*;

#[component]
pub fn Library(app: App, tab: LibraryTab) -> impl IntoView {
    let _ = app;
    let query = RwSignal::new(String::new());
    let intent = RwSignal::new(None::<archifier_core::ViewIntent>);
    let notation = RwSignal::new(None::<archifier_core::DiagramNotation>);
    let publication = RwSignal::new(None::<archifier_core::PublicationStatus>);
    let tabs = [
        (LibraryTab::Views, "library.views"),
        (LibraryTab::Domains, "library.domains"),
        (LibraryTab::Objects, "library.objects"),
        (LibraryTab::Interfaces, "library.interfaces"),
        (LibraryTab::Stories, "library.stories"),
        (LibraryTab::Reviews, "library.reviews"),
    ];
    view! {
        <main class="page library">
            <header class="page-head">
                <h1>{t("library.title")}</h1>
                <p class="muted">{t("library.subtitle")}</p>
            </header>
            <nav class="tabs" aria-label=t("library.title")>
                {tabs.into_iter().map(|(value, key)| {
                    let href = Route::Library { tab: value.clone() }.to_hash();
                    view! {
                        <a class="tab" class:active=tab == value href=href>{t(key)}</a>
                    }
                }).collect_view()}
            </nav>
            <div class="library-filters">
                <input
                    placeholder=t("library.search")
                    prop:value=move || query.get()
                    on:input=move |ev| query.set(input_value(&ev))
                />
                <select aria-label=t("library.filter.intent") on:change=move |ev| {
                    intent.set(archifier_core::ViewIntent::parse(&input_value(&ev)));
                }>
                    <option value="">{t("library.filter.all")}</option>
                    {archifier_core::ViewIntent::ALL.into_iter().map(|value| view! {
                        <option value=value.key()>{t(value.label_key())}</option>
                    }).collect_view()}
                </select>
                <select aria-label=t("library.filter.publication") on:change=move |ev| {
                    publication.set(match input_value(&ev).as_str() {
                        "PUBLISHED" => Some(archifier_core::PublicationStatus::Published),
                        "DRAFT" => Some(archifier_core::PublicationStatus::Draft),
                        "WITHDRAWN" => Some(archifier_core::PublicationStatus::Withdrawn),
                        _ => None,
                    });
                }>
                    <option value="">{t("library.filter.all")}</option>
                    <option value="PUBLISHED">"Publiées"</option>
                    <option value="DRAFT">"Brouillons"</option>
                    <option value="WITHDRAWN">"Retirées"</option>
                </select>
            </div>
            {match tab {
                LibraryTab::Views => view! {
                    <section class="covers">
                        {move || {
                            let filter = ViewFilter {
                                domain_id: None,
                                intent: intent.get(),
                                notation: notation.get(),
                                publication: publication.get(),
                                owner: None,
                                query: Some(query.get()),
                            };
                            let cards = repo().view_cards(&filter);
                            if cards.is_empty() {
                                return view! { <p class="callout">{t("library.empty")}</p> }.into_any();
                            }
                            cards.into_iter().map(|card| {
                                let design = format!("#/design/{}", card.id);
                                let reading = format!("#/view/diagrams/{}", card.id);
                                view! {
                                    <article class="cover">
                                        <a class="cover-link" href=reading>
                                            {cover_vignette(&card)}
                                            <h2>{card.title.clone()}</h2>
                                        </a>
                                        <div class="cover-meta">
                                            <span class="badge">{t(card.intent.label_key())}</span>
                                            <span class="muted small">{card.domain_name.clone()}</span>
                                            <span class="muted small">
                                                {t("library.cover.nodes").replace("{count}", &card.node_count.to_string())}
                                            </span>
                                            {if card.draft_count > 0 {
                                                view! { <span class="badge warn">{t("library.cover.drafts").replace("{count}", &card.draft_count.to_string())}</span> }.into_any()
                                            } else { ().into_any() }}
                                        </div>
                                        <div class="cover-actions">
                                            <a class="btn ghost small" href=design>{t("library.open_design")}</a>
                                        </div>
                                    </article>
                                }
                            }).collect_view().into_any()
                        }}
                    </section>
                }.into_any(),
                LibraryTab::Domains => view! {
                    <section class="domain-list">
                        {repo().domains().into_iter().map(|domain| {
                            let href = Route::DomainSummary { id: domain.id }.to_hash();
                            view! {
                                <a class="card domain-row" href=href>
                                    <strong>{domain.name.clone()}</strong>
                                    <span class="muted small">{domain.description.clone()}</span>
                                    <span class="counts">
                                        {format!("{} vues · {} interfaces · {} éléments", domain.counts.diagrams, domain.counts.interfaces, domain.counts.elements)}
                                    </span>
                                </a>
                            }
                        }).collect_view()}
                    </section>
                }.into_any(),
                LibraryTab::Objects => view! {
                    <ObjectList filter=ObjectFilterType::Element />
                }.into_any(),
                LibraryTab::Interfaces => view! {
                    <InterfaceList query=query />
                }.into_any(),
                LibraryTab::Stories => view! {
                    <SummaryList items=repo().stories() title_key="library.stories" />
                }.into_any(),
                LibraryTab::Reviews => view! {
                    <SummaryList items=repo().reviews() title_key="library.reviews" />
                }.into_any(),
            }}
        </main>
    }
}

fn cover_vignette(card: &archifier_repo::ViewCard) -> AnyView {
    let (min_x, min_y, max_x, max_y) =
        card.mini_nodes
            .iter()
            .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |acc, node| {
                (
                    acc.0.min(node.x),
                    acc.1.min(node.y),
                    acc.2.max(node.x + node.w),
                    acc.3.max(node.y + node.h),
                )
            });
    let (min_x, min_y, max_x, max_y) = if card.mini_nodes.is_empty() {
        (0.0, 0.0, 800.0, 600.0)
    } else {
        (min_x - 20.0, min_y - 20.0, max_x + 20.0, max_y + 20.0)
    };
    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);
    view! {
        <svg class="cover-canvas" viewBox=format!("{} {} {} {}", min_x, min_y, width, height) role="img" aria-label=card.title.clone()>
            {card.mini_nodes.iter().map(|node| view! {
                <rect
                    x=format!("{:.1}", node.x) y=format!("{:.1}", node.y)
                    width=format!("{:.1}", node.w) height=format!("{:.1}", node.h)
                    rx="6"
                ></rect>
            }).collect_view()}
        </svg>
    }
    .into_any()
}

#[component]
fn ObjectList(filter: ObjectFilterType) -> impl IntoView {
    let objects = repo().search("", &[filter]);
    view! {
        <section class="object-list">
            {if objects.is_empty() {
                view! { <p class="callout">{t("control.inventory.empty")}</p> }.into_any()
            } else {
                view! {
                    <ul class="link-list">
                        {objects.into_iter().take(200).map(|hit| view! {
                            <li>
                                <a href=hit.href>{hit.title.clone()}</a>
                                <span class="muted small">{hit.domain_name.clone()}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn InterfaceList(query: RwSignal<String>) -> impl IntoView {
    let rows = move || {
        let mut filter = archifier_repo::InterfaceFilter::default();
        filter.query = Some(query.get());
        repo().interfaces(&filter)
    };
    view! {
        <section class="object-list">
            <ul class="link-list">
                {move || rows().into_iter().take(200).map(|row| view! {
                    <li>
                        <a href=format!("#/view/interfaces/{}", row.id)>{row.name.clone()}</a>
                        <span class="muted small">{row.provider_domain_name.clone()}</span>
                    </li>
                }).collect_view()}
            </ul>
        </section>
    }
}

#[component]
fn SummaryList(
    items: Vec<archifier_core::ObjectSummary>,
    title_key: &'static str,
) -> impl IntoView {
    view! {
        <section class="object-list">
            <h2>{t(title_key)}</h2>
            <ul class="link-list">
                {items.into_iter().map(|item| {
                    let href = format!("#/view/{}/{}", item.object_type.route_segment(), item.id);
                    view! {
                        <li><a href=href>{item.name.clone()}</a></li>
                    }
                }).collect_view()}
            </ul>
        </section>
    }
}

#[component]
pub fn DomainSummary(app: App, id: archifier_core::ObjectId) -> impl IntoView {
    let _ = app;
    let overview = repo().domain_overview(id);
    match overview {
        Ok(overview) => view! {
            <main class="page">
                <header class="page-head">
                    <a class="muted small" href="#/library/domains">"← " {t("library.domains")}</a>
                    <h1>{overview.domain.name.clone()}</h1>
                    <p class="muted">{overview.domain.description.clone()}</p>
                </header>
                <section class="cards two">
                    <div class="card">
                        <h2>{t("library.domain.exposed")}</h2>
                        <ul class="link-list">
                            {overview.exposed_interfaces.into_iter().map(|row| view! {
                                <li><a href=format!("#/view/interfaces/{}", row.id)>{row.name.clone()}</a></li>
                            }).collect_view()}
                        </ul>
                    </div>
                    <div class="card">
                        <h2>{t("library.domain.consumed")}</h2>
                        <ul class="link-list">
                            {overview.consumed_interfaces.into_iter().map(|row| view! {
                                <li><a href=format!("#/view/interfaces/{}", row.id)>{row.name.clone()}</a></li>
                            }).collect_view()}
                        </ul>
                    </div>
                </section>
                <section class="card">
                    <h2>{t("library.domain.views")}</h2>
                    <ul class="link-list">
                        {overview.published_diagrams.into_iter().map(|item| view! {
                            <li><a href=format!("#/design/{}", item.id)>{item.name.clone()}</a></li>
                        }).collect_view()}
                    </ul>
                </section>
                <section class="card">
                    <h2>{t("library.domain.dependencies")}</h2>
                    <ul class="link-list">
                        {overview.dependencies_out.into_iter().map(|link| view! {
                            <li>{link.domain_name.clone()} <span class="muted small">{link.via.join(", ")}</span></li>
                        }).collect_view()}
                        {overview.dependencies_in.into_iter().map(|link| view! {
                            <li>"← " {link.domain_name.clone()}</li>
                        }).collect_view()}
                    </ul>
                </section>
            </main>
        }.into_any(),
        Err(_) => view! {
            <main class="page">
                <h1>{t("state.not_found.title")}</h1>
                <a class="btn" href="#/library/domains">{t("library.domains")}</a>
            </main>
        }.into_any(),
    }
}
