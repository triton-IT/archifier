# ADR-022 — L’Atelier de vues est le point d’entrée et la surface de création

| | |
|---|---|
| Statut | **Accepté** — **partiellement remplacé par ADR-023** (le point d'entrée est la Carte ; l'Atelier reste la surface d'édition) |
| Date | 18 novembre 2026 |
| Remplace | Corrige l’interprétation du § « Modes MODÉLISER et VISUALISER » de `functional-specifications.md` et le plan de routes §15.2 de `technical-specifications.md` |
| Portée | Expérience utilisateur, routage, surface de création, modèle de vue |
| Décisions liées | ADR-008 (rendu SVG), ADR-017 (proxy LLM), ADR-020 (virtualisation du canvas), ADR-011 (snapshot IndexedDB) |

> **Avenant ADR-023 (18/11/2026).** Cet ADR reste valide pour la **surface d'édition** : création depuis la vue, formalisation progressive, deux postures, commandes validées. Son §1 (l'Atelier est le point d'entrée) et son §4 (la vue a une intention comme préalable) sont remplacés : le point d'entrée est la **Carte de l'entreprise**, et les intentions deviennent des lentilles ou des actions de détail. Voir `ADR-023-produit-carte-entreprise.md`.

## Contexte

Le lot 1 a livré une application correcte mais dont le point d’entrée est un **inventaire de domaines** : cartes, compteurs, formulaires. L’utilisateur doit d’abord créer ou choisir un domaine, puis naviguer vers une vue pour commencer à travailler. Les objets se créent principalement par des formulaires, et la vue est traitée comme un rendu secondaire.

Ce parcours produit trois effets mesurables :

1. **Le produit se lit comme une CMDB.** L’inventaire est le contenu dominant ; l’architecture est une conséquence. Or une CMDB documente l’existant ; un outil d’architecture produit des vues, des arbitrages et des récits.
2. **La création est lente.** Créer un élément demande de quitter la vue, remplir un formulaire, revenir. Un architecte qui pense en schéma est interrompu à chaque geste.
3. **L’UI est austère.** Sans bandes de couches, zones de domaine, frontières ni ports, une vue ne dit rien de l’architecture : elle ressemble à un formulaire dessiné.

La spec fonctionnelle affirmait déjà que le référentiel est la source de vérité. Cette décision est conservée. Ce qui change est le **plan d’interface** : la vérité n’impose pas un parcours de saisie.

## Décision

### 1. L’Atelier de vues est le point d’entrée

La route `/` ouvre l’**Atelier** : dernière vue éditée, ou écran « Nouvelle vue ». La Bibliothèque (`/library`) et le Contrôle (`/control`) sont des espaces secondaires. Aucun écran d’inventaire n’est requis pour créer, modifier ou comprendre une architecture.

### 2. On conçoit tous les objets depuis une vue

Éléments typés, relations, interfaces et versions, domaines et zones, groupes, annotations, documentation, cadrages, Revues et Stories se créent ou s’éditent depuis le canevas. Aucune création ne doit **exiger** un formulaire modal préalable ni un changement de route. L’inspecteur est un panneau contextuel non bloquant.

### 3. Formalisation progressive, sans laxisme

Une forme suit `Croquis → Brouillon formalisé → Objet valide`. Le croquis est local, jamais publiable, jamais exporté au référentiel. La publication exige un type de métamodèle et un domaine responsable (`DSG-001`). Une relation est validée par `is_relation_allowed` au moment du geste (`DSG-002`).

### 4. La vue a une intention et une grammaire

Chaque vue porte une `intent` parmi un catalogue fermé (contexte, applicatif, processus, déploiement, capacité, migration, revue d’interface). L’intention pilote la palette, les bandes et les invitations. La grammaire visuelle est stable : bandes de couches, zones de domaine, frontières et ports, encodage temporel redondant (motif + texte), hiérarchie typographique, densité progressive.

### 5. Deux postures, une seule surface

CONCEPTION et LECTURE ne sont plus deux applications ni deux jeux de routes. Ce sont deux postures du même Atelier, avec conservation de la vue, du cadrage, du curseur temporel et de la sélection. Aucun droit ne change.

### 6. Édition fluide par commandes

Toute mutation passe par un `ViewCommand` validé par `archifier-core`. L’interface n’écrit jamais le layout. Enregistrement automatique au repos, pile d’annulation locale, palette de commandes, parité clavier.

## Conséquences

### Positives

- Le produit se juge sur sa capacité à produire une vue lisible, pas sur son exhaustivité documentaire.
- La création est mesurable : première forme < 5 s, création ≤ 3 gestes, aucun modal.
- Le référentiel reste la vérité : aucune forme non identifiée n’est publiée, aucune suppression de forme ne supprime un objet.
- La grammaire visuelle rend la temporalité et les interfaces lisibles sans ouvrir de fiche.
- Le routage est plus simple : un espace de conception, un espace de lecture, deux espaces secondaires.

### Négatives, acceptées

- Le lot 1 doit être réorganisé : l’accueil, la bibliothèque, le contrôle et le canvas changent de rôle. Ce n’est pas une retouche cosmétique.
- Les tests E2E existants qui supposent un accueil par domaine doivent être réécrits.
- Deux vocabulaires coexistent un temps (`Diagram` dans le modèle, « vue » dans l’interface) ; le modèle n’est pas renommé pour éviter une migration destructive.
- Le croquis ajoute un état local à persister et à expliquer ; il doit être visible sans ambiguïté (NFR-029).
- La densité visuelle demande un vrai travail de design system ; une implémentation paresseuse produirait un « dessin austère » au lieu d’un outil d’architecture.

## Invariants vérifiés par des tests nommés

| Invariant | Test |
|---|---|
| `/` ouvre l’Atelier, jamais un inventaire | `home_opens_atelier_not_inventory` |
| Un élément se crée sans changer de route | `create_element_without_leaving_view` |
| Aucun modal n’est requis pour créer un objet | `no_modal_required_for_creation` |
| Une relation interdite est refusée avant création | `forbidden_relation_is_rejected_before_creation` |
| Un croquis non typé ne peut pas être publié | `draft_cannot_be_published` |
| Supprimer une forme ne supprime pas l’objet | `delete_representation_keeps_object` |
| Le cadrage est restauré depuis un lien partagé | `shared_link_restores_framing` |
| Le curseur temporel ne modifie pas la géométrie | `temporal_cursor_does_not_mutate_layout` |
| L’inventaire est absent du parcours de création | `inventory_is_not_in_creation_path` |

## Ce que cet ADR ne couvre pas

- L’algorithme de disposition automatique et le routage avancé des arêtes.
- La collaboration temps réel sur le layout (ADR-014) : cet ADR prépare des commandes idempotentes, il ne l’active pas.
- Le contenu exact des sept intentions de vue : il appartient à `docs/ui-spec.md`.
- La charte de marque et les polices : les tokens restent la source.
- La migration des données : aucune migration destructive n’est autorisée.

## Références

- `functional-specifications.md` — FR-040 à FR-054, NFR-021 à NFR-030, § « L’Atelier de vues ».
- `technical-specifications.md` — §14.4 (modèle de l’Atelier), §15.0 à §15.6 (frontend, grammaire visuelle).
- `technical-specifications-lot1-front-local.md` — §7 (lots F0.5/F1), §8.6 (livrables UX).
- `docs/backlog-mvp1.md` — à compléter par les points de l’Atelier lors de la vague W0.
