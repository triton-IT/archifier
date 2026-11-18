# Spécifications techniques — Archifier v0.1 (MVP1)

| | |
|---|---|
| **Document** | Spécifications techniques d'implémentation |
| **Version** | 1.0 |
| **Date** | 17 novembre 2026 |
| **Entrée** | `functional-specifications.md` (v0.1, 757 lignes) |
| **Statut** | Prêt pour découpage en lots d'implémentation |
| **Portée** | MVP1 / v0.1 uniquement. Aucune fonctionnalité « après MVP1 » n'est spécifiée ici. |

---

## 0. Comment lire ce document

Ce document est **prescriptif**. Il ne discute pas les exigences fonctionnelles : il les traduit en décisions d'implémentation suffisamment précises pour être exécutées sans arbitrage supplémentaire.

Conventions :

- **DOIT / NE DOIT PAS** : contrainte vérifiable par un test ou une revue.
- **DEVRAIT** : choix par défaut ; toute déviation doit être consignée dans un ADR (`docs/adr/`).
- Les identifiants entre backticks (`BR-002`, `MM-REL-001`, `WP-05`) sont des références stables et **doivent rester identiques** dans le code, les tests et les messages d'erreur.
- Les extraits de code sont **normatifs sur la forme**, illustratifs sur les détails sans conséquence.

### 0.1 Décisions ouvertes héritées des spécifications fonctionnelles

La spec fonctionnelle laisse sept arbitrages ouverts (§ « Risques et points à arbitrer »). Ce document les tranche pour le MVP1 :

| Arbitrage ouvert | Décision retenue | Réf. |
|---|---|---|
| Granularité ArchiMate 3.2 dans la palette | Sous-ensemble curé de **42 métaclasses** + 11 types de relations, déclaré dans `profiles/archimate-3.2.toml` | §5 |
| Sémantique des relations transverses | Métamodèle natif `native:1.0`, 7 types, aucune équivalence implicite | §5.4 |
| Versionnage des objets non-interface | `revision` monotone entière + snapshot JSONB immuable à chaque écriture | §6.3 |
| « Publié » vs « visible » pour diagrammes/Stories | Un seul axe : `lifecycle = ACTIVE \| ARCHIVED`. Pas de brouillon de diagramme au MVP1 | §6.2 |
| Conventions de nommage | Unicité insensible à la casse par domaine pour les éléments, globale pour les interfaces (`logical_interface_key`) | §6.5 |
| Profondeur d'analyse d'impact | 3 sauts par défaut, paramétrable de 1 à 6 | §10.3 |
| Format canonique des contrats OpenAPI | JSON canonique (RFC 8785) + conservation du document source brut | §11 |
| Point d’entrée et nature de l’UI | **Atelier de vues** : designer de vues comme surface première, posture conception/lecture sur un même canevas ; l’inventaire est un espace de contrôle secondaire | §15.0, ADR-022 |

---

## 1. Stack technique : front Rust/WASM vs Angular

### 1.1 Décision

**Le front est écrit en Rust et compilé en WebAssembly.** La stack est intégralement Rust :

| Couche | Choix | Version |
|---|---|---|
| Serveur HTTP | `axum` | majeure courante stable |
| Runtime async | `tokio` | majeure courante stable |
| Base de données | PostgreSQL **16+** | — |
| Accès SQL | `sqlx` (queries runtime + `FromRow`, migrations embarquées) | majeure courante stable |
| Front | `leptos` en mode **CSR**, bundlé par `trunk` | ≥ 0.7 |
| Cible WASM | `wasm32-unknown-unknown` | — |
| Temps | `time` (`OffsetDateTime`, feature `serde-well-known`) | majeure courante |
| Identifiants | `ulid` | majeure courante |
| Tests E2E | Playwright (Node 24 déjà présent) | — |

Les versions exactes DOIVENT être figées dans `Cargo.lock` au lot WP-00, et toute déviation d'API d'une dépendance DOIT produire un ADR. Ne jamais coder contre une API supposée : vérifier sur `docs.rs` avant usage.

### 1.2 Pourquoi Rust/WASM est le bon choix **pour ce projet**

Ce n'est pas un pari esthétique. Quatre propriétés du produit poussent dans cette direction :

1. **Le cœur du produit est un moteur de validation déterministe** (`NFR-003`, `FR-025`, `BR-001..012`). En Rust, ce moteur est un crate sans I/O, compilé **à la fois** dans le serveur et dans le bundle WASM. Le front peut donc afficher les erreurs de métamodèle et de temporalité *avant* l'appel réseau, avec **exactement** le même code que celui qui rejettera l'écriture. En Angular il faudrait réimplémenter les règles en TypeScript — duplication et divergence garanties sur 12 règles métier + une matrice de relations.
2. **Pas de dérive de contrat.** Les types d'échange vivent dans `archifier-dto`, partagé serveur/front. Un champ renommé casse la compilation des deux côtés. C'est ce qui rend le travail de plusieurs agents en parallèle soutenable.
3. **`NFR-007`** (500 représentations sans blocage) est plus facile à tenir avec un rendu Rust/WASM qu'avec un framework à réconciliation d'arbre virtuel : le canvas est en SVG piloté par signaux à grain fin, sans re-render global.
4. **Un seul modèle mental, un seul lint, un seul formateur.** Pour une implémentation largement automatisée par agents, l'homogénéité réduit mécaniquement les erreurs d'intégration.

### 1.3 Contreparties assumées et parades

| Risque | Réalité | Parade |
|---|---|---|
| Pas de bibliothèque de diagrammation mûre en Rust/WASM (pas d'équivalent React Flow/GoJS) | Vrai | Le rendu est **écrit sur mesure en SVG** (~600 lignes). C'est de toute façon nécessaire : les besoins de cadrage, d'atténuation, de style ArchiMate et de sémantique ID ne sont couverts par aucune lib générique |
| Poids du bundle (≈ 1,5–2,5 Mo gzip) | Acceptable pour une app interne | `wasm-opt -Oz`, `codegen-units=1`, `lto="fat"`, `strip`, `panic="abort"` ; mesuré et bloquant au-delà de 3,5 Mo gzip |
| Écosystème : markdown, i18n, a11y | Bon pour markdown (`pulldown-cmark`), plus faible pour a11y | Éditeur Markdown maison (textarea + aperçu), `<dialog>` natif, navigation clavier explicite, rôles ARIA posés à la main |
| Compétence de reprise par une équipe JS | Vraie | Frontières nettes : `archifier-ui` ne contient que des composants ; toute logique est dans `archifier-core` testé côté serveur |
| Outillage : `trunk`, cible wasm absente de la machine | Vérifié | WP-00 installe et vérifie la chaîne (§16.1) |

### 1.4 Procédure de repli vers Angular (si la décision est renversée)

Le document reste valable à 90 % : seuls les §15 et §17.4 changent. Le repli est propre parce que `archifier-api` expose un contrat HTTP complet et versionné.

1. `archifier-api` sert `GET /api/openapi.json` (dogfooding, §9.7).
2. Le client Angular est **généré** depuis ce document (`openapi-generator`, `typescript-fetch`).
3. `archifier-core` reste la référence de validation ; le front TypeScript appelle un endpoint `POST /api/validate` (dry-run) au lieu de valider localement. Coût : un aller-retour réseau par frappe, donc *debounce* 250 ms obligatoire.
4. Le canvas Angular serait écrit à l'identique (SVG + signaux/Zone-less), pas avec une lib tierce.

Le repli coûte **+15 à +20 % d'effort** et introduit une validation dupliquée. Il n'est pas recommandé.

---

## 2. Vue d'ensemble de l'architecture

```text
┌───────────────────────────────────────────────────────────────┐
│ archifier-ui  (Leptos CSR → WASM, servi en statique)           │
│  Router │ Atelier de vues │ Bibliothèque │ Contrôle │ Story │ AI │
└───────────────────────────┬───────────────────────────────────┘
                            │ HTTPS + JSON (cookie de session)
┌───────────────────────────▼───────────────────────────────────┐
│ archifier-api (axum)                                          │
│  auth │ domains │ elements │ relationships │ interfaces       │
│  diagrams │ stories │ reviews │ docs │ search │ temporal      │
│  openapi │ export │ ai │ changesets │ mcp │ diagnostics       │
└──────┬──────────────┬──────────────┬──────────────┬───────────┘
       │              │              │              │
┌──────▼─────┐ ┌──────▼──────┐ ┌─────▼──────┐ ┌────▼──────────┐
│archifier-  │ │archifier-   │ │archifier-  │ │archifier-ai   │
│store       │ │search       │ │openapi     │ │ (LLM client,  │
│(sqlx)      │ │(FTS+pgvector│ │(import/    │ │  tools, MCP)  │
│            │ │ optionnel)  │ │ export)    │ │               │
└──────┬─────┘ └──────┬──────┘ └─────┬──────┘ └────┬──────────┘
       │              │              │              │
┌──────▼──────────────▼──────────────▼──────────────▼───────────┐
│ archifier-core (pur, sans I/O, compilé aussi en WASM)         │
│  ids │ objets │ invariants │ validation │ temporalité         │
│  changeset │ arch:// │ diff de contrat │ markdown             │
└───────────────────────────────────────────────────────────────┘
       ▲                                    ▲
       │                            ┌───────┴────────┐
       │                            │archifier-      │
       └────────────────────────────│metamodel       │
                                    │(profils TOML)  │
                                    └────────────────┘
```

### 2.1 Règles de dépendance entre crates (vérifiées en CI)

| Crate | Peut dépendre de | NE DOIT PAS dépendre de |
|---|---|---|
| `archifier-core` | `serde`, `serde_json`, `thiserror`, `time`, `ulid`, `pulldown-cmark` | `sqlx`, `tokio`, `axum`, `reqwest`, `leptos` |
| `archifier-metamodel` | `core`, `toml`, `serde` | `sqlx`, `axum`, `leptos` |
| `archifier-dto` | `core`, `serde` | tout le reste |
| `archifier-store` | `core`, `dto`, `sqlx`, `tokio` | `axum`, `leptos` |
| `archifier-search` | `core`, `store`, `sqlx` | `axum`, `leptos` |
| `archifier-openapi` | `core`, `serde_json`, `serde_yaml` | `axum` |
| `archifier-docgen` | `core`, `dto` | `sqlx`, `axum` |
| `archifier-ai` | `core`, `dto`, `reqwest`, `tokio` | `store` (reçoit un trait de lecture) |
| `archifier-api` | tout sauf `ui` | `leptos` |
| `archifier-server` | `api`, `store` | `leptos` |
| `archifier-ui` | `core`, `dto`, `metamodel`, `leptos` | `sqlx`, `axum`, `reqwest` natif |
| `archifier-testkit` | `core`, `dto` | `leptos` |

Le test `crates/core/tests/architecture.rs` DOIT échouer si `archifier-core` tire une dépendance d'I/O (vérification via `cargo tree -p archifier-core --prefix none`).

---

## 3. Arborescence du dépôt

```text
archifier/
├─ Cargo.toml                      # workspace + profils de build
├─ Cargo.lock                      # committé
├─ rust-toolchain.toml             # channel + targets wasm32-unknown-unknown
├─ .cargo/config.toml              # alias, flags wasm
├─ clippy.toml
├─ rustfmt.toml
├─ archifier.toml                  # configuration par défaut (surchargée par env)
├─ Trunk.toml
├─ index.html                      # hôte Trunk
├─ README.md
├─ functional-specifications.md
├─ technical-specifications.md
├─ crates/
│  ├─ core/            src/{lib,ids,object,validation,temporal,changeset,uri,markdown,diff,error}.rs
│  ├─ metamodel/       src/{lib,profile,index,validate,loader}.rs + profiles/*.toml
│  ├─ dto/             src/{lib,requests,responses,page,problem}.rs
│  ├─ repo/            src/{lib,pool,repos/*,tx,search_index,diagram_usages}.rs
│  ├─ collab/          src/{lib,doc,schema,sync,local_broadcast,presence,merge_check}.rs  (MVP2)
│  ├─ search/          src/{lib,fts,semantic,rank,hybrid}.rs
│  ├─ openapi/         src/{lib,parse,canonical,fingerprint,map_m2r,map_r2m,diff}.rs
│  ├─ docgen/          src/{lib,markdown_export,mermaid,anchors,zip}.rs
│  ├─ ai/              src/{lib,provider,tools,schema,orchestrator,mcp}.rs
│  ├─ api/             src/{lib,router,auth,error,handlers/*,sse,extract}.rs
│  ├─ server/          src/main.rs
│  ├─ ui/              src/{lib,app,routes/*,components/*,canvas/*,api_client,state,i18n}.rs
│  └─ testkit/         src/{lib,builders,fixtures}.rs
├─ migrations/                     # sqlx, numérotées, additives
├─ migrations_optional/            # pgvector (non exécutées par défaut)
├─ specs/
│  ├─ features/*.feature           # Gherkin de la spec fonctionnelle
│  └─ scenarios/*.md               # les 5 chaînes de recette
├─ tests/
│  └─ e2e/                         # Playwright
├─ scripts/                        # PowerShell (dev Windows) + bash (CI)
└─ docs/
   ├─ adr/
   ├─ api-contract.md
   └─ runbook.md
```

---

## 4. Modèle d'identifiants, d'URI et de nommage

### 4.1 Identifiants

- Tout objet du référentiel porte un **ULID** de 26 caractères, en majuscules, stocké en `text`.
- L'ULID est **monotone** (généré avec l'horloge) pour garantir un tri approximativement temporel en base.
- L'identifiant est **opaque** : il n'encode ni le type, ni le domaine, ni le nom (FR-004, NFR-014).
- Aucun ID n'est jamais réattribué, même après archivage.
- Les identifiants « parlants » d'exemple de la spec fonctionnelle (`I-123`, `01ABC`) ne sont **pas** des formats valides en production ; les tests peuvent les utiliser uniquement comme alias d'affichage.

### 4.2 URI `arch://`

Forme canonique (FR-017) :

```text
arch://<kind>/<ulid>
kind ∈ { domain, element, relationship, interface, version, diagram, story, review }
```

- Le parseur `archifier_core::uri::ArchUri` DOIT accepter la casse mixte du `kind`, la rejeter si le kind est inconnu, et refuser tout suffixe.
- L'**affichage** transforme en URL applicative : `/view/<route-segment>/<ulid>`.
- L'**export** transforme en lien relatif (§12.2).
- Une URI dont la cible existe mais est `ARCHIVED` reste **résolvable** ; une URI dont la cible n'existe pas devient `BROKEN` (§6.6).

### 4.3 Nommage

| Objet | Règle |
|---|---|
| Domaine | `name` unique insensible à la casse parmi les domaines `ACTIVE` ; 2–80 caractères |
| Élément / relation / diagramme / story / review | `name` non unique, 0–200 caractères ; unicité *suggérée* par domaine (avertissement, pas d'erreur) |
| Interface | `name` libre ; `logical_interface_key` **globalement unique**, `^[a-z0-9]+(-[a-z0-9]+)*$`, 3–80 caractères, immuable après création |
| Version sémantique | SemVer 2.0.0 strict, sans préfixe `v` |
| Domaine (slug) | Dérivé de `name` par slugification ASCII, unique, utilisé dans les chemins d'export |

---

## 5. Métamodèles : moteur piloté par profils

### 5.1 Principe

Aucune classe, relation ou règle de notation n'est **codée en dur**. Le code manipule des chaînes `metaclass` / `relation_type` validées contre un profil chargé. C'est l'exigence NFR-014 et la préparation ArchiMate 4 (§ « Statut d'ArchiMate 4 »).

Profils livrés au MVP1 :

| Identifiant | Fichier | Notation |
|---|---|---|
| `archimate:3.2` | `profiles/archimate-3.2.toml` | `ARCHIMATE` |
| `bpmn:2.0.2-core` | `profiles/bpmn-2.0.2-core.toml` | `BPMN` |
| `uml:2.5.1-static` | `profiles/uml-2.5.1-static.toml` | `UML` |
| `native:1.0` | `profiles/native-1.0.toml` | `NATIVE` |

Le niveau de conformité est **explicitement un sous-ensemble annoncé**, conformément à la spec fonctionnelle : « tout ce que nous annonçons comme supporté possède un comportement déterministe et testé ». L'UI DOIT afficher le libellé « Cible ArchiMate 4 — règles actuellement basées sur ArchiMate 3.2 ».

Un profil non reconnu ou un profil dont la version ne correspond pas à celle déclarée par un élément DOIT produire l'erreur `MM-PROFILE-001`.

### 5.2 Format de profil (normatif)

```toml
[profile]
id               = "archimate:3.2"
notation         = "ARCHIMATE"
language         = "ArchiMate"
version          = "3.2"
target_family    = "ArchiMate 4"          # informatif, jamais utilisé par la validation
status           = "SUBSET"               # SUBSET | FULL
reference        = "The Open Group, ArchiMate 3.2 Specification"
palette_order    = 10

[[layers]]
id    = "business"
label = "Métier"

[[elements]]
id            = "business-actor"
metaclass     = "BusinessActor"
layer         = "business"
label         = "Acteur métier"
glyph         = "actor"                   # clé de glyphe côté UI
abstract      = false
default_relations_out = ["assignment", "triggering", "serving", "association"]
documentation_hint = "Rôle joué par une personne ou une unité organisationnelle."

[[relations]]
id        = "serving"
metaclass = "Serving"
label     = "Sert"
direction = "DIRECTED"                    # DIRECTED | UNDIRECTED
symmetric = false
transitive = false

# Matrice d'autorisation. Sélecteurs :
#   "layer:<id>"  -> toute classe de la couche
#   "class:<id>"  -> une classe précise
#   "*"           -> n'importe quelle classe du profil
[[relation_rules]]
relation = "serving"
sources  = ["layer:business", "layer:application", "layer:technology"]
targets  = ["layer:business", "layer:application", "layer:technology"]
```

Contraintes vérifiées par `cargo test -p archifier-metamodel` (test `profile_lint`) :

1. Tout `metaclass` est unique dans le profil.
2. Tout `layer` référencé existe.
3. Tout `relation` référencé par une règle existe.
4. Tout sélecteur `layer:` / `class:` résout vers au moins une classe.
5. Aucune règle ne référence une relation `symmetric` avec `direction = "DIRECTED"` incohérent.
6. Le profil est **auto-suffisant** : aucune règle ne cite de classe hors profil.
7. Un rapport de couverture est généré (`docs/metamodel-coverage.md`) : nombre de classes, relations, paires autorisées, paires refusées.

### 5.3 Contenu ArchiMate 3.2 — sous-ensemble MVP1 (42 classes)

