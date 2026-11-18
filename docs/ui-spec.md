# Spécification d'interface — La carte produit et l'Atelier de vues

| | |
|---|---|
| **Document** | Grammaire visuelle, navigation, lentilles et pages produit/équipe/value stream/application/infrastructure |
| **Version** | 2.0 — cible v0.4, 18 novembre 2026 |
| **Statut** | **v2.0 — cible produit ; remplace le point d'entrée d'ADR-022 par ADR-023** |
| **Sources** | `functional-specifications.md` § « Cible v0.4 », FR-040..081, NFR-021..036 ; `docs/contracts/C-10-produit-carte-navigation.md` ; ADR-023 ; ADR-022 (Atelier) ; `docs/contracts/*` |
| **Opposable en revue** | Oui. Une violation est un motif de NO-GO (voir §16). |

Ce document décrit **deux surfaces complémentaires** : la **Carte**, point d'entrée et
surface de navigation, et l'**Atelier**, surface d'édition des vues authored. Il fixe ce
que l'architecte **voit**, **comprend** et **fait**. Les contrats de données sont dans
`docs/contracts/` ; la carte et ses routes sont fixées par C-10.

---

## 1. Principes

1. **La Carte est le point d'entrée.** `/` ouvre la carte de l'entreprise, jamais un
   inventaire, un tableau de bord ou une galerie d'intentions.
2. **Le produit est l'unité de lecture et de responsabilité.** Un produit appartient à
   une équipe ; une application est utilisée par un ou plusieurs produits ; un domaine
   métier classifie, il ne possède pas.
3. **Le zoom est sémantique et géographique.** Cinq niveaux : entreprise, portefeuille,
   produit, application, infrastructure. Franchir un seuil change la requête, pas
   seulement l'échelle.
4. **Une seule géographie, plusieurs lentilles.** Les lentilles changent la couleur, les
   arêtes et les filtres ; elles ne déplacent jamais un objet.
5. **L'Atelier est la surface d'édition.** On y compose les vues authored, les diagrammes
   narratifs, les Stories et les Revues ; il n'est plus l'écran d'accueil.
6. **Le dessin est la surface, le référentiel est la vérité.** Une forme non identifiée
   est un croquis visible comme tel ; supprimer une forme ne supprime jamais un objet.
7. **Formalisation progressive.** `Croquis → Brouillon formalisé → Objet valide`, visible
   en permanence, sans ambiguïté sur ce qui est publié.
8. **L'inventaire est secondaire.** Bibliothèque et Contrôle existent, jamais comme
   passage obligé ; aucune table n'est le contenu principal de la carte.

## 2. Grille, espace, densité

- Grille de monde : **8 px**, aimantation active par défaut (`grid.size = 8`).
- Espacement d'interface : multiples de `--step` (4 px), tokens `--space-1..8`.
- Une carte commence par **un cadrage lisible** (zoom 0.85–1.0), pas par un zoom à 100 %
  sur un coin vide.
- Les quartiers occupent des zones stables ; un produit peut déborder de son quartier
  (avertissement visuel, pas une erreur).
- L'inspecteur fait `--inspector-width` (320 px) et **réduit** la zone de carte, il ne
  la recouvre pas. La palette de lentilles fait `--palette-width` (232 px), repliable.
- La barre d'état fait `--statusbar-height` (28 px) et reste visible en permanence.
- Une carte affiche au plus **500 nœuds** à L0/L1, **200** à L2, **150** à L3/L4 ;
  au-delà, elle agrège ou demande un filtre (C-10, `MAP-011`).

## 3. Hiérarchie typographique

