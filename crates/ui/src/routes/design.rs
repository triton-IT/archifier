//! `/design/:id` (conception) et `/view/diagrams/:id` (lecture).
//!
//! Toute mutation passe par `design::Designer`, donc par le bus de commandes.

use crate::canvas::Canvas;
use crate::design::commands::{self, Designer};
use crate::design::history::SaveState;
use crate::design::{Selection, Tool};
use crate::i18n::{t, tf};
use crate::inspector::Inspector;
use crate::router::{Route, ViewLink};
use crate::state::{download_text, new_object_id, parse_date, repo, App};
use archifier_core::{
    DiagramFraming, DraftZone, StoryBlock, ViewCommandBody, ViewFraming, ZoneKind,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// `/design/:id` (conception) et `/view/diagrams/:id` (lecture).
///
/// Toute mutation passe par `design::Designer`, donc par le bus de commandes.
#[allow(non_snake_case)]
pub fn Design(app: App, id: archifier_core::ObjectId, reading: bool, link: ViewLink) -> AnyView {
    let Some(designer) = Designer::load(app, id) else {
        return view! {
            <main class="page">
                <h1>{t("state.not_found.title")}</h1>
                <p class="muted">{t("error.DGM-000")}</p>
                <a class="btn" href="#/">{t("state.not_found.atelier")}</a>
            </main>
        }
        .into_any();
    };

    let palette_open = RwSignal::new(false);
    let palette_query = RwSignal::new(String::new());
    let share_url = RwSignal::new(String::new());
    let menu_open = RwSignal::new(false);
    let main_ref = NodeRef::<leptos::html::Main>::new();

    // Cadrage partagé : appliqué localement, jamais réécrit dans le dépôt.
    let link_applied = StoredValue::new(false);
    let link_for_effect = link.clone();
    let designer_link = designer.clone();
    Effect::new(move |_| {
        if link_applied.get_value() {
            return;
        }
        link_applied.set_value(true);
        let link = link_for_effect.clone();
        if link.zoom.is_some() || link.x.is_some() || link.y.is_some() || !link.focus.is_empty() {
            designer_link.layout.update(|layout| {
                layout.framing = ViewFraming {
                    zoom: link.zoom,
                    x: link.x.unwrap_or(0.0),
                    y: link.y.unwrap_or(0.0),
                    focus: link.focus.clone(),
                    dim_others: link.dim,
                    caption: link.caption.clone(),
                };
            });
        }
        if let Some(cursor) = link.t {
            designer_link.layout.update(|layout| {
                layout.temporal_cursor = Some(cursor);
            });
        }
    });

    Effect::new(move |_| {
        if let Some(main) = main_ref.get() {
            let _ = main.focus();
        }
    });

    let designer_keys = designer.clone();
    let on_key = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        let lower = key.to_ascii_lowercase();
        let ctrl = ev.ctrl_key() || ev.meta_key();
        let typing = ev
            .target()
            .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
            .map(|element| {
                let tag = element.tag_name();
                matches!(tag.as_str(), "INPUT" | "TEXTAREA" | "SELECT")
            })
            .unwrap_or(false);

        if ctrl && lower == "k" {
            palette_open.update(|open| *open = !*open);
            ev.prevent_default();
            return;
        }
        if ctrl && lower == "s" {
            designer_keys.save_now();
            ev.prevent_default();
            return;
        }
        if ctrl && lower == "z" {
            if ev.shift_key() {
                designer_keys.redo();
            } else {
                designer_keys.undo();
            }
            ev.prevent_default();
            return;
        }
        if lower == "escape" {
            palette_open.set(false);
            designer_keys.link_from.set(None);
            designer_keys.link_to.set(None);
            designer_keys.tool.set(Tool::Select);
            ev.prevent_default();
            return;
        }
        if typing {
            return;
        }
        if lower == "f2" {
            designer_keys.focus_name.set(true);
            ev.prevent_default();
            return;
        }
        if lower == "delete" || lower == "backspace" {
            let selection = designer_keys.selection.get_untracked();
            if let Some(target) = commands::target_of_selection(&selection) {
                let _ = designer_keys.apply(ViewCommandBody::DeleteRepresentation { target });
                designer_keys.selection.set(Selection::None);
                ev.prevent_default();
            }
            return;
        }
        if let Some(tool) = Tool::from_key(&lower) {
            if tool == Tool::Frame {
                frame_nodes(&designer_keys, None);
                designer_keys.tool.set(Tool::Select);
            } else {
                designer_keys.tool.set(tool);
            }
            ev.prevent_default();
        }
    };

    let intent = designer.intent();
    let domains = repo().domains();
    let selected_domain = designer
        .layout
        .get_untracked()
        .scope
        .domain_id
        .or_else(|| domains.first().map(|d| d.id));
    let save_label = move || match designer.save.get() {
        SaveState::Saved { revision } => {
            t("design.save.saved").replace("{revision}", &revision.to_string())
        }
        SaveState::Saving => t("design.save.saving"),
        SaveState::Dirty => t("design.save.dirty"),
        SaveState::Conflict => t("design.save.conflict"),
        SaveState::Error { code, .. } => format!("{} · {code}", t("design.save.error")),
    };
    let designer_date = designer.clone();
    let designer_today = designer.clone();
    let designer_error = designer.clone();
    let designer_draft_banner = designer.clone();
    let designer_zone_domain = designer.clone();
    let designer_zone_group = designer.clone();
    let designer_relation = designer.clone();
    let designer_read = designer.clone();
    let designer_inspector = designer.clone();
    let designer_conflict = designer.clone();
    let designer_palette = designer.clone();
    let designer_tools = designer.clone();
    let designer_banner = designer.clone();
    let designer_palette_panel = designer.clone();
    let designer_canvas = designer.clone();
    let designer_temporal = designer.clone();
    let temporal_value = move || -> String {
        designer_temporal
            .layout
            .get()
            .temporal_cursor
            .clone()
            .unwrap_or_default()
            .chars()
            .take(10)
            .collect()
    };
    let title = designer.title();
    let posture_href = if reading {
        let link = share_link(designer.view_id, &designer.layout.get_untracked());
        format!("#/design/{}{}", designer.view_id, link.to_query())
    } else {
        let link = share_link(designer.view_id, &designer.layout.get_untracked());
        format!("#/view/diagrams/{}{}", designer.view_id, link.to_query())
    };
    let posture_label = if reading {
        t("design.to_design")
    } else {
        t("design.to_reading")
    };

    let designer_publish = designer.clone();
    let designer_share = designer.clone();
    let designer_svg = designer.clone();
    let designer_md = designer.clone();
    let designer_story = designer.clone();
    let designer_review = designer.clone();
    let designer_withdraw = designer.clone();
    let intent_signal = intent;

    view! {
        <main class="design" tabindex="0" node_ref=main_ref on:keydown=on_key>
            <header class="design-bar">
                <a class="btn ghost small" href="#/" aria-label=t("nav.atelier")>"←"</a>
                <div class="design-title">
                    <h1>{title}</h1>
                    <span class="badge">{t(intent_signal.label_key())}</span>
                    <span class="muted small">{save_label}</span>
                </div>
                <div class="design-tools" role="toolbar" aria-label=t("design.tool.select")>
                    {Tool::all().into_iter().map(|tool| {
                        let d_active = designer_tools.clone();
                        let d_click = designer_tools.clone();
                        let active = move || d_active.tool.get() == tool;
                        view! {
                            <button
                                class="tool"
                                class:active=active
                                title=tool.label()
                                aria-pressed=move || active().to_string()
                                on:click=move |_| {
                                    if tool == Tool::Frame {
                                        frame_nodes(&d_click, None);
                                        d_click.tool.set(Tool::Select);
                                    } else {
                                        d_click.tool.set(tool);
                                    }
                                }
                            >{tool.label()}</button>
                        }
                    }).collect_view()}
                </div>
                <div class="design-actions">
                    <input
                        class="temporal-input"
                        type="date"
                        aria-label=t("design.temporal")
                        prop:value=temporal_value
                        on:change=move |ev| {
                            let value = crate::state::input_value(&ev);
                            let at = parse_date(&value);
                            if reading {
                                designer_date.layout.update(|layout| {
                                    layout.temporal_cursor = at.map(|value| {
                                        value
                                            .format(&time::format_description::well_known::Rfc3339)
                                            .unwrap_or_default()
                                    });
                                });
                            } else {
                                let _ = designer_date.apply(ViewCommandBody::SetTemporalCursor { at });
                            }
                        }
                    />
                    <button class="btn ghost small" on:click=move |_| {
                        let _ = designer_today.apply(ViewCommandBody::SetTemporalCursor { at: Some(crate::state::now()) });
                    }>{t("design.temporal.today")}</button>
                    <a class="btn ghost small" href=posture_href>{posture_label}</a>
                    {if !reading {
                        view! {
                            <button class="btn primary small" on:click=move |_| {
                                match designer_publish.apply(ViewCommandBody::PublishView {
                                    visibility: commands::publish_visibility(),
                                }) {
                                    Ok(()) => designer_publish.app.notify(t("design.status.published")),
                                    Err(_) => {}
                                }
                            }>{t("design.publish")}</button>
                        }.into_any()
                    } else { ().into_any() }}
                    <button class="btn ghost small" on:click=move |_| {
                        let url = absolute_share_link(designer_share.view_id, &designer_share.layout.get_untracked());
                        copy_to_clipboard(&url);
                        share_url.set(url);
                        designer_share.app.notify(t("design.shared"));
                    }>{t("design.share")}</button>
                    <button class="btn ghost small" on:click=move |_| menu_open.update(|v| *v = !*v)>
                        "⋯"
                    </button>
                </div>
                {move || {
                    let designer_svg = designer_svg.clone();
                    let designer_md = designer_md.clone();
                    let designer_story = designer_story.clone();
                    let designer_review = designer_review.clone();
                    let designer_withdraw = designer_withdraw.clone();
                    menu_open.get().then(move || view! {
                        <div class="design-menu">
                            <button class="btn ghost small" on:click=move |_| {
                                download_text(&commands::export_name(&designer_svg, "svg"), &commands::export_svg(&designer_svg));
                                menu_open.set(false);
                            }>{t("design.export_svg")}</button>
                            <button class="btn ghost small" on:click=move |_| {
                                download_text(&commands::export_name(&designer_md, "md"), &commands::export_markdown(&designer_md));
                                menu_open.set(false);
                            }>{t("design.export_md")}</button>
                            <button class="btn ghost small" on:click=move |_| {
                                add_to_story(&designer_story);
                                menu_open.set(false);
                            }>{t("library.stories")}</button>
                            <button class="btn ghost small" on:click=move |_| {
                                let _ = designer_withdraw.apply(ViewCommandBody::PublishView {
                                    visibility: archifier_core::Visibility::Private,
                                });
                                menu_open.set(false);
                            }>{t("design.withdraw")}</button>
                            <button class="btn ghost small" on:click=move |_| {
                                capture_in_review(&designer_review);
                                menu_open.set(false);
                            }>{t("library.reviews")}</button>
                        </div>
                    })
                }}
            </header>

            {move || designer_banner.error.get().map(|error| {
                let detail = error.detail.clone().unwrap_or_default();
                let message = if error.code == "DSG-003" {
                    let count = detail.split(',').filter(|part| !part.is_empty()).count();
                    tf("error.DSG-003", &[("count", count.to_string())])
                } else {
                    t(&error.message_key)
                };
                view! {
                    <div class="banner error" role="alert">
                        <strong>{error.code.clone()}</strong>
                        <span>{message}</span>
                        {if detail.is_empty() { ().into_any() } else {
                            view! { <span class="muted small">{detail}</span> }.into_any()
                        }}
                        <button class="btn ghost small" on:click=move |_| designer_error.error.set(None)>"×"</button>
                    </div>
                }
            })}

            {move || {
                let layout = designer_draft_banner.layout.get();
                if !layout.drafts.is_empty() && !reading {
                    view! {
                        <div class="banner info">
                            <span>{tf("design.drafts", &[("count", layout.drafts.len().to_string())])}</span>
                        </div>
                    }.into_any()
                } else { ().into_any() }
            }}

            <div class="design-body">
                {if reading { ().into_any() } else {
                    view! {
                        <aside class="palette" aria-label=t("design.palette")>
                            <h2>{t("design.palette")}</h2>
                            <input
                                class="palette-search"
                                placeholder=t("design.palette.search")
                                prop:value=move || palette_query.get()
                                on:input=move |ev| palette_query.set(crate::state::input_value(&ev))
                            />
                            {move || {
                                let query = palette_query.get().to_lowercase();
                                let groups = commands::palette(designer_palette_panel.intent(), designer_palette_panel.notation());
                                if groups.is_empty() {
                                    return view! { <p class="muted small">{t("design.palette.empty")}</p> }.into_any();
                                }
                                groups.into_iter().map(|group| {
                                    let entries = group.entries.into_iter().filter(|entry| {
                                        query.is_empty() || entry.label.to_lowercase().contains(&query)
                                    }).collect::<Vec<_>>();
                                    if entries.is_empty() {
                                        return ().into_any();
                                    }
                                    let designer_entry = designer_palette_panel.clone();
                                    view! {
                                        <section class="palette-group">
                                            <h3>{t(&format!("layer.{}", group.layer_id))}</h3>
                                            {entries.into_iter().map(|entry| {
                                                let designer = designer_entry.clone();
                                                view! {
                                                    <button class="palette-entry" on:click=move |_| {
                                                        let layout = designer_entry.layout.get_untracked();
                                                        let mut draft = commands::draft_shape(designer.intent(), layout.drafts.len());
                                                        draft.suggested_metaclass = Some(entry.metaclass.clone());
                                                        draft.suggested_domain_id = layout.scope.domain_id;
                                                        draft.zone_id = layout.zones.first().map(|z| z.id.clone());
                                                        let draft_id = draft.id.clone();
                                                        let _ = designer.apply(ViewCommandBody::PlaceDraft { draft });
                                                        designer.selection.set(Selection::Draft(draft_id));
                                                        designer.focus_name.set(true);
                                                        designer.tool.set(Tool::Select);
                                                    }>
                                                        <span class="glyph">{entry.glyph.clone()}</span>
                                                        <span>{entry.label.clone()}</span>
                                                    </button>
                                                }
                                            }).collect_view()}
                                        </section>
                                    }.into_any()
                                }).collect_view().into_any()
                            }}
                            <div class="palette-foot">
                                <button class="btn ghost small" on:click=move |_| {
                                    add_zone(designer_zone_domain.clone(), ZoneKind::Domain, selected_domain);
                                }>{t("design.zone.new_domain")}</button>
                                <button class="btn ghost small" on:click=move |_| {
                                    add_zone(designer_zone_group.clone(), ZoneKind::Group, None);
                                }>{t("design.zone.new_group")}</button>
                            </div>
                        </aside>
                    }.into_any()
                }}

                <div class="design-stage">
                    <Canvas designer=designer_canvas.clone() />
                    {move || relation_bar(&designer_relation)}
                </div>

                {if reading {
                    view! { <ReadPanel designer=designer_read.clone() /> }.into_any()
                } else {
                    view! { <Inspector designer=designer_inspector.clone() /> }.into_any()
                }}
            </div>

            {move || {
                if share_url.get().is_empty() { ().into_any() } else {
                    view! {
                        <div class="banner info">
                            <input readonly prop:value=move || share_url.get() />
                            <button class="btn ghost small" on:click=move |_| share_url.set(String::new())>"×"</button>
                        </div>
                    }.into_any()
                }
            }}

            {move || {
                let d = designer_conflict.clone();
                designer_conflict.save.get().eq(&SaveState::Conflict).then(move || conflict_dialog(d.clone()))
            }}
            {move || {
                let d = designer_palette.clone();
                palette_open.get().then(move || command_palette(d.clone(), palette_open, app))
            }}
        </main>
    }
    .into_any()
}

