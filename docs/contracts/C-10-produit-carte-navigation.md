# C-10 — Produit, carte et navigation par zoom

| | |
|---|---|
| **Réf.** | C-10 |
| **Version** | 1.0 — proposition, 18/11/2026 |
| **Statut** | **PROPOSÉ** — adossé à ADR-023 ; ne modifie aucun contrat gelé |
| **Propriétaires** | Produit (ontologie), UX (carte et lentilles), Dev Navigation (routes), Dev Core (projection) |
| **Sources** | ADR-023 ; `functional-specifications.md` § « Cible v0.4 », FR-055..081 ; `docs/ui-spec.md` v2.0 |
| **Dépend de** | C-01 (`ViewCommand`), C-02 (layout), C-03 (`ViewIntent`, conservé), C-06 (routes), C-08 (codes), C-09 (dépôt) |
| **Compatible** | Oui : ajouts only. Aucun champ de C-01..C-09 n'est retiré ni renommé. |

## 1. Objet

Ce contrat fixe :

1. les objets natifs qui portent le produit, l'équipe, le value stream, la capacité, l'application, l'interface et l'infrastructure ;
2. les cardinalités et les règles de propriété ;
3. la projection de carte (`MapQuery` → `MapProjection`) ;
4. les niveaux, lentilles, agrégations et routes ;
5. les codes d'erreur et les tests nommés.

Il ne fixe pas l'algorithme de placement, le rendu SVG, ni le stockage physique. Il fixe ce qui doit être vrai quel que soit le rendu.

## 2. Vocabulaire opposable

| Terme | Type natif | Définition | N'est pas |
|---|---|---|---|
| **Produit** | `Product` | Unité durable de valeur, possédée par une équipe, avec cycle de vie, capacités, applications, interfaces et données. | Une application, un projet, un domaine. |
| **Équipe** | `Team` | Unité durable de responsabilité, avec un type de topologie et une mission. | Un service, un département, un domaine. |
| **Value stream** | `ValueStream` | Flux de bout en bout qui produit un résultat pour un client ou un ayant droit. | Un processus BPMN, un produit. |
| **Étape de value stream** | `ValueStreamStage` | Étape ordonnée du flux, avec la valeur produite. | Une tâche BPMN. |
| **Capacité** | `Capability` | Aptitude stable et hiérarchique de l'entreprise. | Une fonction, un processus, une application. |
| **Portefeuille** | `Portfolio` | Regroupement de produits pour l'investissement ou la navigation. | Un value stream. |
| **Application** | `Application` | Système logiciel déployé, composé de composants et exposant/consommant des interfaces. | Un produit. |
| **Composant applicatif** | `ApplicationComponent` | Partie d'une application (module, service, batch, adaptateur). | Une application. |
| **Interface** | `Interface` | Point d'accès versionné et contractualisé entre un fournisseur et des consommateurs. | Une intégration, une flèche. |
| **Version d'interface** | `InterfaceVersion` | Contrat daté, publiable et consommable. | L'interface elle-même. |
| **Consommation** | `InterfaceConsumption` | Usage d'une version précise par un consommateur identifié. | Un simple lien graphique. |
| **Nœud technique** | `TechnologyNode` | Infrastructure, plateforme, environnement ou réseau. | Une application. |
| **Domaine métier** | `BusinessDomain` | Classification de connaissance ou de compétence. | Un propriétaire. |
| **Initiative** | `Initiative` | Lot de changement daté (migration, programme, vague). | Un produit. |

## 3. Énumérations

```text
ProductKind        = CUSTOMER | INTERNAL | PLATFORM | DATA | PARTNER
LifecycleState     = IDEA | DISCOVERY | BUILD | RUN | SUNSET | RETIRED
Criticality        = CRITICAL | HIGH | MEDIUM | LOW
TeamTopology       = STREAM_ALIGNED | PLATFORM | ENABLING
                   | COMPLICATED_SUBSYSTEM | OTHER
ApplicationKind    = INTERNAL | COTS | SAAS | PLATFORM | LEGACY | EXTERNAL
InterfaceLevel     = BUSINESS | APPLICATION | INFRASTRUCTURE
InterfaceCategory  = BUSINESS | APPLICATION | TECHNICAL   // conservé de l'existant
InterfaceKind      = API | EVENT | FILE | BATCH | UI | DATABASE | NETWORK | OTHER
InterfaceExposure  = INTERNAL | PRODUCT | PARTNER | PUBLIC
MapLevel           = ENTERPRISE | PORTFOLIO | PRODUCT | APPLICATION | INFRASTRUCTURE
MapLens            = VALUE | CAPABILITY | PRODUCT | ORGANIZATION | APPLICATION
                   | INTERFACE | TECHNOLOGY | DATA | TIME | RISK
MapScopeKind       = ENTERPRISE | VALUE_STREAM | PORTFOLIO | PRODUCT | APPLICATION | NODE
MapEdgeKind        = PRODUCT_INTERFACE | APPLICATION_INTERFACE | COMPONENT_INTERFACE
                   | TECHNICAL_INTERFACE | DEPLOYMENT | VALUE_CONTRIBUTION
                   | CAPABILITY_SUPPORT | DATA_FLOW
MapMode            = READ | EDIT
```

