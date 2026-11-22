# Carte directe — première tranche

| | |
|---|---|
| **Date** | 21 novembre 2026 |
| **Périmètre** | Semaine 1 de `docs/proposition-carte-directe.md` |
| **Statut** | Implémenté et vérifié dans le navigateur intégré |
| **Non commité** | Oui, les changements sont dans le répertoire de travail |

> **Périmètre.** Cette tranche couvre le canevas, les ancres, la sélection et la barre
> unique. L'accueil (`/`) et l'onboarding restent régis par ADR-024/025 et
> `docs/ui-spec.md` §13.1/§13.13 (portefeuille de produits ; carte sur `/map` ;
> produit → équipe → value stream).

## Ce qui est livré

- **Canevas plein écran** : la carte occupe l'espace sous la barre, sans barre de lecture
  permanente.
- **Pan** : glisser le fond de la carte déplace la fenêtre.
- **Zoom** : molette sous le curseur, boutons `−` / `+`, et **Ajuster** qui recadre sur le
  contenu.
- **Déplacement d'un objet** : glisser une forme la déplace en direct ; au relâchement,
  l'ancre est persistée (`MAP-022`) et la carte est rafraîchie. Le geste passe par
  `map/commands.rs`, jamais directement par la vue.
- **Sélection** : un clic sur une forme ou une route ouvre un panneau à droite. Le panneau
  affiche l'identité, le type, l'état, les badges, et les actions « Ouvrir la fiche »,
  « Recentrer », « Retirer l'ancre ».
- **Premier cadrage** : une carte vide qui reçoit son premier objet, ou l'exemple chargé,
  se cadre une fois à 100 %. Ensuite le cadrage reste celui de l'utilisateur ; un
  déplacement ne le réinitialise plus.
- **Un seul panneau** : sélectionner un objet ferme le tiroir ouvert ; ouvrir un tiroir
  désélectionne l'objet.
- **Entrer** : double-clic (ou Entrée) sur un produit ouvre sa carte L2 ; sur une
  application, sa carte L3. Les autres objets gardent leur fiche.
- **Barre unique** : titre ou fil d'Ariane, recherche, **Calques**, **Créer**.
- **Calques** : la lentille, la date, la légende et les panneaux (signaux, liste, ports,
  à propos) sont rangés dans un popover fermé par défaut.
- **Navigation secondaire** : `Carte` et `Atelier` restent visibles ; Bibliothèque,
  Contrôle, Recherche et Réglages passent dans le menu `⋯`.
- **Clavier** : `Entrée` entre, `Espace` sélectionne, `Échap` ferme le panneau ou la
  sélection, les formes et les routes sont focusables.

## Fichiers

| Fichier | Rôle |
|---|---|
| `crates/ui/src/map/viewport.rs` | Géométrie pure : cadrage, zoom, pan, gestes, tests |
| `crates/ui/src/map/commands.rs` | Écritures d'ancre (`move_anchor`, `clear_anchor`) |
| `crates/ui/src/map/page.rs` | Barre unique, canevas gestuel, inspecteur, tiroirs |
| `crates/ui/src/map/mod.rs` | Déclaration des modules |
| `crates/ui/src/components.rs` | Navigation secondaire dans `⋯` |
| `crates/ui/tests/architecture.rs` | Garde : aucune ancre écrite hors `map/commands.rs` |
| `assets/map-direct.css` | Styles de la carte directe, chargés après `app.css` |
| `index.html` | Chargement de `map-direct.css` |

## Vérifications

```powershell
cargo check -p archifier-ui --target wasm32-unknown-unknown
cargo test --workspace
.\scripts\build.ps1
```

- 18 tests unitaires UI, dont 4 sur la géométrie de la carte.
- 9 tests d'architecture, dont le nouveau `map_layout_writes_go_through_map_commands`.
- 12 tests d'acceptation de la carte (`cargo test -p archifier-repo --test map_acceptance`).
- Parcours rejoué dans le navigateur intégré sur le référentiel d'exemple :
  - 5 produits, 6 routes rendus ;
  - sélection d'un produit → panneau, canvas qui se redimensionne ;
  - glisser un produit → déplacement immédiat, toast « Position enregistrée », badge
    « Ancre manuelle » ;
  - rechargement → l'ancre est toujours là ;
  - « Retirer l'ancre » → retour au placement automatique ;
  - double-clic produit → `#/map?scope=PRODUCT:…&level=PRODUCT` ;
  - double-clic application → `#/map?scope=APPLICATION:…&level=APPLICATION` ;
  - Calques → changement de lentille, la géographie des objets reste stable ;
  - Liste → tiroir ; clic sur un objet → le tiroir se ferme, le panneau s'ouvre ;
  - console sans erreur.

## Limites assumées de cette tranche

- La **création inline** (double-clic pour poser un objet) reste le panneau « Créer » :
  c'est la tranche 2.
- Les **calques ne sont pas encore des superpositions indépendantes** : la lentille reste
  un choix de lecture unique. Le modèle accepte plusieurs lentilles, l'UI n'en expose
  qu'une.
- La **géographie n'est pas conservée entre niveaux** : une ancre est persistée par
  `(objet, niveau)`. Un produit déplacé à L0 peut avoir une autre position à L1.
- Le **cadrage est réinitialisé** quand on change de lentille ou de date, car la route
  remonte la page. Les objets ne bougent pas, mais la fenêtre revient au cadrage complet.
- Les **pages d'objet** restent des tableaux de bord : la tranche 3 les transformera en
  panneaux de focus.
- Pas encore de **lasso / Grouper**, ni de **relier par glisser** (tranche 2).
- L'**Atelier** reste dans la navigation : sa mise en retrait est prévue en tranche 4.

## Prochaine tranche

1. Double-clic sur le vide → poser un objet, titre éditable sur place.
2. Point de liaison → créer une interface réelle avec fournisseur/consommateur.
3. Palette `Ctrl/⌘ K` pour créer ou rechercher sans quitter la carte.
4. Inspecteur de route avec qualification d'une interface brouillon.