| Couche | Classes |
|---|---|
| Business (14) | BusinessActor, BusinessRole, BusinessCollaboration, BusinessInterface, BusinessProcess, BusinessFunction, BusinessService, BusinessEvent, BusinessObject, Contract, Representation, Product, BusinessInteraction, BusinessInternalBehaviorElement *(abstrait, non instanciable)* |
| Application (10) | ApplicationComponent, ApplicationCollaboration, ApplicationInterface, ApplicationProcess, ApplicationFunction, ApplicationInteraction, ApplicationService, ApplicationEvent, ApplicationDataObject, ApplicationInternalBehaviorElement *(abstrait)* |
| Technology (9) | Node, Device, SystemSoftware, TechnologyCollaboration, TechnologyInterface, Path, CommunicationNetwork, TechnologyService, TechnologyEvent |
| Physique (3) | Equipment, Facility, DistributionNetwork |
| Motivation (4) | Stakeholder, Driver, Assessment, Goal |
| Implémentation & Migration (2) | WorkPackage, Deliverable |

Relations supportées (11) : `composition`, `aggregation`, `assignment`, `realization`, `serving`, `access`, `influence`, `triggering`, `flow`, `association`, `specialization`.

Les relations de la couche motivation strictement nécessaires au MVP1 sont couvertes par `influence` et `association` ; `junction`, `grouping`, `location` et `plateau` sont **hors périmètre** et refusés avec `MM-ELEM-002`.

**Note normative :** cette liste est un sous-ensemble délibéré. Toute classe hors liste DOIT être rejetée à la création, avec un message indiquant qu'elle n'est pas couverte par le profil MVP1 — jamais avec un message générique.

### 5.4 Métamodèle natif `native:1.0`

Utilisé **uniquement** pour les relations qui traversent les notations. Aucune équivalence implicite n'est jamais supposée (exigence explicite de la spec fonctionnelle).

| Relation | Sens | Contrainte |
|---|---|---|
| `REFINES` | source affine la cible | pas d'auto-référence, pas de cycle |
| `IMPLEMENTS` | source implémente la cible | pas d'auto-référence |
| `REALIZES` | source réalise la cible | pas d'auto-référence |
| `DESCRIBES` | source documente la cible | cible = tout objet |
| `EXPOSES` | source expose l'interface cible | cible DOIT être une `Interface` |
| `CONSUMES` | source consomme la cible | cible DOIT être une `Interface` ou une `InterfaceVersion` |
| `TRACE_TO` | traçabilité libre | pas d'auto-référence |

Règle de sélection (DOIT être implémentée telle quelle dans `resolve_relation_notation`) :

```text
si source.notation == target.notation
   et relation_type ∈ profil de cette notation
   et (source.metaclass, relation_type, target.metaclass) autorisé
 → notation = cette notation

sinon si relation_type ∈ native:1.0
 → notation = NATIVE

sinon
 → erreur MM-REL-003 « relation non autorisée entre ces métaclasses »
```

### 5.5 Sous-ensemble BPMN 2.0.2 Core

Conforme au tableau de la spec fonctionnelle. `metaclass` BPMN utilisé tel quel : `Process`, `Participant`, `Lane`, `SubProcess`, `CallActivity`, `Task`, `UserTask`, `ServiceTask`, `ManualTask`, `BusinessRuleTask`, `StartEvent`, `EndEvent`, `IntermediateCatchEvent`, `IntermediateThrowEvent`, `ExclusiveGateway`, `ParallelGateway`, `InclusiveGateway`, `SequenceFlow`, `MessageFlow`, `Association`, `DataObject`, `DataStore`, `TextAnnotation`.

Contraintes structurelles validées (`MM-BPMN-*`) :

- `SequenceFlow` : source et cible ∈ {activités, événements, gateways}. Interdit vers un `Participant` ou une `Lane`.
- `MessageFlow` : source et cible ≠ dans le même `Participant` (nécessite la hiérarchie `parent_id`, §6.4).
- Un `StartEvent` n'a aucun flux entrant ; un `EndEvent` aucun sortant.
- `Lane` DOIT avoir un `Participant` ou un `Process` parent.

### 5.6 Sous-ensemble UML 2.5.1 statique

Métaclasses : `Component`, `Interface`, `Port`, `Package`, `Class`, `DataType`, `Enumeration`, `Property`, `Node`, `Device`, `ExecutionEnvironment`, `Artifact`, `Actor`, `UseCase`, `SystemBoundary`.

Relations : `Association`, `Dependency`, `Usage`, `Realization`, `InterfaceRealization`, `Generalization`, `Composition`, `Aggregation`, `Deployment`.

Contraintes validées (`MM-UML-*`) : `Generalization` interdit tout cycle ; `InterfaceRealization` source = `Component`/`Class`, cible = `Interface` ; `Deployment` source = `Artifact`, cible = `Node`/`Device`/`ExecutionEnvironment`.

### 5.7 Chargement des profils et préparation ArchiMate 4

```rust
pub struct MetamodelRegistry {
    profiles: HashMap<ProfileId, Arc<Profile>>,   // ProfileId = "archimate:3.2"
    index:    RelationIndex,                      // (notation, source, rel, target) -> bool
    hashes:   HashMap<ProfileId, ContentHash>,
}

impl MetamodelRegistry {
    pub fn load(embedded: &[&str], overrides: Option<&Path>) -> Result<Self, MetamodelError>;
    pub fn profile(&self, id: &ProfileId) -> Option<&Profile>;
    pub fn is_relation_allowed(&self, n: Notation, s: &ClassId, r: &RelId, t: &ClassId) -> bool;
    pub fn classes_for_palette(&self, n: Notation) -> Vec<PaletteEntry>;
    pub fn default_metamodel_for(&self, n: Notation) -> ProfileId;
}
```

- Les profils sont **embarqués** avec `include_str!` et peuvent être **surchargés** par le répertoire désigné par `ARCHIFIER_PROFILES_DIR` (usage : livrer `archimate:4.x` sans redéploiement de binaire).
- Au démarrage, le hash SHA-256 de chaque profil est enregistré dans `metamodel_versions`. Le hash est exposé dans `GET /api/meta/metamodels`.
- Fichier de migration futur `profiles/mappings/archimate-3.2_to_4.x.toml` avec `[[element_map]] from/to`, `[[relation_map]] from/to`, `[unmapped] policy = "keep" | "block"`. **Aucun mapping n'est livré au MVP1** : le fichier n'existe pas et le chargeur doit refuser un identifiant de mapping inconnu.

> **Amendement MVP2 (ADR-021, 18/11/2026).** Le chemin et le schéma retenus sont
> `crates/metamodel/mappings/archimate-3.2-to-4.x.toml` avec `source_profile`,
> `target_profile`, `status = "unpublished"`, `classes = []`, `relations = []` et
> `default_policy = "preserve"`. La formulation ci-dessus est supersédée.

---

## 6. Modèle de domaine (crate `archifier-core`)

### 6.1 Agrégat `RepositoryObject`

```rust
pub struct RepositoryObject {
    pub id: ObjectId,
    pub object_type: ObjectType,
    pub name: String,
    pub responsible_domain_id: DomainId,
    pub lifecycle: Lifecycle,
    pub revision: Revision,          // entier monotone ≥ 1
    pub created_at: OffsetDateTime,
    pub created_by: UserId,
    pub updated_at: OffsetDateTime,
    pub updated_by: UserId,
}

pub enum ObjectType { Domain, ModelElement, Relationship, Interface, Diagram, Story, Review }
pub enum Lifecycle  { Active, Archived }
```

**Invariants vérifiés par `validate_object` (crate `core`, testés en proptest) :**

| Code | Invariant |
|---|---|
| `OBJ-001` | `revision >= 1` |
| `OBJ-002` | `name` ne contient pas de caractères de contrôle |
| `OBJ-003` | `responsible_domain_id` pointe vers un objet de type `Domain` non supprimé |
| `OBJ-004` | Le type concret (element/relationship/…) est cohérent avec `object_type` |
| `OBJ-005` | Pour une `Interface`, `responsible_domain_id == provider_domain_id` |
| `OBJ-006` | Pour un `Domain`, `responsible_domain_id == self.id` |

### 6.2 Cycle de vie et archivage

- Il n'existe **pas de suppression physique** exposée par l'API. `DELETE /api/<res>/<id>` archive (idempotent).
- Archiver un objet n'archive **jamais** ses voisins ; les références restent résolvables (§4.2, FR-027).
- Un objet archivé est en lecture seule : toute écriture renvoie `409` avec `code = OBJ-007`.
- Un domaine ne peut être archivé que si aucun objet `ACTIVE` ne le référence comme `responsible_domain_id` — sinon `409` `DOM-001` avec la liste des objets bloquants (limite 20).

### 6.3 Versionnage : chemin d'écriture obligatoire

Chaque écriture d'un objet suit **exactement** cette transaction. C'est le cœur de NFR-001/NFR-002 et de FR-026.

```text
BEGIN
 1. SELECT ... FROM objects WHERE id = :id FOR UPDATE           -- verrou de ligne
 2. vérifier If-Match (revision attendue)  -> 409 OBJ-CONFLICT si divergence
 3. valider l'agrégat complet (core::validate_*)
 4. UPDATE <table détail>                                       -- état courant dénormalisé
 5. UPDATE objects SET revision = revision + 1, updated_at, updated_by,
        name, lifecycle
 6. INSERT INTO object_revisions (id, object_id, revision_no = nouvelle revision,
        payload = snapshot JSON complet de l'agrégat, created_by)
 7. UPDATE objects SET current_revision_id = :nouvelle_revision
 8. INSERT INTO audit_events (actor_id, operation, object_id,
        before_revision, after_revision, metadata)
 9. mettre à jour la projection de recherche (search_documents)  -- même transaction
10. recalculer les liens arch:// entrants de cet objet (statut OK/BROKEN/ARCHIVED)
COMMIT
```

- `payload` est un **snapshot complet** et immuable : il permet la comparaison de deux révisions (FR-026) sans reconstruire l'historique depuis les tables détail.
- Aucune écriture hors de cette fonction `store::tx::write_object`. Un test d'architecture vérifie qu'aucun handler n'appelle `sqlx::query` d'écriture directement.
- Le diff de révisions est **structurel** : `core::diff::diff_revision(&payload_a, &payload_b) -> Vec<FieldChange>` avec chemins JSON (`/name`, `/versions/2/contract`).

### 6.4 Hiérarchie et composition

`ModelElement` peut avoir un `parent_id` optionnel (`BPMN Lane → Participant`, `UML Port → Component`, `ArchiMate Grouping` exclu). Contraintes :

- Le parent DOIT appartenir à la même notation et au même `metamodel_version`.
- Pas de cycle (`MM-HIER-001`).
- Profondeur maximale 5 (`MM-HIER-002`).
- Le parent ne peut pas être archivé si des enfants sont actifs (`MM-HIER-003`).

### 6.5 Entités détaillées

```rust
pub struct Domain { pub object: RepositoryObject, pub slug: String, pub description: String }

pub struct ModelElement {
    pub object: RepositoryObject,
    pub notation: Notation,
    pub metamodel_version: ProfileId,
    pub metaclass: ClassId,
    pub parent_id: Option<ObjectId>,
    pub semantic_properties: serde_json::Map<String, Value>,
}

pub struct Relationship {
    pub object: RepositoryObject,
    pub source_id: ObjectId,
    pub target_id: ObjectId,
    pub relation_type: RelId,
    pub notation: Notation,
    pub metamodel_version: ProfileId,
}
```

`semantic_properties` est un sac typé **déclaré par le profil** : chaque `[[elements]]` peut porter `[[elements.properties]] { key, kind, required, values }` avec `kind ∈ { string, text, number, boolean, enum, date }`. Les clés inconnues sont **refusées** (`MM-PROP-001`) : c'est ce qui empêche le référentiel de devenir un fourre-tout.

### 6.6 Liens documentaires (`links`)

- Un lien est créé à chaque `arch://` détecté dans un bloc Markdown ou un `contract_snapshot` documentaire.
- `status` : `OK` (cible `ACTIVE`), `ARCHIVED` (cible archivée, résolvable), `BROKEN` (cible inexistante).
- Le recalcul est fait à l'écriture (étape 10) **et** par une commande `POST /api/diagnostics/links/rebuild` (admin).
- `GET /api/diagnostics/broken-links` renvoie les liens `BROKEN` groupés par propriétaire, avec extrait du Markdown environnant (±80 caractères).

---

## 7. Schéma PostgreSQL

PostgreSQL 16+. Extensions : `pg_trgm` (obligatoire), `unaccent` (obligatoire), `vector` (optionnelle, détectée au démarrage).

### 7.1 Migration `0001_init.sql`

```sql
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;

CREATE TABLE users (
    id            text PRIMARY KEY,
    email         text NOT NULL,
    display_name  text NOT NULL,
    password_hash text NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    disabled_at   timestamptz
);
CREATE UNIQUE INDEX users_email_lower_uq ON users (lower(email));

CREATE TABLE sessions (
    id           text PRIMARY KEY,
    user_id      text NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL
);
CREATE INDEX sessions_user ON sessions (user_id);

CREATE TABLE metamodel_versions (
    id           text PRIMARY KEY,          -- "archimate:3.2"
    notation     text NOT NULL,
    version      text NOT NULL,
    content_hash text NOT NULL,
    loaded_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE objects (
    id                    text PRIMARY KEY,
    object_type           text NOT NULL CHECK (object_type IN
        ('DOMAIN','MODEL_ELEMENT','RELATIONSHIP','INTERFACE','DIAGRAM','STORY','REVIEW')),
    name                  text NOT NULL,
    responsible_domain_id text NOT NULL,
    lifecycle             text NOT NULL DEFAULT 'ACTIVE' CHECK (lifecycle IN ('ACTIVE','ARCHIVED')),
    revision              bigint NOT NULL DEFAULT 1 CHECK (revision >= 1),
    current_revision_id   text,
    created_at            timestamptz NOT NULL DEFAULT now(),
    created_by            text NOT NULL REFERENCES users(id),
    updated_at            timestamptz NOT NULL DEFAULT now(),
    updated_by            text NOT NULL REFERENCES users(id)
);
CREATE INDEX objects_type       ON objects (object_type, lifecycle);
CREATE INDEX objects_domain     ON objects (responsible_domain_id, object_type);
CREATE INDEX objects_name_trgm  ON objects USING gin (name gin_trgm_ops);

CREATE TABLE object_revisions (
    id             text PRIMARY KEY,
    object_id      text NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    revision_no    bigint NOT NULL,
    payload        jsonb NOT NULL,
    change_summary text,
    created_at     timestamptz NOT NULL DEFAULT now(),
    created_by     text NOT NULL REFERENCES users(id),
    UNIQUE (object_id, revision_no)
);
CREATE INDEX object_revisions_obj ON object_revisions (object_id, revision_no DESC);

ALTER TABLE objects
    ADD CONSTRAINT objects_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES object_revisions(id)
    DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE objects
    ADD CONSTRAINT objects_responsible_domain_fk
    FOREIGN KEY (responsible_domain_id) REFERENCES objects(id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE domains (
    object_id   text PRIMARY KEY REFERENCES objects(id) ON DELETE CASCADE,
    slug        text NOT NULL,
    description text NOT NULL DEFAULT '',
    name_lower  text NOT NULL,
    is_active   boolean NOT NULL DEFAULT true
);
CREATE UNIQUE INDEX domains_slug_uq ON domains (slug);
CREATE UNIQUE INDEX domains_name_uq ON domains (name_lower) WHERE is_active;
```

`domains.name_lower` et `domains.is_active` sont des **dénormalisations assumées** de `objects.name` et `objects.lifecycle`, maintenues à l'étape 4 du chemin d'écriture (§6.3), dans la même transaction. Elles existent uniquement pour rendre possible l'index partiel d'unicité, qu'une vue ou une sous-requête ne permet pas. Un test d'intégrité (`domains_denormalization_is_consistent`) vérifie à chaque exécution de la suite qu'aucune ligne ne diverge de son objet.

