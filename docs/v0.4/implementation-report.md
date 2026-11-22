# Rapport d'implémentation — première tranche verticale v0.4

| | |
|---|---|
| Date | 19 novembre 2026 |
| Périmètre | W0/W1/W2 : ontologie native, projection, carte L0–L4, pages d'objet, création inline |
| Statut | **GO technique** — la carte est le point d'entrée, le parcours vertical fonctionne |
| Sources | ADR-023/024/025, C-10, `docs/ui-spec.md` v2.2 |

## Ce qui est livré

### Ontologie native (`archifier-core::map`)

`Product`, `Team`, `ValueStream`, `ValueStreamStage`, `Capability`, `Application`,
`ApplicationComponent`, `MapInterface`, `MapInterfaceVersion`, `InterfaceConsumption`,
`TechnologyNode`, `BusinessDomain`, `Portfolio`, `Initiative`, `MapAnchor`.

Règles implémentées et testées : PRD-001, PRD-004, PRD-006, PRD-007, PRD-009,
PRD-011, PRD-018, PRD-019, PRD-022, PRD-024, PRD-025, PRD-026, PRD-029, PRD-031,
PRD-032, PRD-033.

### Projection (`MapQuery → MapProjection`)

- cinq niveaux : entreprise, portefeuille, produit, application, infrastructure ;
- quartiers : value streams, plateformes partagées, non rattaché, environnements ;
- nœuds typés, statuts, criticité, badges, routes canoniques ;
- arêtes agrégées par couple fournisseur/consommateur, avec `interface_ids` et
  `interface_version_ids` conservés pour le dépliage ;
- ports produit (`exposure=PRODUCT`) ;
- légende par lentille, compteurs, anomalies ;
- corrections `MAP-001`, `MAP-002`, `MAP-004`, `MAP-006` ;
- ancres persistées par objet et par niveau.

### Dépôt (`archifier-repo::map`)

- lecture : `map_data`, `map_projection`, `map_search`, `map_inventory`, `map_anomalies` ;
- écritures : `map_create_team`, `map_create_value_stream`, `map_create_product`,
  `map_create_application`, `map_create_component`, `map_create_interface`,
  `map_create_interface_version`, `map_add_interface_consumption`, `map_create_capability`,
  `map_create_node`, `map_deploy_application`, `map_add_application_to_product`,
  `map_add_capability_to_product`, `map_set_anchor`, `map_clear_anchor` ;
- persistance atomique via le snapshot existant (`Dataset.map`, additif) ;
- exemple de démonstration non destructif.

### Interface

- `#/` : portefeuille de produits filtrable (`ProductQuery::home()`, ADR-025) ;
- `#/map` : carte de l'entreprise en lecture Produits (`MapQuery::map_default()`),
  lentilles, niveaux, date, recherche, liste accessible ;
- `#/map?scope=…&level=…&lens=…&date=…&focus=…` : lien profond reproductible ;
- `#/products/:id`, `#/teams/:id`, `#/value-streams/:id`,
  `#/value-streams/:id/stages/:stageId`, `#/capabilities/:id`,
  `#/applications/:id`, `#/applications/:id/components/:componentId`,
  `#/infrastructure/:id`, `#/interfaces/:id`, `#/business-domains/:id` ;
- création inline depuis la carte : produit, équipe, value stream, application,
  interface (+ première version), capacité, nœud ;
- pages d'action : rattacher une application, déployer sur un nœud, ajouter un
  composant, publier une version ;
- liens historiques conservés : Atelier (`#/atelier`), anciens domaines.

## Preuves