fn relation_bar(designer: &Designer) -> AnyView {
    let Some(from) = designer.link_from.get() else {
        return ().into_any();
    };
    let to = designer.link_to.get();
    let source_class = designer
        .layout
        .get()
        .nodes
        .iter()
        .find(|n| n.id == from)
        .and_then(|n| repo().element(n.object_id))
        .map(|e| e.metaclass)
        .unwrap_or_default();
    let target_class = to
        .as_ref()
        .and_then(|to| {
            designer
                .layout
                .get()
                .nodes
                .iter()
                .find(|n| &n.id == to)
                .and_then(|n| repo().element(n.object_id))
                .map(|e| e.metaclass)
        })
        .unwrap_or_default();
    let choices = commands::relation_choices(designer.notation(), &source_class, &target_class);
    let relation_type = RwSignal::new(
        choices
            .first()
            .map(|(id, _)| id.clone())
            .unwrap_or_default(),
    );
    let designer_confirm = designer.clone();
    let designer_cancel = designer.clone();
    let from_name = from.clone();
    let from_for_apply = from.clone();
    let to_name = to.clone().unwrap_or_default();
    view! {
        <div class="relation-bar" role="region" aria-label=t("design.connect.title")>
            <span class="small">
                {format!("{from_name} → {to_name}")}
            </span>
            {if choices.is_empty() {
                view! { <span class="badge danger">{t("design.connect.no_relation")}</span> }.into_any()
            } else {
                view! {
                    <select on:change=move |ev| relation_type.set(crate::state::input_value(&ev))>
                        {choices.iter().map(|(id, label)| view! {
                            <option value=id.clone()>{label.clone()}</option>
                        }).collect_view()}
                    </select>
                    <button class="btn primary small" on:click=move |_| {
                        if let Some(to) = designer_confirm.link_to.get_untracked() {
                            let from_node = designer_confirm.layout.get_untracked().nodes.iter().find(|n| n.id == from_for_apply).cloned();
                            let to_node = designer_confirm.layout.get_untracked().nodes.iter().find(|n| n.id == to).cloned();
                            if let (Some(from_node), Some(to_node)) = (from_node, to_node) {
                                let _ = designer_confirm.apply(ViewCommandBody::Connect {
                                    from: commands::endpoint_of(&from_for_apply, from_node.port.is_some()),
                                    to: commands::endpoint_of(&to, to_node.port.is_some()),
                                    relation_type: relation_type.get_untracked(),
                                    new_relationship_id: new_object_id(),
                                });
                            }
                        }
                        designer_confirm.link_from.set(None);
                        designer_confirm.link_to.set(None);
                    }>{t("design.connect.confirm")}</button>
                }.into_any()
            }}
            <button class="btn ghost small" on:click=move |_| {
                designer_cancel.link_from.set(None);
                designer_cancel.link_to.set(None);
            }>"×"</button>
        </div>
    }
    .into_any()
}