```sql
CREATE TABLE model_elements (
    object_id           text PRIMARY KEY REFERENCES objects(id) ON DELETE CASCADE,
    notation            text NOT NULL CHECK (notation IN ('ARCHIMATE','BPMN','UML','NATIVE')),
    metamodel_version   text NOT NULL REFERENCES metamodel_versions(id),
    metaclass           text NOT NULL,
    parent_id           text REFERENCES objects(id),
    semantic_properties jsonb NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX model_elements_class ON model_elements (notation, metamodel_version, metaclass);
CREATE INDEX model_elements_parent ON model_elements (parent_id) WHERE parent_id IS NOT NULL;

CREATE TABLE relationships (
    object_id         text PRIMARY KEY REFERENCES objects(id) ON DELETE CASCADE,
    source_id         text NOT NULL REFERENCES objects(id),
    target_id         text NOT NULL REFERENCES objects(id),
    relation_type     text NOT NULL,
    notation          text NOT NULL CHECK (notation IN ('ARCHIMATE','BPMN','UML','NATIVE')),
    metamodel_version text NOT NULL REFERENCES metamodel_versions(id),
    CHECK (source_id <> target_id)
);
CREATE INDEX relationships_source ON relationships (source_id, notation, relation_type);
CREATE INDEX relationships_target ON relationships (target_id, notation, relation_type);

CREATE TABLE interfaces (
    object_id             text PRIMARY KEY REFERENCES objects(id) ON DELETE CASCADE,
    logical_interface_key text NOT NULL,
    category              text NOT NULL CHECK (category IN ('BUSINESS','APPLICATION','TECHNICAL')),
    provider_domain_id    text NOT NULL,
    contract_type         text NOT NULL DEFAULT 'NONE'
        CHECK (contract_type IN ('NONE','OPENAPI','EVENT','OTHER')),
    notation_bindings     jsonb NOT NULL DEFAULT '[]'::jsonb
);
CREATE UNIQUE INDEX interfaces_key_uq ON interfaces (logical_interface_key);
CREATE INDEX interfaces_provider ON interfaces (provider_domain_id, category);

CREATE TABLE interface_versions (
    id                  text PRIMARY KEY,
    interface_id        text NOT NULL REFERENCES interfaces(object_id) ON DELETE CASCADE,
    semantic_version    text NOT NULL,
    revision            bigint NOT NULL DEFAULT 1,
    publication_status  text NOT NULL DEFAULT 'DRAFT'
        CHECK (publication_status IN ('DRAFT','PUBLISHED','WITHDRAWN')),
    created_at          timestamptz NOT NULL DEFAULT now(),
    created_by          text NOT NULL REFERENCES users(id),
    published_at        timestamptz,
    available_from      timestamptz,
    available_until     timestamptz,
    breaking_change     boolean NOT NULL DEFAULT false,
    supersedes_version_id text REFERENCES interface_versions(id),
    contract_snapshot   jsonb NOT NULL DEFAULT '{}'::jsonb,
    contract_fingerprint text,
    CHECK (available_from IS NULL OR available_until IS NULL
           OR available_until > available_from),
    CHECK ((available_from IS NULL) = (available_until IS NULL)),
    CHECK (publication_status <> 'PUBLISHED'
           OR (available_from IS NOT NULL AND available_until IS NOT NULL
               AND published_at IS NOT NULL))
);
CREATE UNIQUE INDEX interface_versions_semver_uq
    ON interface_versions (interface_id, semantic_version)
    WHERE publication_status <> 'WITHDRAWN';
CREATE INDEX interface_versions_window
    ON interface_versions USING gist (tstzrange(available_from, available_until, '[)'))
    WHERE available_from IS NOT NULL;

CREATE TABLE interface_consumers (
    id                    text PRIMARY KEY,
    interface_version_id  text NOT NULL REFERENCES interface_versions(id) ON DELETE CASCADE,
    consumer_object_id    text NOT NULL REFERENCES objects(id),
    consumer_domain_id    text NOT NULL,
    declared_at           timestamptz NOT NULL DEFAULT now(),
    declared_by           text NOT NULL REFERENCES users(id),
    UNIQUE (interface_version_id, consumer_object_id)
);
CREATE INDEX interface_consumers_version ON interface_consumers (interface_version_id);
CREATE INDEX interface_consumers_domain  ON interface_consumers (consumer_domain_id);

CREATE TABLE diagrams (
    object_id   text PRIMARY KEY REFERENCES objects(id) ON DELETE CASCADE,
    notation    text NOT NULL CHECK (notation IN ('ARCHIMATE','BPMN','UML','FREE')),
    layout      jsonb NOT NULL DEFAULT '{"viewport":{"x":0,"y":0,"zoom":1},"nodes":[],"edges":[]}'::jsonb,
    description text NOT NULL DEFAULT ''
);

CREATE TABLE diagram_usages (
    diagram_id text NOT NULL REFERENCES diagrams(object_id) ON DELETE CASCADE,
    object_id  text NOT NULL REFERENCES objects(id),
    usage_kind text NOT NULL CHECK (usage_kind IN ('NODE','EDGE')),
    PRIMARY KEY (diagram_id, object_id, usage_kind)
);
CREATE INDEX diagram_usages_object ON diagram_usages (object_id);

CREATE TABLE stories (
    object_id text PRIMARY KEY REFERENCES objects(id) ON DELETE CASCADE,
    structure jsonb NOT NULL DEFAULT '{"chapters":[]}'::jsonb
);

CREATE TABLE reviews (
    object_id             text PRIMARY KEY REFERENCES objects(id) ON DELETE CASCADE,
    domain_id             text NOT NULL,
    filter_snapshot       jsonb NOT NULL DEFAULT '{}'::jsonb,
    temporal_window_start timestamptz,
    temporal_window_end   timestamptz,
    status                text NOT NULL DEFAULT 'OPEN' CHECK (status IN ('OPEN','CLOSED')),
    markdown_summary      text NOT NULL DEFAULT '',
    CHECK (temporal_window_start IS NULL OR temporal_window_end IS NULL
           OR temporal_window_end >= temporal_window_start)
);
CREATE INDEX reviews_domain ON reviews (domain_id, status);

CREATE TABLE documentation_blocks (
    id              text PRIMARY KEY,
    owner_object_id text NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    markdown        text NOT NULL,
    revision        bigint NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    created_by      text NOT NULL REFERENCES users(id),
    UNIQUE (owner_object_id, revision)
);

CREATE TABLE links (
    id               text PRIMARY KEY,
    source_object_id text NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    target_uri       text NOT NULL,
    target_object_id text REFERENCES objects(id),
    link_type        text NOT NULL DEFAULT 'REFERENCE'
        CHECK (link_type IN ('REFERENCE','DESCRIBES','REFINES','IMPLEMENTS',
                             'REALIZES','EXPOSES','CONSUMES','TRACE_TO')),
    status           text NOT NULL DEFAULT 'OK' CHECK (status IN ('OK','ARCHIVED','BROKEN')),
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX links_target ON links (target_object_id);
CREATE INDEX links_broken ON links (source_object_id) WHERE status <> 'OK';

CREATE TABLE comments (
    id               text PRIMARY KEY,
    target_object_id text NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    author_id        text NOT NULL REFERENCES users(id),
    markdown         text NOT NULL,
    created_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX comments_target ON comments (target_object_id, created_at DESC);

CREATE TABLE search_documents (
    object_id   text PRIMARY KEY REFERENCES objects(id) ON DELETE CASCADE,
    object_type text NOT NULL,
    domain_id   text,
    title       text NOT NULL,
    body        text NOT NULL DEFAULT '',
    tsv         tsvector GENERATED ALWAYS AS (
                    setweight(to_tsvector('french', coalesce(title,'')), 'A') ||
                    setweight(to_tsvector('french', coalesce(body,'')),  'B')
                ) STORED,
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX search_documents_tsv ON search_documents USING gin (tsv);
CREATE INDEX search_documents_type ON search_documents (object_type);

CREATE TABLE audit_events (
    id              text PRIMARY KEY,
    actor_id        text NOT NULL REFERENCES users(id),
    operation       text NOT NULL,
    object_id       text REFERENCES objects(id),
    before_revision bigint,
    after_revision  bigint,
    metadata        jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX audit_object ON audit_events (object_id, created_at DESC);
CREATE INDEX audit_actor  ON audit_events (actor_id, created_at DESC);
```

### 7.2 Migration `0002_changesets.sql`

```sql
CREATE TABLE changesets (
    id                text PRIMARY KEY,
    origin            text NOT NULL CHECK (origin IN ('AI','USER','IMPORT')),
    requested_by      text NOT NULL REFERENCES users(id),
    reason            text NOT NULL,
    operations        jsonb NOT NULL,
    base_revisions    jsonb NOT NULL DEFAULT '{}'::jsonb,
    preview           jsonb,
    status            text NOT NULL DEFAULT 'DRAFT'
        CHECK (status IN ('DRAFT','VALIDATED','APPLIED','REJECTED','STALE')),
    validation_report jsonb,
    created_at        timestamptz NOT NULL DEFAULT now(),
    validated_at      timestamptz,
    applied_at        timestamptz
);
CREATE INDEX changesets_status ON changesets (status, created_at DESC);
```

### 7.3 Migration `0003_openapi.sql`

```sql
CREATE TABLE openapi_documents (
    id                 text PRIMARY KEY,
    source_fingerprint text NOT NULL,
    spec_family        text NOT NULL CHECK (spec_family IN ('3.0','3.1','3.2')),
    raw                jsonb NOT NULL,
    original_bytes     bytea NOT NULL,
    canonical          jsonb NOT NULL,
    interface_id       text REFERENCES interfaces(object_id),
    imported_at        timestamptz NOT NULL DEFAULT now(),
    imported_by        text NOT NULL REFERENCES users(id)
);
CREATE UNIQUE INDEX openapi_documents_fingerprint_uq ON openapi_documents (source_fingerprint);
CREATE INDEX openapi_documents_interface ON openapi_documents (interface_id);
```

### 7.4 Migration optionnelle `migrations_optional/0001_pgvector.sql`

```sql
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE embeddings (
    id         text PRIMARY KEY,
    object_id  text NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    revision_id text REFERENCES object_revisions(id) ON DELETE CASCADE,
    chunk_id   integer NOT NULL,
    model      text NOT NULL,
    vector     vector(1536) NOT NULL,
    text_hash  text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (object_id, revision_id, chunk_id, model)
);
CREATE INDEX embeddings_hnsw ON embeddings USING hnsw (vector vector_cosine_ops);
```

Cette migration n'est appliquée que si `ARCHIFIER_EMBEDDINGS=enabled` **et** si l'extension `vector` est disponible. Sinon le système démarre en mode `lexical` et `GET /api/meta/features` renvoie `{"semantic_search": false}`.

### 7.5 Devise de stockage des dates

- Toutes les dates sont `timestamptz`, stockées en UTC (NFR-008).
- En Rust : `time::OffsetDateTime`, toujours construites avec `OffsetDateTime::now_utc()`.
- Le serveur renvoie du RFC 3339 avec suffixe `Z`. Le front convertit au fuseau du navigateur **à l'affichage uniquement**, jamais avant un envoi.
- Une date sans fuseau en entrée est **refusée** (`400`, `code = TMP-004`).

---

## 8. Règles métier : implémentation et codes d'erreur

`archifier-core::validation` expose une fonction pure par agrégat, retournant `Result<(), Vec<ValidationIssue>>`. `ValidationIssue` porte `code`, `severity` (`ERROR`/`WARNING`), `path`, `message`, `rule_ref`.

```rust
pub struct ValidationIssue {
    pub code: &'static str,
    pub severity: Severity,
    pub path: String,          // chemin JSON de l'élément fautif
    pub message: String,       // français, affichable tel quel
    pub rule_ref: Option<&'static str>,
}
```

### 8.1 Règles temporelles (BR-001 → BR-012)

```rust
pub fn temporal_status(v: &InterfaceVersion, now: OffsetDateTime) -> Option<TemporalStatus> {
    let from = v.available_from?;
    let until = v.available_until?;
    Some(if now < from {
        TemporalStatus::Scheduled
    } else if now < until {
        TemporalStatus::Active
    } else {
        TemporalStatus::Expired
    })
}
```

| Code | Règle | Où | Test |
|---|---|---|---|
| `BR-001` | Version `PUBLISHED` ⇒ `available_from` et `available_until` non nuls | publication | `br_001` |
| `BR-002` | `available_until > available_from` | toute écriture | `br_002` |
| `BR-003` | `available_from > now` autorisé ⇒ `SCHEDULED` | calcul | `br_003` |
| `BR-004` | `now < available_from` ⇒ `SCHEDULED` | calcul | `br_004` |
| `BR-005` | `available_from ≤ now < available_until` ⇒ `ACTIVE` | calcul | `br_005` |
| `BR-006` | `now ≥ available_until` ⇒ `EXPIRED` | calcul | `br_006` |
| `BR-007` | Statut éditorial orthogonal au statut temporel | sérialisation | `br_007` |
| `BR-008` | Changement de contrat publié ⇒ nouvelle version, jamais d'écrasement | store | `br_008` |
| `BR-009` | Chevauchement de versions autorisé | store | `br_009` |
| `BR-010` | `breaking_change` posé manuellement ou par diff, **confirmé** par l'utilisateur | API | `br_010` |
| `BR-011` | Expiration ne supprime pas les consommations ; marquées `DEPENDENCY_ON_EXPIRED_INTERFACE` | review | `br_011` |
| `BR-012` | Une référence pointe l'ID canonique, jamais une copie | validation d'écriture | `br_012` |

Le message de `BR-002` DOIT contenir littéralement `BR-002` (exigence du scénario Gherkin correspondant).

### 8.2 Règles de métamodèle

| Code | Situation |
|---|---|
| `MM-PROFILE-001` | Profil inconnu ou version incohérente avec l'élément |
| `MM-ELEM-001` | `metaclass` absente du profil |
| `MM-ELEM-002` | `metaclass` abstraite instanciée |
| `MM-PROP-001` | Propriété sémantique non déclarée par le profil |
| `MM-PROP-002` | Propriété requise manquante ou type invalide |
| `MM-REL-001` | `relation_type` absente du profil |
| `MM-REL-002` | Relation réflexive interdite (`source_id == target_id`) |
| `MM-REL-003` | Couple (source, relation, target) non autorisé **et** pas de repli natif |
| `MM-REL-004` | Relation native `EXPOSES`/`CONSUMES` avec cible du mauvais type |
| `MM-REL-005` | Cycle interdit (`REFINES`, `Generalization`) |
| `MM-HIER-001` | Cycle de composition |
| `MM-HIER-002` | Profondeur > 5 |
| `MM-HIER-003` | Parent archivé avec enfants actifs |
| `MM-BPMN-001..004` | Contraintes BPMN (§5.5) |
| `MM-UML-001..003` | Contraintes UML (§5.6) |

### 8.3 Règles documentaires et transverses

| Code | Situation |
|---|---|
| `DOC-LINK-001` | Lien `arch://` non parsable |
| `DOC-LINK-002` | Lien vers un objet inexistant (`BROKEN`) |
| `DOM-001` | Archivage de domaine avec objets actifs |
| `OBJ-007` | Écriture sur objet archivé |
| `OBJ-CONFLICT` | `If-Match` divergent (HTTP 409) |
| `TMP-004` | Date sans fuseau horaire |
| `TMP-005` | Fenêtre de revue incohérente |

---

## 9. API HTTP

### 9.1 Conventions

- Préfixe `/api`. JSON uniquement (`Content-Type: application/json`), UTF-8.
- Toute ressource versionnée porte `revision` et renvoie `ETag: "rev-<n>"`.
- `PUT` / `PATCH` / `DELETE` sur un objet versionné **exigent** `If-Match`. Absence ⇒ `428 Precondition Required`.
- Listes paginées par curseur : `?limit=` (défaut 50, max 200) et `?cursor=` (opaque, base64url du dernier tri). Réponse : `{ "items": [...], "next_cursor": "..." | null, "total_estimate": n }`.
- Erreurs au format RFC 9457 `application/problem+json` :

```json
{
  "type": "https://archifier.local/problems/business-rule",
  "title": "Règle métier violée",
  "status": 422,
  "code": "BR-002",
  "detail": "BR-002 : availableUntil doit être postérieur à availableFrom.",
  "instance": "/api/interfaces/01J.../versions/01K...",
  "issues": [
    { "path": "/availableUntil", "code": "BR-002", "severity": "ERROR" }
  ]
}
```

- Codes HTTP utilisés : `200`, `201`, `204`, `400` (malformé), `401`, `403` (réservé), `404`, `409` (conflit), `412`, `422` (règle métier), `428`, `500`, `503` (LLM indisponible).

### 9.2 Authentification

MVP1 : comptes locaux, capacités identiques pour tous, pas de RBAC/ABAC (hors périmètre explicite).

| Méthode | Route | Rôle |
|---|---|---|
| `POST` | `/api/auth/login` | `{email, password}` → `Set-Cookie: archifier_session=...; HttpOnly; SameSite=Lax; Secure` |
| `POST` | `/api/auth/logout` | invalide la session |
| `GET` | `/api/auth/me` | utilisateur courant |

- Mot de passe : Argon2id (`argon2` crate, paramètres par défaut OWASP).
- Session serveur (table `sessions`), TTL glissant 12 h, absolu 7 jours.
- Toute requête non `GET`/`HEAD` exige un en-tête `X-CSRF-Token` égal au cookie `archifier_csrf` (double-submit).
- Bootstrap : `archifier-server admin create-user --email ... --name ...` en CLI.

### 9.3 Domaines

```text
GET    /api/domains                          ?q=&lifecycle=&limit=&cursor=
POST   /api/domains
GET    /api/domains/{id}
PUT    /api/domains/{id}                     If-Match
DELETE /api/domains/{id}                     If-Match  (archive)
GET    /api/domains/{id}/overview            agrégat de la page d'accueil VISUALISER
GET    /api/domains/{id}/interfaces          ?category=&status=&at=
GET    /api/domains/{id}/dependencies        ?direction=in|out|both&depth=1..6
GET    /api/domains/{id}/diagrams
GET    /api/domains/{id}/stories
GET    /api/domains/{id}/reviews
GET    /api/domains/{id}/elements            ?notation=&metaclass=&limit=&cursor=
```

`GET /api/domains/{id}/overview` renvoie exactement ce dont la page d'accueil VISUALISER a besoin, en un appel (NFR-005) :

```json
{
  "domain": { "id": "...", "name": "Paiements", "revision": 7 },
  "published_diagrams": [{ "id": "...", "name": "...", "notation": "ARCHIMATE" }],
  "exposed_interfaces": [{ "id": "...", "name": "...", "category": "APPLICATION",
                           "active_versions": 2, "status": "ACTIVE" }],
  "consumed_interfaces": [{ "id": "...", "name": "...", "provider_domain": {...},
                            "status": "ACTIVE", "anomalies": [] }],
  "top_dependencies": [{ "direction": "out", "target_domain": {...}, "weight": 12 }],
  "stories": [{ "id": "...", "title": "..." }],
  "counters": { "elements": 84, "interfaces": 12, "diagrams": 9, "open_reviews": 1 }
}
```

### 9.4 Éléments et relations

```text
GET    /api/elements                          ?notation=&metaclass=&domain=&q=&lifecycle=
POST   /api/elements
GET    /api/elements/{id}
PUT    /api/elements/{id}                     If-Match
DELETE /api/elements/{id}                     If-Match
GET    /api/elements/{id}/dependencies        ?direction=in|out|both&depth=1..6&notation=
GET    /api/elements/{id}/diagrams
GET    /api/elements/{id}/stories
GET    /api/elements/{id}/history
GET    /api/elements/{id}/revisions/{n}
GET    /api/elements/{id}/revisions/{a}/diff/{b}
GET    /api/elements/{id}/usages              inverse : diagrammes, stories, interfaces, revues

GET    /api/relationships                     ?source=&target=&type=&notation=&domain=
POST   /api/relationships
GET    /api/relationships/{id}
PUT    /api/relationships/{id}                If-Match
DELETE /api/relationships/{id}                If-Match
```

`POST /api/elements` accepte un `metaclass` + `notation` et refuse tout ce qui n'est pas dans le profil avec `422` + `MM-ELEM-001`. Le champ `metamodel_version` est optionnel : s'il est absent, le registre applique `default_metamodel_for(notation)`.

