//! `/atelier` — Atelier : dernière vue éditée, sinon écran « Nouvelle vue » (VD-01).

use crate::design::commands;
use crate::i18n::t;
use crate::router::{Route, ViewLink};
use crate::state::{clear_last_view, input_value, read_last_view, repo, App};
use archifier_core::{ObjectId, ViewIntent};
use leptos::prelude::*;

#[component]
pub fn Atelier(app: App, initial_intent: Option<ViewIntent>) -> impl IntoView {
    let last = read_last_view();
    let resume = if initial_intent.is_some() {
        None
    } else {
        last.as_ref()
            .filter(|entry| repo().diagram_detail(entry.view_id).is_some())
            .cloned()
    };

    if last.is_some() && resume.is_none() {
        clear_last_view();
    }

    let navigated = StoredValue::new(false);
    let resume_for_effect = resume.clone();
    Effect::new(move |_| {
        if navigated.get_value() {
            return;
        }
        if let Some(entry) = resume_for_effect.clone() {
            navigated.set_value(true);
            app.navigate(Route::ViewEditor {
                id: entry.view_id,
                link: ViewLink {
                    zoom: entry.framing.zoom,
                    x: Some(entry.framing.x),
                    y: Some(entry.framing.y),
                    focus: entry.framing.focus,
                    dim: entry.framing.dim_others,
                    caption: entry.framing.caption,
                    t: entry.temporal_cursor,
                },
            });
        }
    });

    let domains = RwSignal::new(repo().domains());
    let initial_domain = domains.get_untracked().first().map(|d| d.id);
    let domain_signal = RwSignal::new(initial_domain);
    let intent = RwSignal::new(initial_intent.unwrap_or(ViewIntent::ApplicationLandscape));
    let (new_domain_name, set_new_domain_name) = signal(String::new());

    let create_domain = move |_| {
        let name = new_domain_name.get();
        if name.trim().is_empty() {
            app.notify(t("atelier.domain_name_required"));
            return;
        }
        if app.run(repo().create_domain(&name, "")).is_some() {
            let next = repo().domains();
            domain_signal.set(next.first().map(|d| d.id));
            domains.set(next);
            set_new_domain_name.set(String::new());
            app.notify(t("atelier.domain_created"));
        }
    };

    let create = move |_| {
        let Some(domain_id) = domain_signal.get_untracked() else {
            return;
        };
        match commands::create_view(intent.get_untracked(), domain_id) {
            Ok(id) => app.navigate(Route::ViewEditor {
                id,
                link: ViewLink::default(),
            }),
            Err(error) => app.fail(&error),
        }
    };

    let resume_href = resume
        .as_ref()
        .map(|entry| format!("#/design/{}", entry.view_id));
    let show_onboarding = RwSignal::new(resume.is_none());
    let resume_title = resume
        .as_ref()
        .and_then(|entry| repo().diagram_detail(entry.view_id))
        .map(|detail| detail.diagram.object.name)
        .unwrap_or_default();

    view! {
        <main class="page atelier">
            <header class="atelier-head">
                <div>
                    <h1>{t("atelier.title")}</h1>
                    <p class="muted">{t("atelier.subtitle")}</p>
                </div>
                {if let Some(href) = resume_href {
                    view! {
                        <a class="resume-card" href=href>
                            <span class="resume-label">{t("atelier.resume")}</span>
                            <strong>{resume_title}</strong>
                            <span class="muted small">{t("atelier.resume_hint")}</span>
                        </a>
                    }.into_any()
                } else { ().into_any() }}
            </header>

            {move || show_onboarding.get().then(|| view! {
                <section class="onboarding" aria-label=t("onboarding.title")>
                    <div>
                        <strong>{t("onboarding.title")}</strong>
                        <ol>
                            <li>{t("onboarding.step1")}</li>
                            <li>{t("onboarding.step2")}</li>
                            <li>{t("onboarding.step3")}</li>
                            <li>{t("onboarding.step4")}</li>
                        </ol>
                    </div>
                    <button class="btn ghost small" on:click=move |_| show_onboarding.set(false)>
                        {t("onboarding.skip")}
                    </button>
                </section>
            })}

            <div class="atelier-body">
                <nav class="intent-list" aria-label=t("atelier.intentions")>
                    <h2 class="small caps">{t("atelier.intentions")}</h2>
                    {ViewIntent::ALL.into_iter().map(|value| {
                        let selected = move || intent.get() == value;
                        view! {
                            <button
                                class="intent-item"
                                class:selected=selected
                                aria-pressed=move || selected().to_string()
                                on:click=move |_| intent.set(value)
                            >
                                <span>{t(value.label_key())}</span>
                                <span class="muted small">{t(&value.label_key().replace(".label", ".tagline"))}</span>
                            </button>
                        }
                    }).collect_view()}
                </nav>

                <section class="intent-preview" aria-label=t("atelier.preview")>
                    {move || {
                        let selected = intent.get();
                        let domain = domain_signal.get().unwrap_or_else(|| ObjectId::from_parts(0, 0));
                        preview(selected, domain)
                    }}
                    <div class="intent-footer">
                        <p>{move || t(&intent.get().label_key().replace(".label", ".empty"))}</p>
                        <div class="intent-actions">
                            {move || if domains.get().is_empty() {
                                view! {
                                    <>
                                        <label class="field inline">
                                            <span>{t("atelier.first_domain")}</span>
                                            <input
                                                type="text"
                                                placeholder=t("atelier.first_domain_placeholder")
                                                prop:value=move || new_domain_name.get()
                                                on:input=move |ev| set_new_domain_name.set(input_value(&ev))
                                            />
                                        </label>
                                        <button class="btn primary" on:click=create_domain>
                                            {t("atelier.create_domain")}
                                        </button>
                                    </>
                                }.into_any()
                            } else {
                                view! {
                                    <>
                                        <label class="field inline">
                                            <span>{t("atelier.choose_domain")}</span>
                                            <select
                                                on:change=move |ev| {
                                                    domain_signal.set(crate::state::parse_object_id(&input_value(&ev)));
                                                }
                                            >
                                                {move || domains.get().into_iter().map(|domain| view! {
                                                    <option value=domain.id.to_string() selected=domain_signal.get() == Some(domain.id)>
                                                        {domain.name.clone()}
                                                    </option>
                                                }).collect_view()}
                                            </select>
                                        </label>
                                        <button class="btn primary" on:click=create>
                                            {t("atelier.create")}
                                        </button>
                                    </>
                                }.into_any()
                            }}
                        </div>
                        {move || domains.get().is_empty().then(|| view! {
                            <p class="callout warn">{t("atelier.no_domain")}</p>
                        })}
                    </div>
                </section>
            </div>

            <footer class="atelier-foot">
                <a class="btn ghost" href="#/control/exports">{t("atelier.import")}</a>
                <span class="muted small">{t("atelier.resume_hint")}</span>
            </footer>
        </main>
    }
}