fn frame_nodes(designer: &Designer, focus: Option<String>) {
    let layout = designer.layout.get_untracked();
    let nodes: Vec<_> = layout
        .nodes
        .iter()
        .filter(|node| focus.as_ref().map(|id| &node.id == id).unwrap_or(true))
        .collect();
    if nodes.is_empty() {
        return;
    }
    let min_x = nodes.iter().map(|n| n.x).fold(f64::MAX, f64::min);
    let min_y = nodes.iter().map(|n| n.y).fold(f64::MAX, f64::min);
    let zoom = 0.9;
    let x = 40.0 - min_x * zoom;
    let y = 60.0 - min_y * zoom;
    let _ = designer.apply(ViewCommandBody::SetFraming {
        framing: ViewFraming {
            zoom: Some(zoom),
            x,
            y,
            focus: focus.into_iter().collect(),
            dim_others: false,
            caption: None,
        },
    });
}

fn add_zone(designer: Designer, kind: ZoneKind, domain: Option<archifier_core::ObjectId>) {
    let layout = designer.layout.get_untracked();
    let layer = layout
        .layers
        .iter()
        .find(|l| l.id == "application")
        .or_else(|| layout.layers.get(1))
        .or_else(|| layout.layers.first());
    let zone = DraftZone {
        id: format!("z{}", new_object_id()),
        kind,
        domain_id: if kind == ZoneKind::Domain {
            domain
        } else {
            None
        },
        label: if kind == ZoneKind::Domain {
            domain
                .and_then(|id| repo().domain(id).map(|d| d.object.name))
                .unwrap_or_else(|| t("design.zone.new_domain"))
        } else {
            t("design.zone.new_group")
        },
        x: 48.0,
        y: layer.map(|l| l.y + 24.0).unwrap_or(48.0),
        w: 420.0,
        h: 240.0,
        layer_id: layer.map(|l| l.id.clone()),
        boundary: kind == ZoneKind::Domain,
    };
    let _ = designer.apply(ViewCommandBody::CreateZone { zone });
}

