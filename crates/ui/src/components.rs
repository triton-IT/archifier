//! Coquille applicative et primitives visuelles.
//!
//! Les primitives sont volontairement peu nombreuses et figées : les écrans les
//! consomment, ils ne les redéfinissent pas.

use crate::i18n::t;
use crate::router::Route;
use crate::state::{download_bundle, fmt_date, input_value, repo, storage_is_temporary, App};
use archifier_core::{
    ContractDiff, DiffClass, FieldDifference, Lifecycle, Notation, ObjectId, Revision,
    TemporalStatus,
};
use archifier_repo::{OpenApiImportStatus, RevisionDiff, RevisionMeta, TemporalWindow};
use leptos::prelude::*;

/// Nombre maximal de lignes du tableau de comparaison inter-onglets.
const EXTERNAL_DIFF_ROWS: usize = 100;

#[component]
pub fn Shell(app: App, children: Children) -> impl IntoView {
    view! {
        <div class="app">
            <header class="topbar">
                <a class="brand" href="#/">
                    <span class="brand-mark">"◧"</span>
                    <span>{t("app.name")}</span>
                </a>
                <span class="tagline">{t("app.tagline")}</span>
                <nav class="topnav">
                    <a
                        href="#/"
                        class:active=move || app.route.get().is_products()
                        aria-current=move || if app.route.get().is_products() { "page" } else { "false" }
                    >{t("nav.products")}</a>
                    <a
                        href="#/map"
                        class:active=move || app.route.get().is_map()
                        aria-current=move || if app.route.get().is_map() { "page" } else { "false" }
                    >{t("nav.map")}</a>
                    <a
                        href="#/atelier"
                        class:active=move || app.route.get().is_atelier()
                        aria-current=move || if app.route.get().is_atelier() { "page" } else { "false" }
                    >{t("nav.atelier")}</a>
                    <details class="nav-more">
                        <summary
                            aria-label="Plus d'espaces"
                            class:active=move || matches!(
                                app.route.get(),
                                Route::Library { .. }
                                    | Route::Control { .. }
                                    | Route::Search { .. }
                                    | Route::Settings
                            )
                        >"⋯"</summary>
                        <div class="nav-more-panel">
                            <a
                                href="#/library"
                                class:active=move || matches!(app.route.get(), Route::Library { .. })
                                aria-current=move || if matches!(app.route.get(), Route::Library { .. }) { "page" } else { "false" }
                            >{t("nav.library")}</a>
                            <a
                                href="#/control"
                                class:active=move || matches!(app.route.get(), Route::Control { .. })
                                aria-current=move || if matches!(app.route.get(), Route::Control { .. }) { "page" } else { "false" }
                            >{t("nav.control")}</a>
                            <a
                                href="#/search"
                                title="Recherche globale (Ctrl+K)"
                                class:active=move || matches!(app.route.get(), Route::Search { .. })
                                aria-current=move || if matches!(app.route.get(), Route::Search { .. }) { "page" } else { "false" }
                            >{t("nav.search")}<kbd class="nav-kbd">"⌘K"</kbd></a>
                            <a
                                href="#/settings"
                                class:active=move || matches!(app.route.get(), Route::Settings)
                                aria-current=move || if matches!(app.route.get(), Route::Settings) { "page" } else { "false" }
                            >{t("nav.settings")}</a>
                        </div>
                    </details>
                </nav>
                <UserPicker />
            </header>
            <Banners app=app />
            <ExternalNotice app=app />
            <div class="workspace">{children()}</div>
            <ToastHost app=app />
        </div>
    }
}

