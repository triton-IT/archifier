//! Tableau de bord d'un domaine : vue VISUALISER complète.

use crate::components::{
    date_or_dash, openapi_status_badge, openapi_status_code, openapi_status_label, Card, DiffTable,
    Empty, ExportMarkdownButton, LifecycleBadge, TemporalBadge,
};
use crate::state::{fmt_date, input_value, is_checked, read_text_file, repo, selected_file, App};
use archifier_core::{DiagramNotation, InterfaceCategory, Notation, ObjectId};
use archifier_repo::OpenApiImportPreview;
use leptos::prelude::*;

fn notation_of(value: &str) -> Notation {
    match value {
        "BPMN" => Notation::Bpmn,
        "UML" => Notation::Uml,
        _ => Notation::Archimate,
    }
}

#[component]
pub fn DomainView(app: App, id: ObjectId) -> impl IntoView {
    let overview = move || {
        let _ = app.version.get();
        repo().domain_overview(id).ok()
    };

    let (panel, set_panel) = signal(String::new());

    // Formulaire élément
    let (el_name, set_el_name) = signal(String::new());
    let (el_notation, set_el_notation) = signal("ARCHIMATE".to_string());
    let (el_class, set_el_class) = signal(String::new());
    let choices = move || {
        let _ = app.version.get();
        repo().template_choices(notation_of(&el_notation.get()))
    };
    let create_element = move || {
        let class = if el_class.get().is_empty() {
            choices()
                .first()
                .map(|(id, _)| id.clone())
                .unwrap_or_default()
        } else {
            el_class.get()
        };
        let name = el_name.get();
        if name.trim().is_empty() || class.is_empty() {
            app.notify("Nom et classe sont obligatoires.");
            return;
        }
        if app
            .run(repo().create_element(
                id,
                notation_of(&el_notation.get()),
                &class,
                &name,
                Default::default(),
            ))
            .is_some()
        {
            set_el_name.set(String::new());
            set_panel.set(String::new());
            app.notify("Élément créé.");
        }
    };

    // Formulaire interface
    let (if_name, set_if_name) = signal(String::new());
    let (if_key, set_if_key) = signal(String::new());
    let (if_cat, set_if_cat) = signal("APPLICATION".to_string());
    let create_interface = move || {
        let category = match if_cat.get().as_str() {
            "BUSINESS" => InterfaceCategory::Business,
            "TECHNICAL" => InterfaceCategory::Technical,
            _ => InterfaceCategory::Application,
        };
        let key = if if_key.get().trim().is_empty() {
            archifier_core::slugify(&if_name.get())
        } else {
            if_key.get()
        };
        if app
            .run(repo().create_interface(id, &if_name.get(), &key, category))
            .is_some()
        {
            set_if_name.set(String::new());
            set_if_key.set(String::new());
            set_panel.set(String::new());
            app.notify("Interface créée.");
        }
    };

    // Contrat OpenAPI (W1a) : aperçu sans écriture, puis application explicite.
    let (oas_text, set_oas_text) = signal(String::new());
    let (oas_preview, set_oas_preview) = signal(None::<OpenApiImportPreview>);
    let (oas_confirm, set_oas_confirm) = signal(false);
    let analyse_openapi = move || {
        let raw = oas_text.get();
        if raw.trim().is_empty() {
            app.notify("Collez un document OpenAPI ou choisissez un fichier .json / .yaml.");
            return;
        }
        match repo().import_openapi_preview(id, &raw) {
            Ok(preview) => {
                set_oas_confirm.set(false);
                set_oas_preview.set(Some(preview));
            }
            Err(error) => {
                set_oas_preview.set(None);
                app.fail(&error);
            }
        }
    };
    let apply_openapi = move || {
        let Some(preview) = oas_preview.get() else {
            return;
        };
        match repo().apply_openapi_import(&preview, oas_confirm.get()) {
            Ok(outcome) => {
                let semver = preview.semantic_version.clone();
                set_oas_preview.set(None);
                set_oas_text.set(String::new());
                set_oas_confirm.set(false);
                app.refresh();
                if outcome.written {
                    // Écriture réelle : publier l'empreinte aux autres onglets.
                    app.publish_write();
                    app.notify(format!("Contrat OpenAPI importé : version {semver}."));
                } else {
                    app.notify("Empreinte déjà importée : aucune écriture.");
                }
                app.navigate(crate::router::Route::Interface {
                    id: outcome.interface_id,
                    version: None,
                });
            }
            Err(error) => app.fail(&error),
        }
    };

    // Formulaire vue
    let (dg_name, set_dg_name) = signal(String::new());
    let (dg_notation, set_dg_notation) = signal("ARCHIMATE".to_string());
    let create_diagram = move || {
        let notation = match dg_notation.get().as_str() {
            "BPMN" => DiagramNotation::Bpmn,
            "UML" => DiagramNotation::Uml,
            "FREE" => DiagramNotation::Free,
            _ => DiagramNotation::Archimate,
        };
        if let Some(new_id) = app.run(repo().create_diagram(id, &dg_name.get(), notation)) {
            set_dg_name.set(String::new());
            set_panel.set(String::new());
            app.navigate(crate::router::Route::ViewEditor {
                id: new_id,
                link: Default::default(),
            });
        }
    };

    view! {
        <main class="page">
            {move || match overview() {
                None => view! { <Empty text="Domaine introuvable.".into() /> }.into_any(),
                Some(view_model) => {
                    let domain = view_model.domain.clone();
                    let class = format!("page-head domain-{}", domain.slug);
                    view! {
                        <header class=class>
                            <div class="title-row">
                                <h1>{domain.name.clone()}</h1>
                                <LifecycleBadge lifecycle=domain.lifecycle />
                            </div>
                            <p class="muted">{domain.description.clone()}</p>
                            <div class="counts">
                                <span>{domain.counts.elements} " éléments"</span>
                                <span>{domain.counts.interfaces} " interfaces"</span>
                                <span>{domain.counts.diagrams} " vues"</span>
                                <span>{domain.counts.reviews} " revues"</span>
                            </div>
                        </header>

                        <div class="toolbar">
                            <button class="btn primary" on:click=move |_| set_panel.set("element".into())>
                                "Nouvel élément"
                            </button>
                            <button class="btn" on:click=move |_| set_panel.set("interface".into())>
                                "Nouvelle interface"
                            </button>
                            <button class="btn" on:click=move |_| set_panel.set("diagram".into())>
                                "Nouvelle vue"
                            </button>
                            <button class="btn" on:click=move |_| set_panel.set("openapi".into())>
                                "Contrat OpenAPI"
                            </button>
                            <ExportMarkdownButton app=app id=id domain=true />
                        </div>

                        {move || match panel.get().as_str() {
                            "element" => view! {
                                <Card title="Créer un élément".into()>
                                    <div class="form-grid">
                                        <label class="field">
                                            <span>"Notation"</span>
                                            <select on:change=move |ev| {
                                                set_el_notation.set(input_value(&ev));
                                                set_el_class.set(String::new());
                                            }>
                                                <option value="ARCHIMATE" selected=true>"ArchiMate 3.2"</option>
                                                <option value="BPMN">"BPMN 2.0 Core"</option>
                                                <option value="UML">"UML 2.5.1 statique"</option>
                                            </select>
                                        </label>
                                        <label class="field">
                                            <span>"Classe du métamodèle"</span>
                                            <select on:change=move |ev| set_el_class.set(input_value(&ev))>
                                                {move || choices().into_iter().map(|(key, label)| view! {
                                                    <option value=key>{label}</option>
                                                }).collect_view()}
                                            </select>
                                        </label>
                                        <label class="field">
                                            <span>"Nom"</span>
                                            <input
                                                prop:value=move || el_name.get()
                                                on:input=move |ev| set_el_name.set(input_value(&ev))
                                            />
                                        </label>
                                    </div>
                                    <div class="actions">
                                        <button class="btn primary" on:click=move |_| create_element()>"Créer"</button>
                                        <button class="btn ghost" on:click=move |_| set_panel.set(String::new())>"Annuler"</button>
                                    </div>
                                </Card>
                            }.into_any(),
                            "interface" => view! {
                                <Card title="Créer une interface".into()>
                                    <div class="form-grid">
                                        <label class="field">
                                            <span>"Nom"</span>
                                            <input
                                                prop:value=move || if_name.get()
                                                on:input=move |ev| set_if_name.set(input_value(&ev))
                                                placeholder="Payment API"
                                            />
                                        </label>
                                        <label class="field">
                                            <span>"Clé logique (immuable)"</span>
                                            <input
                                                prop:value=move || if_key.get()
                                                on:input=move |ev| set_if_key.set(input_value(&ev))
                                                placeholder="payment-api"
                                            />
                                        </label>
                                        <label class="field">
                                            <span>"Catégorie"</span>
                                            <select on:change=move |ev| set_if_cat.set(input_value(&ev))>
                                                <option value="BUSINESS">"Métier"</option>
                                                <option value="APPLICATION" selected=true>"Applicatif"</option>
                                                <option value="TECHNICAL">"Technique"</option>
                                            </select>
                                        </label>
                                    </div>
                                    <div class="actions">
                                        <button class="btn primary" on:click=move |_| create_interface()>"Créer"</button>
                                        <button class="btn ghost" on:click=move |_| set_panel.set(String::new())>"Annuler"</button>
                                    </div>
                                </Card>
                            }.into_any(),
                            "diagram" => view! {
                                <Card title="Créer une vue".into()>
                                    <div class="form-grid">
                                        <label class="field">
                                            <span>"Nom"</span>
                                            <input
                                                prop:value=move || dg_name.get()
                                                on:input=move |ev| set_dg_name.set(input_value(&ev))
                                            />
                                        </label>
                                        <label class="field">
                                            <span>"Notation"</span>
                                            <select on:change=move |ev| set_dg_notation.set(input_value(&ev))>
                                                <option value="ARCHIMATE" selected=true>"ArchiMate"</option>
                                                <option value="BPMN">"BPMN"</option>
                                                <option value="UML">"UML"</option>
                                                <option value="FREE">"Libre (multi-notation)"</option>
                                            </select>
                                        </label>
                                    </div>
                                    <div class="actions">
                                        <button class="btn primary" on:click=move |_| create_diagram()>"Créer et ouvrir"</button>
                                        <button class="btn ghost" on:click=move |_| set_panel.set(String::new())>"Annuler"</button>
                                    </div>
                                </Card>
                            }.into_any(),
                            "openapi" => view! {
                                <Card title="Contrat OpenAPI".into()>
                                    <p class="muted small">
                                        "Analyse sans écriture : le document est comparé aux versions existantes avant toute application."
                                    </p>
                                    <div class="form-grid">
                                        <label class="field">
                                            <span>"Fichier .json / .yaml"</span>
                                            <input
                                                type="file"
                                                accept=".json,.yaml,.yml,application/json,application/yaml"
                                                on:change=move |ev| {
                                                    if let Some(file) = selected_file(&ev) {
                                                        read_text_file(&file, move |text| {
                                                            set_oas_preview.set(None);
                                                            set_oas_text.set(text);
                                                        });
                                                    }
                                                }
                                            />
                                        </label>
                                    </div>
                                    <div class="editor">
                                        <textarea
                                            rows="10"
                                            placeholder="Collez ici un document OpenAPI (JSON ou YAML)"
                                            prop:value=move || oas_text.get()
                                            on:input=move |ev| {
                                                set_oas_preview.set(None);
                                                set_oas_text.set(input_value(&ev));
                                            }
                                        ></textarea>
                                    </div>
                                    <div class="actions">
                                        <button class="btn primary" on:click=move |_| analyse_openapi()>
                                            "Analyser"
                                        </button>
                                        <button class="btn ghost" on:click=move |_| {
                                            set_panel.set(String::new());
                                            set_oas_preview.set(None);
                                        }>"Fermer"</button>
                                    </div>

                                    {move || oas_preview.get().map(|preview| {
                                        let requires_confirmation = preview.diff.requires_confirmation;
                                        let status = preview.status;
                                        let name = preview.interface_name.clone();
                                        let semver = preview.semantic_version.clone();
                                        let format = format!("{:?}", preview.format);
                                        let raw_bytes = preview.raw_bytes;
                                        let warnings = preview.warnings.clone();
                                        let diff = preview.diff.clone();
                                        view! {
                                            <div class="preview">
                                                <dl class="props">
                                                    <div>
                                                        <dt>"Statut"</dt>
                                                        <dd>
                                                            <span class=openapi_status_badge(status)>{openapi_status_label(status)}</span>
                                                            <span class="muted small mono">" "{openapi_status_code(status)}</span>
                                                        </dd>
                                                    </div>
                                                    <div><dt>"Nom proposé"</dt><dd>{name}</dd></div>
                                                    <div><dt>"Version"</dt><dd class="mono">{semver}</dd></div>
                                                    <div><dt>"Format"</dt><dd class="mono">{format}</dd></div>
                                                    <div><dt>"Taille du document"</dt><dd>{format!("{raw_bytes} octets")}</dd></div>
                                                </dl>

                                                {(!warnings.is_empty()).then(|| view! {
                                                    <div class="banner subtle">
                                                        <strong>"Avertissements"</strong>
                                                        <ul class="link-list">
                                                            {warnings.into_iter().map(|warning| view! {
                                                                <li>{warning}</li>
                                                            }).collect_view()}
                                                        </ul>
                                                    </div>
                                                })}

                                                <h3>"Diff du contrat"</h3>
                                                <DiffTable diff=diff />

                                                <label class="field check">
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || oas_confirm.get()
                                                        disabled=move || !requires_confirmation
                                                        on:change=move |ev| set_oas_confirm.set(is_checked(&ev))
                                                    />
                                                    <span>"Confirmer la rupture"</span>
                                                </label>
                                                {requires_confirmation.then(|| view! {
                                                    <p class="muted small">
                                                        "Cette analyse contient une rupture ou un changement inconnu : la confirmation est obligatoire pour appliquer."
                                                    </p>
                                                })}

                                                <div class="actions">
                                                    <button
                                                        class="btn primary"
                                                        disabled=move || requires_confirmation && !oas_confirm.get()
                                                        on:click=move |_| apply_openapi()
                                                    >"Appliquer"</button>
                                                </div>
                                            </div>
                                        }
                                    })}
                                </Card>
                            }.into_any(),
                            _ => ().into_any(),
                        }}

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
                                            <span>{anomaly.message}</span>
                                        </a>
                                    }).collect_view()}
                                </section>
                            }.into_any()
                        }}

                        <div class="two-cols">
                            <Card title="Interfaces exposées".into()>
                                <InterfaceTable rows=view_model.exposed_interfaces.clone() />
                            </Card>
                            <Card title="Interfaces consommées".into()>
                                <InterfaceTable rows=view_model.consumed_interfaces.clone() />
                            </Card>
                        </div>

                        <div class="two-cols">
                            <Card title="Dépendances sortantes".into()>
                                {if view_model.dependencies_out.is_empty() {
                                    view! { <Empty text="Aucune dépendance sortante.".into() /> }.into_any()
                                } else {
                                    view! {
                                        <ul class="link-list">
                                            {view_model.dependencies_out.clone().into_iter().map(|link| view! {
                                                <li>
                                                    <a href=format!("#/view/domains/{}", link.domain_id)>{link.domain_name.clone()}</a>
                                                    <span class="muted">{format!("{} dépendance(s) · {}", link.weight, link.via.join(", "))}</span>
                                                </li>
                                            }).collect_view()}
                                        </ul>
                                    }.into_any()
                                }}
                            </Card>
                            <Card title="Dépendances entrantes".into()>
                                {if view_model.dependencies_in.is_empty() {
                                    view! { <Empty text="Aucune dépendance entrante.".into() /> }.into_any()
                                } else {
                                    view! {
                                        <ul class="link-list">
                                            {view_model.dependencies_in.clone().into_iter().map(|link| view! {
                                                <li>
                                                    <a href=format!("#/view/domains/{}", link.domain_id)>{link.domain_name.clone()}</a>
                                                    <span class="muted">{format!("{} dépendance(s)", link.weight)}</span>
                                                </li>
                                            }).collect_view()}
                                        </ul>
                                    }.into_any()
                                }}
                            </Card>
                        </div>

                        <Card title="Vues publiées".into()>
                            {if view_model.published_diagrams.is_empty() {
                                view! { <Empty text="Aucune vue. Créez-en une pour commencer à modéliser.".into() /> }.into_any()
                            } else {
                                view! {
                                    <div class="grid cards small">
                                        {view_model.published_diagrams.clone().into_iter().map(|diagram| view! {
                                            <a class="mini-card" href=diagram.href>
                                                <strong>{diagram.name.clone()}</strong>
                                                <span class="muted small">{diagram.subtitle.clone().unwrap_or_default()}</span>
                                            </a>
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }}
                        </Card>

                        <div class="two-cols">
                            <Card title="Stories".into()>
                                {if view_model.stories.is_empty() {
                                    view! { <Empty text="Aucune story.".into() /> }.into_any()
                                } else {
                                    view! {
                                        <ul class="link-list">
                                            {view_model.stories.clone().into_iter().map(|story| view! {
                                                <li><a href=story.href>{story.name.clone()}</a></li>
                                            }).collect_view()}
                                        </ul>
                                    }.into_any()
                                }}
                            </Card>
                            <Card title="Revues".into()>
                                {if view_model.reviews.is_empty() {
                                    view! { <Empty text="Aucune revue.".into() /> }.into_any()
                                } else {
                                    view! {
                                        <ul class="link-list">
                                            {view_model.reviews.clone().into_iter().map(|review| view! {
                                                <li>
                                                    <a href=review.href>{review.name.clone()}</a>
                                                    <span class="muted">{review.subtitle.clone().unwrap_or_default()}</span>
                                                </li>
                                            }).collect_view()}
                                        </ul>
                                    }.into_any()
                                }}
                            </Card>
                        </div>
                    }.into_any()
                }
            }}
        </main>
    }
}