Règles :

- `InterfaceLevel` est la lecture de navigation ; `InterfaceCategory` reste le binding de métamodèle. `BUSINESS` correspond à `BUSINESS`, `APPLICATION` à `APPLICATION`, `INFRASTRUCTURE` à `TECHNICAL`.
- `ProductKind.PLATFORM` désigne un produit dont les clients sont internes ; il n'est pas synonyme d'`ApplicationKind.PLATFORM`.
- `LifecycleState` s'applique aux produits et aux applications ; une interface a ses propres statuts éditoriaux et temporels (C-02, BR-001..012).
- `Capability` et `ValueStream` sont des objets **natifs** tant que le profil `archimate:3.2` ne contient pas la couche Strategy (écart E-04). Leur binding ArchiMate officiel est prévu, sans invention de classe.

## 4. Objets et cardinalités

### 4.1 `Product`

```text
Product
  id
  name
  kind: ProductKind
  lifecycle: LifecycleState
  criticality: Criticality
  parentProductId?: ProductId          // profondeur max 3
  portfolioIds: PortfolioId[]
  accountableTeamId: TeamId            // exactement 1
  contributingTeamIds: TeamId[]
  primaryValueStreamId?: ValueStreamId // quartier d'accueil à L0
  primaryPortfolioId?: PortfolioId     // repli si pas de value stream
  businessDomainIds: BusinessDomainId[]
  valuePropositionMarkdown?
  userIds: ObjectId[]                  // personas, acteurs, segments
  outcomeIds: ObjectId[]               // objectifs, OKR, résultats
  capabilityIds: CapabilityId[]        // réalisations
  applicationIds: ApplicationId[]      // usages
  exposedInterfaceIds: InterfaceId[]   // interfaces produit
  dataObjectIds: ObjectId[]
  riskIds: ObjectId[]
  initiativeIds: InitiativeId[]
  revision
```

Règles :

| ID | Règle |
|---|---|
| `PRD-001` | Un produit a exactement une équipe accountable. |
| `PRD-002` | Un produit contribue à au moins une étape de value stream, ou porte l'anomalie `PRODUCT_WITHOUT_VALUE_STAGE`. |
| `PRD-003` | Un produit réalise au moins une capacité, ou porte l'anomalie `PRODUCT_WITHOUT_CAPABILITY`. |
| `PRD-004` | Un produit peut être composé de sous-produits, sur trois niveaux maximum. |
| `PRD-005` | Un produit **utilise** des applications ; il ne les possède pas. La relation est `PRODUCT_USES_APPLICATION`. |
| `PRD-006` | Une application utilisée par plus d'un produit est marquée `shared=true` et exposée comme couplage. |
| `PRD-007` | Un produit plateforme expose au moins une interface à d'autres produits, ou porte l'anomalie `PLATFORM_WITHOUT_INTERFACE`. |
| `PRD-033` | Un produit a un **value stream principal** (ou un portefeuille de repli) qui détermine son quartier d'accueil à L0. Il n'apparaît qu'une fois sur la carte ; ses contributions aux autres value streams sont des liens de contexte, jamais des duplications. Sans value stream principal, il est placé dans « Non rattaché » et signalé. |

### 4.2 `Team`

```text
Team
  id
  name
  topology: TeamTopology
  missionMarkdown?
  orgUnitId?: ObjectId
  ownedProductIds: ProductId[]
  operatedApplicationIds: ApplicationId[]
  providedInterfaceIds: InterfaceId[]
  consumedInterfaceIds: InterfaceId[]
  interactionModes: TeamInteractionMode[]
  revision
```

`TeamInteractionMode = COLLABORATION | X_AS_A_SERVICE | FACILITATION`.

Règles :

| ID | Règle |
|---|---|
| `PRD-008` | Une équipe peut posséder plusieurs produits ; un produit n'a qu'une équipe accountable. |
| `PRD-009` | Une application a exactement une équipe qui l'opère. Une application partagée reste opérée par une seule équipe. |
| `PRD-010` | Une équipe peut contribuer à un produit sans le posséder ; la contribution est explicite. |
| `PRD-011` | Une équipe de type `ENABLING` ne possède pas d'application en propre ; si c'est le cas, l'anomalie `ENABLING_TEAM_OWNS_APPLICATION` est levée. |

### 4.3 `ValueStream` et `ValueStreamStage`