#[component]
fn UserPicker() -> impl IntoView {
    let users = repo().users();
    let current = move || repo().current_user();
    if users.len() <= 1 {
        return view! {
            <span class="user-chip" title="Utilisateur courant">
                {move || {
                    let id = current();
                    users
                        .iter()
                        .find(|u| u.id == id)
                        .map(|u| u.display_name.clone())
                        .unwrap_or_else(|| "Utilisateur".into())
                }}
            </span>
        }
        .into_any();
    }
    view! {
        <select
            class="user-select"
            title="Changer d'utilisateur"
            on:change=move |ev| {
                let value = crate::state::input_value(&ev);
                repo().set_current_user(value);
            }
        >
            {users.into_iter().map(|user| {
                let id = user.id.clone();
                let selected = user.id == repo().current_user();
                view! {
                    <option value=id.clone() selected=selected>{user.display_name.clone()}</option>
                }
            }).collect_view()}
        </select>
    }
    .into_any()
}

#[component]
fn Banners(app: App) -> impl IntoView {
    let evicted = move || {
        let _ = app.version.get();
        repo().storage_evicted()
    };
    let persist_error = move || {
        let _ = app.version.get();
        repo().persist_error()
    };
    view! {
        <div class="banners">
            {move || storage_is_temporary().then(|| view! {
                <div class="banner error">
                    <strong>"Stockage indisponible"</strong>
                    <span>
                        "Les modifications ne seront pas conservées après la fermeture. "
                        "Exportez régulièrement une sauvegarde depuis les réglages."
                    </span>
                </div>
            })}
            {move || persist_error().map(|message| view! {
                <div class="banner error">
                    <strong>"Persistance impossible"</strong>
                    <span>{message} " — vos modifications restent en mémoire. Exportez vos données depuis les réglages."</span>
                </div>
            })}
            {move || evicted().then(|| view! {
                <div class="banner error">
                    <strong>"Stockage évincé"</strong>
                    <span>"Le navigateur a supprimé les données locales. Importez une sauvegarde depuis les réglages."</span>
                </div>
            })}
        </div>
    }
}

#[component]
fn ToastHost(app: App) -> impl IntoView {
    view! {
        {move || app.toast.get().map(|toast| {
            let class = if toast.error { "toast error" } else { "toast ok" };
            view! {
                <div class=class role="status">
                    <span>{toast.message}</span>
                    <button class="icon-btn" on:click=move |_| app.toast.set(None)>"×"</button>
                </div>
            }
        })}
    }
}

/// Dialogue global d'écriture externe (T1-08, contrat §5).
///
/// Trois choix seulement : recharger, comparer, forcer. Le dialogue ne
/// déclenche aucune écriture et n'applique jamais l'état distant.
#[component]
fn ExternalNotice(app: App) -> impl IntoView {
    view! {
        {move || {
            let write = app.external_write.get();
            let note = app.external_note.get();
            let changes = app.external_diff.get();
            if write.is_none() && note.is_none() {
                return ().into_any();
            }

            let origin = write.as_ref().map(|write| {
                let tab: String = write.tab.chars().take(8).collect();
                format!(
                    "Autre onglet « {tab} », écriture à {}. Aucune donnée n'a été appliquée ici.",
                    clock_label(write.at_ms)
                )
            });

            let reload = move |_| {
                if let Some(window) = web_sys::window() {
                    // Aucun état local conservé : la page repart du snapshot.
                    let _ = window.location().reload();
                }
            };
            let compare = move |_| {
                app.external_diff.set(None);
                app.external_note.set(None);
                crate::tabs::request_snapshot();
            };
            let force = move |_| app.dismiss_external();
            let close = move |_| app.clear_external();

            let diff_view = match changes {
                Some(list) if list.is_empty() => view! {
                    <p class="muted small">
                        "Aucune différence de contenu : l'autre onglet a peut-être déjà été rechargé."
                    </p>
                }.into_any(),
                Some(list) => external_diff_view(list),
                None => ().into_any(),
            };

            view! {
                <section class="banner info" role="alertdialog" aria-label="Écriture externe">
                    <strong>"Écriture externe détectée"</strong>
                    {origin.map(|text| view! { <span>{text}</span> })}
                    {note.map(|text| view! { <span>{text}</span> })}
                    <div class="actions">
                        <button class="btn primary" on:click=reload>"Recharger"</button>
                        <button class="btn" on:click=compare>"Comparer"</button>
                        <button class="btn danger" on:click=force>"Forcer"</button>
                        <button class="btn ghost" on:click=close>"Fermer"</button>
                    </div>
                    {diff_view}
                </section>
            }.into_any()
        }}
    }
}

