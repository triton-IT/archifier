# ADR-020 — Virtualisation du canvas : culling par viewport, sans dépendance

| | |
|---|---|
| Statut | **Proposé pour W7** |
| Date | 18 novembre 2026 |
| Remplace | Aucun ADR ; précise la décision ADR-008 du §20 de `technical-specifications.md` et le §16 de `technical-specifications-lot2.md` |
| Portée | Rendu SVG des diagrammes ; performance de navigation, pas de persistance |
| Décisions liées | ADR-014 (layout collaboratif) ; `technical-specifications-lot2.md` §3.3 (viewport local) |

## Contexte

`NFR-007` exige une navigation interactive sur un diagramme de 500 représentations sans
blocage perceptible. Le rendu est SVG : chaque nœud, arête, libellé et marqueur est un
élément DOM. À 500 nœuds, le coût de création, de mise en page et de peinture devient le
facteur limitant, avant même les calculs de layout.

Le produit dispose déjà d'une intention forte : « SVG, culling au-delà de 200 nœuds »
(ADR-008 du §20). Elle mélange deux mécanismes distincts :

- le **culling** : ne pas monter ce qui n'est pas visible ;
- le **niveau de détail** (LOD) : simplifier ce qui est visible.

Le MVP2 doit trancher quand chacun s'applique, sans ajouter de bibliothèque de
virtualisation. Une dépendance de virtualisation générique est calibrée pour des listes
linéaires, pas pour un graphe 2D avec pan, zoom, sélection et arêtes. Elle ajouterait du
poids au bundle et une abstraction à contourner pour les arêtes.

La contrainte la plus importante est ailleurs : **le layout reste la source de vérité**.
La virtualisation ne doit jamais déplacer, supprimer ou réécrire un nœud. Elle décide
seulement de ce qui est monté dans le DOM.

## Décision

### 1. Culling par viewport, avec marge

- Le viewport est un rectangle en coordonnées monde, dérivé du pan et du zoom courants.
- Une marge fixe de **200 unités monde** est ajoutée de chaque côté. Elle évite le
  clignotement d'un nœud qui entre ou sort au bord pendant un pan lent.
- Un nœud est monté si son rectangle (`x`, `y`, `w`, `h`) intersecte le viewport élargi.
- Un nœud sélectionné, survolé ou ayant le focus est **toujours** monté, même hors
  viewport.
- Les voisins directs d'un nœud sélectionné sont montés, même hors viewport : le halo de
  sélection et les arêtes incidentes restent cohérents. « Direct » signifie une arête
  incidente, sans transitivité.
- Le culling est recalculé à chaque frame de pan, de zoom ou de déplacement, dans un
  `requestAnimationFrame`. Il n'y a pas de recalcul par nœud lors d'une frappe ou d'une
  édition de documentation.

### 2. Arêtes : une règle simple, une limite assumée

- Une arête est montée si au moins une de ses extrémités est montée.
- Une arête dont les **deux** extrémités sont hors viewport n'est pas montée, même si son
  tracé traverse le viewport. C'est la règle explicite du §16.
- Une arête sélectionnée est toujours montée, ainsi que ses deux extrémités.
- Les libellés d'arête suivent la règle de leur arête et le niveau de détail (§3).

La limite est réelle : une arête longue qui traverse l'écran entre deux nœuds hors champ
disparaît. Elle est acceptée au MVP2 parce qu'elle est rare dans les diagrammes produits et
qu'elle évite de calculer l'intersection de chaque courbe avec le viewport. Un correctif
par boîte englobante d'arête est possible plus tard, mais il changera cette règle et devra
être mesuré : il remonterait justement les arêtes les plus longues, donc les plus coûteuses.

### 3. Niveau de détail au-delà de 200 nœuds

| Nombre total de nœuds du diagramme | Libellés | Marqueurs secondaires |
|---|---|---|
| ≤ 200 | tous les libellés visibles | tous |
| > 200 | libellés du nœud sélectionné, survolé ou focalisé uniquement | réduits aux stéréotypes essentiels |

- Le seuil porte sur le **nombre total de nœuds du layout**, pas sur le nombre de nœuds
  visibles. Il est donc stable pendant un pan et ne provoque pas de bascule en rafale.
