//! Exemple WASM du spike T7-00 : plus petit binaire qui **utilise** Yrs.
//!
//! Le but n'est pas de fonctionner dans un navigateur, mais de mesurer ce que
//! l'édition collaborative ajoute au binaire `wasm32-unknown-unknown`. Tout le
//! travail passe par l'API publique de `archifier-collab` (donc par Yrs) et le
//! résultat traverse `black_box` pour qu'aucun code ne soit éliminé.
//!
//! Construire avec :
//!
//! ```text
//! cargo build -p archifier-collab --example spike_wasm \
//!   --target wasm32-unknown-unknown --release
//! ```

/// Longueur du texte généré par l'exemple.
const SEED_CHARS: usize = 512;

#[unsafe(no_mangle)]
pub extern "C" fn archifier_collab_spike_len() -> u32 {
    run().unwrap_or_default()
}

fn run() -> Result<u32, archifier_collab::CollabError> {
    let seed = "archifier ".repeat(SEED_CHARS / 10 + 1);

    let local = archifier_collab::TextDocument::new(7_001, "documentation")?;
    local.insert(0, &seed)?;

    let remote = archifier_collab::TextDocument::new(7_002, "documentation")?;
    let update = local.full_update_v1()?;
    remote.apply_v1(&update)?;

    local.insert(3, "é")?;
    let keystroke = local.diff_v1(&remote.state_vector_v1()?)?;
    remote.apply_v1(&keystroke)?;

    remote.len()
}

fn main() {
    std::hint::black_box(archifier_collab_spike_len());
}
