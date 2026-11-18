//! Revue d'interfaces : vues synthétique et détaillées, anomalies, commentaires.

use crate::components::{
    compact_value, date_or_dash, window_label, Empty, Markdown, TemporalBadge,
};
use crate::state::{fmt_date, input_value, parse_object_id, repo, App};
use archifier_core::{ObjectId, ReviewStatus};
use archifier_repo::{Anomaly, InterfaceRow, ReviewComparison};
use leptos::prelude::*;

#[component]
pub fn ReviewView(app: App, id: ObjectId) -> impl IntoView {
    let detail = move || {
        let _ = app.version.get();
        repo().review_detail(id)
    };
    let (tab, set_tab) = signal("summary".to_string());
    let (comment, set_comment) = signal(String::new());

    let add_comment = move || {
        let text = comment.get();
        if text.trim().is_empty() {
            app.notify("Le commentaire est vide.");
            return;
        }
        if app.run(repo().add_comment(id, &text)).is_some() {
            set_comment.set(String::new());
            app.notify("Commentaire ajouté.");
        }
    };

    view! {
        <main class="page">
            {move || match detail() {
                None => view! { <Empty text="Revue introuvable.".into() /> }.into_any(),
                Some(view_model) => {
                    let review = view_model.review.clone();
                    let open = review.status == ReviewStatus::Open;
                    let status_label = if open { "Ouverte" } else { "Clôturée" };
                    let exposed = view_model.exposed.clone();
                    let consumed = view_model.consumed.clone();
                    let active = exposed.iter().filter(|row| row.active_versions > 0).count();
                    let scheduled = exposed
                        .iter()
                        .filter(|row| row.total_versions > 0 && row.active_versions == 0)
                        .count();
                    let breaking = exposed.iter().map(|row| row.breaking_versions).sum::<usize>();
                    let consumers = exposed.iter().map(|row| row.consumers).sum::<usize>();

                    view! {
                        <header class="page-head">
                            <div class="crumb">
                                <a href=format!("#/view/domains/{}", review.domain_id)>
                                    {view_model.domain_name.clone()}
                                </a>
                                <span>" / revue"</span>
                            </div>
                            <div class="title-row">
                                <h1>{review.object.name.clone()}</h1>
                                <span class=if open { "badge ok" } else { "badge archived" }>{status_label}</span>
                            </div>
                            <p class="muted small">
                                "Ouverte le "
                                {fmt_date(review.object.created_at)}
                                " · dernière modification "
                                {fmt_date(review.object.updated_at)}
                            </p>
                        </header>

                        <div class="toolbar">
                            <button class="btn" on:click=move |_| {
                                let target = if open { ReviewStatus::Closed } else { ReviewStatus::Open };
                                if app.run(repo().set_review_status(id, target)).is_some() {
                                    app.notify("Statut de la revue mis à jour.");
                                }
                            }>{if open { "Clôturer la revue" } else { "Rouvrir la revue" }}</button>
                        </div>

                        {move || {
                            if view_model.anomalies.is_empty() {
                                return ().into_any();
                            }
                            view! {
                                <section class="anomalies">
                                    <h2>"Anomalies"</h2>
                                    {view_model.anomalies.clone().into_iter().map(|anomaly| view! {
                                        <a class="anomaly" href=anomaly.href>
                                            <code>{anomaly.code.as_str()}</code>
                                            <span>{anomaly.message.clone()}</span>
                                        </a>
                                    }).collect_view()}
                                </section>
                            }.into_any()
                        }}

                        <div class="tabs">
                            {["summary", "exposed", "consumed", "calendar", "contracts", "consumers", "compare", "detail"]
                                .into_iter()
                                .map(|key| {
                                    let label = match key {
                                        "summary" => "Synthèse",
                                        "exposed" => "Exposées",
                                        "consumed" => "Consommées",
                                        "calendar" => "Calendrier",
                                        "contracts" => "Contrats",
                                        "consumers" => "Consommateurs",
                                        "compare" => "Comparaison",
                                        _ => "Détail",
                                    };
                                    view! {
                                        <button
                                            class=move || if tab.get() == key { "tab active" } else { "tab" }
                                            on:click=move |_| set_tab.set(key.to_string())
                                        >{label}</button>
                                    }
                                })
                                .collect_view()}
                        </div>

                        {move || match tab.get().as_str() {
                            "summary" => view! {
                                <div class="grid cards">
                                    <div class="card metric"><span class="metric-value">{active}</span><span>"interface(s) active(s)"</span></div>
                                    <div class="card metric"><span class="metric-value">{scheduled}</span><span>"versions en attente"</span></div>
                                    <div class="card metric"><span class="metric-value">{breaking}</span><span>"ruptures identifiées"</span></div>
                                    <div class="card metric"><span class="metric-value">{consumers}</span><span>"consommations déclarées"</span></div>
                                </div>
                            }.into_any(),
                            "exposed" => view! {
                                <InterfaceList rows=exposed.clone() empty="Aucune interface exposée.".into() />
                            }.into_any(),
                            "consumed" => view! {
                                <InterfaceList rows=consumed.clone() empty="Aucune interface consommée.".into() />
                            }.into_any(),
                            "calendar" => view! {
                                <div class="card">
                                    <h2>"Fenêtres de disponibilité"</h2>
                                    <table class="table">
                                        <thead><tr><th>"Interface"</th><th>"Version"</th><th>"Début"</th><th>"Fin (exclue)"</th><th>"Statut"</th></tr></thead>
                                        <tbody>
                                            {exposed.clone().into_iter().flat_map(|row| {
                                                let versions = repo().interface_detail(row.id).map(|d| d.versions).unwrap_or_default();
                                                versions.into_iter().map(move |summary| {
                                                    let name = row.name.clone();
                                                    view! {
                                                        <tr>
                                                            <td>{name}</td>
                                                            <td class="mono">{summary.version.semantic_version.clone()}</td>
                                                            <td>{date_or_dash(summary.version.available_from)}</td>
                                                            <td>{date_or_dash(summary.version.available_until)}</td>
                                                            <td><TemporalBadge status=summary.status /></td>
                                                        </tr>
                                                    }
                                                })
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                </div>
                            }.into_any(),
                            "contracts" => view! {
                                <div class="card">
                                    <h2>"Contrats et ruptures"</h2>
                                    <ul class="link-list">
                                        {exposed.clone().into_iter().map(|row| view! {
                                            <li>
                                                <a href=row.href>{row.name.clone()}</a>
                                                <span class="muted">
                                                    {format!("{} version(s) · {} rupture(s)", row.total_versions, row.breaking_versions)}
                                                </span>
                                            </li>
                                        }).collect_view()}
                                    </ul>
                                </div>
                            }.into_any(),
                            "consumers" => view! {
                                <div class="card">
                                    <h2>"Consommateurs par interface"</h2>
                                    <ul class="link-list">
                                        {exposed.clone().into_iter().map(|row| view! {
                                            <li>
                                                <a href=row.href>{row.name.clone()}</a>
                                                <span class="muted">{format!("{} consommateur(s)", row.consumers)}</span>
                                            </li>
                                        }).collect_view()}
                                    </ul>
                                </div>
                            }.into_any(),
                            "compare" => view! { <ComparePanel app=app id=id /> }.into_any(),
                            "detail" => view! {
                                <div class="card">
                                    <h2>"Détail"</h2>
                                    {if review.markdown_summary.trim().is_empty() {
                                        view! { <Empty text="Aucun résumé saisi.".into() /> }.into_any()
                                    } else {
                                        view! { <Markdown markdown=review.markdown_summary.clone() /> }.into_any()
                                    }}
                                </div>
                            }.into_any(),
                            _ => ().into_any(),
                        }}

                        <section class="card">
                            <h2>"Commentaires"</h2>
                            {if view_model.comments.is_empty() {
                                view! { <Empty text="Aucun commentaire. Une revue ne vaut pas approbation : elle enregistre des observations.".into() /> }.into_any()
                            } else {
                                view! {
                                    <ul class="comments">
                                        {view_model.comments.clone().into_iter().map(|comment| view! {
                                            <li>
                                                <span class="muted small">
                                                    {fmt_date(comment.created_at)}
                                                    " · "
                                                    {comment.author_id.clone()}
                                                </span>
                                                <Markdown markdown=comment.markdown.clone() />
                                            </li>
                                        }).collect_view()}
                                    </ul>
                                }.into_any()
                            }}
                            <div class="editor">
                                <textarea
                                    rows="3"
                                    placeholder="Observation en Markdown"
                                    prop:value=move || comment.get()
                                    on:input=move |ev| set_comment.set(input_value(&ev))
                                ></textarea>
                                <button class="btn primary" on:click=move |_| add_comment()>"Ajouter"</button>
                            </div>
                        </section>
                    }.into_any()
                }
            }}
        </main>
    }
}

/// Onglet **Comparaison** : choix d'une seconde revue et diff des ensembles.
///
/// La comparaison est strictement en lecture seule ; `REVIEW-001` est affiché
/// avec son code, en bandeau local et en notification.
#[component]
fn ComparePanel(app: App, id: ObjectId) -> impl IntoView {
    let candidates = repo()
        .reviews()
        .into_iter()
        .filter(|review| review.id != id)
        .collect::<Vec<_>>();
    let initial_target = candidates
        .first()
        .map(|review| review.id.to_string())
        .unwrap_or_default();
    let (target, set_target) = signal(initial_target);
    let (comparison, set_comparison) = signal(None::<ReviewComparison>);
    let (compare_error, set_compare_error) = signal(None::<String>);

    let compare = move || {
        let Some(other) = parse_object_id(&target.get()) else {
            set_compare_error.set(Some("Choisissez une revue à comparer.".into()));
            return;
        };
        match repo().compare_reviews(id, other) {
            Ok(result) => {
                set_compare_error.set(None);
                set_comparison.set(Some(result));
            }
            Err(error) => {
                set_comparison.set(None);
                set_compare_error.set(Some(format!("{} — {}", error.message, error.code)));
                app.fail(&error);
            }
        }
    };

    let options = candidates
        .iter()
        .map(|review| {
            let value = review.id.to_string();
            view! { <option value=value>{review.name.clone()}</option> }
        })
        .collect_view();

    view! {
        <div class="card">
            <h2>"Comparer avec une autre revue"</h2>
            {if candidates.is_empty() {
                view! {
                    <Empty text="Aucune autre revue à comparer.".into() />
                }.into_any()
            } else {
                view! {
                    <label class="field">
                        <span>"Revue à comparer"</span>
                        <select
                            prop:value=move || target.get()
                            on:change=move |ev| {
                                set_target.set(input_value(&ev));
                                set_comparison.set(None);
                                set_compare_error.set(None);
                            }
                        >
                            {options}
                        </select>
                    </label>
                    <div class="actions">
                        <button class="btn primary" on:click=move |_| compare()>"Comparer"</button>
                    </div>
                }.into_any()
            }}
            {move || compare_error.get().map(|message| view! {
                <p class="banner error inline">{message}</p>
            })}
            {move || comparison.get().map(|result| view! {
                <div class="compare">
                    <p class="muted small">
                        {result.from.name.clone()}
                        " → "
                        {result.to.name.clone()}
                    </p>
                    <dl class="props">
                        <div>
                            <dt>"Fenêtre de départ"</dt>
                            <dd>{window_label(result.window_from)}</dd>
                        </div>
                        <div>
                            <dt>"Fenêtre d'arrivée"</dt>
                            <dd>{window_label(result.window_to)}</dd>
                        </div>
                        <div>
                            <dt>"Filtres de départ"</dt>
                            <dd class="mono small">{compact_value(&result.filter_from)}</dd>
                        </div>
                        <div>
                            <dt>"Filtres d'arrivée"</dt>
                            <dd class="mono small">{compact_value(&result.filter_to)}</dd>
                        </div>
                    </dl>

                    <div class="two-cols">
                        <CompareRows
                            title="Exposées ajoutées".into()
                            rows=result.exposed_added.clone()
                            empty="Aucune interface exposée ajoutée.".into()
                        />
                        <CompareRows
                            title="Exposées retirées".into()
                            rows=result.exposed_removed.clone()
                            empty="Aucune interface exposée retirée.".into()
                        />
                        <CompareRows
                            title="Exposées modifiées".into()
                            rows=result.exposed_changed.clone()
                            empty="Aucune interface exposée modifiée.".into()
                        />
                        <CompareRows
                            title="Consommées ajoutées".into()
                            rows=result.consumed_added.clone()
                            empty="Aucune interface consommée ajoutée.".into()
                        />
                        <CompareRows
                            title="Consommées retirées".into()
                            rows=result.consumed_removed.clone()
                            empty="Aucune interface consommée retirée.".into()
                        />
                        <CompareRows
                            title="Consommées modifiées".into()
                            rows=result.consumed_changed.clone()
                            empty="Aucune interface consommée modifiée.".into()
                        />
                        <CompareAnomalies
                            title="Anomalies ajoutées".into()
                            rows=result.anomalies_added.clone()
                            empty="Aucune anomalie ajoutée.".into()
                        />
                        <CompareAnomalies
                            title="Anomalies retirées".into()
                            rows=result.anomalies_removed.clone()
                            empty="Aucune anomalie retirée.".into()
                        />
                    </div>
                </div>
            })}
        </div>
    }
    .into_any()
}

#[component]
fn InterfaceList(rows: Vec<archifier_repo::InterfaceRow>, empty: String) -> impl IntoView {
    if rows.is_empty() {
        return view! { <Empty text=empty /> }.into_any();
    }
    view! {
        <div class="card">
            <table class="table">
                <thead>
                    <tr><th>"Interface"</th><th>"Fournisseur"</th><th>"Statut"</th><th>"Versions"</th><th>"Consommateurs"</th></tr>
                </thead>
                <tbody>
                    {rows.into_iter().map(|row| view! {
                        <tr>
                            <td><a href=row.href>{row.name.clone()}</a></td>
                            <td>{row.provider_domain_name.clone()}</td>
                            <td><TemporalBadge status=row.status /></td>
                            <td>{format!("{} / {} actives", row.total_versions, row.active_versions)}</td>
                            <td>{row.consumers}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }.into_any()
}

/// Section de comparaison : lignes d'interface ajoutées, retirées ou modifiées.
#[component]
fn CompareRows(title: String, rows: Vec<InterfaceRow>, empty: String) -> impl IntoView {
    let count = rows.len();
    view! {
        <div class="compare-block">
            <h3>
                {title}
                <span class="muted small">" " {count}</span>
            </h3>
            {if rows.is_empty() {
                view! { <Empty text=empty /> }.into_any()
            } else {
                view! {
                    <ul class="link-list">
                        {rows.into_iter().map(|row| view! {
                            <li>
                                <a href=row.href>{row.name.clone()}</a>
                                <span class="muted small">{row.provider_domain_name.clone()}</span>
                                <TemporalBadge status=row.status />
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </div>
    }
    .into_any()
}

/// Section de comparaison : anomalies ajoutées ou retirées.
#[component]
fn CompareAnomalies(title: String, rows: Vec<Anomaly>, empty: String) -> impl IntoView {
    let count = rows.len();
    view! {
        <div class="compare-block">
            <h3>
                {title}
                <span class="muted small">" " {count}</span>
            </h3>
            {if rows.is_empty() {
                view! { <Empty text=empty /> }.into_any()
            } else {
                view! {
                    <ul class="link-list">
                        {rows.into_iter().map(|anomaly| view! {
                            <li>
                                <a class="anomaly" href=anomaly.href>
                                    <code>{anomaly.code.as_str()}</code>
                                    <span>{anomaly.message.clone()}</span>
                                    {anomaly.detail.map(|detail| view! {
                                        <span class="muted small">{detail}</span>
                                    })}
                                </a>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </div>
    }
    .into_any()
}
