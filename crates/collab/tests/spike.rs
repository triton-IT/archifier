//! Spike T7-00 — portes de sortie de l'ADR-014, vérifiées sans UI ni réseau.
//!
//! Chaque test correspond à un point de la porte de sortie :
//!
//! 1. [`concurrent_200_insertions_into_20k_text_converge_without_loss`]
//! 2. [`keystroke_update_stays_under_2_kib`]
//! 3. [`decoding_20k_document_stays_under_30_ms`]
//!
//! Deux tests de contrat complètent la vérification de l'encodage Yrs v1 :
//! [`text_offsets_are_utf16_code_units`] et
//! [`out_of_bounds_index_is_a_typed_error`].

use std::time::{Duration, Instant};

use archifier_collab::{CollabError, TextDocument};

/// Longueur du texte de référence demandé par l'ADR-014.
const BASE_CHARS: usize = 20_000;

/// Texte ASCII reproductible de [`BASE_CHARS`] caractères.
fn base_text() -> String {
    let alphabet = b"abcdefghijklmnopqrstuvwxyz ";
    let mut text = String::with_capacity(BASE_CHARS);
    for i in 0..BASE_CHARS {
        text.push(alphabet[i % alphabet.len()] as char);
    }
    text
}

/// Générateur congruentiel linéaire : positions variées mais reproductibles.
fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

/// Réplique préparée avec le texte de référence, synchronisée depuis `source`.
fn replica_with_base(client_id: u64, source: &TextDocument) -> TextDocument {
    let replica = TextDocument::new(client_id, "documentation").unwrap();
    replica.apply_v1(&source.full_update_v1().unwrap()).unwrap();
    replica
}

// ---------------------------------------------------------------------------
// 1. Convergence sans perte : 20 000 caractères + 200 insertions concurrentes
// ---------------------------------------------------------------------------

#[test]
fn concurrent_200_insertions_into_20k_text_converge_without_loss() {
    let a = TextDocument::new(1, "documentation").unwrap();
    a.insert(0, &base_text()).unwrap();
    let b = replica_with_base(2, &a);

    assert_eq!(a.len().unwrap(), BASE_CHARS as u32);
    assert_eq!(b.len().unwrap(), BASE_CHARS as u32);
    assert_eq!(a.content().unwrap(), b.content().unwrap());

    // Vecteurs d'état figés avant toute fusion : chacune des deux mises à jour
    // ne contient que les insertions locales de l'autre côté.
    let sv_a = a.state_vector_v1().unwrap();
    let sv_b = b.state_vector_v1().unwrap();

    let mut rng = 0x5EED_u64;
    let mut markers = Vec::with_capacity(200);

    // Un caractère unique de la zone à usage privé par insertion : aucune
    // insertion ne peut en scinder une autre, et chaque caractère est
    // dénombrable dans le texte final.
    for (replica, first_code) in [(&a, 0xE000_u32), (&b, 0xE100_u32)] {
        for i in 0..100 {
            let marker = char::from_u32(first_code + i).unwrap();
            let len = replica.len().unwrap();
            let position = (next_random(&mut rng) % (u64::from(len) + 1)) as u32;
            replica.insert(position, &marker.to_string()).unwrap();
            markers.push(marker);
        }
    }

    assert_eq!(a.len().unwrap(), BASE_CHARS as u32 + 100);
    assert_eq!(b.len().unwrap(), BASE_CHARS as u32 + 100);

    // Échange symétrique par vecteurs d'état (équivalent BroadcastChannel).
    let diff_a = a.diff_v1(&sv_b).unwrap();
    let diff_b = b.diff_v1(&sv_a).unwrap();
    a.apply_v1(&diff_b).unwrap();
    b.apply_v1(&diff_a).unwrap();

    let text_a = a.content().unwrap();
    let text_b = b.content().unwrap();

    assert_eq!(text_a, text_b, "les deux réplicas doivent converger");
    assert_eq!(
        text_a.chars().count(),
        BASE_CHARS + 200,
        "aucune insertion ne doit être perdue ni dupliquée"
    );
    assert_eq!(
        text_a.len(),
        BASE_CHARS + 200 * 3,
        "chaque marqueur inséré pèse 3 octets UTF-8"
    );
    assert_eq!(
        text_a
            .chars()
            .filter(|c| c.is_ascii_lowercase() || *c == ' ')
            .collect::<String>(),
        base_text(),
        "le texte de base ne doit pas être altéré"
    );
    for marker in &markers {
        assert_eq!(
            text_a.matches(*marker).count(),
            1,
            "insertion U+{:04X} absente ou dupliquée",
            *marker as u32
        );
    }

    println!(
        "200 insertions concurrentes : {} caractères finaux ({:?} unités UTF-16), 200 marqueurs présents",
        text_a.chars().count(),
        a.len().unwrap()
    );
}

// ---------------------------------------------------------------------------
// 2. Une frappe reste sous 2 Ko
// ---------------------------------------------------------------------------

