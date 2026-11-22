# Proposition — Archifier, une carte que l'on dessine

| | |
|---|---|
| **Date** | 21 novembre 2026 |
| **Objet** | Clarifier l'intention et faire de la carte la surface principale de travail |
| **Statut** | Proposition — à arbitrer |
| **Décisions liées** | ADR-023 (la carte comme point d'entrée), ADR-022 (croquis → objet) |
| **Maquette** | `docs/proposition-maquette.html` (à ouvrir dans un navigateur) |

> **Suite** : les tranches 1 et 2 (canevas, ancres, sélection, barre unique, palette,
> création inline, lien, renommage) sont implémentées. Voir
> `docs/carte-directe-slice-1.md` et `docs/carte-directe-slice-2.md`.

> **Alignement ADR-024/025 (21/11/2026).** Proposition **non normative**. L'accueil et
> l'onboarding courants sont fixés par ADR-024/025 et `docs/ui-spec.md` §13.1/§13.13 : `/`
> est le **portefeuille de produits** filtrable, `/map` est la carte, et le parcours propose
> **produit → équipe → value stream**. Les écarts de cette proposition (notamment « pas de
> parcours en trois étapes ») restent des pistes à arbitrer, pas des règles.

---

## En une phrase

> **Archifier est une carte blanche pour dessiner son entreprise : on pose, on relie, on entre.
> Le référentiel se remplit derrière.**

Pas un assistant. Pas un parcours en trois étapes. Pas un formulaire avant la première forme.
Une surface, trois gestes, zéro prérequis.

---

## 1. Le diagnostic : le problème n'est pas le modèle, c'est le chemin

Le modèle d'Archifier est solide : produits, équipes, value streams, applications, interfaces,
capacités, nœuds, temporalité, validation. Le problème est ce que l'écran demande **avant**
de produire une carte.

### 1.1 L'écran parle plus qu'il ne montre

Aujourd'hui, la carte ouvre sur :

- six entrées de navigation (Carte, Atelier, Bibliothèque, Contrôle, Recherche, Réglages) ;
- un fil d'Ariane, un compteur de produits et de routes ;
- un sélecteur de lentille parmi dix ;
- un sélecteur de date ;
- un bouton « ports », un bouton « liste », un menu « ⋯ » ;
- une légende permanente ;
- un panneau de création flottant ;
- des tiroirs pour les signaux, la liste, les routes, les ports ;
- un parcours d'accueil en trois étapes (équipe, value stream, produit).

Chacun de ces éléments est justifié isolément. Ensemble, ils donnent un **tableau de bord
d'administration** là où l'on attend une **carte**. L'utilisateur doit lire le processus avant
de pouvoir poser un objet.

### 1.2 La carte est en lecture seule

La carte est une **projection** : on la regarde, on clique pour ouvrir une page, mais on ne la
dessine pas. Le glisser-déposer des ancres existe côté commandes et persistance, mais n'est pas
exposé. Pour créer, il faut ouvrir le panneau de création ; pour relier, il faut aller sur la
fiche d'une interface ; pour déplacer, il faut passer par l'Atelier.

C'est exactement l'inverse d'un outil de cartographie. On cartographie **en bougeant des choses**,
pas en remplissant des formulaires.

### 1.3 Le produit raconte trois histoires

| Surface | Ce qu'elle raconte |
|---|---|
| Carte | « voici l'entreprise, elle est générée » |
| Atelier | « dessinez une vue, mais nommez d'abord un domaine » |
| Contrôle / Bibliothèque | « voici un inventaire et des anomalies » |

Trois vocabulaires, trois modèles de lecture, parfois trois référentiels. L'intention devient
illisible parce qu'elle est **répartie**. Le mot « carte » devrait suffire.

### 1.4 Le verrou : le processus est le produit

Le parcours actuel dit, dans cet ordre :

1. créez une équipe ;
2. créez un value stream ;
3. créez un produit ;
4. choisissez une lentille ;
5. ouvrez l'Atelier pour dessiner.

Or un architecte qui découvre l'outil veut d'abord **poser trois boîtes et deux flèches** pour
voir si l'outil pense comme lui. Tant que ce geste n'est pas possible en dix secondes, le
processus est perçu comme une barrière, pas comme une méthode.

---

## 2. La proposition : la carte directe

### 2.1 Principe

**La carte est l'éditeur.** Elle n'est plus une projection en lecture seule ni un préalable à
l'Atelier. On y pose, on y relie, on y entre. Le modèle s'écrit au fil des gestes, avec un
niveau de qualification progressif.

Ce n'est pas un renoncement au modèle :

- la carte reste une **projection du référentiel** ; les positions manuelles sont des **ancres**,
  pas une seconde vérité ;
- chaque forme est **un objet** ; chaque route est **une interface** ;
- la sémantique manquante est marquée **« à qualifier »**, jamais « anomalie » ;
- la géographie reste stable ; le zoom change la requête et le détail, pas la position ;
- l'Atelier ne disparaît pas : il devient la **présentation** et l'édition avancée, pas le passage
  obligé.

### 2.2 Anatomie de l'écran

```text
┌────────────────────────────────────────────────────────────────────────────┐
│  ◧ Archifier   Entreprise / Sinistre          Rechercher… ⌘K   Calques  RS │  ← une seule barre
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│      ┌ Parcours Sinistre ──────────────────────────────┐                   │
│      │                                                │   ┌─────────────┐   │
│      │   ┌──────────────┐        ┌──────────────┐     │   │  Panneau    │   │
│      │   │ Sinistre     │───────▶│ Sinistre API │     │   │  de sélection│  │
│      │   └──────────────┘        └──────┬───────┘     │   │             │   │
│      │   ┌──────────────┐               │             │   │  Titre      │   │
│      │   │ Souscription │───────────────┘             │   │  Type       │   │
│      │   └──────────────┘                             │   │  Équipe     │   │
│      └────────────────────────────────────────────────┘   │  État       │   │
│                                                            └─────────────┘   │
│   Double-cliquez pour poser un objet · Glissez un point bleu pour relier    │
│                                                   −   100 %   +   Ajuster    │
└────────────────────────────────────────────────────────────────────────────┘
```

À l'état de repos, il ne reste que :

1. la marque et le fil d'Ariane ;
2. la recherche (qui sert aussi à créer) ;
3. le bouton « Calques » ;
4. le zoom et « Ajuster ».

Tout le reste est **contextuel** : il apparaît sur sélection, sur survol d'une route, ou à la
demande.

### 2.3 Les trois gestes

| Geste | Ce que l'on voit | Ce qui est écrit |
|---|---|---|
| **Poser** — double-clic sur le vide (ou `⌘K` → type) | une carte apparaît à l'endroit cliqué, titre éditable sur place | un objet, type proposé par le contexte, état « croquis » |
| **Relier** — glisser le point bleu d'une carte vers une autre | une route se dessine, lâcher sur une carte la crée | une interface avec fournisseur, consommateur, version brouillon |
| **Entrer** — double-clic sur une carte | le canevas se recentre, les enfants apparaissent, le fil d'Ariane change | une requête de niveau, aucune écriture métier |

Gestes de soutien :

| Geste | Effet |
|---|---|
| glisser une carte | déplace et persiste l'ancre |
| molette / pincement | zoom |
| espace + glisser, ou molette + glisser | déplacement du canevas |
| lasso puis « Grouper » | crée un quartier (value stream, domaine, équipe, environnement) |
| `F2` ou double-clic sur le titre | renomme sur place |
| `Suppr` | retire la **représentation**, jamais l'objet |
| `⌘Z` / `⌘⇧Z` | annule / rétablit |
| `Échap` | ferme le panneau ou quitte le mode courant |

### 2.4 Le lien visuel écrit un objet

C'est la règle qui protège le modèle : **on ne dessine pas une flèche, on crée une interface.**

- À l'échelle produit/application, le glisser d'un point bleu crée une `Interface` avec un
  fournisseur et un consommateur ; la version est `0.0.1-brouillon` tant qu'elle n'est pas qualifiée.
- À l'échelle équipe/capacité, le même geste crée une relation typée (`POSSÈDE`, `COUVRE`,
  `DÉPLOIE`), avec le même badge « à qualifier ».
