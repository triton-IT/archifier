# Spécification fonctionnelle et technique — MVP1 / v0.1 de l’outil de modélisation d’architecture d’entreprise

## Cadre, objectifs et décisions structurantes

Cette spécification décrit un **MVP1 / v0.1 réalisé sur mesure** d’un référentiel de modélisation d’architecture d’entreprise collaboratif. Le produit doit permettre à plusieurs domaines de construire leurs modèles, de publier des interfaces qu’ils exposent aux autres domaines, de réutiliser les objets publiés sans les dupliquer, de documenter l’architecture, de la parcourir et de l’interroger en langage naturel.

Le produit possède une surface principale, l’**Atelier de vues**, qui est son point d’entrée, et deux postures de cette même surface : **CONCEPTION**, orientée création et modification, et **LECTURE**, orientée navigation, recherche, consultation historique, comparaison, storytelling et revue des interfaces. Un espace secondaire de **Contrôle** héberge inventaires, anomalies et couverture. Archifier est un outil d’architecture, pas une CMDB : on y conçoit en dessinant, et le référentiel se remplit comme conséquence du geste de conception.

Le référentiel est la source de vérité ; une vue est une **représentation d’objets du référentiel**. La suppression d’une occurrence graphique ne supprime donc jamais implicitement l’objet qu’elle représente. Inversement, un même objet peut apparaître dans plusieurs vues, plusieurs histoires et plusieurs domaines consommateurs. Cette séparation est ce qui permet à la surface d’être fluide sans que la vérité devienne approximative.

### Statut d’ArchiMate 4

Il faut recadrer explicitement la cible ArchiMate 4. Au **17 novembre 2026**, les sources officielles de The Open Group que j’ai pu vérifier continuent d’indiquer **ArchiMate 3.2 comme dernière version du standard**. Le programme de certification officiel reste nommé « ArchiMate 3 » et The Open Group décrit 3.2 comme la version la plus récente du standard. Je n’ai trouvé aucune spécification ArchiMate 4 publiée officiellement par The Open Group à cette date. citeturn992156search0turn992156search1

La v0.1 doit donc avoir **ArchiMate 4 comme cible explicite**, mais **ne doit inventer aucun métamodèle ArchiMate 4**. L’implémentation initiale doit être conforme au métamodèle ArchiMate 3.2 disponible, avec une architecture permettant de charger ultérieurement un métamodèle `archimate:4.x` sans migration destructive du référentiel.

La décision de conception est donc :

| Sujet | Décision v0.1 |
|---|---|
| Cible métier | ArchiMate 4 |
| Métamodèle effectivement implémenté au lancement | ArchiMate 3.2 |
| Identifiant de métamodèle | `archimate:3.2` |
| Préparation ArchiMate 4 | métamodèle versionné et externe au code métier |
| Migration future | mapping explicite 3.2 → 4.x après publication officielle |
| Communication UI | « Cible ArchiMate 4 — règles actuellement basées sur ArchiMate 3.2 » |
| Interdiction | aucune classe, relation ou règle dite « ArchiMate 4 » ne peut être créée avant publication normative |

Cette séparation est d’autant plus importante qu’ArchiMate définit de vraies contraintes de métamodèle : tous les types de relations ne sont pas autorisés entre tous les types d’éléments. The Open Group présente également explicitement les couches Business, Application et Technology comme des dimensions centrales du langage. citeturn414250search1turn414250search2

BPMN sera basé sur **BPMN 2.0.2**, qui reste la spécification formelle publiée par l’OMG avec métamodèles et XSD normatifs. UML sera basé sur **UML 2.5.1**, également publié avec métamodèle XMI normatif. citeturn339674search6turn339674search7

**Hypothèses MVP1.** Une instance correspond à un référentiel d’entreprise. Les utilisateurs sont identifiés individuellement afin d’assurer auteur, historique et commentaires. Tous les utilisateurs ont exactement les mêmes capacités. Un « domaine » est une unité de responsabilité architecturale, pas une frontière de sécurité.

**Hors périmètre MVP1.** Il n’existe ni RBAC, ni ABAC, ni workflow d’approbation obligatoire, ni restriction de modification par domaine. Aucun connecteur applicatif tiers n’est fourni, à l’exception de l’import/export OpenAPI. La synchronisation temps réel de type édition simultanée caractère par caractère est **reportée au MVP2** (voir « Collaboration temps réel ») ; le moteur BPMN exécutable, la simulation de processus et la conformité UML/BPMN intégrale restent différés.

## Périmètre fonctionnel du MVP1

Le produit est organisé autour de sept objets métier principaux : **Domain**, **ModelElement**, **Relationship**, **Interface**, **Diagram**, **Story** et **Review**. La notion d’interface est volontairement un objet du référentiel et non une simple forme graphique.

Les exigences fonctionnelles sont les suivantes. Les FR-040 à FR-054 formalisent la décision structurante de l’**Atelier de vues** ; les FR-001 à FR-039 couvrent le référentiel, les interfaces, la temporalité, la documentation, l’IA et la collaboration.

