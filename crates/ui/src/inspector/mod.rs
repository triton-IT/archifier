//! Inspecteur latéral non bloquant (VD-11).
//!
//! Il ne masque jamais le canevas, n'ouvre aucun modal et ne construit aucune
//! commande lui-même : la formalisation passe par `design::formalize`.

use crate::design::commands::{self, Designer, NodeFacts};
use crate::design::formalize::formalize_body;
use crate::design::Selection;
use crate::i18n::t;
use crate::state::{new_object_id, repo};
use archifier_core::{Attachment, BoundarySide, DraftShape, ShapeKind, Target, ViewCommandBody};
use leptos::html::Input;
use leptos::prelude::*;

#[component]
pub fn Inspector(designer: Designer) -> impl IntoView {
    let name_value = RwSignal::new(String::new());
    let type_value = RwSignal::new(String::new());
    let domain_value = RwSignal::new(String::new());
    let version_value = RwSignal::new("1.0.0".to_string());
    let doc_value = RwSignal::new(String::new());
    let focus_ref: NodeRef<Input> = NodeRef::new();
    let (port_interface, set_port_interface) = signal(String::new());

    let designer_effect = designer.clone();
    Effect::new(move |_| {
        let selection = designer_effect.selection.get();
        let layout = designer.layout.get();
        let (name, class, domain) = match &selection {
            Selection::Node(id) => {
                let facts = commands::node_facts(&layout, None);
                let info = facts.get(id).cloned().unwrap_or(NodeFacts {
                    name: String::new(),
                    type_label: String::new(),
                    domain_name: String::new(),
                    glyph: String::new(),
                    temporal: None,
                    version_label: None,
                    archived: false,
                });
                let element = designer_effect.detail().and_then(|_| {
                    layout
                        .nodes
                        .iter()
                        .find(|n| &n.id == id)
                        .and_then(|n| repo().element(n.object_id))
                });
                let class = element
                    .as_ref()
                    .map(|e| e.metaclass.clone())
                    .unwrap_or_default();
                let domain = element
                    .as_ref()
                    .map(|e| e.object.responsible_domain_id.to_string())
                    .unwrap_or_default();
                (info.name, class, domain)
            }
            Selection::Draft(id) => layout
                .drafts
                .iter()
                .find(|d| &d.id == id)
                .map(|d| {
                    (
                        d.label.clone(),
                        d.suggested_metaclass.clone().unwrap_or_default(),
                        d.suggested_domain_id
                            .map(|id| id.to_string())
                            .unwrap_or_default(),
                    )
                })
                .unwrap_or_default(),
            Selection::Zone(id) => layout
                .zones
                .iter()
                .find(|z| &z.id == id)
                .map(|z| {
                    (
                        z.label.clone(),
                        String::new(),
                        z.domain_id.map(|id| id.to_string()).unwrap_or_default(),
                    )
                })
                .unwrap_or_default(),
            _ => (String::new(), String::new(), String::new()),
        };
        name_value.set(name);
        type_value.set(class);
        domain_value.set(domain);
        if let Selection::Node(id) = designer_effect.selection.get() {
            let object_id = designer_effect
                .layout
                .get()
                .nodes
                .iter()
                .find(|n| n.id == id)
                .map(|n| n.object_id);
            if let Some(object_id) = object_id {
                doc_value.set(repo().documentation(object_id));
            }
        }
    });

    let designer_focus = designer.clone();
    Effect::new(move |_| {
        if designer_focus.focus_name.get() {
            if let Some(input) = focus_ref.get() {
                let _ = input.focus();
                let _ = input.select();
            }
            designer_focus.focus_name.set(false);
        }
    });

    let designer_close = designer.clone();
    view! {
        <aside class="inspector" aria-label=t("design.inspector")>
            <header class="inspector-head">
                <h2>{t("design.inspector")}</h2>
                <button
                    class="btn ghost small"
                    on:click=move |_| designer_close.selection.set(Selection::None)
                    aria-label=t("design.conflict.cancel")
                >"×"</button>
            </header>
            {move || {
                let selection = designer.selection.get();
                match selection {
                    Selection::None => view! {
                        <p class="muted small">{t("design.inspector.empty")}</p>
                    }.into_any(),
                    Selection::Draft(id) => {
                        let draft = designer.layout.get().drafts.iter().find(|d| d.id == id).cloned();
                        match draft {
                            Some(draft) => draft_panel(designer.clone(), draft, name_value, type_value, domain_value, version_value).into_any(),
                            None => view! { <p class="muted small">{t("design.inspector.empty")}</p> }.into_any(),
                        }
                    }
                    Selection::Node(id) => {
                        let node = designer.layout.get().nodes.iter().find(|n| n.id == id).cloned();
                        match node {
                            Some(node) => node_panel(designer.clone(), node, name_value, doc_value, focus_ref.clone()).into_any(),
                            None => view! { <p class="muted small">{t("design.inspector.empty")}</p> }.into_any(),
                        }
                    }
                    Selection::Zone(id) => {
                        let zone = designer.layout.get().zones.iter().find(|z| z.id == id).cloned();
                        match zone {
                            Some(zone) => zone_panel(designer.clone(), zone, name_value, port_interface, set_port_interface).into_any(),
                            None => view! { <p class="muted small">{t("design.inspector.empty")}</p> }.into_any(),
                        }
                    }
                    Selection::Edge(id) => {
                        let designer_edge = designer.clone();
                        let edge = designer.layout.get().edges.iter().find(|e| e.id == id).cloned();
                        match edge {
                            Some(edge) => {
                                let edge_id = edge.id.clone();
                                view! {
                                <section class="inspector-section">
                                    <h3>{t("design.connect.title")}</h3>
                                    <p class="small">{edge.relationship_id.to_string()}</p>
                                    <button class="btn danger small" on:click=move |_| {
                                        let _ = designer_edge.apply(ViewCommandBody::DeleteRepresentation {
                                            target: Target::Edge { edge_id: edge_id.clone() },
                                        });
                                        designer_edge.selection.set(Selection::None);
                                    }>{t("design.delete_representation")}</button>
                                </section>
                            }.into_any()
                            }
                            None => view! { <p class="muted small">{t("design.inspector.empty")}</p> }.into_any(),
                        }
                    }
                }
            }}
        </aside>
    }
}