- Le LOD ne supprime jamais un nœud qui passe le culling : il simplifie son rendu.
- Les libellés d'arête suivent la même règle : sélection, survol, focus.
- Il n'y a que deux niveaux au MVP2. Un troisième niveau (mode « déplacement » sans aucun
  libellé) n'est ajouté que si la mesure l'exige.
- Le survol et le focus sont des états transitoires : ils ne sont ni persistés, ni
  synchronisés, ni exportés.

### 4. Aucune dépendance de virtualisation

- Le culling est un calcul de rectangles et d'intersections, pas une bibliothèque.
- Aucune dépendance de type `react-virtualized`, `tanstack-virtual`, `virtual-scroller` ou
  équivalent n'est ajoutée. Le bundle reste sous le budget de 4 Mio gzip.
- Aucun moteur Canvas 2D ou WebGL n'est introduit au MVP2. Le rendu reste SVG, conformément
  à ADR-008.
- Les nœuds montés conservent des clés stables : un pan ne doit pas recréer les nœuds déjà
  visibles ni perdre leur focus.
- Le pan et le zoom s'appliquent à un groupe de transformation, pas en recalculant les
  coordonnées de chaque nœud.
- La liste des nœuds à monter est dérivée du layout à chaque frame ; aucun cache persistant
  de visibilité n'est écrit dans le snapshot ni dans IndexedDB.

### 5. Le layout reste la source de vérité

La virtualisation est **purement un rendu**. Elle ne doit jamais :

- modifier `DiagramLayout.nodes`, `edges` ou `annotations` ;
- supprimer un nœud ou une arête du modèle ;
- changer un identifiant, une position ou une taille ;
- influencer un calcul de validation, de recherche, de diff ou d'export.

Le pan et le zoom mettent à jour le viewport local ; ils ne modifient ni les nœuds, ni les
arêtes, ni le layout persisté.

Conséquences directes :

- l'export SVG, l'impression et la génération Markdown utilisent le **layout complet**,
  jamais le sous-ensemble monté ;
- la recherche et le fil d'Ariane parcourent le layout complet ; sélectionner un résultat
  hors viewport force son montage et centre la vue avant de lui donner le focus ;
- les tests de convergence CRDT et de projection ignorent le culling : ils lisent le
  layout, pas le DOM ;
- le viewport est un état local de fenêtre : il n'est jamais un champ du document CRDT.
  S'il est présent dans `DiagramLayout` pour la compatibilité ou l'export, il n'est ni
  projeté ni synchronisé (`technical-specifications-lot2.md` §3.3 ; ADR-014 §3 pour la
  frontière du plan collaboratif).

### 6. Accessibilité et navigation clavier

- Le focus clavier ne dépend pas de la présence DOM : la navigation parcourt le modèle,
  puis force le montage du nœud cible avant de le focaliser.
- Un nœud focalisé est toujours monté, même hors viewport.
- L'ordre de tabulation suit le modèle ou l'ordre visuel documenté, pas l'ordre d'insertion
  dans le DOM virtualisé.
- L'interface annonce le nombre de nœuds hors viewport dans une zone de résumé accessible,
  sans prétendre qu'ils sont absents du diagramme.
- Un utilisateur de lecteur d'écran ne doit pas croire que le diagramme ne contient que les
  nœuds montés : le résumé et la navigation vers le modèle complet corrigent cette
  impression.

### 7. Budgets et dégradation

| Mesure | Cible | Méthode |
|---|---|---|
| 500 nœuds | 60 fps en pan, zoom et déplacement | trace Chromium sur la machine de référence G7 |
| Frame p95 | ≤ 16,7 ms | mesure sur 2 s d'interaction continue |
| Frame maximale | ≤ 50 ms | aucune frame longue pendant l'interaction |
| Ouverture de fiche | < 100 ms | mesure locale |
| Recherche | < 50 ms | mesure locale |

Ordre de dégradation si le budget casse :