| ID | Exigence MVP1 |
|---|---|
| FR-001 | Créer, modifier, archiver et consulter des domaines. |
| FR-002 | Affecter à chaque élément, relation, diagramme et interface un `responsibleDomainId`. |
| FR-003 | Créer des éléments ArchiMate, BPMN et UML à partir des métamodèles supportés. |
| FR-004 | Attribuer à chaque objet un identifiant stable global indépendant de son nom et de ses représentations graphiques. |
| FR-005 | Créer plusieurs représentations graphiques du même objet sans dupliquer l’objet. |
| FR-006 | Référencer dans un diagramme du domaine B un élément ou une interface relevant du domaine A. |
| FR-007 | Distinguer trois catégories d’interface : `BUSINESS`, `APPLICATION`, `TECHNICAL`. |
| FR-008 | Publier une version d’interface avec contrat, version, domaine fournisseur, date de disponibilité et date de fin. |
| FR-009 | Déclarer qu’un domaine, élément ou interface consomme une interface publiée. |
| FR-010 | Rechercher et filtrer les interfaces par domaine, catégorie, état, consommateur, version et fenêtre temporelle. |
| FR-011 | Afficher les dépendances entrantes et sortantes d’un élément ou d’une interface. |
| FR-012 | Comparer deux versions d’une interface et faire ressortir les changements de contrat. |
| FR-013 | Effectuer une revue des interfaces d’un domaine avec vues synthétique et détaillée. |
| FR-014 | Ajouter commentaires et observations à une revue sans workflow d’approbation. |
| FR-015 | Passer à tout moment entre les postures conception et lecture d’une même vue, en conservant la vue, le cadrage et la sélection. |
| FR-016 | Documenter tout domaine, élément, relation, interface, diagramme et histoire avec Markdown. |
| FR-017 | Créer des liens internes typés vers tout objet du référentiel. |
| FR-018 | Composer une Story à partir de texte, diagrammes, vues, éléments et transitions. |
| FR-019 | Exporter domaine, objet, revue ou Story sous forme Markdown navigable. |
| FR-020 | Importer un document OpenAPI et créer ou rapprocher les objets correspondants dans le référentiel. |
| FR-021 | Exporter une interface applicative compatible en OpenAPI. |
| FR-022 | Interroger le référentiel en langage naturel et recevoir des réponses contenant les IDs des objets sources. |
| FR-023 | Demander à l’IA une analyse d’impact incluant dépendances et temporalité. |
| FR-024 | Demander à l’IA une proposition de modèle, relation, diagramme, documentation ou Story. |
| FR-025 | Toute proposition IA modifiant le référentiel doit être présentée en aperçu, validée par le moteur déterministe, puis confirmée explicitement par l’utilisateur. |
| FR-026 | Consulter l’historique de chaque objet et comparer deux révisions. |
| FR-027 | Identifier les liens internes cassés à la suite d’un archivage ou d’une évolution. |
| FR-028 | Naviguer d’un élément vers les diagrammes, Stories, interfaces et domaines qui l’utilisent. |
| FR-029 | Produire une vue « architecture à la date T » tenant compte des périodes d’effet des interfaces. |
| FR-030 | Effectuer une recherche plein texte sur les noms, descriptions, documentation et contrats. |
| FR-040 | Faire de l’**Atelier de vues** le point d’entrée par défaut du produit : ouvrir l’application place l’utilisateur devant une surface de conception, jamais devant un inventaire. |
| FR-041 | Créer tout objet du référentiel — élément typé, relation, interface, version, domaine, groupe, annotation, documentation — directement depuis une vue, sans formulaire préalable obligatoire. |
| FR-042 | Éditer sur place, dans le canevas, le libellé, le type, le domaine responsable et les propriétés essentielles d’une forme, sans changer d’écran. |
| FR-043 | Dessiner une relation en tirant d’un nœud vers un autre, avec un choix de types filtré par le métamodèle chargé. |
| FR-044 | Concevoir une interface comme une frontière porteuse de ports, et rendre ses versions et son état temporel lisibles dans la vue. |
| FR-045 | Créer et manipuler des domaines, zones, couloirs, groupes, cadres et annotations comme des objets de la vue, et non comme des réglages cachés. |
| FR-046 | Démarrer une vue depuis une **intention de vue** (contexte, applicatif, processus, déploiement, capacité, migration, revue d’interface) qui prépare notation, zones, palette et règles de disposition. |
| FR-047 | Formaliser progressivement : une forme peut exister à l’état de **croquis** puis recevoir un type de métamodèle, un domaine et ses propriétés, la publication restant bloquée tant que la formalisation est incomplète. |
| FR-048 | Donner à chaque vue une identité d’objet : titre, intention, périmètre, notation, domaine responsable, cadrage et visibilité ; la retrouver dans une bibliothèque de vues. |
| FR-049 | Traiter le temps comme une dimension visuelle de la vue : curseur de date T, états `SCHEDULED` / `ACTIVE` / `EXPIRED` et chevauchements encodés sur les formes et les liens. |
| FR-050 | Garantir l’édition fluide : enregistrement automatique, annulation/rétablissement, palette de commandes et raccourcis clavier, sans bouton « Enregistrer » comme action principale. |
| FR-051 | Publier, partager et exporter une vue (lien stable, cadrage, SVG, Markdown), et l’utiliser comme bloc de Story ou comme pièce de Revue. |
| FR-052 | Reléguer l’inventaire et les indicateurs dans un espace **Contrôle** secondaire : aucun écran de type CMDB ne doit être requis pour créer, modifier ou comprendre une architecture. |
| FR-053 | Offrir la parité clavier de tous les gestes de conception et des états vides qui invitent à une action productive. |
| FR-054 | Accompagner le premier lancement par une vue guidée et des modèles de départ, afin que la première valeur soit produite en moins de trois minutes. |

### L’Atelier de vues : point d’entrée et surface principale

**Décision structurante.** Le point d’entrée du produit n’est ni un tableau de bord, ni un inventaire, ni une liste de domaines : c’est un **Atelier de vues** (designer de vues). Archifier est un outil d’architecture avant tout, pas une CMDB. L’architecte pense, arbitre et communique en construisant des vues ; le référentiel se remplit comme **conséquence** du geste de conception, jamais comme préalable administratif.

Cette décision ne contredit pas le principe « le référentiel est la source de vérité ». Elle en sépare les plans :

| Plan | Rôle | Conséquence UX |
|---|---|---|
| **Vérité** | Objets, relations, interfaces, versions, historique | Aucune forme graphique ne crée un objet implicite non validé ; aucune suppression de forme ne supprime un objet. |
| **Surface** | L’Atelier, les vues, le cadrage, le récit | C’est l’interface première : on y crée, on y édite, on y comprend, on y publie. |
| **Contrôle** | Inventaires, anomalies, couverture, exports | Espace secondaire, consultatif, jamais passage obligé. |

**Renversement du parcours.** Le parcours de référence est :

1. j’ouvre l’Atelier ;
2. je choisis une intention de vue, ou je repars d’une vue existante ;
3. je place des formes, je les relie, je les nomme ;
4. le produit **formalise** ces formes en objets du référentiel (type, domaine, propriétés) au moment opportun ;
5. je publie, je raconte, je compare, je révise.

Le parcours explicitement interdit est : « créer un objet dans un formulaire, puis chercher une vue où le poser ». Aucune création ne doit **exiger** de quitter le canevas.

**Ce que l’on conçoit depuis une vue.** Tous les objets sont designables depuis l’Atelier :

| Geste dans la vue | Objet produit ou modifié |
|---|---|
| Déposer une classe depuis la palette | `ModelElement` typé, avec domaine responsable |
| Tirer un lien entre deux formes | `Relationship`, type filtré par le métamodèle |
| Dessiner une frontière et poser des ports | `Interface` + rattachement de versions |
| Tracer une bande ou une zone étiquetée | `Domain`, groupe ou couloir, selon l’intention |
| Double-cliquer sur un libellé | `name`, propriété ou documentation Markdown |
| Encadrer une zone et choisir « Cadrer » | cadrage de vue réutilisable par une Story |
| Capturer la vue à une date T | vue temporelle, pièce de Revue ou d’analyse d’impact |
| Publier la vue | vue publiée, lien stable, export SVG/Markdown |

**Formalisation progressive, sans laxisme sémantique.** La fluidité du dessin ne doit pas produire un référentiel faux. Une forme suit un cycle explicite :

```text
Croquis (local, non persisté comme objet)
   │  nommer + choisir un type + un domaine
   ▼
Brouillon formalisé (persisté, révisable, non publiable)
   │  compléter les propriétés requises par le profil
   ▼
Objet valide (publiable, référençable, exportable)
```

| Code | Règle |
|---|---|
| `DSG-001` | Une forme publiée doit être formalisée : type de métamodèle et domaine responsable renseignés. |
| `DSG-002` | Une relation dessinée doit être autorisée par le métamodèle chargé ; sinon elle est refusée avec le type fautif. |
| `DSG-003` | Une forme sans objet référencé ne peut être ni publiée ni exportée ; elle reste un croquis local. |
| `DSG-004` | Un port d’interface doit référencer une interface existante ou créer une interface formalisée dans le même geste. |
| `DSG-005` | Une suppression de forme retire la représentation, jamais l’objet ; la suppression d’objet reste une action explicite et confirmée. |

**Grammaire visuelle : l’outil doit ressembler à de l’architecture.** L’austérité n’est pas une neutralité : une grille de cartes et de formulaires ne dit rien de l’architecture. La vue doit porter une grammaire spatiale stable, commune à toutes les notations :

- **Bandage par couche** : les couches ArchiMate (Business, Application, Technology, Physique, Motivation, Migration) sont des bandes horizontales, nommées et atténuables.
- **Zones de domaine** : un domaine est un territoire visible et attribuable, pas une colonne de table. Les interfaces se posent sur ses frontières.
- **Ports et frontières** : une interface exposée est un point d’attache sur la frontière d’un domaine ; une consommation est un lien qui traverse la frontière.
- **Encodage temporel non chromatique seul** : `ACTIVE` plein, `SCHEDULED` pointillé, `EXPIRED` fantôme barré ; le libellé d’état est toujours présent.
- **Hiérarchie typographique** : nom d’objet, type, domaine, état temporel, chacun avec un niveau distinct ; aucune information critique dans une infobulle seule.
- **Densité maîtrisée** : la vue doit rester lisible à 40 formes comme à 500 ; le détail se révèle au zoom sémantique, pas par empilement de panneaux.
- **Mouvement utile** : transitions brèves pour la sélection, le cadrage et l’apparition d’un lien ; `prefers-reduced-motion` respecté ; aucun effet décoratif gratuit.

