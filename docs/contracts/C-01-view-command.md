# C-01 — `ViewCommand` : bus de commandes de l'Atelier

| | |
|---|---|
| **Réf.** | C-01 |
| **Version** | 1.0 — figé G0, 18/11/2026 |
| **Propriétaire** | Dev « Atelier & commandes » — `crates/core/src/view_command.rs` |
| **Sources** | `technical-specifications.md` §14.4.2, §14.4.3, §14.4.4 ; ADR-022 §6 ; VD-03, VD-04 |
| **Dépend de** | C-02 (layout), C-03 (`ViewIntent`), C-04 (`DraftStatus`), C-08 (codes), C-09 (dépôt) |

## 1. Principe

Toute mutation d'une vue — géométrie, croquis, relation, zone, cadrage, temporalité,
publication — est une **commande sérialisable** validée par `archifier-core` **avant**
toute projection. L'interface ne modifie jamais `DiagramLayout` directement.

```text
geste UI ──▶ ViewCommand ──▶ validate (core, pur) ──▶ CommandReceipt
                                                        ├─ layout projeté (pur)
                                                        ├─ effets dépôt ordonnés
                                                        └─ révision attendue
                                      │
                                      └─▶ Rejected { code, path, message }   (vue intacte)
```

Trois invariants bloquants :

1. **Aucune écriture de layout hors commande.** Un test d'architecture interdit
   `save_layout(` dans `crates/ui/src/canvas/**` et `crates/ui/src/design/**` hors du
   module d'exécution `design::commands`.
2. **Refus = vue intacte.** Un `Rejected` ne produit ni effet dépôt ni layout modifié.
3. **Idempotence par `command_id`.** Une commande déjà appliquée (`Applied` ou
   `Ignored::Duplicate`) ne produit jamais deux fois le même effet.

## 2. Types exacts

```rust
pub type CommandId = ObjectId;      // ULID, alloué par l'appelant avant émission
pub type NodeId = String;           // stable dans la vue, préfixe "n"
pub type EdgeId = String;           // préfixe "e"
pub type ZoneId = String;           // préfixe "z"
pub type DraftId = String;          // préfixe "d"

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewCommand {
    /// Identité de la commande : clé d'idempotence et de rejeu.
    pub command_id: CommandId,
    /// Vue cible. Une commande ne s'applique jamais à une autre vue.
    pub view_id: ObjectId,
    /// Révision de vue attendue (contrôle optimiste, VD-04). `None` = dernière connue.
    pub expected_revision: Option<Revision>,
    pub body: ViewCommandBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViewCommandBody {
    PlaceDraft {
        draft: DraftShape,
    },
    PlaceExisting {
        object_id: ObjectId,
        at: Point,
        size: Size,
        /// Renseigné quand la forme est un port posé sur une frontière (VD-10).
        #[serde(default)]
        attachment: Option<Attachment>,
    },
    Rename {
        target: Target,
        label: String,
    },
    Formalize {
        draft_id: DraftId,
        metaclass: String,
        domain_id: ObjectId,
        #[serde(default)]
        properties: PropertyBag,
        /// Identité de l'objet créé. Obligatoire quand la formalisation crée un objet
        /// (`CMD-002` sinon) ; ignorée quand un port rattache une interface existante.
        #[serde(default)]
        new_object_id: Option<ObjectId>,
        /// Identité de la version d'interface, si la formalisation en crée une.
        #[serde(default)]
        new_version_id: Option<ObjectId>,
    },
    Connect {
        from: EndPoint,
        to: EndPoint,
        relation_type: String,
        /// Identité de la relation créée. Fournie par l'appelant : rejeu = même relation.
        new_relationship_id: ObjectId,
    },
    Reconnect {
        edge_id: EdgeId,
        to: EndPoint,
    },
    MoveNode {
        node_id: NodeId,
        delta: Point,
    },
    ResizeNode {
        node_id: NodeId,
        size: Size,
    },
    CreateZone {
        zone: DraftZone,
    },
    SetFraming {
        framing: ViewFraming,
    },
    SetTemporalCursor {
        #[serde(with = "time::serde::rfc3339::option", default)]
        at: Option<OffsetDateTime>,
    },
    PublishView {
        visibility: Visibility,
    },
    DeleteRepresentation {
        target: Target,
    },
}
```

> **Note d'écart (additive, sans retrait).** §14.4.2 définit l'intention de chaque
> variante mais ne porte ni `command_id`, ni `view_id`, ni `expected_revision`, ni les
> identités des objets créés. Ces champs sont **ajoutés** pour satisfaire VD-03
> (idempotence) et VD-04 (conflit de révision) ; aucun champ de la spec n'est retiré
> ou renommé. Une formalisation ne peut pas générer un identifiant côté UI puis le
> perdre au rejeu : l'identité est dans la commande.

