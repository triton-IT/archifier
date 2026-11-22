//! `/control` — espace secondaire : inventaire, anomalies, couverture, exports.
//! Aucune création d'objet depuis le Contrôle (VD-16).

use crate::components::LifecycleBadge;
use crate::i18n::t;
use crate::router::{ControlTab, Route};
use crate::state::{download_text, read_text_file, repo, selected_file, App};
use leptos::prelude::*;

#[component]
pub fn Control(app: App, tab: ControlTab) -> impl IntoView {
    let counts_line = move || {
        let _ = app.version.get();
        let data = repo().map_data();
        let views = repo().stats().diagrams;
        format!(
            "{} produits · {} équipes · {} applications · {} interfaces · {} vues",
            data.products.len(),
            data.teams.len(),
            data.applications.len(),
            data.interfaces.len(),
            views
        )
    };
    let tabs = [
        (ControlTab::Anomalies, "control.anomalies"),
        (ControlTab::Inventory, "control.inventory"),
        (ControlTab::Coverage, "control.coverage"),
        (ControlTab::Exports, "control.exports"),
    ];
    view! {
        <main class="page control">
            <header class="page-head">
                <h1>{t("control.title")}</h1>
                <p class="muted">{t("control.subtitle")}</p>
                <p class="muted small">{counts_line}</p>
            </header>
            <nav class="tabs" aria-label=t("control.title")>
                {tabs.into_iter().map(|(value, key)| {
                    let href = Route::Control { tab: value.clone() }.to_hash();
                    view! { <a class="tab" class:active=tab == value href=href>{t(key)}</a> }
                }).collect_view()}
            </nav>
            {match tab {
                ControlTab::Anomalies => view! { <Anomalies /> }.into_any(),
                ControlTab::Inventory => view! { <Inventory /> }.into_any(),
                ControlTab::Coverage => view! { <Coverage /> }.into_any(),
                ControlTab::Exports => view! { <Exports /> }.into_any(),
            }}
        </main>
    }
}