**L’Atelier n’est pas un outil de dessin libre.** Trois garde-fous empêchent la dérive vers le « PowerPoint d’architecture » :

1. Toute forme est soit un croquis explicitement marqué, soit le rendu d’un objet identifié par un ID stable.
2. Les types de relations proposés sont filtrés par le métamodèle ; une relation interdite est refusée avant création.
3. Réutiliser un objet existant se fait par recherche contextuelle depuis le canevas ; le produit ne crée jamais silencieusement un doublon.

**Postures CONCEPTION et LECTURE (anciennement MODÉLISER / VISUALISER).** Ce ne sont plus deux applications ni deux jeux de routes, mais deux **postures d’une même surface** :

- **Posture conception** : palette ouverte, poignées, ports, inspecteur contextuel, outils de zone et de cadrage.
- **Posture lecture** : chrome réduit, zoom sémantique, navigation par dépendances, curseur temporel, liens vers fiches, Stories et Revues.

La bascule conserve la vue, le cadrage et la sélection. Elle ne change aucun droit. Un lecteur qui reçoit un lien de vue arrive directement dans la posture lecture, sans passer par un menu.

**Accueil, bibliothèque et contrôle.** L’accueil est l’Atelier : soit la dernière vue ouverte, soit « Nouvelle vue » avec des intentions de départ. La **Bibliothèque** (liste des vues, recherche, domaines) et le **Contrôle** (inventaires, anomalies, couverture des métamodèles, exports) sont accessibles depuis la barre latérale, mais aucun des deux n’est un passage obligé. Une fiche d’objet reste disponible et complète : elle est la conséquence d’un clic dans une vue, pas la porte d’entrée du produit.

La page d’un **domaine** reste une vue de synthèse utile — interfaces exposées et consommées, dépendances, vues publiées, Stories — mais elle est atteinte depuis une zone de domaine ou la Bibliothèque, jamais imposée au lancement.

### Revue d’interfaces

Une Review est un objet pérenne :

```text
Review
  id
  title
  domainId
  temporalWindow
  filterSnapshot
  createdAt
  createdBy
  status = OPEN | CLOSED
  markdownSummary
```

Une revue d’un domaine doit pouvoir afficher :

| Vue | Informations |
|---|---|
| Synthèse | nombre d’interfaces actives, futures, expirées, versions en transition, ruptures identifiées |
| Exposées | interfaces dont le domaine est fournisseur |
| Consommées | interfaces externes référencées par le domaine |
| Entrantes | dépendances venant d’autres domaines |
| Sortantes | dépendances vers d’autres domaines |
| Calendrier | mises en disponibilité, échéances et chevauchements |
| Contrats | versions, changements compatibles ou ruptures |
| Consommateurs | domaines et objets utilisant chaque version |
| Détail | documentation, contrat OpenAPI éventuel, diagrammes et historique |

Une Review ne vaut pas approbation. Elle enregistre une photographie de filtres, une date et des commentaires ; elle peut être rouverte ou comparée ultérieurement.

## Métamodèle commun, interfaces et temporalité

Le référentiel commun ne doit pas « fusionner » ArchiMate, BPMN et UML. Il contient un noyau agnostique et des **concepts typés par notation**.

```text
RepositoryObject
 ├── ModelElement
 ├── Relationship
 ├── Interface
 ├── Diagram
 ├── Story
 ├── Review
 └── Domain

ModelElement
 ├── notation = ARCHIMATE | BPMN | UML | NATIVE
 ├── metamodelVersion
 ├── metaclass
 └── semanticProperties
```

Un élément UML `Component`, un élément ArchiMate `ApplicationComponent` et un participant BPMN restent trois objets distincts. Une relation transversale peut les relier, mais ne doit jamais prétendre qu’ils sont équivalents sans une sémantique explicitement déclarée.

Le MVP introduit donc des relations natives de référencement telles que `REFINES`, `IMPLEMENTS`, `REALIZES`, `DESCRIBES`, `EXPOSES`, `CONSUMES` et `TRACE_TO`. Lorsque la relation appartient à une notation normalisée, son type canonique de cette notation est conservé ; lorsqu’elle traverse les notations, elle appartient au métamodèle natif du produit.

### Sous-ensemble BPMN MVP1

BPMN 2.0.2 contient un ensemble beaucoup plus large que ce qui est nécessaire pour la v0.1 ; prétendre à une conformité BPMN exhaustive serait trompeur. La spécification OMG officielle expose d’ailleurs un métamodèle complet et des artefacts machine-readable normatifs. citeturn339674search6

Le support MVP1 est volontairement « **BPMN 2.0.2 Core Modeling Subset** » :

| Famille | Support MVP1 |
|---|---|
| Conteneurs | Process, Participant/Pool, Lane, SubProcess, CallActivity |
| Activités | Task, UserTask, ServiceTask, ManualTask, BusinessRuleTask |
| Événements | Start, End, Intermediate Catch/Throw ; message et timer |
| Gateways | Exclusive, Parallel, Inclusive |
| Flux | SequenceFlow, MessageFlow, Association |
| Données | DataObject, DataStore |
| Annotation | TextAnnotation |

Les chorégraphies, conversations, transactions complexes, compensation avancée et sémantique d’exécution sont différées.

### Sous-ensemble UML MVP1

UML 2.5.1 reste une spécification très large, avec métamodèle XMI normatif. citeturn339674search7

Le support MVP1 vise l’architecture statique :

| Diagramme | Métaclasses principales |
|---|---|
| Component | Component, Interface, Port, Package |
| Class/Data | Class, DataType, Enumeration, Property |
| Deployment | Node, Device, ExecutionEnvironment, Artifact |
| Use Case | Actor, UseCase, System boundary |

Relations supportées : `Association`, `Dependency`, `Usage`, `Realization`, `InterfaceRealization`, `Generalization`, `Composition`, `Aggregation`, `Deployment`.

Les State Machines, Activity Diagrams et Sequence Diagrams complets sont différés.

Pour BPMN comme UML, le moteur doit refuser à la validation toute combinaison de types déclarée non supportée dans le profil MVP1. Le critère n’est donc pas « tout UML fonctionne », mais « tout ce que nous annonçons comme supporté possède un comportement déterministe et testé ».

### Interface comme objet de premier rang

Une interface publiée n’est pas simplement une interface graphique ArchiMate/UML. C’est un objet de gouvernance :

```text
Interface
  id
  logicalInterfaceId
  name
  category
  providerDomainId
  contractType
  documentation
  lifecycle
  notationBindings[]
```

Exemples :

| Catégorie | Exemple | Binding possible |
|---|---|---|
| BUSINESS | « Demander remboursement » | Business Service / Business Interface |
| APPLICATION | « Customer API » | Application Service / Application Interface + OpenAPI |
| TECHNICAL | « Flux événements orders.v1 » | Technology Service / Technology Interface |

L’association exacte aux concepts ArchiMate doit être contrôlée par le métamodèle officiel effectivement chargé ; la v0.1 ne doit pas supposer que les futurs concepts ArchiMate 4 seront identiques à ceux de 3.2.

