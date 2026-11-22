# ADR-023 — Le produit et la carte d'entreprise remplacent le domaine et l'intention comme entrée

| | |
|---|---|
| Statut | **Accepté** — décision produit structurante |
| Date | 18 novembre 2026 |
| Remplace | ADR-022 §1 (l'Atelier est le point d'entrée) et §4 (la vue a une intention) |
| Conserve | ADR-022 §2 (création depuis la vue), §3 (formalisation progressive), §5 (deux postures), §6 (commandes) |
| Portée | Modèle métier, navigation, page d'accueil, pages produit/équipe/value stream, contrats de routes et de projection |
| Décisions liées | ADR-012 (routage par fragment), ADR-013 (pas d'IA dans le navigateur), ADR-019 (cycle de vie des interfaces), ADR-020 (virtualisation), ADR-021 (ArchiMate 3.2/4), ADR-022 (Atelier) |
| Complété par | **ADR-024** — le produit est le pivot de la carte, pas le centre du référentiel ; **ADR-025** — l'accueil est le portefeuille de produits, la carte reste `/map` |

## Contexte

Le MVP3 a fait de l'**Atelier de vues** le point d'entrée du produit. Cette décision a corrigé un défaut réel : le lot 1 s'ouvrait sur un inventaire de domaines et se lisait comme une CMDB. Mais elle a déplacé le problème plutôt que de le résoudre.

Trois symptômes sont désormais visibles.

1. **L'accueil est ultra guidé, mais il ne raconte rien de l'entreprise.** Il propose sept intentions de vue avant que l'utilisateur ait vu l'entreprise. On demande « quel type de vue voulez-vous créer ? » à quelqu'un qui vient d'abord chercher « à quoi ressemble mon SI ? ».
2. **« Créer un domaine à partir d'une intention » mélange deux notions.** Un domaine est tantôt une zone de connaissance, tantôt un périmètre de responsabilité ; une intention est un modèle de vue. L'un ne se déduit pas de l'autre. L'utilisateur ne peut pas répondre à « nommez le domaine » avant d'avoir situé les produits, les équipes et les flux.
3. **Le mot « domaine » est ambigu.** Il désigne tour à tour une zone de connaissance métier (DDD), une unité d'organisation, un périmètre de responsabilité, une frontière de sécurité ou un regroupement d'applications. Utilisé comme clé primaire de navigation et de propriété, il brouille la responsabilité réelle : **une équipe possède un produit**.

La demande produit est claire : **voir la big picture, puis descendre jusqu'à l'infrastructure**, en passant par les produits, les applications et les interfaces. Le cœur du modèle reste ArchiMate ; BPMN et UML interviennent là où ils apportent de la précision.

## Décision

### 1. L'unité de lecture et de responsabilité est le produit

Le produit est l'unité durable de valeur offerte à des utilisateurs internes ou externes. Il est possédé par une équipe, il a un cycle de vie, une valeur, des capacités, des applications, des interfaces, des données et des risques.

Un produit n'est **pas** :

- une application : une application est un système logiciel déployé ; un produit peut utiliser plusieurs applications, et une application peut servir plusieurs produits ;
- un projet : un projet a une fin ; un produit vit ;
- un domaine : un domaine est une classification de connaissance, pas un propriétaire ;
- un value stream : un value stream est un flux de bout en bout, un produit y contribue.

### 2. Le domaine devient une classification, l'équipe devient le propriétaire

Le `Domain` historique est conservé sous le nom de **domaine métier** (`BusinessDomain`) : il classifie les produits, capacités et applications par zone de connaissance. Il ne possède plus rien et n'est plus le point d'entrée.

La responsabilité est portée par une **équipe** (`Team`) :

- une équipe possède un ou plusieurs produits ;
- une équipe opère une ou plusieurs applications ;
- une équipe peut contribuer à un produit sans le posséder ;
- une équipe a un type (stream-aligned, plateforme, enabling, sous-système compliqué, autre) et des modes d'interaction ;
- une application partagée est un signal de couplage, pas une anomalie à cacher.

### 3. Le point d'entrée est la carte de l'entreprise

La route `/` ouvre la **Carte de l'entreprise**, pas l'Atelier et pas une galerie d'intentions.

> **Depuis ADR-025**, `/` ouvre le **portefeuille de produits** ; la carte est servie par
> `/map`.

La carte est une **projection générée, à géographie stable, du référentiel**. Elle n'est ni un diagramme dessiné à la main, ni un inventaire :

- les **quartiers** sont des value streams, portefeuilles ou unités d'organisation ;
- les **bâtiments** sont des produits ;
- les **blocs** sont des applications, visibles quand on ouvre un produit ;
- les **utilités** sont les produits plateformes et les infrastructures partagées ;
- les **routes** sont des interfaces ou des flux agrégés ;
- les **ports** sont les interfaces exposées sur une frontière de produit ou d'application.

La géographie est **stable** : un produit reste au même endroit d'une session à l'autre et d'une lentille à l'autre. Les lentilles changent la couleur, les arêtes et les filtres, jamais la disposition.

### 4. La navigation est un zoom sémantique à cinq niveaux

| Niveau | Nom | Ce que l'on voit | Ce que l'on ne voit pas |
|---|---|---|---|
| **L0** | Entreprise | value streams, produits, produits plateformes, flux agrégés | applications individuelles, composants, nœuds |
| **L1** | Portefeuille / quartier | produits du périmètre, plateformes partagées, interfaces produit à produit | composants, infrastructure |
| **L2** | Produit | frontière du produit, ports, applications, interfaces applicatives, dépendances externes | composants internes, nœuds |
| **L3** | Application | composants, services, interfaces, données, déploiement | nœuds détaillés |
| **L4** | Infrastructure | nœuds, environnements, réseaux, applications déployées, interfaces techniques | composants métier |

Le zoom est **sémantique** : franchir un seuil change la requête et l'agrégation, pas seulement l'échelle du dessin. Il est aussi **géographique** : le cadrage reste continu, l'objet cliqué reste au même endroit, le fil d'Ariane situe le niveau courant.

### 5. Les dimensions sont des lentilles, pas des diagrammes séparés

Une **lentille** est une projection du même graphe. Elle répond à une question et encode une dimension :

- valeur : value streams, étapes, contributions ;
- capacité : couverture, maturité, duplication ;
- produit : portefeuille, cycle de vie, criticité ;
- organisation : équipes, propriété, dépendances inter-équipes ;
- applicatif : applications, composants, services, âge ;
- interface : fournisseurs, consommateurs, versions, contrats ;
- technologie : plateformes, nœuds, obsolescence ;
- données : objets, flux, classification ;
- temps : états planifié/actif/expiré, migrations ;
- risque et coût : criticité, conformité, dette, propagation.

Les lentilles se **superposent** : couleur par équipe, arêtes par interface, filtre par cycle de vie. Elles ne se remplacent pas et ne recalculent jamais la géographie.

Les sept intentions de l'Atelier ne disparaissent pas : elles deviennent des **lentilles ou des actions de détail**. « Applicatif » devient la lentille applicative au niveau produit ; « Processus » ouvre le BPMN d'une étape de value stream ; « Revue d'interface » ouvre la lentille interface. Elles ne sont plus un préalable à la création d'un domaine.

### 6. ArchiMate est la colonne vertébrale, BPMN et UML sont contextuels

- **ArchiMate** porte la structure aux niveaux L0 à L4 : produits, services, capacités, applications, composants, interfaces, nœuds, relations. C'est la notation par défaut de la carte.
- **BPMN** intervient au niveau d'une étape de value stream ou d'un processus de produit. Un processus BPMN `REALIZES` ou `DESCRIBES` une étape ; il n'apparaît pas au niveau entreprise.
- **UML** intervient au niveau d'une application ou d'une interface : composant, classe, séquence, déploiement. Un composant UML `IMPLEMENTS` un composant applicatif ; une interface UML `REALIZES` une interface applicative.
- Aucune notation n'est imposée à un niveau où elle n'apporte pas de précision. Une carte d'entreprise en UML est un contre-sens ; un contrat d'interface en ArchiMate seul est souvent insuffisant.

Le référentiel reste **un seul graphe d'objets**. Une représentation BPMN ou UML est une vue d'un objet existant, jamais une copie.

### 7. L'Atelier reste la surface d'édition

La carte est le **point d'entrée** et la **surface de navigation**. L'Atelier reste la **surface d'édition** des vues authored : un diagramme narratif, une vue de revue, une Story.

La règle de continuité est conservée : on peut créer et modifier un produit, une équipe, une application ou une interface **depuis la carte**, sans formulaire modal ni changement de route. Un clic profond ouvre la page de l'objet, qui reste une vue de la carte à son niveau, pas un formulaire.

### 8. Interfaces de premier rang, à tous les niveaux

Une interface n'est pas une flèche. C'est un objet de gouvernance avec :

- un **niveau** : métier/produit, applicatif, infrastructure ;
- une **catégorie** : `BUSINESS`, `APPLICATION`, `TECHNICAL` ;
- un **type** : API, événement, fichier, batch, UI, base de données, réseau, autre ;
- un **fournisseur** : produit, application ou nœud ;
- des **consommateurs** : produits, applications ou nœuds ;
- des **versions** datées et un contrat ;
- une **exposition** : interne, produit, partenaire, public ;
- une **criticité**, une classification de données, un SLA.

À la carte, une route peut agréger plusieurs interfaces. Elle est toujours **dépliable** : le clic montre les interfaces réelles, leurs versions et leurs consommateurs.

## Conséquences

### Positives

- Le produit répond à la question « à quoi ressemble mon entreprise ? » avant de demander « quelle vue voulez-vous créer ? ».
- La responsabilité est explicite : une équipe possède un produit, opère des applications, expose des interfaces.
- Le zoom donne un parcours unique de l'entreprise à l'infrastructure, sans changer d'outil ni perdre le contexte.
- Les dimensions (valeur, capacité, organisation, interface, temps, risque) sont des lentilles d'une même géographie, pas des diagrammes concurrents qui divergent.
- ArchiMate reste le socle ; BPMN et UML sont utilisés au bon niveau de détail.
- Le référentiel reste la vérité : une route agrégée se déplie vers des interfaces identifiées, pas vers une approximation.

### Négatives, acceptées

- L'accueil, les routes, la Bibliothèque et les tests E2E changent. Ce n'est pas une retouche cosmétique.
- Le modèle natif s'enrichit de `Product`, `Team`, `ValueStream`, `Capability`, `Application`, `TechnologyNode` et `InterfaceConsumption`. Ces objets ne sont pas tous des classes ArchiMate : ils forment la couche native qui relie ArchiMate, BPMN et UML.
- Le `Domain` existant ne peut pas être converti automatiquement en produit ou en équipe. Une migration guidée, non destructive, est nécessaire.
- La carte exige un moteur de projection, des ancres de position et des règles d'agrégation. Ce n'est pas un composant de rendu trivial.
- Deux vocabulaires coexistent : `ViewIntent` pour les vues authored, `MapLens` pour la carte. Cette séparation est volontaire ; les fusionner créerait une collision de contrats.

### Ce que cette décision ne dit pas

- Ce n'est pas une CMDB : la carte ne remplace pas un inventaire exhaustif, et le Contrôle reste secondaire.
- Ce n'est pas un outil de découverte automatique : les objets viennent du modèle, d'un import ou d'une saisie explicite.
- Ce n'est pas une conformité BPMN ou UML complète : les sous-ensembles annoncés restent la règle.
- Ce n'est pas un remplacement d'ArchiMate : la cible reste ArchiMate 3.2 implémenté, ArchiMate 4 préparé.
- Ce n'est pas un moteur de disposition automatique parfait : la géographie est stable, les nouvelles formes sont placées par une heuristique et peuvent être déplacées explicitement.

## Invariants vérifiés par des tests nommés

| Invariant | Test |
|---|---|
| `/map` ouvre la carte de l'entreprise, jamais l'Atelier ni une galerie d'intentions | `map_opens_enterprise_map` |
| La carte se zoome de l'entreprise à l'infrastructure | `map_zooms_enterprise_to_infrastructure` |
| Changer de lentille ne déplace aucun objet | `map_keeps_geometry_across_lenses` |
| Une route agrégée se déplie vers des interfaces identifiées | `map_aggregate_edge_expands_to_interfaces` |
| Un produit exige une équipe accountable | `product_requires_accountable_team` |
| Une application est utilisée par des produits, jamais possédée par eux | `application_is_used_by_products_not_owned` |
| Un domaine est une classification, pas un propriétaire | `domain_is_classification_not_owner` |
| Un lien profond restaure niveau, périmètre, lentille et date | `map_deep_link_restores_context` |
| Le curseur temporel ne modifie pas la géométrie | `map_temporal_cursor_does_not_mutate_layout` |
| Les anciennes routes de domaine restent résolvables | `legacy_domain_routes_still_resolve` |
| Un processus BPMN s'ouvre depuis une étape de value stream | `bpmn_process_opens_from_value_stream_stage` |
| Un composant UML s'ouvre depuis une application | `uml_component_opens_from_application` |
| Le premier lancement crée un produit, pas un domaine | `first_run_creates_product_not_domain` |
| Aucune migration destructive n'est exécutée | `legacy_domain_migration_is_additive` |

## Migration depuis le modèle MVP3

| Objet MVP3 | Devient | Règle |
|---|---|---|
| `Domain` | `BusinessDomain` | Conservé comme classification. Ne possède plus d'objets. |
| `responsibleDomainId` | `businessDomainId` | Conservé pour l'historique. Les nouveaux objets utilisent `accountableTeamId`. |
| Page `/library/domains/:id` | `/business-domains/:id` | Ancienne route conservée en alias. |
| `ViewIntent` | `ViewIntent` (Atelier) + `MapLens` (Carte) | Aucune fusion : les deux catalogues sont distincts. |
| Interface `providerDomainId` | `providerObjectId` + `businessDomainId?` | Le fournisseur peut être un produit, une application, un composant ou un nœud. |
| Diagramme authored | Vue authored | Inchangé ; il reste éditable dans l'Atelier. |
| Carte | Nouvel objet de projection | Pas de migration de layout : la carte a ses propres ancres. |

La migration est **guidée et additive** : le produit propose de rattacher un ancien domaine à une équipe et à un produit, mais ne supprime jamais l'objet ni son historique. Un domaine sans équipe reste visible avec l'anomalie `PRD-031`.

## Références

- The Open Group, *ArchiMate 3.2 Specification* : `Product` est un élément composite métier (services + structure passive + contrat), `ValueStream` et `Capability` appartiennent à la couche Strategy. Un produit ArchiMate n'est donc pas une unité de propriété : la couche native `Product` est nécessaire.
- Simon Brown, *C4 model* : les abstractions hiérarchiques (système, conteneur, composant, code) valident le zoom, mais s'arrêtent au logiciel ; la carte les étend à l'entreprise et à l'infrastructure.
- Matthew Skelton et Manuel Pais, *Team Topologies* : quatre types d'équipes, trois modes d'interaction, équipe stream-aligned propriétaire d'un flux de valeur.
- TOGAF / Business Architecture Guild : la capability map est stable et indépendante de l'organisation ; le value stream est un flux ; les deux se croisent par les étapes.
- Product operating model et taxonomies produit : portefeuille → ligne/famille → produit → plateforme ; le produit n'est pas l'application.
- Pratiques de catalogue d'API : une interface a un fournisseur, des consommateurs, des versions, un cycle de vie et un propriétaire ; une intégration n'est pas une interface.
- Pratiques de « city map » / paysage applicatif : géographie stable, pan/zoom, calques et styles ; la carte doit être générée depuis le référentiel pour ne pas pourrir.

## Documents impactés

- `functional-specifications.md` — nouvelle section « Cible v0.4 — Le produit, la carte et le zoom », FR-055 à FR-081.
- `docs/ui-spec.md` — v2.2 : portefeuille de produits, carte, niveaux, lentilles, pages produit/équipe/value stream/application/infrastructure.
- `docs/contracts/C-10-produit-carte-navigation.md` — contrat des objets, de la projection et des routes.
- `docs/contracts/C-06-routes.md` — conservé ; les routes de la carte sont ajoutées par C-10.
- `README.md` — positionnement et état (cible v0.4 vs implémentation MVP3).
- `technical-specifications.md` — avenant au §15.0/§15.2 ; le moteur de carte fera l'objet d'un lot dédié.
