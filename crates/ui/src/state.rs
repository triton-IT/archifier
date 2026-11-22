//! État applicatif : signaux réactifs et accès au dépôt.
//!
//! Le dépôt vit dans un `thread_local` plutôt que dans un signal : il est
//! volumineux, non `Send`, et jamais remplacé. Les signaux, eux, restent
//! légers et copiables.

use crate::router::{current_hash, parse_hash, Route};
use crate::tabs;
use archifier_core::{FieldDifference, ObjectId, ViewFraming};
use archifier_repo::tabs::ExternalWrite;
use archifier_repo::{ExportBundle, MemoryStore, RepoError, Repository};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use time::OffsetDateTime;
use wasm_bindgen::JsCast;

thread_local! {
    static REPO: RefCell<Option<Rc<Repository>>> = const { RefCell::new(None) };
    static FALLBACK_REPO: RefCell<Option<Rc<Repository>>> = const { RefCell::new(None) };
    static STORAGE: RefCell<String> = const { RefCell::new(String::new()) };
    static ID_COUNTER: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub fn install(repo: Rc<Repository>, storage: impl Into<String>) {
    REPO.with(|slot| *slot.borrow_mut() = Some(repo));
    STORAGE.with(|slot| *slot.borrow_mut() = storage.into());
}

/// Rend le dépôt installé, ou un dépôt vide en mémoire si le démarrage n'a pas
/// encore eu lieu. Ce repli évite tout panic dans une vue : l'écran affiche un
/// référentiel vide au lieu de faire tomber l'application.
pub fn repo() -> Rc<Repository> {
    if let Some(repository) = REPO.with(|slot| slot.borrow().clone()) {
        return repository;
    }
    FALLBACK_REPO.with(|slot| {
        slot.borrow_mut()
            .get_or_insert_with(|| Rc::new(Repository::new(Rc::new(MemoryStore::new()), 0, 0)))
            .clone()
    })
}

pub fn storage_label() -> String {
    STORAGE.with(|slot| slot.borrow().clone())
}

/// Vrai quand le stockage durable n'a pas pu être ouvert et que la session
/// courante travaille uniquement en mémoire.
pub fn storage_is_temporary() -> bool {
    storage_label() == "Mémoire de session"
}

/// Demande un démarrage à vide au prochain chargement. Marqueur non métier :
/// les données restent dans IndexedDB, ce drapeau force le démarrage sur un
/// référentiel neuf après une réinitialisation.
pub fn request_empty_start() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("archifier:force_empty", "1");
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub error: bool,
    pub message: String,
}

