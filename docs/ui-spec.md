# Spécification d'interface — L'Atelier de vues (MVP3)

| | |
|---|---|
| **Document** | Grammaire visuelle, gestes, états et intentions de l'Atelier |
| **Version** | 1.0 — base G0, 18 novembre 2026 |
| **Statut** | **v1.0 — appliquée par l'implémentation du 18/11/2026** |
| **Sources** | `functional-specifications.md` § « L'Atelier de vues », FR-040..054, NFR-021..030 ; `technical-specifications.md` §14.4, §15.0..15.6 ; ADR-022 ; `docs/contracts/*` |
| **Opposable en revue** | Oui. Une violation est un motif de NO-GO (voir §14). |

Ce document ne décrit pas des écrans isolés : il décrit **une surface de conception** et
les règles qui empêchent qu'elle redevienne un back-office. Les contrats de données sont
dans `docs/contracts/` ; ce document fixe ce que l'architecte **voit** et **fait**.

---

## 1. Principes

1. **L'Atelier est le point d'entrée.** `/` ouvre une surface de conception, jamais un
   tableau de bord, jamais une liste de domaines, jamais un inventaire.
2. **On conçoit tout depuis une vue.** Aucune création n'exige de quitter le canevas ni
   d'ouvrir un formulaire modal.
3. **Le dessin est la surface, le référentiel est la vérité.** Une forme non identifiée est
   un croquis visible comme tel ; supprimer une forme ne supprime jamais un objet.
4. **Formalisation progressive.** `Croquis → Brouillon formalisé → Objet valide`, visible
   en permanence, sans ambiguïté sur ce qui est publié.
5. **La vue a une intention et une grammaire.** Bandes de couches, zones de domaine,
   frontières et ports, temps encodé, densité progressive.
6. **Deux postures, une seule surface.** CONCEPTION et LECTURE conservent vue, cadrage,
   curseur temporel et sélection. Aucun droit ne change.
7. **L'inventaire est secondaire.** Bibliothèque et Contrôle existent, jamais comme
   passage obligé.

## 2. Grille, espace, densité

- Grille de monde : **8 px**, aimantation active par défaut (`grid.size = 8`).
- Espacement d'interface : multiples de `--step` (4 px), tokens `--space-1..8`.
- Une vue commence par **un cadrage lisible** (zoom 0.85–1.0), pas par un zoom à 100 %
  sur un coin vide.
- Les bandes occupent toute la largeur du monde ; une forme peut déborder une bande
  (avertissement visuel, pas une erreur).
- L'inspecteur fait `--inspector-width` (320 px) et **réduit** la zone de canevas, il ne
  la recouvre pas. La palette fait `--palette-width` (232 px), repliable.
- La barre d'état fait `--statusbar-height` (28 px) et reste visible en permanence.

## 3. Hiérarchie typographique

