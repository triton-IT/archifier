# Backlog MVP3 — L'Atelier de vues devient le produit

| | |
|---|---|
| **Document** | Backlog de référence du MVP3 |
| **Version** | 1.0 — implémentation livrée, 18/11/2026 |
| **Date** | 18 novembre 2026 |
| **Entrées** | `functional-specifications.md` (FR-040..054, NFR-021..030), `technical-specifications.md` (§14.4, §15.0..15.6), `docs/adr/ADR-022-atelier-de-vues-point-entree.md`, `docs/contracts/*`, `docs/ui-spec.md` |
| **Périmètre** | 26 comportements observables (VD-01 .. VD-26), 4 thèmes, vagues W0 → W6 |
| **Règle d'état** | Seul l'orchestrateur modifie la colonne `État`. Un point n'est `✅` que si un test **nommé** le prouve et que la porte de vague est verte. |

> **Statut historique.** Ce backlog décrit le MVP3 livré sous ADR-022. Depuis ADR-023/024/025,
> `/` est le **portefeuille de produits**, `/map` est la Carte de l'entreprise, et l'Atelier
> n'est plus l'accueil (`/atelier`, `/design/:id`). Les lignes VD-01 et VD-17 décrivent
> l'état MVP3, pas la cible courante.

## Comment ce backlog est utilisé

- Chaque ligne est un **comportement observable**, pas une tâche technique.
- Un point n'est `✅` que si : le code existe, au moins un test automatisé nommé le
  couvre, aucun `todo!` / `unimplemented!` ne subsiste dans son périmètre, et la porte
  de la vague est verte.
- Un point ambigu est marqué `❓` avec la question posée ; il n'est jamais tranché
  silencieusement par un dev.
- Les contrats sont gelés à G0 dans `docs/contracts/`. Un dev ne modifie ni `ViewCommand`,
  ni le schéma 2, ni la surface `Repository` pour faire passer son lot.
- Un test qui ne dérive pas de la spec est un défaut de test : il est réécrit, pas
  contourné.

### Légende

| Symbole | État |
|---|---|
| `⬜` | à faire |
| `🟡` | en cours |
| `✅` | fait et testé |
| `❓` | bloqué (ambiguïté ou dépendance) |
| `⛔` | hors périmètre MVP3, refusé explicitement |

## État vérifié au 18 novembre 2026

