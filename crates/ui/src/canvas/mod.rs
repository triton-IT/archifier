//! Rendu SVG de l'Atelier : bandes, zones, formes, ports, arêtes, croquis.
//!
//! Le canvas ne lit jamais le dépôt : les faits (`NodeFacts`, `EdgeLine`) sont
//! préparés par `design::commands`.

use crate::design::commands::{edge_lines, node_facts, EdgeLine, NodeFacts};
use crate::design::{Designer, Selection};
use crate::i18n::t;
use archifier_core::{BoundarySide, DiagramLayout, DraftShape, TemporalStatus};
use leptos::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
struct View {
    zoom: f64,
    x: f64,
    y: f64,
}

#[derive(Clone, PartialEq)]
struct Drag {
    id: String,
    pan: bool,
    start_x: f64,
    start_y: f64,
    dx: f64,
    dy: f64,
    zoom: f64,
}

#[component]
pub fn Canvas(designer: Designer) -> impl IntoView {
    let initial = designer.layout.get_untracked();
    let view = RwSignal::new(View {
        zoom: initial
            .framing
            .zoom
            .or(Some(initial.viewport.zoom))
            .unwrap_or(1.0),
        x: initial.framing.x + initial.viewport.x,
        y: initial.framing.y + initial.viewport.y,
    });
    let drag = RwSignal::new(None::<Drag>);

    let facts = Memo::new(move |_| {
        let layout = designer.layout.get();
        node_facts(&layout, temporal_at(&layout))
    });
    let edges = Memo::new(move |_| edge_lines(&designer.layout.get()));

    let designer_commit = designer.clone();
    let commit_move = move |state: Drag| {
        if state.dx.abs() > 0.5 || state.dy.abs() > 0.5 {
            let _ = designer_commit.apply(archifier_core::ViewCommandBody::MoveNode {
                node_id: state.id.clone(),
                delta: archifier_core::Point {
                    x: state.dx,
                    y: state.dy,
                },
            });
        }
    };

    let on_move = move |ev: web_sys::PointerEvent| {
        if let Some(mut state) = drag.get_untracked() {
            let cx = ev.client_x() as f64;
            let cy = ev.client_y() as f64;
            if state.pan {
                let dx = cx - state.start_x;
                let dy = cy - state.start_y;
                view.update(|v| {
                    v.x += dx;
                    v.y += dy;
                });
                state.start_x = cx;
                state.start_y = cy;
            } else {
                state.dx = (cx - state.start_x) / state.zoom;
                state.dy = (cy - state.start_y) / state.zoom;
            }
            drag.set(Some(state));
        }
    };

    let on_up = move |_ev: web_sys::PointerEvent| {
        if let Some(state) = drag.get_untracked() {
            if !state.pan {
                commit_move(state);
            }
        }
        drag.set(None);
    };

    let designer_bg = designer.clone();
    let background_down = move |ev: web_sys::PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        let zoom = view.get_untracked().zoom;
        drag.set(Some(Drag {
            id: String::new(),
            pan: true,
            start_x: ev.client_x() as f64,
            start_y: ev.client_y() as f64,
            dx: 0.0,
            dy: 0.0,
            zoom,
        }));
        designer_bg.selection.set(Selection::None);
        designer_bg.focus_name.set(false);
    };

    let on_wheel = move |ev: web_sys::WheelEvent| {
        if ev.ctrl_key() || ev.meta_key() {
            return;
        }
        ev.prevent_default();
        let delta = ev.delta_y();
        view.update(|v| {
            let next = if delta > 0.0 {
                v.zoom * 0.9
            } else {
                v.zoom / 0.9
            };
            v.zoom = next.clamp(0.25, 4.0);
        });
    };

    let designer_layers = designer.clone();
    let designer_zones = designer.clone();
    let designer_nodes = designer.clone();
    let designer_drafts = designer.clone();
    let designer_hint = designer.clone();

    let transform = move || {
        let v = view.get();
        format!("translate({} {}) scale({})", v.x, v.y, v.zoom)
    };
    let detail = move || view.get().zoom > 1.15;
    let aggregate = move || view.get().zoom < 0.55;

    view! {
        <div class="canvas-wrap">
            <svg
                class="canvas"
                role="application"
                aria-label=t("design.palette")
                on:pointermove=on_move
                on:pointerup=on_up
                on:pointerleave=move |_| drag.set(None)
                on:wheel=on_wheel
                on:pointerdown=background_down
            >
                <g transform=transform>
                    {move || render_layers(&designer_layers.layout.get(), detail(), aggregate())}
                    {move || render_zones(designer_zones.clone())}
                    {move || render_edges(edges.get())}
                    {move || render_nodes(designer_nodes.clone(), facts.get(), drag.get(), detail())}
                    {move || render_drafts(designer_drafts.clone())}
                </g>
                <defs>
                    <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5"
                        markerWidth="7" markerHeight="7" orient="auto-start-reverse">
                        <path d="M 0 0 L 10 5 L 0 10 z" fill="#6b7787"></path>
                    </marker>
                </defs>
            </svg>
            <div class="canvas-hint" aria-live="polite">
                {move || {
                    let layout = designer_hint.layout.get();
                    if layout.nodes.is_empty() && layout.drafts.is_empty() {
                        t("design.empty.sketch")
                    } else {
                        format!(
                            "{} · {}",
                            t("design.drafts").replace("{count}", &layout.drafts.len().to_string()),
                            t(designer_hint.intent().label_key())
                        )
                    }
                }}
            </div>
        </div>
    }
}