Recherche avant création (exigence MODÉLISER : « la création d'une occurrence passe par une recherche du référentiel ») : `GET /api/elements?q=<nom>&limit=5` est appelé par l'UI **avant** d'ouvrir le formulaire, et le formulaire affiche les résultats comme suggestion. Ce n'est jamais bloquant.

### 9.5 Interfaces, versions et temporalité

```text
GET    /api/interfaces                       ?domain=&category=&state=&consumer=&version=&from=&until=&q=
POST   /api/interfaces
GET    /api/interfaces/{id}
PUT    /api/interfaces/{id}                  If-Match  (métadonnées seules)
DELETE /api/interfaces/{id}                  If-Match

GET    /api/interfaces/{id}/versions
POST   /api/interfaces/{id}/versions         corps: version complète
GET    /api/interfaces/{id}/versions/{vid}
PUT    /api/interfaces/{id}/versions/{vid}   If-Match  (uniquement DRAFT)
POST   /api/interfaces/{id}/versions/{vid}/publish
POST   /api/interfaces/{id}/versions/{vid}/withdraw
POST   /api/interfaces/{id}/versions/{vid}/breaking   { breaking: true, reason }
GET    /api/interfaces/{id}/versions/{a}/diff/{b}

GET    /api/interfaces/{id}/consumers        ?at=
POST   /api/interfaces/{id}/consumers        { consumer_object_id, interface_version_id }
DELETE /api/interfaces/{id}/consumers/{cid}
GET    /api/interfaces/{id}/impact           ?at=&depth=1..6
GET    /api/interfaces/{id}/timeline         ?from=&until=
```

`publish` est l'unique transition `DRAFT → PUBLISHED`. Elle :

1. exige `available_from`, `available_until`, `published_at = now()` ;
2. valide `BR-001`, `BR-002` ;
3. si l'interface a déjà une version publiée, impose soit `supersedes_version_id`, soit une justification explicite (`allow_parallel_versions: true`) — c'est ce qui rend visible la période de migration `BR-009` ;
4. interdit la modification ultérieure du `contract_snapshot` (toute évolution ⇒ nouvelle version, `BR-008`) ;
5. renvoie `201` avec la version et son statut temporel calculé.

`GET /api/interfaces/{id}/timeline` renvoie un calendrier calculé :

```json
{
  "interface_id": "01J...",
  "now": "2026-11-17T10:00:00Z",
  "segments": [
    { "version_id": "01K1", "semver": "1.0.0", "status": "ACTIVE",
      "available_from": "2026-10-01T00:00:00Z", "available_until": "2027-06-01T00:00:00Z",
      "consumers": 4, "breaking_change": false },
    { "version_id": "01K2", "semver": "2.0.0", "status": "SCHEDULED",
      "available_from": "2027-03-01T00:00:00Z", "available_until": "2028-03-01T00:00:00Z",
      "consumers": 0, "breaking_change": true }
  ],
  "overlaps": [{ "from": "2027-03-01T00:00:00Z", "until": "2027-06-01T00:00:00Z",
                 "versions": ["01K1", "01K2"] }]
}
```

Le champ `overlaps` est ce qui alimente la « période de transition immédiatement visible » exigée par la spec.

### 9.6 Diagrammes, Stories, Revues, Documentation

```text
GET    /api/diagrams                          ?domain=&notation=
POST   /api/diagrams
GET    /api/diagrams/{id}
PUT    /api/diagrams/{id}                     If-Match  (layout complet)
PATCH  /api/diagrams/{id}/layout              If-Match  (delta: upsert_nodes, remove_node_ids, upsert_edges, remove_edge_ids, viewport)
DELETE /api/diagrams/{id}                     If-Match

GET    /api/stories
POST   /api/stories
GET    /api/stories/{id}
PUT    /api/stories/{id}                      If-Match  (structure complète)
PATCH  /api/stories/{id}/structure            If-Match  (opérations ciblées)
DELETE /api/stories/{id}                      If-Match
GET    /api/stories/{id}/play                 résolution de tous les blocs en un appel

GET    /api/reviews                           ?domain=&status=
POST   /api/reviews
GET    /api/reviews/{id}
PUT    /api/reviews/{id}                      If-Match
POST   /api/reviews/{id}/close
POST   /api/reviews/{id}/reopen
GET    /api/reviews/{id}/views/{view}         view ∈ summary|exposed|consumed|incoming|outgoing|calendar|contracts|consumers|detail
GET    /api/reviews/{id}/anomalies
GET    /api/reviews/{id}/comments
POST   /api/reviews/{id}/comments

GET    /api/objects/{id}/documentation
PUT    /api/objects/{id}/documentation        If-Match  (révision des blocs gérée côté serveur)
GET    /api/objects/{id}/links
GET    /api/objects/{id}/history
GET    /api/diagnostics/broken-links
```

`GET /api/reviews/{id}/views/{view}` ne calcule que la vue demandée : la vue de synthèse est celle qui agrège, les autres sont des listes filtrables. Toutes les vues partagent le `filter_snapshot` figé de la revue, mais acceptent `?at=` pour recalculer le statut temporel à une date donnée sans altérer la revue.

`GET /api/reviews/{id}/anomalies` renvoie au minimum :

```json
[{ "code": "DEPENDENCY_ON_EXPIRED_INTERFACE",
   "severity": "WARNING",
   "message": "Customer API v1 est expirée depuis le 2027-06-01T00:00:00Z.",
   "consumer_object_id": "01M...",
   "interface_version_id": "01K1",
   "link": "/view/interfaces/01JDEF?version=01K1" }]
```

### 9.7 Recherche, métadonnées, santé

```text
GET    /api/search?q=&types=&domain=&notation=&category=&limit=&cursor=
GET    /api/meta/metamodels                  profils chargés, hash, compteurs
GET    /api/meta/features                    { semantic_search, ai_enabled, mcp_enabled }
GET    /api/openapi.json                     contrat de cette API (dogfooding)
GET    /healthz                              liveness
GET    /readyz                               readiness (DB + profils)
```

`GET /api/search` renvoie des résultats **typés** avec un score et un extrait :

```json
{ "items": [ { "object_id": "01J...", "object_type": "INTERFACE",
               "title": "Customer API", "excerpt": "...contrôle de <em>solvabilité</em>...",
               "score": 0.87, "domain": { "id": "...", "name": "Paiements" },
               "href": "/view/interfaces/01J..." } ],
  "mode": "lexical", "next_cursor": null }
```

`mode ∈ { lexical, hybrid }` est renvoyé explicitement (NFR honnêteté : ne jamais prétendre à du sémantique quand il est désactivé).

### 9.8 Import/export OpenAPI et Markdown

```text
POST   /api/import/openapi                   multipart ou corps JSON/YAML brut
       ?mode=preview|apply                   preview par défaut
GET    /api/import/openapi/{job_id}          résultat du preview (diff, décisions)
POST   /api/import/openapi/{job_id}/apply    applique le ChangeSet issu de l'import
GET    /api/export/openapi/{interface_version_id}    → application/yaml
GET    /api/export/markdown/{object_id}              → application/zip
GET    /api/export/markdown/domain/{domain_id}       → application/zip
```

### 9.9 IA, ChangeSets et MCP

```text
POST   /api/ai/chat                          SSE: token | tool_call | sources | changeset | error | done
POST   /api/ai/query                         requête structurée non conversationnelle
GET    /api/ai/proposals                     ?status=
POST   /api/ai/proposals                     demande de proposition (modèle, relation, diagramme, doc, story)

POST   /api/changesets/{id}/validate
POST   /api/changesets/{id}/apply            If-Match implicite via base_revisions
POST   /api/changesets/{id}/reject

POST   /mcp                                  JSON-RPC 2.0 (transport streamable HTTP)
```

`POST /api/changesets/{id}/apply` est **le seul** point d'écriture issu de l'IA. Il exige que le ChangeSet soit `VALIDATED` **et** que l'utilisateur confirme explicitement (le client envoie `{"confirmed": true}` ; c'est redondant avec le `VALIDATED` et c'est voulu — FR-025 est une exigence de sûreté, pas d'ergonomie).

---

## 10. Temporalité, graphe et recherche

### 10.1 Vue « architecture à la date T » (FR-029)

Aucune table dédiée : c'est une requête paramétrée par `$at`.

```sql
-- versions d'interface effectives à $at
SELECT v.*, iv.object_id AS interface_id
FROM interface_versions v
JOIN interfaces iv ON iv.object_id = v.interface_id
WHERE v.publication_status IN ('PUBLISHED','WITHDRAWN')
  AND v.available_from <= $1
  AND v.available_until >  $1
```

```text
GET /api/architecture/at?at=2027-04-15T00:00:00Z&domain=&include=
```

Réponse : snapshot lecture seule contenant domaines, éléments actifs, relations, interfaces effectives, consommateurs et anomalies temporelles. Utilisé par la page `VISUALISER → à la date T` et par les revues.

### 10.2 Graphe de dépendances

Pas de base graphe (décision héritée de la spec). Parcours par CTE récursif borné :

```sql
WITH RECURSIVE walk(object_id, depth, path) AS (
    SELECT $1::text, 0, ARRAY[$1::text]
  UNION ALL
    SELECT e.next_id, w.depth + 1, w.path || e.next_id
    FROM walk w
    JOIN (
        SELECT source_id AS from_id, target_id AS next_id
          FROM relationships WHERE relation_type IN ('SERVING','FLOW','TRIGGERING','IMPLEMENTS','REALIZES','REFINES')
        UNION
        SELECT rc.consumer_object_id, iv.interface_id
          FROM interface_consumers rc
          JOIN interface_versions iv ON iv.id = rc.interface_version_id
    ) e ON e.from_id = w.object_id
    WHERE w.depth < $2 AND NOT e.next_id = ANY(w.path)
)
SELECT * FROM walk WHERE depth > 0;
```

- `depth` par défaut **3**, borné à **6** (§0.1). Valeur hors bornes ⇒ `400` `QUERY-DEPTH-001`.
- Le paramètre `direction` inverse la jointure (sources ↔ cibles).
- Le parcours est **toujours** filtré par date quand `at` est fourni : une arête dont l'interface est expirée à `at` est exclue du parcours et signalée séparément.

### 10.3 Analyse d'impact IA (FR-023)

`GET /api/interfaces/{id}/impact?at=&depth=` produit un objet **déterministe** (aucun LLM). Le LLM ne fait que le commenter. Structure :

```json
{
  "interface": { "id": "...", "name": "Payment API" },
  "at": "2027-07-01T00:00:00Z",
  "removed_versions": [{ "id": "01K1", "semver": "1.0.0", "expired_at": "..." }],
  "direct_consumers": [{ "object_id": "...", "name": "Order Service", "domain": {...} }],
  "transitive_consumers": [{ "object_id": "...", "path": ["01M", "01N"], "depth": 2 }],
  "affected_diagrams": [{ "id": "...", "name": "..." }],
  "affected_stories": [{ "id": "...", "title": "..." }],
  "risk_notes": ["2 consommateurs utilisent encore v1 après sa fin de disponibilité."]
}
```

`risk_notes` est calculé par des règles, pas par le LLM.

### 10.4 Recherche

Deux modes, jamais confondus.

**Lexical (toujours actif).** `websearch_to_tsquery('french', $q)` sur `search_documents.tsv`, complété par une similarité trigramme sur `objects.name` pour les fautes de frappe :

```sql
SELECT d.*, ts_rank(d.tsv, q) AS lex,
       similarity(o.name, $1) AS tri
FROM search_documents d
JOIN objects o ON o.id = d.object_id
CROSS JOIN websearch_to_tsquery('french', $1) q
WHERE d.tsv @@ q OR o.name % $1
ORDER BY (ts_rank(d.tsv, q) * 1.0 + similarity(o.name, $1) * 0.4) DESC
```

**Hybride (si pgvector actif).** `0.65 * normalise(lexical) + 0.35 * (1 - cosine_distance)`, avec `k = 60` de fusion RRF si les deux listes sont disjointes. Résultat toujours **résolu vers des IDs canoniques** avant d'être rendu ou envoyé au LLM (exigence explicite : la similarité ne remplace jamais une requête déterministe).

**Projection `search_documents`.** `title` = nom de l'objet ; `body` = concaténation de : documentation courante (Markdown dégraissé), noms des voisins de premier niveau, noms + semver des versions d'interface, et pour un diagramme, les noms des éléments représentés. Recalculée à chaque écriture (même transaction).

---

## 11. OpenAPI : import et export

> **Amendement W1a (18/11/2026).** Pour le MVP2, `docs/w1a-openapi-contract.md` fait foi.
> La preview est **purement en mémoire** (pas de table `openapi_documents`, pas de
> `job_id`, pas de ChangeSet) ; les codes sont `OAS-001`/`OAS-002` (et non
> `OAS-PARSE-001`), `OAS-060` (export non applicable), `OAS-061` (version sans contrat) ;
> l'export est un **JSON UTF-8 canonique** (et non YAML). Le pipeline ci-dessous décrit
> l'intention serveur du lot 3, pas le contrat W1a.

### 11.1 Import

Pipeline `archifier-openapi::import` :

```text
1. bytes → parse (serde_yaml, puis serde_json)         erreur: OAS-PARSE-001
2. détection de famille: 3.0.x / 3.1.x / 3.2.x         erreur: OAS-VERSION-001
3. canonicalisation RFC 8785 (clés triées, nombres normalisés)
4. fingerprint = sha256(canonical)                     stocké dans openapi_documents
5. résolution d'identité, dans cet ordre:
     a. x-architecture-version-id présent et connu  → mise à jour de version
     b. x-architecture-interface-id présent et connu → nouvelle version
     c. fingerprint déjà connu                       → "source inchangée", aucune écriture
     d. logical_interface_key dérivé du titre présent → rapprochement proposé
     e. sinon                                        → nouvelle interface proposée
6. mapping vers le modèle normalisé (§11.2)
7. production d'un ChangeSet de type IMPORT, statut DRAFT
8. preview renvoyée à l'utilisateur (diff objet par objet)
```

- `mode=preview` (défaut) n'écrit **rien** d'autre que la ligne `openapi_documents` provisoire et le ChangeSet `DRAFT`.
- Le document source brut est **toujours** conservé (`original_bytes` + `raw`), exigence explicite : les constructions non comprises ne sont pas perdues.
- Réimporter le même document ⇒ `200` avec `{ "status": "UNCHANGED", "interface_id": "..." }`, aucune seconde interface (scénario Gherkin).
- Un changement d'identité détecté ⇒ `status: "UPDATE_PROPOSED"` avec diff. Jamais d'écriture silencieuse.

### 11.2 Mapping OpenAPI → référentiel

| Source | Cible |
|---|---|
| `info.title` | `Interface.name` |
| `info.description` | documentation Markdown de l'interface |
| `info.version` | `InterfaceVersion.semantic_version` |
| `servers[].url` | `contract_snapshot.servers` |
| `paths` | `contract_snapshot.operations[]` (`path`, `method`) |
| `operationId` | `operations[].operation_id` (obligatoire ; généré `method_path` sinon, avec avertissement `OAS-WARN-001`) |
| `parameters` | `operations[].parameters[]` |
| `requestBody` | `operations[].request_body` |
| `responses` | `operations[].responses[]` |
| `components.schemas` | `contract_snapshot.schemas` + éléments `UML Class`/`ArchiMate ApplicationDataObject` **proposés**, jamais créés d'office |
| `security` | `contract_snapshot.security` (métadonnées seulement) |

`contract_snapshot` normalisé :

```json
{
  "openapi_version": "3.1.0",
  "servers": ["https://api.example.com/v1"],
  "security_schemes": ["oauth2"],
  "operations": [
    { "operation_id": "getCustomer",
      "method": "GET", "path": "/customers/{id}",
      "parameters": [{ "name": "id", "in": "path", "required": true, "schema": {"type": "string"} }],
      "request_body": null,
      "responses": [{ "status": "200", "schema_ref": "#/components/schemas/Customer" }] }
  ],
  "schemas": { "Customer": { "...": "..." } }
}
```

### 11.3 Diff de contrat et détection de rupture (FR-012, BR-010)

`archifier-core::diff::contract_diff(old, new) -> ContractDiff` produit des changements classés :

| Changement | Classification |
|---|---|
| Suppression d'opération, de paramètre ou de réponse | `BREAKING` |
| Changement de type de schéma, ajout de paramètre `required: true` | `BREAKING` |
| Renommage d'`operationId` | `BREAKING` |
| Suppression de champ optionnel, ajout d'opération | `COMPATIBLE` |
| Ajout de champ optionnel, changement de description | `COMPATIBLE` |
| Changement de serveur, de sécurité | `UNKNOWN` (à confirmer) |

`BREAKING` ⇒ proposition `breaking_change = true`. **L'utilisateur confirme toujours** (BR-010) : le diff est présenté, la case est pré-cochée, l'écriture n'est faite qu'après `POST .../breaking` ou `publish` avec `confirm_breaking: true`.

### 11.4 Export

- Uniquement pour `category = APPLICATION` **et** `contract_type = OPENAPI`. Toute autre combinaison ⇒ `409` `OAS-EXPORT-001` : « aucun faux document OpenAPI n'est fabriqué ».
- Génération d'un document **3.1.0** depuis `contract_snapshot`, avec injection systématique de :

```yaml
x-architecture-interface-id: 01JDEF...
x-architecture-version-id: 01JXYZ...
x-architecture-snapshot-at: 2026-11-17T10:00:00Z
```

- L'export est **déterministe** : mêmes entrées ⇒ mêmes octets (tri stable des clés, pas d'horodatage de génération hors `x-architecture-snapshot-at` figé à la seconde de la requête — ce champ est exclu du test de reproductibilité).

---

## 12. Documentation Markdown, liens et export

### 12.1 Blocs documentaires

- Un objet possède **un** `documentation_block` courant, versionné (`revision` incrémentale), plus l'historique.
- Markdown supporté : CommonMark + tableaux + listes de tâches + `arch://` + blocs de code clôturés. HTML brut **filtré** (liste blanche : `b i em strong a code pre table thead tbody tr td th ul ol li p br hr h1..h6 blockquote`). Toute balise hors liste est échappée, jamais exécutée.
- Rendu côté front : `pulldown-cmark` compilé en WASM (même crate que le serveur), avec un `Event` handler qui réécrit `arch://` en liens applicatifs. Le HTML produit côté serveur pour l'export utilise le même renderer ⇒ rendu identique (NFR-009).

### 12.2 Export Markdown d'un domaine (FR-019)

> **Amendement W1b (18/11/2026).** Pour le MVP2, `docs/w1b-contract.md` §1.2 et §5.2
> prévalent sur l'arborescence ci-dessous : `<root>/index.md`, `<id>.md` sans slug,
> `relationships/`, SVG/Mermaid à côté des pages diagramme, pas de ZIP (téléchargement
> fichier par fichier). Un `arch://` résoluble devient relatif ; un lien non résoluble
> devient texte + note, jamais supprimé silencieusement.

```text
<domain-slug>/
  domain.md
  elements/<ulid>-<slug>.md
  interfaces/<ulid>-<slug>.md
  diagrams/<ulid>-<slug>.md
  reviews/<ulid>-<slug>.md
  stories/<ulid>-<slug>.md
  assets/            (SVG de diagrammes générés)
  index.md           (table des matières)
```

Règles :

- Chaque `arch://` est converti en lien relatif calculé depuis la position du fichier courant ; un lien cassé est rendu en texte + note `> ⚠ lien cassé vers arch://...`.
- Les diagrammes sont exportés en SVG statique (même layout, même style) **et** en bloc Mermaid (`graph LR`/`flowchart`) en complément. Mermaid n'est jamais la source.
- L'export est reproductible à révision identique (NFR-009) : pas d'horodatage de génération, ordre stable, newline `\n`, ZIP avec dates d'entrée figées.
- L'export d'**un objet** produit un bundle minimal avec ses voisins directs, pour que tous les liens relatifs du document soient valides.

