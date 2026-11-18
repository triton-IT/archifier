# Contrat W1c-1 — Hiérarchie, cadrage Story, comparaison de revues

| | |
|---|---|
| Date | 18 novembre 2026 |
| Portée | T1-09, T1-10, T1-11 |
| Statut | **Gelé pour W1c-1** |
| Prérequis | W1a/W1b verts |

## 1. Hiérarchie parent/enfant (T1-09)

```rust
// crates/repo/src/types.rs
pub struct HierarchyNode {
    pub object: ObjectRef,
    pub parent_id: Option<ObjectId>,
    pub depth: u8,
    pub archived_parent: bool,
}

pub struct Hierarchy {
    pub root: ObjectRef,
    pub children: Vec<HierarchyNode>,
    pub ancestors: Vec<ObjectRef>,
}

impl Repository {
    pub fn hierarchy(&self, id: ObjectId) -> RepoResult<Hierarchy>;
    pub fn set_parent(&self, child: ObjectId, parent: Option<ObjectId>) -> RepoResult<()>;
}
```

Règles :

- Seuls les `ModelElement` peuvent être enfant ou parent ; les autres types ⇒ `HIER-001`.
- Parent et enfant doivent partager la même notation ⇒ `HIER-002` (couloirs BPMN entre eux, ports UML entre eux).
- Profondeur maximale **5** ; au-delà ⇒ `HIER-003`.
- Cycle interdit ⇒ `HIER-004`.
- Auto-parenté interdit ⇒ `HIER-004`.
- Parent archivé : accepté mais signalé `archived_parent = true` (pas de code d'erreur ; le libellé UI suffit).
- `set_parent` met à jour `parent_id`, `revision`, `updated_at`, `updated_by`, crée une révision et un audit.
- `hierarchy` renvoie `root` = objet interrogé, `ancestors` du parent direct vers la racine (ordre proche → lointain), `children` directs triés par nom puis id.
- UI fiche élément : sélecteur de parent, liste des enfants, badge « parent archivé ».

## 2. Cadrage Story et épinglage (T1-10)

- `MiniDiagram` reçoit `DiagramFraming` : `zoom` (0.5–3.0, défaut 1.0), `focus_element_ids`, `dimmed_element_ids`, `caption`.
- Helper core exact :
  ```rust
  pub struct FramedNode { pub object_id: ObjectId, pub highlighted: bool, pub opacity: f32 }
  pub struct FramedDiagram { pub zoom: f32, pub nodes: Vec<FramedNode>, pub caption: Option<String> }
  pub fn apply_framing(framing: &DiagramFraming, layout: &DiagramLayout) -> FramedDiagram;
  ```
- Rendu : zoom appliqué à l'échelle, focus mis en avant, éléments atténués (opacité 0.35), légende affichée.
- Aucun impact sur le layout source.
- Bouton **Épingler les révisions** sur la Story : enregistre `StoryStructure.pinned_revisions` pour les objets référencés (diagrammes, éléments, interfaces) à leur révision courante.
- Bouton **Dégeler** : vide `pinned_revisions`.
- Un récit épinglé affiche un badge **Récit figé** et la date de la révision la plus récente.
- API :
  ```rust
  impl Repository {
      pub fn pin_story_revisions(&self, story_id: ObjectId) -> RepoResult<usize>;
      pub fn unpin_story_revisions(&self, story_id: ObjectId) -> RepoResult<()>;
  }
  ```
- `pin_story_revisions` / `unpin_story_revisions` :
  - collecte **uniquement** les objets référencés par les blocs : `diagram_id`, `element_id`, `interface_id` (pas les nœuds de diagramme, pas les versions d'interface) ;
  - enregistre `StoryStructure.pinned_revisions` avec la révision courante ;
  - réutilise `save_story` ; retourne le nombre d'épingles ;
  - une Story sans référence ⇒ `Ok(0)` ;
  - `unpin` sur une carte vide ⇒ `Ok(())` (idempotent).
- `save_story` reste le chemin d'écriture ; les deux méthodes ci-dessus le réutilisent.

## 3. Comparaison de deux revues (T1-11)

```rust
pub struct ReviewComparison {
    pub from: ObjectRef,
    pub to: ObjectRef,
    pub window_from: Option<(OffsetDateTime, Option<OffsetDateTime>)>,
    pub window_to: Option<(OffsetDateTime, Option<OffsetDateTime>)>,
    pub exposed_added: Vec<InterfaceRow>,
    pub exposed_removed: Vec<InterfaceRow>,
    pub consumed_added: Vec<InterfaceRow>,
    pub consumed_removed: Vec<InterfaceRow>,
    pub anomalies_added: Vec<Anomaly>,
    pub anomalies_removed: Vec<Anomaly>,
}

impl Repository {
    pub fn compare_reviews(&self, from: ObjectId, to: ObjectId) -> RepoResult<ReviewComparison>;
}
```

- Les ensembles sont comparés par `InterfaceRow.id` / `Anomaly.code+object_id`.
- Tri stable par nom puis id.
- Revues inconnues **ou objets non-Review** ⇒ `REVIEW-001`.
- UI revue : sélecteur d'une seconde revue, bouton **Comparer**, sections ajoutées/retirées, anomalies, fenêtres temporelles.
- Les filtres sont ceux des `ReviewDetail` existants ; aucune écriture.

## 4. Tests bloquants

- `hierarchy_depth_is_limited_to_five`
- `hierarchy_cycle_is_refused`
- `archived_parent_is_reported`
- `hierarchy_requires_same_notation`
- `story_pins_revision`
- `story_unpin_clears_pins`
- `story_framing_applies_zoom_focus_dim` (helper core testable)
- `review_comparison_shows_filter_and_window_diff`
- `review_comparison_lists_anomalies`
- `review_comparison_rejects_unknown_review`

## 5. Périmètre fichiers

- `crates/repo/src/hierarchy.rs` (nouveau)
- `crates/repo/src/review_compare.rs` (nouveau)
- `crates/repo/src/lib.rs` (intégration, écrivain unique)
- `crates/repo/src/types.rs`
- `crates/core/src/story.rs` (nouveau helper de framing) ou `model.rs` si un helper y trouve sa place
- `crates/ui/src/views/element.rs`
- `crates/ui/src/views/story.rs`
- `crates/ui/src/views/review.rs`
- `crates/ui/src/components.rs`
- `crates/repo/tests/hierarchy_acceptance.rs`, `story_acceptance.rs`, `review_compare_acceptance.rs` (testeur)

## 6. Hors périmètre

- Multi-onglets (T1-08) : W1c-2, contrat séparé.
- Migration de métamodèle 4.x : W2.
- Éditeur complet de Story : W1c se limite au cadrage et à l'épinglage.

## 7. Décisions de revue (gelées)

### 7.1 Profondeur et codes

- Profondeur = nombre d'**arêtes** depuis la racine ; racine `depth = 0`, maximum **5 arêtes** (6 éléments). `depth_of` doit renvoyer 0 pour la racine.
- `HIER-001` type non hiérarchisable, `HIER-002` notation/métamodèle différents, `HIER-003` profondeur > 5, `HIER-004` cycle/auto-parenté, `HIER-005` archivage d'un parent avec enfants actifs.
- Correspondance : `HIER-005` ≙ `MM-HIER-003` de la spec technique.
- `Native` n'est pas hiérarchisable (`HIER-002`) ; `Grouping` ArchiMate également exclu.
- Enfants triés par nom (insensible à la casse) puis id ; ancêtres du parent direct vers la racine.

### 7.2 Parent archivé

- `set_parent` vers un parent archivé est **accepté** et signale `parent_archived = true` (pas d'erreur).
- `archive(parent)` est **refusé** si des enfants actifs existent (`HIER-005`), conformément à `MM-HIER-003`.
- `Hierarchy` expose `parent: Option<ObjectRef>` et `parent_archived: bool` pour que la fiche d'un enfant feuille puisse afficher le badge.
- Le test d'archivage doit archiver l'enfant, puis le parent, puis restaurer l'enfant.

### 7.3 Épinglage

- `pinned_revisions` fige les objets référencés (`diagram_id`, `element_id`, `interface_id`).
- Pour une interface, `StoryStructure.pinned_versions: BTreeMap<ObjectId, ObjectId>` (additif, `serde(default)`) fige la version du bloc si `version_id` est présent ; sinon la dernière version publiée, sinon aucune.
- `playback` doit exposer les pins (`pinned_revisions`, `pinned_versions`) et l'UI affiche « Récit figé » avec la date de la révision la plus récente.
- `pin_story_revisions` vérifie que chaque révision existe dans `revision_snapshot` ; sinon `STORY-003` et aucune écriture.
- `unpin` vide les deux cartes ; no-op sans écriture si déjà vide.
- `save_story` valide une **copie** avant de muter ; en cas d'erreur, l'état mémoire reste inchangé.

### 7.4 Comparaison de revues

```rust
pub struct TemporalWindow {
    pub start: Option<OffsetDateTime>,
    pub end: Option<OffsetDateTime>,
}

pub struct ReviewComparison {
    // … champs précédents …
    pub window_from: Option<TemporalWindow>,
    pub window_to: Option<TemporalWindow>,
    pub filter_from: serde_json::Value,
    pub filter_to: serde_json::Value,
}
```

- La comparaison porte sur les **instantanés de filtres** et les fenêtres, plus l'appartenance exposée/consommée calculée à `now`. L'application des filtres est W5.
- `window_from`/`window_to` conservent une fenêtre avec seulement une fin.
- Identité d'anomalie = `(code, object_id, detail)` pour ne pas masquer deux versions expirées.
- `ReviewComparison` expose `exposed_changed` / `consumed_changed` pour les lignes présentes des deux côtés mais différentes (`InterfaceRow: PartialEq`).
- `REVIEW-001` pour revue inconnue ou objet non-Review. `from == to` renvoie une comparaison vide. Une comparaison inverse (`to` antérieure à `from`) est autorisée : les listes ajoutées/retirées et les fenêtres sont échangées, sans erreur.

### 7.5 Cadrage Story

- `apply_framing` est le helper public ; `FramedDiagram { zoom, nodes, caption }`, `FramedNode { object_id, highlighted, opacity }`.
- Stockage du zoom libre ; clamp 0.5–3.0 au rendu.
- `MiniDiagram` calcule le fit sur le layout **non zoomé**, puis applique `zoom` (viewBox + transform) ; l'auto-fit ne doit pas annuler le zoom.
- Focus : `highlighted = true` ; atténuation : opacité 0.35 ; légende affichée.