### Temps, versions et publication

Il faut distinguer **temps d’enregistrement** et **temps d’effet**.

```text
InterfaceVersion
  id
  interfaceId
  semanticVersion
  revision
  publicationStatus
  createdAt
  createdBy
  publishedAt
  availableFrom
  availableUntil
  breakingChange
  supersedesVersionId?
  contractSnapshot
```

`publishedAt` signifie « date à laquelle cette information a été rendue publique dans le référentiel ».

`availableFrom` signifie « date à partir de laquelle le contrat peut être consommé ».

`availableUntil` signifie « instant à partir duquel le contrat n’est plus disponible ».

Les fenêtres sont définies comme des intervalles **semi-ouverts `[availableFrom, availableUntil)`**. Ainsi, une interface qui se termine le 1er janvier à 00:00 n’est plus considérée active à cet instant.

Les règles métier temporelles sont :

| ID | Règle |
|---|---|
| BR-001 | Toute version publiée possède `availableFrom` et `availableUntil`. |
| BR-002 | `availableUntil > availableFrom`. |
| BR-003 | Une date future est autorisée : l’interface est alors `SCHEDULED`. |
| BR-004 | `now < availableFrom` ⇒ `SCHEDULED`. |
| BR-005 | `availableFrom ≤ now < availableUntil` ⇒ `ACTIVE`. |
| BR-006 | `now ≥ availableUntil` ⇒ `EXPIRED`. |
| BR-007 | `DRAFT`, `PUBLISHED` et `WITHDRAWN` sont des statuts éditoriaux distincts du statut temporel. |
| BR-008 | Un changement de contrat publié crée une nouvelle `InterfaceVersion` ; l’historique n’est jamais écrasé. |
| BR-009 | Plusieurs versions peuvent coexister pendant une migration. |
| BR-010 | Une rupture doit être marquée `breakingChange=true` manuellement ou par l’analyse de contrat, puis confirmée par l’utilisateur. |
| BR-011 | L’expiration d’une interface ne supprime pas les relations de consommation : elles deviennent des dépendances vers une version expirée et sont signalées. |
| BR-012 | Une référence de domaine pointe vers l’ID canonique de l’interface ou de sa version ; elle ne crée jamais une copie. |

Exemple de calendrier :

```text
Customer API v1
publishedAt       2026-09-15
availableFrom     2026-10-01T00:00Z
availableUntil    2027-06-01T00:00Z

Customer API v2
publishedAt       2027-02-15
availableFrom     2027-03-01T00:00Z
availableUntil    2028-03-01T00:00Z
breakingChange    true
supersedes        v1
```

Entre mars et mai 2027, v1 et v2 sont toutes deux actives ; la vue de revue doit rendre cette période de transition immédiatement visible.

## Documentation, navigation et storytelling

Tout objet documentaire utilise Markdown natif.

```text
DocumentationBlock
  id
  ownerObjectId
  markdown
  revision
  createdAt
  createdBy
```

Les liens internes ne doivent pas dépendre du nom ni d’un chemin d’URL instable. La forme canonique est basée sur l’ID :

```text
arch://element/01JABC...
arch://interface/01JDEF...
arch://diagram/01JGHI...
arch://story/01JKLM...
```

L’affichage transforme ces URIs en URLs navigables. Si un objet est archivé, le lien reste résolvable vers sa dernière révision. Si la cible a réellement disparu, le lien devient `BROKEN` et est exposé dans le diagnostic documentaire.

L’export Markdown transforme les URIs en liens relatifs :

```text
[Customer API](../interfaces/01JDEF/customer-api.md)
```

L’export d’un domaine produit au minimum :

> **Amendement W1b (18/11/2026).** L'arborescence exacte est gelée dans
> `docs/w1b-contract.md` §1.2/§5.2 : `<root>/index.md`, `<id>.md` sans slug,
> `relationships/`, SVG + Mermaid, pas de ZIP. Les pages revue/Story restent des
> extensions W1c.

```text
/domain.md
/elements/*.md
/interfaces/*.md
/diagrams/*.md
/reviews/*.md
/stories/*.md
/assets/*
```

Un fichier Markdown d’interface inclut identité, domaine responsable, versions, calendrier, contrat, consommateurs, dépendances et liens vers diagrammes.

Le produit peut proposer **Mermaid en complément** pour les vues simples générées automatiquement, mais Mermaid n’est pas le format source des modèles. Le modèle canonique reste le référentiel ; un diagramme Mermaid n’est qu’un export.

### Storytelling

Une Story est une visite guidée versionnée :

```text
Story
  id
  title
  responsibleDomainId
  documentation
  chapters[]
  revision
```

Chaque chapitre contient une séquence de blocs :

```text
StoryBlock =
    MarkdownBlock
  | DiagramBlock
  | ViewBlock
  | ElementBlock
  | InterfaceBlock
  | TransitionBlock
```

`DiagramBlock` peut définir un cadrage graphique : zoom, éléments mis en avant, éléments atténués et texte contextuel. Cela permet d’utiliser le même diagramme plusieurs fois dans une histoire avec des focales différentes.

Scénario type : **« Comment une commande devient une facture »**.

Le premier chapitre montre un processus BPMN « Traiter commande ». Le lecteur sélectionne la tâche « Calculer tarif », qui pointe vers un élément ArchiMate `ApplicationService`. La transition suivante ouvre la vue applicative montrant le composant `Pricing`, puis met en avant l’interface `Pricing API v2`. Le lecteur suit enfin le lien vers un diagramme UML Component présentant l’interface technique et les composants consommateurs. La dernière étape affiche la fenêtre de disponibilité de v1 et v2 et indique que deux consommateurs utilisent encore v1.

Cette Story fonctionne aussi bien en posture conception qu’en posture lecture. En conception, l’auteur modifie les blocs ; en lecture, l’utilisateur déroule le parcours.

L’export Markdown donne un document navigable avec ancres et liens relatifs entre les fichiers. Une Story ne contient pas de copie des objets : elle référence leurs IDs et, si nécessaire, une révision précise pour préserver le récit historique.

## IA, recherche sémantique et choix MCP / MCP Apps / AG-UI / A2UI

### MCP n’est pas un moteur de recherche

Le point le plus important est de ne pas opposer « MCP » et « vectorisation » comme s’ils accomplissaient la même fonction.

MCP est un **protocole pour exposer à une application IA des ressources, outils et capacités**. La spécification MCP actuelle définit notamment les interactions entre hosts, clients et servers au moyen de JSON-RPC ; la version officielle actuelle au moment de cette recherche est `2026-07-28`. citeturn724807search2turn724807search5

MCP ne définit ni moteur de graphes, ni index plein texte, ni base vectorielle. Il peut exposer un outil `search_repository`, mais la façon dont cet outil recherche réellement appartient à l’application.

La recommandation MVP1 est donc :

```text
LLM
 │
 ├── Repository tools
 │      ├── structuredQuery(...)
 │      ├── searchText(...)
 │      ├── traverseGraph(...)
 │      ├── getElement(...)
 │      ├── getInterfaceAtDate(...)
 │      └── proposeChanges(...)
 │
 └── Retrieval
        ├── PostgreSQL / données structurées     SOURCE DE VÉRITÉ
        ├── index full-text                      recherche lexicale
        └── embeddings/pgvector                  recherche sémantique secondaire
```

**Décision : utiliser les deux concepts, mais pas au même niveau.** MCP peut être le protocole standardisé d’exposition des capacités IA ; l’index sémantique complète la recherche mais ne remplace jamais les requêtes déterministes.