### 12.3 Story : modèle et export

```rust
pub struct Story {
    pub object: RepositoryObject,
    pub structure: StoryStructure,
}

pub struct StoryStructure {
    pub chapters: Vec<Chapter>,
    pub pinned_revisions: BTreeMap<ObjectId, Revision>,  // récit historique optionnel
}

pub struct Chapter { pub id: Ulid, pub title: String, pub blocks: Vec<StoryBlock> }

pub enum StoryBlock {
    Markdown  { id: Ulid, markdown: String },
    Diagram   { id: Ulid, diagram_id: ObjectId, framing: DiagramFraming },
    View      { id: Ulid, view: SavedViewRef },
    Element   { id: Ulid, element_id: ObjectId, highlight: bool },
    Interface { id: Ulid, interface_id: ObjectId, version_id: Option<ObjectId> },
    Transition{ id: Ulid, label: String, target_block_id: Ulid },
}

pub struct DiagramFraming {
    pub zoom: Option<f32>,
    pub focus_element_ids: Vec<ObjectId>,
    pub dimmed_element_ids: Vec<ObjectId>,
    pub caption: Option<String>,
}
```

Invariants (`STORY-*`) : `target_block_id` DOIT exister dans la même Story ; une Story archivée dont un bloc référence un objet archivé reste lisible et affiche un bandeau « objet archivé, dernière révision » ; `pinned_revisions` DOIT référencer des révisions existantes.

`GET /api/stories/{id}/play` résout tous les blocs (objets, diagrammes, versions, statuts temporels) en un seul appel : le lecteur ne subit pas N allers-retours (NFR-005).

---

## 13. Intelligence artificielle

### 13.1 Frontière nette

Le LLM est **lecteur** et **générateur de propositions**. Il n'écrit jamais.

```text
Interface CLI/API          archifier-ai                    archifier-store
─────────────────          ───────────────                 ───────────────
POST /api/ai/chat  ──►  orchestrator
                          ├─ tools (lecture)  ──►  RepositoryReader (trait)
                          ├─ LLM provider
                          ├─ changeset builder
                          └─ SSE stream
                                                    ApplicationService ─► store::tx::apply_changeset
                                                        (seul écrivain, transactionnel)
```

`archifier-ai` ne dépend **pas** de `archifier-store` : il reçoit un `Arc<dyn RepositoryReader>`. Cela rend l'IA testable sans base et interdit structurellement l'écriture directe.

### 13.2 Outils exposés (schéma JSON, source unique de vérité)

| Outil | Entrée | Sortie |
|---|---|---|
| `search_repository` | `{ query, types?, domain? , limit? }` | objets avec IDs, titres, extraits |
| `get_object` | `{ object_id }` | agrégat complet + révision |
| `list_interfaces_at` | `{ domain?, category?, at }` | versions effectives + statut temporel |
| `get_interface_timeline` | `{ interface_id, from?, until? }` | segments + chevauchements |
| `traverse_graph` | `{ object_id, direction, relation_types?, depth ≤ 6 }` | nœuds + chemins |
| `impact_analysis` | `{ interface_id, at, depth? }` | résultat déterministe de §10.3 |
| `search_text` | `{ query, types? }` | résultats lexicaux (+ sémantiques si actif) |
| `propose_changes` | `{ operations[], reason }` | ChangeSet `DRAFT` + rapport de validation |

Chaque outil est déclaré **une seule fois** dans `archifier-ai::schema` et cette déclaration est utilisée à la fois pour le LLM et pour l'exposition MCP. Un test vérifie que tout outil exposé en MCP figure dans le schéma et inversement.

**Règle absolue :** tout identifiant renvoyé au LLM provient d'un outil. Aucun ID n'est inventé ni complété par le modèle. Le validateur rejette un ChangeSet citant un ID inconnu (`CS-001`).

### 13.3 ChangeSet

```rust
pub struct ChangeSet {
    pub id: Ulid,
    pub origin: ChangeOrigin,           // AI | USER | IMPORT
    pub reason: String,
    pub operations: Vec<ChangeOperation>,
    pub base_revisions: BTreeMap<ObjectId, Revision>,
    pub status: ChangeSetStatus,        // Draft | Validated | Applied | Rejected | Stale
}

pub enum ChangeOperation {
    CreateElement      { temp_id: String, payload: NewModelElement },
    CreateRelationship { temp_id: String, payload: NewRelationship },
    CreateInterface    { temp_id: String, payload: NewInterface },
    CreateInterfaceVersion { temp_id: String, interface_id: ObjectId, payload: NewInterfaceVersion },
    UpdateObject       { object_id: ObjectId, patch: JsonPatch },
    CreateDiagram      { temp_id: String, payload: NewDiagram },
    UpdateDiagramLayout{ diagram_id: ObjectId, patch: LayoutPatch },
    CreateStory        { temp_id: String, payload: NewStory },
    UpdateDocumentation{ object_id: ObjectId, markdown: String },
    ArchiveObject      { object_id: ObjectId },
}
```

- `temp_id` est un identifiant local au ChangeSet (`tmp-1`), résolu en ULID réel à l'application. Une opération peut référencer un `temp_id` créé plus haut dans le **même** ChangeSet. Les références avant création ⇒ `CS-002`.
- **Toute** opération passe par les validations de `core` — le ChangeSet ne contourne aucune règle (NFR-004).
- Validation en deux temps :
  - `validate` : pure, sans écriture, retourne `{ valid, issues[], preview }` ;
  - `apply` : transaction unique, `SELECT ... FOR UPDATE` sur chaque objet de `base_revisions`, `409` + `CS-STALE` si la concurrence a bougé, sinon application atomique + `audit_events` avec `metadata.changeset_id`.

### 13.4 Fournisseur LLM

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, req: ChatRequest) -> Result<ChatResponse, LlmError>;
    async fn stream(&self, req: ChatRequest) -> Result<ChatStream, LlmError>;
}
```

- Implémentations : `OpenAiCompatibleProvider` (base URL configurable ⇒ compatible vLLM, Ollama, Azure, Mistral) et `MockProvider` (déterministe, scripté, utilisé en CI et en démo hors ligne).
- Configuration : `ARCHIFIER_AI_BASE_URL`, `ARCHIFIER_AI_API_KEY`, `ARCHIFIER_AI_MODEL`. Absence de clé ⇒ `GET /api/meta/features` renvoie `ai_enabled: false` et l'UI affiche « IA non configurée » — jamais une erreur 500 au démarrage.
- Aucune donnée sensible dans les logs : les prompts et contrats complets sont tronqués à 0 par défaut (NFR-013), loggés uniquement si `ARCHIFIER_AI_LOG_PROMPTS=true`.
- Le contexte envoyé est **minimal** : l'orchestrateur ne transmet que les objets effectivement retournés par les outils (NFR-011).

### 13.5 SSE et vérifiabilité des réponses (FR-022)

Événements émis dans l'ordre :

```text
event: step       data: {"stage":"retrieval"}
event: tool_call  data: {"tool":"list_interfaces_at","args":{...}}
event: tool_result data: {"tool":"list_interfaces_at","count":2}
event: source     data: {"kind":"interface","id":"01J...","label":"Payment API v1"}
event: token      data: {"text":"Les systèmes"}
event: changeset  data: {"id":"01K...","status":"DRAFT"}
event: error      data: {"code":"AI-TOOL-001","message":"..."}
event: done       data: {"sources":3,"elapsed_ms":1840}
```

Le front affiche la liste des `source` sous la réponse, sous forme de pastilles cliquables (exigence : « chaque réponse IA doit afficher ses objets sources »). Si aucune source n'est résolue, un bandeau explicite « Réponse non sourcée » est affiché (exigence : « l'absence de source résolue doit être explicite »).

### 13.6 MCP

- `POST /mcp` implémente JSON-RPC 2.0 en transport *streamable HTTP*, méthodes `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`.
- Chaque outil de §13.2 est ré-exposé ; les ressources `arch://<kind>/<ulid>` sont exposées en `resources/read` renvoyant le Markdown de l'objet.
- MCP Apps, AG-UI et A2UI sont **hors périmètre** (décision de la spec fonctionnelle). Aucune dépendance à ces protocoles n'est introduite.

---

## 14. Diagrammes : modèle graphique et rendu

### 14.1 Structure de layout (JSONB `diagrams.layout`)

```json
{
  "schema": 1,
  "viewport": { "x": 0, "y": 0, "zoom": 1.0 },
  "grid": { "size": 8, "snap": true },
  "nodes": [
    { "id": "n1", "object_id": "01JABC...", "kind": "ELEMENT",
      "x": 120, "y": 80, "w": 160, "h": 72,
      "style_override": { "fill": "#dbeafe", "stroke": null },
      "collapsed": false }
  ],
  "edges": [
    { "id": "e1", "relationship_id": "01JDEF...", "kind": "RELATIONSHIP",
      "waypoints": [{ "x": 280, "y": 116 }],
      "label_visible": true, "routing": "ORTHOGONAL" }
  ],
  "annotations": [
    { "id": "a1", "text": "Zone de migration v1→v2", "x": 40, "y": 20,
      "w": 320, "h": 180, "style": "CALLOUT" }
  ]
}
```

- `kind` d'un nœud ∈ `ELEMENT | INTERFACE | DIAGRAM_REF | GROUP`. Un nœud `INTERFACE` référence une interface **ou** une version précise (`version_id` optionnel) — c'est ainsi que le scénario « réutiliser une interface d'un autre domaine » reste sans duplication (nœud → `interface.object_id`, jamais de nouvelle Interface).
- Un nœud ne peut référencer qu'un objet existant : validation `DGM-001` sinon, et l'écriture est refusée.
- `diagram_usages` est recalculée à chaque écriture de layout (diff des ensembles), dans la même transaction.
- `style_override` est limité à 4 clés (`fill`, `stroke`, `text_color`, `icon`) — pas de CSS libre, pour garantir la cohérence visuelle et éviter l'injection.

### 14.2 Contraintes de validation

| Code | Règle |
|---|---|
| `DGM-001` | `object_id` d'un nœud inexistant |
| `DGM-002` | Deux nœuds avec le même `id` dans un diagramme |
| `DGM-003` | Une arête référence une relation dont la source ou la cible n'est **pas** représentée dans le diagramme (avertissement, non bloquant : une arête peut être « sortante ») |
| `DGM-004` | `relationship_id` référençant un objet qui n'est pas une `Relationship` |
| `DGM-005` | Géométrie non finie ou dimensions ≤ 0 |
| `DGM-006` | Plus de **1 000** nœuds par diagramme (limite dure, protège NFR-007) |

### 14.3 Rendu côté front

- **SVG**, pas de `<canvas>` : hit-testing natif, accessibilité, inspection aisée, impression.
- `transform="translate(x y) scale(z)"` sur un unique `<g>` : le pan/zoom ne déclenche **aucun** re-render des enfants.
- Chaque nœud est un composant Leptos mémoïsé dont les props sont `(x, y, w, h, label, glyph, style_hash)` ; un déplacement local met à jour un signal `drag_state` et déplace l'élément par `transform`, puis **commite** le layout au `drop` (une seule écriture `PATCH .../layout`).
- Culling : au-delà de 200 nœuds, seuls les nœuds intersectant la vue ± 200 px sont rendus. Mesure instrumentée au WP-22.
- Arêtes : `path` orthogonal calculé à partir de `waypoints`, sinon un simple « manhattan » à un coude. Pas de routage automatique complexe au MVP1.

### 14.4 Modèle de l’Atelier : vue, intentions, croquis et commandes

L’Atelier est un **éditeur de vue** adossé au référentiel. Techniquement, il ne crée jamais un objet par le rendu : il émet des **commandes** validées par `archifier-core`, puis projette le résultat dans le `layout`. Cette séparation est ce qui rend l’annulation, l’enregistrement automatique et la collaboration possibles sans dupliquer les règles.

#### 14.4.1 Métadonnées de vue

Le layout existant est étendu de façon **additive** (schéma 2, migration sans perte) :

```json
{
  "schema": 2,
  "intent": "APPLICATION_LANDSCAPE",
  "scope": { "kind": "DOMAIN", "domain_id": "01J..." },
  "notation": "ARCHIMATE",
  "temporal_cursor": "2027-08-01T00:00:00Z",
  "layers": [
    { "id": "business", "label": "Métier", "y": 0, "h": 240, "collapsed": false },
    { "id": "application", "label": "Applicatif", "y": 240, "h": 280, "collapsed": false }
  ],
  "zones": [
    { "id": "z1", "kind": "DOMAIN", "domain_id": "01J...", "x": 40, "y": 260,
      "w": 420, "h": 240, "label": "Paiements" }
  ],
  "framing": { "zoom": 1.0, "focus": ["n1", "n4"], "dim_others": true },
  "nodes": [ { "id": "n1", "object_id": "01J...", "kind": "ELEMENT", "x": 80, "y": 300 } ],
  "edges": [ { "id": "e1", "relationship_id": "01J...", "kind": "RELATIONSHIP" } ],
  "drafts": [
    { "id": "d1", "label": "Service de paiement", "shape": "RECT",
      "x": 200, "y": 340, "w": 160, "h": 72,
      "suggested_metaclass": "ApplicationService",
      "suggested_domain_id": "01J...", "status": "SKETCH" }
  ],
  "annotations": []
}
```

| Champ | Règle |
|---|---|
| `intent` | Valeur d’un catalogue fermé (`CONTEXT`, `APPLICATION_LANDSCAPE`, `PROCESS`, `DEPLOYMENT`, `CAPABILITY`, `MIGRATION`, `INTERFACE_REVIEW`). Pilote la palette, les bandes et les invitations, **jamais** une validation de métamodèle. |
| `temporal_cursor` | Optionnel. S’il est présent, le rendu applique le statut temporel à cette date ; il ne modifie aucune donnée. |
| `layers` | Bandes visuelles nommées ; une couche est un repère de lecture, pas un objet du référentiel. |
| `zones` | `DOMAIN` référence un domaine existant ; `GROUP` est un regroupement visuel. Une zone `DOMAIN` ne crée jamais de domaine implicite. |
| `framing` | Cadrage réutilisable par une Story ou un lien partagé (FR-051). |
| `drafts` | Croquis **locaux** : jamais exportés vers le référentiel, jamais publiables (`DSG-003`), visibles avec un marquage explicite. |

#### 14.4.2 Commandes d’édition

Toute mutation passe par un `ViewCommand` sérialisable. L’interface n’écrit jamais directement le layout.

```rust
pub enum ViewCommand {
    PlaceDraft     { draft: DraftShape },
    PlaceExisting  { object_id: ObjectId, at: Point, size: Size },
    Rename         { target: Target, label: String },
    Formalize      { draft_id: DraftId, metaclass: String, domain_id: ObjectId, properties: PropertyBag },
    Connect        { from: EndPoint, to: EndPoint, relation_type: String },
    Reconnect      { edge_id: EdgeId, to: EndPoint },
    MoveNode       { node_id: NodeId, delta: Point },
    ResizeNode     { node_id: NodeId, size: Size },
    CreateZone     { zone: DraftZone },
    SetFraming     { framing: Framing },
    SetTemporalCursor { at: Option<OffsetDateTime> },
    PublishView    { visibility: Visibility },
    DeleteRepresentation { target: Target },
}
```

Règles :

1. **Commande = intention**, pas un diff de pixels. Le layout est recalculé après validation.
2. `Connect` interroge `metamodel.is_relation_allowed` **avant** de créer la relation ; un refus renvoie `DSG-002` et laisse la vue intacte.
3. `Formalize` est la seule commande qui crée un `ModelElement` depuis un croquis ; elle exige `metaclass` et `domain_id`.
4. `DeleteRepresentation` ne supprime que la représentation ; la suppression d’objet est une commande distincte et confirmée.
5. Toute commande est **idempotente** par `command_id`, ce qui prépare la collaboration (ADR-014) sans l’activer.

#### 14.4.3 Pile d’annulation et enregistrement

- Une pile locale de commandes (au moins 100 entrées) fournit `undo`/`redo`. Elle est **par vue** et **par utilisateur** ; elle n’est jamais partagée ni persistée.
- L’enregistrement est **automatique** : après 800 ms d’inactivité, la vue est committée en une écriture unique (`revision` optimiste). Le bouton « Enregistrer » disparaît de l’interface principale ; un indicateur discret affiche `Enregistré · révision N`.
- Un `409` conserve la vue locale, ouvre le dialogue recharger / comparer / forcer, et **ne perd aucun croquis**.
- Un croquis non formalisé est persisté dans le layout, donc restauré à la réouverture ; il n’apparaît jamais dans la recherche, l’export référentiel ou l’analyse d’impact.

#### 14.4.4 Sélection, outils et parité clavier

| Outil | Geste | Équivalent clavier |
|---|---|---|
| Sélection | clic / rectangle de sélection | `V`, `Maj`+flèches |
| Élément | glisser depuis la palette | `E` puis flèches, `Entrée` |
| Relation | tirer d’un port à un autre | `R`, sélection source/cible, `Entrée` |
| Zone | tracer un rectangle | `Z` |
| Annotation | double-clic sur le fond | `N` |
| Cadrage | `Maj`+clic sur zone, puis « Cadrer » | `F` |
| Palette de commandes | barre globale | `Ctrl/Cmd+K` |
| Renommer sur place | double-clic sur le libellé | `F2` |
| Inspecteur | panneau contextuel non bloquant | `Tab` / `Échap` |

L’outil actif est visible, le curseur change, et `Échap` revient toujours à la sélection. Aucun geste de conception n’est exclusivement souris (NFR-026).

#### 14.4.5 Rendu et performance de l’Atelier

- Le rendu reste **SVG** ; les bandes, zones et arêtes sont dans des groupes distincts pour éviter de repeindre la vue entière au déplacement.
- Les couches et zones sont rendues avant les nœuds ; les nœuds sont virtualisés par viewport au-delà de 200 formes (ADR-020).
- Le curseur temporel ne recalcule que les styles d’état, jamais la géométrie.
- Le cadrage applique `transform` sur le groupe racine ; le déplacement d’un nœud ne déclenche que la mise à jour de son `transform` local.

---

## 15. Frontend Leptos

### 15.0 Principe directeur (MVP3) : l’Atelier est le point d’entrée

L’application s’ouvre sur l’**Atelier de vues**, pas sur un inventaire, un tableau de bord de domaines ou une liste d’objets. Trois espaces structurent la coque :

| Espace | Rôle | Route d’entrée |
|---|---|---|
| **Atelier** | Concevoir, lire et partager des vues ; créer et éditer tous les objets depuis le canevas | `/` (dernière vue ou nouvelle vue) |
| **Bibliothèque** | Retrouver une vue, un domaine, un objet, une interface, une Story, une Revue | `/library` |
| **Contrôle** | Inventaires, anomalies, couverture des métamodèles, exports, réglages | `/control` |