/// Tableau chemin / local / distant, plafonné et jamais appliqué.
fn external_diff_view(changes: Vec<FieldDifference>) -> AnyView {
    let total = changes.len();
    let shown: Vec<FieldDifference> = changes.into_iter().take(EXTERNAL_DIFF_ROWS).collect();
    let caption = if total > shown.len() {
        format!(
            "{total} changements détectés — {EXTERNAL_DIFF_ROWS} premiers affichés. Aucune fusion n'est proposée : rechargez ou forcez l'état local."
        )
    } else {
        format!("{total} changement(s) détecté(s). Aucune fusion n'est proposée : rechargez ou forcez l'état local.")
    };
    view! {
        <div class="diff">
            <p class="muted small">{caption}</p>
            <table class="table">
                <thead>
                    <tr>
                        <th>"Chemin"</th>
                        <th>"Local"</th>
                        <th>"Distant"</th>
                    </tr>
                </thead>
                <tbody>
                    {shown.into_iter().map(|change| {
                        let path = if change.path.is_empty() {
                            "(racine)".to_string()
                        } else {
                            change.path.clone()
                        };
                        view! {
                            <tr>
                                <td class="mono">{path}</td>
                                <td class="mono small">{compact_json(&change.before)}</td>
                                <td class="mono small">{compact_json(&change.after)}</td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

/// Heure locale de l'avis, à la minute (`HH:MM`).
fn clock_label(at_ms: u64) -> String {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(at_ms as f64));
    format!("{:02}:{:02}", date.get_hours(), date.get_minutes())
}

#[component]
pub fn Card(title: String, children: Children) -> impl IntoView {
    view! {
        <section class="card">
            <header class="card-head"><h2>{title}</h2></header>
            <div class="card-body">{children()}</div>
        </section>
    }
}

#[component]
pub fn Empty(text: String) -> impl IntoView {
    view! { <p class="empty">{text}</p> }
}

#[component]
pub fn PageHeader(title: String, subtitle: String) -> impl IntoView {
    view! {
        <header class="page-head">
            <h1>{title}</h1>
            <p class="muted">{subtitle}</p>
        </header>
    }
}

#[component]
pub fn TemporalBadge(status: Option<TemporalStatus>) -> impl IntoView {
    let (class, label) = match status {
        Some(TemporalStatus::Active) => ("badge active", "Active"),
        Some(TemporalStatus::Scheduled) => ("badge scheduled", "Planifiée"),
        Some(TemporalStatus::Expired) => ("badge expired", "Expirée"),
        None => ("badge none", "Sans version"),
    };
    view! { <span class=class>{label}</span> }
}

#[component]
pub fn LifecycleBadge(lifecycle: Lifecycle) -> impl IntoView {
    let (class, label) = match lifecycle {
        Lifecycle::Active => ("badge ok", "Actif"),
        Lifecycle::Archived => ("badge archived", "Archivé"),
    };
    view! { <span class=class>{label}</span> }
}

#[component]
pub fn Markdown(markdown: String) -> impl IntoView {
    let html = archifier_core::markdown::render(&markdown);
    if markdown.trim().is_empty() {
        return view! { <p class="empty">"Aucune documentation."</p> }.into_any();
    }
    view! { <div class="markdown" inner_html=html></div> }.into_any()
}

/// Libellé lisible d'une notation, avec la version du standard effectivement chargée.
pub fn notation_label(notation: Notation) -> &'static str {
    notation.label()
}

/// Libellé lisible d'une relation dans sa notation.
///
/// Le code du profil (`serving`, `realization`, …) n'est jamais affiché tel quel
/// dans l'interface : on retombe sur ce code uniquement si le profil ne fournit
/// pas de libellé.
pub fn relation_label(notation: Notation, relation_type: &str) -> String {
    repo()
        .metamodel()
        .profile_for(notation)
        .and_then(|profile| profile.relation(relation_type))
        .map(|relation| relation.label.clone())
        .unwrap_or_else(|| relation_type.to_string())
}

/// Famille de glyphe pour l'affichage (aucun jeu d'icônes normatif reproduit).
pub fn glyph_family(glyph: &str) -> &'static str {
    match glyph {
        "actor" => "glyph-actor",
        "event" | "event-start" | "event-end" => "glyph-event",
        "gateway-x" | "gateway-plus" | "gateway-o" => "glyph-gateway",
        "service" => "glyph-service",
        "interface" => "glyph-interface",
        "object" | "contract" | "product" | "deliverable" | "artifact" => "glyph-object",
        "process" | "subprocess" | "task" | "pool" | "lane" => "glyph-process",
        "component" | "node" | "device" | "network" | "path" | "store" => "glyph-component",
        "goal" | "driver" | "assessment" => "glyph-motivation",
        _ => "glyph-default",
    }
}

pub fn status_word(status: Option<TemporalStatus>) -> &'static str {
    match status {
        Some(TemporalStatus::Active) => "active",
        Some(TemporalStatus::Scheduled) => "planifiée",
        Some(TemporalStatus::Expired) => "expirée",
        None => "sans version",
    }
}

pub fn date_or_dash(date: Option<time::OffsetDateTime>) -> String {
    date.map(fmt_date).unwrap_or_else(|| "—".into())
}

/// Libellé d'une fenêtre temporelle de revue, bornes ouvertes comprises.
pub fn window_label(window: Option<TemporalWindow>) -> String {
    match window {
        Some(window) => format!(
            "{} → {}",
            date_or_dash(window.start),
            date_or_dash(window.end)
        ),
        None => "aucune fenêtre".into(),
    }
}

/// Valeur JSON compacte pour une cellule ou un aperçu de filtre.
pub fn compact_value(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "—".into())
}

/// Libellé français d'une classe de changement de contrat.
pub fn diff_class_label(class: DiffClass) -> &'static str {
    match class {
        DiffClass::Breaking => "Rupture",
        DiffClass::Unknown => "Inconnue",
        DiffClass::Compatible => "Compatible",
    }
}