- Une route agrégée reste dépliable : le clic montre les interfaces réelles, leurs versions et
  leurs consommateurs. On ne perd pas la vérité derrière une flèche.

Le panneau de la route tient en quatre champs : nom, fournisseur → consommateur, version, état.
Le reste (contrat, SLA, classification) est dans la fiche, à la demande.

### 2.5 Le texte vient après le geste

Le reproche « du texte et des contrôles partout » se traite par une règle simple :
**une information n'apparaît que si elle est utile au geste en cours.**

| Moment | Ce que la carte montre |
|---|---|
| repos | forme, icône de type, nom. Rien d'autre. |
| survol | le point de liaison, une ombre, le nom complet si tronqué |
| sélection | un panneau de quatre champs : titre, type, propriétaire, état |
| route sélectionnée | fournisseur → consommateur, version, état, « à qualifier » |
| calque activé | une seule dimension en plus : équipe, interfaces, temps ou signaux |
| fiche ouverte | tout le reste : contrats, versions, historique, anomalies, données |

Les codes (`PRD-006`, `MAP-002`) ne sont jamais dans la phrase principale. Ils sont dans le
détail technique, pour l'équipe.

### 2.6 Les dix lentilles deviennent quatre calques

Les dix lentilles mélangent trois choses différentes : des **niveaux**, des **types d'objet** et
des **dimensions de lecture**. C'est une taxonomie utile pour le modèle, mais un mauvais menu.

