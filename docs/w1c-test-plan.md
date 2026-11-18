# Plan de test W1c-1 — hiérarchie, cadrage Story, comparaison de revues

| | |
|---|---|
| Date | 18 novembre 2026 (deuxième passe, décisions de revue §7) |
| Périmètre | T1-09 (hiérarchie), T1-10 (cadrage et épinglage Story), T1-11 (comparaison de revues) |
| Contrat de référence | `docs/w1c-contract.md` §1 à §4 et **§7** |
| Suites | `crates/repo/tests/hierarchy_acceptance.rs`, `story_acceptance.rs`, `review_compare_acceptance.rs` |
| Fixtures | `fixtures/mvp2/w1c/hierarchy.json`, `story-framing.json`, `reviews.json` |
| État | 22 tests écrits (10 bloquants + 12 complémentaires). Aucun `cargo` lancé. Aucun test existant ni code de production modifié. |

## Principes

- Les tests n'utilisent que l'API publique du contrat : `hierarchy`, `set_parent`,
  `archive`, `restore`, `pin_story_revisions`, `unpin_story_revisions`,
  `save_story`, `compare_reviews`, `story_detail`, `playback`, `diagram_detail`,
  `review_detail`, `element`, `revisions`, `history`, `export_json`.
- Chaque test reconstruit son dépôt avec `Repository::from_json` et une horloge
  figée (`NOW_MS = 1_795_000_000_000`, 18 novembre 2026).
- Les lectures pures (`hierarchy`, `compare_reviews`) et les refus d'écriture
  sont vérifiés par `export_json()` avant/après.
- Les codes `HIER-*`, `STORY-003` et `REVIEW-001` sont testés par
  `RepoError::is(code)` ; aucun test ne dépend du texte du message, sauf pour
  vérifier qu'un libellé contient le nom d'une interface.

## Fixtures

### `fixtures/mvp2/w1c/hierarchy.json`

| Élément | Rôle |
|---|---|
| `Entreprise` (`…201`, Participant BPMN) | racine `depth = 0` |
| `Couloir A` (`…202`), `Couloir A` (`…203`), `Couloir B` (`…204`) | enfants directs `depth = 1` ; deux noms identiques pour le départage par id |
| `Traiter commande` (`…205`) → `Calculer tarif` (`…206`) → `Encaisser` (`…207`) → `Notifier` (`…208`) | chaîne de 5 arêtes ; `Notifier` a `depth = 5` |
| `Archiver commande` (`…209`) | tâche libre : un sixième niveau serait refusé |
| `Partenaire` (`…20A`, Participant `ARCHIVED`) → `Couloir partenaire` (`…20B`) | parent archivé préexistant, enfant **feuille** |
| `Facturation` (`…20C`, ArchiMate) | notation différente (`HIER-002`) |
| `Billing Component` (`…20D`, UML) + `http` (`…20E`, UML Port) | couple de même notation accepté |
| `Vue processus` (`…20F`, Diagramme BPMN) | type non `ModelElement` (`HIER-001`), enfant **et** parent |
| `Atelier` (`…210`) + `Tâche unique` (`…211`) | branche à enfant unique : `HIER-005`, puis archiver enfant → parent → restaurer enfant |
| `Relation native parente` (`…212`) + `Relation native enfant` (`…213`) | notation `NATIVE` non hiérarchisable (`HIER-002`) |
| `Groupe ArchiMate` (`…214`, `Grouping`) | `Grouping` exclu malgré une notation identique (`HIER-002`) |

### `fixtures/mvp2/w1c/story-framing.json`