fn conflict_dialog(designer: Designer) -> AnyView {
    let designer_reload = designer.clone();
    let designer_force = designer.clone();
    let designer_close = designer.clone();
    let local_revision = designer.revision.get();
    let remote_revision = repo()
        .diagram_detail(designer.view_id)
        .map(|d| d.diagram.object.revision)
        .unwrap_or(local_revision);
    view! {
        <div class="dialog-backdrop">
            <div class="dialog" role="dialog" aria-modal="true" aria-label=t("design.conflict.title")>
                <h2>{t("design.conflict.title")}</h2>
                <p>{t("design.conflict.body")}</p>
                <dl class="facts">
                    <dt>{t("design.conflict.local")}</dt><dd>{format!("révision {local_revision}")}</dd>
                    <dt>{t("design.conflict.remote")}</dt><dd>{format!("révision {remote_revision}")}</dd>
                </dl>
                <div class="actions">
                    <button class="btn" on:click=move |_| designer_reload.reload()>{t("design.conflict.reload")}</button>
                    <button class="btn" on:click=move |_| { designer_close.save.set(SaveState::Saved { revision: local_revision }); }>{t("design.conflict.compare")}</button>
                    <button class="btn primary" on:click=move |_| designer_force.force_save()>{t("design.conflict.force")}</button>
                </div>
            </div>
        </div>
    }
    .into_any()
}

