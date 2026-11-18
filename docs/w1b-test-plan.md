# Plan de test W1b — diff, export Markdown, liens cassés, graphe

| | |
|---|---|
| Date | 18 novembre 2026 (deuxième passe, revue de contrat §5–§6) |
| Périmètre | T1-04, T1-05, T1-06, T1-07 |
| Contrat de référence | `docs/w1b-contract.md` §1, §2, §5 et §6 ; `technical-specifications.md` §10.2 et §12 |
| Suites | `crates/repo/tests/history_diff_acceptance.rs`, `markdown_export_acceptance.rs`, `diagnostics_acceptance.rs` |
| Fixtures | `fixtures/mvp2/export/dataset.json`, `dataset-unreadable-diagram.json` (voir `README.md` du dossier) |
| État | 19 tests écrits côté dépôt ; `graph_route_parses` à ajouter par le dev UI dans `crates/ui/src/router.rs`. Aucun `cargo` lancé cette passe. |

## Principes

- Les tests n'utilisent que l'API publique du contrat : `Repository::revision_diff`,
  `export_domain_markdown`, `export_object_markdown`, `broken_links`,
  `dependencies`, `revisions`, plus `archifier_core::export::export_domain` pour
  le seul cas impossible à représenter en JSON (coordonnée non finie).
- Le dépôt est reconstruit à chaque test depuis la fixture JSON via
  `Repository::from_json`, avec une horloge figée (`NOW_MS = 1_795_000_000_000`,
  2026-11-18). Aucune écriture préalable, aucun accès au stockage.
- Les décisions de revue §5 priment sur les formulations antérieures : chemins de
  bundle relatifs à `root`, `flowchart LR`, `owner: Option<ObjectRef>`, fenêtre de
  contexte 80 + 80, `DependencyNode.direction`/`lifecycle`, exclusion des champs
  volatils du diff.

## Fixtures

### `dataset.json`

| Élément de fixture | Rôle dans les tests |
|---|---|
| `revisions["…0002"]["1"\|"2"]` | diff champ par champ, immutabilité, exclusion des métadonnées volatiles ; une ligne de diff seulement change de contenu |
| documentation de `…0002` | lien `arch://` valide (`…0003`), cassé (`…000C`, **deux occurrences**), archivé (`…000B`) |
| cache `links` | `OK`, `BROKEN`, et `OK` **périmé** pour la cible archivée : force le recalcul §5.4 |
| diagramme `…000N` | 2 nœuds, 1 arête `serving` ; SVG (viewBox, rect, ligne) + Mermaid `flowchart LR` |
| objet archivé `…000B` | bandeau d'export exact, lien archivé, cible archivée du graphe |
| domaine vide `Vide` (`slug = vide`, `…000W`) | `index.md` valide sans aucun objet |
| `Facturation` (`…000X`) → `…000B` | nœud de graphe avec `lifecycle = ARCHIVED` |
| chaîne `serving` sur 8 niveaux | chemins, profondeur 3, borne 6 et `truncated` |
| `Pricing Service` (1 entrant, 1 sortant) | graphe `BOTH` étiqueté `IN` / `OUT` |

### `dataset-unreadable-diagram.json`

Domaine `invalide` avec un diagramme `w = 0.0` : branche `EXPORT-002` côté dépôt.
La coordonnée non finie (`NaN`) est construite en mémoire dans le test (JSON ne
peut pas la porter) et soumise à la fonction pure du noyau.

## Tests bloquants du §6