### 2.1 Types satellites

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point { pub x: f64, pub y: f64 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Size { pub w: f64, pub h: f64 }

pub type PropertyBag = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShapeKind { Rect, Rounded, Port, Sticky, Boundary }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftShape {
    pub id: DraftId,
    pub label: String,
    pub shape: ShapeKind,
    pub x: f64, pub y: f64, pub w: f64, pub h: f64,
    /// Toujours `SKETCH` à la pose : la commande ne formalise pas.
    pub status: DraftStatus,
    #[serde(default)] pub suggested_metaclass: Option<String>,
    #[serde(default)] pub suggested_domain_id: Option<ObjectId>,
    #[serde(default)] pub layer_id: Option<String>,
    #[serde(default)] pub zone_id: Option<ZoneId>,
    /// Port : zone d'accueil et côté de frontière.
    #[serde(default)] pub attachment: Option<Attachment>,
    /// Port rattaché à une interface existante (`DSG-004`).
    #[serde(default)] pub interface_id: Option<ObjectId>,
    #[serde(default)] pub version_id: Option<ObjectId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub zone_id: ZoneId,
    pub side: BoundarySide,
    /// Position relative le long du côté, dans `[0.0, 1.0]`.
    pub offset: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoundarySide { North, South, East, West }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftZone {
    pub id: ZoneId,
    pub kind: ZoneKind,               // DOMAIN | GROUP
    #[serde(default)] pub domain_id: Option<ObjectId>,
    pub label: String,
    pub x: f64, pub y: f64, pub w: f64, pub h: f64,
    #[serde(default)] pub layer_id: Option<String>,
    /// Une zone `DOMAIN` est une frontière par défaut.
    #[serde(default = "yes")] pub boundary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ZoneKind { Domain, Group }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewFraming {
    #[serde(default)] pub zoom: Option<f64>,
    #[serde(default)] pub x: f64,
    #[serde(default)] pub y: f64,
    /// Nœuds mis en focus. Le cadrage ne renomme ni ne déplace rien.
    #[serde(default)] pub focus: Vec<NodeId>,
    #[serde(default)] pub dim_others: bool,
    #[serde(default)] pub caption: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Visibility { Private, Domain, Public }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Target {
    Node { node_id: NodeId },
    Edge { edge_id: EdgeId },
    Zone { zone_id: ZoneId },
    Draft { draft_id: DraftId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EndPoint {
    Node { node_id: NodeId },
    /// Port d'interface : nœud `INTERFACE` ancré sur une zone.
    Port { node_id: NodeId },
}

// Un croquis n'est jamais une extrémité de relation : il n'a pas de classe, donc
// aucune règle de métamodèle ne peut être évaluée. `Connect` depuis ou vers un
// croquis renvoie `DSG-002` avec `detail = "DRAFT_NOT_FORMALIZED"` ; l'UI propose
// alors la formalisation comme geste suivant (voir C-04 §6).
```

`DraftStatus` est défini par C-04. `PropertyBag` réutilise `serde_json::Map` pour rester
compatible avec `ModelElement.semantic_properties` existant.

## 3. Le monde de validation (core reste pur)

`archifier-core` ne dépend ni du dépôt ni du métamodèle. La validation reçoit une
**vue en lecture seule** implémentée par l'adaptateur UI (`design::commands`).

```rust
pub trait ViewWorld {
    fn view_exists(&self, view: ObjectId) -> bool;
    fn view_revision(&self, view: ObjectId) -> Option<Revision>;
    fn object_exists(&self, object: ObjectId) -> bool;
    fn object_is_active(&self, object: ObjectId) -> bool;
    fn object_name(&self, object: ObjectId) -> Option<String>;
    fn object_metaclass(&self, object: ObjectId) -> Option<String>;
    fn object_domain(&self, object: ObjectId) -> Option<ObjectId>;
    fn object_kind(&self, object: ObjectId) -> Option<ObjectKind>;   // ELEMENT | RELATIONSHIP | INTERFACE | DIAGRAM | DOMAIN | STORY | REVIEW
    fn relation_allowed(&self, source_metaclass: &str, relation: &str, target_metaclass: &str) -> bool;
    fn relation_type_exists(&self, notation: Notation, relation: &str) -> bool;
    fn domain_exists(&self, domain: ObjectId) -> bool;
    fn domain_is_active(&self, domain: ObjectId) -> bool;
    fn interface_exists(&self, interface: ObjectId) -> bool;
    fn version_belongs_to(&self, version: ObjectId, interface: ObjectId) -> bool;
    fn required_properties_missing(&self, metaclass: &str, properties: &PropertyBag) -> Vec<String>;
    /// Valide la classe et les propriétés contre le profil chargé. Défaut : aucune
    /// règle supplémentaire (implémentations de test). Ajout additif 1.1.
    fn validate_properties(&self, metaclass: &str, properties: &PropertyBag)
        -> Result<(), (String, String)> { Ok(()) }
}
```

Règles :

- `ViewWorld` ne fait **aucune** I/O ; l'adaptateur UI lit l'index en mémoire.
- Aucune méthode nouvelle n'est ajoutée sans révision de C-01 (voir §8).
- `required_properties_missing` renvoie les clés manquantes, jamais un booléen : le
  message `DSG-001` doit nommer le champ.

## 4. Validation : ordre et codes

L'ordre est **déterministe** et un refus s'arrête au premier code bloquant.

| # | Vérification | Code si refus |
|---|---|---|
| 1 | `view_id` existe et est un diagramme actif | `DGM-000` |
| 2 | `expected_revision` correspond à la révision courante | `REV-409` |
| 3 | Identifiant de cible unique dans la vue (nœud, arête, zone, croquis) | `DGM-002` |
| 4 | Géométrie finie, largeurs/hauteurs > 0 | `DGM-005` |
| 5 | `object_id` d'une forme existant, actif, non archivé | `DGM-001` |
| 6 | Relation autorisée par le métamodèle chargé | **`DSG-002`** |
| 7 | Type de relation déclaré dans la notation | `MM-REL-003` |
| 8 | Formalisation : métamodèle + domaine + propriétés requises | **`DSG-001`** |
| 9 | Classe absente du profil / propriété inconnue | `MM-ELEM-001`, `MM-PROP-001` |
| 10 | Port sans interface référencée ou créée dans le geste | **`DSG-004`** |
| 11 | Publication avec croquis non validés | **`DSG-003`** |
| 12 | Zone `DOMAIN` référençant un domaine inconnu | `DGM-008` |
| 13 | Zone `DOMAIN` référençant un domaine archivé | avertissement `DGM-008-W` |

Les codes et leurs messages sont fixés par C-08. Un refus renvoie :

```rust
pub struct CommandError {
    pub code: String,            // "DSG-002"
    pub path: String,            // "/body/to/node_id" ou "/layout/drafts/d1"
    pub message_key: String,     // clé i18n, jamais une phrase en dur
    pub detail: Option<String>,  // type fautif, champ manquant…
}
```

## 5. Résultat et plan d'effets

```rust
pub struct CommandReceipt {
    pub command_id: CommandId,
    pub view_id: ObjectId,
    /// Layout projeté, prêt à persister. Aucun effet de bord.
    pub layout: DiagramLayout,
    /// Révision de vue après application (révision lue + 1).
    pub revision: Revision,
    /// Effets dépôt, ordonnés, exécutés par l'UI dans cet ordre.
    pub effects: Vec<RepoEffect>,
    pub warnings: Vec<CommandWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepoEffect {
    CreateElement { object_id: ObjectId, domain_id: ObjectId, notation: Notation,
                    metaclass: String, name: String, properties: PropertyBag },
    UpdateElementName { object_id: ObjectId, name: String },
    UpdateElementProperties { object_id: ObjectId, properties: PropertyBag },
    CreateRelationship { object_id: ObjectId, domain_id: ObjectId, source_id: ObjectId,
                         target_id: ObjectId, relation_type: String },
    CreateInterface { object_id: ObjectId, domain_id: ObjectId, name: String,
                      logical_key: String, category: InterfaceCategory },
    CreateInterfaceVersion { object_id: ObjectId, interface_id: ObjectId,
                             semantic_version: String,
                             available_from: Option<OffsetDateTime>,
                             available_until: Option<OffsetDateTime> },
    RenameInterface { object_id: ObjectId, name: String },
    RenameDomain { domain_id: ObjectId, name: String },
    SetPublication { view_id: ObjectId, status: PublicationStatus },
}
```

Ordre d'exécution imposé : `CreateElement` / `CreateInterface` → `CreateInterfaceVersion` →
`CreateRelationship` → renommages → `SetPublication` → `save_layout_expect`. Une erreur
dépôt **annule la persistance du layout** et remonte le code dépôt tel quel ; la vue
locale est restaurée depuis le layout d'avant commande. Aucun effet partiel n'est publié
dans l'historique d'audit sous le nom de la commande.

### 5.1 Projection par commande

| Commande | Effets dépôt | Mutation de layout |
|---|---|---|
| `PlaceDraft` | aucun | ajoute `drafts[id]` |
| `PlaceExisting` | aucun | ajoute `nodes[id]` (`kind` déduit de `object_kind`) + `attachment` |
| `Rename` (draft) | aucun | `drafts[id].label` |
| `Rename` (node élément) | `UpdateElementName` | libellé projeté depuis l'objet |
| `Rename` (node interface) | `RenameInterface` | idem |
| `Rename` (zone DOMAIN) | `RenameDomain` | libellé de **toutes** les zones du domaine |
| `Rename` (zone GROUP) | aucun | `zones[id].label` |
| `Formalize` | `CreateElement` ou `CreateInterface` (+ version) | remplace le croquis par un nœud ; `status = FORMALIZED` ou `VALID` |
| `Connect` | `CreateRelationship` | ajoute `edges[id]` |
| `Reconnect` | aucun | modifie l'extrémité ; refuse `DSG-002` si le nouveau couple est interdit |
| `MoveNode` | aucun | `x += dx`, `y += dy` ; aimantation appliquée au layout, guides non persistés |
| `ResizeNode` | aucun | `w`, `h` |
| `CreateZone` | aucun | ajoute `zones[id]` |
| `SetFraming` | aucun (sauf autosave) | `framing` |
| `SetTemporalCursor` | aucun | `temporal_cursor` |
| `PublishView` | `SetPublication` | `publication_status` |
| `DeleteRepresentation` | aucun | retire nœud, arête, zone ou croquis ; **jamais** l'objet (`DSG-005`) |

### 5.2 Règles de `Formalize`

1. Le croquis doit exister, être de statut `SKETCH`, et ne pas être déjà formalisé.
2. `metaclass` doit appartenir au profil de la notation de la vue (`MM-ELEM-001` sinon).
3. `domain_id` doit exister et être actif (`DSG-001` sinon). Un domaine archivé est
   refusé, pas silencieusement remplacé.
4. Un croquis `PORT` :
   - avec `interface_id` renseigné : rattache l'interface existante, ne crée aucun objet ;
   - sans `interface_id` : exige `new_object_id` + `new_version_id` (`CMD-002` sinon),
     crée l'interface (catégorie et clé logique dérivées du libellé normalisé) et sa
     première version ;
   - dans les deux cas, sans `attachment` valide : `DSG-004`.
5. Un croquis `RECT` / `ROUNDED` : exige `new_object_id` (`CMD-002` sinon), crée un
   `ModelElement`, le nœud reprend exactement la géométrie du croquis (aucun déplacement
   implicite).
6. Le statut résultant est `VALID` si `required_properties_missing` est vide, sinon
   `FORMALIZED`.

## 6. Idempotence, rejeu et concurrence

- Le bus conserve les `command_id` **appliqués** de la vue courante (LRU ≥ 1 000).
- Rejouer un `command_id` connu renvoie `Ignored::Duplicate { command_id }` et le
  `CommandReceipt` **d'origine** (layout et effets vides). Aucun effet n'est rejoué.
- Un `command_id` réutilisé avec un corps différent est refusé : `CMD-001`.
- `expected_revision` différent de la révision courante : `REV-409`. L'UI conserve la
  vue locale et les croquis, puis propose **Recharger / Comparer / Forcer** (VD-04).
  « Forcer » réémet la commande avec `expected_revision = None`, jamais avec une
  révision inventée.
- Le rejeu ne dépend jamais de l'ordre d'arrivée réseau : chaque commande porte son
  identité et ses identités créées. C'est la préparation ADR-014.

## 7. Tests obligatoires (noms figés)

| Test | Prouve |
|---|---|
| `place_draft_creates_named_sketch_without_object` | VD-07, NFR-022 |
| `place_existing_does_not_duplicate_object` | ADR-022 |
| `rename_draft_does_not_write_repository` | C-01 §5.1 |
| `rename_element_writes_object_name` | FR-042 |
| `formalize_requires_metaclass_and_domain` | **DSG-001** |
| `formalize_port_without_interface_is_refused` | **DSG-004** |
| `connect_forbidden_relation_is_rejected_before_creation` | **DSG-002** |
| `connect_allowed_relation_creates_one_relationship` | FR-043 |
| `connect_from_sketch_is_refused_until_formalized` | C-04 §6 |
| `reconnect_forbidden_relation_keeps_edge_intact` | DSG-002 |
| `move_node_does_not_touch_repository` | NFR-025 |
| `delete_representation_keeps_object` | **DSG-005** |
| `publish_view_with_sketch_is_refused` | **DSG-003** |
| `same_command_id_is_applied_once` | VD-03 |
| `same_command_id_with_other_body_is_refused` | CMD-001 |
| `stale_revision_returns_conflict_without_data_loss` | **REV-409**, VD-04 |
| `temporal_cursor_does_not_mutate_geometry` | VD-12 |

## 8. Évolutions autorisées

Une révision de C-01 peut ajouter :

- une variante de `ViewCommandBody` (jamais en retirer ni renommer) ;
- un champ `#[serde(default)]` ;
- une méthode de `ViewWorld` avec implémentation par défaut documentée ;
- un `RepoEffect`.

Elle ne peut pas changer la sémantique d'un code existant (`DSG-*`, `REV-409`).