#[derive(Clone, Copy)]
pub struct App {
    pub version: RwSignal<u64>,
    pub route: RwSignal<Route>,
    pub toast: RwSignal<Option<Toast>>,
    /// Palette de création / recherche de la carte (Ctrl/⌘ K).
    pub map_palette: RwSignal<bool>,
    /// Avis d'écriture reçu d'un autre onglet (dialogue global).
    pub external_write: RwSignal<Option<ExternalWrite>>,
    /// Diff local → distant calculé sur demande (« Comparer »).
    pub external_diff: RwSignal<Option<Vec<FieldDifference>>>,
    /// Message du dialogue (attente, pair indisponible, état trop volumineux).
    pub external_note: RwSignal<Option<String>>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let app = Self {
            version: RwSignal::new(0),
            route: RwSignal::new(parse_hash(&current_hash())),
            toast: RwSignal::new(None),
            map_palette: RwSignal::new(false),
            external_write: RwSignal::new(None),
            external_diff: RwSignal::new(None),
            external_note: RwSignal::new(None),
        };
        // Écoute du routage par hash : aucune dépendance à un routeur externe,
        // et l'application reste servable par n'importe quel serveur statique.
        if let Some(window) = web_sys::window() {
            let route = app.route;
            let callback = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                route.set(parse_hash(&current_hash()));
            });
            let _ = window
                .add_event_listener_with_callback("hashchange", callback.as_ref().unchecked_ref());
            callback.forget();

            // Recherche globale : ⌘K / Ctrl+K ouvre la palette de la carte
            // lorsque la carte est active ; ailleurs, il ouvre la recherche.
            let palette = app.map_palette;
            let shortcut_route = app.route;
            let shortcut = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |event: web_sys::KeyboardEvent| {
                    if (event.meta_key() || event.ctrl_key())
                        && event.key().eq_ignore_ascii_case("k")
                    {
                        event.prevent_default();
                        if shortcut_route.get().is_map() {
                            palette.update(|open| *open = !*open);
                        } else if let Some(window) = web_sys::window() {
                            let _ = window.location().set_hash("#/search");
                        }
                    }
                },
            );
            let _ = window
                .add_event_listener_with_callback("keydown", shortcut.as_ref().unchecked_ref());
            shortcut.forget();
        }
        // Avis inter-onglets : un seul abonnement par session de page.
        tabs::bind(app.external_write, app.external_diff, app.external_note);
        app
    }

    /// Rafraîchit l'horloge, reconstruit l'index et notifie les vues.
    pub fn refresh(&self) {
        repo().set_now_ms(js_sys::Date::now() as u64);
        repo().refresh();
        self.version.update(|v| *v += 1);
    }

    pub fn touch(&self) {
        self.version.update(|v| *v += 1);
    }

    pub fn notify(&self, message: impl Into<String>) {
        self.toast.set(Some(Toast {
            error: false,
            message: message.into(),
        }));
    }

    pub fn fail(&self, error: &RepoError) {
        self.toast.set(Some(Toast {
            error: true,
            message: format!("{} — {}", error.message, error.code),
        }));
    }

    /// Exécute une commande, notifie en cas de succès ou d'échec.
    pub fn run<T>(&self, result: Result<T, RepoError>) -> Option<T> {
        match result {
            Ok(value) => {
                self.refresh();
                self.publish_write();
                Some(value)
            }
            Err(error) => {
                self.fail(&error);
                None
            }
        }
    }

    /// Publie l'empreinte du dépôt après une écriture locale réussie.
    ///
    /// Point unique de publication : rien n'est publié si la persistance a
    /// échoué (le bandeau de persistance reste la source de vérité). Le message
    /// ne transporte que l'empreinte, jamais le snapshot.
    pub fn publish_write(&self) {
        let repository = repo();
        if repository.persist_error().is_some() {
            return;
        }
        let fingerprint = archifier_core::sha256_hex(repository.export_json().as_bytes());
        tabs::publish_write(&fingerprint);
    }

    /// Ferme le dialogue d'écriture externe sans rien appliquer (« Forcer »).
    pub fn dismiss_external(&self) {
        tabs::dismiss();
        self.clear_external();
    }

    /// Ferme le dialogue affiché ; aucune écriture n'est déclenchée.
    pub fn clear_external(&self) {
        self.external_write.set(None);
        self.external_diff.set(None);
        self.external_note.set(None);
    }

    pub fn navigate(&self, route: Route) {
        self.route.set(route.clone());
        if let Some(window) = web_sys::window() {
            let _ = window.location().set_hash(&route.to_hash());
        }
    }
}

impl Route {
    // `to_hash` est défini dans `router.rs` (C-06).
}

pub fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(js_sys::Date::now() as i128 * 1_000_000)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

pub fn fmt_date(date: OffsetDateTime) -> String {
    format!(
        "{:02}/{:02}/{}",
        date.day(),
        u8::from(date.month()),
        date.year()
    )
}

pub fn fmt_date_short(date: Option<OffsetDateTime>) -> String {
    date.map(fmt_date).unwrap_or_else(|| "—".into())
}

/// Lit la valeur d'un champ de saisie, quel que soit son type HTML.
pub fn input_value(event: &web_sys::Event) -> String {
    use wasm_bindgen::JsCast;
    let Some(target) = event.target() else {
        return String::new();
    };
    if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
        return input.value();
    }
    if let Some(area) = target.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        return area.value();
    }
    if let Some(select) = target.dyn_ref::<web_sys::HtmlSelectElement>() {
        return select.value();
    }
    String::new()
}

pub fn is_checked(event: &web_sys::Event) -> bool {
    use wasm_bindgen::JsCast;
    event
        .target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.checked())
        .unwrap_or(false)
}

/// Convertit `AAAA-MM-JJ` (champ date natif) en instant UTC à minuit.
pub fn parse_date(value: &str) -> Option<OffsetDateTime> {
    let format = time::macros::format_description!("[year]-[month]-[day]");
    time::Date::parse(value.trim(), &format)
        .ok()
        .map(|date| date.midnight().assume_utc())
}