Pour MVP1, il n’est pas nécessaire de déployer une base vectorielle séparée. Si PostgreSQL est retenu, un index vectoriel embarqué peut suffire. Les embeddings portent en priorité sur noms, descriptions Markdown, documentation d’interfaces, Stories et descriptions de diagrammes.

Une question telle que :

> « Quelles applications dépendront d’une interface de Paiements expirant avant juin 2027 ? »

ne doit **pas** être résolue par similarité vectorielle. Le LLM doit produire un appel structuré portant sur `providerDomain`, `availableUntil` et les relations de consommation.

En revanche :

> « Où parle-t-on du contrôle de solvabilité client ? »

peut utilement faire l’objet d’une recherche hybride lexical + embeddings, puis d’une résolution des résultats vers les IDs canoniques.

Chaque réponse IA doit afficher ses objets sources :

```text
Pricing API v1      [interface:01J...]
Order Service       [element:01K...]
Billing domain      [domain:01L...]
```

L’absence de source résolue doit être explicite.

### Flux de modification par IA

```text
Demande utilisateur
       ↓
LLM
       ↓
Tools de lecture du référentiel
       ↓
Proposition ChangeSet JSON
       ↓
VALIDATEUR DÉTERMINISTE
       ↓
Preview / diff
       ↓
Confirmation utilisateur
       ↓
Transaction
       ↓
Nouvelle révision + audit
```

Le LLM ne reçoit pas le droit de faire des modifications silencieuses.

Un `ChangeSet` ressemble à :

```json
{
  "operations": [
    {
      "op": "createRelationship",
      "sourceId": "01ABC",
      "targetId": "01DEF",
      "type": "REALIZATION"
    }
  ],
  "reason": "Proposition issue de la demande utilisateur"
}
```

Le backend contrôle métamodèle, cardinalités, dates, existence des IDs et concurrence de version avant toute écriture.

### MCP Apps, AG-UI et A2UI

Il n’y a pas besoin de cumuler ces trois technologies.

**MCP Apps** est depuis le 26 janvier 2026 une extension officielle stable de MCP destinée à permettre à un serveur MCP de fournir une UI HTML interactive à afficher dans un host MCP. L’extension utilise notamment des ressources `ui://`, un affichage sandboxé et une communication host/UI. citeturn952960search2turn952960search3turn952960search5

Ce mécanisme devient intéressant si l’on veut demain faire apparaître une mini-vue d’architecture, un formulaire ou une revue directement **dans un client MCP externe**. Ce n’est pas le besoin fondamental de l’application web elle-même.

**AG-UI 1.0**, publié comme version stable en septembre 2026, standardise plutôt le flux bidirectionnel entre un backend agentique et une application utilisateur : messages streamés, appels d’outils, état partagé et événements associés. citeturn485588search0turn485588search1

**A2UI 1.0** est un protocole de description d’interfaces générées par agent au moyen de structures JSON et de renderers natifs. La version 1.0 a été annoncée le 11 novembre 2026, six jours avant la date de cette étude. citeturn952960search7turn952960search8

La décision v0.1 recommandée est donc :

| Technologie | MVP1 | Justification |
|---|---|---|
| MCP core | **Oui, recommandé côté IA** | interface stable entre LLM et capacités du référentiel |
| MCP Apps | Non | utile surtout pour embarquer l’UI dans un host MCP |
| AG-UI | Optionnel | pertinent si l’on veut standardiser immédiatement le streaming agent ↔ UI |
| A2UI | Non | l’éditeur d’architecture doit rester une UI déterministe, pas une UI générée dynamiquement |
| Embeddings | Oui mais secondaires | amélioration de recherche sémantique, pas source de vérité |
| UI web native | **Oui** | canvas, revue, navigation et édition nécessitent un contrôle précis |

Il faut aussi noter la maturité relative : MCP Apps possède déjà une spécification stable depuis janvier 2026 et AG-UI une 1.0 stable depuis septembre ; A2UI 1.0 vient tout juste d’être publié. Dire que ce dernier est « stable » au sens de sa spécification est exact ; en déduire qu’il dispose déjà du même recul opérationnel serait prématuré. Cette dernière appréciation est une **inférence d’architecture**, pas une affirmation de la norme. citeturn952960search3turn485588search0turn952960search8

## Architecture technique, stockage, API et OpenAPI

L’architecture recommandée reste volontairement compacte pour le MVP1 :

```text
┌─────────────────────────────────────────────┐
│                Web Application              │
│  Atelier de vues | Bibliothèque | Contrôle | Story | AI │
└──────────────────┬──────────────────────────┘
                   │ HTTPS
┌──────────────────▼──────────────────────────┐
│                Application API              │
│ Repository │ Modeling │ Docs │ Review      │
│ Temporal │ Search │ OpenAPI │ AI Gateway   │
└──────────────┬──────────────────┬───────────┘
               │                  │
        ┌──────▼──────┐    ┌──────▼─────────┐
        │ PostgreSQL  │    │ AI Orchestrator│
        │ + FTS       │    │ + MCP façade   │
        │ + vector    │    └────────────────┘
        └─────────────┘
```

Un moteur de graphes dédié n’est pas nécessaire au MVP1. Les relations sont représentables en SQL avec tables d’arêtes et index adaptés. Un CTE récursif suffit pour les parcours d’impact bornés. Une base graphe spécialisée pourra être envisagée si les volumes ou requêtes multi-hop deviennent dominants.

### Modèle physique minimal

| Table | Clés essentielles |
|---|---|
| `domains` | `id`, `name`, `description`, `revision` |
| `objects` | `id`, `object_type`, `responsible_domain_id`, `current_revision_id` |
| `object_revisions` | `id`, `object_id`, `revision_no`, `payload`, `created_at`, `created_by` |
| `model_elements` | `object_id`, `notation`, `metamodel_version`, `metaclass` |
| `relationships` | `object_id`, `source_id`, `target_id`, `relation_type`, `notation` |
| `interfaces` | `object_id`, `logical_interface_key`, `category`, `provider_domain_id` |
| `interface_versions` | `id`, `interface_id`, `semver`, `available_from`, `available_until`, `published_at`, `contract` |
| `interface_consumers` | `interface_version_id`, `consumer_object_id`, `consumer_domain_id` |
| `diagrams` | `object_id`, `notation`, `layout` |
| `diagram_nodes` | `diagram_id`, `element_id`, `geometry`, `style_override` |
| `diagram_edges` | `diagram_id`, `relationship_id`, `geometry` |
| `stories` | `object_id`, `story_structure` |
| `reviews` | `object_id`, `filter_snapshot`, `temporal_window` |
| `comments` | `id`, `target_id`, `author_id`, `markdown`, `created_at` |
| `links` | `source_id`, `target_uri`, `link_type`, `status` |
| `embeddings` | `object_id`, `revision_id`, `chunk_id`, `vector`, `text_hash` |
| `audit_events` | `id`, `actor_id`, `operation`, `object_id`, `before_revision`, `after_revision` |

Les IDs doivent être des identifiants opaques stables, par exemple UUIDv7 ou ULID. Le choix exact est une décision d’implémentation ; la propriété requise est l’indépendance vis-à-vis du nom, de la notation et du domaine.

La concurrence MVP1 utilise un **optimistic lock** :

```http
PUT /api/elements/{id}
If-Match: "rev-17"
```

Une révision concurrente produit `409 Conflict`, avec possibilité de comparer les versions.

### API fonctionnelle

Exemples d’endpoints :

