//! Fiche d'un élément : propriétés, relations, usages, documentation, historique.

use crate::components::{
    notation_label, relation_label, Card, Empty, ExportMarkdownButton, LifecycleBadge, Markdown,
    RevisionCompare,
};
use crate::state::{input_value, parse_object_id, repo, App};
use archifier_core::ObjectId;
use archifier_repo::ElementFilter;
use leptos::prelude::*;

#[component]
pub fn ElementView(app: App, id: ObjectId) -> impl IntoView {
    let detail = move || {
        let _ = app.version.get();
        repo().element_detail(id)
    };
    let (draft, set_draft) = signal(repo().documentation(id));
    let (editing, set_editing) = signal(false);

    let save = move || {
        if app
            .run(repo().set_documentation(id, &draft.get()))
            .is_some()
        {
            set_editing.set(false);
            app.notify("Documentation enregistrée.");
        }
    };

    let initial_parent = repo()
        .hierarchy(id)
        .ok()
        .and_then(|hierarchy| hierarchy.parent)
        .map(|parent| parent.id.to_string())
        .unwrap_or_default();
    let (parent_choice, set_parent_choice) = signal(initial_parent);
    let (hierarchy_error, set_hierarchy_error) = signal(None::<String>);

    let apply_parent = move || {
        let Some(parent) = parse_object_id(&parent_choice.get()) else {
            set_hierarchy_error.set(Some("Choisissez un parent dans la liste.".into()));
            return;
        };
        match repo().set_parent(id, Some(parent)) {
            Ok(()) => {
                set_hierarchy_error.set(None);
                set_parent_choice.set(parent.to_string());
                app.notify("Parent défini.");
                app.refresh();
            }
            Err(error) => {
                set_hierarchy_error.set(Some(format!("{} — {}", error.message, error.code)));
                app.fail(&error);
            }
        }
    };

    let detach = move || match repo().set_parent(id, None) {
        Ok(()) => {
            set_hierarchy_error.set(None);
            set_parent_choice.set(String::new());
            app.notify("Élément détaché de son parent.");
            app.refresh();
        }
        Err(error) => {
            set_hierarchy_error.set(Some(format!("{} — {}", error.message, error.code)));
            app.fail(&error);
        }
    };

    view! {
        <main class="page">
            {move || match detail() {
                None => view! { <Empty text="Élément introuvable.".into() /> }.into_any(),
                Some(view_model) => {
                    let element = view_model.element.clone();
                    let object = element.object.clone();
                    let notation = element.notation;
                    view! {
                        <header class="page-head">
                            <div class="crumb">
                                <a href=format!("#/view/domains/{}", object.responsible_domain_id)>
                                    {view_model.domain_name.clone()}
                                </a>
                                <span>" / élément"</span>
                            </div>
                            <div class="title-row">
                                <h1>{object.name.clone()}</h1>
                                <LifecycleBadge lifecycle=object.lifecycle />
                            </div>
                            <p class="muted">
                                {element.metaclass.clone()}
                                {" · "}
                                {notation_label(element.notation)}
                                {" · révision "}
                                {object.revision}
                            </p>
                        </header>

                        <div class="toolbar">
                            <ExportMarkdownButton app=app id=id domain=false />
                            <a class="btn" href=format!("#/graph/{}", object.id)>
                                "Graphe de dépendances"
                            </a>
                        </div>

                        <div class="two-cols">
                            <Card title="Propriétés".into()>
                                {if element.semantic_properties.is_empty() {
                                    view! { <Empty text="Aucune propriété déclarée.".into() /> }.into_any()
                                } else {
                                    view! {
                                        <dl class="props">
                                            {element.semantic_properties.clone().into_iter().map(|(key, value)| view! {
                                                <div>
                                                    <dt>{key}</dt>
                                                    <dd>{value.as_str().unwrap_or(&value.to_string()).to_string()}</dd>
                                                </div>
                                            }).collect_view()}
                                        </dl>
                                    }.into_any()
                                }}
                            </Card>
                            <Card title="Relations".into()>
                                {if view_model.outgoing.is_empty() && view_model.incoming.is_empty() {
                                    view! { <Empty text="Aucune relation.".into() /> }.into_any()
                                } else {
                                    view! {
                                        <div class="relations">
                                            <h3>"Sortantes"</h3>
                                            {view_model.outgoing.clone().into_iter().map(|rel| view! {
                                                <div class="relation">
                                                    <span class="rel-type">{relation_label(notation, &rel.relation_type)}</span>
                                                    <a href=rel.href>{rel.other_name.clone()}</a>
                                                </div>
                                            }).collect_view()}
                                            <h3>"Entrantes"</h3>
                                            {view_model.incoming.clone().into_iter().map(|rel| view! {
                                                <div class="relation">
                                                    <a href=rel.href>{rel.other_name.clone()}</a>
                                                    <span class="rel-type">{relation_label(notation, &rel.relation_type)}</span>
                                                </div>
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }}
                            </Card>
                        </div>

                        <Card title="Hiérarchie".into()>
                            {match repo().hierarchy(id) {
                                Err(error) => view! {
                                    <p class="banner error inline">
                                        {format!("{} — {}", error.message, error.code)}
                                    </p>
                                }.into_any(),
                                Ok(hierarchy) => {
                                    let ancestors = hierarchy
                                        .ancestors
                                        .iter()
                                        .rev()
                                        .cloned()
                                        .collect::<Vec<_>>();
                                    let parent = hierarchy.parent.clone();
                                    let parent_archived = hierarchy.parent_archived;
                                    let root = hierarchy.root.clone();
                                    let children = hierarchy.children.clone();
                                    let candidates = repo().elements(&ElementFilter {
                                        notation: Some(notation),
                                        ..Default::default()
                                    });
                                    view! {
                                        <p>
                                            <strong>"Parent : "</strong>
                                            {match parent {
                                                Some(parent) => view! {
                                                    <a href=parent.href>{parent.name.clone()}</a>
                                                }.into_any(),
                                                None => view! {
                                                    <span class="muted">"aucun (racine)"</span>
                                                }.into_any(),
                                            }}
                                            {parent_archived.then(|| view! {
                                                <span class="badge archived">"parent archivé"</span>
                                            })}
                                        </p>
                                        <p class="muted small">
                                            "Fil d'Ariane : "
                                            {ancestors.into_iter().map(|ancestor| view! {
                                                <a href=ancestor.href>{ancestor.name.clone()}</a>
                                                <span>" / "</span>
                                            }).collect_view()}
                                            <strong>{root.name.clone()}</strong>
                                        </p>

                                        <h3>"Enfants directs"</h3>
                                        {if children.is_empty() {
                                            view! {
                                                <Empty text="Aucun enfant direct.".into() />
                                            }.into_any()
                                        } else {
                                            view! {
                                                <ul class="link-list">
                                                    {children.into_iter().map(|node| view! {
                                                        <li>
                                                            <a href=node.object.href>
                                                                {node.object.name.clone()}
                                                            </a>
                                                            <span class="badge none">
                                                                {format!("profondeur {}", node.depth)}
                                                            </span>
                                                            {node.archived_parent.then(|| view! {
                                                                <span class="badge archived">
                                                                    "parent archivé"
                                                                </span>
                                                            })}
                                                        </li>
                                                    }).collect_view()}
                                                </ul>
                                            }.into_any()
                                        }}

                                        <label class="field">
                                            <span>"Nouveau parent"</span>
                                            <select
                                                prop:value=move || parent_choice.get()
                                                on:change=move |ev| {
                                                    set_parent_choice.set(input_value(&ev));
                                                    set_hierarchy_error.set(None);
                                                }
                                            >
                                                <option value="">"— Aucun parent —"</option>
                                                {candidates.into_iter()
                                                    .filter(|candidate| candidate.id != id)
                                                    .map(|candidate| {
                                                        let value = candidate.id.to_string();
                                                        view! {
                                                            <option value=value>
                                                                {candidate.name.clone()}
                                                            </option>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </select>
                                        </label>
                                        <div class="actions">
                                            <button
                                                class="btn primary"
                                                on:click=move |_| apply_parent()
                                            >
                                                "Définir le parent"
                                            </button>
                                            <button class="btn" on:click=move |_| detach()>
                                                "Détacher"
                                            </button>
                                        </div>
                                        {move || hierarchy_error.get().map(|message| view! {
                                            <p class="banner error inline">{message}</p>
                                        })}
                                    }.into_any()
                                }
                            }}
                        </Card>

                        <Card title="Documentation".into()>
                            <div class="actions">
                                <button class="btn" on:click=move |_| set_editing.update(|v| *v = !*v)>
                                    {move || if editing.get() { "Aperçu" } else { "Modifier" }}
                                </button>
                            </div>
                            {move || if editing.get() {
                                view! {
                                    <div class="editor">
                                        <textarea
                                            rows="10"
                                            prop:value=move || draft.get()
                                            on:input=move |ev| set_draft.set(input_value(&ev))
                                            placeholder="Markdown — liens internes : arch://element/01J…"
                                        ></textarea>
                                        <div class="actions">
                                            <button class="btn primary" on:click=move |_| save()>"Enregistrer"</button>
                                            <button class="btn ghost" on:click=move |_| set_editing.set(false)>"Annuler"</button>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <Markdown markdown=draft.get() /> }.into_any()
                            }}
                        </Card>

                        <div class="two-cols">
                            <Card title="Utilisé par".into()>
                                <ul class="link-list">
                                    <li><strong>"Vues : "</strong>
                                        {if view_model.usages.diagrams.is_empty() {
                                            view! { <span class="muted">"aucune"</span> }.into_any()
                                        } else {
                                            view_model.usages.diagrams.clone().into_iter().map(|d| view! {
                                                <a href=d.href>{d.name.clone()}</a>
                                            }).collect_view().into_any()
                                        }}
                                    </li>
                                    <li><strong>"Stories : "</strong>
                                        {if view_model.usages.stories.is_empty() {
                                            view! { <span class="muted">"aucune"</span> }.into_any()
                                        } else {
                                            view_model.usages.stories.clone().into_iter().map(|s| view! {
                                                <a href=s.href>{s.name.clone()}</a>
                                            }).collect_view().into_any()
                                        }}
                                    </li>
                                    <li><strong>"Interfaces : "</strong>
                                        {if view_model.usages.interfaces.is_empty() {
                                            view! { <span class="muted">"aucune"</span> }.into_any()
                                        } else {
                                            view_model.usages.interfaces.clone().into_iter().map(|i| view! {
                                                <a href=i.href>{i.name.clone()}</a>
                                            }).collect_view().into_any()
                                        }}
                                    </li>
                                </ul>
                            </Card>
                            <RevisionCompare app=app id=id />
                        </div>

                        <p class="muted small mono">
                            {"ID "} {object.id.to_string()}
                            {" · arch://element/"} {object.id.to_string()}
                        </p>
                    }.into_any()
                }
            }}
        </main>
    }
}