| Lentille actuelle | Devient |
|---|---|
| Valeur | type « value stream » + quartier dessiné |
| Capacité | type « capacité » |
| Produit | niveau Produit (zoom) |
| Applicatif | niveau Application (zoom) |
| Technologie | niveau Infrastructure (zoom) |
| Organisation | calque « Organisation » |
| Interface | calque « Interfaces » |
| Temps | calque « Temps » |
| Risque / coût | calque « Signaux » |
| Données | propriété d'interface et d'application, dans le panneau |

Il reste donc :

- **cinq niveaux** de zoom : Entreprise, Portefeuille, Produit, Application, Infrastructure ;
- **cinq types de formes** : Produit, Équipe, Application, Capacité, Nœud ; les interfaces sont des
  **routes**, pas des boîtes ;
- **quatre calques** optionnels, tous désactivés par défaut : Organisation, Interfaces, Temps,
  Signaux.

Un calque **ajoute** une lecture ; il ne change ni la géographie ni le mode. On peut en activer
plusieurs, mais on n'a jamais à en choisir un avant de commencer.

### 2.6 bis Ce que ce n'est pas

- **Pas un tableau blanc générique** : chaque forme est un objet du référentiel, chaque route est
  une interface. On ne perd pas le modèle en gagnant la liberté.
- **Pas une CMDB exhaustive** : la carte est un plan de travail, pas un inventaire de 5 000
  serveurs. Les vues larges passent par les quartiers, les filtres et le zoom.
- **Pas un éditeur de diagrammes** : on ne dessine pas des flèches décoratives ; on relie des
  objets, et la relation a un sens (fournisseur, consommateur, version, état).

### 2.7 L'onboarding est une phrase

```text
Carte vide
  « Double-cliquez pour poser un premier objet. »
       ↓ premier objet
  « Tirez le point bleu pour relier. »
       ↓ première route
  (plus rien)
```

- Aucun champ obligatoire avant la première forme.
- Un produit peut exister sans équipe : il est marqué « à qualifier », en neutre, pas en anomalie.
- Un value stream n'est pas un prérequis : c'est un quartier que l'on dessine quand on en a besoin.
- « Charger un exemple » reste discret, en bas de la carte, jamais automatique.
- La complétude est un **état de travail**, pas une erreur. Les signaux n'apparaissent que si on
  ouvre le calque « Signaux ».

