//! Gabarit commun des fiches : contexte système visuel et panneaux latéraux.
//!
//! Toute fiche d'objet construit un `FocusGraph` et obtient le même rendu :
//! l'objet au centre, ses objets liés autour, les propriétés dans les rails.

use archifier_core::ObjectId;
use leptos::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextSide {
    Inside,
    Left,
    Right,
}

#[derive(Clone, Debug)]
struct ContextNode {
    id: ObjectId,
    label: String,
    sublabel: String,
    color_key: String,
    route: String,
    side: ContextSide,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Clone, Debug)]
struct ContextEdge {
    path: String,
    title: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ContextVisual {
    width: f64,
    height: f64,
    center_x: f64,
    center_y: f64,
    center_w: f64,
    center_h: f64,
    center_label: String,
    center_meta: String,
    center_badge: Option<String>,
    empty_label: Option<String>,
    left_label: Option<String>,
    right_label: Option<String>,
    left_zone_y: Option<f64>,
    right_zone_y: Option<f64>,
    inside: Vec<ContextNode>,
    around: Vec<ContextNode>,
    edges: Vec<ContextEdge>,
    center_legend: String,
    inside_legend: Option<String>,
    around_legend: Option<String>,
}

impl ContextVisual {
    pub(crate) fn inside_count(&self) -> usize {
        self.inside.len()
    }

