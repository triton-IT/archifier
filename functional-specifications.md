# Spécification fonctionnelle — La carte produit de l’entreprise (cible v0.4, base MVP1)

## Cadre, objectifs et décisions structurantes

Cette spécification décrit la cible **v0.4** d’un référentiel de modélisation d’architecture d’entreprise collaboratif, adossée au MVP1/MVP3 existant. Le produit doit permettre à plusieurs équipes de construire et de posséder leurs produits, de publier les interfaces qu’ils exposent aux autres produits, de réutiliser les objets publiés sans les dupliquer, de documenter l’architecture, de la parcourir de l’entreprise jusqu’à l’infrastructure et de l’interroger en langage naturel.

Le produit possède une surface principale, la **Carte de l’entreprise**, qui est son point d’entrée, et une surface d’édition, l’**Atelier de vues**, qui conserve deux postures : **CONCEPTION**, orientée création et modification, et **LECTURE**, orientée navigation, recherche, consultation historique, comparaison, storytelling et revue des interfaces. Un espace secondaire de **Contrôle** héberge inventaires, anomalies et couverture. Archifier est un outil d’architecture, pas une CMDB : on y navigue en zoomant et on y conçoit en dessinant ; le référentiel se remplit comme conséquence du geste de conception.

Le référentiel est la source de vérité ; une carte ou une vue est une **représentation d’objets du référentiel**. La suppression d’une occurrence graphique ne supprime donc jamais implicitement l’objet qu’elle représente. Inversement, un même objet peut apparaître dans plusieurs cartes, plusieurs vues, plusieurs histoires et plusieurs contextes produits. Cette séparation est ce qui permet à la surface d’être fluide sans que la vérité devienne approximative.

### Statut d’ArchiMate 4

Il faut recadrer explicitement la cible ArchiMate 4. Au **17 novembre 2026**, les sources officielles de The Open Group que j’ai pu vérifier continuent d’indiquer **ArchiMate 3.2 comme dernière version du standard**. Le programme de certification officiel reste nommé « ArchiMate 3 » et The Open Group décrit 3.2 comme la version la plus récente du standard. Je n’ai trouvé aucune spécification ArchiMate 4 publiée officiellement par The Open Group à cette date. citeturn992156search0turn992156search1

La cible doit donc avoir **ArchiMate 4 comme cible explicite**, mais **ne doit inventer aucun métamodèle ArchiMate 4**. L’implémentation initiale doit être conforme au métamodèle ArchiMate 3.2 disponible, avec une architecture permettant de charger ultérieurement un métamodèle `archimate:4.x` sans migration destructive du référentiel.

La décision de conception est donc :

| Sujet | Décision cible |
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

**Hypothèses de la cible.** Une instance correspond à un référentiel d’entreprise. Les utilisateurs sont identifiés individuellement afin d’assurer auteur, historique et commentaires. Tous les utilisateurs ont exactement les mêmes capacités. Une **équipe** est l’unité de responsabilité ; un **domaine métier** est une classification, pas une frontière de sécurité.

**Hors périmètre MVP1.** Il n’existe ni RBAC, ni ABAC, ni workflow d’approbation obligatoire, ni restriction de modification par domaine. Aucun connecteur applicatif tiers n’est fourni, à l’exception de l’import/export OpenAPI. La synchronisation temps réel de type édition simultanée caractère par caractère est **reportée au MVP2** (voir « Collaboration temps réel ») ; le moteur BPMN exécutable, la simulation de processus et la conformité UML/BPMN intégrale restent différés.

**Lecture des sections historiques.** Les sections MVP1 conservées ci-dessous parlent encore de « domaine responsable ». Elles se lisent désormais comme **équipe accountable** pour la propriété et **domaine métier** pour la classification. Les FR-055 à FR-081 et l’ADR-023 prévalent.

## Cible v0.4 — Le produit, la carte et le zoom

> **Décision structurante (ADR-023).** Le point d’entrée du produit n’est plus l’Atelier de vues ni un formulaire de domaine. C’est la **Carte de l’entreprise**, une projection navigable du référentiel qui se zoome de l’entreprise jusqu’à l’infrastructure en passant par les produits, les applications et les interfaces. Le **produit** devient l’unité de lecture et de responsabilité ; le **domaine** redevient ce qu’il aurait toujours dû être : une classification.

Cette section définit la cible. Les sections « Périmètre fonctionnel du MVP1 » et « L’Atelier de vues » restent la base historique de l’Atelier ; elles sont désormais **subordonnées** à la carte et à l’ontologie produit. En cas de conflit, la présente section et l’ADR-023 prévalent.

### 1. Le problème que la cible résout

Le MVP3 a corrigé un défaut réel : le lot 1 s’ouvrait sur un inventaire de domaines et se lisait comme une CMDB. Mais l’accueil est devenu **ultra guidé** : il demande de choisir une intention de vue avant d’avoir vu l’entreprise, puis de nommer un domaine. Ce parcours mélange trois questions distinctes :

| Question de l’utilisateur | Réponse attendue | Ce que le MVP3 proposait |
|---|---|---|
| « À quoi ressemble mon entreprise ? » | une carte, des produits, des flux | sept intentions de vue |
| « Qui est responsable de quoi ? » | des équipes, des produits, des interfaces | un domaine, notion ambiguë |
| « Que se passe-t-il si je change ceci ? » | une traversée produit → application → interface → infrastructure | un canevas à dessiner |

Le résultat est un **guide de création** là où l’utilisateur cherchait une **carte mentale**. La cible inverse le parcours : on voit d’abord l’entreprise, on zoome ensuite, on modifie au niveau où l’on se trouve.

**Ce que la cible supprime.**

- On ne demande plus de créer un domaine avant d’avoir vu l’entreprise.
- On ne propose plus une intention de vue comme préalable : elle devient une lentille ou une action de détail.
- On ne dessine plus la carte à la main : elle est générée depuis le référentiel, avec des ancres stables.
- On ne confond plus produit et application : le produit utilise, l’application est utilisée.
- On ne réduit plus une interface à une flèche : elle a un fournisseur, des consommateurs, des versions et un contrat.

### 2. Ce que la recherche confirme

La cible ne part pas d’une intuition isolée. Elle assemble des pratiques établies, en corrigeant leurs limites.

| Source | Ce qu’elle apporte | Ce qu’elle ne suffit pas à faire |
|---|---|---|
| **ArchiMate 3.2** (The Open Group) | `Product`, `ValueStream`, `Capability`, couches Business/Application/Technology, relations contraintes | `Product` ArchiMate est un composite métier (services + structure passive + contrat), pas une unité de propriété ; il ne compose pas des applications |
| **C4 model** (Simon Brown) | zoom hiérarchique système → conteneur → composant → code, public adapté | s’arrête au logiciel ; ne couvre ni l’entreprise, ni les value streams, ni l’infrastructure physique |
| **Team Topologies** (Skelton & Pais) | équipes stream-aligned, plateforme, enabling, sous-système compliqué ; modes d’interaction | ne dit pas comment rattacher une équipe à un produit ni comment visualiser le SI |
| **TOGAF / Business Architecture Guild** | capability map stable, value stream en étapes, croisement capacité × étape | ne fournit pas de modèle de portefeuille produit ni de navigation |
| **Product operating model / taxonomies produit** | portefeuille → ligne/famille → produit → plateforme ; le produit n’est pas l’application | les taxonomies commerciales ne couvrent pas les applications et les interfaces |
| **Pratiques de catalogue d’API** | interface = fournisseur, consommateurs, versions, cycle de vie, contrat | ne couvre pas les interfaces métier ni les interfaces d’infrastructure |
| **Pratiques de « city map » / paysage applicatif** | géographie stable, pan/zoom, calques et styles, mémoire spatiale | une carte dessinée à la main pourrit ; elle doit être générée depuis le référentiel |

La synthèse est donc : **ArchiMate pour la structure, un modèle natif pour le produit et l’équipe, une carte pour la navigation, des lentilles pour les dimensions, BPMN et UML pour le détail.** Aucune de ces pièces ne remplace les autres.

### 3. Vocabulaire : les distinctions qui manquaient

Le mot « domaine » est utilisé pour désigner au moins quatre choses. La cible les sépare.

