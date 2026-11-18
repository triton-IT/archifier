# C-09 — Surface `Repository` : gel et évolutions additives

| | |
|---|---|
| **Réf.** | C-09 |
| **Version** | 1.1 — 18/11/2026 (ajouts additifs d'implémentation) |
| **Propriétaire** | Orchestrateur (gel) ; implémentation : Dev Atelier (`crates/repo`) |
| **Sources** | ADR-016, `technical-specifications.md` §15.3, `crates/repo/src/lib.rs`, `crates/ui/tests/architecture.rs` ; VD-03, VD-04, VD-15 |
| **Dépend de** | C-01, C-02, C-06 |

## 1. Règle

`Repository` reste le **port de dépôt unique** (ADR-016). L'Atelier ne touche jamais
IndexedDB, ni `MemoryStore`, ni `SnapshotStore`. Une vue qui appelle `save_layout` ou
`create_element` directement contourne le bus de commandes : interdit.

**Aucune méthode existante n'est renommée, retirée ou change de signature** dans le MVP3.
Les seules évolutions autorisées sont les méthodes **additives** approuvées ci-dessous
(§3), chacune avec ses tests.

## 2. Surface gelée (lecture)

```text
new, from_json, refresh, metamodel, export_json, load_json
set_now_ms, now_ms, now, set_current_user, current_user, users
persist_error, profile_error, storage_evicted, store_error, reset_storage
domains, domain, domain_overview
elements, element, element_detail, dependencies
interfaces, interface_detail, timeline
diagrams, diagram_detail
stories, story_detail, playback
reviews, review_detail
relationship_endpoints, usages
documentation, documentation_history, history, revision_snapshot, revisions,
revision_diff, comparable_revisions
search, all_objects, palette, stats, audit, comments, object_ref, template_choices
```

## 3. Surface gelée (écriture)

```text
add_user
create_domain, update_domain, archive, restore
create_element, update_element
create_relationship
create_interface, create_interface_version, publish_version, add_consumer
create_diagram, save_layout, add_node, set_node_position, add_edge, remove_node
set_documentation
create_story, save_story, pin_story_revisions, unpin_story_revisions
create_review, set_review_status, add_comment
apply_changeset, apply_operations, make_changeset, preview_changeset
```

Les méthodes `add_node`, `set_node_position`, `add_edge`, `remove_node` restent exposées
pour compatibilité (lot 1/2, tests existants) mais **ne sont plus appelées par l'Atelier** :
elles modifient le layout hors bus. Le test d'architecture le vérifie.

## 4. Évolutions additives approuvées à G0

### 4.1 `save_layout_expect` — écriture optimiste

```rust
pub fn save_layout_expect(
    &self,
    diagram_id: ObjectId,
    layout: DiagramLayout,
    expected_revision: Revision,
) -> RepoResult<()>;
```

- Valide la vue et le layout exactement comme `save_layout` (mêmes codes `DGM-*`).
- Si `diagram.object.revision != expected_revision` : `RepoError` de code **`REV-409`**,
  aucune écriture, aucune révision incrémentée.
- Sinon : écrit, incrémente la révision, enregistre l'événement d'audit.
- `save_layout` reste inchangé et non déprécié.
- Tests : `save_layout_expect_writes_on_matching_revision`,
  `save_layout_expect_returns_409_without_write`, `save_layout_expect_does_not_touch_other_views`.

### 4.2 `view_cards` — couvertures de la Bibliothèque

```rust
pub struct ViewFilter {
    pub domain_id: Option<ObjectId>,
    pub intent: Option<ViewIntent>,
    pub notation: Option<DiagramNotation>,
    pub publication: Option<PublicationStatus>,
    pub owner: Option<String>,
    pub query: Option<String>,        // recherche lexicale sur le titre
}

pub struct MiniNode { pub x: f64, pub y: f64, pub w: f64, pub h: f64, pub kind: NodeKind }

pub struct ViewCard {
    pub id: ObjectId,
    pub title: String,
    pub domain_id: ObjectId,
    pub domain_name: String,
    pub intent: ViewIntent,
    pub notation: DiagramNotation,
    pub node_count: usize,
    pub draft_count: usize,
    pub publication_status: PublicationStatus,
    pub owner: String,
    pub updated_at: OffsetDateTime,
    pub framing: ViewFraming,
    /// Au plus 40 nœuds, triés par identifiant, pour dessiner une vignette fidèle.
    pub mini_nodes: Vec<MiniNode>,
}
```

- Lecture seule, tri déterministe : `updated_at` décroissant, puis `id`.
- Aucun layout complet n'est exposé : la vignette est bornée.
- `query` vide ou absente = pas de filtre.
- Tests : `view_cards_are_sorted_and_filtered`, `view_cards_vignette_is_bounded`,
  `view_cards_expose_intent_and_publication`.

### 4.3 `rename_interface` — renommage depuis la vue

```rust
pub fn rename_interface(&self, id: ObjectId, name: &str) -> RepoResult<()>;
```

- Nom obligatoire (`OBJ-002`), interface existante et active (`IFACE-*`).
- Incrémente la révision de l'objet, enregistre l'audit.
- Ne change ni la clé logique, ni les versions, ni les consommateurs.
- Tests : `rename_interface_updates_name_only`,
  `rename_interface_does_not_change_logical_key`, `rename_interface_keeps_versions`.

### 4.4 `set_diagram_publication` — publier / retirer

```rust
pub fn set_diagram_publication(
    &self,
    id: ObjectId,
    status: PublicationStatus,
) -> RepoResult<()>;
```

- `DRAFT → PUBLISHED` renseigne `published_at` (première fois) ;
  `PUBLISHED → WITHDRAWN` efface `published_at` ;
  `WITHDRAWN → PUBLISHED` republie avec une nouvelle date.
- Une vue archivée ne peut pas être publiée (`DGM-000`).
- La règle métier « aucune publication avec croquis » (`DSG-003`) est vérifiée par le
  **bus**, pas par le dépôt : le dépôt reste mécanique.
- Tests : `publish_diagram_sets_status_and_date`,
  `withdraw_diagram_clears_published_at`, `archived_view_cannot_be_published`.

### 4.5 `interface_of_version` — résolution `arch://version`

```rust
pub fn interface_of_version(&self, version: ObjectId) -> Option<ObjectId>;
```

- Lecture seule, O(1) via l'index existant `versions_by_interface`.
- Sert uniquement à résoudre `#/view/versions/:id` vers la fiche d'interface.
- Tests : `interface_of_version_resolves_owner`, `unknown_version_returns_none`.

### 4.6 `update_relationship_endpoints` — reconnexion depuis la vue

```rust
pub fn update_relationship_endpoints(
    &self,
    id: ObjectId,
    source_id: ObjectId,
    target_id: ObjectId,
) -> RepoResult<()>;
```

- Mêmes règles de métamodèle que `create_relationship` (`MM-REL-002`, `MM-REL-003`).
- Met à jour la révision et l'audit ; ne touche pas au type de relation.
- Nécessaire à `ViewCommand::Reconnect` (C-01 §5.1).
- Tests : `reconnect_forbidden_relation_keeps_edge_intact` (core),
  `update_relationship_endpoints_validates_metamodel` (repo).

### 4.7 `relationship` — lecture pour le rendu et la fiche

```rust
pub fn relationship(&self, id: ObjectId) -> Option<Relationship>;
```

- Lecture seule, nécessaire à `ViewWorld::relationship_type` et à
  `#/view/relationships/:id`.
- Aucune écriture, aucune méthode de liste nouvelle.
- Tests : `relationship_route_resolves_arch_uri` (UI), fiche relation manuelle.

## 5. Ce qui reste interdit

- Un `Repository::save_view_command` ou tout bus logé dans le dépôt : le bus est dans
  `core` + adaptateur UI.
- Un accès au dépôt depuis `canvas/**`, `inspector/**` ou une vue.
- Une méthode qui écrit plusieurs objets sans transaction logique : les effets d'une
  commande sont appliqués dans l'ordre (C-01 §5) et le layout n'est persisté qu'après
  succès de tous les effets.
- Une méthode qui supprime un objet depuis l'Atelier.

## 6. État local de l'UI (hors dépôt)

`localStorage` ne contient que des préférences, jamais de données métier dupliquées.

| Clé | Contenu | Propriétaire |
|---|---|---|
| `archifier:last_view` | JSON `{ "schema": 1, "view_id": "01J…", "posture": "DESIGN", "framing": {…}, "t": null, "at_ms": 0 }` | `state::last_view` |
| `archifier:force_empty` | démarrage à vide demandé | `state` |
| `archifier:demo_purged` | jeu de démonstration historique effacé | `boot` |
| `archifier:locale` | `fr-FR` (VD-25) | `i18n` |

Règles :

- `archifier:last_view` est écrit à chaque autosave réussi (≤ 1 s au repos) et à la
  sortie de vue.
- Une entrée illisible ou d'une version inconnue est **ignorée** : `/` retombe sur
  « Nouvelle vue », jamais sur une erreur.
- Un accès `localStorage` est cantonné au module `state::last_view` ; le test
  `ui_views_do_not_touch_storage` reste vert (il autorise `boot` et un unique module
  d'état local, à mettre à jour explicitement dans le même lot).

## 7. Tests d'architecture (à étendre en W1)

| Test | Règle |
|---|---|
| `ui_views_do_not_touch_storage` | inchangé, liste de modules autorisés mise à jour une seule fois |
| `idb_store_is_only_known_by_boot` | inchangé |
| `memory_fallback_is_documented_exception` | inchangé |
| `canvas_never_writes_layout_directly` | **nouveau** |
| `inspector_never_writes_repository` | **nouveau** |
| `every_layout_write_goes_through_commands` | **nouveau** |
| `repository_surface_is_frozen` | **nouveau** : liste exhaustive des `pub fn` comparée à C-09 |
| `no_hardcoded_string_in_ui` | **nouveau** (VD-25) |
| `no_hardcoded_token_in_components` | **nouveau** (C-07) |

## 8. Journal des évolutions

| Date | Révision | Ajout |
|---|---|---|
| 18/11/2026 | 1.0 | `save_layout_expect`, `view_cards`, `rename_interface`, `set_diagram_publication`, `interface_of_version` approuvées à G0 |
| 18/11/2026 | 1.1 | Ajouts additifs d'implémentation : `update_relationship_endpoints`, `relationship` (lecture). Aucun retrait, aucune signature modifiée. |