| Objet | Révision | Rôle |
|---|---|---|
| `Order Service` (`…401`) | **3** | élément référencé deux fois (blocs `b-3`, `b-4`) ; instantané de révision présent |
| `Pricing Service` (`…402`) | 1 | nœud du diagramme, cible de l'atténuation ; **aucun instantané** pour le test `STORY-003` |
| `Payment API` (`…403`) | **4** | interface référencée ; instantané présent |
| `Vue ciblée` (`…404`) | **2** | diagramme à deux nœuds ; instantané présent |
| `Récit de la commande` (`…405`) | 1 | bloc diagramme `zoom 2.0`, focus `…401`, dim `…402`, légende ; bloc interface avec `version_id = …406` |
| `Récit incomplet` (`…407`) | 1 | bloc élément sur `…402`, sans instantané ⇒ `STORY-003` |
| `Récit sans version` (`…408`) | 1 | bloc interface sans `version_id` ⇒ dernière version publiée |
| `Payment API 1.0.0` (`…406`) | — | version publiée explicite du bloc `b-5` |
| `Payment API 2.0.0` (`…409`) | — | dernière version **publiée** |
| `Payment API 3.0.0` (`…410`) | — | version **brouillon**, jamais épinglée par défaut |

Les instantanés de révision de la fixture sont volontairement minimaux
(`{"object":{"id":…,"revision":n}}`) : seul leur **existence** est requise par
§7.3. Après `update_element`, le snapshot de la nouvelle révision est créé par
le chemin d'écriture normal.

### `fixtures/mvp2/w1c/reviews.json`

| Interface | Fournisseur | Versions / consommateurs |
|---|---|---|
| `Payment API` (`…010`) | Paiements | v1 active ; consommée par Commande |
| `Refund API` (`…011`) | Externe | v1 expirée le 2026-03-01 ; consommée par Paiements |
| `Legacy API` (`…012`) | Externe | v1 expirée le 2026-02-01 ; consommée par Commande |
| `Zebra API` (`…013`) | Partenaire | v1 active ; consommée par Paiements |
| `Shared API` (`…014`) | Externe | v1 expirée le 2026-03-01 (Paiements) **et** v2 expirée le 2026-05-01 (Commande) |

| Revue | Domaine | Filtre (`filter_snapshot`) | Fenêtre |
|---|---|---|---|
| `Revue Paiements T1` (`…040`) | Paiements | `EXPOSED / APPLICATION / ACTIVE` | 2026-01-01 → 2026-06-30 |
| `Revue Commande T2` (`…041`) | Commande | `CONSUMED / APPLICATION / EXPIRED` | 2026-07-01 → ∞ |
| `Revue Paiements T3` (`…042`) | Paiements | `CONSUMED / TECHNICAL / ACTIVE` | **fin seule** : ∞ → 2026-09-30 |

Attendu `…040 → …041` : exposées retirées `[Payment]` ; consommées ajoutées
`[Legacy, Payment]` ; consommées retirées `[Refund, Zebra]` ; anomalies ajoutées
`Legacy (fin 2026-02-01)` et `Shared (fin 2026-05-01)` ; anomalies retirées
`Refund (fin 2026-03-01)` et `Shared (fin 2026-03-01)`. `Shared` apparaît des
deux côtés grâce à la clé `(code, object_id, detail)`. Le format du `detail` est
celui du dépôt : les tests vérifient sa présence et sa différence, pas son
formatage exact.