| Notion | Question à laquelle elle répond | Définition | Exemple | N’est pas |
|---|---|---|---|---|
| **Produit** (`Product`) | Quelle valeur, pour qui, sous quelle responsabilité ? | Unité durable de valeur offerte à des utilisateurs internes ou externes, possédée par une équipe, avec cycle de vie, capacités, applications, interfaces et données. | « Paiement en ligne » | Une application, un projet, un domaine. |
| **Équipe** (`Team`) | Qui possède et qui opère ? | Unité durable de responsabilité, avec une mission et un type de topologie. | « Équipe Paiements » | Un département, un domaine, un service. |
| **Value stream** (`ValueStream`) | Quel parcours de bout en bout ? | Séquence d’étapes qui produit un résultat pour un client ou un ayant droit. | « Souscrire un contrat » | Un processus BPMN, un produit. |
| **Étape de value stream** (`ValueStreamStage`) | Quelle valeur à quelle étape ? | Étape ordonnée du flux, avec la valeur produite. | « Encaisser » | Une tâche BPMN. |
| **Capacité** (`Capability`) | Que sait faire l’entreprise ? | Aptitude stable et hiérarchique, indépendante de l’organisation. | « Gestion des paiements » | Une fonction, un processus, une application. |
| **Portefeuille** (`Portfolio`) | Comment investir et grouper ? | Regroupement de produits pour l’investissement ou la navigation. | « Portefeuille Clients » | Un value stream. |
| **Application** (`Application`) | Quel système logiciel déployé ? | Système composé de composants, exposant et consommant des interfaces, opéré par une équipe. | « Payment Service » | Un produit. |
| **Composant applicatif** (`ApplicationComponent`) | Quelle partie de l’application ? | Module, service, batch, adaptateur, base ou UI à l’intérieur d’une application. | « Ledger » | Une application. |
| **Interface** (`Interface`) | Quel contrat à quelle frontière ? | Point d’accès versionné, avec fournisseur, consommateurs, contrat et cycle de vie. | « Payment API v2 » | Une flèche, une intégration. |
| **Nœud technique** (`TechnologyNode`) | Sur quoi ça tourne ? | Plateforme, serveur, cluster, réseau, base, environnement. | « Cluster paiement » | Une application. |
| **Domaine métier** (`BusinessDomain`) | Quelle zone de connaissance ? | Classification transversale des produits, capacités et applications. | « Paiements » | Un propriétaire. |
| **Initiative** (`Initiative`) | Quel changement daté ? | Lot de changement (migration, programme, vague). | « Migration v2 » | Un produit. |

Trois distinctions sont **opposables** :

1. **Produit ≠ application.** Un produit peut utiliser plusieurs applications ; une application peut servir plusieurs produits. Une application partagée est un fait architectural, pas une anomalie à masquer.
2. **Équipe ≠ domaine.** L’équipe possède et opère ; le domaine classifie. Un domaine ne peut pas remplacer une équipe accountable.
3. **Value stream ≠ processus.** Le value stream est un flux de valeur agrégé, sans gateway ni exception ; le processus BPMN est sa réalisation détaillée, avec chemins et événements.

**Produit informatique.** Un produit est dit informatique lorsque sa valeur est rendue principalement par du logiciel : application, API, plateforme, données. La cible couvre les produits informatiques **internes** (outils métier, plateformes, services partagés) et **externes** (offres clients, partenaires). C’est le point de départ de la carte : les produits informatiques de l’entreprise, leurs applications, leurs interfaces et leur infrastructure.

Un produit métier qui n’est pas rendu par du logiciel n’est pas un nœud de la carte. Il peut apparaître comme acteur, contexte ou produit ArchiMate dans une vue authored, mais il ne dilue pas la carte des produits informatiques.

**Hiérarchie de portefeuille.** Un portefeuille regroupe des produits pour l’investissement ou la navigation. Une ligne ou famille de produits est un regroupement facultatif, utilisé quand le portefeuille est trop large. La cible s’arrête à **trois niveaux** : portefeuille → produit → sous-produit. Les taxonomies commerciales fines (bundle, variante, SKU) ne sont pas modélisées : elles appartiennent au système de gestion produit, pas à l’architecture d’entreprise.

### 4. Ontologie et règles

```text
Entreprise
 ├─ Unités organisationnelles (optionnel)
 ├─ Équipes ──possède──> Produits
 │            └──opère──> Applications
 ├─ Value streams ──étapes──> [Produits, Capacités]
 ├─ Portefeuilles ──regroupent──> Produits
 ├─ Capacités ──réalisées par──> Produits
 │             └──supportées par──> Applications
 ├─ Produits ──utilisent──> Applications ──composées de──> Composants
 │     ├──exposent/consomment──> Interfaces ──versions──> Contrats
 │     └──contribuent à──> Étapes de value stream
 ├─ Applications ──déployées sur──> Nœuds techniques
 └─ Domaines métier ──classifient──> Produits, Capacités, Applications
```

Les relations et cardinalités exactes sont fixées par le contrat `docs/contracts/C-10-produit-carte-navigation.md`. Les règles fonctionnelles suivantes sont, elles, directement vérifiables en revue.

| ID | Règle |
|---|---|
| `PRD-001` | Un produit a exactement une équipe accountable. |
| `PRD-002` | Un produit contribue à au moins une étape de value stream, ou porte l’anomalie `PRODUCT_WITHOUT_VALUE_STAGE`. |
| `PRD-003` | Un produit réalise au moins une capacité, ou porte l’anomalie `PRODUCT_WITHOUT_CAPABILITY`. |
| `PRD-004` | Un produit peut être décomposé en sous-produits, sur trois niveaux maximum. |
| `PRD-005` | Un produit **utilise** des applications ; il ne les possède pas. |
| `PRD-006` | Une application utilisée par plus d’un produit est marquée `shared` et exposée comme couplage. |
| `PRD-007` | Un produit plateforme expose au moins une interface à d’autres produits. |
| `PRD-008` | Une équipe peut posséder plusieurs produits ; un produit n’a qu’une équipe accountable. |
| `PRD-009` | Une application a exactement une équipe qui l’opère. |
| `PRD-010` | Une équipe peut contribuer à un produit sans le posséder ; la contribution est explicite. |
| `PRD-011` | Une équipe `ENABLING` ne possède pas d’application en propre. |
| `PRD-012` | Une étape de value stream appartient à un seul value stream. |
| `PRD-013` | Une étape est réalisée par un processus BPMN ou décrite par un document ; le lien est `REALIZES` ou `DESCRIBES`. |
| `PRD-014` | Un value stream n’est pas un processus : il agrège des chemins, il ne porte pas de gateway. |
| `PRD-015` | Un produit peut contribuer à plusieurs étapes ; la contribution est datable. |
| `PRD-016` | Une capacité est réalisée par des produits, supportée par des applications ; elle n’est possédée par aucune application. |
| `PRD-017` | Une capacité peut être déclinée, mais la profondeur s’arrête là où la décision de portefeuille ne change plus. |
| `PRD-018` | Une capacité sans produit réalisateur porte l’anomalie `CAPABILITY_WITHOUT_REALIZATION`. |
| `PRD-019` | Le cycle de vie d’une application est distinct de celui du produit qui l’utilise ; l’un ne se déduit pas de l’autre. |
| `PRD-020` | Une application peut servir plusieurs produits ; le produit ne la possède pas. |
| `PRD-021` | Une application est composée de composants, ou est une feuille assumée (COTS/SaaS). |
| `PRD-022` | Une application est déployée sur au moins un nœud, ou porte l’anomalie `APPLICATION_WITHOUT_DEPLOYMENT`. |
| `PRD-023` | Un composant n’est pas déployable seul ; c’est l’application qui l’est. |
| `PRD-024` | Une interface a exactement un fournisseur (produit, application, composant ou nœud). |
| `PRD-025` | Une interface a au moins une version pour être publiée. |
| `PRD-026` | Une consommation référence une **version** d’interface, jamais l’interface nue. |
| `PRD-027` | Une interface `PRODUCT` est exposée par un produit ; une interface `INTERNAL` peut rester interne à une application. |
| `PRD-028` | Une interface applicative peut être exposée au niveau produit sans être dupliquée. |
| `PRD-029` | Une interface d’infrastructure a un fournisseur `TechnologyNode` et n’est jamais `PARTNER` ou `PUBLIC`. |
| `PRD-030` | Une rupture de contrat crée une nouvelle version ; les consommateurs restent sur l’ancienne jusqu’à migration explicite. |
| `PRD-031` | Un domaine métier ne peut pas être propriétaire ; l’équipe accountable reste requise. |
| `PRD-032` | Un produit sans application est autorisé en découverte, mais signalé comme incomplet. |
| `PRD-033` | Un produit a un **value stream principal** (ou un portefeuille de repli) qui détermine son quartier d’accueil à L0. Il n’apparaît qu’une fois sur la carte ; ses contributions aux autres value streams sont des liens de contexte, jamais des duplications. |

