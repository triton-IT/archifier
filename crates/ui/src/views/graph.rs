//! Graphe de dépendances d'un élément ou d'une interface (`#/graph/<id>`).
//!
//! Aucun moteur de graphe propre à l'UI : l'écran ne fait que paramétrer et
//! afficher `Repository::dependencies`. La direction et la profondeur restent
//! en état local, conformément au contrat §5.9.

use crate::components::{Card, Empty, LifecycleBadge};
use crate::state::{input_value, repo, App};
use archifier_core::{Lifecycle, ObjectId, ObjectType};
use archifier_repo::{DependencyNode, Direction};
use leptos::prelude::*;

fn direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::In => "entrante",
        Direction::Out => "sortante",
        Direction::Both => "les deux",
    }
}

fn direction_code(direction: Direction) -> &'static str {
    match direction {
        Direction::In => "IN",
        Direction::Out => "OUT",
        Direction::Both => "BOTH",
    }
}

fn direction_choice(value: &str) -> Direction {
    match value {
        "IN" => Direction::In,
        "OUT" => Direction::Out,
        _ => Direction::Both,
    }
}

/// Regroupe des nœuds déjà triés `(depth, path)` en sections par profondeur.
fn group_by_depth(nodes: Vec<DependencyNode>) -> Vec<(usize, Vec<DependencyNode>)> {
    let mut groups: Vec<(usize, Vec<DependencyNode>)> = Vec::new();
    for node in nodes {
        match groups.last_mut() {
            Some((depth, bucket)) if *depth == node.depth => bucket.push(node),
            _ => groups.push((node.depth, vec![node])),
        }
    }
    groups
}

/// Chemin lisible : noms d'objets, séparés par `→`.
fn path_label(path: &[ObjectId]) -> String {
    path.iter()
        .map(|id| {
            repo()
                .object_ref(*id)
                .map(|object| object.name)
                .unwrap_or_else(|| id.to_string())
        })
        .collect::<Vec<_>>()
        .join(" → ")
}