```text
ValueStream
  id
  name
  customerObjectId?
  valuePropositionMarkdown?
  stageIds: ValueStreamStageId[]       // ordonnées
  ownerTeamId?: TeamId
  revision

ValueStreamStage
  id
  valueStreamId
  name
  order: int
  valueItemMarkdown?
  capabilityIds: CapabilityId[]
  productIds: ProductId[]
  processIds: ObjectId[]               // BPMN, réalisation ou description
  revision
```

Règles :

| ID | Règle |
|---|---|
| `PRD-012` | Une étape appartient à un seul value stream. |
| `PRD-013` | Une étape est réalisée par un processus BPMN ou descriptive ; le lien est `REALIZES` ou `DESCRIBES`. |
| `PRD-014` | Un value stream n'est pas un processus : il agrège des chemins et ne porte pas de gateway. |
| `PRD-015` | Un produit peut contribuer à plusieurs étapes ; sa contribution est datée si elle est planifiée. |

### 4.4 `Capability`

```text
Capability
  id
  name
  parentId?: CapabilityId
  level: int
  maturity?: 1..5
  criticality: Criticality
  productIds: ProductId[]              // réalisée par
  applicationIds: ApplicationId[]      // supportée par
  stageIds: ValueStreamStageId[]
  revision
```

Règles :

| ID | Règle |
|---|---|
| `PRD-016` | Une capacité est réalisée par des produits, supportée par des applications ; elle n'est possédée par aucune application. |
| `PRD-017` | Une capacité peut être déclinée, mais la profondeur utile s'arrête là où la décision de portefeuille ne change plus. |
| `PRD-018` | Une capacité sans produit réalisateur porte l'anomalie `CAPABILITY_WITHOUT_REALIZATION`. |

### 4.5 `Application` et `ApplicationComponent`

```text
Application
  id
  name
  kind: ApplicationKind
  lifecycle: LifecycleState
  criticality: Criticality
  operatingTeamId: TeamId              // exactement 1
  usedByProductIds: ProductId[]
  componentIds: ApplicationComponentId[]
  exposedInterfaceIds: InterfaceId[]
  consumedInterfaceIds: InterfaceId[]
  dataObjectIds: ObjectId[]
  nodeIds: TechnologyNodeId[]
  revision

ApplicationComponent
  id
  applicationId
  name
  kind: COMPONENT | SERVICE | BATCH | ADAPTER | DATABASE | UI
  serviceIds: ObjectId[]
  interfaceIds: InterfaceId[]
  dataObjectIds: ObjectId[]
  revision
```

Règles :

| ID | Règle |
|---|---|
| `PRD-019` | Le cycle de vie d'une application est distinct de celui du produit qui l'utilise ; l'un ne se déduit pas de l'autre. |
| `PRD-020` | Une application peut servir plusieurs produits ; le produit ne la possède pas. |
| `PRD-021` | Une application est composée de composants ou est une feuille assumée (COTS/SaaS) ; dans ce cas `componentIds` peut être vide. |
| `PRD-022` | Une application est déployée sur au moins un nœud, ou porte l'anomalie `APPLICATION_WITHOUT_DEPLOYMENT`. |
| `PRD-023` | Un composant n'est pas déployable seul ; c'est l'application qui l'est. |

### 4.6 `Interface`, `InterfaceVersion`, `InterfaceConsumption`

```text
Interface
  id
  name
  level: InterfaceLevel
  category: InterfaceCategory
  kind: InterfaceKind
  providerObjectId: ObjectId            // Product | Application | TechnologyNode | ApplicationComponent
  providerProductId?: ProductId
  exposure: InterfaceExposure
  criticality: Criticality
  dataClassification?: PUBLIC | INTERNAL | CONFIDENTIAL | SECRET
  versionIds: InterfaceVersionId[]
  consumerIds: InterfaceConsumptionId[]
  notationBindings: NotationBinding[]
  revision

InterfaceVersion
  id
  interfaceId
  semanticVersion
  publicationStatus: DRAFT | PUBLISHED | WITHDRAWN
  availableFrom
  availableUntil
  breakingChange
  supersedesVersionId?
  contractType?: OPENAPI | ASYNCAPI | WSDL | FILE | OTHER
  contractSnapshot?
  revision

InterfaceConsumption
  id
  interfaceVersionId
  consumerObjectId: ObjectId            // Product | Application | TechnologyNode | ApplicationComponent
  criticality: Criticality
  since?
  until?
  revision
```

Règles :

| ID | Règle |
|---|---|
| `PRD-024` | Une interface a exactement un fournisseur (`providerObjectId`). |
| `PRD-025` | Une interface a au moins une version pour être publiée. |
| `PRD-026` | Une consommation référence une **version** d'interface, jamais l'interface nue. |
| `PRD-027` | Une interface `PRODUCT` est exposée par un produit ; une interface `INTERNAL` peut rester interne à une application. |
| `PRD-028` | Une interface applicative peut être exposée au niveau produit sans être dupliquée : `exposure=PRODUCT` et `providerProductId` la rendent visible sur la page produit. |
| `PRD-029` | Une interface d'infrastructure a un fournisseur `TechnologyNode` et ne peut pas être exposée `PARTNER` ou `PUBLIC`. |
| `PRD-030` | Une rupture de contrat crée une nouvelle version ; une consommation existante reste attachée à l'ancienne version jusqu'à migration explicite. |