fn command_palette(designer: Designer, palette_open: RwSignal<bool>, app: App) -> AnyView {
    let designer_share = designer.clone();
    view! {
        <div class="command-backdrop" on:click=move |_| palette_open.set(false)>
            <div class="command-palette" role="dialog" aria-modal="false" aria-label=t("command_palette.title")
                on:click=move |ev| ev.stop_propagation()>
                <input
                    autofocus
                    placeholder=t("command_palette.placeholder")
                    aria-label=t("command_palette.title")
                />
                <button class="command" on:click=move |_| {
                    palette_open.set(false);
                    app.navigate(Route::Atelier);
                }>{t("command_palette.new_view")}</button>
                <button class="command" on:click=move |_| {
                    palette_open.set(false);
                    app.navigate(Route::Library { tab: crate::router::LibraryTab::Views });
                }>{t("command_palette.open_view")}</button>
                <button class="command" on:click=move |_| {
                    palette_open.set(false);
                    app.navigate(Route::Search { q: String::new() });
                }>{t("command_palette.search_object")}</button>
                <button class="command" on:click=move |_| {
                    palette_open.set(false);
                    app.navigate(Route::ViewReader {
                        id: designer.view_id,
                        link: share_link(designer.view_id, &designer.layout.get_untracked()),
                    });
                }>{t("command_palette.toggle_posture")}</button>
                <button class="command" on:click=move |_| {
                    palette_open.set(false);
                    let _ = designer.apply(ViewCommandBody::PublishView { visibility: commands::publish_visibility() });
                }>{t("command_palette.publish")}</button>
                <button class="command" on:click=move |_| {
                    palette_open.set(false);
                    download_text(&commands::export_name(&designer_share, "svg"), &commands::export_svg(&designer_share));
                }>{t("command_palette.export")}</button>
            </div>
        </div>
    }
    .into_any()
}

