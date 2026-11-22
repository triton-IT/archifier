# ADR-025 — L'accueil est le portefeuille de produits, la carte reste une surface

| | |
|---|---|
| Statut | **Accepté** — remplace ADR-024 §3 (l'accueil est la carte) |
| Date | 21 novembre 2026 |
| Complète | ADR-024 (le produit est le pivot de la carte, pas le centre du référentiel) |
| Portée | Accueil, routes, filtres produits, contrats C-06/C-10, `docs/ui-spec.md`, README |
| Décisions liées | ADR-023 (carte et produit), ADR-024 (produit pivot), ADR-012 (routage par fragment) |

## Contexte

ADR-024 a rendu l'accueil « produit » en appliquant la lentille `PRODUCT` à la carte L0.
Mais la carte reste **value-stream-first** : `project_enterprise` construit d'abord des
quartiers à partir des value streams, puis place les produits à l'intérieur. Un utilisateur
qui crée un value stream et un produit voit donc **le value stream avec le produit dedans**,
pas un portefeuille de produits.

La lentille ne change ni le regroupement ni la géographie ; elle change la couleur, les
arêtes et la légende. L'accueil « produit » était donc vrai au niveau des nœuds, faux au
niveau de la lecture. Le besoin exprimé est explicite : **un écran qui présente les produits,
avec des filtres sur leurs relations** (équipe, value stream, capacité, application,
interface, complétude).

## Décision

### 1. `/` est le portefeuille de produits

L'accueil applique `ProductQuery::home()` et rend un **catalogue filtrable**, pas la carte :

- en-tête : nombre total de produits, nombre affiché, filtres actifs ;
- filtres : recherche, équipe, type, cycle de vie, criticité, value stream, capacité,
  application, niveau d'interface, exposition, complétude ;
- cartes produit : nom, type, cycle de vie, criticité, équipe accountable, value stream,
  nombre de capacités/applications/interfaces, signaux et manques ;
- actions : ouvrir la fiche produit, voir le produit sur la carte, créer un produit ;
- état vide : créer un produit, charger l'exemple, ouvrir la carte.

### 2. `/map` est la carte de l'entreprise

La carte reste disponible, avec sa requête par défaut `MapQuery::map_default()` :

- scope et niveau Entreprise ;
- lentille primaire `PRODUCT` ;
- quartiers value streams, plateformes partagées, non rattaché ;
- routes agrégées et dépliables.

La carte n'est plus l'accueil. Un lien profond `#/map?scope=…&level=…&lens=…` reste
prioritaire. Un produit ouvert depuis le catalogue peut pointer vers
`#/map?scope=PRODUCT:<id>&level=PRODUCT`.

### 3. Le produit reste le pivot, le catalogue est sa lecture d'ensemble

Le catalogue ne crée aucun objet et ne duplique aucune relation : il lit le même
référentiel que la carte. Il applique les règles ADR-024 :

- un produit a exactement une équipe accountable (`PRD-001`) ;
- une application est utilisée, pas possédée (`PRD-005`) ;
- une capacité est réalisée par le produit (`PRD-016`) ;
- une interface est un objet versionné (`PRD-024`..`PRD-030`) ;
- un domaine classifie, il ne possède pas (`PRD-031`).

### 4. Sémantique des filtres

Une facette est une liste de valeurs ; **OU à l'intérieur d'une facette**, **ET entre
facettes**.

| Facette | Source | Sémantique |
|---|---|---|
| Recherche | produit, équipe, VS, capacité, application, interface | sous-chaîne insensible à la casse |
| Équipe | `Product.accountableTeamId` | produits dont l'équipe est accountable |
| Type | `Product.kind` | `CUSTOMER`, `INTERNAL`, `PLATFORM`, `DATA`, `PARTNER` |
| Cycle de vie / criticité | `Product` | attributs du produit |
| Value stream | `primaryValueStreamId` + `ValueStreamStage.productIds` | produit rattaché ou contributeur |
| Capacité | `Product.capabilityIds` | produit qui réalise la capacité |
| Application | `Product.applicationIds` | produit qui utilise l'application |
| Niveau d'interface | interfaces exposées ou consommées | `BUSINESS`, `APPLICATION`, `INFRASTRUCTURE` |
| Exposition | interfaces exposées | `INTERNAL`, `PRODUCT`, `PARTNER`, `PUBLIC` |
| Complétude | règles et relations | sans équipe, sans VS, sans capacité, sans application |

Les filtres sont **sérialisés dans l'URL** : `#/?team=…&lifecycle=…&capability=…`. Le lien
est partageable et restaure la lecture.

### 5. Périmètre de la première tranche

Le catalogue expose les facettes réellement alimentées aujourd'hui : équipe, type, cycle
de vie, criticité, value stream, capacité, application, interfaces dérivées, complétude.
Les facettes **domaine métier, portefeuille, étape de value stream, données, risque et
initiative** attendent les commandes d'édition correspondantes ; elles ne sont pas affichées
comme si elles étaient renseignées.

## Conséquences

### Positives

- L'accueil répond à « quels produits avons-nous, et lesquels correspondent à ces
  critères ? » sans passer par la géographie d'un value stream.
- Le value stream reste un cadre de carte et une facette de filtre, pas le sommaire du
  portefeuille.
- La carte reste la lecture spatiale, sans concurrence de rôle avec l'accueil.
- Les liens profonds de filtres sont partageables.

### Négatives, acceptées

- Deux surfaces à maintenir : catalogue et carte.
- Les filtres relationnels coûtent des jointures ; le catalogue charge le référentiel en
  mémoire, ce qui convient à la cible locale actuelle.
- Les facettes non alimentées sont absentes : le catalogue ne promet pas plus que le modèle
  ne permet.

## Invariants et tests nommés

| Invariant | Test | État |
|---|---|---|
| `/` ouvre le catalogue produits, pas la carte | `home_opens_product_catalog` | couvert |
| `/map` ouvre la carte entreprise | `map_opens_enterprise_map` | couvert |
| La carte garde la lentille Produit par défaut | `map_default_lens_is_product` | couvert |
| À L0, les nœuds de carte sont des produits | `map_products_are_the_only_nodes_at_l0` | couvert |
| Le catalogue liste tous les produits | `catalog_lists_all_products` | couvert |
| Filtre équipe | `catalog_filters_by_team` | couvert |
| Filtre value stream (principal ou étape) | `catalog_filters_by_value_stream` | couvert |
| Filtre capacité | `catalog_filters_by_capability` | couvert |
| Filtre application | `catalog_filters_by_application` | couvert |
| OU dans une facette | `catalog_filter_or_within_facet` | couvert |
| ET entre facettes | `catalog_filter_and_across_facets` | couvert |
| Complétude sans value stream | `catalog_gap_without_value_stream` | couvert |
| Compteurs de relations sur la carte produit | `catalog_card_counts_relations` | couvert |
| Lien profond de filtres | `home_deep_link_restores_product_filters` | couvert |

## Ce que cette décision ne dit pas

- Elle ne supprime pas la carte ni ses lentilles.
- Elle ne fait pas du catalogue un inventaire exhaustif : le Contrôle reste l'inventaire.
- Elle n'ajoute pas les facettes domaine/portefeuille/étape/données/risque avant leurs
  commandes d'édition.
- Elle ne change pas la géographie stable de la carte.

## Références

- ADR-023 — carte, produit, zoom.
- ADR-024 — produit pivot, pas centre.
- `docs/contracts/C-10-produit-carte-navigation.md` — objets, carte, routes.
- `docs/contracts/C-06-routes.md` — routes `/` et `/map`.
- `crates/core/src/catalog.rs` — `ProductQuery`, `ProductCatalog`, filtres.