### 4.7 `TechnologyNode`, `BusinessDomain`, `Portfolio`, `Initiative`

```text
TechnologyNode
  id
  name
  kind: PLATFORM | SERVER | CLUSTER | NETWORK | DATABASE | ENVIRONMENT | OTHER
  environment?: DEV | TEST | STAGING | PROD | DR
  providerTeamId?: TeamId
  parentNodeId?: TechnologyNodeId
  applicationIds: ApplicationId[]
  interfaceIds: InterfaceId[]
  lifecycle: LifecycleState
  revision

BusinessDomain
  id
  name
  description?
  parentId?: BusinessDomainId
  classifiedObjectIds: ObjectId[]       // lecture ; la relation est portée par l'objet
  revision

Portfolio
  id
  name
  kind: VALUE_STREAM | BUSINESS_UNIT | THEME | PLATFORM_GROUP | PRODUCT_LINE
  productIds: ProductId[]
  revision

Initiative
  id
  name
  kind: MIGRATION | PROGRAM | WAVE | PROJECT
  start
  end?
  productIds: ProductId[]
  applicationIds: ApplicationId[]
  interfaceIds: InterfaceId[]
  revision
```

## 5. Relations natives

| Relation | Source → cible | Cardinalité | Sens |
|---|---|---|---|
| `TEAM_OWNS_PRODUCT` | Team → Product | 1..n | Responsabilité |
| `TEAM_CONTRIBUTES_TO_PRODUCT` | Team → Product | 0..n | Contribution |
| `TEAM_OPERATES_APPLICATION` | Team → Application | 1..n | Exploitation |
| `PRODUCT_USES_APPLICATION` | Product → Application | 1..n | Usage |
| `PRODUCT_REALIZES_CAPABILITY` | Product → Capability | 1..n | Réalisation |
| `PRODUCT_CONTRIBUTES_TO_STAGE` | Product → ValueStreamStage | 1..n | Valeur |
| `VALUE_STREAM_HAS_STAGE` | ValueStream → Stage | 2..n | Composition |
| `CAPABILITY_ENABLES_STAGE` | Capability → Stage | 0..n | Enablement |
| `APPLICATION_SUPPORTS_CAPABILITY` | Application → Capability | 0..n | Support |
| `APPLICATION_COMPOSED_OF_COMPONENT` | Application → Component | 0..n | Composition |
| `COMPONENT_EXPOSES_INTERFACE` | Component → Interface | 0..n | Exposition |
| `INTERFACE_PROVIDED_BY` | Interface → Provider | 1 | Fourniture |
| `INTERFACE_CONSUMED_BY` | Consumption → Consumer | 1 | Consommation |
| `INTERFACE_HAS_VERSION` | Interface → Version | 1..n | Versionnage |
| `CONSUMPTION_OF_VERSION` | Consumption → Version | 1 | Contrat |
| `APPLICATION_DEPLOYED_ON` | Application → TechnologyNode | 1..n | Déploiement |
| `BUSINESS_DOMAIN_CLASSIFIES` | BusinessDomain → Object | 0..n | Classification |
| `PRODUCT_DEPENDS_ON_PRODUCT` | Product → Product | 0..n | **Dérivée** des consommations d'interface ; non saisie par défaut |
| `PLATFORM_PROVIDES_TO_PRODUCT` | Product → Product | 0..n | **Dérivée** des interfaces exposées par un produit plateforme |
| `PROCESS_REALIZES_STAGE` | BPMN Process → Stage | 0..n | Traçabilité BPMN |
| `UML_IMPLEMENTS_COMPONENT` | UML Component → ApplicationComponent | 0..n | Traçabilité UML |
| `UML_INTERFACE_REALIZES_INTERFACE` | UML Interface → Interface | 0..n | Traçabilité UML |

Règles :

- Les relations `PRODUCT_DEPENDS_ON_PRODUCT` et `PLATFORM_PROVIDES_TO_PRODUCT` sont **calculées** à partir des interfaces et des consommations. Elles ne sont pas persistées comme des arêtes manuelles, sauf décision explicite `strategic=true` (affichée différemment).
- Une relation native ne remplace jamais une relation ArchiMate. Elle la complète au niveau de la couche native.
- Le `Product` natif et l'élément ArchiMate `Product` sont reliés par `REALIZES` ou `REFINES` ; ils ne fusionnent jamais. Le premier porte la propriété, le cycle de vie et les applications ; le second porte la composition de services et le contrat.
- Les relations inter-notations existantes (`REFINES`, `IMPLEMENTS`, `REALIZES`, `DESCRIBES`, `EXPOSES`, `CONSUMES`, `TRACE_TO`) restent valides.