**Heuristique d’alignement équipe / produit.** Un produit `CUSTOMER` ou `INTERNAL` est généralement possédé par une équipe **stream-aligned** ; un produit `PLATFORM` par une équipe **plateforme** ; un produit `DATA` par une équipe plateforme ou stream-aligned ; une équipe **enabling** ne possède pas de produit en propre, elle aide les autres à élever leur capacité. Ce n’est pas une contrainte de validation, mais un signal de cohérence que la carte peut afficher.

### 5. La carte de l’entreprise

#### 5.1 Définition

La carte est une **projection générée, à géographie stable, du référentiel**. Elle n’est :

- ni un diagramme authored : on ne la dessine pas à la main ;
- ni un inventaire : elle ne présente pas de table comme contenu principal ;
- ni un tableau de bord : les compteurs sont des symptômes, pas le contenu.

Elle est une **carte** au sens où elle donne des repères stables, se parcourt au zoom et accepte des calques. La métaphore urbaine est une grammaire de lecture, pas une décoration :

| Élément de carte | Objet du référentiel | Rôle |
|---|---|---|
| **Quartier** | Value stream, portefeuille ou unité d’organisation | Périmètre navigable |
| **Bâtiment** | Produit | Unité de valeur et de responsabilité |
| **Bloc / pièce** | Application | Système logiciel à l’intérieur du produit |
| **Étage / local** | Composant applicatif | Partie de l’application |
| **Utilitaire** | Produit plateforme, nœud partagé | Capacité transverse |
| **Route** | Interface ou flux agrégé | Dépendance entre fournisseur et consommateur |
| **Port** | Interface exposée | Point d’entrée sur une frontière |
| **Panneau** | Label, état, criticité, propriétaire | Information lisible sans ouvrir de fiche |

La carte ne crée pas de concept métier : un quartier n’est pas un objet, c’est le rendu d’un value stream ou d’un portefeuille. Un bâtiment est un produit ; un bloc est une application. Un bloc est dessiné dans la frontière d’un produit parce qu’il est **utilisé dans ce contexte**, pas parce qu’il appartient au produit : une application partagée apparaît dans plusieurs produits, et c’est précisément ce que la carte doit rendre visible. Cette discipline évite la « carte jolie mais fausse ».

#### 5.2 Les cinq niveaux

| Niveau | Nom | Périmètre typique | Ce que l’on voit | Ce que l’on ne voit pas | Clic |
|---|---|---|---|---|---|
| **L0** | Entreprise | Toute l’entreprise | Value streams/quartiers, produits, produits plateformes, flux produit agrégés, criticité | Applications individuelles, composants, nœuds | Quartier → L1, produit → L2, route → panneau d’interfaces |
| **L1** | Portefeuille / quartier | Un value stream, un portefeuille, une BU | Produits du périmètre, plateformes partagées, interfaces produit à produit, capacités en chaleur | Composants, infrastructure | Produit → L2 |
| **L2** | Produit | Un produit | Frontière, ports, applications, interfaces applicatives, produits externes, dépendances | Composants internes, nœuds individuels | Application → L3, port → interface |
| **L3** | Application | Une application | Composants, services, interfaces, données, déploiement | Produits, value streams | Composant → détail, nœud → L4 |
| **L4** | Infrastructure | Un nœud, un environnement, une zone réseau | Nœuds, environnements, réseaux, applications déployées, interfaces techniques | Composants métier | Nœud → détail, application → L3 |

Le zoom est **sémantique** : franchir un seuil change la requête et l’agrégation, pas seulement l’échelle du dessin. Il est aussi **géographique** : le cadrage reste continu, l’objet cliqué reste au même endroit, le fil d’Ariane situe le niveau courant.

À L1, la **liste des produits du périmètre** est toujours accessible (panneau latéral ou mode liste) avec nom, équipe, cycle de vie, criticité, nombre d’applications et d’interfaces. La carte reste le contenu principal ; la liste est un index de lecture, pas un tableau de bord.

Lorsque le quartier est un value stream, ses étapes forment une **bande de lecture** en haut du quartier ; les produits restent dans une zone unique et sont reliés aux étapes par des arêtes de contribution. Un produit n’est jamais dupliqué dans plusieurs bandes : sa contribution multiple se lit par ses arêtes et ses badges.

#### 5.3 Géographie stable

- Chaque objet reçoit une **ancre** (`MapAnchor`) par niveau. Une ancre est persistée ; elle n’est pas le layout d’une vue authored.
- Un objet sans ancre reçoit une position déterministe par une heuristique de placement. Deux sessions produisent la même carte.
- Un produit n’apparaît **qu’une fois** à L0, dans son value stream principal (ou son portefeuille de repli). Ses contributions aux autres value streams sont des liens de contexte, jamais des duplications. Sans value stream principal, il est placé dans « Non rattaché » et signalé.
- Déplacer un objet est une **édition explicite** : l’ancre devient `pinned=true`.
- Changer de lentille, de filtre ou de date **ne déplace rien**. C’est la garantie de la mémoire spatiale.
- Les nouveaux objets sont placés dans leur quartier ou leur frontière, avec évitement de collision visuel ; le déplacement manuel reste possible.
- Un conflit d’ancre est résolu visuellement, jamais par une écriture implicite.

#### 5.4 Arêtes et agrégation

Une route représente un ensemble de consommations d’interfaces. Elle n’est pas une flèche anonyme :

- à L0, une route agrège les interfaces entre les applications de deux produits ;
- à L1, elle agrège les interfaces du périmètre et distingue les plateformes ;
- à L2, elle montre les interfaces applicatives réelles et les ports du produit ;
- à L3, elle montre les interfaces de composants ;
- à L4, elle montre les interfaces techniques et les déploiements.

Toute route agrégée est **dépliable** : le clic liste les interfaces, versions, fournisseurs, consommateurs et états. Une route non dépliable est un défaut (`MAP-005`). Une arête stratégique saisie manuellement est marquée comme telle et ne remplace jamais une arête dérivée.

#### 5.5 Contexte, focus, profondeur

- Le périmètre courant est pleinement rendu ; les objets externes restent visibles en **contexte atténué** pour montrer les dépendances sortantes.
- Un objet sélectionné met en évidence ses chemins entrants et sortants, sans quitter la carte.
- Le fil d’Ariane est toujours visible : `Entreprise › Commande client › Paiement › Payment Service › Cluster paiement`.
- Une mini-carte situe le viewport ; la recherche et la palette de commandes permettent de sauter à un objet et de zoomer au bon niveau.
- Une vue « liste » est disponible pour l’accessibilité et le contrôle, mais elle n’est jamais le contenu principal.

#### 5.6 Lire et éditer sur la carte

La carte a deux postures, comme l’Atelier :

- **Lecture** : chrome réduit, pas de poignées, navigation par zoom, panneaux contextuels, liens profonds.
- **Édition** : création et modification **sur place** d’un produit, d’une équipe, d’une application, d’une interface, d’un rattachement de capability ou d’une étape de value stream. Aucun formulaire modal, aucun changement de route obligatoire.

Créer depuis la carte reste un geste explicite : une forme vide n’est pas un objet ; un clic sur « Nouveau produit » ouvre un panneau latéral non bloquant, avec nom, équipe, type et cycle de vie. La formalisation peut être différée, la publication non.

### 6. Les lentilles : plusieurs dimensions, une géographie

Une **lentille** est une projection du même graphe. Elle répond à une question et encode une dimension. Les lentilles se **superposent** : couleur par équipe, arêtes par interface, filtre par cycle de vie.

| Lentille | Question | Encodage principal | Arêtes | Niveaux |
|---|---|---|---|---|
| **Valeur** | Quel parcours, quelles étapes, quels produits ? | Quartiers = value streams, étapes = bandes | Contribution produit → étape | L0, L1, L2 |
| **Capacité** | Quelles capacités, couvertes par quoi ? | Chaleur de couverture et de maturité | Réalisation, support | L0, L1, L2 |
| **Produit** | Quel portefeuille, quel cycle de vie, quelle criticité ? | Couleur = lifecycle/criticality | Dépendances produit | L0, L1, L2 |
| **Organisation** | Qui possède quoi, où sont les dépendances ? | Couleur = équipe, frontières d’équipe | Interfaces inter-équipes | L0, L1, L2 |
| **Applicatif** | Quelles applications, quels composants, quel âge ? | Couleur = type/âge | Interfaces applicatives | L1, L2, L3 |
| **Interface** | Qui expose/consomme quoi, quelles versions ? | Ports + routes, filtre catégorie/état | Consommations | L0, L1, L2, L3, L4 |
| **Technologie** | Quelles plateformes, quels nœuds, quelle obsolescence ? | Couleur = type/âge/environnement | Déploiement, réseau | L2, L3, L4 |
| **Données** | Quels objets, quels flux, quelle classification ? | Couleur = classification | Échanges de données | L1, L2, L3 |
| **Temps** | Où en est-on à T, que se passe-t-il ? | Motif = scheduled/active/expired | Arêtes datées | Tous |
| **Risque / coût** | Où sont les risques, coûts, dettes ? | Chaleur | Propagation de risque | L0, L1, L2, L3 |