/// Lit un identifiant d'objet depuis une chaîne (valeur de `<select>`).
///
/// Une chaîne vide ou invalide renvoie `None` : l'appelant décide du message.
pub fn parse_object_id(value: &str) -> Option<ObjectId> {
    value.trim().parse::<ObjectId>().ok()
}

/// Récupère le premier fichier choisi dans un `<input type="file">`.
pub fn selected_file(event: &web_sys::Event) -> Option<web_sys::File> {
    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
        .and_then(|input| input.files())
        .and_then(|files| files.get(0))
}

/// Lit un fichier local en UTF-8 et transmet son contenu au callback.
///
/// Le callback n'est appelé qu'une fois, à la fin de la lecture. En cas
/// d'échec (fichier binaire, lecteur indisponible) rien n'est transmis : la zone
/// de texte conserve son contenu précédent.
pub fn read_text_file(file: &web_sys::File, on_load: impl Fn(String) + 'static) {
    let Ok(reader) = web_sys::FileReader::new() else {
        return;
    };
    let loaded = reader.clone();
    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
        if let Ok(result) = loaded.result() {
            if let Some(text) = result.as_string() {
                on_load(text);
            }
        }
    });
    reader.set_onload(Some(callback.unchecked_ref()));
    if reader.read_as_text(file).is_err() {
        // La lecture a échoué : aucun contenu ne sera transmis.
    }
}

/// Télécharge un texte UTF-8 via un `Blob` et un lien éphémère.
pub fn download_text(filename: &str, contents: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(contents));
    let Ok(blob) = web_sys::Blob::new_with_str_sequence(&array) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    let Ok(element) = document.create_element("a") else {
        return;
    };
    let _ = element.set_attribute("href", &url);
    let _ = element.set_attribute("download", filename);
    if let Some(anchor) = element.dyn_ref::<web_sys::HtmlAnchorElement>() {
        anchor.click();
    }
    web_sys::Url::revoke_object_url(&url).ok();
}

/// Télécharge tous les fichiers d'un bundle Markdown.
///
/// Les chemins sont aplatis (`/` → `-`) et préfixés par la racine du bundle :
/// aucune archive zip, aucune arborescence locale n'est créée. Retourne le
/// nombre de fichiers proposés au téléchargement.
pub fn download_bundle(bundle: &ExportBundle) -> usize {
    for file in &bundle.files {
        let flattened = file.path.replace('/', "-");
        let filename = format!("{}-{}", bundle.root, flattened);
        download_text(&filename, &file.content);
    }
    bundle.files.len()
}

/// Identifiant ULID pour une commande ou un objet cr?? depuis l'UI.
pub fn new_object_id() -> ObjectId {
    let now_ms = js_sys::Date::now() as u64;
    let counter = ID_COUNTER.with(|c| {
        let v = c.get().wrapping_add(1);
        c.set(v);
        v
    });
    let random = (js_sys::Math::random() * 4_294_967_295.0) as u128;
    let entropy = ((counter as u128) << 64) ^ random;
    ObjectId::from_parts(now_ms, entropy)
}

/// Derni?re vue ?dit?e, pr?f?rence locale (C-09 ?6). Jamais une donn?e m?tier.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LastView {
    #[serde(default = "one")]
    pub schema: u32,
    pub view_id: ObjectId,
    #[serde(default)]
    pub posture: String,
    #[serde(default)]
    pub framing: ViewFraming,
    #[serde(default)]
    pub temporal_cursor: Option<String>,
    #[serde(default)]
    pub at_ms: f64,
}

fn one() -> u32 {
    1
}

const LAST_VIEW_KEY: &str = "archifier:last_view";

pub fn save_last_view(last: &LastView) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(last) {
                let _ = storage.set_item(LAST_VIEW_KEY, &json);
            }
        }
    }
}

pub fn read_last_view() -> Option<LastView> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let raw = storage.get_item(LAST_VIEW_KEY).ok()??;
    let parsed: LastView = serde_json::from_str(&raw).ok()?;
    if parsed.schema != 1 {
        return None;
    }
    Some(parsed)
}

pub fn clear_last_view() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(LAST_VIEW_KEY);
        }
    }
}
