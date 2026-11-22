//! `/library` et `/library/domains/:id` — espace secondaire (VD-15).

use crate::focus::{
    build_focus_context, focus_context_card, focus_layout, panel, FocusGraph, FocusLink, FocusNode,
};
use crate::i18n::t;
use crate::router::{LibraryTab, Route};
use crate::state::{input_value, repo, App};
use archifier_repo::ViewFilter;
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
        (LibraryTab::Products, "library.products"),
        (LibraryTab::Teams, "library.teams"),
        (LibraryTab::ValueStreams, "library.value_streams"),
        (LibraryTab::Capabilities, "library.capabilities"),
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
                <p class="muted">"Retrouver un objet du référentiel ou une vue authored. La carte reste le point d'entrée."</p>
            </header>
            <nav class="tabs" aria-label=t("library.title")>
                {tabs.into_iter().map(|(value, key)| {
                    let href = Route::Library { tab: value.clone() }.to_hash();
                    view! {
                        <a class="tab" class:active=tab == value href=href>{t(key)}</a>
                    }
                }).collect_view()}
            </nav>
            {(tab == LibraryTab::Views).then(|| view! {
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
            })}
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
                LibraryTab::Products => view! {
                    <MapInventory kind=archifier_core::map::MapObjectKind::Product />
                }.into_any(),
                LibraryTab::Teams => view! {
                    <MapInventory kind=archifier_core::map::MapObjectKind::Team />
                }.into_any(),
                LibraryTab::ValueStreams => view! {
                    <MapInventory kind=archifier_core::map::MapObjectKind::Stage />
                }.into_any(),
                LibraryTab::Capabilities => view! {
                    <MapInventory kind=archifier_core::map::MapObjectKind::Capability />
                }.into_any(),
                LibraryTab::Domains => view! {
                    <section class="domain-list">
                        {repo().map_data().domains.into_iter().map(|domain| {
                            view! {
                                <a class="card domain-row" href=format!("#/business-domains/{}", domain.id)>
                                    <strong>{domain.name.clone()}</strong>
                                    <span class="muted small">{domain.description.clone()}</span>
                                    <span class="counts">"Domaine métier · classification"</span>
                                </a>
                            }
                        }).collect_view()}
                        {repo().domains().into_iter().map(|domain| {
                            let href = Route::BusinessDomain { id: domain.id }.to_hash();
                            view! {
                                <a class="card domain-row" href=href>
                                    <strong>{domain.name.clone()}</strong>
                                    <span class="muted small">{domain.description.clone()}</span>
                                    <span class="counts">
                                        {format!("Domaine historique MVP3 · {} vues · {} interfaces · {} éléments", domain.counts.diagrams, domain.counts.interfaces, domain.counts.elements)}
                                    </span>
                                </a>
                            }
                        }).collect_view()}
                    </section>
                }.into_any(),
                LibraryTab::Objects => view! {
                    <MapInventoryAll />
                }.into_any(),
                LibraryTab::Interfaces => view! {
                    <MapInventory kind=archifier_core::map::MapObjectKind::Interface />
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

/// Liste d'objets natifs v0.4. La Bibliothèque reste un espace secondaire :
/// la carte est le point d'entrée, pas cette liste.
#[component]
fn MapInventory(kind: archifier_core::map::MapObjectKind) -> impl IntoView {
    let items: Vec<archifier_core::map::MapSearchHit> = repo()
        .map_inventory()
        .into_iter()
        .filter(|item| item.kind == kind)
        .collect();
    if items.is_empty() {
        return view! {
            <p class="callout">
                "Aucun objet de ce type. Créez-le depuis la carte, sans formulaire modal."
            </p>
        }
        .into_any();
    }
    view! {
        <section class="domain-list">
            {items.into_iter().map(|item| view! {
                <a class="card domain-row" href=item.route.clone()>
                    <strong>{item.label.clone()}</strong>
                    <span class="muted small">{item.detail.clone()}</span>
                </a>
            }).collect_view()}
        </section>
    }
    .into_any()
}

/// Inventaire complet des objets natifs v0.4, sans filtre de type.
#[component]
fn MapInventoryAll() -> impl IntoView {
    let items = repo().map_inventory();
    if items.is_empty() {
        return view! {
            <p class="callout">
                "Aucun objet de carte. Créez-le depuis la carte, sans formulaire modal."
            </p>
        }
        .into_any();
    }
    view! {
        <section class="domain-list">
            {items.into_iter().map(|item| view! {
                <a class="card domain-row" href=item.route.clone()>
                    <strong>{item.label.clone()}</strong>
                    <span class="muted small">{item.detail.clone()}</span>
                </a>
            }).collect_view()}
        </section>
    }
    .into_any()
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
        Ok(overview) => {
            let exposed: Vec<FocusNode> = overview
                .exposed_interfaces
                .iter()
                .map(|row| FocusNode {
                    id: row.id,
                    label: row.name.clone(),
                    sublabel: format!("Interface · {}", row.category.label()),
                    color_key: "external".into(),
                    route: row.href.clone(),
                })
                .collect();
            let consumed: Vec<FocusNode> = overview
                .consumed_interfaces
                .iter()
                .map(|row| FocusNode {
                    id: row.id,
                    label: row.name.clone(),
                    sublabel: format!("Interface · {}", row.category.label()),
                    color_key: "external".into(),
                    route: row.href.clone(),
                })
                .collect();
            let mut links: Vec<FocusLink> = exposed
                .iter()
                .map(|interface| FocusLink {
                    from: id,
                    to: interface.id,
                    label: "expose".into(),
                })
                .collect();
            links.extend(consumed.iter().map(|interface| FocusLink {
                from: interface.id,
                to: id,
                label: "consomme".into(),
            }));
            let context = build_focus_context(FocusGraph {
                center_id: id,
                center_label: overview.domain.name.clone(),
                center_meta: "Domaine métier · classification".into(),
                center_badge: Some(format!("{} interface(s)", exposed.len() + consumed.len())),
                empty_label: None,
                inside: Vec::new(),
                left_label: (!exposed.is_empty()).then_some("INTERFACES EXPOSÉES".into()),
                left: exposed,
                right_label: (!consumed.is_empty()).then_some("INTERFACES CONSOMMÉES".into()),
                right: consumed,
                links,
                center_legend: "Domaine".into(),
                inside_legend: None,
                around_legend: Some("Interface".into()),
            });
            let exposed_rows = overview.exposed_interfaces;
            let consumed_rows = overview.consumed_interfaces;
            let context_summary = format!(
                "{} · {} · {}",
                format!("{} interface(s) exposée(s)", exposed_rows.len()),
                format!("{} interface(s) consommée(s)", consumed_rows.len()),
                format!("{} vue(s) publiée(s)", overview.published_diagrams.len())
            );
            let context_card = focus_context_card(context_summary, context, None);

            let left_panels = vec![
                panel(
                    "Exposition",
                    if exposed_rows.is_empty() {
                        view! { <p class="empty">{t("library.domain.exposed")}</p> }.into_any()
                    } else {
                        view! {
                            <ul class="detail-list">
                                {exposed_rows.into_iter().map(|row| view! {
                                    <li>
                                        <a href=format!("#/view/interfaces/{}", row.id)>{row.name.clone()}</a>
                                        <span class="muted small">{row.category.label()}</span>
                                    </li>
                                }).collect_view()}
                            </ul>
                        }
                        .into_any()
                    },
                ),
                panel(
                    "Dépendances entrantes",
                    if overview.dependencies_in.is_empty() {
                        view! { <p class="empty">"Aucune dépendance entrante."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="detail-list">
                                {overview.dependencies_in.into_iter().map(|link| view! {
                                    <li>
                                        <span>{link.domain_name.clone()}</span>
                                        <span class="muted small">{link.via.join(", ")}</span>
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
                    "Consommation",
                    if consumed_rows.is_empty() {
                        view! { <p class="empty">{t("library.domain.consumed")}</p> }.into_any()
                    } else {
                        view! {
                            <ul class="detail-list">
                                {consumed_rows.into_iter().map(|row| view! {
                                    <li>
                                        <a href=format!("#/view/interfaces/{}", row.id)>{row.name.clone()}</a>
                                        <span class="muted small">{row.category.label()}</span>
                                    </li>
                                }).collect_view()}
                            </ul>
                        }
                        .into_any()
                    },
                ),
                panel(
                    "Vues publiées",
                    if overview.published_diagrams.is_empty() {
                        view! { <p class="empty">"Aucune vue publiée."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="detail-list">
                                {overview.published_diagrams.into_iter().map(|item| view! {
                                    <li><a href=format!("#/design/{}", item.id)>{item.name.clone()}</a></li>
                                }).collect_view()}
                            </ul>
                        }
                        .into_any()
                    },
                ),
                panel(
                    "Dépendances sortantes",
                    if overview.dependencies_out.is_empty() {
                        view! { <p class="empty">"Aucune dépendance sortante."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="detail-list">
                                {overview.dependencies_out.into_iter().map(|link| view! {
                                    <li>
                                        <span>{link.domain_name.clone()}</span>
                                        <span class="muted small">{link.via.join(", ")}</span>
                                    </li>
                                }).collect_view()}
                            </ul>
                        }
                        .into_any()
                    },
                ),
            ];

            view! {
                <main class="page entity-page entity-focus-page">
                    <header class="page-head">
                        <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                            <a href="#/library/domains">{t("library.domains")}</a>
                            <span>"›"</span>
                            <span aria-current="page">{overview.domain.name.clone()}</span>
                        </nav>
                        <div class="entity-title">
                            <div>
                                <h1>{overview.domain.name.clone()}</h1>
                                <p class="muted">{overview.domain.description.clone()}</p>
                            </div>
                        </div>
                    </header>
                    {focus_layout(left_panels, context_card, right_panels)}
                </main>
            }
            .into_any()
        }
        Err(_) => view! {
            <main class="page">
                <h1>{t("state.not_found.title")}</h1>
                <a class="btn" href="#/library/domains">{t("library.domains")}</a>
            </main>
        }
        .into_any(),
    }
}