Règles :

- Au moins une lentille est active ; la lentille par défaut est `PRODUIT` à L0/L1 et `INTERFACE` à L2/L3.
- Une lentille ne modifie jamais la position, la taille ou la hiérarchie.
- Une lentille indisponible au niveau courant est grisée avec sa raison, pas silencieusement retirée.
- La légende est toujours accessible et nomme la lentille, l’encodage et les états.
- Les sept intentions de l’Atelier deviennent des **lentilles ou des actions de détail** : « Applicatif » → lentille applicative ; « Processus » → ouverture BPMN d’une étape ; « Revue d’interface » → lentille interface. Elles ne sont plus un préalable à la création d’un domaine.

### 7. Les pages : du produit à l’infrastructure

Chaque page est une **carte à son niveau**, pas un formulaire. Un panneau latéral porte les propriétés ; le contenu principal reste spatial.

#### 7.1 Page produit

- **Bandeau** : nom, type, cycle de vie, criticité, équipe accountable, value streams, domaine métier, complétude.
- **Carte L2** : frontière du produit, ports d’interfaces exposées, applications à l’intérieur, produits/applications externes en contexte, socle technique en bas.
- **Sections** : Vue d’ensemble · Valeur & clients · Capacités · Applications · Interfaces · Données · Équipe & organisation · Cycle de vie & roadmap · Risques & conformité · Vues, Stories, décisions.
- **Actions** : ajouter une application, exposer une interface, rattacher une capacité, changer le cycle de vie, ouvrir une vue authored, lancer une analyse d’impact.
- **État vide** : « Ce produit n’a pas encore d’application. Ajoutez la première, ou rattachez-en une existante. »

#### 7.2 Page équipe

- **Bandeau** : nom, type de topologie, mission, unité d’organisation.
- **Carte** : produits possédés, applications opérées, interfaces fournies et consommées, dépendances vers d’autres équipes.
- **Sections** : Produits · Applications · Interfaces · Dépendances · Modes d’interaction · Charge & criticité.
- **Signal** : une équipe qui possède trop de produits critiques ou trop d’applications partagées est un signal de charge cognitive, pas une faute.

#### 7.3 Page value stream

- **Bandeau** : nom, client, proposition de valeur, équipe propriétaire.
- **Carte L1** : étapes de gauche à droite, produits contributeurs, capacités habilitantes, interfaces critiques, applications de support.
- **Sections** : Étapes · Produits · Capacités · Interfaces critiques · Processus BPMN · Indicateurs · Initiatives.
- **Action** : ouvrir le processus BPMN d’une étape, ou créer le lien `REALIZES`.

#### 7.4 Page capacité

- **Bandeau** : nom, niveau, maturité, criticité.
- **Carte** : hiérarchie de capacités, chaleur de couverture, produits réalisateurs, applications de support, étapes de value stream habilitées.
- **Deux lectures** : la carte des produits qui réalisent la capacité, et la **capability map** hiérarchique (grille) qui montre la décomposition et la couverture. Les deux lectures partagent le même modèle ; aucune ne duplique les objets.
- **Sections** : Décomposition · Couverture · Produits · Applications · Value streams · Écarts et duplications · Roadmap.
- **Usage** : identifier les capacités sans produit, les duplications d’applications et les investissements à arbitrer.

#### 7.5 Page application

- **Bandeau** : nom, type, cycle de vie, criticité, équipe opératrice, produits servis, partagée ou non.
- **Carte L3** : composants, services, interfaces, données, déploiement, consommateurs.
- **Sections** : Vue d’ensemble · Composants · Interfaces · Données · Déploiement · Produits servis · Consommateurs · Risques & dette · Historique.
- **Notations** : ArchiMate par défaut ; UML Component, Class ou Sequence pour le détail.

#### 7.6 Page infrastructure

- **Bandeau** : nom, type, environnement, équipe, cycle de vie, criticité.
- **Carte L4** : nœuds, réseaux, environnements, applications déployées, interfaces techniques.
- **Sections** : Nœuds · Applications déployées · Interfaces techniques · Résilience · Obsolescence · Coûts · Incidents.
- **Usage** : répondre à « qu’est-ce qui tourne où ? », « qui dépend de ce nœud ? », « qu’est-ce qui est obsolète ? ».

#### 7.7 Page interface

La page interface existante est conservée et enrichie :

- fournisseur et consommateurs **objets** (produit, application, composant, nœud), et non seulement des domaines ;
- niveau, catégorie, type, exposition, criticité, classification de données ;
- versions, calendrier, contrat, consommateurs par version, impact, historique ;
- lien vers le produit qui expose l’interface et vers les applications qui la consomment.

### 8. Les interfaces, à tous les niveaux

Une interface est un **objet de gouvernance**, pas une arête. Elle existe à trois niveaux :

| Niveau | Frontière | Exemple | Binding possible |
|---|---|---|---|
| **BUSINESS** | Produit / partenaire | « Demander un remboursement » | Business Service / Business Interface / Product |
| **APPLICATION** | Application / composant | « Payment API » | Application Service / Application Interface + OpenAPI |
| **INFRASTRUCTURE** | Nœud / réseau / plateforme | « Flux événements orders.v1 » | Technology Service / Technology Interface |

Propriétés obligatoires : fournisseur, niveau, catégorie, type, versions, exposition, criticité. Les consommateurs sont optionnels mais suivis : une interface sans consommateur connu est un signal, pas une erreur.

Règles de lecture :

- **Interface produit** : une interface `exposure=PRODUCT` est visible sur la frontière du produit et sur la carte L2. Elle n’est pas dupliquée depuis l’interface applicative : c’est la même interface, exposée.
- **Interface applicative** : elle existe entre applications ou composants. Elle peut être interne, exposée au produit, ou consommée par un autre produit.
- **Interface d’infrastructure** : elle relie des nœuds ou des applications à des nœuds. Elle n’est jamais exposée à un partenaire.
- **Consommation** : elle référence une version. Une consommation d’une version expirée est une anomalie visible (`DEPENDENCY_ON_EXPIRED_INTERFACE`).
- **Agrégation** : une route de carte agrège des consommations ; elle est toujours dépliable vers les interfaces et versions réelles.
- **Contrat** : OpenAPI pour HTTP, AsyncAPI ou autre pour les événements ; aucun faux contrat OpenAPI pour une interface non HTTP.

### 9. ArchiMate, BPMN, UML : la bonne notation au bon niveau

| Niveau | Notation par défaut | Notations de détail | Ce que l’on gagne |
|---|---|---|---|
| **L0 Entreprise** | ArchiMate (Business, Strategy) | — | Value streams, produits, capacités, flux |
| **L1 Portefeuille** | ArchiMate | — | Produits, plateformes, interfaces produit |
| **L2 Produit** | ArchiMate (Business + Application) | BPMN pour un processus de produit | Applications, services, ports, interfaces |
| **L3 Application** | ArchiMate (Application) | UML Component / Class / Sequence | Composants, services, données, contrats |
| **L4 Infrastructure** | ArchiMate (Technology) | UML Deployment | Nœuds, réseaux, artefacts, déploiement |

Règles :

- **ArchiMate est la colonne vertébrale.** La carte ne mélange pas les notations au même niveau : elle utilise ArchiMate pour la structure, puis ouvre BPMN ou UML dans un panneau ou une vue de détail.
- **BPMN n’apparaît pas à L0/L1.** Il s’ouvre depuis une étape de value stream ou un processus de produit, avec un lien `REALIZES` ou `DESCRIBES`.
- **UML n’apparaît pas à L0/L1.** Il s’ouvre depuis une application, un composant ou une interface, avec un lien `IMPLEMENTS` ou `REALIZES`.
- **Un objet, plusieurs représentations.** Un composant applicatif et un composant UML restent deux objets distincts reliés par une relation native ; ils ne fusionnent jamais silencieusement.
- **Le métamodèle valide.** Une relation interdite est refusée avant création, même si elle est visuellement pratique.

> **Produit natif et produit ArchiMate.** Le `Product` natif ne remplace pas l'élément ArchiMate `Product`. L'élément ArchiMate porte la composition de services, le contrat et la sémantique de l'offre ; le produit natif porte la propriété (équipe), le cycle de vie, les applications, les interfaces et les capacités. Les deux sont reliés par `REALIZES` ou `REFINES`, jamais fusionnés. Une carte ne montre qu'un produit natif ; une vue authored peut montrer l'élément ArchiMate correspondant.