#[component]
fn Anomalies() -> impl IntoView {
    let inventory = repo().map_inventory();
    let map_anomalies = repo().map_anomalies();
    let mut anomalies = Vec::new();
    for domain in repo().domains() {
        if let Ok(overview) = repo().domain_overview(domain.id) {
            for anomaly in overview.anomalies {
                anomalies.push(anomaly);
            }
        }
    }
    let broken = repo().broken_links();
    let total = map_anomalies.len() + anomalies.len() + broken.len();
    view! {
        <section>
            {if total == 0 {
                view! { <p class="callout ok">{t("control.no_anomaly")}</p> }.into_any()
            } else {
                view! {
                    {(!map_anomalies.is_empty()).then(|| view! {
                        <h2 class="small caps">"Carte de l'entreprise"</h2>
                    })}
                    <ul class="anomaly-list">
                        {map_anomalies.into_iter().map(|issue| {
                            let href = if issue.code.starts_with("MAP-") {
                                None
                            } else {
                                issue
                                    .object_id
                                    .and_then(|id| inventory.iter().find(|hit| hit.object_id == id))
                                    .map(|hit| hit.route.clone())
                            };
                            view! {
                                <li class=if issue.blocking { "anomaly blocking" } else { "anomaly" }>
                                    <span class=if issue.is_completeness() { "badge" } else { "badge warn" }>{issue.code.clone()}</span>
                                    <span>{issue.message.clone()}</span>
                                    {href.map(|href| view! {
                                        <a class="small" href=href>"Voir l'objet"</a>
                                    })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                    {(!anomalies.is_empty() || !broken.is_empty()).then(|| view! {
                        <h2 class="small caps">"Référentiel historique MVP3"</h2>
                    })}
                    <ul class="anomaly-list">
                        {anomalies.into_iter().map(|anomaly| view! {
                            <li class="anomaly">
                                <span class="badge warn">{anomaly.code.as_str()}</span>
                                <span>{anomaly.message.clone()}</span>
                                <a class="small" href=anomaly.href.clone()>{t("control.anomaly.target")}</a>
                            </li>
                        }).collect_view()}
                        {broken.into_iter().map(|link| view! {
                            <li class="anomaly">
                                <span class="badge warn">{link.code.clone()}</span>
                                <span>{link.context.clone()}</span>
                                <a class="small" href=link.source.href.clone()>{t("control.anomaly.target")}</a>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn Inventory() -> impl IntoView {
    let map_items = repo().map_inventory();
    let objects = repo().all_objects();
    view! {
        <section class="table-wrap">
            {if map_items.is_empty() && objects.is_empty() {
                view! { <p class="callout">{t("control.inventory.empty")}</p> }.into_any()
            } else {
                view! {
                    {(!map_items.is_empty()).then(|| view! {
                        <h2 class="small caps">"Carte de l'entreprise"</h2>
                        <table class="table">
                            <thead>
                                <tr><th>"Nom"</th><th>"Détail"</th></tr>
                            </thead>
                            <tbody>
                                {map_items.into_iter().map(|item| view! {
                                    <tr>
                                        <td><a href=item.route.clone()>{item.label.clone()}</a></td>
                                        <td>{item.detail.clone()}</td>
                                    </tr>
                                }).collect_view()}
                            </tbody>
                        </table>
                    })}
                    {(!objects.is_empty()).then(|| view! {
                        <h2 class="small caps">"Référentiel historique MVP3"</h2>
                        <table class="table">
                            <thead>
                                <tr>
                                    <th>"Nom"</th><th>"Type"</th><th>"Domaine"</th><th>"Cycle"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {objects.into_iter().map(|object| view! {
                                    <tr>
                                        <td>
                                            <a href=format!("#/view/{}/{}", object.object_type.route_segment(), object.id)>
                                                {object.name.clone()}
                                            </a>
                                        </td>
                                        <td>{object.object_type.label()}</td>
                                        <td>{object.domain_name.clone()}</td>
                                        <td><LifecycleBadge lifecycle=object.lifecycle /></td>
                                    </tr>
                                }).collect_view()}
                            </tbody>
                        </table>
                    })}
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn Coverage() -> impl IntoView {
    let coverage = repo().metamodel().coverage();
    view! {
        <section class="table-wrap">
            <table class="table">
                <thead>
                    <tr>
                        <th>{t("control.coverage.profile")}</th>
                        <th>{t("control.coverage.classes")}</th>
                        <th>{t("control.coverage.relations")}</th>
                    </tr>
                </thead>
                <tbody>
                    {coverage.into_iter().map(|(profile, classes, relations, rules)| view! {
                        <tr>
                            <td>{profile}</td>
                            <td>{format!("{classes}")}</td>
                            <td>{format!("{relations} · {rules} règles")}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </section>
    }
}

#[component]
fn Exports() -> impl IntoView {
    let message = RwSignal::new(String::new());
    let import = move |ev: web_sys::Event| {
        let Some(file) = selected_file(&ev) else {
            return;
        };
        let setter = message;
        read_text_file(&file, move |text| match repo().load_json(&text) {
            Ok(()) => setter.set(t("control.exports.snapshot")),
            Err(error) => setter.set(format!("{} — {}", error.message, error.code)),
        });
    };
    view! {
        <section class="card">
            <h2>{t("control.exports")}</h2>
            <div class="actions">
                <button class="btn" on:click=move |_| {
                    download_text("archifier-snapshot.json", &repo().export_json());
                }>{t("control.exports.snapshot")}</button>
                <label class="btn ghost">
                    {t("control.exports.import")}
                    <input type="file" accept=".json" style="display:none" on:change=import />
                </label>
            </div>
            <p class="muted small">{move || message.get()}</p>
        </section>
    }
}