### 2.8 Les pages d'objet deviennent des zooms

Une page d'objet n'est plus un tableau de bord de quatre cartes. C'est :

1. **un panneau de focus** ancré à la sélection, qui garde la carte derrière ;
2. **quatre champs** utiles au moment présent ;
3. **un lien « Ouvrir la fiche »** pour le travail profond (contrats, versions, historique,
   anomalies, données).

Le double-clic sur une carte **entre** dans l'objet : le canevas se recentre, les enfants
apparaissent, le fil d'Ariane se met à jour. On ne quitte pas la carte pour comprendre la carte.

### 2.9 L'Atelier change de rôle

L'Atelier reste utile pour :

- une vue authored narrative (revue, story, présentation) ;
- un export soigné ;
- un cadrage précis avec des cadres et des annotations.

Il ne doit plus être :

- le point d'entrée ;
- un détour obligatoire pour dessiner ;
- un concurrent de la carte avec son propre vocabulaire.

Proposition : **l'Atelier devient « Présentation »**, accessible depuis une carte ou une fiche,
et les vues authored deviennent des **cadres posés sur la carte** ou des exports. Le menu
principal ne le liste plus au même niveau que la carte.

---

## 3. Avant / après

| Aujourd'hui | Proposition |
|---|---|
| 6 entrées de navigation | 1 surface + recherche + menu secondaire |
| 10 lentilles au choix | 4 calques optionnels, désactivés par défaut |
| parcours d'accueil en 3 étapes | 2 phrases contextuelles, 0 étape obligatoire |
| sélecteur de date permanent | calque Temps |
| compteur de signaux permanent | calque Signaux |
| boutons ports / liste / ⋯ | recherche unifiée + panneau contextuel |
| panneau de création flottant | double-clic + palette `⌘K` |
| 4 tiroirs | 1 panneau de sélection |
| 5 pages d'objet en tableau de bord | 1 panneau de focus + fiche optionnelle |
| carte en lecture seule | carte éditable, ancres persistées |
| 2 surfaces d'édition (carte / Atelier) | 1 surface d'édition (carte) + présentation |
| flèche décorative | interface réelle, fournisseur/consommateur/version |
| complétude = anomalie | complétude = état « à qualifier » |

---

## 4. Ce que l'on garde

La proposition ne jette pas l'architecture existante. Elle garde :

- le **référentiel unique** et la validation dans `archifier-core` ;
- la **projection** de carte, les quartiers, les routes, les ports, la géographie stable ;
- le **zoom sémantique** à cinq niveaux et les liens profonds `#/map?…` ;
- les **interfaces de premier rang** (fournisseur, consommateurs, versions, exposition) ;
- la **temporalité** et les états `ACTIVE / SCHEDULED / EXPIRED` ;
- le **bus de commandes** et la règle « toute écriture de layout passe par les commandes » ;
- l'**annulation/rétablissement**, l'enregistrement immédiat, la gestion de conflit ;
- l'**accessibilité** : formes focusables, clavier, alternatives textuelles, pas de couleur seule ;
- les **signaux** et la revue, mais à la demande ;
- les **tests nommés** de l'ADR-023.

---

## 5. Conséquences techniques

### 5.1 Commandes de carte

Ajouter au bus existant (`design/commands.rs` ou un module `map/commands.rs` qui partage la
validation) :

| Commande | Effet |
|---|---|
| `PlaceObject` | crée un objet à une position, état croquis |
| `MoveAnchor` | déplace une représentation et persiste l'ancre |
| `RenameObject` | renomme sans changer l'identité |
| `SetObjectType` | qualifie un objet « à qualifier » |
| `ConnectObjects` | crée une interface ou une relation typée |
| `GroupDistrict` | crée un quartier et rattache les objets |
| `RemoveRepresentation` | retire la forme, conserve l'objet |

La règle existante reste : **le canvas ne lit pas le dépôt et n'écrit jamais le layout
directement**. Les gestes passent par les commandes, qui valident et persistent.

### 5.2 Projection

`MapProjection` accepte désormais :