    pub(crate) fn around_count(&self) -> usize {
        self.around.len()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct FocusNode {
    pub(crate) id: ObjectId,
    pub(crate) label: String,
    pub(crate) sublabel: String,
    pub(crate) color_key: String,
    pub(crate) route: String,
}

#[derive(Clone, Debug)]
pub(crate) struct FocusLink {
    pub(crate) from: ObjectId,
    pub(crate) to: ObjectId,
    pub(crate) label: String,
}

pub(crate) struct FocusGraph {
    pub(crate) center_id: ObjectId,
    pub(crate) center_label: String,
    pub(crate) center_meta: String,
    pub(crate) center_badge: Option<String>,
    pub(crate) empty_label: Option<String>,
    pub(crate) inside: Vec<FocusNode>,
    pub(crate) left_label: Option<String>,
    pub(crate) left: Vec<FocusNode>,
    pub(crate) right_label: Option<String>,
    pub(crate) right: Vec<FocusNode>,
    pub(crate) links: Vec<FocusLink>,
    pub(crate) center_legend: String,
    pub(crate) inside_legend: Option<String>,
    pub(crate) around_legend: Option<String>,
}

fn navigate_hash(route: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_hash(route);
    }
}

fn shorten(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn context_anchor(
    id: ObjectId,
    toward: ObjectId,
    center_id: ObjectId,
    center_box: (f64, f64, f64, f64),
    nodes: &BTreeMap<ObjectId, ContextNode>,
) -> (f64, f64) {
    let (x, y, w, h) = center_box;
    let target_x = nodes
        .get(&toward)
        .map(|node| node.x + node.w / 2.0)
        .unwrap_or(x + w / 2.0);
    if id == center_id {
        return (if target_x < x + w / 2.0 { x } else { x + w }, y + h / 2.0);
    }
    let Some(node) = nodes.get(&id) else {
        return (x + w / 2.0, y + h / 2.0);
    };
    let anchor_x = match node.side {
        ContextSide::Left => node.x + node.w,
        ContextSide::Right => node.x,
        ContextSide::Inside => {
            if target_x > node.x + node.w / 2.0 {
                node.x + node.w
            } else {
                node.x
            }
        }
    };
    (anchor_x, node.y + node.h / 2.0)
}

pub(crate) fn build_focus_context(graph: FocusGraph) -> ContextVisual {
    let FocusGraph {
        center_id,
        center_label,
        center_meta,
        center_badge,
        empty_label,
        inside,
        left_label,
        left,
        right_label,
        right,
        links,
        center_legend,
        inside_legend,
        around_legend,
    } = graph;

    let center_x = 182.0;
    let center_y = 44.0;
    let center_w = 396.0;
    let header_h = 64.0;
    let padding = 18.0;
    let node_w = 172.0;
    let node_h = 62.0;
    let node_gap = 12.0;
    let columns = 2usize;
    let rows = inside.len().max(1).div_ceil(columns);
    let grid_h = if inside.is_empty() {
        52.0
    } else {
        rows as f64 * node_h + rows.saturating_sub(1) as f64 * node_gap
    };
    let center_h = header_h + padding + grid_h + padding;

    let mut inside_nodes = Vec::new();
    for (index, node) in inside.into_iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        inside_nodes.push(ContextNode {
            id: node.id,
            label: node.label,
            sublabel: node.sublabel,
            color_key: node.color_key,
            route: node.route,
            side: ContextSide::Inside,
            x: center_x + padding + column as f64 * (node_w + node_gap),
            y: center_y + header_h + row as f64 * (node_h + node_gap),
            w: node_w,
            h: node_h,
        });
    }

    let around_w = 150.0;
    let around_h = 58.0;
    let around_gap = 14.0;
    let stack_height = |count: usize| {
        if count == 0 {
            0.0
        } else {
            count as f64 * around_h + count.saturating_sub(1) as f64 * around_gap
        }
    };
    let left_height = stack_height(left.len());
    let right_height = stack_height(right.len());
    let left_top = center_y + ((center_h - left_height) / 2.0).max(0.0);
    let right_top = center_y + ((center_h - right_height) / 2.0).max(0.0);
    let along_left = !left.is_empty();
    let along_right = !right.is_empty();

    let mut around = Vec::new();
    for (index, node) in left.into_iter().enumerate() {
        around.push(ContextNode {
            id: node.id,
            label: node.label,
            sublabel: node.sublabel,
            color_key: node.color_key,
            route: node.route,
            side: ContextSide::Left,
            x: 8.0,
            y: left_top + index as f64 * (around_h + around_gap),
            w: around_w,
            h: around_h,
        });
    }
    for (index, node) in right.into_iter().enumerate() {
        around.push(ContextNode {
            id: node.id,
            label: node.label,
            sublabel: node.sublabel,
            color_key: node.color_key,
            route: node.route,
            side: ContextSide::Right,
            x: center_x + center_w + 24.0,
            y: right_top + index as f64 * (around_h + around_gap),
            w: around_w,
            h: around_h,
        });
    }

    let mut positions: BTreeMap<ObjectId, ContextNode> = BTreeMap::new();
    for node in inside_nodes.iter().chain(around.iter()) {
        positions.insert(node.id, node.clone());
    }

    let center_box = (center_x, center_y, center_w, center_h);
    let mut edges = Vec::new();
    for link in links {
        if link.from == link.to {
            continue;
        }
        let from_known = link.from == center_id || positions.contains_key(&link.from);
        let to_known = link.to == center_id || positions.contains_key(&link.to);
        if !from_known || !to_known {
            continue;
        }
        let (sx, sy) = context_anchor(link.from, link.to, center_id, center_box, &positions);
        let (tx, ty) = context_anchor(link.to, link.from, center_id, center_box, &positions);
        let mid = (sx + tx) / 2.0;
        let from_label = if link.from == center_id {
            center_label.clone()
        } else {
            positions
                .get(&link.from)
                .map(|node| node.label.clone())
                .unwrap_or_else(|| "?".into())
        };
        let to_label = if link.to == center_id {
            center_label.clone()
        } else {
            positions
                .get(&link.to)
                .map(|node| node.label.clone())
                .unwrap_or_else(|| "?".into())
        };
        edges.push(ContextEdge {
            path: format!(
                "M {sx:.1} {sy:.1} C {mid:.1} {sy:.1}, {mid:.1} {ty:.1}, {tx:.1} {ty:.1}"
            ),
            title: format!("{} — {} → {}", link.label, from_label, to_label),
        });
    }

    let content_bottom = (center_y + center_h)
        .max(left_top + left_height)
        .max(right_top + right_height);
    ContextVisual {
        width: 760.0,
        height: content_bottom + 28.0,
        center_x,
        center_y,
        center_w,
        center_h,
        center_label,
        center_meta,
        center_badge,
        empty_label,
        left_label: along_left.then_some(left_label).flatten(),
        right_label: along_right.then_some(right_label).flatten(),
        left_zone_y: along_left.then_some(left_top - 10.0),
        right_zone_y: along_right.then_some(right_top - 10.0),
        inside: inside_nodes,
        around,
        edges,
        center_legend,
        inside_legend,
        around_legend,
    }
}

pub(crate) fn focus_context_card(
    summary: String,
    visual: ContextVisual,
    action: Option<(String, String)>,
) -> AnyView {
    let ContextVisual {
        width,
        height,
        center_x,
        center_y,
        center_w,
        center_h,
        center_label,
        center_meta,
        center_badge,
        empty_label,
        left_label,
        right_label,
        left_zone_y,
        right_zone_y,
        inside,
        around,
        edges,
        center_legend,
        inside_legend,
        around_legend,
    } = visual;
    let has_inside = !inside.is_empty();
    let has_around = !around.is_empty();
    let inside_swatch = inside
        .first()
        .map(|node| node.color_key.clone())
        .unwrap_or_else(|| "run".into());

    view! {
        <section class="card entity-context" aria-labelledby="entity-context-title">
            <header class="card-head entity-context-head">
                <div>
                    <h2 id="entity-context-title">"Système · contexte visuel"</h2>
                    <p class="muted small">{summary}</p>
                </div>
                {action.map(|(href, label)| view! {
                    <a class="btn ghost small" href=href>{label}</a>
                })}
            </header>
            <div class="entity-context-canvas">
                <svg
                    class="entity-context-svg"
                    role="group"
                    aria-label=format!("Contexte système : {center_label}")
                    viewBox=format!("0 0 {width} {height}")
                    preserveAspectRatio="xMidYMid meet"
                >
                    <defs>
                        <marker
                            id="entity-context-arrow"
                            viewBox="0 0 10 10"
                            refX="9"
                            refY="5"
                            markerWidth="7"
                            markerHeight="7"
                            orient="auto-start-reverse"
                        >
                            <path d="M 0 0 L 10 5 L 0 10 z" class="map-arrow-head"></path>
                        </marker>
                    </defs>
                    {edges.into_iter().map(|edge| view! {
                        <g class="map-edge context-edge">
                            <path
                                class="map-edge-line"
                                d=edge.path
                                marker-end="url(#entity-context-arrow)"
                            ></path>
                            <title>{edge.title}</title>
                        </g>
                    }).collect_view()}
                    <g class="context-center">
                        <rect
                            x=center_x
                            y=center_y
                            width=center_w
                            height=center_h
                            rx="16"
                        ></rect>
                        <text
                            x=center_x + 18.0
                            y=center_y + 28.0
                            class="context-center-title"
                        >
                            {shorten(&center_label, 34)}
                        </text>
                        <text
                            x=center_x + 18.0
                            y=center_y + 47.0
                            class="context-center-sub"
                        >
                            {center_meta}
                        </text>
                        {center_badge.map(|badge| view! {
                            <text
                                x=center_x + center_w - 16.0
                                y=center_y + 28.0
                                text-anchor="end"
                                class="context-center-badge"
                            >
                                {badge}
                            </text>
                        })}
                    </g>
                    {left_label.map(|label| view! {
                        <text
                            x=8.0
                            y=left_zone_y.unwrap_or(center_y)
                            class="context-zone-label"
                        >
                            {label}
                        </text>
                    })}
                    {right_label.map(|label| view! {
                        <text
                            x=center_x + center_w + 24.0
                            y=right_zone_y.unwrap_or(center_y)
                            class="context-zone-label"
                        >
                            {label}
                        </text>
                    })}
                    {empty_label.filter(|_| !has_inside).map(|label| view! {
                        <text
                            x=center_x + center_w / 2.0
                            y=center_y + 110.0
                            text-anchor="middle"
                            class="context-empty"
                        >
                            {label}
                        </text>
                    })}
                    {inside.into_iter().map(context_node_view).collect_view()}
                    {around.into_iter().map(context_node_view).collect_view()}
                </svg>
            </div>
            <footer class="entity-context-legend">
                <span>
                    <i class="swatch context-swatch-center"></i>{center_legend}
                </span>
                {inside_legend.map(|label| view! {
                    <span><i class=format!("swatch c-{inside_swatch}")></i>{label}</span>
                })}
                {around_legend.map(|label| view! {
                    <span><i class="swatch c-external"></i>{label}</span>
                })}
                {(has_inside || has_around).then(|| view! {
                    <span><i class="context-line-swatch"></i>"Relation"</span>
                })}
                {(!has_around).then(|| view! {
                    <span class="muted">"Aucun objet lié visible"</span>
                })}
            </footer>
        </section>
    }
    .into_any()
}

fn context_node_view(node: ContextNode) -> AnyView {
    let label = shorten(&node.label, 22);
    let sublabel = shorten(&node.sublabel, 24);
    let aria = format!("{} — {} — ouvrir la fiche", node.label, node.sublabel);
    let route = node.route.clone();
    let key_route = node.route.clone();
    view! {
        <g
            class=format!("map-node context-node c-{}", node.color_key)
            role="link"
            tabindex="0"
            aria-label=aria
            on:click=move |_| navigate_hash(&route)
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Enter" || ev.key() == " " {
                    ev.prevent_default();
                    navigate_hash(&key_route);
                }
            }
        >
            <rect x=node.x y=node.y width=node.w height=node.h rx="9"></rect>
            <text x=node.x + 12.0 y=node.y + 24.0 class="map-node-label">{label}</text>
            <text x=node.x + 12.0 y=node.y + 42.0 class="map-node-sub">{sublabel}</text>
            <title>{node.label.clone()}</title>
        </g>
    }
    .into_any()
}

pub(crate) fn panel(title: &str, body: AnyView) -> AnyView {
    view! {
        <section class="card">
            <header class="card-head"><h2>{title.to_string()}</h2></header>
            <div class="card-body">{body}</div>
        </section>
    }
    .into_any()
}

pub(crate) fn focus_layout(left: Vec<AnyView>, center: AnyView, right: Vec<AnyView>) -> AnyView {
    view! {
        <div class="entity-focus">
            <aside class="entity-rail entity-rail-left">{left.into_iter().collect_view()}</aside>
            {center}
            <aside class="entity-rail entity-rail-right">{right.into_iter().collect_view()}</aside>
        </div>
    }
    .into_any()
}