### Diff de révisions (5)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 1 | `revision_diff_is_field_by_field` | `dataset.json` | dépôt chargé | `diff(1 → 2)` sur `…0002` : `object_id`, `from`, `to` exacts ; chemins de contenu exactement `/object/name`, `/semantic_properties/owner`, `/semantic_properties/tier` ; `before`/`after` par feuille ; aucun diff de conteneur (`/object`, `/semantic_properties`) ; champ inchangé (`criticality`) absent | — |
| 2 | `revision_diff_is_sorted` | `dataset.json` | idem | `paths` triés et uniques ; `from == to` ⇒ `changes` vide sans erreur | — |
| 3 | `revision_diff_is_immutable` | `dataset.json` | snapshots et JSON exporté mémorisés | deux diffs identiques ; diff inverse cohérent (`from = 2`, `to = 1`) ; `revisions()` et `export_json()` inchangés, aucune écriture | — |
| 4 | `revision_diff_excludes_volatile_metadata` | `dataset.json` | idem | `/object/revision` et `/object/updated_at` absents ; `/object/name` et `/semantic_properties/owner` toujours présents (l'exclusion ne vide pas le diff) | — |
| 5 | `revision_diff_missing_revision_is_diff_001` | `dataset.json` | révision 42 absente ; objet `…000C` absent | `diff(1, 42)`, `diff(42, 42)` et objet inconnu refusés ; `from == to` ne court-circuite pas l'existence des snapshots ; 2 révisions toujours lisibles | `DIFF-001` |

### Export Markdown (7)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 6 | `markdown_export_has_valid_relative_links` | `dataset.json` | domaine `Paiements` exporté | `root = paiements` ; `index.md` + `elements/<id>.md` (paths **relatifs à root**) ; aucun `](arch://` ; toute cible de lien existe dans le bundle ; lien vers `Pricing Service` résout vers `elements/…0003.md` ; index : `# Paiements`, sections `Éléments`/`Diagrammes`, section `Interfaces` omise (vide), liens triés par `ObjectId` ; page archivée présente. Les sections/pages Story ne sont pas exigées (export Story reporté en W1c, §5.1) : si l'index les cite, les liens doivent néanmoins résoudre | — |
| 7 | `markdown_export_is_reproducible` | `dataset.json` | deux dépôts identiques, horloges à un an d'écart ; second export du même dépôt | bundles strictement égaux (`path` + `content`) ; fichiers triés par `path`, chemins uniques ; UTF-8 sans BOM, `\n` seul, saut de ligne final ; aucune date d'export (`2026-11-18`, `2027-11-18`) | — |
| 8 | `markdown_export_includes_svg_and_mermaid` | `dataset.json` | diagramme à 2 nœuds / 1 arête | `diagrams/<id>.svg|.mmd|.md` ; SVG : `viewBox="32.0 32.0 476.0 96.0"` (bbox + padding 8), `<rect`, trait d'arête, libellés, ni `<script>` ni `http(s)://` ni `src=` ; Mermaid : `flowchart LR`, `n0`/`n1`, `-->|serving|`, exactement 1 `-->` ; page liée aux deux fichiers | — |
| 9 | `markdown_export_includes_neighbors_for_object` | `dataset.json` | export de `…0002` seul | `root = order-service-v2` ; page racine + page du voisin direct `…0003` ; chemins canoniques ; aucun lien `arch://` ; tous les liens résolvent ; bundle plus petit que l'export de domaine | — |
| 10 | `markdown_export_empty_domain_has_index` | `dataset.json` (domaine `Vide`) | export du domaine sans objet | `root = vide` ; `index.md` présent et valide (`# Vide`, slug, id, aucune section vide, saut de ligne final) ; aucune page `elements/` | — |
| 11 | `markdown_export_archived_object_has_banner` | `dataset.json` | export du domaine | page `elements/…000B.md` : titre `Legacy Billing`, puis **première ligne non vide** = `> **Archivé** — consultation seule, dernière révision` | — |
| 12 | `markdown_export_non_finite_coordinate_is_export_002` | `dataset.json` (muté) + `dataset-unreadable-diagram.json` | `x = NaN` en mémoire ; fixture disque `w = 0.0` | noyau : `export_domain` renvoie `ExportError::UnreadableDiagram` ; dépôt : `export_domain_markdown` renvoie `EXPORT-002`, aucune écriture | `EXPORT-002` |

### Liens cassés (3)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 13 | `broken_link_diagnostic_has_context_and_owner` | `dataset.json` | documentation chargée | entrée `BROKEN` : `source` = `…0002`, `owner = Some(domaine Paiements)` avec `#/view/domains/…0001`, `target = None`, `target_uri` exacte ; contexte sur une ligne, centré sur l'URI (80 avant + 80 après, `…` si rogné), conserve « Lien cassé volontaire » ; tri `(source.id, target_uri, link.id)` ; aucun lien `OK` dans la liste | `DOC-LINK-001` (statut) |
| 14 | `broken_link_diagnostic_includes_archived` | `dataset.json` | cache `links` = `OK` périmé, cible `…000B` archivée | entrée `ARCHIVED` recalculée : `target = …000B`, `target_object_id` renseigné, `owner` présente ; cible lisible en lecture seule ; 2 diagnostics au total ; `stats().broken_links` = 2 (même recalcul) | `DOC-LINK-002` (statut) |
| 15 | `broken_link_diagnostic_deduplicates` | `dataset.json` | URI cassée citée deux fois | une seule entrée par `(source_object_id, target_uri)` ; contexte issu de la **première** occurrence (`Lien cassé volontaire`, jamais `Deuxième mention`) ; liste et statistique non gonflées | `DOC-LINK-001` (statut) |

### Graphe (4)

| # | Test | Fixture | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 16 | `dependency_graph_shows_paths_and_depth` | `dataset.json` | chaîne `serving` de 8 arêtes | `dependencies(…0002, OUT, 3)` : voisin `depth = 1` (`path = [racine, voisin]`, `relation = serving`, `lifecycle = ACTIVE`, `direction = OUT`) ; nœud `depth = 3` avec chemin complet ; `path` commence par la racine et finit par le nœud, `len == depth + 1` ; aucun doublon ; tri `(depth, path)` ; `IN` étiquette `direction = IN` | — |
| 17 | `dependency_graph_is_bounded_to_six` | `dataset.json` | idem | `depth = 99` ramené à 6 : 6 nœuds, profondeur max 6, `truncated = true` ; `depth = 0` ramené à 1 : seul le voisin direct | — |
| 18 | `dependency_graph_both_direction_is_labelled` | `dataset.json` | `Pricing Service` : 1 entrant, 1 sortant | `BOTH` = union : prédécesseur (`Order Service v2`) étiqueté `IN`, successeur (`Tarification`) étiqueté `OUT` ; chemins partant de la racine ; union triée `(depth, path)` | — |
| 19 | `dependency_graph_archived_target_is_flagged` | `dataset.json` | `Facturation → Legacy Billing` | cible archivée incluse, `lifecycle = ARCHIVED`, `direction = OUT`, `depth = 1`, chemin `[Facturation, Legacy Billing]` | — |

### Route (1, hors périmètre dépôt)

| # | Test | Emplacement | Précondition | Attendu | Code |
|---|---|---|---|---|---|
| 20 | `graph_route_parses` | **`crates/ui/src/router.rs`** (à ajouter par le dev UI) | route `#/graph/<id>` | `Route::Graph { id }` pour un élément et une interface ; `to_hash` puis `parse_hash` = aller-retour ; `#/graph/<id>` = deux segments ; profondeur/direction en état local, hors route ; mise à jour de `hash_roundtrip_for_each_route` | — |

> Ce périmètre ne peut ni lire ni modifier `crates/ui` (aucune dépendance de
> `archifier-repo` vers `archifier-ui`). Le test a donc été retiré des suites
> dépôt : il appartient au dev UI, conformément à §5.9 et §6.

## Codes d'erreur

| Code | Sens | Couverture |
|---|---|---|
| `DIFF-001` | révision absente ou objet inconnu | test 5, y compris `from == to` sur révision absente |
| `EXPORT-001` | domaine/objet inconnu | **non couvert** par les tests bloquants du §6 ; complément recommandé : `export_domain_markdown(ObjectId::from_parts(1, 999))` et `export_object_markdown` sur le même identifiant |
| `EXPORT-002` | diagramme illisible : coordonnée non finie ou `w/h ≤ 0` | test 12 (noyau et dépôt) |
| `DOC-LINK-001` | lien cassé (informationnel) | tests 13 et 15 : `BrokenLink.link.status == BROKEN`, aucune erreur d'écriture |
| `DOC-LINK-002` | lien archivé (informationnel) | test 14 : statut recalculé `ARCHIVED` malgré un cache `OK` |
| ~~`LINK-001`~~ | supprimé par §5.4 | aucun test ne s'y réfère |

`BrokenLink` n'expose pas de champ `code` : les codes `DOC-LINK-*` sont vérifiés
indirectement par `LinkStatus` (libellés UI `Cassé` / `Archivé`). Voir points
incertains.

Le graphe n'a pas de code d'erreur propre : `dependencies` borne silencieusement
1–6 (§5.9) ; `QUERY-DEPTH-001` est un sujet HTTP du lot 2.

## Vérifications transverses

1. **`arch://` uniquement dans le texte** : §5.2 autorise la citation dans une note
   de lien cassé, mais aucun **lien généré** ne doit rester `arch://`
   (`](arch://` interdit ; cibles extraites vérifiées).
2. **Liens relatifs valides** : toutes les cibles `](…)` sont extraites, les
   schémas externes (`http`, `https`, `mailto`) et chemins absolus sont refusés,
   puis la cible est normalisée (`./`, `..`) depuis le dossier du fichier et
   confrontée au `BTreeSet` des `path` du bundle.
3. **Chemins de bundle** : relatifs à `root`, séparateur `'/'`, pas de `\`, pas de
   `..`, pas de préfixe `<root>/`.
4. **Canonicalisation** : aucun BOM, aucun `\r`, saut de ligne final dans chaque
   fichier ; `files` trié par `path` (octets).
5. **Reproductibilité** : égalité stricte de deux bundles produits à horloges
   différentes ; aucune date d'export.
6. **SVG autonome** : `viewBox` calculé, pas de script, pas de ressource externe.
7. **Diagnostic recalculé** : cache `OK` périmé → `ARCHIVED` ; `stats()` aligné.

## Points d'API incertains

1. **`graph_route_parses` (UI)** : à écrire dans `crates/ui/src/router.rs`. Forme
   attendue : `Route::Graph { id }` (variante à nommer), `to_hash` produisant
   `#/graph/<id>`, round-trip `parse_hash(to_hash(route)) == route`, deux segments
   seulement, types admis élément et interface. `hash_roundtrip_for_each_route`
   doit inclure la nouvelle variante.
2. **Mermaid** : §1.2 dit `flowchart TD`, §5.7 dit `flowchart LR`. Les tests
   appliquent la décision de revue (`LR`) ; à confirmer côté moteur.
3. **Fenêtre de contexte** : §1.3 annonce « tronquée à 160 caractères », §5.4
   précise 80 avant + 80 après l'occurrence (URI comprise) plus `…`. Le test borne
   à `80 + len(target_uri) + 80 + 2` et exige que l'URI et le libellé amont
   restent visibles.
4. **`owner`** : §1.3 déclare `owner: ObjectRef`, §5.4 `Option<ObjectRef>`. Les
   tests suivent §5.4 (`as_ref().expect(...)`) ; un retour à `ObjectRef` casserait
   les tests 13 et 14.
5. **`DependencyNode`** : §5.9 ajoute `direction: Direction` et
   `lifecycle: Lifecycle`. Les tests 16 à 19 en dépendent ; l'ancienne structure
   ne compile pas contre ces tests.
6. **Chemins d'export** : §5.6 rend `ExportFile.path` relatif à `root`
   (`"index.md"`, `"elements/<id>.md"`), alors que §1.2 écrit `<root>/index.md`.
   Les tests suivent §5.6 et refusent tout `path` préfixé par `root`.
7. **Payload du diff** : §5.3 parle du snapshot « objet (`RepositoryObject`) »
   mais exclut `/object/revision` et `/object/updated_at`, ce qui suppose une
   racine `{ object: …, semantic_properties: … }`. Les tests 1 à 4 supposent cette
   forme (celle du snapshot actuel) et tolèrent les autres champs volatils
   (`updated_by`, `created_*`).
8. **Champs volatils** : seuls `/object/revision` et `/object/updated_at` sont
   explicitement exclus. Si le moteur exclut aussi `updated_by`, le test 4 reste
   vert ; le test 1 tolère ces champs.
9. **Coordonnée non finie** : impossible à écrire en JSON. Couverture noyau en
   mémoire (`f64::NAN`) + couverture dépôt via `w = 0.0`. Si le moteur valide
   `w/h` dès le chargement (`from_json`), la fixture devient illisible : il
   faudrait alors déplacer ce cas entièrement côté noyau.
10. **Codes `DOC-LINK-*`** : aucune API ne les expose (`BrokenLink` n'a pas de
    champ `code`). Les tests vérifient `LinkStatus` ; si un champ `code` est
    ajouté, ajouter les assertions correspondantes.
11. **Bandeau archivé** : texte exact `> **Archivé** — consultation seule,
    dernière révision` (§5.7), inséré comme première ligne après le titre. Une
    reformulation UI ferait échouer le test 11.
12. **`viewBox`** : dépend de la bbox + padding 8 et du format `{:.1}`. La valeur
   attendue `32.0 32.0 476.0 96.0` découle des coordonnées de la fixture ; toute
   modification de la fixture doit être répercutée.
13. **Index vide** : §5.8 omet les sections vides. Le test 10 exige l'absence de
    `## Éléments` mais n'impose pas que `index.md` soit le seul fichier du
    bundle (une page `domains/<id>.md` reste possible).
14. **Racine d'objet** : `slugify("Order Service v2") = "order-service-v2"`
    (§5.6) ; une racine basée sur l'id ferait échouer le test 9.
15. **`EXPORT-001`** : non couvert par les 20 tests du §6 ; candidat pour la
    troisième passe ou les tests HTTP du lot 2.
16. **Story dans l'export** : §5.1 reporte l'export Story en W1c (stub seulement
    si atteint par un lien de documentation) alors que §5.8 liste une section
    `## Histoires` dans `index.md`. Le test 6 n'exige ni la section ni la page,
    mais `assert_links_resolve` refuse tout lien d'index vers une page absente.

## Exécution prévue (troisième passe)

```powershell
cargo test -p archifier-repo --test history_diff_acceptance
cargo test -p archifier-repo --test markdown_export_acceptance
cargo test -p archifier-repo --test diagnostics_acceptance
```

Puis, côté UI : `cargo test -p archifier-ui router` (test 20). Aucun test existant
n'est modifié : `acceptance.rs`, `storage_boundary.rs` et
`openapi_acceptance.rs` restent inchangés.