```text
GET    /api/domains
GET    /api/domains/{id}/interfaces
GET    /api/domains/{id}/dependencies

GET    /api/elements/{id}
POST   /api/elements
PUT    /api/elements/{id}

GET    /api/interfaces
POST   /api/interfaces
POST   /api/interfaces/{id}/versions
POST   /api/interfaces/{id}/versions/{version}/publish

GET    /api/interfaces/{id}/impact
GET    /api/interfaces/{id}/timeline
GET    /api/interfaces/{id}/consumers

POST   /api/reviews
GET    /api/reviews/{id}

POST   /api/stories
GET    /api/stories/{id}

GET    /api/search
POST   /api/query/natural-language
POST   /api/ai/proposals
POST   /api/changesets/{id}/validate
POST   /api/changesets/{id}/apply

POST   /api/import/openapi
GET    /api/export/openapi/{interfaceVersionId}
GET    /api/export/markdown/{objectId}
```

### Import/export OpenAPI

OpenAPI définit une description standard et indépendante des langages pour les API HTTP. La branche officielle actuelle contient les familles 3.0, 3.1 et 3.2 ; la version 3.2.1 est datée du 10 septembre 2026. citeturn339674search3turn339674search5

Le MVP1 doit lire :

```text
OpenAPI 3.0.x
OpenAPI 3.1.x
OpenAPI 3.2.x
```

L’import réalise le mapping suivant :

| OpenAPI | Référentiel |
|---|---|
| `info.title` | nom de l’interface |
| `info.version` | `InterfaceVersion.semanticVersion` |
| `servers` | endpoints du contrat |
| `paths` | ressources/opérations |
| `operationId` | identifiant fonctionnel d’opération |
| `parameters` | paramètres de contrat |
| `requestBody` | contrat d’entrée |
| `responses` | contrats de sortie |
| `components.schemas` | objets de données associés |
| `security` | métadonnées contractuelles de sécurité |

Le moteur doit **conserver le document OpenAPI original** en plus du modèle normalisé. Ainsi, les constructions qui ne sont pas comprises par le mapping ne sont pas perdues.

L’idempotence repose sur deux mécanismes : un `sourceFingerprint` calculé sur une représentation canonique du document et des extensions exportables telles que :

```yaml
x-architecture-interface-id: 01JDEF...
x-architecture-version-id: 01JXYZ...
```

Un nouvel import du même document ne crée donc pas une seconde interface.

Si une spécification de même identité change, l’import crée une **proposition de nouvelle version ou de mise à jour** et affiche le diff avant application.

L’export OpenAPI est prévu pour les interfaces de type HTTP seulement. Il ne faut pas fabriquer un faux document OpenAPI pour une interface business ou une interface technique de type messaging.

## Collaboration temps réel (MVP2)

La v0.1 est mono-utilisateur de fait : la persistance locale ne partage rien. Le MVP2
introduit l’édition simultanée **sans rendre un serveur obligatoire**. La synchronisation
commence entre onglets et fenêtres d’un même navigateur ; un relais réseau est branché
plus tard sur le même contrat, sans changer les documents ni les écrans.

### Deux plans, deux sémantiques

| Plan | Objets | Sémantique | En cas de conflit |
|---|---|---|---|
| **Gouvernance** | domaines, éléments, relations, interfaces, versions, contrats, statuts de revue, ChangeSets | écriture unique, verrou optimiste par révision | `409`, dialogue recharger / comparer / forcer. Jamais de fusion automatique |
| **Collaboration** | documentation Markdown, layout de diagramme, blocs de Story, commentaires, présence | CRDT, convergence automatique | fusion déterministe, sans dialogue |

Cette séparation est structurante : un contrat d’interface ne se fusionne pas comme un
paragraphe. Une version publiée reste protégée par une confirmation humaine ; un texte ou
une position de nœud se réconcilie tout seul.

### Exigences

| ID | Exigence MVP2 |
|---|---|
| FR-031 | Afficher la présence : qui est connecté et quel objet est en cours d’édition. |
| FR-032 | Co-éditer la documentation Markdown de tout objet sans perte de frappe ni écrasement. |
| FR-033 | Co-éditer le layout d’un diagramme : déplacements simultanés convergents, sans saut visuel. |
| FR-034 | Co-éditer les blocs de Story et les commentaires, y compris en ajout concurrent. |
| FR-035 | Continuer à travailler hors ligne et fusionner à la reconnexion, sans perte ni duplication. |
| FR-036 | Fonctionner sans serveur : la synchronisation entre onglets d’un même navigateur est le mode nominal du MVP2. |
| FR-037 | Signaler toute fusion qui produit un état non conforme au métamodèle ou aux règles métier, sans rien supprimer automatiquement. |
| FR-038 | Conserver un historique lisible : les rafales d’édition collaborative sont regroupées en révisions, pas une par frappe. |
| FR-039 | Préparer le relais réseau derrière un contrat de transport unique, activable sans modification des écrans. |

### Règles

| ID | Règle |
|---|---|
| CR-001 | Le plan de gouvernance ne transite jamais par le CRDT. |
| CR-002 | Une édition collaborative ne peut pas publier une version d’interface ni clore une revue. |
| CR-003 | Une fusion ne supprime jamais un objet : au pire elle le signale comme non conforme. |
| CR-004 | La présence est éphémère et n’est jamais persistée dans le référentiel. |
| CR-005 | Deux éditions du même champ convergent vers une valeur unique et reproductible. |
| CR-006 | L’état fusionné est identique quel que soit l’ordre d’arrivée des opérations. |
| CR-007 | Un client hors ligne peut produire un état divergent localement, mais il ne peut pas le présenter comme publié. |

### Scénarios d’acceptation

```gherkin
Scenario: Deux fenêtres éditent la même documentation
  Given la documentation de "Payment API" est ouverte dans deux fenêtres
  When chaque fenêtre ajoute un paragraphe distinct
  Then les deux paragraphes sont présents dans les deux fenêtres
  And aucune frappe n'a été perdue

Scenario: Déplacement concurrent d'un nœud
  Given deux fenêtres déplacent le même nœud du diagramme "Paiements — applicatif"
  When les deux déplacements sont synchronisés
  Then les deux fenêtres affichent la même position finale
  And le nœud n'a pas été dupliqué

Scenario: Travail hors ligne puis fusion
  Given une fenêtre est hors ligne depuis plusieurs minutes
  And elle a modifié trois blocs de la Story
  When elle se reconnecte
  Then ses modifications sont fusionnées sans perte
  And l'historique ne contient qu'une révision regroupée

Scenario: Fusion non conforme signalée
  Given deux fenêtres créent simultanément des relations incompatibles entre les mêmes objets
  When la fusion est appliquée
  Then une anomalie "MERGE_VIOLATION" est affichée
  And aucune relation n'est supprimée automatiquement

Scenario: Présence
  Given deux fenêtres sont ouvertes sur le même domaine
  When l'une ouvre la documentation d'un élément
  Then l'autre affiche qui édite cet élément
  And la présence disparaît à la fermeture de la fenêtre
```

> **Amendement MVP2 (décision G1 O-04, option A).** Les relations appartiennent au plan
> gouvernance : la seconde écriture concurrente est refusée par le verrou optimiste ou la
> validation de métamodèle, et aucune relation n'est supprimée. `MERGE_VIOLATION` est
> réservé aux vues collaboratives fusionnées (layout, Story). Aucune relation ne transite
> par la CRDT (`CR-001`). Le scénario est accepté sous cette lecture.

### Exigences non fonctionnelles complémentaires