## Tests bloquants du §4

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 1 | `hierarchy_depth_is_limited_to_five` | `hierarchy.json` | chaîne `…201 → …202 → …205 → …206 → …207 → …208` | `ancestors(…208)` = 5 ; `hierarchy(…207).children` contient `…208` avec `depth = 5` ; racine sans ancêtre/parent, enfants directs `depth = 1` (racine = 0) ; `set_parent(…209, …208)` refuse le 6e niveau sans écriture | `HIER-003` |
| 2 | `hierarchy_cycle_is_refused` | idem | chaîne existante | `set_parent(…201, …208)` et `set_parent(…207, …208)` refusés ; export et parents inchangés | `HIER-004` |
| 3 | `archived_parent_is_reported` | idem | `…20A` archivé, `…20B` feuille active | `hierarchy(…20B)` expose `parent = Some(…20A)` et `parent_archived = true` ; `hierarchy(…20A).children` porte `archived_parent = true` ; un nouvel enfant du parent archivé est accepté | — |
| 4 | `hierarchy_requires_same_notation` | idem | ArchiMate/UML/BPMN, diagramme | ArchiMate sous BPMN, port UML sous BPMN refusés ; diagramme comme enfant ou parent refusé ; BPMN/BPMN et UML/UML acceptés | `HIER-002`, `HIER-001` |
| 5 | `story_pins_revision` | `story-framing.json` | épingles vides ; `D=2`, `E=3`, `I=4` | `pin` rend **3** ; `pinned_revisions = {D:2, E:3, I:4}` ; `pinned_versions = {I:…406}` ; `playback` expose les deux cartes ; révision de Story augmentée ; après `update_element(E)`, second pin remplace `E` par sa nouvelle révision et laisse `D`/`I`/version inchangés | — |
| 6 | `story_unpin_clears_pins` | idem | récit épinglé | `unpin` vide les **deux** cartes et crée une révision ; second `unpin` : `Ok`, aucune écriture, **aucune révision** | — |
| 7 | `story_framing_applies_zoom_focus_dim` | idem + helper noyau | cadrage `zoom 2.0`, focus, dim, légende | `zoom = 2.0`, coordonnées ×2 (`x=80`, `w=360`), focus `highlighted` opacité `1.0`, dim `dimmed` opacité **`0.35`**, légende conservée ; défaut `zoom 1.0` ; bornes `0.1 → 0.5`, `9.0 → 3.0` ; layout source intact | — |
| 8 | `review_comparison_shows_filter_and_window_diff` | `reviews.json` | revues `…040`, `…041` | `ObjectRef` source/cible corrects ; `window_from = {01-01, 06-30}`, `window_to = {07-01, None}` ; `filter_from/filter_to` égaux aux instantanés ; ensembles égaux aux différences des `ReviewDetail` ; `exposed_changed`/`consumed_changed` vides ; listes attendues ; inversion symétrique ; export inchangé | — |
| 9 | `review_comparison_lists_anomalies` | idem | mêmes revues | 2 ajouts (`Legacy` fin 02-01, `Shared` fin 05-01) et 2 retraits (`Refund` fin 03-01, `Shared` fin 03-01) ; `Shared` présent des deux côtés avec des `detail` différents ; 4 clés `(code, object_id, detail)` ; listes cohérentes avec `review_detail` ; inversion symétrique ; export inchangé | — |
| 10 | `review_comparison_rejects_unknown_review` | idem | `…0ZZ` absent ; domaine `…002` non-Review | inconnue en source/cible, double inconnue, **et objet non-Review en source comme en cible** refusés ; export inchangé | `REVIEW-001` |

## Cas complémentaires

