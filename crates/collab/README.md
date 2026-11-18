# `archifier-collab` — spike T7-00 (Yrs)

Spike headless, bloquant, de la porte de sortie de l'ADR-014 : vérifier que
[Yrs](https://docs.rs/yrs/0.25.0) tient les quatre budgets de l'édition
collaborative avant de câbler quoi que ce soit dans l'UI.

**Verdict : les quatre points passent, avec une marge confortable. Aucun repli
sur Automerge n'est nécessaire.**

- Yrs : `0.25.0` (version imposée : 0.27/0.28 utilisent des `if let` guards
  instables sur le toolchain 1.94).
- Cible WASM : `wasm32-unknown-unknown`, profil `release` du workspace
  (`opt-level = "s"`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`,
  `strip = true`).

## Périmètre

Le crate n'expose qu'un document mono-texte, volontairement minimal :

```rust
pub struct TextDocument { /* Doc + TextRef */ }

impl TextDocument {
    pub fn new(client_id: u64, root: &str) -> Result<Self, CollabError>;
    pub fn client_id(&self) -> u64;
    pub fn len(&self) -> Result<u32, CollabError>;          // unités UTF-16
    pub fn is_empty(&self) -> Result<bool, CollabError>;
    pub fn content(&self) -> Result<String, CollabError>;
    pub fn insert(&self, index: u32, chunk: &str) -> Result<(), CollabError>;
    pub fn state_vector_v1(&self) -> Result<Vec<u8>, CollabError>;
    pub fn diff_v1(&self, remote_sv: &[u8]) -> Result<Vec<u8>, CollabError>;
    pub fn full_update_v1(&self) -> Result<Vec<u8>, CollabError>;
    pub fn apply_v1(&self, update: &[u8]) -> Result<(), CollabError>;
}
```

`CollabError` (`src/error.rs`) couvre : transaction indisponible, binaire v1
illisible, update non intégrable, indice hors bornes, indice au milieu d'une paire
de substitution UTF-16. Aucun `unwrap`, `expect`, `panic!` ni `todo!` hors tests :
les indices et les frontières UTF-16 sont validés avant l'appel Yrs (qui, lui,
panique au-delà de la longueur).

Ne sont **pas** couverts par ce spike : `Y.Map`/`Y.Array` côté API publique,
les sous-documents, la présence (`Awareness`), les transports
(`BroadcastChannel`/WebSocket), IndexedDB et les conversions Markdown.

## Mesures

Toutes les valeurs ci-dessous viennent des commandes de la section suivante,
sur le poste de développement (Windows, `x86_64-pc-windows-msvc`, Rust 1.94.0).

| # | Porte de sortie (ADR-014) | Seuil | Mesure | Verdict |
|---|---|---|---|---|
| 1 | 20 000 caractères + 200 insertions concurrentes, sans perte | convergence exacte | texte final 20 200 caractères, 200 marqueurs uniques présents une seule fois, texte de base intact, les deux réplicas identiques | OK |
| 2 | encodage d'une frappe | < 2 Kio | **17 octets** (`é` au milieu du texte) ; mot + emoji : 21 octets | OK (0,8 %) |
| 3 | décodage + application d'un document de 20 000 caractères | < 30 ms | debug : min 1,635 ms / médiane 1,739 ms / max 1,807 ms — release : min 27,3 µs / médiane 32,6 µs / max 87,9 µs | OK |
| 4 | empreinte WASM brute | < +900 Kio | **+296,9 Kio** (`Y.Text` seul) ; **+332,3 Kio** (formes racines du schéma C-01) | OK (≤ 37 % du budget) |

Repères utiles : l'état complet d'un document de 20 000 caractères pèse
**20 024 octets** en v1 (soit ~1 octet par caractère, en-têtes compris).

### Détail de la mesure WASM

Trois exemples sont compilés dans le même profil et la même cible :

| Binaire | Contenu | Taille |
|---|---|---|
| `spike_baseline` | `String` + `black_box`, **aucun Yrs** | 12 507 octets |
| `spike_wasm` | `TextDocument` : insertion, état complet, diff par vecteur d'état, application | 316 486 octets |
| `spike_wasm_schema` | + `Y.Map`, `Y.Array<Y.Map>`, `Y.Text` imbriqué (forme racine du contrat C-01) | 352 732 octets |

Le delta retenu est la différence avec le témoin :

- `316 486 − 12 507 = 303 979` octets = **296,9 Kio** pour `Y.Text` ;
- `352 732 − 12 507 = 340 225` octets = **332,3 Kio** pour l'ensemble des
  formes racines.

Méthode et caractère conservateur :

1. La mesure exacte du delta UI demanderait de câbler Yrs dans `archifier-ui`
   (interdit à cette vague) puis de recompiler `pkg/archifier_ui_bg.wasm`.
2. À défaut, on isole le coût marginal : même crate, même cible, même profil
   `release`, un témoin sans Yrs et un binaire qui ne fait que les opérations
   réellement prévues. Tout le code atteint depuis `main` est conservé (appels
   traversant `std::hint::black_box`), donc Yrs n'est pas éliminé par le
   linker.
3. Le delta **surestime** le coût pour l'UI sur un point : l'exemple embarque
   la chaîne `yrs → fastrand (feature "js") → getrandom/wasm_js →
   wasm-bindgen` (imports `__wbindgen_placeholder__` / `getRandomValues`
   présents dans le `.wasm`). L'UI paie déjà wasm-bindgen et js-sys ; cette part
   serait mutualisée dans un vrai build.
4. Le delta **sous-estime** légèrement le périmètre futur : `Awareness`
   (présence) n'est pas mesuré. Pour ordre de grandeur, le pkg actuel
   (`pkg/archifier_ui_bg.wasm`, build wasm-bindgen du 18/11) pèse 2 043 766
   octets ; +331,9 Kio représenterait ~+16,6 %.
5. Le seuil de l'ADR porte sur du **brut** : ni gzip ni brotli ne sont
   appliqués ici (une compression HTTP réduirait typiquement ce delta de
   moitié).

## Tests

Fichier `tests/spike.rs` (les tests peuvent utiliser `unwrap`) :

| Test | Porte |
|---|---|
| `concurrent_200_insertions_into_20k_text_converge_without_loss` | 1 |
| `keystroke_update_stays_under_2_kib` | 2 |
| `decoding_20k_document_stays_under_30_ms` | 3 |
| `text_offsets_are_utf16_code_units` | contrat §3.4 (accents, emoji, caractères combinés) |
| `insert_inside_surrogate_pair_is_refused` | frontière UTF-16 refusée, pas de décalage silencieux |
| `out_of_bounds_index_is_a_typed_error` | erreurs typées, pas de panique |

**6 tests** au total.

Les valeurs mesurées sont affichées avec `-- --nocapture` (les assertions
échouent avec le seuil et la valeur réelle).

## Commandes

```text
cargo test -p archifier-collab
cargo test -p archifier-collab -- --nocapture
cargo test -p archifier-collab --release -- --nocapture

cargo build -p archifier-collab --example spike_baseline \
  --target wasm32-unknown-unknown --release
cargo build -p archifier-collab --example spike_wasm \
  --target wasm32-unknown-unknown --release
cargo build -p archifier-collab --example spike_wasm_schema \
  --target wasm32-unknown-unknown --release
```

Les `.wasm` sont produits dans
`target/wasm32-unknown-unknown/release/examples/` :

```powershell
Get-ChildItem target\wasm32-unknown-unknown\release\examples\spike_*.wasm |
  Where-Object { $_.Name -notmatch '-[0-9a-f]{16}\.wasm' } |
  Select-Object Name, Length
```

## Constats vérifiés sur Yrs 0.25

- API réelle (lue dans le source `yrs-0.25.0`, pas devinée) :
  `Doc::with_options`, `TextRef::root(name).get_or_create(&mut txn)`,
  `Text::insert/len`, `GetString::get_string`, `Transact::try_transact_mut`,
  `ReadTxn::state_vector/encode_diff_v1/encode_state_as_update_v1`,
  `TransactionMut::apply_update`, `Update::decode_v1`,
  `StateVector::encode_v1/decode_v1`.
- `ClientID` est un `u64` en 0.25 (pas `u32`).
- `OffsetKind::Utf16` est **opt-in** : `Doc::new()` est en `Bytes`. Le crate
  force `Utf16`, ce qui aligne les indices sur `String.prototype.length` et les
  sélections DOM (contrat §3.4). Vérifié par test : `"a😀b"` mesure 4 unités
  (6 octets UTF-8), l'insertion à l'indice 3 tombe après l'emoji.
- Yrs ne découpe pas les graphèmes : insérer entre `e` et `U+0301` est accepté.
  Le refus d'une frontière invalide reste à la charge du pont UI, pas du CRDT.
- L'encodage v1 est bien celui de Yjs (`EncoderV1`) ; v2 existe mais n'est pas
  utilisé (compatibilité protocole et spécification §3.4).

## Points ouverts

- Mesurer le delta réel sur `archifier_ui_bg.wasm` quand Yrs y sera câblé
  (vague suivante), et vérifier que `pkg` reste sous le budget global.
- Mesurer `Awareness` (présence) séparément avant de l'activer.
- Les tables de 20 000 caractères ne disent rien du pire cas (document Markdown
  de 200 Ko) : à couvrir par un budget de taille d'état complet, pas seulement
  par la latence de décodage.
- `cargo test` reste en debug : le seuil de 30 ms y est tenu avec ~17× de marge
  (1,8 ms), donc la porte ne dépend pas du profil.