| Niveau | Rôle | Taille / graisse | Couleur |
|---|---|---|---|
| 1 | Nom de la carte ou de la page (barre d'outils) | `--text-xl` / `--weight-strong` | `--ink` |
| 2 | Nom de quartier / produit / application | `--node-title-size` / `--node-title-weight` | `--ink` |
| 3 | Type d'objet (produit, application, nœud) | `--node-type-size` / `--weight-regular` | `--ink-soft` |
| 4 | Équipe / domaine métier | `--node-domain-size` / `--weight-regular` | `--muted` |
| 5 | État temporel ou cycle de vie | `--node-status-size` / `--weight-medium` | badge + motif |
| 6 | Annotation de carte (route, légende, compteur) | `--text-sm` italique | `--muted` |

Une information critique n'existe **jamais** uniquement dans une infobulle ni uniquement
par la couleur. Le type, l'état et le propriétaire sont écrits.

## 4. Zoom sémantique — cinq niveaux

| Niveau | Nom | Ce qui est visible | Ce qui disparaît | Clic |
|---|---|---|---|---|
| **L0** | **Entreprise** | quartiers (value streams/portefeuilles), bâtiments (produits), utilités (plateformes), routes agrégées, criticité | applications, composants, nœuds | quartier → L1, produit → L2, route → panneau |
| **L1** | **Portefeuille / quartier** | produits du périmètre, plateformes partagées, interfaces produit à produit, capacités en chaleur | composants, infrastructure | produit → L2 |
| **L2** | **Produit** | frontière, ports, applications, interfaces applicatives, produits externes atténués | composants internes, nœuds | application → L3, port → interface |
| **L3** | **Application** | composants, services, interfaces, données, déploiement | produits, value streams | composant → détail, nœud → L4 |
| **L4** | **Infrastructure** | nœuds, environnements, réseaux, applications déployées, interfaces techniques | composants métier | nœud → détail, application → L3 |

Règles :

- Le passage d'un niveau à l'autre est **continu** (opacité, taille, apparition), jamais un
  « mode » à activer.
- À l'agrégat, un clic sur un agrégat **zoome** sur son contenu ; il ne l'ouvre pas dans une
  fiche. `Entrée` ouvre la fiche complète.
- Le zoom sémantique ne modifie ni les ancres, ni le cadrage persisté d'une vue authored.
- Un niveau ne montre jamais individuellement un objet d'un niveau inférieur : il affiche
  un compteur (`aggregateCount`) et un badge.
- À L1, une **liste des produits du périmètre** est accessible en panneau ou en mode liste,
  avec nom, équipe, cycle de vie et criticité ; elle ne remplace pas la carte.
- Le fil d'Ariane est toujours visible et cliquable.
- Un changement de niveau met à jour la route (`scope`, `level`) : le niveau courant est
  partageable et le bouton retour du navigateur fonctionne.
- Le retour au niveau précédent restaure le cadrage exact.

## 5. Quartiers, bâtiments, blocs, utilités

| Élément | Objet | Règle |
|---|---|---|
| **Quartier** | Value stream, portefeuille ou unité d'organisation | Zone stable, nommée, teintée ; ne crée pas d'objet. |
| **Bâtiment** | Produit | Position stable ; porte le nom, l'équipe, le cycle de vie, la criticité. |
| **Bloc / pièce** | Application | Visible à L2+ ; positionné dans la frontière du produit. |
| **Étage / local** | Composant applicatif | Visible à L3 ; pas de déplacement à L2. |
| **Utilitaire** | Produit plateforme ou nœud partagé | Placé en bordure de quartier ; relié par des routes. |
| **Route** | Interface ou flux agrégé | Dépliable ; jamais une flèche anonyme. |
| **Port** | Interface exposée | Posé sur la frontière du produit ou de l'application. |
| **Zone `BUSINESS_DOMAIN`** | Domaine métier | Classification visuelle, pas un territoire de propriété ; ne crée jamais d'équipe. |
| **Groupe** | Regroupement visuel | Couloir, environnement, lot ; bordure pointillée. |

Règles :

- Une zone de domaine métier **ne crée jamais** de domaine ni d'équipe.
- Un bloc applicatif dessiné dans un produit est un **usage**, pas une possession : une
  application partagée apparaît dans plusieurs produits.
- Un quartier qui représente un value stream affiche ses étapes comme une **bande de lecture**
  en haut du quartier ; les produits restent dans une zone unique et sont reliés aux étapes
  par des arêtes de contribution. Un produit n'est jamais dupliqué dans plusieurs bandes.
- Un quartier qui représente un portefeuille affiche ses produits groupés.
- Un produit n'apparaît qu'**une fois** à L0, dans son value stream principal (ou son
  portefeuille de repli) ; ses contributions aux autres value streams sont des liens de
  contexte, jamais des duplications.
- Une zone peut être repliée ; les ports restent ancrés et visibles sur la frontière.
- Les objets externes au périmètre restent visibles, atténués, pour montrer les dépendances
  sortantes.

### 5.1 Bandes de couches dans l'Atelier

La carte n'utilise pas de bandes de couches : elle a des quartiers, des bâtiments et des
routes. Les **bandes de couches** restent la grammaire des vues authored dans l'Atelier.

- Une bande est un rectangle horizontal nommé, teinté (`--layer-*-bg`), avec un filet
  d'accent à gauche.
- Elle se **déplace** (poignée en tête), se **replie** (`--layer-collapsed-h`), se
  **renomme** (`F2` quand la bande est sélectionnée).
- Une bande repliée montre son nom et le nombre de formes qu'elle contient.
- Une bande n'est **pas** un objet du référentiel : elle n'apparaît ni dans la recherche,
  ni dans l'inventaire, ni dans l'export référentiel. Elle apparaît dans l'export SVG.
- L'ordre des bandes suit le profil (Métier → Applicatif → Technologique → Physique →
  Motivation → Mise en œuvre), sauf réorganisation explicite de la vue.
