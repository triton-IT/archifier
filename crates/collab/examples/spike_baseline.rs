//! Exemple WASM témoin du spike T7-00 : même forme de binaire, sans Yrs.
//!
//! Il sert de base de comparaison pour la taille : la différence entre
//! `spike_wasm.wasm` et `spike_baseline.wasm` est le coût marginal de Yrs et de
//! son câblage dans un binaire `wasm32-unknown-unknown` du même profil.
//!
//! Construire avec :
//!
//! ```text
//! cargo build -p archifier-collab --example spike_baseline \
//!   --target wasm32-unknown-unknown --release
//! ```

/// Longueur du texte généré par l'exemple.
const SEED_CHARS: usize = 512;

#[unsafe(no_mangle)]
pub extern "C" fn archifier_collab_spike_len() -> u32 {
    let seed = "archifier ".repeat(SEED_CHARS / 10 + 1);
    let mut copy = String::from(seed.as_str());
    copy.push('é');
    copy.insert(3, 'x');
    copy.len() as u32
}

fn main() {
    std::hint::black_box(archifier_collab_spike_len());
}