## 6. Projection de carte

### 6.1 Requête

```text
MapQuery
  scope: MapScope
  level: MapLevel
  lenses: MapLens[]                    // au moins 1
  date?: RFC3339
  focus?: ObjectId[]
  filters: MapFilter
  mode: MapMode

MapScope =
    { kind: ENTERPRISE }
  | { kind: VALUE_STREAM, id }
  | { kind: PORTFOLIO, id }
  | { kind: PRODUCT, id }
  | { kind: APPLICATION, id }
  | { kind: NODE, id }

MapFilter
  productKinds: ProductKind[]
  lifecycles: LifecycleState[]
  criticalities: Criticality[]
  teamIds: TeamId[]
  interfaceLevels: InterfaceLevel[]
  interfaceKinds: InterfaceKind[]
  exposures: InterfaceExposure[]
  includeExternal: bool = true
  includeExpired: bool = false
```

Une carte sauvegardée (lien, bloc de Story, capture de Revue) sérialise `MapQuery` + `sourceRevision` + `caption`. Elle ne crée **pas** un `Diagram` et ne duplique pas la géométrie : les ancres restent la seule source de position.

### 6.2 Résultat

```text
MapProjection
  query: MapQuery
  breadcrumb: BreadcrumbEntry[]
  districts: MapDistrict[]
  nodes: MapNode[]
  edges: MapEdge[]
  legend: Legend
  counts: MapCounts
  anomalies: AnomalyRef[]
  sourceRevision
  generatedAt

MapDistrict
  id
  kind: VALUE_STREAM | PORTFOLIO | BUSINESS_UNIT | PRODUCT_GROUP | PLATFORM_GROUP
  objectId
  label
  position
  size
  colorKey?
  nodeIds[]

MapNode
  id
  objectId
  objectKind: PRODUCT | APPLICATION | COMPONENT | NODE | CAPABILITY | TEAM | STAGE
  label
  sublabel?
  status?: LifecycleState | TemporalStatus
  criticality?
  parentId?
  districtId?
  position
  size
  aggregateCount?
  expandable: bool
  badges: Badge[]

MapEdge
  id
  kind: MapEdgeKind
  sourceNodeId
  targetNodeId
  label?
  interfaceIds: InterfaceId[]
  interfaceVersionIds: InterfaceVersionId[]
  count: int
  criticality?
  temporalStatus?
  aggregated: bool
  expandable: bool
  strategic: bool = false
```

### 6.3 Règles de projection

| ID | Règle |
|---|---|
| `MAP-001` | La carte est la route `/`. Elle ne montre ni table de données ni galerie d'intentions comme contenu principal. |
| `MAP-002` | Cinq niveaux existent : `ENTERPRISE`, `PORTFOLIO`, `PRODUCT`, `APPLICATION`, `INFRASTRUCTURE`. |
| `MAP-003` | Un niveau ne montre jamais individuellement un objet d'un niveau inférieur : il l'agrège et indique un `aggregateCount`. |
| `MAP-004` | La géométrie est stable : changer de lentille, de filtre ou de date ne déplace aucun nœud. |
| `MAP-005` | Une arête de niveau N est l'agrégat d'arêtes de niveau N+1 et doit être `expandable=true`. |
| `MAP-006` | Le zoom est sémantique : le franchissement d'un seuil change la requête, pas seulement l'échelle. |
| `MAP-007` | Le contexte externe reste visible, atténué, pour montrer les dépendances sortantes. |
| `MAP-008` | Le curseur temporel applique `SCHEDULED`/`ACTIVE`/`EXPIRED` sans modifier la géométrie ni les données. |
| `MAP-009` | Toute carte a un lien profond reproductible : `scope`, `level`, `lens`, `date`, `focus`, `filter`. |
| `MAP-010` | Une création depuis la carte est explicite et confirmée ; aucun objet n'est créé implicitement par un déplacement. |
| `MAP-011` | Au-delà de 500 nœuds visibles, la projection agrège ou demande un filtre (`MAP-003`). |
| `MAP-012` | Une projection est déterministe : même requête + même révision + mêmes ancres ⇒ même carte. |
| `MAP-013` | Une lentille indisponible à un niveau est refusée avec `MAP-004`, sans changer le niveau. |
| `MAP-014` | Un périmètre inconnu retombe sur `ENTERPRISE` et signale `MAP-001`, sans perdre le lien. |

### 6.4 Contenu obligatoire par niveau