fn render_layers(layout: &DiagramLayout, _detail: bool, aggregate: bool) -> AnyView {
    layout
        .layers
        .clone()
        .into_iter()
        .map(|layer| {
            let tone = layer.tone.clone().unwrap_or_else(|| layer.id.clone());
            let class = format!("layer layer-{tone}");
            view! {
                <g class=class>
                    <rect x="0" y=fmt(layer.y) width="1600" height=fmt(layer.h) rx="8"></rect>
                    <text x="12" y=fmt(layer.y + 18.0) class="layer-label">{layer.label.clone()}</text>
                    {if aggregate {
                        view! { <text x="12" y=fmt(layer.y + 36.0) class="layer-count">"…"</text> }.into_any()
                    } else { ().into_any() }}
                </g>
            }
        })
        .collect_view()
        .into_any()
}

fn render_zones(designer: Designer) -> AnyView {
    let layout = designer.layout.get();
    let selection = designer.selection.get();
    layout
        .zones
        .into_iter()
        .map(|zone| {
            let zone_id = zone.id.clone();
            let selected = selection == Selection::Zone(zone_id.clone());
            let designer_zone = designer.clone();
            let class = format!(
                "zone {} {}",
                match zone.kind {
                    archifier_core::ZoneKind::Domain => "zone-domain",
                    archifier_core::ZoneKind::Group => "zone-group",
                },
                if selected { "selected" } else { "" }
            );
            let dashed = if zone.boundary { "none" } else { "6 4" };
            view! {
                <g
                    class=class
                    on:pointerdown=move |ev: web_sys::PointerEvent| {
                        ev.stop_propagation();
                        designer_zone.selection.set(Selection::Zone(zone_id.clone()));
                    }
                >
                    <rect
                        x=fmt(zone.x) y=fmt(zone.y) width=fmt(zone.w) height=fmt(zone.h)
                        rx="8"
                        stroke-dasharray=dashed
                    ></rect>
                    <text x=fmt(zone.x + 10.0) y=fmt(zone.y + 20.0) class="zone-label">{zone.label.clone()}</text>
                </g>
            }
        })
        .collect_view()
        .into_any()
}

fn render_edges(lines: Vec<EdgeLine>) -> AnyView {
    lines
        .into_iter()
        .map(|line| {
            let label = line.label.clone();
            view! {
                <g class="edge" class:dimmed=line.dimmed>
                    <line
                        x1=fmt(line.x1) y1=fmt(line.y1)
                        x2=fmt(line.x2) y2=fmt(line.y2)
                        marker-end="url(#arrow)"
                    ></line>
                    {if label.is_empty() { ().into_any() } else {
                        view! {
                            <text
                                x=fmt((line.x1 + line.x2) / 2.0)
                                y=fmt((line.y1 + line.y2) / 2.0 - 6.0)
                                class="edge-label"
                            >{label}</text>
                        }.into_any()
                    }}
                </g>
            }
        })
        .collect_view()
        .into_any()
}