fn preview(intent: ViewIntent, domain: ObjectId) -> AnyView {
    let layout = crate::templates::layout(intent, domain);
    let colors = [
        ("business", "#eef2fb", "#5b6ea8"),
        ("application", "#e9f5ef", "#2f7d5a"),
        ("technology", "#fdf0e6", "#a9622a"),
        ("physical", "#f3ecfa", "#7a4fa3"),
        ("motivation", "#fff5e3", "#a97a12"),
        ("implementation", "#eef1f4", "#62748a"),
        ("process", "#eaf1fb", "#3c6aa6"),
        ("flow", "#f2f7fd", "#5b86b8"),
        ("data", "#eef6f2", "#3d8a63"),
        ("annotation", "#f6f5ef", "#8a8360"),
        ("structure", "#eef3f7", "#4f7488"),
        ("deployment", "#fdf0e6", "#a9622a"),
    ];
    let layer_views = layout.layers.clone().into_iter().map(|layer| {
        let (_, bg, accent) = colors
            .iter()
            .find(|(id, _, _)| *id == layer.id)
            .copied()
            .unwrap_or(("", "#f7f9fd", "#6b7787"));
        view! {
            <g>
                <rect x="0" y=fmt(layer.y) width="1000" height=fmt(layer.h) fill=bg rx="8"></rect>
                <rect x="0" y=fmt(layer.y) width="6" height=fmt(layer.h) fill=accent rx="3"></rect>
                <text x="16" y=fmt(layer.y + 22.0) font-size="13" fill="#28374f" font-weight="600">
                    {layer.label.clone()}
                </text>
            </g>
        }
    }).collect_view();
    let zone_views = layout.zones.clone().into_iter().map(|zone| {
        let dashed = if zone.boundary { "none" } else { "6 4" };
        view! {
            <g>
                <rect
                    x=fmt(zone.x) y=fmt(zone.y) width=fmt(zone.w) height=fmt(zone.h)
                    fill="#f7f9fd" stroke="#9fb2cc" stroke-width="2" rx="8"
                    stroke-dasharray=dashed
                ></rect>
                <text x=fmt(zone.x + 12.0) y=fmt(zone.y + 24.0) font-size="13" fill="#1b2430" font-weight="600">
                    {zone.label.clone()}
                </text>
            </g>
        }
    }).collect_view();
    let draft_views = layout.drafts.clone().into_iter().map(|draft| {
        let is_port = draft.shape == archifier_core::ShapeKind::Port;
        if is_port {
            view! {
                <circle cx=fmt(draft.x) cy=fmt(draft.y) r="9" fill="#ffffff" stroke="#2f6feb" stroke-width="2"></circle>
            }.into_any()
        } else {
            view! {
                <g>
                    <rect
                        x=fmt(draft.x) y=fmt(draft.y) width=fmt(draft.w) height=fmt(draft.h)
                        fill="#fbfbfd" stroke="#6b7787" stroke-dasharray="4 3" rx="8"
                    ></rect>
                    <text x=fmt(draft.x + 12.0) y=fmt(draft.y + 26.0) font-size="13" fill="#48566a" font-style="italic">
                        {draft.label.clone()}
                    </text>
                </g>
            }.into_any()
        }
    }).collect_view();
    view! {
        <svg class="intent-canvas" viewBox="0 0 1000 760" role="img" aria-label=t(intent.label_key())>
            {layer_views}
            {zone_views}
            {draft_views}
        </svg>
    }
    .into_any()
}

fn fmt(value: f64) -> String {
    format!("{value:.1}")
}
