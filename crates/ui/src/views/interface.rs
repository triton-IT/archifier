//! Fiche d'interface : contexte système, versions, frise temporelle,
//! chevauchements, consommateurs, documentation et historique.

use crate::components::{
    date_or_dash, Card, DiffTable, Empty, ExportMarkdownButton, Markdown, RevisionCompare,
    TemporalBadge,
};
use crate::focus::{
    build_focus_context, focus_context_card, focus_layout, FocusGraph, FocusLink, FocusNode,
};
use crate::state::{download_text, fmt_date, input_value, is_checked, parse_date, repo, App};
use archifier_core::{ContractDiff, ContractType, InterfaceCategory, ObjectId, TemporalStatus};
use leptos::prelude::*;

struct Bar {
    label: String,
    left: f64,
    width: f64,
    class: String,
    detail: String,
}

fn contract_label(contract: ContractType) -> &'static str {
    match contract {
        ContractType::None => "Sans contrat",
        ContractType::OpenApi => "OpenAPI",
        ContractType::Event => "Événement",
        ContractType::Other => "Autre",
    }
}

#[component]
pub fn InterfaceView(app: App, id: ObjectId) -> impl IntoView {
    let detail = move || {
        let _ = app.version.get();
        repo().interface_detail(id)
    };
    let timeline = move || {
        let _ = app.version.get();
        repo().timeline(id).ok()
    };

    let bars = move || -> Vec<Bar> {
        let Some(line) = timeline() else {
            return vec![];
        };
        let mut min: Option<time::OffsetDateTime> = None;
        let mut max: Option<time::OffsetDateTime> = None;
        for segment in &line.segments {
            if let (Some(from), Some(until)) = (segment.from, segment.until) {
                min = Some(min.map_or(from, |m| m.min(from)));
                max = Some(max.map_or(until, |m| m.max(until)));
            }
        }
        let (Some(min), Some(max)) = (min, max) else {
            return vec![];
        };
        let span = (max - min).as_seconds_f64().max(1.0);
        line.segments
            .iter()
            .filter_map(|segment| {
                let (from, until) = (segment.from?, segment.until?);
                let left = ((from - min).as_seconds_f64() / span * 100.0).clamp(0.0, 100.0);
                let width =
                    (((until - from).as_seconds_f64() / span) * 100.0).clamp(1.5, 100.0 - left);
                let class = match segment.status {
                    Some(TemporalStatus::Active) => "bar active",
                    Some(TemporalStatus::Scheduled) => "bar scheduled",
                    Some(TemporalStatus::Expired) => "bar expired",
                    None => "bar draft",
                };
                Some(Bar {
                    label: format!("v{}", segment.semantic_version),
                    left,
                    width,
                    class: class.to_string(),
                    detail: format!(
                        "{} → {} · {} consommateur(s){}",
                        fmt_date(from),
                        fmt_date(until),
                        segment.consumers,
                        if segment.breaking_change {
                            " · rupture"
                        } else {
                            ""
                        }
                    ),
                })
            })
            .collect()
    };

    let now_pct = move || -> f64 {
        let Some(line) = timeline() else { return 0.0 };
        let mut min: Option<time::OffsetDateTime> = None;
        let mut max: Option<time::OffsetDateTime> = None;
        for segment in &line.segments {
            if let (Some(from), Some(until)) = (segment.from, segment.until) {
                min = Some(min.map_or(from, |m| m.min(from)));
                max = Some(max.map_or(until, |m| m.max(until)));
            }
        }
        let (Some(min), Some(max)) = (min, max) else {
            return 0.0;
        };
        let span = (max - min).as_seconds_f64().max(1.0);
        ((line.now - min).as_seconds_f64() / span * 100.0).clamp(0.0, 100.0)
    };

    // Export OpenAPI et comparaison de versions (W1a / T1-03)
    let initial_versions = detail()
        .map(|view_model| view_model.versions)
        .unwrap_or_default();
    let first_version = initial_versions
        .first()
        .map(|summary| summary.version.id.to_string())
        .unwrap_or_default();
    let last_version = initial_versions
        .last()
        .map(|summary| summary.version.id.to_string())
        .unwrap_or_default();
    let (cmp_from, set_cmp_from) = signal(first_version);
    let (cmp_to, set_cmp_to) = signal(last_version);
    let (cmp_diff, set_cmp_diff) = signal(None::<ContractDiff>);

    let export_openapi = move || {
        let Some(view_model) = detail() else {
            return;
        };
        let latest = view_model
            .versions
            .iter()
            .rev()
            .find(|summary| {
                summary
                    .version
                    .contract_snapshot
                    .as_object()
                    .map(|snapshot| !snapshot.is_empty())
                    .unwrap_or(false)
            })
            .or_else(|| view_model.versions.last());
        let Some(latest) = latest else {
            app.notify("Aucune version à exporter.");
            return;
        };
        match repo().export_openapi(latest.version.id) {
            Ok(document) => {
                let filename = format!(
                    "{}-{}.openapi.json",
                    view_model.interface.logical_interface_key, latest.version.semantic_version
                );
                download_text(&filename, &document);
                app.notify("Export OpenAPI téléchargé.");
            }
            Err(error) => app.fail(&error),
        }
    };

    let compare_versions = move || {
        let Some(view_model) = detail() else {
            return;
        };
        if view_model.versions.len() < 2 {
            app.notify("Au moins deux versions sont nécessaires pour comparer.");
            return;
        }
        let ids: Vec<ObjectId> = view_model
            .versions
            .iter()
            .map(|summary| summary.version.id)
            .collect();
        let (Some(first), Some(last)) = (ids.first().copied(), ids.last().copied()) else {
            return;
        };
        let from_id = cmp_from
            .get()
            .parse::<ObjectId>()
            .ok()
            .filter(|id| ids.contains(id))
            .unwrap_or(first);
        let to_id = cmp_to
            .get()
            .parse::<ObjectId>()
            .ok()
            .filter(|id| ids.contains(id))
            .unwrap_or(last);
        set_cmp_from.set(from_id.to_string());
        set_cmp_to.set(to_id.to_string());
        match repo().diff_openapi_versions(from_id, to_id) {
            Ok(diff) => set_cmp_diff.set(Some(diff)),
            Err(error) => {
                set_cmp_diff.set(None);
                app.fail(&error);
            }
        }
    };

    // Formulaire de publication
    let (semver, set_semver) = signal(String::new());
    let (from, set_from) = signal(String::new());
    let (until, set_until) = signal(String::new());
    let (breaking, set_breaking) = signal(false);
    let (show_form, set_show_form) = signal(false);
    let publish = move || {
        let Some(from_date) = parse_date(&from.get()) else {
            app.notify("Date de début invalide (format AAAA-MM-JJ).");
            return;
        };
        let Some(until_date) = parse_date(&until.get()) else {
            app.notify("Date de fin invalide (format AAAA-MM-JJ).");
            return;
        };
        if let Some(version) = app.run(repo().create_interface_version(
            id,
            &semver.get(),
            Some(from_date),
            Some(until_date),
            breaking.get(),
            true,
        )) {
            let _ = version;
            set_semver.set(String::new());
            set_from.set(String::new());
            set_until.set(String::new());
            set_show_form.set(false);
            app.notify("Version publiée.");
        }
    };

    view! {
        <main class="page entity-page entity-focus-page">
            {move || match detail() {
                None => view! { <Empty text="Interface introuvable.".into() /> }.into_any(),
                Some(view_model) => {
                    let interface = view_model.interface.clone();
                    let object = interface.object.clone();
                    let versions = view_model.versions.clone();
                    let version_count = versions.len();
                    let version_choices_from = versions.clone();
                    let version_choices_to = versions.clone();

                    // Contexte système : le domaine fournisseur à gauche,
                    // les consommateurs à droite, l'interface au centre.
                    let provider = FocusNode {
                        id: interface.provider_domain_id,
                        label: view_model.provider_domain_name.clone(),
                        sublabel: "Domaine fournisseur".into(),
                        color_key: "external".into(),
                        route: format!("#/view/domains/{}", interface.provider_domain_id),
                    };
                    let consumers: Vec<FocusNode> = view_model
                        .consumers
                        .iter()
                        .map(|consumer| FocusNode {
                            id: consumer.id,
                            label: consumer.name.clone(),
                            sublabel: consumer
                                .object_type
                                .label()
                                .to_string(),
                            color_key: "external".into(),
                            route: consumer.href.clone(),
                        })
                        .collect();
                    let mut links = vec![FocusLink {
                        from: interface.provider_domain_id,
                        to: object.id,
                        label: "fournit".into(),
                    }];
                    links.extend(view_model.consumers.iter().map(|consumer| FocusLink {
                        from: object.id,
                        to: consumer.id,
                        label: "consomme".into(),
                    }));
                    let context = build_focus_context(FocusGraph {
                        center_id: object.id,
                        center_label: object.name.clone(),
                        center_meta: format!(
                            "Interface {} · {}",
                            interface.category.label(),
                            contract_label(interface.contract_type)
                        ),
                        center_badge: Some(if version_count <= 1 {
                            format!("{version_count} version")
                        } else {
                            format!("{version_count} versions")
                        }),
                        empty_label: None,
                        inside: Vec::new(),
                        left_label: Some("FOURNISSEUR".into()),
                        left: vec![provider],
                        right_label: (!consumers.is_empty()).then_some("CONSOMMATEURS".into()),
                        right: consumers,
                        links,
                        center_legend: "Interface".into(),
                        inside_legend: None,
                        around_legend: Some("Objet lié".into()),
                    });
                    let consumer_count = view_model.consumers.len();
                    let context_summary = format!(
                        "{} · {} · {}",
                        if version_count <= 1 {
                            format!("{version_count} version")
                        } else {
                            format!("{version_count} versions")
                        },
                        if consumer_count <= 1 {
                            format!("{consumer_count} consommateur")
                        } else {
                            format!("{consumer_count} consommateurs")
                        },
                        format!("{} vue(s)", view_model.diagrams.len())
                    );
                    let context_card = focus_context_card(
                        context_summary,
                        context,
                        Some((format!("#/graph/{}", object.id), "Graphe de dépendances".into())),
                    );

                    let new_version_panel = view! {
                        {move || show_form.get().then(|| view! {
                            <Card title="Nouvelle version".into()>
                                <div class="form-grid">
                                    <label class="field">
                                        <span>"Version sémantique"</span>
                                        <input placeholder="2.0.0"
                                            prop:value=move || semver.get()
                                            on:input=move |ev| set_semver.set(input_value(&ev)) />
                                    </label>
                                    <label class="field">
                                        <span>"Disponible à partir du"</span>
                                        <input type="date"
                                            prop:value=move || from.get()
                                            on:input=move |ev| set_from.set(input_value(&ev)) />
                                    </label>
                                    <label class="field">
                                        <span>"Jusqu'au (exclu)"</span>
                                        <input type="date"
                                            prop:value=move || until.get()
                                            on:input=move |ev| set_until.set(input_value(&ev)) />
                                    </label>
                                    <label class="field check">
                                        <input type="checkbox"
                                            prop:checked=move || breaking.get()
                                            on:change=move |ev| set_breaking.set(is_checked(&ev)) />
                                        <span>"Rupture de contrat"</span>
                                    </label>
                                </div>
                                <div class="actions">
                                    <button class="btn primary" on:click=move |_| publish()>"Publier"</button>
                                    <button class="btn ghost" on:click=move |_| set_show_form.set(false)>"Annuler"</button>
                                </div>
                            </Card>
                        })}
                    }
                    .into_any();

                    let left_panels = vec![
                        view! {
                            <Card title="Calendrier des versions".into()>
                                {if bars().is_empty() {
                                    view! { <Empty text="Aucune version dotée d'une fenêtre de disponibilité.".into() /> }.into_any()
                                } else {
                                    view! {
                                        <div class="frise">
                                            <div class="frise-track">
                                                {bars().into_iter().map(|bar| view! {
                                                    <div
                                                        class=bar.class
                                                        style=format!("left:{:.2}%;width:{:.2}%", bar.left, bar.width)
                                                        title=bar.detail.clone()
                                                    >
                                                        <span>{bar.label}</span>
                                                    </div>
                                                }).collect_view()}
                                                <div class="frise-now" style=format!("left:{:.2}%", now_pct()) title="Maintenant"></div>
                                            </div>
                                        </div>
                                        <ul class="legend">
                                            <li><span class="swatch active"></span>"Active"</li>
                                            <li><span class="swatch scheduled"></span>"Planifiée"</li>
                                            <li><span class="swatch expired"></span>"Expirée"</li>
                                        </ul>
                                    }.into_any()
                                }}
                                {move || {
                                    let Some(line) = timeline() else { return ().into_any() };
                                    if line.overlaps.is_empty() {
                                        return ().into_any();
                                    }
                                    view! {
                                        <div class="overlaps">
                                            <h3>"Périodes de transition"</h3>
                                            {line.overlaps.into_iter().map(|overlap| view! {
                                                <p>
                                                    {overlap.versions.join(" et ")}
                                                    " coexistent du "
                                                    {fmt_date(overlap.from)}
                                                    " au "
                                                    {fmt_date(overlap.until)}
                                                </p>
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }}
                            </Card>
                        }
                        .into_any(),
                        new_version_panel,
                        view! {
                            <Card title="Versions".into()>
                                <table class="table">
                                    <thead>
                                        <tr>
                                            <th>"Version"</th>
                                            <th>"Statut"</th>
                                            <th>"Éditorial"</th>
                                            <th>"Début"</th>
                                            <th>"Fin (exclue)"</th>
                                            <th>"Consommateurs"</th>
                                            <th>"Rupture"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {versions.into_iter().map(|summary| view! {
                                            <tr>
                                                <td class="mono">{summary.version.semantic_version.clone()}</td>
                                                <td><TemporalBadge status=summary.status /></td>
                                                <td>{summary.version.publication_status.label()}</td>
                                                <td>{date_or_dash(summary.version.available_from)}</td>
                                                <td>{date_or_dash(summary.version.available_until)}</td>
                                                <td>{summary.consumer_count}</td>
                                                <td>{if summary.version.breaking_change { "oui" } else { "non" }}</td>
                                            </tr>
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </Card>
                        }
                        .into_any(),
                        view! {
                            <Card title="Comparer deux versions".into()>
                                {if interface.contract_type != ContractType::OpenApi {
                                    view! {
                                        <Empty text="Comparaison disponible pour les contrats OpenAPI.".into() />
                                    }.into_any()
                                } else if version_count < 2 {
                                    view! {
                                        <Empty text="Au moins deux versions sont nécessaires pour comparer.".into() />
                                    }.into_any()
                                } else {
                                    let from_selected = cmp_from.get();
                                    let to_selected = cmp_to.get();
                                    view! {
                                        <div class="form-grid">
                                            <label class="field">
                                                <span>"Version de départ"</span>
                                                <select
                                                    prop:value=move || cmp_from.get()
                                                    on:change=move |ev| {
                                                        set_cmp_from.set(input_value(&ev));
                                                        set_cmp_diff.set(None);
                                                    }
                                                >
                                                    {version_choices_from.iter().map(|summary| {
                                                        let id = summary.version.id.to_string();
                                                        let value = id.clone();
                                                        view! {
                                                            <option value=value selected=id == from_selected>
                                                                {format!(
                                                                    "v{} · {}",
                                                                    summary.version.semantic_version,
                                                                    summary.version.publication_status.label()
                                                                )}
                                                            </option>
                                                        }
                                                    }).collect_view()}
                                                </select>
                                            </label>
                                            <label class="field">
                                                <span>"Version d'arrivée"</span>
                                                <select
                                                    prop:value=move || cmp_to.get()
                                                    on:change=move |ev| {
                                                        set_cmp_to.set(input_value(&ev));
                                                        set_cmp_diff.set(None);
                                                    }
                                                >
                                                    {version_choices_to.iter().map(|summary| {
                                                        let id = summary.version.id.to_string();
                                                        let value = id.clone();
                                                        view! {
                                                            <option value=value selected=id == to_selected>
                                                                {format!(
                                                                    "v{} · {}",
                                                                    summary.version.semantic_version,
                                                                    summary.version.publication_status.label()
                                                                )}
                                                            </option>
                                                        }
                                                    }).collect_view()}
                                                </select>
                                            </label>
                                        </div>
                                        <div class="actions">
                                            <button class="btn primary" on:click=move |_| compare_versions()>
                                                "Comparer"
                                            </button>
                                        </div>
                                        {move || cmp_diff.get().map(|diff| view! {
                                            <DiffTable diff=diff />
                                        })}
                                    }.into_any()
                                }}
                            </Card>
                        }
                        .into_any(),
                    ];

                    let right_panels = vec![
                        view! {
                            <Card title="Consommateurs".into()>
                                {if view_model.consumers.is_empty() {
                                    view! { <Empty text="Aucune consommation déclarée.".into() /> }.into_any()
                                } else {
                                    view! {
                                        <ul class="link-list">
                                            {view_model.consumers.clone().into_iter().map(|consumer| view! {
                                                <li><a href=consumer.href>{consumer.name.clone()}</a></li>
                                            }).collect_view()}
                                        </ul>
                                    }.into_any()
                                }}
                            </Card>
                        }
                        .into_any(),
                        view! {
                            <Card title="Vues qui la représentent".into()>
                                {if view_model.diagrams.is_empty() {
                                    view! { <Empty text="Aucune vue.".into() /> }.into_any()
                                } else {
                                    view! {
                                        <ul class="link-list">
                                            {view_model.diagrams.clone().into_iter().map(|diagram| view! {
                                                <li><a href=diagram.href>{diagram.name.clone()}</a></li>
                                            }).collect_view()}
                                        </ul>
                                    }.into_any()
                                }}
                            </Card>
                        }
                        .into_any(),
                        view! {
                            <Card title="Documentation".into()>
                                <Markdown markdown=view_model.documentation.clone() />
                            </Card>
                        }
                        .into_any(),
                        view! { <RevisionCompare app=app id=id /> }.into_any(),
                    ];

                    view! {
                        <header class="page-head">
                            <nav class="map-breadcrumb" aria-label="Fil d'Ariane">
                                <a href=format!("#/view/domains/{}", interface.provider_domain_id)>
                                    {view_model.provider_domain_name.clone()}
                                </a>
                                <span>"›"</span>
                                <span aria-current="page">{object.name.clone()}</span>
                            </nav>
                            <div class="entity-title">
                                <div>
                                    <h1>{object.name.clone()}</h1>
                                    <p class="muted">
                                        {interface.category.label()}
                                        {" · "}
                                        {contract_label(interface.contract_type)}
                                        {" · "}
                                        <span class="mono">{interface.logical_interface_key.clone()}</span>
                                    </p>
                                </div>
                                <div class="actions">
                                    <button class="btn primary" on:click=move |_| set_show_form.update(|value| *value = !*value)>
                                        "Publier une version"
                                    </button>
                                    {if interface.category == InterfaceCategory::Application
                                        && interface.contract_type == ContractType::OpenApi
                                    {
                                        view! {
                                            <button class="btn" on:click=move |_| export_openapi()>
                                                "Exporter OpenAPI"
                                            </button>
                                        }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                    <ExportMarkdownButton app=app id=id domain=false />
                                </div>
                            </div>
                        </header>
                        {focus_layout(left_panels, context_card, right_panels)}
                        <p class="muted small mono">
                            {"ID "} {object.id.to_string()}
                        </p>
                    }.into_any()
                }
            }}
        </main>
    }
}