| Niveau | Doit montrer | Peut montrer | Ne doit pas montrer |
|---|---|---|---|
| **ENTERPRISE** | value streams/portefeuilles, produits, produits plateformes, flux produit agrégés | criticité, cycle de vie, propriétaire | applications individuelles, composants, nœuds |
| **PORTFOLIO** | produits du périmètre, plateformes partagées, interfaces produit à produit | capacités, équipes, criticité | composants, nœuds |
| **PRODUCT** | frontière du produit, ports, applications, interfaces applicatives | produits externes, dépendances, données | composants internes, nœuds individuels |
| **APPLICATION** | composants, services, interfaces, données, déploiement | nœuds agrégés | produits, value streams |
| **INFRASTRUCTURE** | nœuds, environnements, réseaux, applications déployées, interfaces techniques | criticité, obsolescence | composants métier |

## 7. Lentilles

| Lentille | Question | Encodage principal | Arêtes | Niveaux |
|---|---|---|---|---|
| `VALUE` | Quel parcours, quelles étapes, quels produits ? | quartiers = value streams, étapes = bandes | contribution produit → étape | L0, L1, L2 |
| `CAPABILITY` | Quelles capacités, couvertes par quoi ? | heatmap de couverture/maturité | réalisation, support | L0, L1, L2 |
| `PRODUCT` | Quel portefeuille, quel cycle de vie, quelle criticité ? | couleur = lifecycle/criticality | dépendances produit | L0, L1, L2 |
| `ORGANIZATION` | Qui possède quoi, où sont les dépendances ? | couleur = équipe, frontières d'équipe | interfaces inter-équipes | L0, L1, L2 |
| `APPLICATION` | Quelles applications, quels composants, quel âge ? | couleur = type/âge | interfaces applicatives | L1, L2, L3 |
| `INTERFACE` | Qui expose/consomme quoi, quelles versions ? | ports + routes, filtre catégorie/état | consommations | L0, L1, L2, L3, L4 |
| `TECHNOLOGY` | Quelles plateformes, quels nœuds, quelle obsolescence ? | couleur = type/âge/environnement | déploiement, réseau | L2, L3, L4 |
| `DATA` | Quels objets, quels flux, quelle classification ? | couleur = classification | échanges de données | L1, L2, L3 |
| `TIME` | Où en est-on à T, que se passe-t-il ? | motif = scheduled/active/expired | arêtes datées | Tous |
| `RISK` | Où sont les risques, coûts, dettes ? | heatmap | propagation de risque | L0, L1, L2, L3 |

Règles :

| ID | Règle |
|---|---|
| `MAP-015` | Au moins une lentille est active ; la lentille par défaut est `PRODUCT` à L0/L1 et `INTERFACE` à L2/L3. |
| `MAP-016` | Les lentilles se superposent : une couleur, un style d'arête et un filtre peuvent être actifs ensemble. |
| `MAP-017` | Une lentille ne modifie ni `position`, ni `size`, ni la hiérarchie. |
| `MAP-018` | Une lentille indisponible au niveau courant est grisée avec sa raison, pas silencieusement retirée. |
| `MAP-019` | La légende est toujours accessible et nomme la lentille, l'encodage et les états. |

### 7.1 Correspondance avec les sept intentions de l'Atelier

| `ViewIntent` (C-03) | Devient sur la carte |
|---|---|
| `CONTEXT` | Lentille `VALUE` + panneau produit (valeur, utilisateurs, objectifs). |
| `APPLICATION_LANDSCAPE` | Lentille `APPLICATION` au niveau produit ou application. |
| `PROCESS` | Action « Ouvrir le processus BPMN » depuis une étape de value stream. |
| `DEPLOYMENT` | Lentille `TECHNOLOGY` au niveau application/infrastructure. |
| `CAPABILITY` | Lentille `CAPABILITY`. |
| `MIGRATION` | Lentille `TIME` + initiatives. |
| `INTERFACE_REVIEW` | Lentille `INTERFACE` + revue. |

`ViewIntent` reste le catalogue des vues authored ; `MapLens` est le catalogue de la carte. Aucune valeur n'est ajoutée à `ViewIntent` sans révision de C-03.

## 8. Agrégation des arêtes

### 8.1 Règle générale

Une arête de carte représente un ensemble de `InterfaceConsumption`. Elle est construite ainsi :

1. regrouper les consommations par couple `(providerNode, consumerNode)` au niveau courant ;
2. agréger les versions par interface ;
3. calculer `count` = nombre d'interfaces distinctes ;
4. calculer `criticality` = maximum des criticités ;
5. calculer `temporalStatus` = état le plus contraignant (`EXPIRED` > `SCHEDULED` > `ACTIVE`) ;
6. conserver `interfaceIds` et `interfaceVersionIds` pour le dépliage ;
7. marquer `aggregated=true` si `count > 1` ou si les extrémités sont des agrégats.

### 8.2 Règles par niveau