| Niveau | Rôle | Taille / graisse | Couleur |
|---|---|---|---|
| 1 | Nom de la vue (barre d'outils) | `--text-xl` / `--weight-strong` | `--ink` |
| 2 | Nom d'objet sur la forme | `--node-title-size` / `--node-title-weight` | `--ink` |
| 3 | Type d'objet | `--node-type-size` / `--weight-regular` | `--ink-soft` |
| 4 | Domaine | `--node-domain-size` / `--weight-regular` | `--muted` |
| 5 | État temporel | `--node-status-size` / `--weight-medium` | badge + motif |
| 6 | Annotation | `--text-sm` italique | `--muted` |

Une information critique n'existe **jamais** uniquement dans une infobulle ni uniquement
par la couleur. Le type et l'état sont écrits.

## 4. Zoom sémantique — trois niveaux

| Zoom | Nom | Ce qui est visible | Ce qui disparaît |
|---|---|---|---|
| `< --zoom-aggregate-max` (0.55) | **Agrégat** | Bandes, zones, libellés de domaine, compteurs par bande | noms individuels, ports, types |
| `0.55 … 1.15` | **Standard** | Nom, glyphe, type, domaine, état | propriétés, versions de port |
| `> --zoom-detail-min` (1.15) | **Détail** | Propriétés essentielles, versions de port, documentation tronquée, liens | rien de plus (pas d'empilement de panneaux) |

Règles :

- Le passage d'un niveau à l'autre est **continu** (opacité et taille), jamais un
  « mode » à activer.
- À l'agrégat, un clic sur un agrégat **zoome** sur son contenu ; il ne l'ouvre pas dans
  une fiche.
- Le zoom sémantique ne modifie ni le layout, ni le cadrage persisté.

## 5. Bandes de couches

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

## 6. Zones de domaine et groupes

- **Zone `DOMAIN`** : territoire d'un domaine existant. Elle porte le nom du domaine, une
  teinte stable (`--zone-palette-N`), une bordure (`--zone-domain-border`). Elle ne crée
  jamais de domaine.
- **Zone `GROUP`** : regroupement visuel (couloir, environnement, lot). Bordure
  pointillée, sans teinte de domaine.
- Une zone se redimensionne par ses poignées ; ses enfants ne se déplacent pas
  implicitement (pas de « groupe » au sens outil de dessin).
- Une zone `DOMAIN` archivée affiche un bandeau discret « domaine archivé »
  (`DGM-008-W`), sans bloquer la lecture.
- Une zone peut être repliée ; les ports restent ancrés et visibles sur la frontière.

## 7. Frontières et ports

- La **frontière** est le périmètre d'une zone `DOMAIN` (`--boundary-stroke`, pointillé).
- Un **port** est un petit cercle posé sur la frontière (`--port-*`) ; il représente une
  interface exposée, éventuellement une version précise.
- Poser un port propose immédiatement :
  1. rattacher une interface existante (recherche contextuelle, sans quitter la vue) ;
  2. ou créer une interface (nom + catégorie) et sa première version dans le même geste.
     Sans l'un des deux : `DSG-004`.
- Le port affiche le nom de l'interface au survol **et** un `aria-label` complet ; la
  version (`v2`) apparaît au zoom de détail.
- Une **consommation** est une arête qui traverse la frontière
  (`--port-consumer-stroke`, pointillé). Elle ne crée pas de port côté consommateur : le
  lien part du composant consommateur et pointe le port.
- Un port sélectionné montre ses consommateurs en surbrillance, sans ouvrir de fiche.

## 8. Temps dans la vue

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
- Changer T ne modifie ni la géométrie, ni les données, ni la révision de la vue tant que
  l'autosave n'a pas enregistré le seul champ `temporal_cursor`.
- Le curseur s'applique aux formes **et** aux arêtes (une relation peut être planifiée).

## 9. Croquis, brouillon, validé

| État | Forme | Badge | Ce que l'utilisateur peut faire |
|---|---|---|---|
| `SKETCH` | contour pointillé, libellé italique | « Croquis » | renommer, déplacer, formaliser, supprimer la représentation |
| `FORMALIZED` | forme pleine | « Brouillon » | compléter les propriétés, publier impossible |
| `VALID` | forme pleine, aucun signal | « Validé » | publier, exporter, raconter |
| Publié | badge « Publié » dans la barre d'état de la vue | — | partager, retirer |

Un croquis ne se distingue pas par une teinte « brouillon » : il se distingue par son
**trait**, son **badge** et son **absence d'identité** (pas de type, pas de domaine).

## 10. Les sept intentions

Le catalogue est défini par C-03. Chaque intention a : un **aperçu spatial** (bandes,
zones, croquis d'exemple), une palette filtrée, un état vide, un modèle JSON.

| Intention | Ce que la vue doit montrer d'emblée | État vide (phrase d'invitation) |
|---|---|---|
| **Contexte** | pour qui, pourquoi, avec quoi ; bande Motivation dominante | « Posez d'abord les acteurs et les objectifs, puis les composants qui les servent. » |
| **Applicatif** | composants, services, une frontière de domaine | « Déposez les composants du domaine, nommez-les, reliez-les. » |
| **Processus** | couloirs, événement de début, tâches | « Commencez par l'événement de début, puis les tâches dans l'ordre. » |
| **Déploiement** | nœuds d'exécution, artefacts, environnements | « Placez les nœuds d'exécution, puis glissez les composants déployés. » |
| **Capacité** | services rendus, objectifs, lots | « Cartographiez ce que l'entreprise sait faire comme des services rendus. » |
| **Migration** | existant, cible, lots, curseur temporel ouvert | « Posez l'existant, puis l'état cible ; le curseur montre la bascule. » |
| **Revue d'interface** | frontière, ports, versions, consommateurs | « Dessinez la frontière, posez un port par interface exposée. » |

Aucune intention ne diffère d'une autre **uniquement par son titre** : bandes, palette,
zones et exemple changent (interdit §15.6).

## 11. Gestes

| Geste | Souris | Clavier | Résultat | Refus attendu |
|---|---|---|---|---|
| Sélectionner | clic / rectangle | `V`, `Maj`+flèches | sélection simple/multiple | — |
| Déposer un élément | glisser depuis la palette | `E` puis flèches, `Entrée` | croquis nommé sur place | `DGM-005` si géométrie invalide |
| Nommer | double-clic sur le libellé | `F2` | édition sur place, sans modal | `OBJ-002` si vide |
| Relier | tirer d'un nœud à un autre | `R`, source, cible, `Entrée` | relation autorisée uniquement | `DSG-002` |
| Relier un croquis | tirer depuis un croquis | `R` | propose « Formaliser puis relier » | `DSG-002` (`DRAFT_NOT_FORMALIZED`) |
| Formaliser | inspecteur, bouton « Formaliser » | `Tab` vers l'inspecteur, `Entrée` | croquis → brouillon/valide | `DSG-001`, `MM-*` |
| Poser une zone | `Z` puis tracer | `Z`, flèches | zone DOMAIN ou GROUP | `DGM-008` |
| Poser une annotation | double-clic sur le fond | `N` | annotation éditable | — |
| Cadrer | `Maj`+clic sur zone, « Cadrer » | `F` | cadrage persisté | — |
| Curseur temporel | champ date | focus barre, flèches | ré-encodage d'état | `DGM-012` si date illisible |
| Publier | menu de vue | `Ctrl/Cmd+Maj+P` | vue publiée + lien | `DSG-001`, `DSG-003` |
| Partager | menu de vue | `Ctrl/Cmd+Maj+L` | lien avec cadrage | — |
| Palette de commandes | barre | `Ctrl/Cmd+K` | actions, objets, vues | — |
| Annuler / rétablir | — | `Ctrl/Cmd+Z` / `Ctrl/Cmd+Maj+Z` | ≥ 100 actions | — |
| Enregistrer (secours) | — | `Ctrl/Cmd+S` | flush immédiat | `REV-409` |
| Revenir à la sélection | — | `Échap` | outil Sélection | — |

Trois gestes maximum pour créer un élément typé : **déposer → nommer → choisir le type**
(NFR-022). Le choix du type peut être différé : le croquis reste, la publication attend.

## 12. Inspecteur

Panneau latéral droit, non bloquant, sections dans cet ordre :

1. **Identité** — nom (éditable), type, domaine, statut de formalisation.
2. **Propriétés** — générées depuis le profil ; champ requis marqué ; propriété inconnue
   refusée (`MM-PROP-001`).
3. **Documentation** — Markdown, aperçu filtré.
4. **Liens** — entrants/sortants, `arch://` résolus.
5. **Historique** — révisions, diff à la demande.

Il ne masque jamais le canevas (il réduit la zone), ne prend jamais le focus de force, et
`Échap` le referme en rendant le focus au canevas. Un formulaire modal n'existe pas dans
l'Atelier.

## 13. Écrans

### 13.1 `/` — Atelier (dernière vue ou Nouvelle vue)

L'écran « Nouvelle vue » est un **aperçu spatial**, pas une grille de cartes :

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ ◧ Archifier      Atelier   Bibliothèque   Contrôle            Claire ▾   │
├───────────────┬──────────────────────────────────────────────────────────┤
│ INTENTIONS    │  APERÇU — Applicatif d'un domaine                        │
│               │  ┌ bande Métier ────────────────────────────────────────┐│
│ ● Contexte    │  │        (acteur)        (service métier)              ││
│ ▸ Applicatif  │  ├ bande Applicatif ────────────────────────────────────┤│
│ ▸ Processus   │  │   ┌ domaine Paiements ─────────────┐   ○ port v2     ││
│ ▸ Déploiement │  │   │ (composant)   (service)        │                 ││
│ ▸ Capacité    │  │   └────────────────────────────────┘                 ││
│ ▸ Migration   │  ├ bande Technologique ─────────────────────────────────┤│
│ ▸ Revue       │  │        (nœud)                                        ││
│               │  └──────────────────────────────────────────────────────┘│
│               │  « Déposez les composants du domaine, nommez-les, … »     │
│               │  [ Créer cette vue ]   [ Voir l'exemple ]                │
├───────────────┴──────────────────────────────────────────────────────────┤
│ Reprendre  « Paiements — applicatif »  · il y a 2 h                  →   │
│ Importer une vue · Explorer la démonstration                             │
└──────────────────────────────────────────────────────────────────────────┘
```

- La dernière vue éditée est proposée **en premier** (ligne « Reprendre »), jamais une
  carte de domaine.
- Les 7 intentions sont à gauche ; l'aperçu change au clic, sans navigation.
- Aucun compteur, aucun « total d'éléments », aucune liste de domaines.
- Un premier lancement affiche l'onboarding guidé dans cet aperçu (§13.7).

### 13.2 `/design/:id` — conception

- Palette à gauche (repliable), canevas au centre, inspecteur à droite (repliable).
- Barre d'outils : titre de vue éditable, intention (badge, cliquable pour changer avec
  avertissement), outils, curseur temporel, bascule lecture, menu de vue.
- Barre d'état : `Enregistré · révision N`, nombre de croquis, alerte de conflit,
  légende temporelle, zoom.
- Aucun bouton « Enregistrer » principal ; `Ctrl/Cmd+S` est un secours discret.

### 13.3 `/view/diagrams/:id` — lecture

- Même rendu, chrome réduit : pas de palette, pas de poignées, pas d'outils de création.
- Le cadrage du lien est appliqué à l'ouverture.
- Clic sur une forme : aperçu latéral + « Ouvrir la fiche ». `Entrée` ouvre la fiche.
- Navigation par dépendances sans quitter la vue (surbrillance des chemins).
- Bascule « Modifier » ramène en conception **avec le même cadrage**.

### 13.4 `/library` — Bibliothèque

- Recherche transversale en tête.
- Filtres : domaine, intention, notation, état temporel, publication.
- Les vues sont des **couvertures** : titre, intention, vignette de cadrage, propriétaire,
  date. Pas un tableau.
- Les domaines sont atteignables par une liste courte et une synthèse, pas comme contenu
  principal.
- Une couverture ouvre la vue en lecture ; un bouton secondaire ouvre la fiche.

### 13.5 `/control` — Contrôle

- Onglets : Anomalies · Inventaire · Couverture · Exports.
- L'inventaire est **assumé** ici : c'est le seul endroit où un tableau est le contenu
  principal.
- Chaque anomalie pointe vers la vue ou la forme concernée (lien profond).
- Aucun bouton de création d'objet.

### 13.6 Fiche objet

- Atteinte depuis une forme, la Bibliothèque, la recherche ou un lien `arch://`.
- Toujours complète (identité, propriétés, documentation, liens, historique, versions,
  consommateurs).
- Jamais imposée au lancement.

### 13.7 Onboarding

Quatre gestes guidés, dans l'aperçu de `/`, sans modal : déposer, relier, formaliser,
partager. Objectif : première valeur en moins de 3 minutes. « Passer » est toujours
visible et n'a pas de conséquences.

### 13.8 `/_design` — galerie interne

Page de référence du design system : tous les tokens, toutes les primitives, tous les
états (croquis, brouillon, validé, publié, temporel, conflit, erreur, vide, chargement).
Elle n'est pas dans la navigation produit. Elle sert de test visuel et de contrat.

## 14. Matrice d'états

| Écran | Chargement | Vide | Erreur | Conflit | Partiel |
|---|---|---|---|---|---|
| `/` Atelier | squelette de l'aperçu (bandes grisées) | 7 intentions + invitation | « Impossible de relire la dernière vue » + choisir une intention | — | dernière vue introuvable → intentions, bandeau « vue retirée » |
| Nouvelle vue | aperçu progressif | invitation + exemple | modèle invalide → état vide générique + anomalie au Contrôle | — | 6 intentions sur 7 disponibles → les 6 + mention |
| `/design/:id` | canevas + barre d'état | croquis d'exemple du modèle | vue introuvable → Atelier + recherche | `REV-409` : recharger/comparer/forcer | objets manquants → formes fantômes marquées |
| `/view/diagrams/:id` | canevas lecture | « Cette vue est vide » + ouvrir l'Atelier | lien invalide → `LINK-001` | — | `focus` inconnu → cadrage sans focus |
| `/library` | couvertures squelettes | recherche + intentions + importer | erreur de lecture → bandeau + réessayer | — | 0 résultat → élargir les filtres |
| `/control` | table squelettes | « Aucune anomalie » + couverture | erreur → code + action | — | couverture partielle → profil manquant listé |
| Fiche | sections squelettes | objet archivé → bandeau + restauration | introuvable → recherche | — | version non publiée signalée |
| Story / Revue | blocs squelettes | invitation à ajouter une vue | vue source retirée → bloc « source retirée » | — | cadrage orphelin → cadrage par défaut |

Aucun état vide n'affiche « aucune donnée » nu (NFR-027) : chaque état vide porte une
phrase et une action.

## 15. Accessibilité

- Parité clavier complète (tableau §11) ; aucun geste de conception exclusivement souris.
- Focus visible partout (`--focus-ring`, 2 px, offset 2 px) ; l'ordre de tabulation suit
  palette → canevas → inspecteur → barre d'état.
- Contrastes AA ; les états temporels et les statuts ont motif + texte.
- `prefers-reduced-motion` : transitions à 0 ms, aucun mouvement automatique.
- Le canevas expose une **liste alternative** des formes (mode « liste ») pour les
  lecteurs d'écran et la navigation clavier linéaire.
- Les changements d'état sont annoncés (`aria-live="polite"`) : enregistrement, refus,
  publication, conflit.
- Aucun piège de focus : `Échap` ferme toujours l'élément contextuel.

## 16. Anti-CMDB — checklist de revue

Une porte est **NO-GO** si l'une de ces réponses est fausse :

1. `/` montre-t-il une surface de conception (aperçu + intentions), et non un inventaire ?
2. Peut-on créer un élément typé sans changer de route et sans modal ?
3. La forme supprimée laisse-t-elle l'objet intact ?
4. La palette est-elle dérivée du métamodèle chargé, sans classe codée en dur ?
5. Les bandes, zones, frontières et ports sont-ils des objets de la vue, pas des réglages
   cachés ?
6. Le croquis est-il visible comme croquis, et refusé à la publication ?
7. Les états temporels sont-ils lisibles sans couleur seule ni fiche ?
8. La Bibliothèque et le Contrôle sont-ils des espaces secondaires ?
9. Aucune dépendance nouvelle (layout, graphe, icônes, virtualisation) sans ADR ?
10. Aucune chaîne en dur, aucun token en dur, aucun accès dépôt depuis une vue ?

## 17. Légende de vue (toujours accessible)

La légende est un panneau discret de la barre d'état, jamais un modal :

- états temporels (motif + mot) ;
- statuts de formalisation (croquis, brouillon, validé, publié) ;
- teintes de domaine ;
- signification des ports et des consommations.

Elle est aussi rendue dans `/_design` (VD-23) et dans l'export Markdown de la vue.

## 18. Ce que ce document ne fixe pas

- Les polices exactes et la marque (tokens, hors périmètre MVP3).
- Le contenu rédactionnel des exemples (analyste, `i18n/fr.json`).
- L'algorithme de disposition automatique (hors périmètre).
- Les icônes normatives : aucune reproduction (risque de licence) ; les glyphes du profil
  restent des pictogrammes internes.
