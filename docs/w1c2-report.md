# Rapport de fin de sous-lot — W1c-2 (multi-onglets)

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | T1-08 |
| Statut | **Livré et vérifié** |
| Contrat | `docs/w1c2-contract.md` §9 |

## 1. Commandes et résultats

| Commande | Résultat |
|---|---|
| `cargo test --workspace` | ✅ **253 tests**, 0 échec |
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| `cargo check -p archifier-ui --target wasm32-unknown-unknown` | ✅ |
| `.\scripts\build.cmd` | ✅ bundle **2,76 Mo** |

## 2. Livrables moteur

- `crates/repo/src/tabs.rs` : `TAB_CHANNEL_NAME`, `TabId`, `TabMessage`, `ExternalWrite`, `TabStatus`, `TabError`, `TabTransport`, `InProcessTabTransport`, `TabGuard`.
- `pair()` crée deux endpoints distincts ; `send` ne s'écho pas ; `close()` ne ferme que l'onglet local ; `TabGuard` ne touche jamais le Dataset.
- 14 tests unitaires + 9 tests d'acceptation.

## 3. Livrables UI

- `crates/ui/src/tabs.rs` : `LocalTabTransport` sur `BroadcastChannel("archifier:repo:v1")`, session `TabId`, `publish_write`, `request_snapshot`, `dismiss`, `close`, `resume`.
- `App` : `external_write`, `external_diff`, `external_note` ; publication dans `App::run` si `persist_error().is_none()`.
- `Shell` : bandeau **Recharger / Comparer / Forcer / Fermer**, diff plafonné à 100 lignes, timeout 1,5 s.
- `boot.rs` : installation, `pagehide`/`pageshow`/`visibilitychange` best-effort.
- Import, reset et import OpenAPI publient explicitement.

## 4. Limites assumées

- **Dernier écrivain gagnant** : T1-08 détecte, ne verrouille pas ; le verrou de snapshot est T7/W3.
- Snapshot de comparaison plafonné à 8 Mio ; au-delà, message « trop volumineux ».
- `pagehide`/`pageshow` best-effort : un message manqué en arrière-plan n'est pas garanti.
- Tout script de même origine peut émettre un faux `Write` ; aucun message n'est authentifié, mais aucun ne déclenche d'écriture.
- `TabStatus::Connected` réservé au transport navigateur/T7.

## 5. Porte

**G2 franchie** : T1-01 → T1-11 ✅, 253 tests, fmt/clippy/wasm/build verts, bundle 2,76 Mo.