> **Application native et composant ArchiMate.** Une `Application` native est portée par un ou plusieurs `ApplicationComponent` ArchiMate ; un composant natif correspond à un `ApplicationComponent` (ou à un service exposé). L'application native porte l'équipe opératrice, le cycle de vie, les produits servis et les interfaces ; l'élément ArchiMate porte la structure et les relations. Aucun objet n'est dupliqué : la carte agrège, la vue authored détaille.

> **Note de profil (écart E-04).** Le profil `archimate:3.2` livré au MVP3 ne contient pas la couche Strategy : `Capability` et `ValueStream` n’y sont pas encore disponibles, alors qu’ils font partie d’ArchiMate 3.2. La cible doit **compléter le profil** avec ces classes officielles avant de présenter les lentilles Valeur/Capacité comme des bindings ArchiMate. En attendant, `Capability` et `ValueStream` existent comme objets **natifs** et sont rendus par le modèle natif : aucune classe n’est inventée, aucune donnée n’est perdue. Voir `technical-specifications-lot3.md` §E-04.

### 10. Le parcours de zoom : de la carte au contrat

Le parcours de référence, démontrable en recette :

1. **J’ouvre `/`.** Je vois la carte de l’entreprise : des quartiers (value streams), des bâtiments (produits), des utilités (plateformes), des routes (flux).
2. **Je clique sur le quartier « Commande client ».** La carte zoome en L1 : les produits du value stream, leurs interfaces, les plateformes partagées.
3. **Je clique sur le produit « Paiement ».** La carte zoome en L2 : la frontière du produit, ses ports, ses applications, ses dépendances externes.
4. **Je clique sur le port « Payment API v2 ».** Le panneau interface s’ouvre : fournisseur, consommateurs, versions, contrat, calendrier.
5. **Je clique sur un consommateur « Checkout ».** La carte zoome en L3 : les composants de l’application, ses interfaces, ses données, son déploiement.
6. **Je clique sur un nœud « Cluster paiement ».** La carte zoome en L4 : les applications déployées, les interfaces techniques, l’environnement.
7. **Je change de lentille.** « Organisation » colore par équipe et montre les dépendances inter-équipes ; « Temps » montre la migration v1 → v2 ; « Capacité » montre la couverture. La géographie ne bouge pas.
8. **Je demande un impact.** « Que casse la fin de Payment API v1 ? » La carte surligne les consommateurs, les produits impactés, les équipes et les échéances, avec un chemin vers chaque objet.

Aucun écran d’inventaire n’a été ouvert. Le référentiel s’est pourtant enrichi de produits, d’équipes, d’applications, d’interfaces et de liens.

### 11. Migration depuis le modèle MVP3

| Objet MVP3 | Devient | Règle |
|---|---|---|
| `Domain` | `BusinessDomain` | Conservé comme classification ; ne possède plus d’objets. |
| `responsibleDomainId` | `businessDomainId` | Conservé pour l’historique ; les nouveaux objets utilisent `accountableTeamId`. |
| Page `/library/domains/:id` | `/business-domains/:id` | Ancienne route conservée en alias. |
| `ViewIntent` | `ViewIntent` (Atelier) + `MapLens` (Carte) | Aucune fusion ; les deux catalogues sont distincts. |
| Interface `providerDomainId` | `providerObjectId` + `businessDomainId?` | Le fournisseur peut être un produit, une application, un composant ou un nœud. |
| Diagramme authored | Vue authored | Inchangé ; il reste éditable dans l’Atelier. |
| Carte | Nouvel objet de projection | Pas de migration de layout ; la carte a ses propres ancres. |

La migration est **guidée et additive** : le produit propose de rattacher un ancien domaine à une équipe et à un produit, mais ne supprime jamais l’objet ni son historique. Un domaine sans équipe reste visible avec l’anomalie `PRD-031`.

### 12. Exigences de la cible

Ces exigences complètent FR-001 à FR-054. Elles prévalent en cas de conflit avec l’entrée « Atelier ».

| ID | Exigence |
|---|---|
| FR-055 | Ouvrir `/` sur la **Carte de l’entreprise**, et non sur un inventaire, un tableau de bord ou une galerie d’intentions. |
| FR-056 | Naviguer de l’entreprise au value stream/portefeuille, puis au produit, à l’application et à l’infrastructure par zoom sémantique, avec fil d’Ariane et lien profond. |
| FR-057 | Gérer les **produits** comme objets de premier rang : type, cycle de vie, criticité, équipe accountable, value streams, capacités. |
| FR-058 | Gérer les **équipes** comme objets de premier rang : topologie, mission, produits possédés, applications opérées, interfaces fournies/consommées. |
| FR-059 | Gérer les **value streams** et leurs étapes, avec produits contributeurs, capacités habilitantes et processus BPMN liés. |
| FR-060 | Gérer les **capacités** comme une hiérarchie stable, avec réalisation par les produits, support par les applications, couverture et écarts. |
| FR-061 | Gérer les **applications** comme systèmes utilisés par des produits, composés de composants, déployés sur des nœuds. |
| FR-062 | Afficher les **interfaces** à tous les niveaux, avec fournisseur, consommateurs, niveau, type, version, exposition, criticité et état temporel. |
| FR-063 | Proposer des **lentilles** (valeur, capacité, produit, organisation, applicatif, interface, technologie, données, temps, risque) qui changent l’encodage et les filtres sans changer la géographie. |
| FR-064 | Appliquer un **curseur temporel** à la carte, aux produits, aux applications, aux interfaces et aux initiatives. |
| FR-065 | Créer et modifier un produit, une équipe, une application, une interface, une étape ou un rattachement de capacité **depuis la carte**, sans modal ni changement de route obligatoire. |
| FR-066 | Fournir une **page produit** centrée sur une carte L2 et un panneau de valeur, capacités, applications, interfaces, données, risques, équipe et roadmap. |
| FR-067 | Fournir une **page équipe** avec produits, applications, interfaces, dépendances et charge. |
| FR-068 | Fournir une **page value stream** avec étapes, produits, capacités, interfaces critiques et processus BPMN. |
| FR-069 | Fournir une **page capacité** avec maturité, couverture, duplications, écarts et roadmap. |
| FR-070 | Fournir une **page application** avec composants, interfaces, données, déploiement, produits servis et consommateurs. |
| FR-071 | Fournir une **page infrastructure** avec nœuds, environnements, réseaux, applications déployées et interfaces techniques. |
| FR-072 | Fournir une **analyse d’impact** traversant produit, application, interface, équipe et infrastructure, avec chemins et effets temporels. |
| FR-073 | Utiliser **ArchiMate** comme notation par défaut, **BPMN** pour le détail des processus et **UML** pour le détail des applications et interfaces ; ne jamais imposer une notation à un niveau où elle n’apporte pas de précision. |
| FR-074 | Garantir la **traçabilité inter-notations** par des relations natives, sans duplication d’objets. |
| FR-075 | Importer produits, applications et interfaces depuis CSV/JSON/OpenAPI et réconcilier sans doublon. |
| FR-076 | Exporter une carte (SVG/Markdown) avec niveau, lentille, date, légende et sources. |
| FR-077 | Accompagner le premier lancement par un parcours qui crée un **premier produit** et une **première équipe**, pas un domaine ni une intention. |
| FR-078 | Permettre à la recherche et à la palette de commandes de sauter à tout objet et de zoomer au bon niveau. |
| FR-079 | Signaler les anomalies de modèle : produit sans équipe, application sans opérateur, interface sans fournisseur ou version, consommation d’une version expirée, application partagée non marquée, capacité sans réalisation, produit orphelin, domaine utilisé comme propriétaire. |
| FR-080 | Garantir l’accessibilité de la carte : navigation clavier entre niveaux, liste alternative, lecteur d’écran, réduction de mouvement. |
| FR-081 | Capturer une carte (périmètre, niveau, lentilles, date, focus) dans une Story ou une Revue, avec sa révision, sans dupliquer les objets. |

#### 12.1 Révision des exigences existantes

| Exigence | Lecture MVP3 | Lecture cible v0.4 |
|---|---|---|
| FR-001 | Créer, modifier, archiver et consulter des domaines. | Créer des produits, équipes, value streams et applications ; le domaine devient une classification. |
| FR-002 | Affecter un `responsibleDomainId`. | Affecter un `accountableTeamId` et, facultativement, un `businessDomainId`. |
| FR-040 | L’Atelier est le point d’entrée. | La Carte est le point d’entrée ; l’Atelier est la surface d’édition des vues authored. |
| FR-046 | Démarrer une vue depuis une intention. | La carte se lit par niveau et lentille ; une vue authored garde une intention. |
| FR-048 | Chaque vue a une intention, un périmètre, une notation, un cadrage. | La carte a un niveau, un périmètre, des lentilles, une date et des ancres ; la vue authored garde son identité. |
| FR-052 | L’inventaire est secondaire. | Inchangé : la carte n’est pas un inventaire ; le Contrôle reste secondaire. |
| FR-054 | Onboarding Atelier en moins de trois minutes. | Onboarding Carte : premier produit et première équipe en moins de trois minutes. |