fn render_nodes(
    designer: Designer,
    facts: HashMap<String, NodeFacts>,
    drag_state: Option<Drag>,
    detail: bool,
) -> AnyView {
    let layout = designer.layout.get();
    let selection = designer.selection.get();
    layout
        .nodes
        .into_iter()
        .map(|node| {
            let info = facts.get(&node.id).cloned().unwrap_or_else(|| NodeFacts {
                name: "?".into(),
                type_label: String::new(),
                domain_name: String::new(),
                glyph: "rect".into(),
                temporal: None,
                version_label: None,
                archived: false,
            });
            let node_id = node.id.clone();
            let node_down = node.id.clone();
            let node_click = node.id.clone();
            let node_dbl = node.id.clone();
            let selected = selection == Selection::Node(node_id.clone());
            let offset = drag_state
                .as_ref()
                .filter(|d| !d.pan && d.id == node_id)
                .map(|d| (d.dx, d.dy))
                .unwrap_or((0.0, 0.0));
            let class = format!(
                "node {} {} {}",
                temporal_class(info.temporal),
                if selected { "selected" } else { "" },
                if info.archived { "archived" } else { "" }
            );
            let designer_down = designer.clone();
            let designer_click = designer.clone();
            let designer_dbl = designer.clone();
            let name = info.name.clone();
            let type_label = info.type_label.clone();
            let domain_name = info.domain_name.clone();
            let version_label = info.version_label.clone().unwrap_or_default();
            let port = node.port.clone();
            let geometry = (node.x, node.y, node.w, node.h);
            let temporal = info.temporal;
            view! {
                <g
                    class=class
                    transform=format!("translate({} {})", geometry.0 + offset.0, geometry.1 + offset.1)
                    on:pointerdown=move |ev: web_sys::PointerEvent| {
                        ev.stop_propagation();
                        designer_down.selection.set(Selection::Node(node_down.clone()));
                        designer_down.focus_name.set(false);
                    }
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        designer_click.selection.set(Selection::Node(node_click.clone()));
                        if designer_click.tool.get_untracked() == crate::design::Tool::Relation {
                            if designer_click.link_from.get_untracked().is_none() {
                                designer_click.link_from.set(Some(node_click.clone()));
                            } else if designer_click.link_from.get_untracked().as_deref() != Some(node_click.as_str()) {
                                designer_click.link_to.set(Some(node_click.clone()));
                            }
                        }
                    }
                    on:dblclick=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        designer_dbl.selection.set(Selection::Node(node_dbl.clone()));
                        designer_dbl.focus_name.set(true);
                    }
                >
                    <rect width=fmt(geometry.2) height=fmt(geometry.3) rx="8" class="node-box"></rect>
                    {if let Some(port) = port {
                        let (cx, cy) = port_center(geometry, &port.side);
                        view! {
                            <g class="port">
                                <circle cx=fmt(cx) cy=fmt(cy) r="8"></circle>
                                <text x=fmt(cx + 12.0) y=fmt(cy + 4.0) class="port-version">{version_label}</text>
                            </g>
                        }.into_any()
                    } else { ().into_any() }}
                    <text x="10" y="22" class="node-title">{name}</text>
                    {if detail {
                        view! {
                            <>
                                <text x="10" y="40" class="node-type">{type_label}</text>
                                <text x="10" y="56" class="node-domain">{domain_name}</text>
                            </>
                        }.into_any()
                    } else { ().into_any() }}
                    {temporal.map(|status| view! {
                        <text x="10" y=fmt(geometry.3 - 8.0) class="node-state">{t(temporal_label(status))}</text>
                    })}
                </g>
            }
        })
        .collect_view()
        .into_any()
}

fn render_drafts(designer: Designer) -> AnyView {
    let layout = designer.layout.get();
    let selection = designer.selection.get();
    layout
        .drafts
        .into_iter()
        .map(|draft| render_draft(designer.clone(), draft, &selection))
        .collect_view()
        .into_any()
}

fn render_draft(designer: Designer, draft: DraftShape, selection: &Selection) -> AnyView {
    let draft_id = draft.id.clone();
    let selected = *selection == Selection::Draft(draft_id.clone());
    let class = format!("draft {}", if selected { "selected" } else { "" });
    let is_port = draft.shape == archifier_core::ShapeKind::Port;
    let designer_click = designer.clone();
    let designer_dbl = designer.clone();
    let down_id = draft.id.clone();
    let dbl_id = draft.id.clone();
    view! {
        <g
            class=class
            transform=format!("translate({} {})", draft.x, draft.y)
            on:pointerdown=move |ev: web_sys::PointerEvent| {
                ev.stop_propagation();
                designer_click.selection.set(Selection::Draft(down_id.clone()));
                designer_click.focus_name.set(false);
            }
            on:dblclick=move |ev: web_sys::MouseEvent| {
                ev.stop_propagation();
                designer_dbl.selection.set(Selection::Draft(dbl_id.clone()));
                designer_dbl.focus_name.set(true);
            }
        >
            {if is_port {
                view! { <circle cx="0" cy="0" r="9" class="port prospect"></circle> }.into_any()
            } else {
                view! { <rect width=fmt(draft.w) height=fmt(draft.h) rx="8" class="draft-box"></rect> }.into_any()
            }}
            <text x="8" y="20" class="draft-label">{draft.label.clone()}</text>
        </g>
    }
    .into_any()
}

fn temporal_at(layout: &DiagramLayout) -> Option<time::OffsetDateTime> {
    layout.temporal_cursor.as_ref().and_then(|raw| {
        time::OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339).ok()
    })
}

fn temporal_class(status: Option<TemporalStatus>) -> &'static str {
    match status {
        Some(TemporalStatus::Active) => "temporal-active",
        Some(TemporalStatus::Scheduled) => "temporal-scheduled",
        Some(TemporalStatus::Expired) => "temporal-expired",
        None => "",
    }
}

fn temporal_label(status: TemporalStatus) -> &'static str {
    match status {
        TemporalStatus::Active => "design.temporal.active",
        TemporalStatus::Scheduled => "design.temporal.scheduled",
        TemporalStatus::Expired => "design.temporal.expired",
    }
}

fn fmt(value: f64) -> String {
    format!("{value:.2}")
}

fn port_center(geometry: (f64, f64, f64, f64), side: &BoundarySide) -> (f64, f64) {
    let (x, y, w, h) = geometry;
    match side {
        BoundarySide::North => (x + w / 2.0, y),
        BoundarySide::South => (x + w / 2.0, y + h),
        BoundarySide::East => (x + w, y + h / 2.0),
        BoundarySide::West => (x, y + h / 2.0),
    }
}