#[component]
fn ReadPanel(designer: Designer) -> impl IntoView {
    view! {
        <aside class="inspector" aria-label=t("design.inspector")>
            <header class="inspector-head"><h2>{t("design.inspector")}</h2></header>
            {move || {
                let selection = designer.selection.get();
                match selection {
                    Selection::Node(id) => {
                        let facts = commands::node_facts(&designer.layout.get(), None);
                        facts.get(&id).cloned().map(|info| view! {
                            <section class="inspector-section">
                                <h3>{info.name.clone()}</h3>
                                <dl class="facts">
                                    <dt>{t("design.type")}</dt><dd>{info.type_label.clone()}</dd>
                                    <dt>{t("design.domain")}</dt><dd>{info.domain_name.clone()}</dd>
                                </dl>
                                <a class="btn ghost small" href=format!("#/view/elements/{}", designer.layout.get().nodes.iter().find(|n| n.id == id).map(|n| n.object_id).unwrap_or(archifier_core::ObjectId::from_parts(0,0)))>
                                    {t("library.open_reading")}
                                </a>
                            </section>
                        }.into_any()).unwrap_or_else(|| view! { <p class="muted small">{t("design.inspector.empty")}</p> }.into_any())
                    }
                    _ => view! { <p class="muted small">{t("design.inspector.empty")}</p> }.into_any(),
                }
            }}
        </aside>
    }
}