| # | Test | Fixture | Attendu | Code |
|---|---|---|---|---|
| 11 | `hierarchy_lists_direct_children_sorted_by_name_then_id` | `hierarchy.json` | `hierarchy(…201).children = […202, …203, …204]`, deux « Couloir A » départagés par id, puis « Couloir B » ; tous `parent_id = …201` | — |
| 12 | `hierarchy_self_parent_is_refused` | idem | `set_parent(x, Some(x))` refusé pour racine et descendants ; parents d'origine conservés ; export inchangé | `HIER-004` |
| 13 | `hierarchy_set_parent_is_versioned` | idem | `set_parent(…209, …204)` met à jour `parent_id`, incrémente `revision`, renseigne `updated_by`, crée audit et snapshot de la nouvelle révision | — |
| 14 | `archive_parent_with_active_child_is_hier_005` | idem | `archive(…210)` refusé avec enfant actif ; puis `archive(…211)`, `archive(…210)`, `restore(…211)` ; la fiche de `…211` expose `parent = …210` et `parent_archived = true` avant et après restauration | `HIER-005` |
| 15 | `hierarchy_excludes_native_and_grouping` | idem | `Native` sous `Native` refusé ; `Grouping` ArchiMate sous composant ArchiMate refusé | `HIER-002` |
| 16 | `story_framing_fit_is_computed_before_zoom` | `story-framing.json` | le helper livre des coordonnées déjà ×2 ; `fit(layout non zoomé) × zoom` conserve l'échelle ; fitter le layout zoomé diviserait le zoom par 2 (invariant UI) | — |
| 17 | `story_pins_interface_version` | idem | bloc avec `version_id` ⇒ `…406` ; bloc sans version ⇒ dernière **publiée** `…409`, jamais le brouillon `…410` | — |
| 18 | `story_pin_missing_snapshot_is_story_003` | idem | story `…407` : refus, deux cartes vides, structure et révision inchangées, export inchangé | `STORY-003` |
| 19 | `save_story_invalid_is_atomic` | idem | transition vers un bloc absent : `save_story` refuse, structure et révision inchangées, export inchangé | `STORY-002` |
| 20 | `story_pin_without_references_returns_zero` | idem + `create_story` | récit sans référence : `Ok(0)`, deux cartes vides | — |
| 21 | `story_unpin_is_idempotent_on_empty_pins` | idem | récit jamais épinglé : `unpin` `Ok`, aucune révision, aucune écriture | — |
| 22 | `review_comparison_same_domain_exposes_window_and_filter_diff` | `reviews.json` | `…040 → …042` : même domaine, aucun ajout/retrait/changement ; `window_to = {start: None, end: 2026-09-30}` ; filtres différents ; anomalies identiques | — |

## Codes d'erreur

| Code | Sens | Couverture |
|---|---|---|
| `HIER-001` | type non `ModelElement` (enfant **ou** parent) | test 4 (diagramme des deux côtés) |
| `HIER-002` | notation/métamodèle différents, `Native` ou `Grouping` exclus | tests 4 et 15 |
| `HIER-003` | profondeur > 5 arêtes | test 1 |
| `HIER-004` | cycle ou auto-parenté | tests 2 et 12 |
| `HIER-005` | archivage d'un parent avec enfants actifs | test 14 (≙ `MM-HIER-003`) |
| `STORY-002` | transition vers un bloc absent ; `save_story` doit rester atomique | test 19 |
| `STORY-003` | instantané de révision absent pendant l'épinglage | test 18 |
| `REVIEW-001` | revue inconnue ou objet non-Review, en source comme en cible | test 10 |

`HIER-WARN-001` a disparu de §7 : le parent archivé est un **état** exposé par
`archived_parent` / `parent_archived`, sans code d'erreur.

## Vérifications transverses

1. **Lecture pure** : `hierarchy` et `compare_reviews` ne modifient pas
   `export_json()`.
2. **Refus atomiques** : `set_parent`, `archive`, `pin`, `save_story` en erreur
   laissent le JSON exporté, les structures et les révisions inchangés.
3. **Profondeur en arêtes** : racine 0, enfants directs 1, nœud profond 5,
   sixième niveau refusé.
4. **Ordre** : enfants triés par nom (insensible à la casse) puis id ; ancêtres
   du parent direct vers la racine ; comparaison triée par nom puis id.
5. **Épinglage** : objets seuls dans `pinned_revisions`, version d'interface
   dans `pinned_versions`, doublons dédupliqués, nœuds de diagramme exclus.
6. **Dégel** : vide les deux cartes ; no-op sans écriture si déjà vide.
7. **Identité d'anomalie** : `(code, object_id, detail)` ; deux versions
   expirées de la même interface ne se masquent pas.
8. **Cadrage** : zoom appliqué aux coordonnées, opacité d'atténuation `0.35`,
   focus prioritaire ; le fit UI se calcule avant le zoom.

## Points d'API incertains