- Les formes s'aimantent aux bords de bande et aux guides de zone.

## 6. Frontières et ports

- La **frontière** est le périmètre d'un produit (L2) ou d'une application (L3)
  (`--boundary-stroke`, pointillé).
- Un **port** est un petit cercle posé sur la frontière (`--port-*`) ; il représente une
  interface exposée, éventuellement une version précise.
- Poser un port propose immédiatement :
  1. rattacher une interface existante (recherche contextuelle, sans quitter la carte) ;
  2. ou créer une interface (nom + niveau + catégorie) et sa première version dans le même
     geste. Sans l'un des deux : `PRD-024` / `DSG-004`.
- Le port affiche le nom de l'interface au survol **et** un `aria-label` complet ; la
  version (`v2`) apparaît au zoom de détail.
- Une **consommation** est une arête qui traverse la frontière
  (`--port-consumer-stroke`, pointillé). Elle ne crée pas de port côté consommateur : le
  lien part du produit/application consommateur et pointe le port.
- Un port sélectionné montre ses consommateurs en surbrillance, sans ouvrir de fiche.
- Une interface `INTERNAL` n'a pas de port sur la frontière du produit ; elle reste
  visible à L3.

## 7. Routes et agrégation

- Une route à L0/L1 est **agrégée** : elle représente plusieurs interfaces. Son épaisseur
  encode la criticité, son motif encode l'état temporel, sa couleur encode la lentille.
- Le clic ouvre un panneau latéral : interfaces, versions, fournisseurs, consommateurs,
  états. Le panneau ne remplace pas la carte.
- « Déplier » isole le couple fournisseur/consommateur ou descend d'un niveau.
- Une route non dépliable est un défaut (`MAP-005`).
- Une route stratégique saisie manuellement est marquée `strategic` et visuellement
  distincte ; elle ne remplace jamais une route dérivée.
- À L2, les routes relient des applications ; à L3, des composants ; à L4, des nœuds.

## 8. Temps et cycle de vie dans la carte

Le curseur temporel est dans la barre d'outils : un champ date + « Aujourd'hui » +
« Effacer ». Il ne déplace rien, ne réécrit rien.

| État | Motif | Texte | Couleur d'appui |
|---|---|---|---|
| `ACTIVE` | trait plein | « Active » | `--temporal-active-stroke` |
| `SCHEDULED` | pointillé `6 4` | « Planifiée » | `--temporal-scheduled-stroke` |
| `EXPIRED` | fantôme + barre | « Expirée » | `--temporal-expired-stroke` |
| Chevauchement | hachures 45° | « v1 et v2 actives du … au … » | `--temporal-overlap-fill` |

Règles :

- Aucun état n'est encodé par la couleur seule (NFR-028).
- La légende temporelle est toujours accessible depuis la barre d'état, jamais une aide
  cachée.
- Changer T ne modifie ni la géométrie, ni les données, ni la révision tant que
  l'autosave n'a pas enregistré le seul champ `temporal_cursor`.
- Le curseur s'applique aux produits (cycle de vie), aux applications, aux interfaces
  (versions) et aux initiatives.
- Les états de cycle de vie (`IDEA`, `DISCOVERY`, `BUILD`, `RUN`, `SUNSET`, `RETIRED`) sont
  distincts des états temporels d'interface et sont toujours écrits.

## 9. Croquis, brouillon, validé

| État | Forme | Badge | Ce que l'utilisateur peut faire |
|---|---|---|---|
| `SKETCH` | contour pointillé, libellé italique | « Croquis » | renommer, déplacer, formaliser, supprimer la représentation |
| `FORMALIZED` | forme pleine | « Brouillon » | compléter les propriétés, publier impossible |
| `VALID` | forme pleine, aucun signal | « Validé » | publier, exporter, raconter |
| Publié | badge « Publié » dans la barre d'état | — | partager, retirer |

Un croquis ne se distingue pas par une teinte « brouillon » : il se distingue par son
**trait**, son **badge** et son **absence d'identité** (pas de type, pas de propriétaire).

## 10. Les lentilles

Une lentille est une projection du même graphe. Elle répond à une question et encode une
dimension. Les lentilles se **superposent**.

