//! Recherche plein texte : noms, documentation, contrats.

use crate::components::{Empty, PageHeader};
use crate::state::{input_value, repo, App};
use leptos::prelude::*;

#[component]
pub fn Search(app: App, #[prop(optional, into)] q: Option<String>) -> impl IntoView {
    let (query, set_query) = signal(q.unwrap_or_default());
    let (only, set_only) = signal("ALL".to_string());

    let results = move || {
        let _ = app.version.get();
        let text = query.get();
        if text.trim().len() < 2 {
            return vec![];
        }
        let types: Vec<archifier_repo::ObjectFilterType> = match only.get().as_str() {
            "ELEMENTS" => vec![archifier_repo::ObjectFilterType::Element],
            "INTERFACES" => vec![archifier_repo::ObjectFilterType::Interface],
            "DIAGRAMS" => vec![archifier_repo::ObjectFilterType::Diagram],
            "DOCS" => vec![],
            _ => vec![],
        };
        repo().search(&text, &types)
    };

    view! {
        <main class="page">
            <PageHeader
                title="Recherche".into()
                subtitle="Recherche dans les noms, la documentation, les contrats et les vues.".into()
            />
            <div class="search-bar">
                <input
                    autofocus=true
                    placeholder="Customer API, solvabilité, encaissement…"
                    prop:value=move || query.get()
                    on:input=move |ev| set_query.set(input_value(&ev))
                />
                <select on:change=move |ev| set_only.set(input_value(&ev))>
                    <option value="ALL">"Tout"</option>
                    <option value="ELEMENTS">"Éléments"</option>
                    <option value="INTERFACES">"Interfaces"</option>
                    <option value="DIAGRAMS">"Vues"</option>
                </select>
            </div>

            {move || {
                let hits = results();
                if query.get().trim().len() < 2 {
                    return view! {
                        <Empty text="Saisissez au moins deux caractères.".into() />
                    }.into_any();
                }
                if hits.is_empty() {
                    return view! {
                        <Empty text="Aucun résultat. Le référentiel ne contient peut-être pas ce concept.".into() />
                    }.into_any();
                }
                view! {
                    <ul class="results">
                        {hits.into_iter().map(|hit| view! {
                            <li>
                                <a href=hit.href>
                                    <span class="result-title">{hit.title.clone()}</span>
                                    <span class="muted small">{hit.domain_name.clone()}</span>
                                </a>
                                {if hit.excerpt.is_empty() {
                                    ().into_any()
                                } else {
                                    view! { <p class="muted small">{hit.excerpt.clone()}</p> }.into_any()
                                }}
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </main>
    }
}