#[component]
fn InterfaceTable(rows: Vec<archifier_repo::InterfaceRow>) -> impl IntoView {
    if rows.is_empty() {
        return view! { <Empty text="Aucune interface.".into() /> }.into_any();
    }
    view! {
        <table class="table">
            <thead>
                <tr>
                    <th>"Interface"</th>
                    <th>"Statut"</th>
                    <th>"Versions"</th>
                    <th>"Consommateurs"</th>
                    <th>"Prochaine échéance"</th>
                </tr>
            </thead>
            <tbody>
                {rows.into_iter().map(|row| view! {
                    <tr>
                        <td>
                            <a href=row.href>{row.name.clone()}</a>
                            <div class="muted small">{format!("{} · {}", row.category.label(), row.logical_key)}</div>
                        </td>
                        <td><TemporalBadge status=row.status /></td>
                        <td>{row.total_versions}{" ("}{row.active_versions}{" actives)"}</td>
                        <td>{row.consumers}</td>
                        <td>{date_or_dash(row.next_change)}</td>
                    </tr>
                }).collect_view()}
            </tbody>
        </table>
    }.into_any()
}

/// Utilitaire partagé par les écrans : libellé de date pour un en-tête.
pub fn today_label() -> String {
    fmt_date(crate::state::now())
}
