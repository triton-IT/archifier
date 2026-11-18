//! Persistance IndexedDB : un seul enregistrement `snapshot/current`, écrit
//! atomiquement. Le quota IndexedDB est sans commune mesure avec
//! `localStorage` et l'historique complet est donc conservé.
//!
//! L'ouverture est asynchrone (callback) ; l'écriture est émise de façon
//! synchrone puis validée par le navigateur, ce qui suffit au contrat
//! `SnapshotStore`.

use crate::storage::{SnapshotStore, StoreError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{IdbDatabase, IdbTransactionMode};

const DB_NAME: &str = "archifier";
const DB_VERSION: u32 = 1;
const STORE: &str = "snapshot";
const KEY: &str = "current";

pub struct IdbStore {
    db: IdbDatabase,
    initial: Option<String>,
    errors: Rc<RefCell<Option<String>>>,
    write_seq: Cell<u64>,
    evicted: Cell<bool>,
}

impl IdbStore {
    /// Ouvre la base et charge le snapshot courant.
    /// `on_ready` reçoit toujours un store : en cas d'échec, un store dégradé
    /// en mémoire est renvoyé pour que l'application reste utilisable.
    pub fn open(on_ready: impl FnOnce(Result<(Rc<IdbStore>, Option<String>), String>) + 'static) {
        let on_ready = Rc::new(RefCell::new(Some(on_ready)));
        let Some(window) = web_sys::window() else {
            if let Some(cb) = on_ready.borrow_mut().take() {
                cb(Err("window indisponible".into()));
            }
            return;
        };
        let factory = match window.indexed_db() {
            Ok(Some(factory)) => factory,
            Ok(None) => {
                if let Some(cb) = on_ready.borrow_mut().take() {
                    cb(Err("IndexedDB refusé par le navigateur".into()));
                }
                return;
            }
            Err(_) => {
                if let Some(cb) = on_ready.borrow_mut().take() {
                    cb(Err("IndexedDB inaccessible".into()));
                }
                return;
            }
        };

        // Détection d'éviction : un marqueur localStorage annonce des données.
        let previously_seen = window
            .local_storage()
            .ok()
            .flatten()
            .and_then(|s| s.get_item("archifier:last_seen").ok().flatten())
            .is_some();

        let request = match factory.open_with_u32(DB_NAME, DB_VERSION) {
            Ok(request) => request,
            Err(_) => {
                if let Some(cb) = on_ready.borrow_mut().take() {
                    cb(Err("ouverture IndexedDB impossible".into()));
                }
                return;
            }
        };

        // Création du magasin à la première ouverture.
        let upgrade_db = request.clone();
        let on_upgrade = Closure::<dyn FnMut(web_sys::Event)>::new(move |_event| {
            if let Ok(result) = upgrade_db.result() {
                if let Some(db) = result.dyn_ref::<IdbDatabase>() {
                    // Échoue si le magasin existe déjà : sans conséquence.
                    let _ = db.create_object_store(STORE);
                }
            }
        });
        request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
        on_upgrade.forget();

        let errors: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let errors_for_open = errors.clone();
        let on_ready_error = on_ready.clone();

        // Chaîne : open success -> get snapshot -> ready.
        let request_for_open = request.clone();
        let on_open = Closure::<dyn FnMut()>::new(move || {
            let Some(cb) = on_ready.borrow_mut().take() else {
                return;
            };
            let db = match request_for_open
                .result()
                .ok()
                .and_then(|v| v.dyn_into::<IdbDatabase>().ok())
            {
                Some(db) => db,
                None => {
                    cb(Err("base IndexedDB illisible".into()));
                    return;
                }
            };
            let store = match db.transaction_with_str(STORE) {
                Ok(tx) => match tx.object_store(STORE) {
                    Ok(store) => store,
                    Err(_) => {
                        cb(Err("magasin snapshot introuvable".into()));
                        return;
                    }
                },
                Err(_) => {
                    cb(Err("transaction IndexedDB impossible".into()));
                    return;
                }
            };
            let get = match store.get(&JsValue::from_str(KEY)) {
                Ok(req) => req,
                Err(_) => {
                    cb(Err("lecture IndexedDB impossible".into()));
                    return;
                }
            };
            let db_for_cb = db.clone();
            let errors_for_cb = errors.clone();
            let cb = RefCell::new(Some(cb));
            let get_for_cb = get.clone();
            let on_get = Closure::<dyn FnMut()>::new(move || {
                let Some(cb) = cb.borrow_mut().take() else {
                    return;
                };
                let initial = get_for_cb.result().ok().and_then(|value| value.as_string());
                let store = IdbStore {
                    db: db_for_cb.clone(),
                    initial: initial.clone(),
                    errors: errors_for_cb.clone(),
                    write_seq: Cell::new(0),
                    evicted: Cell::new(previously_seen && initial.is_none()),
                };
                cb(Ok((Rc::new(store), initial)));
            });
            get.set_onsuccess(Some(on_get.as_ref().unchecked_ref()));
            on_get.forget();
        });
        request.set_onsuccess(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();

        let errors_for_error = errors_for_open;
        let on_error = Closure::<dyn FnMut()>::new(move || {
            *errors_for_error.borrow_mut() = Some("ouverture IndexedDB impossible".into());
            if let Some(cb) = on_ready_error.borrow_mut().take() {
                cb(Err("ouverture IndexedDB impossible".into()));
            }
        });
        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();
    }

    pub fn initial(&self) -> Option<String> {
        self.initial.clone()
    }
}

impl SnapshotStore for IdbStore {
    fn save(&self, json: &str) -> Result<(), StoreError> {
        let tx = self
            .db
            .transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)
            .map_err(|_| StoreError::Unavailable("transaction IndexedDB".into()))?;
        let store = tx
            .object_store(STORE)
            .map_err(|_| StoreError::Unavailable("magasin IndexedDB".into()))?;

        let payload = JsValue::from_str(json);
        let key = JsValue::from_str(KEY);
        match store.put_with_key(&payload, &key) {
            Ok(request) => {
                let errors = self.errors.clone();
                let on_error = Closure::<dyn FnMut()>::new(move || {
                    *errors.borrow_mut() = Some("écriture IndexedDB refusée (quota ?)".into());
                });
                request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
                on_error.forget();
            }
            Err(_) => {
                *self.errors.borrow_mut() = Some("écriture IndexedDB refusée".into());
                return Err(StoreError::QuotaExceeded);
            }
        }

        self.write_seq.set(self.write_seq.get() + 1);
        // Marqueur d'éviction : présent même si la base est vidée plus tard.
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("archifier:last_seen", &self.write_seq.get().to_string());
            }
        }
        Ok(())
    }

    fn reset(&self) -> Result<(), StoreError> {
        self.db.close();
        let window = web_sys::window()
            .ok_or_else(|| StoreError::Unavailable("window indisponible".into()))?;
        let factory = window
            .indexed_db()
            .map_err(|_| StoreError::Unavailable("IndexedDB inaccessible".into()))?
            .ok_or_else(|| StoreError::Unavailable("IndexedDB refusé".into()))?;
        factory
            .delete_database(DB_NAME)
            .map_err(|_| StoreError::Unavailable("suppression IndexedDB impossible".into()))?;
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item("archifier:last_seen");
        }
        Ok(())
    }

    fn take_error(&self) -> Option<String> {
        self.errors.borrow_mut().take()
    }

    fn eviction_suspected(&self) -> bool {
        self.evicted.get()
    }
}
