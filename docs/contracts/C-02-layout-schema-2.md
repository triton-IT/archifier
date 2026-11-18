# C-02 — `DiagramLayout` schéma 2 et migration depuis le schéma 1

| | |
|---|---|
| **Réf.** | C-02 |
| **Version** | 1.0 — figé G0, 18/11/2026 |
| **Propriétaire** | Dev « Atelier & commandes » — `crates/core/src/model.rs` |
| **Sources** | `technical-specifications.md` §14.1, §14.4.1, §14.4.5 ; VD-02, VD-09, VD-10, VD-12, VD-13 |
| **Dépend de** | C-01 (`DraftShape`, `ViewFraming`), C-03 (`ViewIntent`), C-04, C-07 |

## 1. Règles générales

1. Le schéma 2 est **additif** : tout champ du schéma 1 reste lisible et garde son sens.
2. Tout champ nouveau porte `#[serde(default)]` dans le modèle Rust. Un layout schéma 1
   se désérialise donc **sans erreur**, puis se migre explicitement.
3. La migration ne touche **aucune géométrie** : les positions et tailles sont conservées
   au bit près. Elle n'écrit aucun objet du référentiel.
4. La migration est **idempotente** : un layout déjà en schéma 2 est rendu inchangé.
5. Aucune donnée n'est supprimée : un champ inconnu est ignoré à la lecture mais le
   layout migré est réécrit complet ; l'export de snapshot conserve les objets.

## 2. Objet racine — schéma 2

```json
{
  "schema": 2,
  "intent": "APPLICATION_LANDSCAPE",
  "scope": { "kind": "DOMAIN", "domain_id": "01J000000000000000000000PA" },
  "notation": "ARCHIMATE",
  "temporal_cursor": "2027-08-01T00:00:00Z",
  "viewport": { "x": 0.0, "y": 0.0, "zoom": 1.0 },
  "grid": { "size": 8, "snap": true },
  "layers": [
    { "id": "business", "label": "Métier", "y": 0, "h": 240,
      "collapsed": false, "tone": "business" },
    { "id": "application", "label": "Applicatif", "y": 240, "h": 280,
      "collapsed": false, "tone": "application" }
  ],
  "zones": [
    { "id": "z1", "kind": "DOMAIN", "domain_id": "01J000000000000000000000PA",
      "x": 40, "y": 260, "w": 420, "h": 240, "label": "Paiements",
      "layer_id": "application", "boundary": true, "collapsed": false,
      "tone": "domain-1" },
    { "id": "z2", "kind": "GROUP", "domain_id": null,
      "x": 40, "y": 40, "w": 420, "h": 180, "label": "Parcours client",
      "layer_id": "business", "boundary": false, "collapsed": false,
      "tone": null }
  ],
  "framing": {
    "zoom": 1.0, "x": 0.0, "y": 0.0,
    "focus": ["n1", "n4"], "dim_others": true, "caption": null
  },
  "nodes": [
    { "id": "n1", "object_id": "01J000000000000000000000PS",
      "kind": "ELEMENT", "version_id": null,
      "x": 80, "y": 300, "w": 160, "h": 72,
      "layer_id": "application", "zone_id": "z1", "collapsed": false,
      "port": null,
      "style_override": { "fill": null, "stroke": null } },
    { "id": "n2", "object_id": "01J000000000000000000000PI",
      "kind": "INTERFACE", "version_id": "01J00000000000000000000V20",
      "x": 500, "y": 380, "w": 24, "h": 24,
      "layer_id": "application", "zone_id": "z1", "collapsed": false,
      "port": { "zone_id": "z1", "side": "EAST", "offset": 0.42 },
      "style_override": {} }
  ],
  "edges": [
    { "id": "e1", "relationship_id": "01J000000000000000000000RE",
      "kind": "RELATIONSHIP",
      "waypoints": [{ "x": 280, "y": 116 }],
      "label_visible": true, "routing": "ORTHOGONAL",
      "layer_id": null }
  ],
  "drafts": [
    { "id": "d1", "label": "Service de paiement", "shape": "RECT",
      "x": 200, "y": 340, "w": 160, "h": 72,
      "suggested_metaclass": "ApplicationService",
      "suggested_domain_id": "01J000000000000000000000PA",
      "status": "SKETCH", "layer_id": "application", "zone_id": "z1",
      "attachment": null, "interface_id": null, "version_id": null }
  ],
  "annotations": [
    { "id": "a1", "text": "Migration v1 → v2", "x": 40, "y": 20,
      "w": 320, "h": 180, "style": "CALLOUT" }
  ]
}
```

## 3. Champs — règle opposable