### 13. Scénarios d’acceptation

```gherkin
Scenario: Ouvrir la carte de l'entreprise
  Given un référentiel contenant des value streams, des produits et des équipes
  When l'utilisateur ouvre "/"
  Then la Carte de l'entreprise s'affiche
  And aucun tableau d'inventaire n'est le contenu principal
  And aucun formulaire de domaine n'est demandé
```

```gherkin
Scenario: Zoomer de l'entreprise à l'infrastructure
  Given la carte est ouverte au niveau ENTERPRISE
  When l'utilisateur clique sur "Commande client", puis "Paiement", puis "Payment Service", puis "Cluster paiement"
  Then le fil d'Ariane affiche les cinq niveaux
  And chaque niveau montre uniquement les objets de son niveau
  And le retour au niveau précédent conserve le cadrage
```

```gherkin
Scenario: Lire les interfaces d'un produit
  Given le produit "Paiement" expose "Payment API v2"
  And l'application "Checkout" consomme "Payment API v2"
  When l'utilisateur ouvre la carte L2 du produit
  Then "Payment API v2" apparaît comme port sur la frontière
  And la route vers "Checkout" est visible
  And le clic sur la route liste l'interface, sa version et son consommateur
```

```gherkin
Scenario: Changer de lentille sans perdre la géographie
  Given la carte L1 du value stream "Commande client"
  When l'utilisateur passe de la lentille PRODUIT à la lentille ORGANISATION
  Then les produits sont colorés par équipe
  And aucun produit n'a changé de position
```

```gherkin
Scenario: Créer un produit depuis la carte
  Given la carte L1 d'un value stream
  When l'utilisateur choisit "Nouveau produit"
  And nomme le produit, choisit son équipe et son type
  Then le produit apparaît sur la carte
  And aucun formulaire modal n'a été ouvert
  And la route n'a pas changé
```

```gherkin
Scenario: Ouvrir un processus BPMN depuis une étape de value stream
  Given l'étape "Encaisser" est réalisée par un processus BPMN
  When l'utilisateur ouvre l'étape
  Then le processus BPMN s'ouvre dans une vue de détail
  And le lien REALIZES est conservé
  And aucun objet BPMN n'est dupliqué
```

```gherkin
Scenario: Ouvrir un composant UML depuis une application
  Given l'application "Payment Service" a un composant applicatif "Ledger"
  And ce composant est implémenté par un composant UML
  When l'utilisateur ouvre le composant
  Then le composant UML s'ouvre dans une vue de détail
  And le lien IMPLEMENTS est conservé
```

```gherkin
Scenario: Analyser l'impact d'une fin de version
  Given "Payment API v1" est consommée par "Checkout"
  And "Payment API v1" expire le 30 juin 2027
  When l'utilisateur demande l'impact au 1er juillet 2027
  Then "Checkout" est signalée comme impactée
  And le produit "Paiement" et l'équipe qui opère "Checkout" sont cités
  And chaque impact possède un lien vers l'objet source
```

```gherkin
Scenario: Migrer un domaine legacy
  Given un domaine "Paiements" possède des éléments dans le référentiel MVP3
  When la migration est lancée
  Then le domaine devient un domaine métier
  And l'utilisateur peut le rattacher à une équipe et à un produit
  And aucun objet ni historique n'est supprimé
  And l'ancienne route reste résolvable
```

### 14. Trajectoire

| Vague | Contenu | Critère de sortie |
|---|---|---|
| **V0.4.0 — Socle produit et carte** | Ontologie Product/Team/ValueStream/Capability/Application/Interface ; carte L0–L2 ; pages produit, équipe, value stream, interface ; lentille interface ; création inline ; import CSV/JSON/OpenAPI ; onboarding premier produit. | Un utilisateur ouvre `/`, voit la carte, zoome jusqu'à une interface et crée un produit sans formulaire. |
| **V0.4.1 — Profondeur et dimensions** | L3–L4 ; page application et infrastructure ; lentilles capacité, organisation, temps, risque ; analyse d'impact ; export de carte. | Le parcours entreprise → infrastructure est complet et l'impact d'une version est démontrable. |
| **V0.5 — Détail et récit** | BPMN depuis les étapes ; UML depuis les applications ; Stories et Revues ancrées sur la carte ; lentilles données et coût ; collaboration. | Une Story traverse carte, BPMN et UML sans dupliquer les objets. |
| **Différé** | Découverte automatique, synchronisation CMDB, BPMN exécutable, UML complet, RBAC/ABAC, portail externe. | Décision ultérieure. |

### 15. Risques et arbitrages

| Risque | Mitigation |
|---|---|
| **La carte devient un bel inventaire.** | Projection générée, clic profond, édition inline, aucune table comme contenu principal. |
| **Produit et application se confondent.** | `PRD-005` et `PRD-020` : le produit utilise, ne possède pas ; l'application partagée est un signal. |
| **Le modèle devient trop lourd.** | Trois niveaux de produit, cinq niveaux de carte, profondeur de capacité arrêtée à la décision. |
| **La géographie se dégrade.** | Ancres persistées, placement déterministe, lentilles sans re-layout, revue de carte. |
| **La migration casse les liens.** | Migration additive, alias de routes, aucun objet supprimé. |
| **La carte est lente.** | Agrégation, seuils de nœuds, virtualisation (ADR-020), projection mise en cache. |
| **Les lentilles divergent.** | Un seul graphe, une seule projection, des encodages ; aucune copie de données. |
| **La gouvernance des données produit/équipe n'est pas claire.** | Les propriétaires produit/équipe mettent à jour ; l'architecte arbitre et signale les anomalies. |

**Arbitrages ouverts avant implémentation.**

| Sujet | Options | Recommandation |
|---|---|---|
| Quartier par défaut à L0 | value stream / portefeuille / unité d'organisation | **Value stream** si l'entreprise pilote par la valeur ; sinon portefeuille ; configurable par instance. |
| Profondeur de la hiérarchie produit | 2 / 3 / 4 niveaux | **3 maximum** ; au-delà, le produit est un portefeuille, pas un produit. |
| Agrégation d'une route | criticité max / volume / nombre d'interfaces | **Criticité max + badge count** ; le volume est une option de la lentille Interface. |
| Propriétaire d'une capacité | équipe / rôle / aucun | **Rôle ou équipe optionnel** ; la capacité reste stable même si l'organisation change. |
| Placement sur la carte | automatique / manuel / hybride | **Hybride** : heuristique déterministe + ancres `pinned` pour les objets importants. |
| Migration des domaines | big bang / guidée | **Guidée et additive** ; aucun objet ni historique supprimé. |

### 16. Références

- The Open Group, *ArchiMate 3.2 Specification* — `Product`, `ValueStream`, `Capability`, couches et relations : <https://publications.opengroup.org/c226>
- Simon Brown, *C4 model* — abstractions et diagrammes hiérarchiques : <https://c4model.com/>
- Matthew Skelton & Manuel Pais, *Team Topologies* — types d'équipes et modes d'interaction : <https://teamtopologies.com/key-concepts>
- The Open Group, *TOGAF Business Capabilities Guide* — capability map et croisement avec les value streams.
- Business Architecture Guild, *Business Architecture Metamodel Guide* — value stream, étapes, capabilities.
- Thoughtworks, *How to create a product operating model* — produit, topologie produit, alignement équipe/produit : <https://www.thoughtworks.com/insights/articles/how-to-create-a-product-operating-model-to-support-product-organization-transformation>
- Product Focus, *Product Taxonomy* — portefeuille, ligne, produit, plateforme.
- Pratiques de catalogue d'API — fournisseur, consommateur, version, cycle de vie : <https://api7.ai/blog/api-catalog-vs-developer-portal>
- Pratiques de « city map » et paysage applicatif — géographie stable, pan/zoom, calques : <https://aplas.com/use-cases/architecture>

## Périmètre fonctionnel du MVP1

Le MVP1 était organisé autour de sept objets métier principaux : **Domain**, **ModelElement**, **Relationship**, **Interface**, **Diagram**, **Story** et **Review**. La notion d’interface est volontairement un objet du référentiel et non une simple forme graphique. La cible v0.4 (ADR-023) ajoute **Product**, **Team**, **ValueStream**, **Capability**, **Application**, **TechnologyNode** et **InterfaceConsumption**, et fait du domaine métier une classification.

