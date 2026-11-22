# Decision log — reprise v0.4

| | |
|---|---|
| Date | 19 novembre 2026 |
| Autorité | Delivery Lead, mandat de décision explicite (pas d'attente de validation intermédiaire) |
| Sources | ADR-023/024/025, `functional-specifications.md` § Cible v0.4, C-10, `docs/ui-spec.md` v2.2 |
| Portée | Ontologie native, projection de carte, routes, UI, persistance |

Le **sponsor** est le donneur d'ordre du produit. Dans cette reprise, le sponsor a
demandé que le Delivery Lead tranche lui-même et livre l'application sans lui poser de
questions intermédiaires. Les décisions ci-dessous sont donc prises, consignées et
réversibles ; elles ne sont pas des demandes d'arbitrage.

## Décisions

### D-001 — Le premier écran est le portefeuille de produits, l'Atelier devient secondaire

`#/` ouvre le portefeuille de produits filtrable (`ProductQuery::home()`, ADR-025) ; la
carte est sur `#/map` (`MapQuery::map_default()`, scope entreprise, lentille Produit). Le
produit reste le pivot (ADR-024). L'Atelier reste accessible par `#/atelier` et par les
liens des surfaces. Les tests d'accueil du MVP3 sont réécrits : ils contredisaient
ADR-023/025.

Preuve : `crates/ui/src/router.rs::home_opens_product_catalog`,
`crates/ui/src/router.rs::map_opens_enterprise_map`,
`crates/core/src/map.rs::map_opens_on_products`, complétés par
`map_default_lens_is_product`, `map_products_are_the_only_nodes_at_l0` et les tests
`catalog_*` (ADR-025).

### D-002 — Migration additive : `Dataset.map`, pas de renommage destructif

La couche native v0.4 vit dans un champ `map` additif de `Dataset`. Le `Domain` MVP3
reste lisible, ses routes restent résolues (`#/library/domains/:id`,
`#/view/domains/:id` → page domaine métier). Aucun snapshot MVP3 n'est invalidé.

Preuve : `Dataset` sérialise `#[serde(default)] map` ; test
`map_commands_survive_reload` ; alias de route dans `legacy_domain_routes_still_resolve`.

### D-003 — Deux types d'interface, un seul graphe

`archifier_core::Interface` reste le binding de métamodèle MVP3.
`archifier_core::map::MapInterface` porte le niveau de navigation, l'exposition, le
fournisseur polymorphe et les versions de la carte. Aucune duplication d'objet : les
deux couches sont reliées par la même identité de référentiel et la carte ne crée pas de
diagramme.

### D-004 — Géographie déterministe, lentilles non géométriques

Le placement est calculé par quartier + grille stable, puis remplacé par les ancres
persistées (`MapAnchor`). Une lentille ne touche ni `x`, ni `y`, ni `w`, ni `h`.
Les liens profonds sérialisent `scope`, `level`, `lens`, `date`, `focus`, `mode`.

Preuves : `map_keeps_geometry_across_lenses`, `map_temporal_cursor_does_not_mutate_layout`,
`projection_is_deterministic`, `map_deep_link_restores_context`.

### D-005 — Un niveau n'est proposé que si le périmètre porte son objet racine

Depuis un produit, on ne peut pas basculer sur le niveau Application sans avoir cliqué
une application. Cela évite une projection vide et respecte MAP-002/MAP-006 : le zoom
est sémantique, pas un simple changement d'échelle. Les niveaux incompatibles sont
grisés avec leur raison.

### D-006 — Les produits plateformes ne portent pas PRD-033

Un produit `PLATFORM` est une utilité de la carte : il vit dans « Plateformes
partagées » et n'est pas signalé comme produit sans value stream. Il reste soumis à
PRD-007 (il doit exposer une interface).

### D-007 — L'exemple est un choix explicite, jamais un seed silencieux

Le premier lancement reste vide. Un bouton « Charger un exemple » crée un référentiel
lisible (assurance) et ne s'exécute que si la carte est vide : aucune donnée utilisateur
n'est écrasée.

### D-008 — Création inline non modale

La création depuis la carte s'ouvre dans un panneau du flux (`CreateTray`), sans
`<dialog>`, sans changement de route imposé. Les commandes sont validées par le dépôt
(PRD-001, PRD-009, PRD-024/025/026) avant toute écriture.

### D-009 — La preuve de porte est exécutable

Une porte se juge sur `cargo test --workspace`, le build WASM et un parcours navigateur,
pas sur une capture. Le détail est dans `docs/v0.4/implementation-report.md`.

### D-010 — L'accueil est le portefeuille de produits

`/` rend le catalogue produits filtrable (`ProductQuery::home()`) ; `/map` rend la carte
(`MapQuery::map_default()`). Le value stream reste un quartier de carte et une facette de
filtre, pas le sommaire du portefeuille. Décision : ADR-025.

## Questions ouvertes (non bloquantes)

1. **Placement manuel** : les ancres sont persistées, mais le glisser-déposer de la
   carte n'est pas encore activé dans cette tranche. Le contrat MAP-022 est prêt
   (`map_set_anchor`), l'interaction viendra après la revue UX.
2. **Filtres** : le catalogue expose les filtres relationnels (ADR-025) ; la carte n'expose
   encore que la date et les lentilles. Les filtres de carte seront ajoutés sans changer
   la géométrie.
3. **Recherche globale** : la recherche de la carte couvre produits, équipes,
   applications, interfaces et value streams. Les éléments ArchiMate restent dans la
   recherche historique (`#/search`).
4. **Lentilles superposées** : la projection accepte plusieurs lentilles, l'UI en
   applique une comme primaire. La superposition visuelle complète (couleur + arêtes +
   filtre) est un raffinement d'encodage, pas un changement de modèle.
5. **Métamodèle** : `Capability` et `ValueStream` restent natifs ; aucune classe
   ArchiMate 4 n'est inventée.