| Niveau | Extrémités | Agrégation |
|---|---|---|
| L0 | Produit → Produit | Toutes les interfaces exposées par les applications du produit A et consommées par les applications du produit B. |
| L1 | Produit → Produit / Plateforme → Produit | Idem, restreint au périmètre ; les plateformes sont des nœuds `UTILITY`. |
| L2 | Application → Application | Interfaces applicatives réelles ; les ports du produit sont les interfaces `exposure=PRODUCT`. |
| L3 | Composant → Composant | Interfaces de composant ; les arêtes vers l'extérieur remontent à l'application. |
| L4 | Nœud → Nœud | Interfaces techniques et déploiements ; les applications sont des badges, pas des nœuds. |

### 8.3 Dépliage

- Cliquer une arête agrégée ouvre un panneau latéral listant les interfaces, versions, fournisseurs, consommateurs et états.
- « Déplier » descend d'un niveau ou isole le couple fournisseur/consommateur.
- Une arête non dépliable est un défaut (`MAP-005`).
- Une arête stratégique saisie manuellement est marquée `strategic=true` et visuellement distincte ; elle ne remplace jamais une arête dérivée.

## 9. Routes et liens profonds

```text
/                                      Carte de l'entreprise (L0)
/map?scope=…&level=…&lens=…&date=…     Carte, contexte explicite
/map/enterprise                        Alias de /map?scope=ENTERPRISE&level=ENTERPRISE
/products/:id                          Page produit (L2)
/products/:id/applications/:appId      Application dans le contexte du produit
/teams/:id                             Page équipe
/value-streams/:id                     Page value stream
/value-streams/:id/stages/:stageId     Étape, avec processus BPMN éventuel
/capabilities/:id                      Page capacité
/applications/:id                      Page application (L3)
/applications/:id/components/:compId   Composant
/infrastructure/:id                    Page nœud (L4)
/interfaces/:id                        Page interface
/business-domains/:id                  Page domaine métier (classification)
/portfolios/:id                        Page portefeuille
/library/products                      Bibliothèque, onglet Produits
/library/teams                         Bibliothèque, onglet Équipes
/library/value-streams                 Bibliothèque, onglet Value streams
/library/capabilities                  Bibliothèque, onglet Capacités
```

Paramètres de lien :

| Paramètre | Type | Défaut | Règle |
|---|---|---|---|
| `scope` | `ENTERPRISE` ou `KIND:id` | `ENTERPRISE` | Inconnu → `ENTERPRISE` + `MAP-001`. |
| `level` | `MapLevel` | déduit du scope | Incompatible → niveau le plus proche + `MAP-002`. |
| `lens` | `MapLens` répétable | `PRODUCT` ou `INTERFACE` | Au moins une ; inconnue ignorée avec avertissement. |
| `date` | RFC 3339 | absent | Date illisible → `MAP-006`, carte sans curseur. |
| `focus` | IDs séparés par `,` | vide | IDs inconnus ignorés individuellement. |
| `filter` | `clé=valeur` répétable | vide | Filtre inconnu ignoré avec avertissement. |
| `mode` | `read`/`edit` | `read` | `edit` exige la capacité d'édition. |
| `caption` | texte | vide | Légende de lien partagé. |

Règles :

1. Un lien partagé ouvre toujours en `read`, même pour l'auteur.
2. Le lien ne transporte ni sélection d'édition, ni secret, ni état de formulaire.
3. Le lien est reproductible : mêmes paramètres + même révision ⇒ même projection.
4. Les anciennes routes `/library/domains/:id` et `/view/domains/:id` restent des alias de `/business-domains/:id`.
5. Les routes `/design/:id` et `/view/diagrams/:id` restent celles de l'Atelier ; la carte ne les remplace pas.

### 9.1 URIs canoniques

| Objet | URI | Route résolue |
|---|---|---|
| Produit | `arch://product/{id}` | `/products/{id}` |
| Équipe | `arch://team/{id}` | `/teams/{id}` |
| Value stream | `arch://value-stream/{id}` | `/value-streams/{id}` |
| Étape | `arch://stage/{id}` | `/value-streams/{valueStreamId}/stages/{id}` |
| Capacité | `arch://capability/{id}` | `/capabilities/{id}` |
| Application | `arch://application/{id}` | `/applications/{id}` |
| Nœud technique | `arch://node/{id}` | `/infrastructure/{id}` |
| Domaine métier | `arch://business-domain/{id}` | `/business-domains/{id}` |
| Interface | `arch://interface/{id}` | `/interfaces/{id}` |
| Version | `arch://version/{id}` | `/interfaces/{interfaceId}` (version ciblée) |

Les anciennes URIs `arch://domain/{id}`, `arch://element/{id}`, `arch://relationship/{id}`, `arch://diagram/{id}`, `arch://story/{id}` restent résolvables. Une carte n'a pas d'URI `arch://` : elle est identifiée par une `MapQuery` sérialisée dans une route `#/map`.