| ID | Exigence |
|---|---|
| NFR-015 | Convergence garantie : deux replicas ayant reçu les mêmes opérations produisent le même état. |
| NFR-016 | Fonctionnement hors ligne de bout en bout, sans dégradation de l’édition. |
| NFR-017 | Taille d’une mise à jour collaborative bornée : une frappe ne produit pas un envoi du document complet. |
| NFR-018 | Présence non persistée, non historisée, non exportée. |
| NFR-019 | Latence perceptible nulle en local : la frappe s’affiche avant toute synchronisation. |
| NFR-020 | Le coût de la collaboration ne s'applique qu'aux objets effectivement édités (chargement paresseux). |

## Exigences non fonctionnelles, validation et trajectoire après v0.1

Les exigences non fonctionnelles proposées sont des **décisions de conception du produit**, et non des obligations des normes citées.

| ID | Exigence |
|---|---|
| NFR-001 | Toute modification produit une révision traçable par utilisateur et date. |
| NFR-002 | Aucun historique publié ne peut être altéré silencieusement. |
| NFR-003 | Les validations de métamodèle sont déterministes côté serveur. |
| NFR-004 | Une erreur du LLM ne peut contourner les règles métier ou métamodèle. |
| NFR-005 | Temps p95 d’ouverture d’une fiche objet < 1 s pour le volume cible MVP1. |
| NFR-006 | Recherche p95 < 2 s hors génération LLM. |
| NFR-007 | Navigation interactive d’un diagramme ≤ 500 représentations sans blocage perceptible. |
| NFR-008 | Toutes les dates sont stockées comme instants UTC et rendues selon le fuseau utilisateur. |
| NFR-009 | Les exports Markdown sont reproductibles à révision identique. |
| NFR-010 | Une sauvegarde/restauration doit préserver IDs et historique. |
| NFR-011 | Le backend IA ne reçoit que les données nécessaires à la requête. |
| NFR-012 | Les fournisseurs de modèles IA ne doivent pas être considérés comme source de vérité du référentiel. |
| NFR-013 | Les journaux ne doivent pas contenir de contrats complets ou prompts sensibles par défaut. |
| NFR-014 | Le modèle de données doit permettre l’ajout d’un nouveau métamodèle ArchiMate sans changement d’ID des objets existants. |

Les exigences suivantes mesurent la qualité de l’**Atelier de vues** (FR-040 à FR-054). Elles sont bloquantes : une vue lente, modaliste ou illisible invalide la promesse « outil d’architecture ».

| ID | Exigence |
|---|---|
| NFR-021 | Première forme posée en moins de 5 s après l’ouverture d’une nouvelle vue, sans formation préalable. |
| NFR-022 | Créer un élément typé depuis une vue demande au plus trois gestes (déposer, nommer, choisir le type) et zéro changement d’écran. |
| NFR-023 | Aucune création d’objet ne peut exiger un formulaire modal préalable ; l’inspecteur est contextuel et non bloquant. |
| NFR-024 | Enregistrement automatique au repos ≤ 1 s après la dernière modification ; annulation/rétablissement d’au moins 100 actions. |
| NFR-025 | Pan, zoom et déplacement d’une vue de 500 représentations restent fluides (objectif 60 fps, plancher 30 fps), sans re-render global. |
| NFR-026 | Parité clavier complète des gestes de conception : créer, relier, renommer, déplacer, cadrer, publier. |
| NFR-027 | Chaque écran vide propose une action productive immédiate (modèle, import, exemple) et n’affiche jamais un simple « aucune donnée ». |
| NFR-028 | L’état temporel d’une interface est lisible sans recours à la couleur seule et sans ouverture d’une fiche. |
| NFR-029 | La distinction croquis / brouillon formalisé / objet valide est visible en permanence, sans ambiguïté sur ce qui est publié. |
| NFR-030 | Le temps d’ouverture d’une vue existante est < 1 s pour 200 formes en mode local, et son cadrage est restauré à l’identique. |

### Critères d’acceptation Gherkin

```gherkin
Scenario: Réutiliser une interface d'un autre domaine
  Given le domaine "Paiements" a publié l'interface "Payment API"
  And cette interface possède l'id "I-123"
  When un utilisateur du domaine "Commande" ajoute cette interface à son diagramme
  Then aucune nouvelle Interface ne doit être créée
  And le noeud du diagramme référence "I-123"
  And le domaine responsable affiché reste "Paiements"
```

```gherkin
Scenario: Refuser une période invalide
  Given une interface en brouillon
  When l'utilisateur définit availableFrom à "2027-05-01T00:00Z"
  And availableUntil à "2027-04-01T00:00Z"
  Then la publication est refusée
  And l'erreur référence BR-002
```

```gherkin
Scenario: Afficher une interface future
  Given une interface publiée
  And availableFrom est postérieur à maintenant
  When l'utilisateur consulte la revue du domaine fournisseur
  Then l'interface apparaît avec le statut "SCHEDULED"
  And sa date de début est visible
```

```gherkin
Scenario: Détecter un consommateur d'une version expirée
  Given "Commande" consomme "Customer API v1"
  And "Customer API v1" est expirée
  When la revue du domaine "Commande" est ouverte
  Then une anomalie "DEPENDENCY_ON_EXPIRED_INTERFACE" est affichée
  And un lien conduit vers "Customer API v1"
```

```gherkin
Scenario: Rendre une réponse IA vérifiable
  Given la question "Quels systèmes consomment Payment API v1 ?"
  When l'IA répond
  Then chaque système cité possède un identifiant du référentiel
  And la réponse contient l'identifiant de Payment API v1
  And les consommateurs proviennent d'une requête structurée
```

```gherkin
Scenario: Interdire une modification IA silencieuse
  Given une demande "Ajoute la dépendance entre A et B"
  When le LLM produit un ChangeSet valide
  Then le référentiel n'est pas modifié
  Until l'utilisateur confirme explicitement le ChangeSet
```

```gherkin
Scenario: Parcourir une Story
  Given une Story contient un diagramme ArchiMate puis un élément puis un diagramme UML
  When le lecteur suit la transition de l'élément
  Then le second diagramme s'ouvre
  And l'élément lié est mis en évidence
```

```gherkin
Scenario: Réimporter le même OpenAPI
  Given un document OpenAPI a déjà été importé
  When le même document est réimporté
  Then aucune seconde Interface n'est créée
  And le système indique que la source est inchangée
```

```gherkin
Scenario: Créer un élément sans quitter la vue
  Given l'utilisateur ouvre l'Atelier sur une nouvelle vue "Applicatif Paiements"
  When il dépose une forme "Service de paiement" depuis la palette
  And il choisit le type "Application Service" et le domaine "Paiements"
  Then la forme devient un Brouillon formalisé
  And la vue n'a jamais affiché de formulaire modal
  And aucune route n'a changé
```

```gherkin
Scenario: Refuser une relation interdite au moment du geste
  Given une forme "Acteur" et une forme "Nœud technique" dans la même vue
  When l'utilisateur tire un lien entre les deux
  Then seuls les types de relations autorisés par le métamodèle sont proposés
  And une tentative de forcer un type interdit affiche DSG-002
  And aucun objet Relation n'est créé
```

```gherkin
Scenario: Formaliser un croquis avant publication
  Given une forme non typée dans une vue
  When l'utilisateur tente de publier la vue
  Then la publication est refusée avec DSG-001
  And la forme reste visible et éditable
  And le produit indique précisément le type ou le domaine manquant
```

```gherkin
Scenario: Lire la migration de versions dans la vue
  Given une interface "Payment API" avec v1 active et v2 planifiée
  When l'utilisateur place le curseur temporel au 1er août 2027
  Then v2 est rendue active et v1 expirée
  And la période de chevauchement reste signalée
  And chaque état est lisible sans dépendre uniquement de la couleur
```

