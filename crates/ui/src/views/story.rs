//! Lecteur de Story : parcours guidé, cadrage des vues, transitions.

use crate::components::Empty;
use crate::state::{fmt_date, repo, App};
use archifier_core::{DiagramFraming, DiagramLayout, ObjectId, Revision};
use leptos::prelude::*;
use std::collections::BTreeMap;
use time::OffsetDateTime;

#[component]
pub fn StoryView(app: App, id: ObjectId) -> impl IntoView {
    let playback = move || {
        let _ = app.version.get();
        repo().playback(id)
    };
    let (highlight, set_highlight) = signal(None::<String>);

    let pin = move || match repo().pin_story_revisions(id) {
        Ok(count) => {
            app.notify(format!("{count} révision(s) épinglée(s)."));
            app.refresh();
        }
        Err(error) => app.fail(&error),
    };

    let unpin = move || match repo().unpin_story_revisions(id) {
        Ok(()) => {
            app.notify("Récit dégelé : les révisions ne sont plus figées.");
            app.refresh();
        }
        Err(error) => app.fail(&error),
    };

    view! {
        <main class="page story">
            {move || match playback() {
                None => view! { <Empty text="Story introuvable.".into() /> }.into_any(),
                Some(story) => {
                    let pinned_count =
                        story.pinned_revisions.len() + story.pinned_versions.len();
                    let latest_pin = latest_pin_date(&story.pinned_revisions);
                    view! {
                        <header class="page-head">
                            <h1>{story.title.clone()}</h1>
                            <p class="muted">
                                {story.chapters.len()} " chapitres · lecture guidée"
                            </p>
                            {(pinned_count > 0).then(|| view! {
                                <p>
                                    <span class="badge archived">"Récit figé"</span>
                                    <span class="muted small">
                                        {format!(" · {pinned_count} épingle(s)")}
                                        {latest_pin
                                            .map(|date| format!(
                                                " · révision la plus récente : {}",
                                                fmt_date(date)
                                            ))
                                            .unwrap_or_default()}
                                    </span>
                                </p>
                            })}
                            <div class="actions">
                                <button class="btn primary" on:click=move |_| pin()>
                                    "Épingler les révisions"
                                </button>
                                <button class="btn" on:click=move |_| unpin()>"Dégeler"</button>
                            </div>
                        </header>

                        {story.chapters.into_iter().map(|chapter| view! {
                            <section class="chapter">
                                <h2>{chapter.title.clone()}</h2>
                                {chapter.blocks.into_iter().map(|block| {
                                    let block_id = block.block_id.clone();
                                    let dimmed_id = block_id.clone();
                                    let dimmed = move || {
                                        highlight
                                            .get()
                                            .map(|h| h != dimmed_id)
                                            .unwrap_or(false)
                                    };
                                    let class = move || if dimmed() { "story-block dimmed" } else { "story-block" };
                                    let title = block.title.clone();
                                    view! {
                                        <article class=class on:click=move |_| set_highlight.set(Some(block_id.clone()))>
                                            {match block.kind.as_str() {
                                                "DIAGRAM" => view! {
                                                    <div class="block-head">
                                                        <span class="badge cat">"Vue"</span>
                                                        <strong>{title}</strong>
                                                    </div>
                                                    {match block.diagram {
                                                        Some(diagram) => view! {
                                                            <MiniDiagram
                                                                layout=diagram.layout
                                                                zoom=block.zoom
                                                                focus=block.focus_element_ids.clone()
                                                                dimmed=block.dimmed_element_ids.clone()
                                                                caption=block.caption.clone()
                                                            />
                                                        }.into_any(),
                                                        None => view! { <Empty text="Vue supprimée.".into() /> }.into_any(),
                                                    }}
                                                }.into_any(),
                                                "ELEMENT" => view! {
                                                    <div class="block-head">
                                                        <span class="badge cat">"Élément"</span>
                                                        <strong>{title}</strong>
                                                    </div>
                                                    {block.element.map(|element| view! {
                                                        <p><a class="btn ghost" href=element.href>"Ouvrir la fiche"</a></p>
                                                    })}
                                                }.into_any(),
                                                "INTERFACE" => view! {
                                                    <div class="block-head">
                                                        <span class="badge cat">"Interface"</span>
                                                        <strong>{title}</strong>
                                                    </div>
                                                    {block.interface.map(|detail| {
                                                        let href = format!("#/view/interfaces/{}", detail.interface.object.id);
                                                        view! {
                                                            <p>
                                                                <a href=href>"Consulter l'interface"</a>
                                                                " — " {detail.provider_domain_name}
                                                            </p>
                                                        }
                                                    })}
                                                }.into_any(),
                                                "TRANSITION" => view! {
                                                    <button class="btn" on:click=move |_| {
                                                        if let Some(target) = block.transition_target.clone() {
                                                            set_highlight.set(Some(target));
                                                        }
                                                    }>
                                                        "→ " {block.transition_label.clone().unwrap_or_default()}
                                                    </button>
                                                }.into_any(),
                                                _ => view! {
                                                    {block.markdown_html.map(|html| view! {
                                                        <div class="markdown" inner_html=html></div>
                                                    })}
                                                }.into_any(),
                                            }}
                                        </article>
                                    }
                                }).collect_view()}
                            </section>
                        }).collect_view()}
                    }.into_any()
                }
            }}
        </main>
    }
}