- une **ancre manuelle** par nœud, prioritaire sur l'heuristique de placement ;
- un **placement automatique** pour les objets sans ancre ;
- une requête simplifiée : `scope`, `level`, `focus`, `layers`, `date` ;
- les lentilles deviennent des **calques** (`layers`) et non un choix exclusif.

La géographie reste stable : changer de calque ne déplace rien. `MoveAnchor` est la seule
commande qui déplace une forme.

### 5.3 Écrans et modules

| Fichier | Changement |
|---|---|
| `crates/ui/src/map/page.rs` | scinder en `MapChrome` (barre unique), `MapCanvas` (gestes), `MapInspector` (sélection) |
| `crates/ui/src/map/create.rs` | remplacer le panneau flottant par la création inline + palette `⌘K` |
| `crates/ui/src/map/pages.rs` | convertir les tableaux de bord en panneaux de focus ; garder les fiches profondes |
| `crates/ui/src/components.rs` | réduire la navigation globale ; recherche globale en palette |
| `crates/ui/src/routes/atelier.rs` | sortir du menu principal ; entrée contextuelle « Présenter » |
| `crates/core/src/map.rs` | ancres, calques, projection simplifiée |
| `crates/repo` | persister les ancres ; un seul index de recherche |

### 5.4 Tests d'acceptation

- `map_double_click_places_object` — un double-clic crée un objet sans formulaire.
- `map_drag_persists_anchor` — déplacer une carte persiste son ancre.
- `map_link_creates_interface` — un glisser crée une interface, pas une flèche anonyme.
- `map_every_edge_has_an_object` — aucune route sans objet.
- `map_rename_is_inline` — `F2` renomme sans changer de route.
- `map_no_required_field_before_first_object` — aucun champ obligatoire avant la première forme.
- `map_rest_state_has_at_most_three_controls` — l'état de repos tient en trois contrôles.
- `map_zoom_keeps_geometry` — le zoom ne déplace aucun objet.
- `map_keyboard_create_and_link` — le parcours est possible au clavier.
- `map_remove_representation_keeps_object` — supprimer une forme ne supprime pas l'objet.
- `map_layer_does_not_change_geometry` — activer un calque ne déplace rien.

### 5.5 Critères d'acceptation produit

1. Un nouvel utilisateur pose son premier objet en **moins de dix secondes**, sans lire de
   paragraphe ni ouvrir de formulaire.
2. Cinq objets et trois routes sont créés **sans quitter le canevas** et sans modal.
3. À l'état de repos, le chrome occupe **moins de 10 %** de l'écran et montre **au plus trois
   contrôles**.
4. Toute écriture est **annulable** et **persistée** immédiatement.
5. Le zoom change le **détail**, jamais la **géographie**.
6. Aucun écran ne présente un manque de complétude comme une erreur.
7. Le parcours clavier couvre poser, relier, renommer, entrer et ouvrir la fiche.

---

## 6. Le premier incrément

Objectif : une démonstration crédible en **quatre semaines**, sans refonte du moteur.

### Semaine 1 — Le canevas respire

- zoom, déplacement, ajustement ;
- sélection, focus, clavier de base ;
- glisser d'une carte et persistance de l'ancre ;
- barre unique : marque, fil d'Ariane, recherche, calques ;
- retrait des contrôles permanents (lentille, date, ports, liste, `⋯`).

**Démo :** déplacer trois produits et zoomer sans perdre la géographie.

### Semaine 2 — Poser et relier

- double-clic pour créer, titre éditable sur place ;
- palette `⌘K` (créer / rechercher) ;
- point de liaison, route temporaire, création d'interface ;
- panneau de sélection à quatre champs ;
- panneau de route (fournisseur, consommateur, version, état) ;
- badge « à qualifier ».

**Démo :** créer cinq objets et trois interfaces sans quitter la carte.

### Semaine 3 — Le calme

- calques Organisation, Interfaces, Temps, Signaux (off par défaut) ;
- signaux et complétude séparés, jamais en anomalies par défaut ;
- indices contextuels (deux phrases), pas de parcours guidé ;
- pages d'objet en panneaux de focus ;
- recherche unifiée.