fn draft_panel(
    designer: Designer,
    draft: DraftShape,
    name_value: RwSignal<String>,
    type_value: RwSignal<String>,
    domain_value: RwSignal<String>,
    version_value: RwSignal<String>,
) -> AnyView {
    let notation = designer.notation();
    let repository = repo();
    let profile = repository.metamodel().profile_for(notation);
    let mut types: Vec<(String, String)> = profile
        .map(|p| {
            p.elements
                .iter()
                .filter(|e| !e.is_abstract)
                .map(|e| (e.metaclass.clone(), e.label.clone()))
                .collect()
        })
        .unwrap_or_default();
    types.sort_by(|a, b| a.1.cmp(&b.1));
    let domains: Vec<(String, String)> = repo()
        .domains()
        .into_iter()
        .map(|d| (d.id.to_string(), d.name))
        .collect();
    let is_port = draft.shape == ShapeKind::Port;
    let draft_defaults = draft.clone();
    Effect::new(move |_| {
        if type_value.get_untracked().is_empty() {
            if let Some(metaclass) = &draft_defaults.suggested_metaclass {
                type_value.set(metaclass.clone());
            }
        }
        if domain_value.get_untracked().is_empty() {
            if let Some(domain) = draft_defaults.suggested_domain_id {
                domain_value.set(domain.to_string());
            }
        }
    });
    let draft_name = draft.clone();
    let draft_commit = draft.clone();
    let draft_formalize = draft.clone();
    let designer_name = designer.clone();
    let designer_formalize = designer.clone();

    view! {
        <section class="inspector-section">
            <h3>{t("design.status.sketch")}</h3>
            <label class="field">
                <span>{t("design.name")}</span>
                <input
                    prop:value=move || name_value.get()
                    on:input=move |ev| name_value.set(crate::state::input_value(&ev))
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            let _ = designer_name.apply(ViewCommandBody::Rename {
                                target: Target::Draft { draft_id: draft_commit.id.clone() },
                                label: name_value.get_untracked(),
                            });
                        }
                    }
                />
            </label>
            <label class="field">
                <span>{t("design.type")}</span>
                <select
                    prop:value=move || type_value.get()
                    on:change=move |ev| type_value.set(crate::state::input_value(&ev))
                >
                    <option value="">{t("design.type.none")}</option>
                    {types.into_iter().map(|(value, label)| view! {
                        <option value=value>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>{t("design.domain")}</span>
                <select
                    prop:value=move || domain_value.get()
                    on:change=move |ev| domain_value.set(crate::state::input_value(&ev))
                >
                    <option value="">{t("design.domain.none")}</option>
                    {domains.into_iter().map(|(value, label)| view! {
                        <option value=value>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            {if is_port {
                view! {
                    <label class="field">
                        <span>{t("design.version")}</span>
                        <input
                            prop:value=move || version_value.get()
                            on:input=move |ev| version_value.set(crate::state::input_value(&ev))
                        />
                    </label>
                }.into_any()
            } else { ().into_any() }}
            <p class="muted small">{t("design.formalize.hint")}</p>
            <div class="actions">
                <button class="btn primary" on:click=move |_| {
                    let class = type_value.get_untracked();
                    let domain = crate::state::parse_object_id(&domain_value.get_untracked());
                    let metaclass = if class.trim().is_empty() { None } else { Some(class) };
                    match formalize_body(
                        &draft_formalize,
                        metaclass,
                        domain,
                        Some(version_value.get_untracked()),
                        new_object_id(),
                        Some(new_object_id()),
                    ) {
                        Ok(body) => {
                            if designer_formalize.apply(body).is_ok() {
                                designer_formalize.selection.set(Selection::Node(draft_formalize.id.clone()));
                                designer_formalize.app.notify(t("design.formalize.done"));
                            }
                        }
                        Err(error) => designer_formalize.error.set(Some(error)),
                    }
                }>{t("design.formalize")}</button>
                <button class="btn ghost" on:click=move |_| {
                    let _ = designer.apply(ViewCommandBody::DeleteRepresentation {
                        target: Target::Draft { draft_id: draft_name.id.clone() },
                    });
                    designer.selection.set(Selection::None);
                }>{t("design.delete_representation")}</button>
            </div>
            <p class="muted small">{t("design.delete_hint")}</p>
        </section>
    }
    .into_any()
}

fn node_panel(
    designer: Designer,
    node: archifier_core::DiagramNode,
    name_value: RwSignal<String>,
    doc_value: RwSignal<String>,
    focus_ref: NodeRef<Input>,
) -> AnyView {
    let node_rename = node.clone();
    let node_delete = node.clone();
    let node_doc = node.object_id;
    let detail = repo().element_detail(node.object_id);
    let facts = commands::node_facts(&designer.layout.get(), None)
        .get(&node.id)
        .cloned();
    let designer_rename = designer.clone();
    let designer_delete = designer.clone();
    let designer_doc = designer.clone();
    let object_id = node.object_id;
    let mut links: Vec<(String, String)> = Vec::new();
    if let Some(detail) = &detail {
        for link in detail.outgoing.iter().take(5) {
            links.push((link.relation_type.clone(), link.other_name.clone()));
        }
        for link in detail.incoming.iter().take(5) {
            links.push((link.relation_type.clone(), link.other_name.clone()));
        }
    }
    let history = repo().history(object_id);
    let status = facts
        .as_ref()
        .map(|f| {
            if f.archived {
                t("design.status.withdrawn")
            } else {
                t("design.status.valid")
            }
        })
        .unwrap_or_else(|| t("design.status.formalized"));
    let type_label = facts
        .as_ref()
        .map(|f| f.type_label.clone())
        .unwrap_or_default();
    let domain_name = facts
        .as_ref()
        .map(|f| f.domain_name.clone())
        .unwrap_or_default();
    let archived = facts.as_ref().map(|f| f.archived).unwrap_or(false);

    view! {
        <section class="inspector-section">
            <h3>{status}</h3>
            <label class="field">
                <span>{t("design.name")}</span>
                <input
                    node_ref=focus_ref
                    prop:value=move || name_value.get()
                    on:input=move |ev| name_value.set(crate::state::input_value(&ev))
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            let _ = designer_rename.apply(ViewCommandBody::Rename {
                                target: Target::Node { node_id: node_rename.id.clone() },
                                label: name_value.get_untracked(),
                            });
                        }
                    }
                />
            </label>
            <dl class="facts">
                <dt>{t("design.type")}</dt><dd>{type_label}</dd>
                <dt>{t("design.domain")}</dt><dd>{domain_name}</dd>
                <dt>{t("design.version")}</dt>
                <dd>{facts.and_then(|f| f.version_label).unwrap_or_else(|| "—".into())}</dd>
            </dl>
            {if archived {
                view! { <p class="callout warn">{t("state.archived")}</p> }.into_any()
            } else { ().into_any() }}
            <div class="actions">
                <a class="btn ghost small" href=format!("#/view/elements/{}", object_id)>
                    {t("library.open_reading")}
                </a>
                <button class="btn danger small" on:click=move |_| {
                    let _ = designer_delete.apply(ViewCommandBody::DeleteRepresentation {
                        target: Target::Node { node_id: node_delete.id.clone() },
                    });
                    designer_delete.selection.set(Selection::None);
                }>{t("design.delete_representation")}</button>
            </div>
            <p class="muted small">{t("design.delete_hint")}</p>
        </section>
        <section class="inspector-section">
            <h3>{t("design.inspector.documentation")}</h3>
            <textarea
                rows="4"
                prop:value=move || doc_value.get()
                on:input=move |ev| doc_value.set(crate::state::input_value(&ev))
            ></textarea>
            <button class="btn small" on:click=move |_| {
                designer_doc.set_documentation(node_doc, &doc_value.get_untracked());
            }>{t("design.save")}</button>
        </section>
        <section class="inspector-section">
            <h3>{t("design.inspector.links")}</h3>
            {if links.is_empty() {
                view! { <p class="muted small">"—"</p> }.into_any()
            } else {
                view! {
                    <ul class="link-list">
                        {links.into_iter().map(|(kind, name)| view! {
                            <li><span class="badge">{kind}</span> " " {name}</li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>
        <section class="inspector-section">
            <h3>{t("design.inspector.history")}</h3>
            <p class="muted small">{format!("{} révisions", history.len())}</p>
        </section>
    }
    .into_any()
}

fn zone_panel(
    designer: Designer,
    zone: archifier_core::Zone,
    name_value: RwSignal<String>,
    port_interface: ReadSignal<String>,
    set_port_interface: WriteSignal<String>,
) -> AnyView {
    let designer_rename = designer.clone();
    let zone_id = zone.id.clone();
    let zone_geometry = (zone.x, zone.y, zone.w, zone.h);
    let interfaces = repo().interfaces(&Default::default());
    let designer_port = designer.clone();
    let zone_for_port = zone.clone();
    view! {
        <section class="inspector-section">
            <h3>{if zone.kind == archifier_core::ZoneKind::Domain { t("design.zone.new_domain") } else { t("design.zone.new_group") }}</h3>
            <label class="field">
                <span>{t("design.zone.label")}</span>
                <input
                    prop:value=move || name_value.get()
                    on:input=move |ev| name_value.set(crate::state::input_value(&ev))
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            let _ = designer_rename.apply(ViewCommandBody::Rename {
                                target: Target::Zone { zone_id: zone_id.clone() },
                                label: name_value.get_untracked(),
                            });
                        }
                    }
                />
            </label>
            <button class="btn ghost small" on:click=move |_| {
                let _ = designer.apply(ViewCommandBody::DeleteRepresentation {
                    target: Target::Zone { zone_id: zone.id.clone() },
                });
                designer.selection.set(Selection::None);
            }>{t("design.delete_representation")}</button>
        </section>
        {if zone_for_port.kind == archifier_core::ZoneKind::Domain && zone_for_port.boundary {
            view! {
                <section class="inspector-section">
                    <h3>{t("design.port.title")}</h3>
                    <select
                        prop:value=move || port_interface.get()
                        on:change=move |ev| set_port_interface.set(crate::state::input_value(&ev))
                    >
                        <option value="">{t("design.port.existing")}</option>
                        {interfaces.into_iter().map(|row| view! {
                            <option value=row.id.to_string()>{row.name.clone()}</option>
                        }).collect_view()}
                    </select>
                    <button class="btn small" on:click=move |_| {
                        let Some(interface_id) = crate::state::parse_object_id(&port_interface.get_untracked()) else {
                            return;
                        };
                        let (x, y, w, h) = zone_geometry;
                        let _ = designer_port.apply(ViewCommandBody::PlaceExisting {
                            object_id: interface_id,
                            at: archifier_core::Point {
                                x: x + w - 12.0,
                                y: y + h / 2.0 - 12.0,
                            },
                            size: archifier_core::Size { w: 24.0, h: 24.0 },
                            attachment: Some(Attachment {
                                zone_id: zone_for_port.id.clone(),
                                side: BoundarySide::East,
                                offset: 0.5,
                            }),
                        });
                    }>{t("design.port.attach")}</button>
                </section>
            }.into_any()
        } else { ().into_any() }}
    }
    .into_any()
}