| Champ | Type | Défaut schéma 2 | Règle |
|---|---|---|---|
| `schema` | entier | `2` | Refus `DGM-013` si `> 2`. Un `1` déclenche la migration. |
| `intent` | `ViewIntent` | voir §5 | Catalogue fermé (C-03). Une vue sans intention est refusée à la création (`DGM-014`). |
| `scope` | objet | `{ "kind": "GLOBAL" }` | `DOMAIN` exige `domain_id` existant et actif. |
| `notation` | `DiagramNotation` | valeur existante | Inchangé ; sert à filtrer la palette et les relations. |
| `temporal_cursor` | RFC 3339 optionnel | `null` | **N'altère aucune donnée** ; n'affecte que le rendu (VD-12). |
| `viewport` | objet | `{0,0,1}` | Pan/zoom courants ; ne fait pas partie du cadrage partagé. |
| `grid` | objet | `{ "size": 8, "snap": true }` | `size ∈ {4,8,16}` ; `snap` pilote l'aimantation. |
| `layers` | liste | selon intention | `id` unique ; bandes ordonnées par `y` croissant. Une couche est un **repère**, jamais un objet. |
| `zones` | liste | `[]` | `DOMAIN` référence un domaine existant ; `GROUP` est visuel. |
| `framing` | objet | `{ zoom:null, x:0, y:0, focus:[], dim_others:false, caption:null }` | Persisté, exportable, restitué par lien partagé (VD-13). |
| `nodes` | liste | `[]` | `object_id` obligatoire ; `kind` ∈ `ELEMENT \| INTERFACE \| DIAGRAM_REF \| GROUP`. |
| `edges` | liste | `[]` | `relationship_id` obligatoire ; une arête ne crée jamais de relation. |
| `drafts` | liste | `[]` | Croquis **locaux** ; jamais exportés en objet, jamais publiables (C-04). |
| `annotations` | liste | `[]` | Inchangé ; `style` reste un jeton (`CALLOUT`, `NOTE`). |

### 3.1 Champs ajoutés à `DiagramNode`

| Champ | Type | Défaut | Règle |
|---|---|---|---|
| `layer_id` | string optionnel | `null` | Une forme hors bande est autorisée (avertissement visuel, pas une erreur). |
| `zone_id` | string optionnel | `null` | Zone d'appartenance visuelle. Un port appartient à la zone de sa frontière. |
| `collapsed` | bool | `false` | Pliage d'un groupe ou d'un conteneur. |
| `port` | objet optionnel | `null` | `{ zone_id, side, offset }`. `side ∈ NORTH\|SOUTH\|EAST\|WEST`, `offset ∈ [0,1]`. Présent uniquement pour un nœud `INTERFACE` posé sur une frontière. |

### 3.2 Champs ajoutés aux arêtes et zones

| Champ | Porte sur | Type | Défaut | Règle |
|---|---|---|---|---|
| `routing` | arête | enum | `ORTHOGONAL` | `ORTHOGONAL \| STRAIGHT`. Pas de routage automatique. |
| `layer_id` | arête, zone, croquis | string optionnel | `null` | Repère de lecture. |
| `tone` | zone | string optionnel | `null` | Jeton de couleur de domaine (`domain-1..6`), jamais une couleur libre. |
| `boundary` | zone | bool | `true` pour `DOMAIN` | Une frontière accepte des ports (`DSG-004`). |
| `collapsed` | zone | bool | `false` | Repliage visuel ; les ports restent ancrés. |

## 4. Métadonnées de vue portées par l'objet `Diagram`

Le layout porte la grammaire ; l'objet porte l'identité éditoriale. Deux champs additifs
sont ajoutés à `Diagram` (sérialisés avec `#[serde(default)]`, donc snapshot lisible) :

| Champ | Type | Défaut | Règle |
|---|---|---|---|
| `publication_status` | `PublicationStatus` | `DRAFT` | `PublishView` le fait passer à `PUBLISHED` ; `WITHDRAWN` retire le lien public. |
| `published_at` | RFC 3339 optionnel | `null` | Renseigné à la première publication ; effacé au retrait. |

`lifecycle` reste distinct : archiver n'est pas dépublier, publier n'est pas activer.

## 5. Migration schéma 1 → schéma 2

Fonction pure `migrate_layout_v1_to_v2(layout, ctx) -> (DiagramLayout, Vec<MigrationNote>)`,
`ctx` portant la notation de la vue et le domaine responsable. Algorithme exact :

1. `schema` : `1` → `2`.
2. `intent` : dérivée de la **notation**, jamais du titre :
   - `ARCHIMATE` → `APPLICATION_LANDSCAPE`
   - `BPMN` → `PROCESS`
   - `UML` → `DEPLOYMENT`
   - `FREE` → `CONTEXT`

   Une note `INTENT_DEFAULTED` est produite ; l'Atelier affiche une invitation non
   bloquante « Préciser l'intention » dans la barre d'état. Aucun modal.
