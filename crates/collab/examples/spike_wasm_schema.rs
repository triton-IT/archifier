//! Exemple WASM du spike T7-00, forme complète du schéma C-01.
//!
//! `spike_wasm` mesure le coût de `Y.Text` seul. Cet exemple-ci ajoute les
//! autres formes racines du schéma (`Y.Map`, `Y.Array<Y.Map>`, `Y.Text` imbriqué),
//! pour borner le coût d'un moteur qui porterait tout le contrat C-01.
//!
//! Construire avec :
//!
//! ```text
//! cargo build -p archifier-collab --example spike_wasm_schema \
//!   --target wasm32-unknown-unknown --release
//! ```

use yrs::{
    Any, Array, ArrayPrelim, ArrayRef, Doc, Map, MapPrelim, MapRef, RootRef, TextPrelim, Transact,
};

#[unsafe(no_mangle)]
pub extern "C" fn archifier_collab_spike_schema_len() -> u32 {
    run().unwrap_or_default()
}

fn run() -> Result<u32, archifier_collab::CollabError> {
    let text = archifier_collab::TextDocument::new(8_101, "documentation")?;
    text.insert(0, "documentation initiale")?;

    let doc = Doc::with_client_id(8_102);
    {
        let mut txn = doc.try_transact_mut()?;

        let meta = MapRef::root("meta").get_or_create(&mut txn);
        meta.insert(&mut txn, "schema", Any::BigInt(1));
        meta.insert(
            &mut txn,
            "object_id",
            Any::from("01J0000000000000000000000".to_owned()),
        );
        meta.insert(&mut txn, "doc_kind", Any::from("OBJECT".to_owned()));
        meta.insert(&mut txn, "seeded", Any::Bool(true));

        let comments = ArrayRef::root("comments").get_or_create(&mut txn);
        let comment = comments.push_back(
            &mut txn,
            MapPrelim::from([
                ("id", Any::from("comment-1".to_owned())),
                ("author_id", Any::from("user-1".to_owned())),
                ("created_at", Any::from("2026-11-18T10:00:00Z".to_owned())),
            ]),
        );
        comment.insert(&mut txn, "markdown", TextPrelim::new("premier commentaire"));

        let layout = MapRef::root("layout").get_or_create(&mut txn);
        let nodes = layout.insert(&mut txn, "nodes", ArrayPrelim::default());
        for i in 0..2 {
            nodes.push_back(
                &mut txn,
                MapPrelim::from([
                    ("id", Any::from(format!("node-{i}"))),
                    ("kind", Any::from("element".to_owned())),
                ]),
            );
        }
        layout.insert(&mut txn, "edges", ArrayPrelim::default());

        let story = MapRef::root("story").get_or_create(&mut txn);
        let chapters = story.insert(&mut txn, "chapters", ArrayPrelim::default());
        let chapter = chapters.push_back(
            &mut txn,
            MapPrelim::from([
                ("id", Any::from("chapter-1".to_owned())),
                ("position", Any::BigInt(0)),
            ]),
        );
        chapter.insert(&mut txn, "title", TextPrelim::new("Chapitre 1"));
    }

    // Même chemin de synchronisation que `spike_wasm`, pour que les deux
    // mesures couvrent le même encodage/décodage binaire.
    let remote = archifier_collab::TextDocument::new(8_103, "documentation")?;
    remote.apply_v1(&text.full_update_v1()?)?;
    text.insert(0, "· ")?;
    remote.apply_v1(&text.diff_v1(&remote.state_vector_v1()?)?)?;

    text.len()
}

fn main() {
    std::hint::black_box(archifier_collab_spike_schema_len());
}