/// Code technique d'une classe de changement (`BREAKING`, …).
pub fn diff_class_code(class: DiffClass) -> &'static str {
    match class {
        DiffClass::Breaking => "BREAKING",
        DiffClass::Unknown => "UNKNOWN",
        DiffClass::Compatible => "COMPATIBLE",
    }
}

/// Classe de badge associée à une classe de changement (styles existants).
pub fn diff_class_badge(class: DiffClass) -> &'static str {
    match class {
        DiffClass::Breaking => "badge expired",
        DiffClass::Unknown => "badge scheduled",
        DiffClass::Compatible => "badge ok",
    }
}

/// Libellé français d'un statut d'aperçu d'import OpenAPI.
pub fn openapi_status_label(status: OpenApiImportStatus) -> &'static str {
    match status {
        OpenApiImportStatus::New => "Nouveau contrat",
        OpenApiImportStatus::UpdateProposed => "Mise à jour proposée",
        OpenApiImportStatus::Unchanged => "Inchangé (empreinte déjà importée)",
    }
}

/// Code technique d'un statut d'import OpenAPI (`NEW`, …).
pub fn openapi_status_code(status: OpenApiImportStatus) -> &'static str {
    match status {
        OpenApiImportStatus::New => "NEW",
        OpenApiImportStatus::UpdateProposed => "UPDATE_PROPOSED",
        OpenApiImportStatus::Unchanged => "UNCHANGED",
    }
}