fn share_link(_id: archifier_core::ObjectId, layout: &archifier_core::DiagramLayout) -> ViewLink {
    ViewLink {
        zoom: layout.framing.zoom,
        x: Some(layout.framing.x),
        y: Some(layout.framing.y),
        focus: layout.framing.focus.clone(),
        dim: layout.framing.dim_others,
        caption: layout.framing.caption.clone(),
        t: layout.temporal_cursor.clone(),
    }
}

fn absolute_share_link(
    id: archifier_core::ObjectId,
    layout: &archifier_core::DiagramLayout,
) -> String {
    let origin = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default();
    format!(
        "{origin}/#/view/diagrams/{id}{}",
        share_link(id, layout).to_query()
    )
}

fn copy_to_clipboard(text: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let clipboard = window.navigator().clipboard();
    let text = text.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let promise = clipboard.write_text(&text);
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    });
}

fn add_to_story(designer: &Designer) {
    let Some(story) = repo().stories().first().cloned() else {
        designer.app.notify(t("library.empty"));
        return;
    };
    let Some(detail) = repo().story_detail(story.id) else {
        return;
    };
    let mut structure = detail.story.structure.clone();
    let block = StoryBlock::Diagram {
        id: format!("b{}", new_object_id()),
        diagram_id: designer.view_id,
        framing: DiagramFraming {
            zoom: designer
                .layout
                .get_untracked()
                .framing
                .zoom
                .map(|z| z as f32),
            focus_element_ids: vec![],
            dimmed_element_ids: vec![],
            caption: designer.layout.get_untracked().framing.caption.clone(),
        },
    };
    if let Some(chapter) = structure.chapters.last_mut() {
        chapter.blocks.push(block);
    } else {
        structure.chapters.push(archifier_core::Chapter {
            id: format!("c{}", new_object_id()),
            title: t("library.stories"),
            blocks: vec![block],
        });
    }
    match repo().save_story(story.id, structure) {
        Ok(()) => designer.app.notify(t("design.formalize.done")),
        Err(error) => designer.app.fail(&error),
    }
}

fn capture_in_review(designer: &Designer) {
    let Some(review) = repo().reviews().first().cloned() else {
        designer.app.notify(t("library.empty"));
        return;
    };
    let zoom = designer
        .layout
        .get_untracked()
        .framing
        .zoom
        .map(|value| format!("{value:.2}×"))
        .unwrap_or_else(|| "non défini".into());
    let markdown = format!(
        "Vue capturée : [{}](#/design/{}) — cadrage zoom {}.",
        designer.title(),
        designer.view_id,
        zoom
    );
    match repo().add_comment(review.id, &markdown) {
        Ok(()) => designer.app.notify(t("design.formalize.done")),
        Err(error) => designer.app.fail(&error),
    }
}

/// Utilisé par le routeur pour la fiche objet d'une relation.
pub fn relationship_label(id: archifier_core::ObjectId) -> String {
    repo()
        .relationship(id)
        .map(|relationship| {
            let source = repo()
                .object_ref(relationship.source_id)
                .map(|r| r.name)
                .unwrap_or_else(|| "?".into());
            let target = repo()
                .object_ref(relationship.target_id)
                .map(|r| r.name)
                .unwrap_or_else(|| "?".into());
            format!("{source} —{}→ {target}", relationship.relation_type)
        })
        .unwrap_or_default()
}
