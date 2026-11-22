# ADR-024 — Le produit est le pivot de la carte, pas le centre du référentiel

| | |
|---|---|
| Statut | **Accepté** — précise ADR-023 ; **§3 remplacé par ADR-025** (l'accueil est le portefeuille de produits) |
| Date | 21 novembre 2026 |
| Complète | ADR-023 §1 (le produit est l'unité de lecture et de responsabilité) |
| Portée | Accueil, projection L0, lentilles, ontologie native, contrats C-10 et docs/ui-spec |
| Décisions liées | ADR-023 (carte et produit), ADR-025 (catalogue produits), ADR-020 (virtualisation), ADR-021 (ArchiMate) |

## Contexte

ADR-023 a fait du **produit** l'unité de lecture et de responsabilité. C'était la bonne
correction face au domaine, qui mélangeait classification, propriété et navigation. Mais
la décision laisse ouverte une ambiguïté : le produit est-il seulement le **point
d'entrée**, ou devient-il la **clé primaire** de tout le modèle ?

Le glissement est déjà visible dans les discussions produit : « quel produit porte cette
capacité ? », « quel produit contient cette donnée ? », « quel produit est cette
société ? ». À force, tout objet doit être rattaché à un produit, et l'on invente des
produits pour ranger ce qui n'en est pas : une application legacy, une base partagée, une
capacité réglementaire, une unité juridique. C'est le **product-washing** : le mot perd son
sens, la carte se remplit de faux produits et les dimensions qui ne sont pas des produits
disparaissent.

Le produit est un excellent **pivot** parce qu'il combine identité durable, propriétaire,
cycle de vie, valeur et interfaces. Ce n'est pas pour autant un bon **centre ontologique** :
il n'est ni la société, ni la capacité, ni le processus, ni le nœud technique.

## Décision

### 1. Le produit est le pivot de lecture et de responsabilité

- Le produit est le **point d'entrée par défaut** de la carte et l'unité d'imputabilité.
- Une équipe possède un produit ; une application est **utilisée** par des produits, elle
  n'est pas possédée par eux.
- Le produit porte la valeur, le cycle de vie, la criticité, les interfaces exposées et les
  applications utilisées.

### 2. Le produit n'est pas la clé primaire du référentiel

Le référentiel reste **multi-centré** :

- les **relations** sont des objets de premier rang (`réalise`, `contribue à`, `utilise`,
  `expose`, `consomme`, `déployé sur`) ; l'architecture est dans les arêtes, pas dans un
  nœud unique ;
- un objet peut exister **sans produit** : une capacité non réalisée, une application sans
  produit consommateur, une interface inter-sociétés, une donnée de référence, un nœud
  partagé. Ces objets portent une **anomalie explicite**, jamais un produit artificiel ;
- le rattachement à un produit n'est **jamais un containment obligatoire**. `PRD-001`
  impose une équipe accountable à un produit ; il n'impose pas qu'un produit soit le
  conteneur de tout le reste.

### 3. L'accueil s'ouvre sur les produits

> **Avenant ADR-025 (21/11/2026).** Le §3 d'origine (l'accueil = la carte) est **remplacé**.
> L'accueil est le **catalogue produits** (`ProductQuery::home()`) ; la carte est servie par
> `/map` (`MapQuery::map_default()`). Voir `ADR-025-accueil-portefeuille-produits.md`.

Conséquences opposables :

- `/` liste les produits avec leurs filtres relationnels ; le value stream est une facette,
  pas le cadre de l'accueil ;
- `/map` applique `scope=ENTERPRISE`, `level=ENTERPRISE`, lentille primaire `PRODUCT` ; à
  L0, les seuls nœuds de carte sont des produits ; les quartiers sont le contexte ;
- le premier objet proposé à la création est un **produit** ; l'équipe accountable est une
  **condition** du produit, pas l'entrée de l'accueil ;
- un lien profond qui change `scope`, `level` ou `lens` reste prioritaire sur la carte ;
  l'accueil n'est qu'un défaut, jamais une contrainte de navigation.

### 4. Le premier plan dépend du niveau

Le centre de lecture change avec le zoom sémantique. Imposer le produit partout aplatit
la carte.

| Niveau | Objet au premier plan | Question |
|---|---|---|
| **L0 — Entreprise** | Produits (quartiers = value streams, portefeuilles ; unités d'organisation en cible) | Quelle valeur, pour qui, sous quelle responsabilité ? |
| **L1 — Portefeuille** | Produits, plateformes | Quel portefeuille, quels trous, quelles dépendances ? |
| **L2 — Produit** | Applications, interfaces | De quoi le produit dépend-il, qu'expose-t-il ? |
| **L3 — Application** | Composants, services, données | Comment est-ce construit ? |
| **L4 — Infrastructure** | Nœuds, environnements, réseaux | Où ça tourne, quelle obsolescence ? |

Les lentilles `VALUE`, `CAPABILITY` et `ORGANIZATION` **doivent pouvoir** promouvoir un
autre objet au premier plan **sans changer la géographie ni le graphe**. Aujourd'hui elles
changent surtout l'encodage (couleur, arêtes, légende) ; la promotion complète d'un autre
objet est une cible, pas un acquis. Une question de capacité ne doit pas exiger la
création d'un produit pour être répondue.

### 5. Le groupe reste une dimension de premier rang

Une société, une unité ou une entité juridique n'est pas un produit. Elle ne se déduit pas
d'un portefeuille de produits. Tant qu'aucun objet `OrganizationUnit` de premier rang
n'est exposé et projeté, le product-centrisme écrase les frontières du groupe. Cette
dimension est traitée comme un sujet distinct (scope et quartier d'organisation), pas comme
un attribut du produit.

## Conséquences

### Positives

- L'accueil répond immédiatement à « quels produits avons-nous ? », sans inventaire ni
  galerie d'intentions.
- Le produit reste le repère stable de la carte, sans devenir un fourre-tout.
- Les capacités, applications, interfaces et données conservent une existence propre et
  une lecture possible sans produit. La société/groupe reste une dimension distincte à
  modéliser, pas un attribut du produit.
- Le modèle résiste au product-washing : un objet sans produit est un cas légitime et
  signalé, pas une erreur de saisie.

### Négatives, acceptées

- Deux vocabulaires de « centre » coexistent : pivot de navigation (produit) et centre de
  lecture (dépend du niveau). La documentation doit le dire explicitement.
- Le titre de l'accueil doit refléter la lentille active : « Produits de l'entreprise » en
  lecture Produits, « Carte de l'entreprise » pour les autres lentilles. Ce n'est pas un
  texte figé.
- Les objets sans produit restent visibles avec une anomalie, ce qui peut être lu comme du
  bruit par un utilisateur habitué aux inventaires exhaustifs.

## Invariants et tests nommés

| Invariant | Test | État |
|---|---|---|
| `/` ouvre le catalogue produits (ADR-025) | `home_opens_product_catalog` (router) | **couvert** |
| La carte garde la lentille primaire `PRODUCT` | `map_default_lens_is_product` | **couvert** |
| À L0, les seuls nœuds de carte sont des produits | `map_products_are_the_only_nodes_at_l0` | **couvert** |
| Changer de lentille ne déplace aucun objet | `map_keeps_geometry_across_lenses` | couvert (ADR-023) |
| Une question de capacité n'exige pas de produit | `capability_view_without_product` | à ajouter |
| Une question applicative n'exige pas de produit | `application_view_without_product` | à ajouter |
| Une société n'est pas un produit | `group_is_not_a_product` | à ajouter (dépend d'`OrganizationUnit`) |
| Le premier plan suit le niveau de zoom | `level_foreground_follows_zoom` | à ajouter |
| Un produit n'est pas un conteneur obligatoire | `product_is_not_mandatory_container` | à ajouter |

## Ce que cette décision ne dit pas

- Elle n'ajoute **aucun niveau** de carte : la carte reste L0 à son point d'entrée ; l'accueil est le catalogue (ADR-025).
- Elle ne fait pas du produit un conteneur de tous les objets.
- Elle ne remplace pas la dimension organisationnelle du groupe.
- Elle ne transforme pas les applications en produits, ni les produits en applications.
- Elle ne change pas la géographie stable ni les règles de projection de C-10.

## Références

- ADR-023 — Le produit et la carte d'entreprise remplacent le domaine et l'intention comme entrée.
- `docs/contracts/C-10-produit-carte-navigation.md` — `MAP-001`, `MAP-015`, projection L0.
- `docs/ui-spec.md` §1 — principes de la Carte.
- `crates/core/src/map.rs` — `MapQuery`, `MapLens::default_for`, `project_enterprise`.