1. réduire les libellés au sélectionné / survolé / focus, même sous 200 nœuds ;
2. masquer les marqueurs secondaires et les ombres ;
3. masquer les libellés d'arête ;
4. réduire la marge de préchargement ;
5. documenter l'écart et demander un arbitrage : aucune dépendance de virtualisation n'est
   ajoutée sans ADR.

On ne dégrade jamais en retirant des nœuds du **layout**. Le pire cas d'un nœud très
connecté (halo et voisins nombreux) est mesuré explicitement ; s'il casse le budget, la
règle de voisinage est plafonnée par une révision de cet ADR.

### 8. Invariants vérifiés par des tests nommés

| Invariant | Test |
|---|---|
| Seuls les nœuds visibles sont montés | `culling_renders_only_visible_nodes` |
| Le nœud sélectionné et ses voisins restent montés | `selected_node_and_neighbors_stay_mounted` |
| Une arête avec une extrémité visible est rendue | `edge_with_one_visible_endpoint_is_rendered` |
| Une arête avec deux extrémités hors viewport n'est pas rendue | `edge_with_both_endpoints_hidden_is_not_rendered` |
| Le LOD réduit les libellés au-delà de 200 nœuds | `lod_reduces_labels_above_200_nodes` |
| Le culling ne modifie pas le layout | `culling_does_not_mutate_layout` |
| L'export utilise le layout complet | `export_uses_full_layout` |
| Le focus force le montage d'un nœud hors viewport | `focus_forces_offscreen_node_mount` |
| Le pan ne recrée pas les nœuds visibles | `pan_preserves_mounted_node_identity` |
| Aucune dépendance de virtualisation n'est ajoutée | `no_virtualization_dependency` |
| 500 nœuds restent interactifs | `canvas_500_nodes_is_responsive` |

## Conséquences

### Positives

- Le coût DOM suit la surface visible, pas la taille totale du diagramme.
- Aucune dépendance nouvelle, donc pas de poids de bundle ni d'abstraction à contourner.
- Le layout reste l'unique source de vérité : export, recherche, validation et CRDT ne
  dépendent pas du rendu.
- Le comportement est déterministe et testable par des rectangles, sans horloge ni
  bibliothèque externe.
- Le LOD est simple : deux niveaux, un seuil stable, des états transitoires non persistés.

### Négatives, acceptées

- Une arête qui traverse le viewport entre deux nœuds hors champ n'est pas rendue. C'est
  une limite visuelle assumée.
- Les nœuds hors viewport n'existent pas dans le DOM : les tests E2E et l'accessibilité
  doivent passer par le modèle, pas par un sélecteur DOM naïf.
- Le halo de sélection et les voisins directs peuvent surcharger le rendu d'un nœud très
  connecté ; ce cas est mesuré et pourra être plafonné.
- Le seuil de 200 nœuds est une discontinuité visible : les libellés apparaissent et
  disparaissent d'un coup au passage du seuil.
- La mesure de 60 fps dépend de la machine de référence ; elle n'est pas une garantie sur
  tout poste.

## Ce que cet ADR ne couvre pas

- L'algorithme de layout automatique et le routage des arêtes.
- Le rendu Canvas 2D ou WebGL, une scène 3D ou un moteur de graphe externe.
- La mini-carte, le zoom sémantique et le rendu hors écran.
- L'export SVG complet, l'impression et la génération Markdown, qui utilisent le layout
  complet mais ont leurs propres budgets.
- La synchronisation du viewport entre fenêtres : il est local
  (`technical-specifications-lot2.md` §3.3).
- La sélection de texte, l'édition inline et les interactions riches dans le canvas.
- Les tests de performance sur matériel bas de gamme.

## Références

- `functional-specifications.md` — `NFR-007`, navigation de diagramme à 500 représentations.
- `technical-specifications.md` §20 — décision ADR-008 (SVG, culling) ; §21 pour la
  collaboration et le viewport local.
- `technical-specifications-lot2.md` §16 — virtualisation et budgets.
- `docs/backlog-mvp2.md` — T6-02, `canvas_500_nodes_is_responsive`, décision D-10.
- `technical-specifications-lot2.md` §3.3 — `DiagramLayout.viewport` interdit en CRDT.
- ADR-014 — le layout est collaboratif, le viewport ne l'est pas.