## 10. Ancres de position

```text
MapAnchor
  objectId
  level: MapLevel
  districtId?
  x
  y
  pinned: bool
  updatedAt
  revision
```

Règles :

| ID | Règle |
|---|---|
| `MAP-020` | Une ancre est persistée par objet et par niveau, pas dans le layout d'une vue authored. |
| `MAP-021` | Un objet sans ancre reçoit une position déterministe par l'heuristique de placement. |
| `MAP-022` | Déplacer un objet sur la carte crée ou met à jour une ancre `pinned=true`. |
| `MAP-023` | Changer de lentille ne modifie jamais une ancre. |
| `MAP-024` | Un conflit d'ancre (deux objets à la même position) est résolu visuellement par un décalage, sans écrire de données. |

## 11. Codes d'erreur

| Code | Sens | Comportement |
|---|---|---|
| `PRD-001` | Produit sans équipe accountable | Bloque la publication du produit ; signalé au Contrôle. |
| `PRD-002` | Produit sans étape de value stream | Anomalie non bloquante. |
| `PRD-003` | Produit sans capacité | Anomalie non bloquante. |
| `PRD-009` | Application sans équipe opératrice | Bloque la publication de l'application. |
| `PRD-018` | Capacité sans réalisation | Anomalie non bloquante. |
| `PRD-022` | Application sans déploiement | Anomalie non bloquante. |
| `PRD-007` | Produit plateforme sans interface | Anomalie non bloquante. |
| `PRD-011` | Équipe enabling propriétaire d'application | Anomalie non bloquante. |
| `PRD-024` | Interface sans fournisseur | Bloque la publication. |
| `PRD-025` | Interface sans version | Bloque la publication. |
| `PRD-026` | Consommation sans version | Refus de la consommation. |
| `PRD-031` | Domaine utilisé comme propriétaire | Anomalie de migration ; l'équipe reste requise. |
| `PRD-032` | Produit sans application | Anomalie non bloquante (produit en découverte). |
| `PRD-033` | Produit sans value stream principal | Anomalie non bloquante ; placé dans « Non rattaché ». |
| `MAP-001` | Périmètre inconnu | Retour à `ENTERPRISE`. |
| `MAP-002` | Niveau incompatible avec le périmètre | Niveau le plus proche. |
| `MAP-003` | Trop de nœuds visibles | Agrégation ou filtre demandé. |
| `MAP-004` | Lentille indisponible au niveau | Lentille grisée avec raison. |
| `MAP-005` | Arête agrégée non dépliable | Défaut de projection. |
| `MAP-006` | Date invalide | Carte sans curseur. |
| `MAP-007` | Conflit d'ancre | Décalage visuel, pas d'écriture. |

## 12. Tests nommés

| Test | Ce qu'il prouve |
|---|---|
| `home_opens_enterprise_map` | `/` ouvre la carte, pas l'Atelier ni une galerie d'intentions. |
| `map_zooms_enterprise_to_infrastructure` | Le parcours L0→L4 est possible et conserve le contexte. |
| `map_keeps_geometry_across_lenses` | Changer de lentille ne déplace aucun nœud. |
| `map_aggregate_edge_expands_to_interfaces` | Une route agrégée se déplie vers des interfaces réelles. |
| `map_does_not_show_components_at_enterprise_level` | L0 ne montre pas les composants. |
| `map_deep_link_restores_context` | `scope`, `level`, `lens`, `date` sont restaurés. |
| `map_temporal_cursor_does_not_mutate_layout` | Le temps ne change pas la géométrie. |
| `product_requires_accountable_team` | `PRD-001` est appliqué. |
| `application_is_used_by_products_not_owned` | Un produit ne possède pas une application. |
| `shared_application_is_flagged` | Une application multi-produits est signalée. |
| `interface_consumption_requires_version` | `PRD-026` est appliqué. |
| `domain_is_classification_not_owner` | Un domaine ne peut pas remplacer une équipe. |
| `legacy_domain_routes_still_resolve` | Les alias de domaine restent valides. |
| `bpmn_process_opens_from_value_stream_stage` | La traçabilité BPMN est accessible. |
| `uml_component_opens_from_application` | La traçabilité UML est accessible. |
| `first_run_creates_product_not_domain` | L'onboarding crée un produit, pas un domaine. |
| `map_edit_does_not_open_modal` | La création depuis la carte reste non modale. |

## 13. Ce que ce contrat ne couvre pas

- L'algorithme de placement et de routage des arêtes.
- Le rendu SVG/Canvas et la virtualisation (ADR-020).
- Le stockage physique (IndexedDB, serveur) et la migration de données.
- La collaboration temps réel (ADR-014).
- La conformité BPMN/UML complète.
- Les politiques RBAC/ABAC.
- La taxonomie produit au-delà de trois niveaux.