**Démo :** un nouvel utilisateur arrive sur une carte vide et produit une carte lisible en
moins d'une minute.

### Semaine 4 — Entrer et présenter

- zoom sémantique en place (double-clic) ;
- quartiers par lasso + « Grouper » ;
- Atelier sorti du menu, entrée « Présenter » ;
- export et partage conservés ;
- passe d'accessibilité et de copie.

**Démo :** parcours complet Entreprise → Produit → Application → Infrastructure sans changer
d'outil.

---

## 7. Décisions à prendre

1. **Le type est-il demandé au moment du geste ou après ?**
   Recommandation : après. Le contexte propose un type, l'utilisateur peut le changer d'un clic.
   Aucun champ n'est obligatoire avant la première forme.

2. **La carte est-elle unique ou par périmètre ?**
   Recommandation : une seule carte, les périmètres sont des zooms. Les liens profonds
   conservent le périmètre, le niveau et le focus.

3. **L'Atelier devient-il « Présentation » ou reste-t-il un éditeur avancé ?**
   Recommandation : les deux, mais hors du parcours principal. La carte est l'édition ;
   l'Atelier est la mise en récit et l'export.

4. **Un lien visuel crée-t-il toujours une interface ?**
   Recommandation : oui aux niveaux produit/application ; ailleurs, une relation typée.
   Dans tous les cas, un objet est créé et porte un badge « à qualifier ».

5. **Que fait-on des dix lentilles ?**
   Recommandation : les replier en types, niveaux et quatre calques. Garder les noms dans le
   modèle et la documentation, pas dans le menu.

---

## 8. Risques et parades

| Risque | Parade |
|---|---|
| « Une carte libre est un dessin qui ment » | chaque forme est un objet ; badge « à qualifier » ; le calque Signaux révèle les manques ; la fiche montre la vérité |
| « On perd la carte générée et stable » | les ancres sont optionnelles ; placement automatique par défaut ; commande « Réorganiser » ; le zoom ne déplace rien |
| « À 500 objets, c'est illisible » | zoom sémantique, quartiers, filtres, calques, clusters ; la carte est un plan de travail, pas une CMDB exhaustive |
| « Le canvas n'est pas accessible » | formes focusables, ordre de tabulation, alternatives textuelles, commandes clavier, recherche comme chemin parallèle |
| « Deux modèles cohabitent encore » | un seul index, un seul bus de commandes, l'Atelier lit la carte ; aucun vocabulaire parallèle |
| « Les utilisateurs cassent le modèle » | validation dans le cœur, annulation, brouillon explicite, pas de suppression d'objet par suppression de forme |

---

## 9. Ce que la maquette montre

Ouvrir `docs/proposition-maquette.html` dans un navigateur. Elle est interactive :

1. **barre unique** : marque, fil d'Ariane, recherche, calques, avatar ;
2. **carte plein écran** : quartier, produits, applications, équipe, capacité, nœud ;
3. **double-clic** sur le vide : pose un objet, titre éditable sur place ;
4. **point bleu** : glisser d'une carte à une autre crée une interface « à qualifier » ;
5. **sélection** : panneau de quatre champs ; le reste est dans la fiche ;
6. **route** : cliquer une route montre fournisseur, consommateur, version, état ;
7. **calques** : Organisation, Interfaces, Temps, Signaux — tous désactivés au départ ;
8. **`⌘K` / `Ctrl+K`** : créer ou rechercher, sans quitter la carte ;
9. **double-clic sur une carte** : entre dans l'objet, recentre le canevas, change le fil d'Ariane.

La maquette n'est pas le produit : c'est la démonstration de la direction. Les données sont
fictives, la persistance absente, l'accessibilité à parfaire.

---

## 10. En une phrase

**Archifier n'a pas besoin d'expliquer son processus ; il a besoin de laisser dessiner.
Une carte, trois gestes, et le modèle se précise au fur et à mesure.**
