//! Interface Archifier — Leptos en rendu client, compilé en WebAssembly.

#[cfg(target_arch = "wasm32")]
pub mod boot;
pub mod canvas;
pub mod catalog;
pub mod components;
pub mod design;
pub mod focus;
pub mod i18n;
pub mod inspector;
pub mod map;
pub mod router;
pub mod routes;
pub mod state;
pub mod tabs;
pub mod templates;
pub mod views;

use leptos::prelude::*;

/// Racine de l'application. Le dépôt est installé avant le montage
/// (voir `main.rs`) : aucun chargement asynchrone dans l'arbre de vues.
#[component]
pub fn Root() -> impl IntoView {
    let app = state::App::new();

    view! {
        <components::Shell app=app>
            {move || {
                let route = app.route.get();
                routes::render(app, route)
            }}
        </components::Shell>
    }
}