Un objet reste consultable et éditable dans une **fiche** (`/view/elements/:id`, …), mais cette fiche est atteinte depuis une vue, la Bibliothèque, la recherche ou un lien `arch://`. Elle n’est jamais le point de départ imposé.

**Règle d’implémentation bloquante.** Aucun composant de création ne DOIT exiger une navigation hors de `/` ou de `/view/diagrams/:id` pour produire un objet. Les formulaires de fiche sont des vues secondaires du même contrat de commande.

> **Avenant ADR-023 (18/11/2026) — la carte est le point d’entrée.** Le §15.0 ci-dessus décrit la coque du MVP3. La cible v0.4 ajoute un espace **Carte** qui devient la route `/` : projection générée du référentiel, cinq niveaux (entreprise, portefeuille, produit, application, infrastructure), lentilles et liens profonds. L’Atelier reste la surface d’édition des vues authored (`/design/:id`, `/view/diagrams/:id`). Les routes, objets, cardinalités, règles d’agrégation et paramètres de lien sont fixés par `docs/contracts/C-10-produit-carte-navigation.md`. Le moteur de projection fera l’objet d’un lot technique dédié ; il ne se confond pas avec le layout d’un `DiagramLayout` (C-02), qui reste la géométrie d’une vue authored. Aucun champ de C-01..C-09 n’est retiré.

### 15.1 Mode de rendu

**CSR pur**, bundlé par `trunk`. Aucun SSR, aucune hydratation : cela supprime une classe entière de bugs (diff mismatch), simplifie le déploiement (fichiers statiques servis par `axum` via `tower-http::ServeDir`) et suffit largement au profil d'usage. `index.html` contient un écran de chargement remplacé au montage.

Décision de routage : `leptos_router` en mode `history`, avec repli SPA côté serveur (`ServeDir` + `fallback` vers `index.html` pour toute route non `/api`).

### 15.2 Plan de routes

```text
/login

/                                   Atelier : dernière vue ouverte, sinon écran « Nouvelle vue »
/design/new?intent=...              création d'une vue depuis une intention
/design/:id                         Atelier, posture conception
/view/diagrams/:id                  Atelier, posture lecture (lien partagé, cadrage restauré)

/library                            Bibliothèque : vues, domaines, objets, interfaces, Stories, Revues
/library/domains/:id                synthèse d'un domaine (exposition, consommation, dépendances)
/library/views?owner=&intent=       liste filtrée des vues
/search?q=

/control                            Contrôle : anomalies, échéances, liens cassés
/control/inventory                  inventaire assumé, secondaire
/control/coverage                   couverture des métamodèles
/control/exports                    exports et imports

/view/elements/:id                  fiche objet (conséquence d'un clic, pas une entrée)
/view/interfaces/:id                versions, frise, consommateurs, impact
/view/reviews/:id
/view/stories/:id
/view/at/:timestamp                 lecture à la date T
/settings                           profil, fuseau, à propos (cible ArchiMate 4)
```

Compatibilité : `/model/diagrams/:id` reste un **alias** de `/design/:id`, et `/view/domains/:id` un alias de `/library/domains/:id`, pendant au moins une version. Aucun lien `arch://` existant ne doit casser.

> **Avenant ADR-023.** Le plan de routes ci-dessus reste valide pour l’Atelier et les espaces secondaires. La cible v0.4 ajoute `/map`, `/products/:id`, `/teams/:id`, `/value-streams/:id`, `/capabilities/:id`, `/applications/:id`, `/infrastructure/:id`, `/business-domains/:id`, `/portfolios/:id`, et fait de `/` la carte de l’entreprise. Les paramètres `scope`, `level`, `lens`, `date`, `focus`, `filter`, `mode` sont spécifiés dans C-10 §9. Les alias de domaine restent résolvables. L’Atelier n’est plus l’accueil, mais conserve `/design/:id` et `/view/diagrams/:id`.

Le passage conception ↔ lecture est un **geste de la barre d’outils**, pas un changement d’application : il conserve la vue, le cadrage, le curseur temporel et la sélection (FR-040, FR-048).

### 15.3 Organisation du crate `archifier-ui`

```text
src/
├─ lib.rs                 mount point
├─ app.rs                 <App/>, coque, barre d’outils, bascule conception/lecture
├─ api_client/            un module par ressource, typé par archifier-dto
│   ├─ mod.rs             ApiClient { base, csrf, fetch } + gestion 409/428
│   └─ ...
├─ state/                 signaux globaux : session, métamodèles, vue courante, toasts
├─ design/                éditeur de vue côté client
│   ├─ commands.rs        ViewCommand, application, idempotence
│   ├─ history.rs         undo/redo, autosave, indicateur de révision
│   ├─ tools.rs           outil actif, gestes, raccourcis
│   ├─ selection.rs       sélection simple/multiple, ports, cadrage
│   └─ formalize.rs       croquis → brouillon formalisé → objet valide
├─ canvas/                rendu : Canvas, Layers, Zones, NodeShape, Port, EdgePath, Minimap
├─ inspector/             inspecteur contextuel non bloquant (propriétés, doc, liens)
├─ library/               vues, domaines, objets, recherche, intentions
├─ control/               inventaires, anomalies, couverture, exports
├─ components/            primitives gelées (boutons, champs, badges, dialogues)
├─ routes/                une fonction par route
└─ i18n/                  fr-FR uniquement, clés typées, aucune chaîne en dur
```

Contraintes DOIVENT :