Les exigences ci-dessous sont la **base MVP1**. Les FR-040 à FR-054 formalisent la décision structurante de l’**Atelier de vues** ; elles sont révisées par la section « Cible v0.4 » et par les FR-055 à FR-081. Les FR-001 à FR-039 couvrent le référentiel, les interfaces, la temporalité, la documentation, l’IA et la collaboration. En cas de conflit, la cible v0.4 prévaut.

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

### L’Atelier de vues : surface d’édition

> **Amendement ADR-023 (18/11/2026).** Le point d’entrée du produit est désormais la **Carte de l’entreprise** (voir « Cible v0.4 — Le produit, la carte et le zoom »). L’Atelier reste la **surface d’édition des vues authored** : il n’est plus l’écran d’accueil, mais il conserve les principes de création depuis la vue, de formalisation progressive et de commandes validées. Les paragraphes ci-dessous décrivent cette surface, pas le parcours d’entrée.

**Décision structurante.** L’Atelier n’est ni un tableau de bord, ni un inventaire, ni une liste de domaines : c’est une **surface de conception** (designer de vues). Archifier est un outil d’architecture avant tout, pas une CMDB. L’architecte pense, arbitre et communique en construisant des vues ; le référentiel se remplit comme **conséquence** du geste de conception, jamais comme préalable administratif.

Cette décision ne contredit pas le principe « le référentiel est la source de vérité ». Elle en sépare les plans :

| Plan | Rôle | Conséquence UX |
|---|---|---|
| **Vérité** | Objets, relations, interfaces, versions, historique | Aucune forme graphique ne crée un objet implicite non validé ; aucune suppression de forme ne supprime un objet. |
| **Surface** | L’Atelier, les vues, le cadrage, le récit | C’est l’interface première : on y crée, on y édite, on y comprend, on y publie. |
| **Contrôle** | Inventaires, anomalies, couverture, exports | Espace secondaire, consultatif, jamais passage obligé. |

**Renversement du parcours (révisé par ADR-023).** Le parcours de référence est :

1. j’ouvre la **Carte de l’entreprise** ;
2. je zoome sur un value stream, un produit, une application ou une infrastructure ;
3. depuis le niveau courant, je crée ou modifie l’objet, ou j’ouvre une vue authored dans l’Atelier ;
4. le produit **formalise** ces formes en objets du référentiel (type, propriétaire, propriétés) au moment opportun ;
5. je publie, je raconte, je compare, je révise.

Dans l’Atelier, le parcours de création reste : je choisis une intention de vue ou je repars d’une vue existante, je place des formes, je les relie, je les nomme. L’Atelier n’est plus le premier écran ; il est la surface où l’on compose un récit ou un diagramme authored.

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

**Accueil, bibliothèque et contrôle (révisé par ADR-023).** L’accueil est la **Carte de l’entreprise** : le dernier cadrage de carte, sinon la vue entreprise. La **Bibliothèque** (produits, équipes, value streams, capacités, applications, vues, interfaces) et le **Contrôle** (inventaires, anomalies, couverture des métamodèles, exports) sont accessibles depuis la barre latérale, mais aucun des deux n’est un passage obligé. Une fiche d’objet reste disponible et complète : elle est la conséquence d’un clic dans la carte ou dans une vue, pas la porte d’entrée du produit.

La page d’un **domaine métier** reste une vue de synthèse utile — produits classifiés, interfaces exposées et consommées, dépendances, vues publiées, Stories — mais elle est atteinte depuis une lentille, une classification ou la Bibliothèque, jamais imposée au lancement. Un domaine métier ne possède pas d’objet : il classifie.

### Revue d’interfaces

> **Amendement ADR-023.** Une revue peut porter sur un **produit**, une **équipe**, un **value stream** ou un **domaine métier**. `domainId` devient un filtre optionnel ; `scopeObjectId` désigne le périmètre principal. Les interfaces affichées sont celles du périmètre, avec fournisseur et consommateurs objets.

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
  level            // BUSINESS | APPLICATION | INFRASTRUCTURE
  category         // BUSINESS | APPLICATION | TECHNICAL
  kind             // API | EVENT | FILE | BATCH | UI | DATABASE | NETWORK | OTHER
  providerObjectId // Product | Application | Component | TechnologyNode
  providerDomainId? // classification, plus un propriétaire
  exposure         // INTERNAL | PRODUCT | PARTNER | PUBLIC
  criticality
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
arch://relationship/01JABD...
arch://interface/01JDEF...
arch://version/01JDEG...
arch://diagram/01JGHI...
arch://story/01JKLM...
arch://product/01JNOP...
arch://team/01JNOQ...
arch://value-stream/01JNOR...
arch://stage/01JNOS...
arch://capability/01JNOT...
arch://application/01JNOU...
arch://node/01JNOV...
arch://business-domain/01JNOW...
```

Une carte n’est pas un objet : un lien de carte est une route `#/map?…` (C-10 §9), pas un `arch://`. Une Story ou une Revue peut capturer une `MapQuery` et une révision, sans créer d’objet carte.

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
  | MapBlock
  | ViewBlock
  | ElementBlock
  | InterfaceBlock
  | TransitionBlock
```

`MapBlock` capture une `MapQuery` (périmètre, niveau, lentilles, date, focus) et une révision ; il ne duplique ni la géométrie ni les objets. `DiagramBlock` peut définir un cadrage graphique : zoom, éléments mis en avant, éléments atténués et texte contextuel. Cela permet d’utiliser le même diagramme ou la même carte plusieurs fois dans une histoire avec des focales différentes.

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

L’architecture recommandée reste volontairement compacte :

```text
┌──────────────────────────────────────────────────────────────────────┐
│                           Web Application                            │
│  Carte | Produit | Équipe | Value stream | Application | Interface   │
│  Atelier de vues | Bibliothèque | Contrôle | Story | AI              │
└───────────────────────────────┬──────────────────────────────────────┘
                                │ HTTPS
┌───────────────────────────────▼──────────────────────────────────────┐
│                           Application API                            │
│ Map projection │ Repository │ Modeling │ Docs │ Review               │
│ Temporal │ Search │ OpenAPI │ AI Gateway                              │
└───────────────┬───────────────────────────────┬──────────────────────┘
                │                               │
        ┌───────▼───────┐               ┌───────▼─────────┐
        │  PostgreSQL   │               │ AI Orchestrator │
        │  + FTS        │               │ + MCP façade    │
        │  + vector     │               └─────────────────┘
        └───────────────┘
```

Un moteur de graphes dédié n’est pas nécessaire. Les relations sont représentables en SQL avec tables d’arêtes et index adaptés. Un CTE récursif suffit pour les parcours d’impact bornés ; la projection de carte est une agrégation calculée, mise en cache par révision. Une base graphe spécialisée pourra être envisagée si les volumes ou requêtes multi-hop deviennent dominants.

### Modèle physique minimal

Le schéma MVP1 ci-dessous est conservé. La cible v0.4 ajoute les tables de l’ontologie produit et de la projection de carte ; aucune table existante n’est supprimée.

| Table | Clés essentielles |
|---|---|
| `domains` | `id`, `name`, `description`, `revision` — devient `business_domains` en lecture cible |
| `objects` | `id`, `object_type`, `responsible_domain_id`, `business_domain_id`, `accountable_team_id`, `current_revision_id` |
| `object_revisions` | `id`, `object_id`, `revision_no`, `payload`, `created_at`, `created_by` |
| `model_elements` | `object_id`, `notation`, `metamodel_version`, `metaclass` |
| `relationships` | `object_id`, `source_id`, `target_id`, `relation_type`, `notation` |
| `interfaces` | `object_id`, `logical_interface_key`, `level`, `category`, `kind`, `provider_object_id`, `provider_domain_id`, `exposure` |
| `interface_versions` | `id`, `interface_id`, `semver`, `available_from`, `available_until`, `published_at`, `contract` |
| `interface_consumers` | `interface_version_id`, `consumer_object_id`, `consumer_domain_id`, `criticality` |
| `diagrams` | `object_id`, `notation`, `layout` |
| `diagram_nodes` | `diagram_id`, `element_id`, `geometry`, `style_override` |
| `diagram_edges` | `diagram_id`, `relationship_id`, `geometry` |
| `stories` | `object_id`, `story_structure` |
| `reviews` | `object_id`, `filter_snapshot`, `temporal_window` |
| `comments` | `id`, `target_id`, `author_id`, `markdown`, `created_at` |
| `links` | `source_id`, `target_uri`, `link_type`, `status` |
| `embeddings` | `object_id`, `revision_id`, `chunk_id`, `vector`, `text_hash` |
| `audit_events` | `id`, `actor_id`, `operation`, `object_id`, `before_revision`, `after_revision` |

Extension cible v0.4 (détail dans C-10) :

| Table | Clés essentielles |
|---|---|
| `products` | `id`, `name`, `kind`, `lifecycle`, `criticality`, `parent_product_id`, `accountable_team_id` |
| `teams` | `id`, `name`, `topology`, `mission`, `org_unit_id` |
| `value_streams` | `id`, `name`, `customer_object_id`, `owner_team_id` |
| `value_stream_stages` | `id`, `value_stream_id`, `order`, `name`, `value_item` |
| `capabilities` | `id`, `name`, `parent_id`, `level`, `maturity`, `criticality` |
| `applications` | `id`, `name`, `kind`, `lifecycle`, `criticality`, `operating_team_id` |
| `application_components` | `id`, `application_id`, `name`, `kind` |
| `technology_nodes` | `id`, `name`, `kind`, `environment`, `parent_node_id`, `provider_team_id` |
| `product_applications` | `product_id`, `application_id` |
| `product_capabilities` | `product_id`, `capability_id` |
| `product_stages` | `product_id`, `stage_id`, `value_item` |
| `capability_stages` | `capability_id`, `stage_id` |
| `business_domains` | `id`, `name`, `parent_id` |
| `object_business_domains` | `object_id`, `business_domain_id` |
| `portfolios` | `id`, `name`, `kind` |
| `initiatives` | `id`, `name`, `kind`, `start`, `end` |
| `map_anchors` | `object_id`, `level`, `district_id`, `x`, `y`, `pinned`, `revision` |

Les IDs doivent être des identifiants opaques stables, par exemple UUIDv7 ou ULID. Le choix exact est une décision d’implémentation ; la propriété requise est l’indépendance vis-à-vis du nom, de la notation, du produit et du domaine.

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

Cible v0.4 (les endpoints existants restent, avec `domains` en alias de `business-domains`) :

```text
GET    /api/map?scope=…&level=…&lens=…&date=…&focus=…
GET    /api/products
POST   /api/products
GET    /api/products/{id}
PUT    /api/products/{id}
GET    /api/products/{id}/applications
GET    /api/products/{id}/interfaces
GET    /api/products/{id}/capabilities