#[component]
pub fn GraphView(app: App, id: ObjectId) -> impl IntoView {
    let (direction, set_direction) = signal(Direction::Both);
    let (depth, set_depth) = signal(3usize);

    let subject = move || {
        let _ = app.version.get();
        repo().object_ref(id)
    };
    let graph = move || {
        let _ = app.version.get();
        repo().dependencies(id, direction.get(), depth.get())
    };

    let direction_selected = direction.get();
    let depth_selected = depth.get();

    view! {
        <main class="page">
            {move || match subject() {
                None => view! {
                    <header class="page-head">
                        <h1>"Graphe de dépendances"</h1>
                        <p class="muted mono">{id.to_string()}</p>
                    </header>
                    <Empty text="Objet introuvable.".into() />
                }.into_any(),
                Some(object) if !matches!(
                    object.object_type,
                    ObjectType::ModelElement | ObjectType::Interface
                ) => view! {
                    <header class="page-head">
                        <div class="crumb">
                            <a href=object.href.clone()>{object.name.clone()}</a>
                            <span>" / graphe"</span>
                        </div>
                        <h1>"Graphe de dépendances"</h1>
                        <p class="muted">
                            {format!("Type « {} » non pris en charge.", object.object_type.as_str())}
                        </p>
                    </header>
                    <Empty text="Le graphe de dépendances ne concerne que les éléments et les interfaces.".into() />
                }.into_any(),
                Some(object) => view! {
                    <header class="page-head">
                        <div class="crumb">
                            <a href=object.href.clone()>{object.name.clone()}</a>
                            <span>" / graphe"</span>
                        </div>
                        <div class="title-row">
                            <h1>"Graphe de dépendances"</h1>
                        </div>
                        <p class="muted">
                            "Relations actives et consommation, lues depuis le référentiel."
                        </p>
                    </header>

                    <Card title="Périmètre".into()>
                        <div class="form-grid">
                            <label class="field">
                                <span>"Direction"</span>
                                <select
                                    prop:value=move || direction_code(direction.get())
                                    on:change=move |ev| {
                                        set_direction.set(direction_choice(&input_value(&ev)));
                                    }
                                >
                                    <option value="IN" selected=direction_selected == Direction::In>
                                        "IN — entrantes"
                                    </option>
                                    <option value="OUT" selected=direction_selected == Direction::Out>
                                        "OUT — sortantes"
                                    </option>
                                    <option value="BOTH" selected=direction_selected == Direction::Both>
                                        "BOTH — les deux"
                                    </option>
                                </select>
                            </label>
                            <label class="field">
                                <span>"Profondeur"</span>
                                <select
                                    prop:value=move || depth.get().to_string()
                                    on:change=move |ev| {
                                        if let Ok(value) = input_value(&ev).parse::<usize>() {
                                            set_depth.set(value.clamp(1, 6));
                                        }
                                    }
                                >
                                    {(1..=6usize).map(|value| view! {
                                        <option value=value.to_string() selected=value == depth_selected>
                                            {value}
                                        </option>
                                    }).collect_view()}
                                </select>
                            </label>
                        </div>
                        <p class="muted small">
                            "Profondeur bornée de 1 à 6 (défaut 3). Les cibles archivées restent visibles et signalées."
                        </p>
                    </Card>

                    {move || match graph() {
                        Err(error) => view! {
                            <Card title="Dépendances".into()>
                                <p class="banner error inline">
                                    {format!("{} — {}", error.message, error.code)}
                                </p>
                            </Card>
                        }.into_any(),
                        Ok(result) => {
                            let truncated = result.truncated;
                            let groups = group_by_depth(result.nodes);
                            view! {
                                <Card title="Dépendances".into()>
                                    {truncated.then(|| view! {
                                        <p>
                                            <span class="badge scheduled">"Graphe tronqué"</span>
                                            <span class="muted small">
                                                " La borne de profondeur ou de 500 nœuds a été atteinte : certains voisins ne sont pas affichés."
                                            </span>
                                        </p>
                                    })}
                                    {if groups.is_empty() {
                                        view! {
                                            <Empty text="Aucun voisin découvert dans ce périmètre.".into() />
                                        }.into_any()
                                    } else {
                                        view! {
                                            {groups.into_iter().map(|(level, nodes)| {
                                                let count = nodes.len();
                                                view! {
                                                    <section class="graph-level">
                                                        <h3>
                                                            "Profondeur " {level}
                                                            <span class="muted small">
                                                                {" · "}{count}{" objet(s)"}
                                                            </span>
                                                        </h3>
                                                        <table class="table">
                                                            <thead>
                                                                <tr>
                                                                    <th>"Objet"</th>
                                                                    <th>"Chemin"</th>
                                                                    <th>"Relation"</th>
                                                                    <th>"Sens"</th>
                                                                    <th>"Cycle de vie"</th>
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {nodes.into_iter().map(|node| {
                                                                    let href = format!(
                                                                        "#/view/{}/{}",
                                                                        node.object_type.route_segment(),
                                                                        node.object_id
                                                                    );
                                                                    let path = path_label(&node.path);
                                                                    let archived =
                                                                        node.lifecycle == Lifecycle::Archived;
                                                                    view! {
                                                                        <tr>
                                                                            <td>
                                                                                <a href=href>{node.name.clone()}</a>
                                                                                <div class="muted small">
                                                                                    {node.domain_name.clone()}
                                                                                </div>
                                                                            </td>
                                                                            <td class="mono small">{path}</td>
                                                                            <td>{node.relation.clone()}</td>
                                                                            <td>{direction_label(node.direction)}</td>
                                                                            <td>
                                                                                {archived.then(|| view! {
                                                                                    <LifecycleBadge lifecycle=node.lifecycle />
                                                                                })}
                                                                            </td>
                                                                        </tr>
                                                                    }
                                                                }).collect_view()}
                                                            </tbody>
                                                        </table>
                                                    </section>
                                                }
                                            }).collect_view()}
                                        }.into_any()
                                    }}
                                </Card>
                            }.into_any()
                        }
                    }}
                }.into_any(),
            }}
        </main>
    }
}