/// Classe de badge associée à un statut d'import OpenAPI.
pub fn openapi_status_badge(status: OpenApiImportStatus) -> &'static str {
    match status {
        OpenApiImportStatus::New => "badge cat",
        OpenApiImportStatus::UpdateProposed => "badge scheduled",
        OpenApiImportStatus::Unchanged => "badge none",
    }
}

/// Tableau partagé du diff de contrat : code, message, classe.
#[component]
pub fn DiffTable(diff: ContractDiff) -> impl IntoView {
    if diff.changes.is_empty() {
        return view! { <Empty text="Aucun changement détecté.".into() /> }.into_any();
    }
    view! {
        <table class="table">
            <thead>
                <tr>
                    <th>"Code"</th>
                    <th>"Changement"</th>
                    <th>"Classe"</th>
                </tr>
            </thead>
            <tbody>
                {diff.changes.into_iter().map(|change| view! {
                    <tr>
                        <td class="mono">{change.code.clone()}</td>
                        <td>
                            {change.message.clone()}
                            <div class="muted small mono">{change.path.clone()}</div>
                        </td>
                        <td>
                            <span class=diff_class_badge(change.class)>
                                {diff_class_label(change.class)}
                            </span>
                            <div class="muted small mono">{diff_class_code(change.class)}</div>
                        </td>
                    </tr>
                }).collect_view()}
            </tbody>
        </table>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// W1b — historique comparé et export Markdown
// ---------------------------------------------------------------------------

fn revision_label(meta: &RevisionMeta) -> String {
    format!(
        "r{} · {} · {}",
        meta.revision,
        fmt_date(meta.created_at),
        meta.summary
    )
}

/// Valeur JSON compacte pour une cellule de diff ; `—` quand le champ est absent.
fn compact_json(value: &Option<serde_json::Value>) -> String {
    match value {
        Some(value) => compact_value(value),
        None => "—".into(),
    }
}

/// Télécharge l'export Markdown d'un domaine ou d'un objet.
///
/// Le libellé distingue les deux portées ; l'erreur moteur (`EXPORT-001`,
/// `EXPORT-002`) est remontée telle quelle dans le bandeau d'échec.
#[component]
pub fn ExportMarkdownButton(app: App, id: ObjectId, domain: bool) -> impl IntoView {
    let export = move || {
        let result = if domain {
            repo().export_domain_markdown(id)
        } else {
            repo().export_object_markdown(id)
        };
        match result {
            Ok(bundle) => {
                let files = download_bundle(&bundle);
                app.notify(format!(
                    "Export Markdown « {} » : {} fichier(s) téléchargé(s).",
                    bundle.root, files
                ));
            }
            Err(error) => app.fail(&error),
        }
    };
    let label = if domain {
        "Exporter le domaine en Markdown"
    } else {
        "Exporter en Markdown"
    };
    view! { <button class="btn" on:click=move |_| export()>{label}</button> }
}

/// Panneau **Historique des révisions** : sélection de deux révisions
/// comparables et diff champ par champ. L'historique de la documentation reste
/// affiché à part (§5.3).
#[component]
pub fn RevisionCompare(app: App, id: ObjectId) -> impl IntoView {
    let revisions: Vec<RevisionMeta> = repo().comparable_revisions(id);
    let documentation: Vec<RevisionMeta> = repo().documentation_history(id);
    if revisions.is_empty() {
        return view! {
            <Card title="Historique des révisions".into()>
                <Empty text="Aucune révision comparable enregistrée.".into() />
            </Card>
        }
        .into_any();
    }

    let newest = revisions.first().map(|meta| meta.revision).unwrap_or(0);
    let previous = revisions.get(1).map(|meta| meta.revision).unwrap_or(newest);
    let (from, set_from) = signal(previous.to_string());
    let (to, set_to) = signal(newest.to_string());
    let (diff, set_diff) = signal(None::<RevisionDiff>);

    let compare = move || {
        let Ok(from_revision) = from.get().trim().parse::<Revision>() else {
            app.notify("Révision de départ invalide.");
            return;
        };
        let Ok(to_revision) = to.get().trim().parse::<Revision>() else {
            app.notify("Révision d'arrivée invalide.");
            return;
        };
        match repo().revision_diff(id, from_revision, to_revision) {
            Ok(result) => set_diff.set(Some(result)),
            Err(error) => {
                set_diff.set(None);
                app.fail(&error);
            }
        }
    };

    let choices_from = revisions.clone();
    let choices_to = revisions.clone();
    let from_selected = from.get();
    let to_selected = to.get();
    let all_revisions = revisions;

    view! {
        <Card title="Historique des révisions".into()>
            <div class="form-grid">
                <label class="field">
                    <span>"Révision de départ"</span>
                    <select
                        prop:value=move || from.get()
                        on:change=move |ev| {
                            set_from.set(input_value(&ev));
                            set_diff.set(None);
                        }
                    >
                        {choices_from.iter().map(|meta| {
                            let value = meta.revision.to_string();
                            let selected = value == from_selected;
                            view! {
                                <option value=value selected=selected>{revision_label(meta)}</option>
                            }
                        }).collect_view()}
                    </select>
                </label>
                <label class="field">
                    <span>"Révision d'arrivée"</span>
                    <select
                        prop:value=move || to.get()
                        on:change=move |ev| {
                            set_to.set(input_value(&ev));
                            set_diff.set(None);
                        }
                    >
                        {choices_to.iter().map(|meta| {
                            let value = meta.revision.to_string();
                            let selected = value == to_selected;
                            view! {
                                <option value=value selected=selected>{revision_label(meta)}</option>
                            }
                        }).collect_view()}
                    </select>
                </label>
            </div>
            <div class="actions">
                <button class="btn primary" on:click=move |_| compare()>
                    "Comparer les révisions"
                </button>
            </div>

            {move || diff.get().map(|result| {
                let empty = result.changes.is_empty();
                let from = result.from;
                let to = result.to;
                let changes = result.changes;
                let count = changes.len();
                view! {
                    <div class="diff">
                        <p class="muted small mono">
                            {"r"} {from} {" → r"} {to} {" · "} {count} {" champ(s) modifié(s)"}
                        </p>
                        {if empty {
                            view! { <Empty text="Aucune différence entre ces révisions.".into() /> }.into_any()
                        } else {
                            view! {
                                <table class="table">
                                    <thead>
                                        <tr>
                                            <th>"Chemin"</th>
                                            <th>"Avant"</th>
                                            <th>"Après"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {changes.into_iter().map(|change| {
                                            let path = if change.path.is_empty() {
                                                "(racine)".to_string()
                                            } else {
                                                change.path.clone()
                                            };
                                            view! {
                                                <tr>
                                                    <td class="mono">{path}</td>
                                                    <td class="mono small">{compact_json(&change.before)}</td>
                                                    <td class="mono small">{compact_json(&change.after)}</td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            }.into_any()
                        }}
                    </div>
                }
            })}

            <h3>"Révisions enregistrées"</h3>
            <ul class="timeline-list">
                {all_revisions.into_iter().map(|meta| view! {
                    <li>
                        <span class="rev">"r" {meta.revision}</span>
                        <span>{meta.summary.clone()}</span>
                        <span class="muted small">
                            {fmt_date(meta.created_at)}
                            {" · "}
                            {meta.created_by.clone()}
                        </span>
                    </li>
                }).collect_view()}
            </ul>

            <h3>"Historique de la documentation"</h3>
            {if documentation.is_empty() {
                view! { <Empty text="Aucune révision de documentation.".into() /> }.into_any()
            } else {
                view! {
                    <ul class="timeline-list">
                        {documentation.into_iter().map(|meta| view! {
                            <li>
                                <span class="rev">"r" {meta.revision}</span>
                                <span>{meta.summary.clone()}</span>
                                <span class="muted small">
                                    {fmt_date(meta.created_at)}
                                    {" · "}
                                    {meta.created_by.clone()}
                                </span>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </Card>
    }
    .into_any()
}