3. `scope` : `{ "kind": "DOMAIN", "domain_id": ctx.responsible_domain_id }`.
4. `notation` : conservée.
5. `temporal_cursor` : `null`.
6. `viewport` : conservé tel quel.
7. `grid` : `{ "size": 8, "snap": true }`.
8. `layers` : construites depuis le profil de la notation, dans l'ordre `order` du profil,
   avec les hauteurs par défaut de C-03 (`h` = 240 si le profil n'en déclare pas).
9. `zones` : vide. Aucune zone implicite n'est créée à partir du domaine responsable.
10. `framing` : `{ zoom: viewport.zoom, x: viewport.x, y: viewport.y, focus: [],
    dim_others: false, caption: null }`.
11. `nodes` : chaque nœud conservé avec `x`, `y`, `w`, `h`, `object_id`, `kind`,
    `version_id`, `style_override`. `collapsed = false`, `zone_id = null`, `port = null`.
    `layer_id` est renseigné **si** le contexte fournit la couche de la classe de l'objet ;
    sinon `null` (jamais deviné depuis `y`).
12. `edges` : conservées ; `routing = ORTHOGONAL`, `layer_id = null`.
13. `drafts` : vide. Aucun nœud existant n'est transformé en croquis.
14. `annotations` : conservées à l'identique.
15. Écriture d'une seule révision « Migration de la vue au schéma 2 », une seule fois.

### 5.1 Interdits de migration

- Déplacer, redimensionner, réordonner un nœud.
- Supprimer un nœud orphelin (`DGM-001` reste un avertissement de lecture).
- Créer un domaine, une zone de domaine ou une interface.
- Convertir une annotation en croquis, ou l'inverse.
- Écrire un `intent` déduit du **titre** ou du contenu.

## 6. Validation du layout (ajouts)

Les codes existants restent inchangés (§14.2). Ajouts, tous vérifiés par
`validate_layout` :

| Code | Sévérité | Règle |
|---|---|---|
| `DGM-007` | erreur | Croquis : `id` dupliqué, géométrie non finie ou dimensions ≤ 0. |
| `DGM-008` | erreur | Zone `DOMAIN` sans `domain_id`, ou référençant un domaine inexistant. |
| `DGM-008-W` | avertissement | Zone `DOMAIN` référençant un domaine archivé. |
| `DGM-009` | erreur | `node.zone_id` ou `zone.layer_id` / `node.layer_id` référençant un identifiant absent. |
| `DGM-010` | avertissement | Bandes qui se chevauchent ou `id` de couche dupliqué. Non bloquant : c'est un repère de lecture. |
| `DGM-011` | avertissement | Nœud avec `port` dont la géométrie n'est pas sur le côté déclaré de la frontière. |
| `DGM-012` | avertissement | `temporal_cursor` illisible : le rendu retombe sur « maintenant ». |
| `DGM-013` | erreur | `schema > 2` (vue produite par une version plus récente). |
| `DGM-014` | erreur | `intent` absent hors migration (création directe). |
| `DGM-015` | avertissement | Croquis lu avec un statut `FORMALIZED`/`VALID` (jamais écrit par l'Atelier) ; traité comme `SKETCH`. |

`DGM-010`, `DGM-011`, `DGM-012` et `DGM-008-W` n'empêchent pas l'enregistrement : ils
sont visibles dans la barre d'état, jamais dans un modal.

## 7. Budget et rendu

- 1 000 nœuds maximum par vue (`DGM-006`, inchangé).
- Le curseur temporel ne recalcule que les styles d'état (VD-12, NFR-030).
- Le cadrage applique un `transform` racine ; il n'écrit pas le layout (sauf commande
  `SetFraming` explicite).
- Les bandes et zones sont rendues **avant** les nœuds, dans des groupes distincts.

## 8. Tests obligatoires (noms figés)

| Test | Prouve |
|---|---|
| `v1_layout_deserializes_without_error` | additivité |
| `migration_preserves_node_geometry` | VD-02, NFR-021 |
| `migration_is_idempotent` | VD-02 |
| `migration_defaults_intent_from_notation` | C-02 §5.2 |
| `migration_creates_no_zone_and_no_object` | DSG-005 |
| `unknown_future_schema_is_refused` | DGM-013 |
| `duplicate_draft_id_is_refused` | DGM-007 |
| `zone_domain_must_exist` | DGM-008 |
| `port_outside_boundary_warns_without_blocking` | DGM-011, VD-10 |
| `temporal_cursor_does_not_mutate_layout` | VD-12 |
| `layout_roundtrip_preserves_ports_and_layers` | VD-09, VD-10 |
