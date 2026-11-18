//! `/control` — espace secondaire : inventaire, anomalies, couverture, exports.
//! Aucune création d'objet depuis le Contrôle (VD-16).

use crate::i18n::t;
use crate::router::{ControlTab, Route};
use crate::state::{download_text, read_text_file, repo, selected_file, App};
use leptos::prelude::*;

#[component]
pub fn Control(app: App, tab: ControlTab) -> impl IntoView {
    let _ = app;
    let stats = repo().stats();
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
                <p class="muted small">
                    {t("control.counts")
                        .replace("{domains}", &stats.domains.to_string())
                        .replace("{elements}", &stats.elements.to_string())
                        .replace("{interfaces}", &stats.interfaces.to_string())
                        .replace("{views}", &stats.diagrams.to_string())}
                </p>
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
    let mut anomalies = Vec::new();
    for domain in repo().domains() {
        if let Ok(overview) = repo().domain_overview(domain.id) {
            for anomaly in overview.anomalies {
                anomalies.push(anomaly);
            }
        }
    }
    let broken = repo().broken_links();
    view! {
        <section>
            {if anomalies.is_empty() && broken.is_empty() {
                view! { <p class="callout ok">{t("control.no_anomaly")}</p> }.into_any()
            } else {
                view! {
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
    let objects = repo().all_objects();
    view! {
        <section class="table-wrap">
            {if objects.is_empty() {
                view! { <p class="callout">{t("control.inventory.empty")}</p> }.into_any()
            } else {
                view! {
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
                                    <td>{object.object_type.as_str()}</td>
                                    <td>{object.domain_name.clone()}</td>
                                    <td>{format!("{:?}", object.lifecycle)}</td>
                                </tr>
                            }).collect_view()}
                        </tbody>
                    </table>
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