1. **Profondeur de la racine.** `Hierarchy` n'expose pas de champ `depth` pour
   l'objet interrogé. Le test 1 prouve `racine = 0` par ses enfants directs
   (`depth = 1`) et par l'absence d'ancêtres. Si le moteur ajoute
   `Hierarchy.depth` (ou un `depth_of` public), remplacer cette preuve indirecte
   par une assertion directe.
2. **`playback`.** §7.3 dit « playback doit exposer les pins
   (`pinned_revisions`, `pinned_versions`) ». Les tests supposent deux champs de
   ces noms sur `Playback`. Un autre nom ou un objet `Pins` imbriqué demande une
   adaptation du test 5.
3. **Valeur de retour de `pin_story_revisions`.** Les tests attendent le nombre
   d'**objets** épinglés (`3`), pas le total objets + versions (`4`). §7.3 ne
   tranche pas explicitement ; à aligner si le moteur compte la version.
4. **Instantanés de fixture.** Les snapshots de `story-framing.json` sont
   minimaux. Si `pin_story_revisions` valide leur forme au lieu de leur seule
   existence, enrichir les snapshots ou déplacer la fixture vers un export réel.
5. **`exposed_changed` / `consumed_changed`.** Au W1c, les deux revues sont
   calculées au même `now` et les filtres ne sont pas appliqués (§7.4 réserve
   leur application à W5). Les lignes communes sont donc toujours identiques :
   les tests vérifient que les listes sont **vides**. Un cas non vide exigera une
   source de divergence (instantané de revue ou application des filtres).
6. **Fenêtre à fin seule.** `TemporalWindow { start: None, end: Some(…) }` est
   attendu tel quel. `window_*` reste `Option<TemporalWindow>` : une revue sans
   aucun début **ni** fin ne peut pas être distinguée d'une absence de fenêtre ;
   non couvert par le contrat.
7. **`from`/`to` inversé.** §7.4 mentionne `REVIEW-001` pour un `from`/`to`
   inversé sans définir l'inversion (chronologie ? ordre des ids ?). Les tests
   ne couvrent que l'inconnue et l'objet non-Review ; à préciser si une règle
   d'ordre est attendue, car elle interdirait la comparaison inverse utilisée
   dans les tests 8 et 9.
8. **`HIER-005` et descendance.** Le test archive un parent dont l'enfant
   **direct** est actif. Si la règle doit aussi considérer les descendants
   (petits-enfants actifs), ajouter une branche dédiée à la fixture.
9. **Restauration d'un enfant sous parent archivé.** Le scénario §7.2 suppose
   que `restore(child)` est accepté alors que le parent reste archivé. Les tests
   l'imposent ; si le moteur refuse, le contrat doit fixer l'ordre inverse.
10. **`Native` / `Grouping`.** §7.1 annonce `HIER-002` pour les deux cas. Le
    test 15 vérifie le code exact ; un moteur qui renvoie `HIER-001` doit être
    aligné avec le contrat.
11. **Fit UI.** `apply_framing` est un helper pur : il ne connaît pas le
    viewport. Le test 16 vérifie l'ordre des opérations par calcul local
    (fit sur layout non zoomé × zoom). L'implémentation `MiniDiagram`
    (viewBox + transform) reste à couvrir par le dev UI, hors de ce périmètre.
12. **Date du badge « Récit figé ».** `playback` expose les cartes, mais aucun
    champ ne porte la « date de la révision la plus récente ». L'UI la dérive
    des épingles ; non testé côté dépôt.

## Exécution prévue (après le dev moteur)

```powershell
cargo test -p archifier-repo --test hierarchy_acceptance
cargo test -p archifier-repo --test story_acceptance
cargo test -p archifier-repo --test review_compare_acceptance
```

Aucun test existant n'est modifié : `acceptance.rs`, `storage_boundary.rs`,
`openapi_acceptance.rs`, `history_diff_acceptance.rs`,
`markdown_export_acceptance.rs` et `diagnostics_acceptance.rs` restent
inchangés.