| Lentille | Question | Encodage | Niveaux |
|---|---|---|---|
| **Valeur** | Quel parcours, quelles étapes, quels produits ? | quartiers = value streams, bandes = étapes | L0, L1, L2 |
| **Capacité** | Quelles capacités, couvertes par quoi ? | chaleur de couverture/maturité | L0, L1, L2 |
| **Produit** | Quel portefeuille, quel cycle de vie ? | couleur = lifecycle/criticality | L0, L1, L2 |
| **Organisation** | Qui possède quoi, où sont les dépendances ? | couleur = équipe, frontières d'équipe | L0, L1, L2 |
| **Applicatif** | Quelles applications, quels composants ? | couleur = type/âge | L1, L2, L3 |
| **Interface** | Qui expose/consomme quoi, quelles versions ? | ports + routes | Tous |
| **Technologie** | Quelles plateformes, quelle obsolescence ? | couleur = type/âge/environnement | L2, L3, L4 |
| **Données** | Quels objets, quels flux, quelle classification ? | couleur = classification | L1, L2, L3 |
| **Temps** | Où en est-on à T ? | motif = scheduled/active/expired | Tous |
| **Risque / coût** | Où sont les risques, coûts, dettes ? | chaleur | L0, L1, L2, L3 |

Règles :

- Au moins une lentille est active ; la lentille par défaut est **Produit** à L0/L1 et
  **Interface** à L2/L3.
- Une lentille ne modifie jamais la position, la taille ou la hiérarchie.
- Une lentille indisponible au niveau courant est grisée avec sa raison, pas silencieusement
  retirée.
- La légende nomme la lentille, l'encodage et les états.
- Les sept intentions de l'Atelier deviennent des **lentilles ou des actions** :
  Contexte → Valeur ; Applicatif → Applicatif ; Processus → BPMN d'une étape ; Déploiement
  → Technologie ; Capacité → Capacité ; Migration → Temps ; Revue d'interface → Interface.
  Elles ne sont plus un préalable à la création d'un domaine.

### 10.1 Les sept intentions de l'Atelier (conservées pour les vues authored)

Le catalogue C-03 reste valable pour les vues authored. Chaque intention a un aperçu
spatial, une palette filtrée, un état vide et un modèle JSON.

| Intention | Ce que la vue doit montrer d'emblée | État vide (phrase d'invitation) |
|---|---|---|
| **Contexte** | pour qui, pourquoi, avec quoi ; bande Motivation dominante | « Posez d'abord les acteurs et les objectifs, puis les composants qui les servent. » |
| **Applicatif** | composants, services, une frontière de produit | « Déposez les composants du produit, nommez-les, reliez-les. » |
| **Processus** | couloirs, événement de début, tâches | « Commencez par l'événement de début, puis les tâches dans l'ordre. » |
| **Déploiement** | nœuds d'exécution, artefacts, environnements | « Placez les nœuds d'exécution, puis glissez les composants déployés. » |
| **Capacité** | services rendus, objectifs, lots | « Cartographiez ce que l'entreprise sait faire comme des services rendus. » |
| **Migration** | existant, cible, lots, curseur temporel ouvert | « Posez l'existant, puis l'état cible ; le curseur montre la bascule. » |
| **Revue d'interface** | frontière, ports, versions, consommateurs | « Dessinez la frontière, posez un port par interface exposée. » |

Aucune intention ne diffère d'une autre **uniquement par son titre** : bandes, palette,
zones et exemple changent.

## 11. Gestes

### 11.1 Carte

| Geste | Souris | Clavier | Résultat | Refus attendu |
|---|---|---|---|---|
| Ouvrir un quartier / produit | clic | `Entrée` | zoome au niveau suivant | `MAP-002` si niveau incompatible |
| Revenir | bouton fil d'Ariane | `Alt+←` | restaure le cadrage précédent | — |
| Déplier une route | clic | `Entrée` sur la route | panneau d'interfaces | `MAP-005` si non dépliable |
| Changer de lentille | palette de lentilles | `L` puis flèches | ré-encodage sans déplacement | `MAP-004` si indisponible |
| Déplacer un objet | glisser (mode édition) | `Maj`+flèches | crée une ancre `pinned` | `MAP-024` si collision |
| Créer un produit | bouton « Nouveau produit » | `P` | panneau latéral non modal | `PRD-001` si équipe absente |
| Créer une interface | port sur frontière | `I` | panneau interface + version | `PRD-024` / `PRD-025` |
| Rechercher | barre de recherche | `Ctrl/Cmd+K` | saute à l'objet et zoome | — |
| Curseur temporel | champ date | focus barre, flèches | ré-encodage d'état | `MAP-006` si date illisible |
| Ouvrir la fiche | double-clic | `Entrée` | page produit/application/interface | — |