#[test]
fn keystroke_update_stays_under_2_kib() {
    let a = TextDocument::new(11, "documentation").unwrap();
    a.insert(0, &base_text()).unwrap();
    let initial = a.full_update_v1().unwrap();
    let b = TextDocument::new(12, "documentation").unwrap();
    b.apply_v1(&initial).unwrap();
    let sv_b = b.state_vector_v1().unwrap();

    // Frappe d'un caractère accentué au milieu d'un document de 20 000 unités.
    a.insert(10_000, "é").unwrap();
    let keystroke = a.diff_v1(&sv_b).unwrap();
    println!(
        "état initial complet : {} octets ; frappe : {} octets",
        initial.len(),
        keystroke.len()
    );
    assert!(
        keystroke.len() < 2 * 1024,
        "une frappe pèse {} octets (seuil 2048)",
        keystroke.len()
    );

    b.apply_v1(&keystroke).unwrap();
    assert_eq!(a.content().unwrap(), b.content().unwrap());
    assert_eq!(b.len().unwrap(), (BASE_CHARS + 1) as u32);

    // Variante informative : un mot court avec emoji (2 unités UTF-16).
    a.insert(20_001, " 😀 ok").unwrap();
    let word = a.diff_v1(&b.state_vector_v1().unwrap()).unwrap();
    println!("mot court + emoji : {} octets", word.len());
    b.apply_v1(&word).unwrap();
    assert_eq!(a.content().unwrap(), b.content().unwrap());
}

// ---------------------------------------------------------------------------
// 3. Décodage + application d'un document de 20 000 caractères sous 30 ms
// ---------------------------------------------------------------------------

#[test]
fn decoding_20k_document_stays_under_30_ms() {
    let source = TextDocument::new(21, "documentation").unwrap();
    source.insert(0, &base_text()).unwrap();
    let update = source.full_update_v1().unwrap();

    let mut samples = Vec::with_capacity(10);
    for round in 0..10 {
        let target = TextDocument::new(22 + round, "documentation").unwrap();
        let start = Instant::now();
        target.apply_v1(&update).unwrap();
        samples.push(start.elapsed());
        assert_eq!(target.len().unwrap(), BASE_CHARS as u32);
        assert_eq!(target.content().unwrap(), source.content().unwrap());
    }

    samples.sort_unstable();
    let min = samples[0];
    let median = samples[samples.len() / 2];
    let max = samples[samples.len() - 1];
    println!(
        "état complet : {} octets ; décodage+application 20k : min {min:?}, médiane {median:?}, max {max:?}",
        update.len()
    );

    assert!(
        max < Duration::from_millis(30),
        "décodage+application trop lent : {max:?} (seuil 30 ms)"
    );
}

// ---------------------------------------------------------------------------
// Contrat d'encodage : indices UTF-16
// ---------------------------------------------------------------------------

#[test]
fn text_offsets_are_utf16_code_units() {
    let doc = TextDocument::new(31, "documentation").unwrap();

    // "a" + emoji (2 unités) + "b" = 4 unités UTF-16, 6 octets UTF-8.
    doc.insert(0, "a😀b").unwrap();
    assert_eq!(doc.len().unwrap(), 4);
    assert_eq!(doc.content().unwrap().len(), 6);

    doc.insert(3, "X").unwrap();
    assert_eq!(doc.content().unwrap(), "a😀Xb");

    // Accent précomposé (1 unité) contre "e" + U+0301 (2 unités).
    let accents = TextDocument::new(32, "documentation").unwrap();
    accents.insert(0, "é").unwrap();
    accents.insert(1, "e\u{0301}").unwrap();
    assert_eq!(accents.len().unwrap(), 3);
    assert_eq!(accents.content().unwrap(), "ée\u{0301}");

    // Yrs ne découpe pas les graphèmes : insérer au milieu d'une séquence
    // combinée est possible, mais un indice au milieu d'une paire de
    // substitution UTF-16 est refusé avant l'appel Yrs.
    accents.insert(2, "\u{0308}").unwrap();
    assert_eq!(accents.content().unwrap(), "ée\u{0308}\u{0301}");
    assert_eq!(accents.len().unwrap(), 4);
}

#[test]
fn insert_inside_surrogate_pair_is_refused() {
    let doc = TextDocument::new(33, "documentation").unwrap();
    doc.insert(0, "a😀b").unwrap();

    match doc.insert(2, "X") {
        Err(CollabError::InvalidBoundary { index: 2 }) => {}
        other => panic!("attendu InvalidBoundary, obtenu {other:?}"),
    }

    // La position après l'emoji reste valide.
    doc.insert(3, "X").unwrap();
    assert_eq!(doc.content().unwrap(), "a😀Xb");
}

#[test]
fn out_of_bounds_index_is_a_typed_error() {
    let doc = TextDocument::new(41, "documentation").unwrap();
    doc.insert(0, "abc").unwrap();

    match doc.insert(4, "x") {
        Err(CollabError::IndexOutOfBounds { index: 4, len: 3 }) => {}
        other => panic!("attendu IndexOutOfBounds, obtenu {other:?}"),
    }

    // Un indice égal à la longueur est une insertion en fin de texte.
    doc.insert(3, "!").unwrap();
    assert_eq!(doc.content().unwrap(), "abc!");
}
