//! Démarrage : ouverture d'IndexedDB, hydratation du dépôt, montage de
//! l'interface. Aucune requête réseau n'est effectuée.

use crate::Root;
use archifier_repo::idb::IdbStore;
use archifier_repo::{seed, MemoryStore, Repository, SnapshotStore};
use leptos::prelude::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    IdbStore::open(|result| match result {
        Ok((store, initial)) => {
            let store: Rc<dyn SnapshotStore> = store;
            boot(store, initial, "IndexedDB");
        }
        Err(reason) => {
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "IndexedDB indisponible ({reason}) — repli en mémoire, les données seront perdues au rechargement."
            )));
            boot(Rc::new(MemoryStore::new()), None, "Mémoire de session");
        }
    });
}

fn boot(store: Rc<dyn SnapshotStore>, initial: Option<String>, label: &str) {
    let now_ms = js_sys::Date::now() as u64;

    // Démarrage à vide demandé depuis les réglages : on ignore le snapshot
    // existant, quel que soit le moment où la suppression aboutit.
    let force_empty = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("archifier:force_empty").ok().flatten())
        .is_some();
    if force_empty {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.remove_item("archifier:force_empty");
        }
    }

    let initial = if force_empty { None } else { initial };

    // Les anciennes installations contiennent le jeu de démonstration livré
    // avec les premières versions. Il n'est jamais rechargé : le nouveau
    // snapshot vide le remplace, une seule fois.
    let purge_demo = !force_empty
        && !local_flag("archifier:demo_purged")
        && initial.as_deref().is_some_and(seed::is_demo_snapshot);
    let initial = if purge_demo { None } else { initial };
    let store_after_boot = store.clone();

    let repo = match initial.as_deref() {
        Some(json) if !json.trim().is_empty() => {
            Repository::from_json(store.clone(), json, now_ms, now_ms).unwrap_or_else(|error| {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Snapshot illisible ({error}) — démarrage sur un référentiel neuf."
                )));
                Repository::new(store, now_ms, now_ms)
            })
        }
        _ => Repository::new(store, now_ms, now_ms),
    };

    if purge_demo && store_after_boot.save(&repo.export_json()).is_ok() {
        set_local_flag("archifier:demo_purged");
    }

    repo.set_now_ms(now_ms);
    if let Some(error) = repo.profile_error() {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "Profils de métamodèle illisibles : {error} — référentiel en mode dégradé."
        )));
    }
    crate::state::install(Rc::new(repo), label);
    crate::tabs::install();
    install_lifecycle();

    leptos::mount::mount_to_body(|| view! { <Root /> });
}

fn local_flag(key: &str) -> bool {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
        .is_some()
}

fn set_local_flag(key: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(key, "1");
    }
}

/// Cycle de vie best-effort du canal inter-onglets (contrat §9.4).
///
/// `pagehide` ferme le canal sauf si la page entre en bfcache ; `pageshow` et le
/// retour de visibilité réabonnent le canal. Aucun message manqué en
/// arrière-plan n'est garanti, aucune vérification réseau n'est lancée.
fn install_lifecycle() {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };

    let pagehide = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(
        move |event: web_sys::Event| {
            if !event_persisted(&event) {
                crate::tabs::close();
            }
        },
    );
    let _ = window.add_event_listener_with_callback("pagehide", pagehide.as_ref().unchecked_ref());
    pagehide.forget();

    let pageshow =
        wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(move |_event| {
            crate::tabs::resume();
        });
    let _ = window.add_event_listener_with_callback("pageshow", pageshow.as_ref().unchecked_ref());
    pageshow.forget();

    if let Some(target) = window.document() {
        let document = target.clone();
        let visibility =
            wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(move |_event| {
                if document_visible(&document) {
                    crate::tabs::resume();
                }
            });
        let _ = target.add_event_listener_with_callback(
            "visibilitychange",
            visibility.as_ref().unchecked_ref(),
        );
        visibility.forget();
    }
}

/// `PageTransitionEvent.persisted`, lu sans activer de feature web-sys
/// supplémentaire (l'événement est reçu comme `Event` générique).
fn event_persisted(event: &web_sys::Event) -> bool {
    js_sys::Reflect::get(event, &JsValue::from_str("persisted"))
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn document_visible(document: &web_sys::Document) -> bool {
    js_sys::Reflect::get(document, &JsValue::from_str("visibilityState"))
        .ok()
        .and_then(|value| value.as_string())
        .is_some_and(|state| state == "visible")
}