GET    /api/teams
GET    /api/teams/{id}
GET    /api/teams/{id}/interfaces
GET    /api/teams/{id}/dependencies

GET    /api/value-streams
GET    /api/value-streams/{id}
GET    /api/value-streams/{id}/stages
GET    /api/capabilities
GET    /api/capabilities/{id}

GET    /api/applications
GET    /api/applications/{id}
GET    /api/applications/{id}/components
GET    /api/applications/{id}/deployment
GET    /api/infrastructure/{id}

GET    /api/interfaces/{id}/consumers
POST   /api/interfaces/{id}/consumptions
GET    /api/impact?object=…&date=…
GET    /api/business-domains
GET    /api/business-domains/{id}
POST   /api/import/inventory
GET    /api/export/map?scope=…&level=…&lens=…&format=svg|md
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
| NFR-031 | La carte de l’entreprise affiche un premier rendu utile en < 2,5 s en local, pour 500 nœuds visibles. |
| NFR-032 | Une projection de carte (périmètre, niveau, lentilles, date) est calculée en p95 < 300 ms pour 1 000 produits et 5 000 applications agrégées. |
| NFR-033 | Pan, zoom et changement de lentille restent fluides (objectif 60 fps, plancher 30 fps) sans recalcul de la géographie. |
| NFR-034 | La géographie est stable : un objet conserve son ancre d’une session à l’autre et d’une lentille à l’autre ; changer de lentille ne déplace aucun nœud. |
| NFR-035 | Un lien profond de carte restaure à l’identique le périmètre, le niveau, les lentilles, la date et le focus. |
| NFR-036 | La carte offre une navigation clavier complète entre niveaux et une liste alternative des objets du niveau courant. |

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

| Capacité | MVP1 v0.1 | Cible v0.4 (ADR-023) |
|---|---|---|
| **Carte de l’entreprise (city map)** | Non — l’accueil était l’Atelier | **Oui — point d’entrée du produit**, zoom L0→L4, lentilles |
| **Produit, équipe, value stream, capacité** | Non — le domaine servait de pivot | **Oui — objets de premier rang**, propriété et classification séparées |
| **Atelier de vues (designer)** | **Oui — point d’entrée du produit** | Surface d’édition des vues authored, atteinte depuis la carte |
| Inventaire de type CMDB | Non prioritaire — espace Contrôle | reporting et tableaux de bord |
| Référentiel multi-domaines | Oui | enrichissement ; domaine = classification |
| Interfaces multi-niveaux | Oui, centrées domaine | **Oui — produit, application, infrastructure**, routes agrégées dépliables |
| Applications et composants | Oui, ArchiMate | **Oui — page application, déploiement, produits servis** |
| Infrastructure | Oui, ArchiMate Technology | **Oui — page infrastructure, nœuds, environnements** |
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

La bonne formulation est donc : **la carte est le point d’entrée, le dessin est la surface, le référentiel est la vérité**. La Carte de l’entreprise donne la big picture et le parcours de zoom ; l’Atelier reste le lieu de création des vues authored ; chaque forme y est reliée à un objet identifié ou explicitement marquée comme croquis ; le métamodèle valide au moment du geste. Ce couplage fort — navigation fluide, création fluide, validation déterministe — est ce qui distingue Archifier d’un outil de dessin et d’une CMDB à la fois.

Le risque secondaire est l’**austérité prise pour de la sobriété**. Une interface dense, sans grammaire spatiale ni hiérarchie visuelle, ne transmet aucune architecture : elle transmet un formulaire. La carte doit avoir une grammaire (quartiers, bâtiments, routes, ports, encodage temporel) dès la cible, sinon le produit sera perçu comme un back-office. Un troisième risque est la **carte décorative** : une belle image qui ne se déplie pas vers les objets réels. Chaque route doit être dépliable, chaque bâtiment doit ouvrir un produit, chaque port doit ouvrir une interface versionnée.

Un autre risque est le coût d’une « conformité totale » simultanée à trois langages. La v0.1 doit au contraire être stricte sur un sous-ensemble annoncé. BPMN 2.0.2 et UML 2.5.1 disposent tous deux de métamodèles normatifs complets ; cela rend techniquement possible une extension ultérieure sans justifier de tout implémenter immédiatement. citeturn339674search6turn339674search7

Le risque ArchiMate 4 est que l’outil figerait son schéma SQL autour des noms de classes 3.2. La cible 4 est légitime comme exigence produit, mais la seule base normative disponible aujourd’hui reste 3.2 d’après The Open Group. citeturn992156search0turn992156search2

Le risque IA est qu’un RAG vectoriel seul donnerait de mauvaises garanties sur les questions d’impact, d’état ou de temporalité. À l’inverse, un moteur purement structuré sera médiocre pour retrouver un concept exprimé différemment dans des centaines de pages Markdown. D’où l’architecture hybride : **graphe logique + SQL pour la vérité, full-text pour les termes, embeddings pour la similarité, LLM pour l’orchestration et l’explication**.

Enfin, MCP Apps, AG-UI et A2UI répondent à des problèmes voisins mais distincts. MCP Apps transporte des interfaces interactives via l’écosystème MCP ; AG-UI standardise la conversation et les événements entre agent et application ; A2UI décrit des UI générées par l’agent. Pour le cœur d’un outil de modélisation où sélection, déplacement, relations, historique et validation doivent rester prédictibles, une UI web native est la meilleure base du MVP1. citeturn952960search3turn485588search1turn952960search7

Les arbitrages qu’il reste réellement à prendre pour transformer cette v0.1 en backlog sont principalement : niveau exact de granularité des concepts ArchiMate 3.2 exposés dans la première palette ; sémantique précise des relations transverses entre notations ; politique de versionnage des objets non-interface ; distinction éventuelle entre « publié » et « visible » pour les diagrammes et Stories ; conventions de nommage des domaines et interfaces ; seuil de profondeur par défaut pour l’analyse d’impact ; et choix final du format canonique interne des contrats OpenAPI.

Le point de départ recommandé pour la conception détaillée n’est donc ni un catalogue, ni un guide de création : c’est une **Carte de l’entreprise** adossée à un **référentiel sémantique versionné, temporel et multi-notation**, avec un **Atelier de vues** comme surface d’édition. Autour de la carte se branchent la Bibliothèque (retrouver un produit, une équipe, une application, une interface, une vue), le Contrôle (anomalies, couverture, inventaires) et le Storytelling (raconter et diffuser les vues). L’IA intervient au-dessus de ce référentiel comme lecteur et générateur de propositions, jamais comme propriétaire de la vérité.