/// Date de la révision épinglée la plus récente, si elle est traçable.
fn latest_pin_date(pins: &BTreeMap<ObjectId, Revision>) -> Option<OffsetDateTime> {
    pins.iter()
        .filter_map(|(object_id, revision)| {
            repo()
                .history(*object_id)
                .into_iter()
                .find(|meta| meta.revision == *revision)
                .map(|meta| meta.created_at)
        })
        .max()
}

/// Rendu compact et en lecture seule d'une vue, pour le storytelling.
///
/// Le fit est calculé sur le layout **non zoomé**, puis le zoom du cadrage est
/// appliqué autour du centre du viewport : l'auto-fit ne peut donc pas annuler
/// le zoom demandé (§7.5). Focus, atténuation et légende viennent du bloc.
#[component]
fn MiniDiagram(
    layout: DiagramLayout,
    zoom: f32,
    focus: Vec<ObjectId>,
    dimmed: Vec<ObjectId>,
    caption: Option<String>,
) -> impl IntoView {
    if layout.nodes.is_empty() {
        return view! { <Empty text="Vue vide.".into() /> }.into_any();
    }
    let objects: std::collections::HashMap<ObjectId, String> = repo()
        .all_objects()
        .into_iter()
        .map(|object| (object.id, object.name))
        .collect();

    let focus_count = focus.len();
    let dimmed_count = dimmed.len();
    let framing = DiagramFraming {
        zoom: Some(zoom),
        focus_element_ids: focus,
        dimmed_element_ids: dimmed,
        caption: caption.clone(),
    };
    let framed = archifier_core::apply_framing(&framing, &layout);

    // Fit sur les coordonnées brutes du layout, avant tout zoom.
    let min_x = layout.nodes.iter().map(|n| n.x).fold(f64::MAX, f64::min);
    let min_y = layout.nodes.iter().map(|n| n.y).fold(f64::MAX, f64::min);
    let max_x = layout
        .nodes
        .iter()
        .map(|n| n.x + n.w)
        .fold(f64::MIN, f64::max);
    let max_y = layout
        .nodes
        .iter()
        .map(|n| n.y + n.h)
        .fold(f64::MIN, f64::max);
    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);
    let fit = (720.0 / width).min(360.0 / height).min(1.0);

    let center_x = 370.0;
    let center_y = 190.0;
    let zoom = f64::from(framed.zoom);
    let framed_zoom = framed.zoom;
    // Le fit est appliqué aux coordonnées, le zoom par un transform : les deux
    // effets se composent au lieu de s'annuler.
    let transform = format!(
        "translate({center_x} {center_y}) scale({zoom}) translate(-{center_x} -{center_y})"
    );

    view! {
        <div class="mini-diagram-figure">
            <svg
                class="mini-diagram"
                width="740"
                height="380"
                viewBox="0 0 740 380"
                style="max-width:100%"
            >
                <g transform=transform>
                    {framed.nodes.into_iter().map(|node| {
                        // Les coordonnées du helper sont déjà zoomées : on
                        // revient au layout brut pour appliquer le fit ; le
                        // transform du groupe rejoue ensuite le zoom.
                        let raw_x = node.x / zoom;
                        let raw_y = node.y / zoom;
                        let raw_w = node.w / zoom;
                        let raw_h = node.h / zoom;
                        let x = (raw_x - min_x) * fit + 10.0;
                        let y = (raw_y - min_y) * fit + 10.0;
                        let w = raw_w * fit;
                        let h = raw_h * fit;
                        let name = objects.get(&node.object_id).cloned().unwrap_or_else(|| "—".into());
                        let class = if node.highlighted { "mini-node focused" } else { "mini-node" };
                        view! {
                            <g class=class style=format!("opacity:{:.2}", node.opacity)>
                                <rect x=format!("{x:.1}") y=format!("{y:.1}") width=format!("{w:.1}") height=format!("{h:.1}") rx="6"></rect>
                                <text x=format!("{:.1}", x + 8.0) y=format!("{:.1}", y + 20.0) class="node-title">{name}</text>
                            </g>
                        }
                    }).collect_view()}
                </g>
            </svg>
            <p class="muted small">
                {format!("Zoom {framed_zoom:.2}×")}
                {(focus_count > 0).then(|| format!(" · {focus_count} élément(s) mis en avant"))}
                {(dimmed_count > 0).then(|| format!(" · {dimmed_count} élément(s) atténué(s)"))}
            </p>
            {caption.map(|caption| view! { <p class="muted small">{caption}</p> })}
        </div>
    }.into_any()
}