| Preuve | Commande / fichier | Résultat |
|---|---|---|
| Tests unitaires carte et catalogue | `cargo test -p archifier-core map::` | 15 tests carte verts à la livraison, dont les invariants nommés C-10 ; tests `catalog_*` ajoutés par ADR-025 |
| Tests d'acceptation dépôt | `cargo test -p archifier-repo --test map_acceptance` | 10 tests verts |
| Tests UI / routes | `cargo test -p archifier-ui` | 34 tests verts |
| Suite workspace | `cargo test --workspace` | à exécuter en porte G2 |
| Build WASM | `scripts\build.cmd` | bundle produit dans `pkg/` (≈4 Mo) |
| Parcours navigateur | `#/` → « Charger un exemple » → portefeuille produits | 5 produits, filtres opérationnels, 0 erreur console |
| Lentille stable | L0 Produit → L0 Organisation | coordonnées `x,y` identiques pour les 6 nœuds |
| L2 avec dépendance | produit → carte L2 | 6 nœuds, quartier « Dépendances externes », arête Payment Hub → Portail Client dépliable |
| L3 / L4 | application → composants ; nœud → L4 | nœud feuille visible, arêtes réseau inter-DC |
| Deep link | `#/map?scope=PRODUCT:<id>&level=PRODUCT&lens=INTERFACE` | contexte restauré, breadcrumb et focus |
| Création inline | carte → « Créer » → produit | produit créé, redirection vers sa page, aucun modal |
| Création inline | carte → « Créer » → interface | interface + première version créées, 0 erreur console |
| Action de page | interface → « Publier la version » | 2.2.0 listée immédiatement, révision persistée |

### Invariants nommés

| Test | Emplacement | Résultat |
|---|---|---|
| `home_opens_product_catalog` | router | ajouté ADR-025 |
| `map_opens_enterprise_map` | router | vert |
| `map_opens_on_products` | core | vert |
| `map_default_lens_is_product` | core | vert |
| `map_products_are_the_only_nodes_at_l0` | core | vert |
| `catalog_filters_by_team` / `catalog_filters_by_value_stream` | core | ajouté ADR-025 |
| `catalog_filters_by_capability` / `catalog_filters_by_application` | core | ajouté ADR-025 |
| `catalog_filter_or_within_facet` / `catalog_filter_and_across_facets` | core | ajouté ADR-025 |
| `home_deep_link_restores_product_filters` | router | ajouté ADR-025 |
| `map_zooms_enterprise_to_infrastructure` | core + repo | vert |
| `map_keeps_geometry_across_lenses` | core + repo | vert |
| `map_aggregate_edge_expands_to_interfaces` | core + repo | vert |
| `map_does_not_show_components_at_enterprise_level` | core | vert |
| `map_deep_link_restores_context` | core + repo + router | vert |
| `map_temporal_cursor_does_not_mutate_layout` | core | vert |
| `product_requires_accountable_team` | core + repo | vert |
| `application_is_used_by_products_not_owned` | core + repo | vert |
| `interface_consumption_requires_version` | core + repo | vert |
| `domain_is_classification_not_owner` | core + repo | vert |
| `legacy_domain_routes_still_resolve` | router | vert |
| `first_run_creates_product_not_domain` | repo | vert |

## Écarts assumés

| Écart | Pourquoi | Suite |
|---|---|---|
| Pas de glisser-déposer d'ancre | La commande et le stockage existent ; l'interaction demande une revue UX et une gestion clavier | W3 |
| Filtres produit/cycle de vie non exposés | La requête les porte déjà ; l'UI garde la date et les lentilles pour ne pas surcharger la première tranche | W3 |
| Lentille primaire seulement | La projection accepte N lentilles ; l'encodage superposé complet est un raffinement | W3 |
| Capability map visuelle | La page capacité et la lentille existent ; la heatmap complète est un travail de rendu | W3 |
| Analyse d'impact chemin/dates | Les consommations versionnées sont prêtes ; l'algorithme de chemins est hors première tranche | W3 |
| Seuil MAP-011 (>500 nœuds) | Non instrumenté dans cette tranche ; la projection reste déterministe mais n'agrège pas encore au-delà du seuil | W3 |
| E2E Playwright | Non exécuté dans cet environnement ; le parcours a été rejoué dans le navigateur intégré | W4 |

## Risques

1. **Volume du snapshot** : la carte est ajoutée au snapshot JSON ; au-delà de quelques
   milliers d'objets, la sérialisation devra être mesurée (NFR-031..033).
2. **Doublons de vocabulaire** : `Interface` MVP3 et `MapInterface` v0.4 coexistent. Une
   passerelle d'import devra être spécifiée avant d'importer des interfaces historiques
   dans la carte.
3. **Migration guidée** : les domaines MVP3 restent lisibles mais ne deviennent pas
   automatiquement des produits ; c'est volontaire et signalé.

## Prochaine porte

**G2** : parcours « Entreprise → value stream → produit → interface » démontré,
changement de lentille sans déplacement, route agrégée dépliable, aucun modal,
revue UX/UI, a11y clavier, E2E du parcours.