```gherkin
Scenario: Ouvrir une vue partagée
  Given un lien de vue publiée avec un cadrage
  When un lecteur ouvre ce lien
  Then l'Atelier s'ouvre en posture lecture
  And le cadrage de la vue est appliqué
  And la palette de conception n'est pas affichée
```

### Scénarios de bout en bout à démontrer pour accepter MVP1

La démonstration de recette doit couvrir cinq chaînes complètes. La première est celle qui prouve la nature du produit : elle part d’une vue vide, pas d’un formulaire.

**Chaîne de conception.** Un architecte ouvre l’Atelier, choisit l’intention « Applicatif d’un domaine » et obtient une vue avec bandes et zone. Il dépose trois composants, les relie en 20 secondes, dessine une frontière avec deux ports, nomme les interfaces, formalise les croquis, place le curseur temporel sur la période de migration, publie la vue et partage son lien. Aucun écran d’inventaire n’a été ouvert ; les objets sont pourtant présents dans le référentiel avec leur domaine, leur type et leur historique.

**Chaîne domaine et interface.** Le domaine Paiements crée une vue ArchiMate, publie une `Payment API v1` active du 1er janvier au 30 septembre, et publie v2 à compter du 1er juillet. Commande réutilise ces objets par ID et déclare sa consommation. La revue Paiements montre simultanément v1, v2, consommateurs et période de migration.

**Chaîne multi-notation.** Un Business Process ArchiMate est raffiné par un Process BPMN. Une activité BPMN est liée à un Application Service ArchiMate. Ce service est réalisé par un Component UML et une interface HTTP. La navigation fonctionne dans les deux directions sans fusion des objets.

**Chaîne documentaire.** Une Story décrit un flux métier, traverse trois diagrammes, cite deux interfaces et exporte un ensemble Markdown navigable dont tous les liens sont relatifs et valides.

**Chaîne IA.** L’utilisateur demande : « Qu’est-ce qui casse si Payment API v1 disparaît le 1er juillet ? ». Le système résout l’interface par ID, applique la fenêtre temporelle, traverse les relations de consommation et fournit la liste des objets concernés avec liens source. Une demande suivante « propose-moi une vue de cet impact » génère un ChangeSet de diagramme qui reste non appliqué tant que l’utilisateur ne confirme pas.

### Priorité MVP1 et différé

| Capacité | MVP1 v0.1 | Après MVP1 |
|---|---|---|
| **Atelier de vues (designer)** | **Oui — point d’entrée du produit** | intentions de vue enrichies, cadrages réutilisables |
| Inventaire de type CMDB | Non prioritaire — espace Contrôle | reporting et tableaux de bord |
| Référentiel multi-domaines | Oui | enrichissement |
| ArchiMate | 3.2 réel, cible 4 | migration 4.x officielle |
| BPMN | sous-ensemble | couverture étendue |
| UML | architecture statique | autres diagrammes |
| Interfaces temporelles | Oui | politiques avancées |
| Revue | Oui | workflow formel |
| Documentation Markdown | Oui | templates avancés |
| Storytelling | Oui | présentations enrichies |
| OpenAPI | Oui | autres contrats/protocoles |
| Recherche structurée | Oui | enrichissement |
| Full-text | Oui | enrichissement |
| Embeddings | Oui, secondaire | optimisation RAG |
| MCP | recommandé | exposition plus large |
| MCP Apps | Non | option |
| AG-UI | option technique | possible standardisation |
| A2UI | Non | réévaluation |
| RBAC/ABAC | Non | éventuel futur |
| SSO et connecteurs externes | Non | futur |
| Édition collaborative CRDT | Non (MVP1) | **Oui, MVP2** — voir « Collaboration temps réel » |

### Risques et points à arbitrer

Le risque principal n’est pas la confusion entre **référentiel** et **dessin** : c’est de construire un **référentiel sans surface de conception**, c’est-à-dire une CMDB élégante que personne n’a envie d’ouvrir. Un outil d’architecture se juge à la vitesse à laquelle un architecte peut passer d’une intuition à une vue lisible et partageable. Si le canvas est traité comme un accessoire de rendu placé au-dessus d’un catalogue, les besoins de réutilisation, d’impact, d’IA et de temporalité resteront théoriques, car personne n’aura produit de vues.

La bonne formulation est donc : **le dessin est la surface, le référentiel est la vérité**. L’Atelier est le point d’entrée et le lieu de la création ; chaque forme y est reliée à un objet identifié ou explicitement marquée comme croquis ; le métamodèle valide au moment du geste. Ce couplage fort — geste fluide, validation déterministe — est ce qui distingue Archifier d’un outil de dessin et d’une CMDB à la fois.

Le risque secondaire est l’**austérité prise pour de la sobriété**. Une interface dense, sans grammaire spatiale ni hiérarchie visuelle, ne transmet aucune architecture : elle transmet un formulaire. La vue doit avoir une grammaire (bandes de couches, zones de domaine, frontières, ports, encodage temporel) dès le MVP1, sinon le produit sera perçu comme un back-office.

Un autre risque est le coût d’une « conformité totale » simultanée à trois langages. La v0.1 doit au contraire être stricte sur un sous-ensemble annoncé. BPMN 2.0.2 et UML 2.5.1 disposent tous deux de métamodèles normatifs complets ; cela rend techniquement possible une extension ultérieure sans justifier de tout implémenter immédiatement. citeturn339674search6turn339674search7

Le risque ArchiMate 4 est que l’outil figerait son schéma SQL autour des noms de classes 3.2. La cible 4 est légitime comme exigence produit, mais la seule base normative disponible aujourd’hui reste 3.2 d’après The Open Group. citeturn992156search0turn992156search2

Le risque IA est qu’un RAG vectoriel seul donnerait de mauvaises garanties sur les questions d’impact, d’état ou de temporalité. À l’inverse, un moteur purement structuré sera médiocre pour retrouver un concept exprimé différemment dans des centaines de pages Markdown. D’où l’architecture hybride : **graphe logique + SQL pour la vérité, full-text pour les termes, embeddings pour la similarité, LLM pour l’orchestration et l’explication**.

Enfin, MCP Apps, AG-UI et A2UI répondent à des problèmes voisins mais distincts. MCP Apps transporte des interfaces interactives via l’écosystème MCP ; AG-UI standardise la conversation et les événements entre agent et application ; A2UI décrit des UI générées par l’agent. Pour le cœur d’un outil de modélisation où sélection, déplacement, relations, historique et validation doivent rester prédictibles, une UI web native est la meilleure base du MVP1. citeturn952960search3turn485588search1turn952960search7

Les arbitrages qu’il reste réellement à prendre pour transformer cette v0.1 en backlog sont principalement : niveau exact de granularité des concepts ArchiMate 3.2 exposés dans la première palette ; sémantique précise des relations transverses entre notations ; politique de versionnage des objets non-interface ; distinction éventuelle entre « publié » et « visible » pour les diagrammes et Stories ; conventions de nommage des domaines et interfaces ; seuil de profondeur par défaut pour l’analyse d’impact ; et choix final du format canonique interne des contrats OpenAPI.

Le point de départ recommandé pour la conception détaillée n’est donc pas un catalogue, mais un **Atelier de vues** adossé à un **référentiel sémantique versionné, temporel et multi-notation**. Autour de l’Atelier se branchent la Bibliothèque (retrouver une vue, un domaine, un objet), le Contrôle (anomalies, couverture, inventaires) et le Storytelling (raconter et diffuser les vues). L’IA intervient au-dessus de ce référentiel comme lecteur et générateur de propositions, jamais comme propriétaire de la vérité.