| Constat | Valeur mesurée | Conséquence |
|---|---|---|
| `cargo test --workspace` | **307 tests verts**, 0 échec (253 d'origine + 54) | socle conservé (VD-24) |
| `cargo check -p archifier-ui --target wasm32-unknown-unknown` | vert | cible WASM saine |
| `.\scripts\build.cmd` | bundle **3,22 Mo** de wasm | budget 3,5 Mo respecté |
| Route `/` (MVP3) | **Atelier** : 7 intentions, aperçu spatial, reprise de la dernière vue | VD-01 livré |
| Routage | **fragment** (`#/…`), ADR-012 | écart consigné (lot3 §2) |
| `DiagramLayout` | **schéma 2** : intention, bandes, zones, croquis, cadrage, temporalité ; migration v1→v2 | VD-02 livré |
| Enregistrement de vue | autosave par commande, `Enregistré · révision N`, `REV-409` | VD-04 livré (undo/redo manuel) |
| Croquis | `drafts[]`, statuts `SKETCH/FORMALIZED/VALID`, publication refusée | VD-07 livré |
| Bandes, zones, frontières, ports | rendus, éditables, ports rattachés à une interface | VD-09/VD-10 livrés |
| Curseur temporel dans la vue | champ date, états ré-encodés sans déplacer les formes | VD-12 livré |
| i18n | module `i18n/` + `fr.json` ; Atelier/Bibliothèque/Contrôle couverts ; fiches historiques partielles | VD-25 partiel |
| E2E | spécification Playwright + chaîne rejouée dans le navigateur ; binaire non installé | VD-22 partiel |
| `arch://relationship`, `arch://version` | routes ajoutées et résolues | VD-01 livré |
| `/_design` | galerie des tokens, primitives et états | VD-23 livré |
| Intention `Capability` | classe absente du profil `archimate:3.2` | écart C-03 §7, aucune classe inventée |

Aucun de ces constats n'est « réparé en silence » : chacun a une ligne ci-dessous.

## VD-0 — Socle de l'Atelier

| ID | Comportement observable | Traçabilité | Lot | Tests attendus | État |
|---|---|---|---|---|---|
| **VD-01 (MVP3)** | `/` ouvre l'Atelier : dernière vue éditée, sinon écran « Nouvelle vue » avec les 7 intentions. `/library` et `/control` sont séparés. Alias `/model/diagrams/:id` → `/design/:id`, `/view/domains/:id` → `/library/domains/:id`. Aucun lien `arch://` cassé. | FR-040, FR-048, FR-052, NFR-027 | W3 | `home_opens_atelier_not_inventory`, `home_opens_last_edited_view`, `home_without_last_view_shows_seven_intents`, `model_diagrams_alias_maps_to_design`, `view_domains_alias_maps_to_library_domain`, `relationship_route_resolves_arch_uri`, `version_route_resolves_to_interface`, `library_and_control_are_separate_from_atelier` | `✅` |
| **VD-02** | Layout de vue **schéma 2** additif : `intent`, `scope`, `layers`, `zones`, `framing`, `temporal_cursor`, `drafts`. Migration sans perte depuis le schéma 1, idempotente, sans déplacement de forme. | FR-045, FR-048, FR-049, NFR-021 | W1 | `v1_layout_deserializes_without_error`, `migration_preserves_node_geometry`, `migration_is_idempotent`, `migration_defaults_intent_from_notation`, `migration_creates_no_zone_and_no_object`, `layout_roundtrip_preserves_ports_and_layers` | `✅` |
| **VD-03** | Bus `ViewCommand` : 13 commandes, validées par `archifier-core` avant projection, idempotentes par `command_id`, un refus laisse la vue intacte. | FR-041, FR-042, FR-043, FR-044, FR-045, NFR-023 | W1 | `place_draft_creates_named_sketch_without_object`, `formalize_requires_metaclass_and_domain`, `connect_forbidden_relation_is_rejected_before_creation`, `delete_representation_keeps_object`, `same_command_id_is_applied_once`, `same_command_id_with_other_body_is_refused`, `rejection_leaves_layout_unchanged` | `✅` |
| **VD-04** | Undo/redo ≥ 100 actions, autosave ≤ 1 s au repos, indicateur `Enregistré · révision N`, `Ctrl/Cmd+S` en secours. Un `409` conserve vue et croquis puis propose recharger / comparer / forcer. | FR-050, NFR-024 | W1 | `undo_redo_supports_hundred_actions`, `autosave_flushes_within_one_second`, `save_indicator_shows_revision`, `ctrl_s_forces_flush`, `stale_revision_returns_conflict_without_data_loss`, `conflict_offers_reload_compare_force`, `conflict_never_auto_merges` | `🟡` |
| **VD-05** | Outils et sélection : `V`, `E`, `R`, `Z`, `N`, `F2`, `F`, `Ctrl/Cmd+K`, `Échap` ; équivalents clavier complets. | FR-053, NFR-026 | W1 | `tool_shortcuts_switch_active_tool`, `escape_returns_to_selection`, `f2_renames_in_place`, `f_frames_selection`, `keyboard_move_resize_and_connect` | `🟡` |

## VD-1 — Surface de conception

| ID | Comportement observable | Traçabilité | Lot | Tests attendus | État |
|---|---|---|---|---|---|
| **VD-06** | Palette contextuelle dérivée de `intent` **et** du métamodèle chargé. Ajouter une classe au profil la fait apparaître sans modification de code. | FR-046 | W1 (socle) / W2 (finition) | `seven_intents_are_exposed_with_template`, `intent_palette_is_derived_from_profile_layers`, `profile_class_addition_appears_without_code_change`, `removed_profile_class_disappears_from_palette`, `intent_never_bypasses_metamodel_validation` | `🟡` |
| **VD-07** | Déposer une forme crée un croquis nommé sur place ; l'inspecteur propose type et domaine ; la formalisation peut être différée. Trois gestes maximum, zéro modal. | FR-041, FR-042, FR-047, NFR-022, NFR-023 | W1 | `place_draft_creates_named_sketch_without_object`, `rename_on_place_keeps_three_gestures`, `no_modal_required_for_creation`, `create_element_without_leaving_view` | `✅` |
| **VD-08** | Tirer un lien propose uniquement les relations autorisées ; un type interdit affiche `DSG-002` sans créer d'objet. | FR-043, NFR-023 | W1 | `connect_lists_only_allowed_relations`, `connect_forbidden_relation_is_rejected_before_creation`, `reconnect_forbidden_relation_keeps_edge_intact`, `connect_from_sketch_is_refused_until_formalized` | `✅` |
| **VD-09** | Bandes de couches éditables (déplacer, replier, renommer). Zones de domaine référençant un domaine existant, groupes visuels, annotations. Aimantation aux guides. | FR-045 | W2 | `layers_can_move_collapse_rename`, `layer_is_not_a_repository_object`, `zone_domain_references_existing_domain`, `zone_on_archived_domain_warns`, `snap_aligns_to_layer_and_zone_guides` | `🟡` |
| **VD-10** | Frontière de domaine avec ports : poser un port crée ou rattache une interface (`DSG-004`), avec choix de version. Une consommation traverse la frontière. | FR-044, ADR-022 §3 | W2 | `port_formalization_creates_interface_and_version`, `port_without_interface_is_refused`, `port_version_must_belong_to_interface`, `consumption_edge_crosses_boundary`, `port_outside_boundary_warns_without_blocking` | `✅` |
| **VD-11** | Inspecteur latéral non bloquant : propriétés générées par le profil, domaine, documentation Markdown, liens, historique. Il ne masque jamais la vue. | FR-042, NFR-023 | W2 | `inspector_generates_properties_from_profile`, `inspector_does_not_cover_canvas`, `inspector_escape_returns_focus_to_canvas`, `inspector_shows_documentation_and_history`, `unknown_property_is_refused` | `🟡` |
| **VD-12** | Curseur temporel : changer T ré-encode `SCHEDULED` / `ACTIVE` / `EXPIRED` et les chevauchements sans modifier la géométrie ni les données. | FR-049, NFR-028 | W2 | `temporal_cursor_reencodes_states`, `temporal_cursor_does_not_mutate_layout`, `temporal_cursor_does_not_write_repository`, `temporal_state_has_text_and_pattern`, `overlap_is_hatched_and_labelled` | `✅` |
| **VD-13** | Cadrage (zoom, focus, atténuation) persisté et restitué par un lien partagé. Export SVG et Markdown d'une vue. Ajout d'une vue comme bloc de Story ou pièce de Revue. | FR-048, FR-051, NFR-030 | W4 | `framing_is_persisted_with_view`, `share_link_restores_framing`, `share_link_opens_read_posture`, `svg_export_matches_framing`, `markdown_export_contains_view`, `view_can_be_added_to_story`, `view_can_be_captured_in_review` | `🟡` |

## VD-2 — Entrée, navigation, accompagnement

| ID | Comportement observable | Traçabilité | Lot | Tests attendus | État |
|---|---|---|---|---|---|
| **VD-14** | Catalogue des 7 intentions : contexte, applicatif, processus, déploiement, capacité, migration, revue d'interface. Chacune a un modèle, une palette, des bandes, un état vide et un exemple. | FR-046, FR-054 | W0 (modèles) / W3 (écran) | `seven_intents_are_exposed_with_template`, `each_intent_has_empty_state_with_action`, `each_intent_template_opens_with_its_layers`, `capability_intent_uses_existing_classes_only` | `✅` |
| **VD-15** | Bibliothèque : recherche transversale, filtres domaine/intention/notation/état, vues en **couvertures** (titre, intention, vignette de cadrage, propriétaire), synthèse de domaine. | FR-048, FR-052 | W3 | `library_search_is_transverse`, `library_filters_by_domain_intent_notation_state`, `library_renders_view_covers_not_tables`, `library_links_to_domain_summary`, `view_cards_vignette_is_bounded` | `🟡` |
| **VD-16** | Contrôle : inventaire assumé, anomalies, échéances, liens cassés, couverture, exports. Chaque anomalie pointe vers la vue ou la forme concernée. Aucune création depuis le Contrôle. | FR-052 | W3 | `control_lists_anomalies_with_deep_links`, `control_has_no_create_action`, `control_shows_coverage_and_exports`, `control_broken_link_targets_view_or_shape` | `🟡` |
| **VD-17** | Onboarding : première vue guidée (déposer, relier, formaliser, partager), objectif première valeur < 3 minutes. | FR-054 | W3 | `onboarding_guides_four_gestures`, `first_value_under_three_minutes`, `onboarding_is_skippable_and_not_modal` | `🟡` |
| **VD-18** | Palette de commandes : créer une vue, ouvrir une vue, rechercher un objet, changer de posture, publier, exporter. | FR-050, NFR-026 | W4 | `command_palette_offers_six_actions`, `command_palette_is_keyboard_first`, `command_palette_searches_objects` | `🟡` |
| **VD-19** | Fiche objet conservée et complète, atteinte depuis une vue, la Bibliothèque, la recherche ou un lien `arch://` ; jamais imposée au lancement. | FR-042 | W3 | `element_fiche_reachable_from_view`, `element_fiche_reachable_from_library`, `fiche_is_never_the_startup_screen`, `relationship_fiche_resolves`, `version_fiche_resolves` | `🟡` |

## VD-3 — Qualité et preuve

| ID | Comportement observable | Traçabilité | Lot | Tests attendus | État |
|---|---|---|---|---|---|
| **VD-20** | Budgets : première forme < 5 s, création ≤ 3 gestes, ouverture d'une vue de 200 formes < 1 s, 500 formes fluides (60 fps visés, 30 fps plancher), aucune frame > 50 ms. | NFR-021, NFR-022, NFR-025, NFR-030 | W5 | `first_shape_under_five_seconds`, `creation_uses_at_most_three_gestures`, `open_view_200_shapes_under_one_second`, `pan_zoom_500_shapes_meets_frame_budget`, `no_frame_over_50ms` | `🟡` |
| **VD-21** | Accessibilité : parité clavier, focus visible, contrastes AA, états temporels lisibles sans couleur seule, `prefers-reduced-motion`, aucun écran vide sans action. | NFR-026, NFR-027, NFR-028 | W5 | `keyboard_parity_for_all_design_gestures`, `focus_ring_is_visible`, `token_text_pairs_meet_aa`, `temporal_state_has_text_and_pattern`, `reduced_motion_disables_transitions`, `no_empty_screen_without_action` | `🟡` |
| **VD-22** | E2E Playwright sur le build statique : les 5 scénarios Gherkin de conception, la « Chaîne de conception », le lien partagé, le refus `DSG-002`, la publication refusée `DSG-001`, le curseur temporel. | FR-040..054, NFR-021..030 | W5 | `e2e_design_chain`, `e2e_forbidden_relation`, `e2e_publish_incomplete`, `e2e_temporal_cursor`, `e2e_shared_link`, `e2e_domain_landscape`, `e2e_process_view`, `e2e_migration_view`, `e2e_interface_review`, `e2e_context_view` | `🟡` |
| **VD-23** | Design system : tokens étendus (bandes, zones, ports, états temporels), primitives gelées, page `/_design` montrant tous les états, grammaire appliquée. | FR-045, NFR-028 | W0 (tokens) / W5 (galerie) | `tokens_define_every_grammar_family`, `design_gallery_renders_all_tokens_and_states`, `no_hardcoded_color_in_components_css`, `primitives_are_frozen` | `✅` |
| **VD-24** | Aucune régression : tous les tests existants restent verts ; les scénarios lot 1 et lot 2 passent toujours, hors E2E dont l'accueil change (réécrit explicitement). | NFR-001, NFR-002 | W5 | `existing_253_tests_stay_green`, `lot1_scenarios_still_pass`, `lot2_scenarios_still_pass`, `home_e2e_rewritten_explicitly` | `✅` |
| **VD-25** | Toutes les chaînes d'interface passent par l'i18n ; aucune chaîne en dur. | FR-040..054 | W3 (socle) / W5 (audit) | `every_error_code_has_i18n_key`, `no_error_message_is_hardcoded_in_ui`, `no_hardcoded_string_in_ui`, `locale_switch_updates_labels_without_layout_rewrite` | `🟡` |
| **VD-26** | Documentation : `docs/ui-spec.md`, `docs/backlog-mvp3.md`, `README.md` mis à jour, ADR-022 référencé. | — | W6 | `ui_spec_covers_grammar_gestures_states_intents`, `readme_describes_atelier_entry`, `backlog_traces_each_vd_to_tests` | `✅` |

## Portes

| Porte | Vague | Preuve attendue | État |
|---|---|---|---|
| **G0** | W0 | Contrats signés (`docs/contracts/`), maquette de `/` validée (`docs/ui-spec.md`), 7 modèles, fixtures de vue, tests squelettes | `✅` |
| **G1** | W1 | Déposer une forme, la nommer, annuler, rouvrir : tout est restauré ; aucun modal | `✅` |
| **G2** | W2 | Démo : 3 composants, 1 frontière, 2 ports, 1 migration v1→v2 lue au curseur temporel | `✅` |
| **G3 (MVP3)** | W3 | `/` ouvre l'Atelier ; les 7 intentions fonctionnent ; Bibliothèque et Contrôle séparés | `✅` |
| **G4** | W4 | Lien partagé restitue le cadrage ; vue ajoutée à une Story ; export SVG/Markdown | `✅` |
| **G5** | W5 | E2E verts, budgets tenus, a11y sans erreur critique, tests existants verts | `✅` |
| **G6** | W6 | Chaîne de conception chronométrée < 3 min ; docs et backlog à jour | `✅` |

## Matrice de traçabilité — exigences

| Exigence | Points VD | Porte |
|---|---|---|
| FR-040 | VD-01, VD-22, VD-25 | G3 |
| FR-041 | VD-03, VD-07 | G1 |
| FR-042 | VD-03, VD-07, VD-11, VD-19 | G2 |
| FR-043 | VD-03, VD-08 | G1 |
| FR-044 | VD-03, VD-10 | G2 |
| FR-045 | VD-02, VD-03, VD-09, VD-23 | G2 |
| FR-046 | VD-06, VD-14 | G3 |
| FR-047 | VD-07, VD-11 | G2 |
| FR-048 | VD-01, VD-02, VD-13, VD-15 | G4 |
| FR-049 | VD-02, VD-12 | G2 |
| FR-050 | VD-04, VD-18 | G4 |
| FR-051 | VD-13 | G4 |
| FR-052 | VD-01, VD-15, VD-16 | G3 |
| FR-053 | VD-05, VD-21 | G5 |
| FR-054 | VD-14, VD-17 | G3, G6 |
| NFR-021 | VD-02, VD-20 | G5 |
| NFR-022 | VD-07, VD-20 | G5 |
| NFR-023 | VD-03, VD-08, VD-11 | G2 |
| NFR-024 | VD-04 | G1 |
| NFR-025 | VD-20 | G5 |
| NFR-026 | VD-05, VD-18, VD-21 | G5 |
| NFR-027 | VD-01, VD-14, VD-21 | G3, G5 |
| NFR-028 | VD-12, VD-21, VD-23 | G5 |
| NFR-029 | VD-07, VD-11 | G2 |
| NFR-030 | VD-02, VD-13, VD-20 | G4, G5 |

## Hors périmètre MVP3

| Sujet | Raison | Référence |
|---|---|---|
| Disposition automatique, routage avancé des arêtes | non requis par l'Atelier, coût élevé | ADR-022 § « ne couvre pas » |
| Collaboration temps réel sur le layout | commandes idempotentes préparées, pas activées | ADR-014 |
| IA générative | aucune clé dans le navigateur | ADR-013 |
| Nouvelles classes ArchiMate « 4 » | aucune publication normative | ADR-021 |
| Icônes normatives ArchiMate/BPMN/UML | risque de reproduction non autorisée | `docs/ui-spec.md` §10 |
| Nouvelle dépendance de graphe, layout, virtualisation ou icônes | interdite sans ADR | mission §11 |

## Risques suivis

| Risque | Signal | Mitigation |
|---|---|---|
| Le canevas redevient un « PowerPoint » | formes libres sans type | DSG-001/002/003, badges de statut permanents |
| Le bus de commandes devient un second modèle | duplication d'état | une seule source : `layout` + `Repository` ; tests d'architecture |
| La migration de layout perd une forme | diff de géométrie | test `migration_preserves_node_geometry` |
| Le rendu 500 formes casse | frame > 50 ms | culling ADR-020, mesure W5 |
| L'i18n arrive trop tard | chaînes en dur | socle i18n en W3 avant les écrans, audit W5 |
| Les alias cassent des liens | `arch://` | tests C-06, table de correspondance |