- Aucun appel `fetch` brut hors de `api_client`. Aucun composant ne construit une URL d'API.
- Aucune vue n’écrit le layout directement : toute mutation passe par `design::commands`.
- Tout `ViewCommand` est validé par `archifier_core` avant projection ; un refus laisse la vue intacte et affiche le code (`DSG-*`, `MM-*`, `DGM-*`).
- Toute validation de formulaire utilise `archifier_core` (mêmes codes d'erreur que le serveur). L'appel réseau reste autoritatif : un `422` est toujours affiché, même si la validation locale est passée.
- Les erreurs `409` déclenchent un dialogue « Conflit de révision » proposant : recharger, comparer, ou forcer (jamais de fusion automatique), sans perdre les croquis locaux.
- Aucune couleur ni taille en dur dans un composant : tout passe par les tokens de `assets/tokens.css`.
- Aucun parcours de création d’objet ne DOIT traverser un formulaire modal (NFR-023). L’inspecteur est un panneau latéral non bloquant.

### 15.4 Écrans clés, comportement attendu

**Atelier — accueil (`/`)**
- Ouvre la dernière vue éditée, sinon un écran « Nouvelle vue » qui n’est pas une page blanche : sept intentions de vue (contexte, applicatif, processus, déploiement, capacité, migration, revue d’interface), chacune avec une vignette et un exemple.
- Propose aussi « Reprendre une vue », « Importer (OpenAPI, snapshot) » et « Explorer la démonstration ». Aucune carte de domaine n’est l’action principale.
- Le premier lancement déclenche une vue guidée : une forme déposée, une relation tirée, une formalisation, un partage. Objectif : première valeur en moins de trois minutes (FR-054).

**Atelier — posture conception (`/design/:id`)**
- Palette contextuelle à gauche, filtrée par `intent` et par métamodèle ; elle ne montre jamais une classe inutilisable dans cette intention.
- Déposer une classe crée d’abord un croquis nommé sur place (`F2`), puis l’inspecteur propose type et domaine. La formalisation peut être différée ; la publication ne le peut pas.
- Tirer d’un nœud vers un autre propose **uniquement** les relations autorisées ; le refus affiche `DSG-002` sans créer d’objet.
- Bandes de couches et zones de domaine sont éditables (déplacer, replier, renommer, colorer par domaine) et se comportent comme des guides aimantés.
- Une frontière de domaine accepte des ports ; poser un port crée ou rattache une interface, avec choix de version.
- Inspecteur à droite, non bloquant : propriétés, domaine, documentation, liens, historique. Il ne masque jamais la vue et ne bloque aucun geste.
- Curseur temporel dans la barre d’outils : changer T ré-encode les formes sans changer la géométrie.
- Barre d’état : `Enregistré · révision N`, nombre de croquis non formalisés, conflit éventuel. Pas de bouton « Enregistrer » principal.
- Actions de vue : publier, partager un lien avec cadrage, exporter SVG/Markdown, ajouter à une Story, capturer pour une Revue.

**Atelier — posture lecture (`/view/diagrams/:id`)**
- Même rendu, chrome réduit : pas de palette, pas de poignées. Le cadrage du lien est appliqué.
- Zoom sémantique : à faible zoom, agrégats par couche et domaine ; au zoom élevé, ports, versions et états.
- Clic sur une forme : aperçu latéral avec lien « ouvrir la fiche ». `Entrée` ouvre la fiche complète.
- Navigation par dépendances sans quitter la vue (surbrillance des chemins entrants/sortants).

**Bibliothèque (`/library`)**
- Recherche transversale (vues, domaines, objets, interfaces, Stories, Revues), filtres par domaine, intention, notation et état temporel.
- Les vues sont présentées comme des **couvertures** (titre, intention, vignette de cadrage, propriétaire, date), pas comme des lignes de table.
- Point d’entrée vers la synthèse d’un domaine : interfaces exposées/consommées, dépendances, vues publiées, Stories.

**Contrôle (`/control`)**
- Inventaire assumé, anomalies, échéances, liens cassés, couverture des métamodèles, exports.
- Chaque anomalie est cliquable vers la vue ou la forme concernée. Aucun écran de contrôle ne propose de « créer » un objet hors vue.

**Canvas conception — détail d’interaction**
- Palette groupée par couche, alimentée par le métamodèle (jamais codée en dur).
- Glisser un type sur le canvas ouvre un libellé éditable sur place, pas un formulaire.
- Panneau d’inspection contextuel : propriétés sémantiques générées depuis le profil, documentation Markdown, liens.
- Indicateur « révision N » et état d’enregistrement automatique ; le bouton d’enregistrement n’existe que comme secours explicite (`Ctrl+S`).

**Fiche d'interface**
- Onglets : Identité · Versions · Calendrier · Contrat · Consommateurs · Dépendances · Documentation · Historique.
- Le calendrier est une frise horizontale avec `now` en repère, une bande par version, coloration `SCHEDULED`/`ACTIVE`/`EXPIRED`, et les périodes de chevauchement hachurées.
- L'onglet Contrat affiche « Aucun contrat OpenAPI » pour une interface non HTTP, sans jamais proposer d'export factice.

**Revue**
- Sélecteur de vue (9 vues de la spec) en barre d'onglets.
- Bandeau d'anomalies en haut, chaque anomalie cliquable vers l'objet.
- Commentaires en fil chronologique, saisie Markdown.
- Statut `OPEN`/`CLOSED` visible et actionnable.

**Lecteur de Story**
- Navigation chapitre par chapitre, blocs empilés, transition vers un bloc cible avec surlignage temporaire (2 s).
- Un bloc `Diagram` applique le `framing` (zoom, focus, atténuation) sans modifier le diagramme source.
- Barre de progression, sommaire latéral, mode plein écran.

**Panneau IA**
- Tiroir latéral droit, persistant entre les routes.
- Streaming token par token, liste de sources cliquables, bouton « Voir la proposition » quand un ChangeSet `DRAFT` arrive.
- L'aperçu du ChangeSet est un diff côte à côte, avec boutons **Valider** puis **Appliquer** distincts (FR-025).

### 15.5 Accessibilité, performance et budgets de l’Atelier

- Navigation clavier complète : `V`/`E`/`R`/`Z`/`N` pour les outils, `Tab` entre formes, flèches pour déplacer, `Entrée` pour ouvrir, `F2` pour renommer, `Suppr` pour retirer la représentation (jamais l'objet), `Ctrl/Cmd+Z` / `Ctrl/Cmd+Maj+Z`, `Ctrl/Cmd+K`.
- Tout état temporel est doublé d’un texte ou d’un motif (NFR-028) ; les contrastes respectent AA.
- `prefers-reduced-motion` respecté sur toutes les transitions ; aucune animation supérieure à 200 ms hors cadrage demandé.
- Budget : bundle WASM ≤ 3,5 Mo gzip, LCP < 2,5 s sur l’Atelier, première forme < 5 s (NFR-021), interaction canvas < 16 ms par frame mesurée au WP-22, ouverture d’une vue de 200 formes < 1 s.
- Aucun écran vide sans action productive (NFR-027).

### 15.6 Grammaire visuelle et anti-CMDB

Ces règles sont **opposables en revue** : elles distinguent un outil d’architecture d’un back-office.

| Règle | Vérification |
|---|---|
| **Une vue, une intention** | La palette, les bandes et le titre découlent de `intent`. Une vue sans intention est refusée à la création. |
| **Le spatial porte du sens** | Les couches sont des bandes, les domaines des zones, les interfaces des ports sur frontière. Aucun de ces éléments n’est relégué dans un panneau de réglages. |
| **Pas de tableau comme écran principal** | L’accueil, la Bibliothèque et une vue n’affichent pas de table de données comme contenu dominant. Les tables appartiennent au Contrôle. |
| **Pas de formulaire comme point d’entrée** | Tout formulaire est contextuel, atteint après un geste de vue, et non bloquant. |
| **Hiérarchie typographique stable** | Nom (fort), type (moyen), domaine (discret), état temporel (badge + motif). Le même vocabulaire sur tous les écrans. |
| **Densité progressive** | À faible zoom, agrégats et étiquettes essentielles ; à zoom élevé, propriétés, ports et versions. Jamais tout au même niveau. |
| **Vide = invitation** | Chaque état vide propose un modèle, un import ou un exemple cliquable, avec une phrase qui dit quoi faire. |
| **Mouvement utile seulement** | Sélection, apparition de lien, cadrage. Aucun effet décoratif. |

**Interdits explicites** (revue bloquante) :

1. Un « tableau de bord » en page d’accueil avec des compteurs comme contenu principal.
2. Une création d’objet qui oblige à quitter la vue.
3. Une vue qui ne se distingue d’une autre que par son titre.
4. Une information critique uniquement dans une infobulle ou uniquement par la couleur.
5. Un écran d’inventaire présenté comme le parcours naturel de l’architecte.

---

## 16. Configuration, exploitation, sécurité

### 16.1 Prérequis de la machine (constatés le 17/11/2026)

| Élément | État | Action WP-00 |
|---|---|---|
| `rustc` / `cargo` 1.94 | présent | — |
| cible `wasm32-unknown-unknown` | **absente** | `rustup target add wasm32-unknown-unknown` |
| `trunk` | **absent** | `cargo install trunk --locked` |
| `sqlx-cli` | **absent** | `cargo install sqlx-cli --no-default-features --features postgres,rustls` |
| PostgreSQL 16+ | **absent** | installer (winget/choco/EDB) ou fournir `DATABASE_URL` |
| Docker | **absent** | non requis ; `scripts/dev-db.ps1` gère une instance locale |
| Node 24 | présent | requis pour Playwright uniquement |

`scripts/verify-toolchain.ps1` DOIT vérifier chacun de ces points et échouer avec un message actionnable.

### 16.2 Configuration

`archifier.toml` (surchargé par variables d'environnement préfixées `ARCHIFIER__`, séparateur `__` pour les sections) :

```toml
[server]
bind = "127.0.0.1:8080"
public_url = "http://127.0.0.1:8080"
static_dir = "dist"

[database]
url = "postgres://archifier:archifier@localhost:5432/archifier"
max_connections = 10
statement_timeout_ms = 5000

[search]
embeddings = "disabled"        # disabled | enabled
embedding_model = "text-embedding-3-small"
embedding_dimensions = 1536

[ai]
enabled = true
base_url = "http://localhost:11434/v1"
model = "qwen3:32b"
max_tool_rounds = 6
request_timeout_s = 60

[session]
ttl_hours = 12
absolute_ttl_hours = 168

[impact]
default_depth = 3
max_depth = 6
```

Le démarrage échoue si un profil de métamodèle est invalide, si la base ne répond pas, ou si les migrations ne sont pas appliquées (`/readyz` renvoie alors `503` avec la raison).

### 16.3 Sécurité

- Cookies `HttpOnly`, `SameSite=Lax`, `Secure` si `public_url` est en HTTPS.
- CSRF double-submit sur toute méthode non idempotente.
- Filigrane HTML : liste blanche stricte (§12.1). Le Markdown n'est jamais injecté en `innerHTML` brut.
- Taille maximale de corps : 2 Mo pour le JSON, 10 Mo pour l'import OpenAPI.
- Rate limit : 120 req/min/utilisateur sur `/api/ai/*`, 600 req/min ailleurs (tower middleware, en mémoire).
- Journalisation : identifiant, route, statut, durée. Jamais de contrat, jamais de prompt (NFR-013).
- Pas de secret en clair dans le dépôt ; `.env` ignoré, `.env.example` fourni.

### 16.4 Déploiement

- Binaire unique `archifier-server` qui sert l'API **et** les fichiers statiques (`dist/` produit par Trunk).
- Migrations appliquées au démarrage si `ARCHIFIER__DATABASE__MIGRATE_ON_START=true`, sinon commande `archifier-server migrate`.
- Sauvegarde/restauration : `pg_dump`/`pg_restore` ; test automatisé prouvant la préservation des IDs et de l'historique (NFR-010) au WP-22.
- Aucune dépendance à un service cloud : PostgreSQL seul.

---

## 17. Stratégie de tests

### 17.1 Pyramide

| Niveau | Outil | Périmètre | Cible |
|---|---|---|---|
| Unitaire pur | `cargo test -p archifier-core -- -p archifier-metamodel` | règles, temporalité, diff, URI, Changeset | > 90 % de branches sur `core` |
| Propriétés | `proptest` | invariants temporels, idempotence de `apply`, absence de cycle, round-trip URI | 5 propriétés minimum |
| Intégration store | `sqlx::test` sur base réelle | chaque transaction du chemin d'écriture | 1 test par règle |
| Contrat HTTP | `axum::Router` + `tower::ServiceExt` | codes, en-têtes, problem+json | tous les endpoints |
| E2E | Playwright | 5 chaînes de recette + 13 scénarios Gherkin | bloquant |
| Non-régression perf | Playwright + `trace` | fiche objet < 1 s, recherche < 2 s, canvas 500 nœuds | bloquant |

### 17.2 Traçabilité Gherkin

Les 13 scénarios de la spec fonctionnelle sont copiés **mot pour mot** dans `specs/features/*.feature` (source de vérité documentaire) et chacun a un test Playwright dont le titre commence par son nom. Un test `specs_manifest` vérifie que chaque scénario a un test correspondant (correspondance par nom, échoue si un scénario est absent).

Correspondance obligatoire :

| Scénario | Test E2E |
|---|---|
| Créer un élément sans quitter la vue | `e2e/create-element-in-view.spec.ts` |
| Refuser une relation interdite au moment du geste | `e2e/forbidden-relation.spec.ts` |
| Formaliser un croquis avant publication | `e2e/draft-requires-formalization.spec.ts` |
| Lire la migration de versions dans la vue | `e2e/view-temporal-cursor.spec.ts` |
| Ouvrir une vue partagée | `e2e/shared-view-link.spec.ts` |
| Réutiliser une interface d'un autre domaine | `e2e/interface-reuse.spec.ts` |
| Refuser une période invalide | `e2e/invalid-window.spec.ts` |
| Afficher une interface future | `e2e/scheduled-interface.spec.ts` |
| Détecter un consommateur d'une version expirée | `e2e/expired-consumer.spec.ts` |
| Rendre une réponse IA vérifiable | `e2e/ai-sourced-answer.spec.ts` |
| Interdire une modification IA silencieuse | `e2e/ai-no-silent-write.spec.ts` |
| Parcourir une Story | `e2e/story-transition.spec.ts` |
| Réimporter le même OpenAPI | `e2e/openapi-idempotent.spec.ts` |

### 17.3 Les cinq chaînes de recette

Elles sont jouées par un test E2E unique `e2e/acceptance-chains.spec.ts`, en cinq `test()` séquentiels partageant une base réinitialisée :

1. **Conception** — l’Atelier s’ouvre sur une nouvelle vue « Applicatif d’un domaine » ; trois composants sont déposés et reliés ; une frontière reçoit deux ports et deux interfaces ; les croquis sont formalisés ; `DSG-002` puis `DSG-001` sont provoqués ; le curseur temporel lit la migration ; la vue est publiée et son lien restitue le cadrage. Aucun écran d’inventaire n’est ouvert.
2. **Domaine et interface** — Paiements publie v1 (1 janv.–30 sept.) puis v2 (à partir du 1 juil.) ; Commande réutilise par ID et déclare la consommation ; la revue montre v1, v2, consommateurs et la fenêtre de migration.
3. **Multi-notation** — un `BusinessProcess` ArchiMate raffiné par un `Process` BPMN ; une `ServiceTask` liée à un `ApplicationService` ; réalisé par un `Component` UML et une interface HTTP ; navigation bidirectionnelle, objets jamais fusionnés.
4. **Documentaire** — une Story traverse trois diagrammes, cite deux interfaces, exporte un bundle Markdown dont **tous** les liens relatifs résolvent (test de validité des liens sur le ZIP extrait).
5. **IA (mock)** — « Qu'est-ce qui casse si Payment API v1 disparaît le 1er juillet ? » ⇒ résolution par ID, fenêtre temporelle, parcours, liste d'objets sourcés. Puis « propose-moi une vue de cet impact » ⇒ ChangeSet `DRAFT` **non appliqué** ; le test vérifie qu'aucune ligne d'objet n'a changé.

### 17.4 Qualité de code (gates CI bloquants)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p archifier-ui --target wasm32-unknown-unknown
trunk build --release
npx playwright test
```

`clippy.toml` interdit : `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!` hors tests (`#[cfg(test)]` autorisé). Le code de bibliothèque retourne des `Result` avec des erreurs typées. Les handlers axum ne contiennent **aucun** `sqlx::query` d'écriture (test d'architecture).

---

## 18. Plan d'implémentation et orchestration multi-agents

### 18.1 Réponse à la question posée

**Oui, c'est faisable**, à condition d'accepter quatre contraintes explicites :

1. **Les contrats doivent être figés avant la parallélisation.** `archifier-core`, `archifier-dto` et le schéma SQL sont gelés à la porte `G1`. Tant qu'ils bougent, aucun agent ne travaille en parallèle : les conflits de compilation coûtent plus cher que le temps gagné.
2. **Un agent = un périmètre de fichiers disjoint.** Deux agents ne modifient jamais le même fichier dans la même vague. Le découpage ci-dessous est conçu pour cela.
3. **Chaque lot est livrable et testable seul.** Un lot sans test vert n'est pas terminé, quelle que soit la qualité perçue du code.
4. **La recette n'est pas déléguée.** Les cinq chaînes de §17.3 sont exécutées et vérifiées à chaque porte par l'intégrateur, pas par l'agent qui a écrit la fonctionnalité.

Limites honnêtes, à ne pas découvrir en cours de route :

- L'IA générative produit du code **plausible** ; la plausibilité n'est pas la correction. Les gates de compilation et de tests sont le seul filet réel. Ils doivent être **bloquants**, pas consultatifs.
- Les décisions d'esthétique (identité visuelle, densité, ton des libellés) ne sont pas dans les spécifications. Le §15 impose un système de tokens minimal ; au-delà, un arbitrage humain est nécessaire.
- La chaîne IA nécessite un modèle accessible. Sans lui, tout est développé et testé avec `MockProvider`, et la démonstration IA se fait en mode scripté.
- Les performances réelles (NFR-005 à 007) ne peuvent être affirmées qu'après mesure au WP-22, pas déduites.

### 18.2 Découpage en lots

Vagues et dépendances :

```text
VAGUE A — Fondations (séquentiel)
  WP-00  Socle & outillage
  WP-01  Domaine & invariants (core)
  WP-02  Métamodèles (profils + registre)
  WP-03  Schéma SQL & accès (store)
  WP-04  DTO & contrat d'API
  WP-05  Erreurs, auth, routing, wiring (api)
        ⇒ PORTE G1 : contrats gelés, `cargo check --workspace` vert

VAGUE B — Domaine métier (parallélisable, 4 agents max)
  WP-06  Domaines, éléments, relations, révisions, audit
  WP-07  Interfaces, versions, temporalité, consommation, diff
  WP-08  Diagrammes, layout, usages
  WP-09  Documentation, liens arch://, diagnostic
  WP-10  Revues, anomalies, commentaires
  WP-11  Stories
  WP-12  Recherche (FTS + hybride optionnel)
  WP-13  OpenAPI import/export
  WP-14  Export Markdown + Mermaid
        ⇒ PORTE G2 : tous les endpoints métier verts, tests unitaires+intégration complets

VAGUE C — Intelligence (séquentiel court, dépend de B)
  WP-15  Fournisseur LLM, outils, orchestrateur, SSE
  WP-16  ChangeSet : validation, preview, application, stale
  WP-17  Façade MCP
        ⇒ PORTE G3 : chaîne IA de recette scriptée verte, aucune écriture silencieuse

VAGUE D — Front (parallélisable, 4 agents max)
  WP-18  Coque Leptos : router, auth, Atelier / Bibliothèque / Contrôle, client API, tokens, i18n
  WP-19  Atelier de vues : commandes, outils, undo/autosave, palette, inspecteur, formalisation
  WP-20  Fiches objet, documentation Markdown, historique/diff
  WP-21  VISUALISER : accueil domaine, interface+timeline, graphe de dépendances, date T
  WP-22  Revues et Stories (éditeur + lecteur)
  WP-23  Recherche globale + panneau IA + ChangeSet diff
  WP-26  Intentions de vue, onboarding, partage de vue, Bibliothèque et Contrôle
        ⇒ PORTE G4 : parcours utilisateur complet, budget bundle respecté

VAGUE E — Recette & durcissement
  WP-24  5 chaînes E2E + 13 scénarios Gherkin + perf + sauvegarde/restauration
  WP-25  CI, packaging, runbook, documentation
        ⇒ PORTE G5 : MVP1 accepté
```

### 18.3 Fiches de lot

Chaque fiche est un fichier `docs/wp/WP-XX-*.md` produit au démarrage du lot, contenant : objectif, périmètre de fichiers **exclusif**, contrats consommés (signatures exactes des types partagés), critères d'acceptation testables, commandes de vérification, et ce qui est explicitement hors périmètre.

| Lot | Périmètre de fichiers | Dépend de | Critère d'acceptation principal |
|---|---|---|---|
| WP-00 | `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `scripts/*`, `.gitignore` | — | `cargo check --workspace` et `trunk build` passent sur un « hello world » |
| WP-01 | `crates/core/src/{ids,object,error,uri,markdown}` | WP-00 | tests d'identifiants, URI, invariants OBJ-001..006 |
| WP-02 | `crates/metamodel/**`, `profiles/*.toml`, `docs/metamodel-coverage.md` | WP-01 | `profile_lint` vert, 42 classes ArchiMate chargées, rapport généré |
| WP-03 | `migrations/*.sql`, `crates/store/src/{pool,repos,tx}` | WP-01, WP-02 | migrations appliquées sur base vierge, `write_object` testé |
| WP-04 | `crates/dto/**` | WP-01 | tous les DTO sérialisables, round-trip JSON |
| WP-05 | `crates/api/src/{router,error,auth,extract}`, `crates/server/src/main.rs`, `archifier.toml` | WP-03, WP-04 | `/healthz`, `/readyz`, login/logout, problem+json conformes |
| WP-06 | `crates/api/src/handlers/{domains,elements,relationships}.rs`, `store/src/repos/{domains,elements,relationships}.rs` | G1 | CRUD complet, `If-Match`, archivage, historique, diff de révisions |
| WP-07 | handlers+repo `interfaces`, `core/src/temporal.rs`, `core/src/diff/contract.rs` | G1 | BR-001..012, timeline, overlaps, diff de contrat classé |
| WP-08 | handlers+repo `diagrams`, `store/src/diagram_usages.rs` | G1 | DGM-001..006, recalcul des usages, PATCH layout |
| WP-09 | handlers+repo `documentation`, `links`, `core/src/uri.rs`, diagnostic | G1 | DOC-LINK-001/002, résolution `ARCHIVED`, rebuild |
| WP-10 | handlers+repo `reviews`, anomalies | WP-07 | les 9 vues, `DEPENDENCY_ON_EXPIRED_INTERFACE`, commentaires |
| WP-11 | handlers+repo `stories` | G1 | invariants STORY-*, `/play` en un appel |
| WP-12 | `crates/search/**`, projection `search_documents` | WP-06..WP-09 | FTS français pertinent, trigramme, mode `hybrid` dégradable |
| WP-13 | `crates/openapi/**`, handlers import/export | WP-07 | import 3.0/3.1/3.2, fingerprint, idempotence, diff |
| WP-14 | `crates/docgen/**`, handlers export | WP-06..WP-11 | bundle Markdown reproductible, liens relatifs valides, Mermaid |
| WP-15 | `crates/ai/src/{provider,tools,orchestrator}`, SSE | WP-12 | streaming, sources, mock en CI, aucun prompt loggé |
| WP-16 | `crates/ai/src/{schema}`, `core/src/changeset.rs`, `store/src/tx/apply_changeset` | WP-15 | CS-001/002/STALE, atomicité, aucune écriture sans `confirmed` |
| WP-17 | `crates/ai/src/mcp.rs` | WP-15 | `tools/list` = schéma unique, `resources/read` sur `arch://` |
| WP-18 | `crates/ui/src/{app,api_client,state,components,i18n,routes,library,control}`, `index.html`, `Trunk.toml`, `assets/tokens.css` | G2/G3 | Atelier par défaut, Bibliothèque et Contrôle, gestion 409, aucun écran d’entrée de type inventaire |
| WP-19 | `crates/ui/src/{design/*,canvas/*,inspector/*,routes/design.rs}`, `crates/core/src/view_command.rs` | WP-18, WP-08 | première forme < 5 s, création sans modal, commandes validées, undo/autosave, 500 formes fluides |
| WP-26 | `crates/ui/src/{library/*,control/*,routes/home.rs,routes/onboarding.rs}`, `assets/view-templates/*.json`, `fixtures/views/*.json` | WP-18, WP-19 | 7 intentions de vue, onboarding < 3 min, lien partagé avec cadrage, zéro table comme écran principal |
| WP-20 | `crates/ui/src/routes/{view_element,model_element,documentation,history}.rs` | WP-18 | fiche < 1 s, éditeur Markdown + aperçu, diff de révisions |
| WP-21 | `crates/ui/src/routes/{home,view_domain,view_interface,timeline,graph,at_date}.rs` | WP-18, WP-07 | accueil domaine, timeline avec overlaps, graphe navigable |
| WP-22 | `crates/ui/src/routes/{review,story_editor,story_player}.rs` | WP-18, WP-10, WP-11 | 9 vues de revue, lecteur avec framing, transitions |
| WP-23 | `crates/ui/src/routes/{search,ai_panel,changeset_diff}.rs` | WP-18, WP-12, WP-16 | sources cliquables, diff, Valider/Appliquer distincts |
| WP-24 | `tests/e2e/**`, `specs/**`, `scripts/measure.*` | G4 | 5 chaînes + 13 scénarios + budgets perf verts |
| WP-25 | `.github/workflows/*`, `docs/runbook.md`, `README.md` | G4 | CI complète < 20 min, runbook exécutable |

### 18.4 Règles de collaboration entre agents

DOIVENT être respectées par tout agent travaillant sur un lot :

1. **Lire la fiche de lot et les contrats figés avant d'écrire.** Ne jamais modifier `archifier-core`, `archifier-dto` ou une migration pour faire passer son propre code : ouvrir une demande d'évolution à l'intégrateur.
2. **Ne jamais inventer une API de dépendance.** Si la signature n'est pas connue avec certitude, la vérifier sur `docs.rs` avant usage.
3. **Aucun `unwrap`/`expect`/`panic!` hors tests.** Erreurs typées, `Result` en cascade.
4. **Un lot = ses tests.** Chaque règle `XX-NNN` citée dans la fiche a un test nommé d'après elle.
5. **Pas de dépendance nouvelle sans accord.** Une crate ajoutée est une décision d'architecture.
6. **Migrations additives uniquement.** Un numéro consommé ne se réutilise jamais.
7. **Le périmètre de fichiers est exclusif.** Un agent qui a besoin de toucher un fichier d'un autre lot le signale, il ne le fait pas.
8. **Le rapport de fin de lot est court et factuel** : fichiers créés, tests ajoutés, règles couvertes, points ouverts, commandes exécutées et leur résultat.

### 18.5 Portes de qualité

| Porte | Moment | Critère de passage |
|---|---|---|
| `G1` | fin vague A | `cargo check --workspace` + `cargo test -p archifier-core -p archifier-metamodel` verts ; migrations rejouables sur base vierge ; contrats signés |
| `G2` | fin vague B | tous les endpoints métier répondent ; chaque règle `BR-*`, `MM-*`, `DGM-*`, `DOC-*` a un test vert ; scénario « période invalide » joué en HTTP |
| `G3` | fin vague C | chaîne IA mock verte ; tentative d'`apply` sans `confirmed` refusée ; `.sqlx`/logs sans prompt |
| `G4` | fin vague D | parcours complet cliquable ; bundle ≤ 3,5 Mo gzip ; aucun `console.error` en démo |
| `G5` | fin vague E | 5 chaînes + 13 scénarios + budgets perf + sauvegarde/restauration verts ; runbook exécuté à froid |

### 18.6 Ce qui reste à décider par un humain

| Sujet | Impact si non tranché |
|---|---|
| Fournisseur et modèle LLM réel | La chaîne IA reste en `MockProvider` (développement et recette restent possibles) |
| Identité visuelle (couleurs, typographie, logo) | Le MVP utilise des tokens neutres ; un restyling ultérieur est peu coûteux si les tokens sont respectés |
| Instance PostgreSQL cible et politique de sauvegarde | Développement local possible, déploiement non |
| Comptes utilisateurs initiaux | L'app démarre avec le compte créé par CLI |
| Activation des embeddings | Recherche purement lexicale ; le mode `hybrid` reste implémenté et testable mais désactivé |

---

## 19. Matrice de traçabilité

| Exigence | Implémentation | Lot | Vérification |
|---|---|---|---|
| FR-001 | `/api/domains` CRUD + archivage | WP-06 | E2E, tests handler |
| FR-002 | `objects.responsible_domain_id` obligatoire, `OBJ-003/005/006` | WP-01, WP-06 | proptest, tests store |
| FR-003 | Création typée par profil, palette alimentée par le registre | WP-02, WP-06, WP-19 | `MM-ELEM-001`, tests palette |
| FR-004 | ULID opaque | WP-01 | round-trip, test d'indépendance nom/type |
| FR-005 | Nœuds de diagramme → `object_id` | WP-08, WP-19 | scénario Gherkin « réutiliser » |
| FR-006 | Aucune restriction de domaine sur les références | WP-08 | test multi-domaines |
| FR-007 | `interfaces.category` énuméré | WP-07 | contrainte CHECK + test |
| FR-008 | `publish` + `available_from/until` + `published_at` | WP-07 | BR-001, BR-002 |
| FR-009 | `interface_consumers` + relation `CONSUMES` | WP-07 | test de consommation |
| FR-010 | Filtres `domain/category/state/consumer/version/from/until` | WP-07 | tests de filtrage combinatoire |
| FR-011 | `dependencies?direction=` + CTE récursif | WP-06, WP-21 | tests de parcours borné |
| FR-012 | `contract_diff` + endpoint `diff/{a}/{b}` | WP-07, WP-13 | tests de classification BREAKING/COMPATIBLE |
| FR-013 | `reviews` + 9 vues + anomalies | WP-10, WP-22 | tests par vue |
| FR-014 | `comments` | WP-10 | CRUD commentaires |
| FR-015 | Bascule persistante conception/lecture dans la même vue | WP-18, WP-19 | E2E de navigation, cadrage conservé |
| FR-040 | Route `/` = Atelier, Bibliothèque et Contrôle séparés | WP-18, WP-26 | E2E : aucun inventaire au lancement |
| FR-041 | `ViewCommand` de création depuis la vue (élément, relation, interface, zone) | WP-19 | test de commande par type d’objet |
| FR-042 | Édition sur place + inspecteur non bloquant | WP-19 | E2E : aucun changement de route |
| FR-043 | `Connect` filtré par `is_relation_allowed` | WP-19 | `DSG-002` |
| FR-044 | Ports et frontières d’interface | WP-19 | E2E de pose de port |
| FR-045 | Bandes, zones, groupes, annotations | WP-19 | tests de layout et de rendu |
| FR-046 | Catalogue d’intentions de vue | WP-26 | 7 intentions, palette dérivée |
| FR-047 | Cycle croquis → formalisé → valide | WP-19, WP-26 | `DSG-001`, `DSG-003` |
| FR-048 | Métadonnées de vue + bibliothèque de vues | WP-26 | filtres, vignettes, cadrage |
| FR-049 | Curseur temporel et encodage des états | WP-21, WP-26 | scénario migration |
| FR-050 | Autosave, undo/redo, palette de commandes | WP-19 | mesure NFR-024 |
| FR-051 | Publication, lien partagé, export, Story/Revue | WP-26, WP-22 | lien restitue le cadrage |
| FR-052 | Espace Contrôle secondaire | WP-18, WP-26 | revue : aucun accès obligatoire |
| FR-053 | Parité clavier et états vides productifs | WP-18, WP-19 | audit a11y, checklist |
| FR-054 | Onboarding première vue en < 3 min | WP-26 | test de parcours chronométré |
| FR-016 | `documentation_blocks` Markdown pour tout objet | WP-09 | test sur chaque `ObjectType` |
| FR-017 | `arch://` typé + résolution | WP-01, WP-09 | `DOC-LINK-001`, round-trip |
| FR-018 | `StoryStructure` 6 types de blocs | WP-11, WP-22 | invariants STORY-* |
| FR-019 | `docgen` bundle ZIP | WP-14 | liens relatifs valides, reproductibilité |
| FR-020 | Import OpenAPI + rapprochement | WP-13 | idempotence, preview |
| FR-021 | Export OpenAPI 3.1 depuis snapshot | WP-13 | test d'export déterministe |
| FR-022 | `/api/ai/chat` SSE + `source` events | WP-15 | scénario « réponse vérifiable » |
| FR-023 | `/api/interfaces/{id}/impact` déterministe | WP-07, WP-15 | chaîne 4 de recette |
| FR-024 | `propose_changes` (5 natures) | WP-15 | tests par nature |
| FR-025 | ChangeSet `DRAFT → VALIDATED → APPLIED` + `confirmed` | WP-16 | scénario « modification silencieuse » |
| FR-026 | `object_revisions` + `diff/{a}/{b}` | WP-03, WP-06 | test de révision immuable |
| FR-027 | `links.status` + diagnostic | WP-09 | `DOC-LINK-002` |
| FR-028 | `diagram_usages` + `/usages` + `/diagrams` | WP-08, WP-20 | test inverse |
| FR-029 | `/api/architecture/at` + `?at=` sur toutes les vues | WP-07, WP-21 | fenêtres semi-ouvertes |
| FR-030 | `search_documents.tsv` + trigramme | WP-12 | pertinence sur corpus de test |
| NFR-001 | `audit_events` à chaque écriture | WP-03 | test d'audit |
| NFR-002 | `object_revisions` en append-only (aucun UPDATE) | WP-03 | test d'architecture SQL |
| NFR-003 | Validation dans `core`, exécutée côté serveur | WP-01 | tests partagés |
| NFR-004 | ChangeSet validé par `core` | WP-16 | test de contournement refusé |
| NFR-005 | `overview` et `/play` en un appel | WP-07, WP-11 | mesure p95 |
| NFR-006 | Index GIN + `statement_timeout` | WP-12 | mesure p95 |
| NFR-007 | SVG + culling + mémoïsation | WP-19 | mesure 500 nœuds |
| NFR-008 | `timestamptz` + UTC + conversion front | WP-03, WP-18 | test de fuseau |
| NFR-009 | Export sans horodatage, ordre stable | WP-14 | test octet-à-octet |
| NFR-010 | IDs et historique hors dump applicatif | WP-24 | test de restauration |
| NFR-011 | Contexte LLM limité aux résultats d'outils | WP-15 | test de payload |
| NFR-012 | Aucune écriture LLM directe (trait read-only) | WP-16 | test d'architecture |
| NFR-013 | Logs sans prompt ni contrat | WP-15 | test de capture de logs |
| NFR-014 | Profils externes + `metaclass` en `text` | WP-02 | test de chargement d'un profil alternatif |
| NFR-021 | Intentions + onboarding + palette contextuelle | WP-26 | mesure première forme < 5 s |
| NFR-022 | Création en ≤ 3 gestes, sans changement d’écran | WP-19 | test de parcours |
| NFR-023 | Inspecteur non modal | WP-18, WP-19 | revue + E2E |
| NFR-024 | Autosave 800 ms + pile d’annulation ≥ 100 | WP-19 | test de temporisation et d’historique |
| NFR-025 | Rendu incrémental, culling, pas de re-render global | WP-19 | profilage 500 formes |
| NFR-026 | Parité clavier des gestes | WP-19 | tests clavier |
| NFR-027 | États vides productifs | WP-26 | revue écran par écran |
| NFR-028 | États temporels redondants (motif + texte) | WP-21 | audit visuel |
| NFR-029 | Croquis/formalisation visibles | WP-19 | `DSG-001` |
| NFR-030 | Ouverture de vue < 1 s, cadrage restauré | WP-19 | mesure |

---

## 20. Journal des décisions d'architecture (ADR à produire)

| ADR | Sujet | Décision |
|---|---|---|
| ADR-001 | Front Rust/WASM vs Angular | Rust/WASM (§1) |
| ADR-002 | Leptos vs Dioxus vs Yew | Leptos CSR (§15.1) |
| ADR-003 | Queries SQL runtime vs macros `sqlx` | Runtime + `FromRow`, tests d'intégration obligatoires |
| ADR-004 | Layout de diagramme en JSONB | JSONB + projection `diagram_usages` (écart assumé au modèle physique de la spec fonctionnelle) |
| ADR-005 | Domaine comme `RepositoryObject` | Oui, pour unifier historique/documentation/liens |
| ADR-006 | Historique par snapshot JSONB vs événements | Snapshot par révision, diff structurel à la lecture |
| ADR-007 | Pas de brouillon pour diagrammes/Stories | Axe unique `lifecycle`, aligné sur l'arbitrage §0.1 |
| ADR-008 | Rendu SVG vs Canvas 2D | SVG, culling au-delà de 200 nœuds |
| ADR-009 | pgvector optionnel | Activé par configuration, dégradation explicite en lexical |
| ADR-010 | MCP core seulement | MCP Apps / AG-UI / A2UI exclus du MVP1 |
| ADR-011 | Persistance locale | Snapshot unique IndexedDB plutôt que 21 object stores |
| ADR-012 | Routage | Routage par fragment `#/...`, sans dépendance ni repli SPA |
| ADR-013 | IA navigateur | Aucune clé d'API dans le bundle ; panneau IA générative non livré |
| ADR-014 | Édition collaborative | CRDT (Yrs) limité au plan collaboratif ; la gouvernance reste en verrou optimiste (§21) |
| ADR-015 | Transport de synchronisation | local d'abord (`BroadcastChannel`), relais WebSocket préparé derrière le même trait (§21.4) |
| ADR-016 | Port de dépôt | Le type `Repository`, centralisé par `state::repo()`, est le port effectif du MVP2 ; pas d'extraction de trait |
| ADR-017 | Proxy LLM local | Binaire optionnel, clé en variable d'environnement, CORS restreint, désactivé par défaut (W6) |
| ADR-018 | Contrats externes | OpenAPI 3.0/3.1/3.2 et AsyncAPI 2.x/3.x, brut + empreinte SHA-256, import YAML/JSON, export reproductible (W1/W5) |
| ADR-019 | Cycle de vie des interfaces | Dépréciation, remplacement, retrait, politique de rupture, notifications dérivées (W5) |
| ADR-020 | Virtualisation du canvas | Culling par viewport, 500 nœuds, sans dépendance (W7) |
| ADR-021 | Migration ArchiMate 3.2 → 4.x | Mapping explicite, aucune classe 4 déclarée, préservation des IDs (W2) |
| ADR-022 | Nature de l’UI et surface d’édition | L’Atelier est la surface de création ; le référentiel reste la vérité ; l’inventaire est un espace de Contrôle secondaire. **Point d’entrée remplacé par ADR-023** |
| ADR-023 | Produit, carte et zoom | La Carte de l’entreprise est le point d’entrée ; produit/équipe/value stream/capacité sont des objets de premier rang ; cinq niveaux de zoom et lentilles ; ArchiMate socle, BPMN/UML contextuels |

---

## 21. Collaboration temps réel (MVP2)

### 21.1 Principe : deux plans, deux algos

La collaboration ne s'applique pas au référentiel entier. Deux plans coexistent, avec des
garanties opposées.

| Plan | Contenu | Garantie | Implémentation |
|---|---|---|---|
| Gouvernance | domaines, éléments, relations, interfaces, versions, contrats, statuts de revue, ChangeSets | écriture unique, `revision` + `If-Match` | inchangée depuis le §6.3 |
| Collaboration | documentation Markdown, `name`, layout de diagramme, blocs de Story, commentaires, présence | convergence automatique | CRDT |

Justification : un contrat d'interface ne se « fusionne » pas comme un paragraphe. Une
version publiée exige une confirmation humaine (`BR-010`, `FR-025`) ; un paragraphe et une
position de nœud se réconcilient sans intervention.

Conséquence architecturale : le chemin d'écriture de gouvernance (cible de nommage `WriteTx`,
réellement `Repository::mutate` au MVP2) ne concerne plus que le plan de gouvernance, et un
second chemin d'écriture, `CollabEdit`, alimente le CRDT. Les deux écrivent dans le
`Dataset`, mais jamais par la même porte.

```rust
impl Repository {
    // … API existante …
    pub fn collab_doc(&self, object_id: ObjectId) -> Option<CollabDoc>;
    pub fn apply_collab_edit(&self, edit: CollabEdit) -> Result<(), RepoError>;
    pub fn subscribe_collab(&self, on: Box<dyn Fn(CollabEvent)>);
}
```

> **Amendement MVP2 (ADR-016, 18/11/2026).** Le port effectif du MVP2 est le type
> concret `archifier_repo::Repository`, centralisé par `crates/ui/src/state.rs::repo()`.
> Aucune vue n'accède au stockage ; `boot.rs` est la racine de composition normale.
> `WriteTx` reste une cible de nommage, pas un type du MVP2. Voir
> `docs/adr/ADR-016-port-depot-type-repository.md`.

### 21.2 CRDT retenu : Yrs (Yjs en Rust)

| Option | Avantage | Inconvénient |
|---|---|---|
| **Yrs** (`y-crdt`) | `Y.Text` éprouvé pour le texte riche, `Y.Map`/`Y.Array`, vecteurs d'état compacts, WASM natif | dépendance lourde (~300 Ko de WASM), surface d'API à valider |
| Automerge | modèle JSON, historique riche | plus lourd, protocole maison, empreinte mémoire supérieure |
| Loro | perf, Rust natif récent | écosystème plus jeune, moins de retours sur le texte |
| CRDT maison (LWW + RGA) | zéro dépendance, contrôle total | réimplémenter un texte CRDT correct est un projet en soi |

Décision : **Yrs**, avec un **spike borné** avant engagement définitif (voir ADR-014).
Le critère d'abandon est explicite : si le spike ne tient pas en une journée, on bascule
sur Automerge ; on ne réécrit pas un CRDT maison.

### 21.3 Une unité de collaboration = un document

Un `Y.Doc` par objet collaboratif (`doc:<object_id>`), plus un document racine
d'index. Ce découpage permet le chargement paresseux exigé par `NFR-020` : ouvrir un
domaine ne charge pas la documentation de ses 200 éléments.

| Donnée du référentiel | Type CRDT | Contrainte |
|---|---|---|
| `DocumentationBlock.markdown` | `Y.Text` | curseurs distants, pas de perte de frappe |
| `RepositoryObject.name` | `Y.Text` | renommage concurrent convergent |
| `DiagramNode.x`, `.y` | `Y.Map` (LWW) | dernière opération gagnante, pas de saut |
| `DiagramLayout.nodes`, `.edges` | `Y.Array<Y.Map>` | ajout et suppression concurrents |
| `StoryStructure.chapters`, `blocks` | `Y.Array<Y.Map>` | réordonnancement convergent |
| `Comment.markdown` | `Y.Text` | ajout concurrent sans perte |
| `InterfaceVersion.contract_snapshot` | **interdit en CRDT** | plan de gouvernance (`CR-001`) |

Le CRDT ne stocke **que** ces champs : les identifiants, domaines, types et relations
restent dans le plan de gouvernance. Un document ne peut donc pas créer un objet ni
publier une version (`CR-002`).

### 21.4 Transport : le même contrat, du local d'abord

```rust
pub type SubscriptionId = u64;

pub trait SyncTransport {
    fn send(&self, message: SyncMessage) -> Result<(), SyncError>;
    fn subscribe(&self, on: Box<dyn Fn(SyncMessage) + 'static>) -> SubscriptionId;
    fn unsubscribe(&self, id: SubscriptionId);
    fn peers(&self) -> Vec<PeerId>;
    fn status(&self) -> SyncStatus;   // Disconnected | Local | Connected
}

pub enum SyncMessage {
    /// Mise à jour différentielle (encodage binaire Yrs).
    Update { doc: DocId, from: PeerId, payload: Vec<u8> },
    /// Rattrapage : le pair demande ce qui lui manque depuis son vecteur d'état.
    StateVector { doc: DocId, from: PeerId, payload: Vec<u8> },
    /// Présence : éphémère, jamais persistée.
    Awareness { from: PeerId, payload: Vec<u8> },
}
```

> **Amendement MVP2 (C-03, ADR-015).** Cette signature est la référence. Le canal exact
> est `archifier:collab:v1` ; l'enveloppe binaire et les limites sont définies dans
> `technical-specifications-lot2.md` §5.

| Implémentation | Statut | Transport |
|---|---|---|
| `LocalBroadcastTransport` | **MVP2, nominal** | `BroadcastChannel("archifier:collab:v1")`, payloads binaires |
| `InProcessTransport` | tests | canal en mémoire, ordre contrôlable, permet de simuler partitions et retard |
| `WebSocketTransport` | **préparé, non activé** | relais réseau, même encodage, ajouté au lot 3 |

`LocalBroadcastTransport` offre une co-édition réelle entre deux fenêtres du même
navigateur : c'est la démonstration exigée par `FR-036`, et elle ne dépend d'aucun serveur.
Ses limites sont assumées : même origine, même profil de navigateur, aucune frontière de
sécurité. Le relais réseau devra ajouter authentification et autorisation — ce n'est pas
couvert ici.

Rattrapage : à la connexion d'un pair, on échange les **vecteurs d'état** et on ne
transmet que le delta (`NFR-017`). Un `Update` ne dépasse jamais quelques kilo-octets ;
le document complet n'est jamais renvoyé à chaque frappe.

### 21.5 Persistance et démarrage

Un store IndexedDB distinct, `collab`, contient un enregistrement binaire par document
(encodage compact Yrs), avec `doc_id`, `schema`, `updated_at`. On ne touche pas au
snapshot unique de `ADR-011` : la documentation et le layout y restent présents pour
l'export, mais la **source de vérité** du texte et du layout devient le document CRDT.

Au démarrage :

```text
1. charger l'index des documents collaboratifs
2. pour chaque document connu : décoder, appliquer les mises à jour locales en attente
3. si un objet est éditable mais n'a pas de document : l'initialiser depuis le snapshot
   (drapeau `seeded: true` dans le document, pour une réinitialisation idempotente)
4. publier un état vers les pairs via le transport
```

Écriture : l'édition locale est appliquée immédiatement à l'écran (`NFR-019`), encodée,
puis diffusée. Le flush disque est débouncé (400 ms) ; un flush forcé a lieu sur
`visibilitychange` et avant fermeture.

### 21.6 Présence

- `PeerId` aléatoire par session (jamais persisté, jamais exporté — `NFR-018`).
- Contenu : nom affiché, objet courant, éventuellement offset de curseur dans un `Y.Text`.
- Battement toutes les 10 s, expiration à 30 s, disparition propre à la fermeture.
- Rendu : pastilles d'éditeurs sur la fiche et le canvas, jamais de curseur intrusif par
  défaut.

### 21.7 Validation après fusion : jamais silencieuse

Un CRDT converge, il ne valide pas. Deux fenêtres peuvent donc produire ensemble un état
que le moteur refuserait séparément. La règle est explicite (`CR-003`, `FR-037`) :

```text
apply(update distant)
  └─ reconstruire la vue de domaine concernée
     └─ archifier_core::validate_*()
        ├─ conforme            → rien à signaler
        └─ non conforme        → diagnostic MERGE_VIOLATION (local, non persisté)
                                 + objet marqué « non conforme » dans l'interface
                                 + aucune suppression, aucune correction automatique
```

L'utilisateur arbitre : il peut annuler sa propre édition, archiver, ou corriger. Le
diagnostic liste les codes de règle violés et le pair à l'origine de la branche quand il
est identifiable.

### 21.8 Historique lisible

Sans précaution, chaque frappe créerait une révision. `FR-038` impose l'inverse :

- les mises à jour collaboratives sont accumulées ;
- après 2 s d'inactivité (ou 30 modifications), **une** entrée est écrite dans
  `object_revisions`, avec le résumé « Édition collaborative » et l'auteur de la session ;
- un flush explicite (navigation, fermeture) produit la révision finale ;
- l'historique affiché reste donc celui d'une action humaine, pas d'un flux machine.

### 21.9 Budgets et modes dégradés

| Mesure | Cible |
|---|---|
| Latence perçue d'une frappe | < 16 ms (édition locale d'abord) |
| Taille d'une mise à jour | < 2 Ko par lot, jamais le document complet |
| Chargement d'un document | < 30 ms |
| Mémoire pour 50 documents ouverts | < 8 Mo |
| Bundle WASM | ≤ 4 Mo gzip (marge pour la CRDT) |

Modes dégradés, tous annoncés :

- transport indisponible ⇒ édition locale seule, bandeau « synchronisation inactive » ;
- document illisible ⇒ reconstruction depuis le snapshot, avec perte des éditions non
  flushées signalée ;
- pair en retard ⇒ rattrapage par vecteur d'état, jamais de blocage de l'interface.

### 21.10 Ce que cette section ne couvre pas

- Le **relais réseau** : `WebSocketTransport` est préparé, pas implémenté.
- L'**authentification et l'autorisation** du plan collaboratif : indispensables dès que
  la synchronisation franchit le navigateur.
- Le **chiffrement** de bout en bout et la conformité RGPD du contenu partagé.
- La **résolution sémantique** des conflits : on signale, on ne décide pas.
- Le partage entre **profils de navigateur différents** ou machines : hors de portée de
  `BroadcastChannel`, donc hors MVP2.

---

## 22. Ce que ce document ne fait pas

Pour éviter toute ambiguïté sur le périmètre :

- Il ne décrit **aucun** comportement hors MVP1 et MVP2 (RBAC, SSO, relais réseau de collaboration, workflow d'approbation, BPMN exécutable, diagrammes UML dynamiques).
- Il ne fixe **aucune** valeur d'identité visuelle au-delà des tokens structurels.
- Il ne garantit **aucune** performance avant mesure effective.
- Il ne remplace pas les spécifications fonctionnelles : en cas de divergence, `functional-specifications.md` fait foi sur le *quoi*, ce document fait foi sur le *comment*.

