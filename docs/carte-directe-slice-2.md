# Carte directe — deuxième tranche

| | |
|---|---|
| **Date** | 21 novembre 2026 |
| **Périmètre** | Semaine 2 de `docs/proposition-carte-directe.md` |
| **Statut** | Implémenté et vérifié dans le navigateur intégré |
| **Routes** | Accueil `/` = portefeuille de produits (ADR-025) ; carte = `#/map` |
| **Non commité** | Oui, les changements sont dans le répertoire de travail |

## Ce qui est livré

- **Palette `Ctrl/⌘ K`** sur la carte : créer un objet ou aller à un objet. Elle
  s'ouvre sans quitter le canevas, prend le focus, et `Échap` la referme.
- **Création au centre de la vue** depuis la palette, avec les types pertinents au
  niveau courant :
  - entreprise / portefeuille : produit, équipe ;
  - produit : application, équipe ;
  - infrastructure : nœud, équipe.
- **Double-clic sur le vide** : pose un produit (ou une équipe si le référentiel n'en a
  pas encore) à l'endroit cliqué, puis ouvre le panneau avec le nom sélectionné.
- **Rattachement contextuel** : au niveau produit, une application créée est ajoutée au
  produit courant ; elle apparaît donc immédiatement sur la carte L2.
- **Renommage immédiat** : le titre du panneau est un champ. `Entrée` ou la perte de
  focus enregistre le nom (`map_rename_object`), sans changer d'écran.
- **Relier deux objets** :
  - bouton **« Relier »** dans le panneau d'un objet ;
  - ou poignée bleue sur la carte ;
  - puis clic sur une seconde carte.
  Le geste crée une **interface réelle** avec fournisseur, consommateur, version
  `0.1.0` **Brouillon** et consommation. Ce n'est pas une flèche anonyme.
- **Panneau de route qualifiable** : nom de l'interface éditable, version, statut
  (`Brouillon` / `Publiée`), note de qualification, lien vers la fiche complète.
- **Un seul panneau** : la sélection ferme le tiroir ; ouvrir un tiroir ferme la
  sélection. La palette et le panneau se ferment au clic sur le fond.
- **Bandeau de liaison** : « Relier : cliquez une seconde carte. » avec `Annuler`.
- **Recherche dans la palette** : dès deux caractères, les actions de création
  disparaissent et les objets du référentiel sont proposés ; un objet visible est
  sélectionné sur la carte, sinon sa fiche s'ouvre.

## Modèle et dépôt

Deux commandes additives dans `crates/repo/src/map.rs` :

| Commande | Effet |
|---|---|
| `map_create_interface_between(provider, consumer, level)` | crée l'interface, sa version `0.1.0` brouillon et la consommation du consommateur, en une seule mutation |
| `map_rename_object(object_id, name)` | renomme un produit, une équipe, un application, une capacité, un nœud, une interface, un composant, un value stream, un domaine, un portefeuille ou une initiative |

Tests d'acceptation ajoutés :

- `map_rename_object_updates_the_label` ;
- `map_link_creates_a_draft_interface_and_consumption`.

## Fichiers

| Fichier | Rôle |
|---|---|
| `crates/ui/src/map/palette.rs` | palette de création / recherche |
| `crates/ui/src/map/commands.rs` | `create_at`, `rename_object`, `link_objects` |
| `crates/ui/src/map/page.rs` | double-clic, poignée de liaison, panneau éditable, bandeau |
| `crates/ui/src/map/viewport.rs` | `MapLinkSource`, conversion écran → monde |
| `crates/ui/src/state.rs` | signal global `map_palette`, `Ctrl/⌘ K` sur la carte |
| `crates/repo/src/map.rs` | création d'interface entre deux objets, renommage polymorphe |
| `crates/repo/tests/map_acceptance.rs` | deux tests d'acceptation |
| `assets/map-direct.css` | palette, poignée, bandeau, panneau éditable |

## Vérifications

```powershell
cargo check -p archifier-ui --target wasm32-unknown-unknown
cargo test --workspace
.\scripts\build.ps1
```

Parcours rejoué sur `#/map` avec le référentiel d'exemple :

- `Ctrl+K` → palette ouverte, entrees « Créer un produit », « Créer une équipe » ;
- création d'un produit → 6e puis 7e carte, panneau ouvert, champ nom focalisé ;
- renommage en « Produit E2E » → titre de la carte et `aria-label` mis à jour ;
- double-clic sur le vide → nouvelle carte, panneau ouvert, champ focalisé ;
- double-clic sur un produit → carte L2 ; palette → « Créer une application » →
  6 → 7 cartes, application rattachée au produit, panneau ouvert, champ focalisé ;
- « Relier » sur un produit → bandeau, source en pointillés, clic sur un autre
  produit → route créée, toast « Interface créée en brouillon » ;
- poignée bleue → même parcours ;
- route créée → nom éditable, version `0.1.0`, badge `Brouillon`, note de
  qualification, lien vers la fiche ;
- console sans erreur.

## Limites assumées

- La palette ne propose que les types **visibles au niveau courant**. Les composants
  et les value streams restent dans le panneau « Créer » complet.
- Aux niveaux **portefeuille / value stream**, un produit créé n'est pas rattaché au
  périmètre courant : il faut le retrouver dans la recherche ou le rattacher depuis sa
  fiche.
- Le lien est **clic-clic** (ou poignée puis clic), pas encore un glisser continu.
- Seuls **produit → produit** (interface métier) et **application → application**
  (interface applicative) créent une interface. Les autres couples affichent une
  explication.
- La version est créée **Brouillon** ; sa publication et la complétion du contrat se
  font dans la fiche d'interface, pas encore depuis la carte.
- Le panneau de route ne renomme que la première interface lorsqu'il y en a une
  seule ; une route agrégée reste une liste.
- Le cadrage revient au complet quand on change de lentille ou de date ; les ancres
  restent par niveau.

## Prochaine tranche

1. Qualification d'interface dans le panneau de route : publier la version, choisir
   l'exposition, ajouter des consommateurs.
2. Lasso et « Grouper » pour dessiner un quartier.
3. Calques réellement superposables (organisation, interfaces, temps, signaux).
4. Fiches d'objet en panneaux de focus, sans quitter la carte.