### 11.2 Atelier

Les gestes de l'Atelier sont conservés (sélection, dépôt, relation, zone, cadrage,
publication, partage, annuler/rétablir). Trois gestes maximum pour créer un élément typé :
**déposer → nommer → choisir le type** (NFR-022). Le choix du type peut être différé : le
croquis reste, la publication attend.

## 12. Inspecteur et panneaux

Panneau latéral droit, non bloquant, sections dans cet ordre :

1. **Identité** — nom (éditable), type, propriétaire (équipe), cycle de vie, statut de
   formalisation.
2. **Propriétés** — générées depuis le profil ; champ requis marqué ; propriété inconnue
   refusée (`MM-PROP-001`).
3. **Appartenance** — value streams, capacités, domaine métier.
4. **Applications / interfaces** — usages, exposition, consommation, versions.
5. **Documentation** — Markdown, aperçu filtré.
6. **Liens** — entrants/sortants, `arch://` résolus.
7. **Historique** — révisions, diff à la demande.

Il ne masque jamais la carte (il réduit la zone), ne prend jamais le focus de force, et
`Échap` le referme en rendant le focus à la carte. Un formulaire modal n'existe ni dans la
carte ni dans l'Atelier.

## 13. Écrans

### 13.1 `/` — Carte de l'entreprise (L0)

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ ◧ Archifier      Carte   Bibliothèque   Contrôle            Claire ▾   │
├───────────────┬──────────────────────────────────────────────────────────┤
│ LENTILLES     │  ENTREPRISE › Commande client › Paiement › …             │
│ ● Produit     │  ┌ quartier Commande client ───────────────────────────┐ │
│ ▸ Organisation│  │  ▣ Checkout     ▣ Paiement      ▣ Facturation       │ │
│ ▸ Interface   │  │      ╲            │              ╱                  │ │
│ ▸ Capacité    │  │       ╲──── Payment API v2 ────╱                    │ │
│ ▸ Temps       │  └─────────────────────────────────────────────────────┘ │
│               │  ┌ quartier Parcours client ──────────────────────────┐ │
│               │  │  ▣ Site      ▣ Mobile      ▣ Boutique             │ │
│               │  └─────────────────────────────────────────────────────┘ │
│               │  ┌ utilités ───────────────────────────────────────────┐ │
│               │  │  ▤ Identité   ▤ Référentiel client   ▤ Réseau       │ │
│               │  └─────────────────────────────────────────────────────┘ │
├───────────────┴──────────────────────────────────────────────────────────┤
│ 42 produits · 7 quartiers · 128 interfaces · 3 anomalies        − 100 % │
└──────────────────────────────────────────────────────────────────────────┘
```

- La carte est le contenu principal ; les compteurs sont une ligne de statut, pas un
  tableau de bord.
- À l'ouverture, la carte restaure le dernier cadrage (`scope`, `level`, `lens`, `date`)
  ou l'entreprise par défaut ; elle ne rouvre pas une vue authored à la place.
- Les quartiers sont des value streams, portefeuilles ou unités d'organisation ; les
  bâtiments sont des produits ; les utilités sont des plateformes.
- Les routes sont agrégées et dépliables ; la lentille par défaut est **Produit**.
- Aucune carte de domaine, aucun formulaire de domaine, aucune galerie d'intentions.
- Le premier lancement affiche un parcours qui crée un **premier produit** et une
  **première équipe** (§13.12).

### 13.2 `/map` — Carte avec contexte explicite

- Paramètres : `scope`, `level`, `lens`, `date`, `focus`, `filter`, `mode`, `caption`
  (C-10 §9).
- Un lien profond restaure le niveau, le périmètre, les lentilles, la date et le focus.
- Un scope inconnu retombe sur l'entreprise avec `MAP-001`, sans perdre le lien.
- Un niveau incompatible est corrigé vers le niveau le plus proche avec `MAP-002`.
- Le mode `edit` exige la capacité d'édition ; le mode `read` est le défaut des liens
  partagés.

### 13.3 `/products/:id` — Page produit (L2)

- **Bandeau** : nom, type, cycle de vie, criticité, équipe accountable, value streams,
  domaine métier, complétude.
- **Carte** : frontière du produit, ports d'interfaces exposées, applications à
  l'intérieur, produits/applications externes atténués, socle technique en bas.
- **Panneau** : valeur, utilisateurs, objectifs, capacités, applications, interfaces,
  données, risques, roadmap, décisions.
- **Sections** : Vue d'ensemble · Valeur & clients · Capacités · Applications · Interfaces ·
  Données · Équipe & organisation · Cycle de vie & roadmap · Risques & conformité ·
  Vues, Stories, décisions.
- **Actions** : ajouter une application, exposer une interface, rattacher une capacité,
  changer le cycle de vie, ouvrir une vue authored, lancer une analyse d'impact.
- **État vide** : « Ce produit n'a pas encore d'application. Ajoutez la première, ou
  rattachez-en une existante. »

### 13.4 `/teams/:id` — Page équipe

- **Bandeau** : nom, topologie, mission, unité d'organisation.
- **Carte** : produits possédés, applications opérées, interfaces fournies/consommées,
  dépendances vers d'autres équipes.
- **Sections** : Produits · Applications · Interfaces · Dépendances · Modes d'interaction ·
  Charge & criticité.
- **Signal** : une équipe qui possède trop de produits critiques ou d'applications
  partagées est un signal de charge cognitive, pas une faute.

### 13.5 `/value-streams/:id` — Page value stream

- **Bandeau** : nom, client, proposition de valeur, équipe propriétaire.
- **Carte L1** : étapes de gauche à droite, produits contributeurs, capacités habilitantes,
  interfaces critiques, applications de support.
- **Sections** : Étapes · Produits · Capacités · Interfaces critiques · Processus BPMN ·
  Indicateurs · Initiatives.
- **Action** : ouvrir le processus BPMN d'une étape (`REALIZES` / `DESCRIBES`).

### 13.6 `/capabilities/:id` — Page capacité

- **Bandeau** : nom, niveau, maturité, criticité.
- **Carte** : hiérarchie, chaleur de couverture, produits réalisateurs, applications de
  support, étapes de value stream habilitées.
- **Deux lectures** : la carte des produits réalisateurs et la **capability map**
  hiérarchique (grille) ; les deux partagent le même modèle, sans duplication.
- **Sections** : Décomposition · Couverture · Produits · Applications · Value streams ·
  Écarts et duplications · Roadmap.
- **Usage** : identifier les capacités sans produit, les duplications d'applications et les
  investissements à arbitrer.

### 13.7 `/applications/:id` — Page application (L3)

- **Bandeau** : nom, type, cycle de vie, criticité, équipe opératrice, produits servis,
  partagée ou non.
- **Carte** : composants, services, interfaces, données, déploiement, consommateurs.
- **Sections** : Vue d'ensemble · Composants · Interfaces · Données · Déploiement ·
  Produits servis · Consommateurs · Risques & dette · Historique.
- **Notations** : ArchiMate par défaut ; UML Component/Class/Sequence pour le détail.
- **État vide** : « Cette application n'a pas encore de composant. Ajoutez le premier, ou
  déclarez-la comme application feuille (COTS/SaaS). »

### 13.8 `/infrastructure/:id` — Page infrastructure (L4)

- **Bandeau** : nom, type, environnement, équipe, cycle de vie, criticité.
- **Carte** : nœuds, réseaux, environnements, applications déployées, interfaces techniques.
- **Sections** : Nœuds · Applications déployées · Interfaces techniques · Résilience ·
  Obsolescence · Coûts · Incidents.
- **Usage** : répondre à « qu'est-ce qui tourne où ? », « qui dépend de ce nœud ? »,
  « qu'est-ce qui est obsolète ? ».

### 13.9 `/interfaces/:id` — Page interface

- Onglets : Identité · Versions · Calendrier · Contrat · Consommateurs · Dépendances ·
  Documentation · Historique.
- Fournisseur et consommateurs sont des **objets** (produit, application, composant, nœud),
  pas seulement des domaines.
- Le calendrier est une frise horizontale avec `now` en repère, une bande par version,
  coloration `SCHEDULED`/`ACTIVE`/`EXPIRED`, périodes de chevauchement hachurées.
- L'onglet Contrat affiche « Aucun contrat OpenAPI » pour une interface non HTTP, sans
  proposer d'export factice.

### 13.10 `/design/:id` — Atelier (conception)

- Palette à gauche (repliable), canevas au centre, inspecteur à droite (repliable).
- Barre d'outils : titre de vue éditable, intention (badge, cliquable pour changer avec
  avertissement), outils, curseur temporel, bascule lecture, menu de vue.
- Barre d'état : `Enregistré · révision N`, nombre de croquis, alerte de conflit, légende
  temporelle, zoom.
- Aucun bouton « Enregistrer » principal ; `Ctrl/Cmd+S` est un secours discret.
- L'Atelier est atteint depuis la carte, la Bibliothèque ou un lien authored ; il n'est plus
  le point d'entrée.

### 13.11 `/view/diagrams/:id` — Atelier (lecture)

- Même rendu, chrome réduit : pas de palette, pas de poignées, pas d'outils de création.
- Le cadrage du lien est appliqué à l'ouverture.
- Clic sur une forme : aperçu latéral + « Ouvrir la fiche ». `Entrée` ouvre la fiche.
- Navigation par dépendances sans quitter la vue (surbrillance des chemins).
- Bascule « Modifier » ramène en conception **avec le même cadrage**.

### 13.12 `/library` et `/control` — Espaces secondaires

- **Bibliothèque** : recherche transversale (produits, équipes, value streams, capacités,
  applications, interfaces, vues, Stories, Revues), filtres par type, équipe, cycle de vie,
  lentille, état temporel.
- Les produits et applications sont présentés comme des **couvertures** (nom, équipe,
  cycle de vie, vignette de carte), pas comme des lignes de table.
- **Contrôle** : onglets Anomalies · Inventaire · Couverture · Exports. L'inventaire est
  assumé ici : c'est le seul endroit où un tableau est le contenu principal.
- Chaque anomalie pointe vers la carte, l'objet ou la forme concernée.
- Aucun bouton de création d'objet hors carte/Atelier.

### 13.13 Onboarding

Quatre gestes guidés, dans la carte, sans modal : **nommer l'entreprise**, **créer une
équipe**, **créer un produit**, **le placer dans un value stream**. Objectif : première
valeur en moins de 3 minutes. « Passer » est toujours visible et n'a pas de conséquences.
Le premier lancement ne demande **jamais** de créer un domaine ni de choisir une intention.

### 13.14 `/_design` — Galerie interne

Page de référence du design system : tokens, primitives, états (croquis, brouillon, validé,
publié, temporel, conflit, erreur, vide, chargement), quartiers, bâtiments, routes, ports,
lentilles. Elle n'est pas dans la navigation produit. Elle sert de test visuel et de
contrat.

### 13.15 `/business-domains/:id` — Domaine métier (classification)

- **Bandeau** : nom, domaine parent, description.
- **Carte** : produits classifiés, capacités, applications ; aucun territoire de propriété,
  aucune équipe implicite.
- **Sections** : Produits · Capacités · Applications · Interfaces · Vues & Stories.
- **État vide** : « Ce domaine métier ne classifie encore aucun objet. Rattachez-lui un
  produit ou une capacité. »
- **Règle** : un domaine métier ne possède pas d'objet ; il classe. La propriété reste
  l'équipe accountable.

### 13.16 `/portfolios/:id` — Portefeuille

- **Bandeau** : nom, type (value stream, BU, thème, plateforme, ligne produit), nombre de
  produits, criticité agrégée.
- **Carte L1** : produits groupés par ligne/équipe, plateformes partagées, interfaces
  produit à produit, signaux de cycle de vie.
- **Sections** : Produits · Plateformes · Interfaces · Équipes · Cycle de vie · Risques ·
  Investissements.
- **Usage** : revue de portefeuille, arbitrage d'investissement, rationalisation.
- **État vide** : « Ce portefeuille ne contient encore aucun produit. »

## 14. Matrice d'états

| Écran | Chargement | Vide | Erreur | Conflit | Partiel |
|---|---|---|---|---|---|
| `/` Carte | squelette de quartiers | onboarding premier produit | « Impossible de lire la carte » + réessayer | — | scope partiel → bandeau + niveau le plus proche |
| `/map` | squelette de quartiers | invitation à créer un produit | `MAP-001` + retour entreprise | — | `MAP-002` → niveau corrigé |
| `/products/:id` | frontière + blocs squelettes | « Ce produit n'a pas d'application » + ajouter | produit introuvable → carte + recherche | `REV-409` | application manquante → bloc fantôme |
| `/teams/:id` | liste + carte squelettes | « Cette équipe ne possède pas encore de produit » | équipe introuvable → carte | — | produit retiré → bandeau |
| `/value-streams/:id` | étapes squelettes | « Ajoutez la première étape » | value stream introuvable | — | étape sans produit → signal |
| `/capabilities/:id` | hiérarchie squelettes | « Aucune capacité réalisée » | capacité introuvable | — | couverture partielle → signal |
| `/applications/:id` | composants squelettes | « Application feuille » + ajouter un composant | application introuvable | — | nœud manquant → badge |
| `/infrastructure/:id` | nœuds squelettes | « Aucun nœud décrit » | nœud introuvable | — | application non déployée → signal |
| `/business-domains/:id` | liste squelettes | « Aucun objet classifié » | domaine introuvable | — | objet sans domaine → signal |
| `/portfolios/:id` | liste squelettes | « Aucun produit » | portefeuille introuvable | — | produit sans portefeuille → signal |
| `/interfaces/:id` | versions squelettes | « Aucune version » + créer | interface introuvable | — | version non publiée → bandeau |
| `/design/:id` | canevas + barre d'état | croquis d'exemple du modèle | vue introuvable → Atelier + recherche | `REV-409` : recharger/comparer/forcer | objets manquants → formes fantômes |
| `/library` | couvertures squelettes | recherche + filtres | erreur de lecture → bandeau | — | 0 résultat → élargir les filtres |
| `/control` | tables squelettes | « Aucune anomalie » + couverture | erreur → code + action | — | couverture partielle → profil manquant |
| Story / Revue | blocs squelettes | invitation à ajouter une vue | vue source retirée → bloc « source retirée » | — | cadrage orphelin → cadrage par défaut |

Aucun état vide n'affiche « aucune donnée » nu (NFR-027) : chaque état vide porte une
phrase et une action.

## 15. Accessibilité

- Parité clavier complète (tableau §11) ; aucun geste de navigation ou de conception
  exclusivement souris.
- Focus visible partout (`--focus-ring`, 2 px, offset 2 px) ; l'ordre de tabulation suit
  lentilles → carte → inspecteur → barre d'état.
- Contrastes AA ; les états temporels, les cycles de vie et les statuts ont motif + texte.
- `prefers-reduced-motion` : transitions à 0 ms, aucun mouvement automatique.
- La carte expose une **liste alternative** des objets du niveau courant (mode « liste »)
  pour les lecteurs d'écran et la navigation clavier linéaire.
- Les changements d'état sont annoncés (`aria-live="polite"`) : enregistrement, refus,
  zoom, publication, conflit.
- Aucun piège de focus : `Échap` ferme toujours l'élément contextuel.

## 16. Anti-CMDB, anti-tableau de bord — checklist de revue

Une porte est **NO-GO** si l'une de ces réponses est fausse :

1. `/` montre-t-il la carte de l'entreprise, et non un inventaire, un tableau de bord ou une
   galerie d'intentions ?
2. Peut-on zoomer de l'entreprise à l'infrastructure sans changer d'outil ni perdre le
   contexte ?
3. Les lentilles changent-elles l'encodage sans déplacer aucun objet ?
4. Une route agrégée se déplie-t-elle vers des interfaces et versions réelles ?
5. Un produit est-il possédé par une équipe, et non par un domaine ?
6. Une application est-elle utilisée par des produits, et non possédée par eux ?
7. Peut-on créer un produit, une application ou une interface depuis la carte sans modal et
   sans changer de route ?
8. La suppression d'une forme laisse-t-elle l'objet intact ?
9. La palette est-elle dérivée du métamodèle chargé, sans classe codée en dur ?
10. Les bandes, quartiers, frontières et ports sont-ils des objets de la carte, pas des
    réglages cachés ?
11. Le croquis est-il visible comme croquis, et refusé à la publication ?
12. Les états temporels et les cycles de vie sont-ils lisibles sans couleur seule ni fiche ?
13. La Bibliothèque et le Contrôle sont-ils des espaces secondaires ?
14. Aucune dépendance nouvelle (layout, graphe, icônes, virtualisation) sans ADR ?
15. Aucune chaîne en dur, aucun token en dur, aucun accès dépôt depuis une vue ?

## 17. Légende de carte (toujours accessible)

La légende est un panneau discret de la barre d'état, jamais un modal :

- quartiers, bâtiments, blocs, utilités ;
- routes agrégées et routes dépliées ;
- ports et consommations ;
- lentilles actives et encodages (couleur, épaisseur, motif) ;
- états temporels (motif + mot) et cycles de vie ;
- statuts de formalisation (croquis, brouillon, validé, publié).

Elle est aussi rendue dans `/_design` et dans l'export Markdown de la carte.

## 18. Ce que ce document ne fixe pas

- Les polices exactes et la marque (tokens, hors périmètre).
- Le contenu rédactionnel des exemples (analyste, `i18n/fr.json`).
- L'algorithme de placement des nouveaux objets et le routage avancé des arêtes.
- Les icônes normatives : aucune reproduction (risque de licence) ; les glyphes du profil
  restent des pictogrammes internes.
- La taxonomie produit au-delà de trois niveaux.
- La synchronisation CMDB et la découverte automatique (hors cible v0.4).
