# Backlog MVP2 — Archifier v0.2

| | |
|---|---|
| **Document** | Backlog de référence du MVP2 |
| **Version** | 0.7 — W1 complet, G2 franchie |
| **Date** | 18 novembre 2026 |
| **Entrées** | `functional-specifications.md` (§ Collaboration temps réel), `technical-specifications.md` (§21), `technical-specifications-lot1-front-local.md`, `docs/adr/ADR-014-collaboration-temps-reel.md`, `backlog-mvp1.md` |
| **Périmètre** | 41 points dont 40 comportements observables + 1 spike bloquant (T7-00), 7 thèmes, lots W0 → W7 |
| **Règle d'état** | Seul l'orchestrateur modifie la colonne `État`. Un point n'est `✅` que si un test automatisé nommé le prouve. |

## Comment ce backlog est utilisé

- Chaque ligne est un **comportement observable et testable**, pas une tâche technique. Seule exception assumée : **T7-00 est un spike de mesure**, pas un comportement produit ; il est bloquant mais ne compte pas dans les 40 comportements livrés.
- Un point n'est `✅` que si : le code existe, au moins un test automatisé le couvre, aucun `todo!` / `unimplemented!` ne subsiste dans son périmètre, et la porte de vague est verte.
- Un point ambigu est marqué `⛔` avec la question posée ; il n'est jamais tranché silencieusement par un dev.
- Les contrats transverses sont gelés en G1 dans `technical-specifications-lot2.md`. Un dev ne modifie ni `archifier-core`, ni le port de dépôt, ni le format de snapshot pour faire passer son lot.
- Chaque code de règle nouveau (`OAS-*`, `ASYNC-*`, `IFACE-*`, `CR-*`, `MERGE-*`) a un test nommé d'après lui.

### Légende

| Symbole | État |
|---|---|
| ⬜ | à faire |
| 🟦 | en cours |
| ✅ | fait et testé |
| ⛔ | bloqué (ambiguïté ou dépendance) |
| ➖ | hors périmètre MVP2, refusé explicitement |

## Contrats gelés en G1

| Réf. | Contrat | Emplacement | État G1 |
|---|---|---|---|
| C-01 | Schéma CRDT par champ (types Yrs, propriétaire, projection, conflit) | `technical-specifications-lot2.md` §3 | **figé — G1** |
| C-02 | `CollabEdit` et `TextPath` | §4 | **figé — G1** |
| C-03 | `SyncTransport`, `SyncMessage`, `SyncError`, `SyncStatus` | §5 | **figé — G1** |
| C-04 | Présence éphémère (`PresenceState` + `AwarenessState`, TTL 30 s, battement 10 s) | §6 | **figé — G1** |
| C-05 | `CollabStore` + `LocalStateStore`, stores IndexedDB `collab` et `local_state`, migration additive v1 → v2 | §7 | **figé — G1** |
| C-06 | Projection vers le snapshot et regroupement d'historique (2 s / 30 éditions) | §8 | **figé — G1** |
| C-07 | `MergeDiagnostic` / `MERGE_VIOLATION`, jamais persisté, jamais correctif | §9 | **figé — G1** |
| C-08 | Port de dépôt = type `Repository` centralisé (ADR-016) | §15 | **figé — option B** |
| C-09 | Champs additifs de snapshot : OpenAPI/AsyncAPI bruts, politique de cycle de vie | §10, §11 | **figé — G1** |
| C-10 | Jeu de dépendances : `yrs 0.25` (ADR-014), `sha2 0.10`, `serde_yaml_ng 0.10` | §0.1 | **figé — approuvé** |

## T1 — Dette du lot 1 (bloquante, en premier)

| ID | Comportement observable | Traçabilité | Lot | Test(s) attendu(s) | État |
|---|---|---|---|---|---|
| T1-01 | Import OpenAPI 3.0/3.1/3.2 avec aperçu, diff, idempotence par empreinte canonique et conservation du document brut. | FR-020 ; NFR-009 | W1a | `oas_import_3_0_3_1_3_2`, `oas_import_creates_preview_without_write`, `oas_import_is_idempotent_by_fingerprint`, `oas_raw_document_is_preserved`, `oas_architecture_extensions_do_not_change_fingerprint` | ✅ |
| T1-02 | Export OpenAPI 3.1 réservé aux interfaces applicatives HTTP, avec `x-architecture-*`, reproductible octet-à-octet. | FR-021 ; NFR-009 | W1a | `oas_export_is_byte_reproducible`, `oas_export_refuses_non_http_interface`, `oas_export_injects_architecture_extensions`, `oas_061_version_without_contract` | ✅ |
| T1-03 | Diff de deux versions d'interface : compatible / rupture / inconnu, rupture confirmée explicitement par l'utilisateur. | FR-012 ; BR-010 | W1a | `oas_diff_classifies_breaking_change`, `oas_breaking_change_requires_confirmation`, `oas_diff_two_stored_versions`, `oas_unknown_change_requires_confirmation` | ✅ |
| T1-04 | Diff de deux révisions d'objet, champ par champ, à partir des snapshots. | FR-026 ; NFR-002 | W1b | `revision_diff_is_field_by_field`, `revision_diff_is_immutable`, `revision_diff_is_sorted`, `revision_diff_excludes_volatile_metadata` | ✅ |
| T1-05 | Export Markdown d'un domaine et d'un objet : arborescence, liens relatifs valides, SVG + complément Mermaid, reproductible. | FR-019 ; NFR-009 | W1b | `markdown_export_has_valid_relative_links`, `markdown_export_is_reproducible`, `markdown_export_includes_svg_and_mermaid`, `markdown_export_empty_domain_has_index` | ✅ |
| T1-06 | Diagnostic des liens `arch://` cassés ou archivés, avec extrait de contexte et accès au propriétaire. | FR-027 | W1b | `broken_link_diagnostic_has_context_and_owner`, `broken_link_diagnostic_includes_archived`, `broken_link_diagnostic_deduplicates` | ✅ |
| T1-07 | Graphe de dépendances dédié : direction, profondeur 1 à 6, chemins affichés. | FR-011 ; FR-028 | W1b | `dependency_graph_shows_paths_and_depth`, `dependency_graph_is_bounded_to_six`, `dependency_graph_both_direction_is_labelled`, `graph_route_parses` | ✅ |
| T1-08 | Multi-onglets : BroadcastChannel, détection d'écriture externe, dialogue recharger / comparer / forcer, aucune fusion automatique. | NFR-002 ; NFR-010 | W1c-2 | `external_write_is_detected`, `external_write_ignores_own_tab`, `external_write_offers_reload_compare_force`, `external_write_is_never_auto_merged`, `snapshot_exchange_is_read_only` | ✅ |
| T1-09 | Hiérarchie parent/enfant exposée (couloirs BPMN, ports UML) : profondeur 5, cycles interdits, parent archivé signalé. | FR-003 | W1c-1 | `hierarchy_depth_is_limited_to_five`, `hierarchy_cycle_is_refused`, `archived_parent_is_reported`, `hierarchy_requires_same_notation` | ✅ |
| T1-10 | Cadrage complet des blocs Story (zoom, focus, atténuation) et épinglage de révision pour figer un récit. | FR-018 | W1c-1 | `story_pins_revision`, `story_unpin_clears_pins`, `story_framing_applies_zoom_focus_dim` | ✅ |
| T1-11 | Comparaison de deux revues : filtres, fenêtres temporelles, anomalies. | FR-013 ; FR-026 | W1c-1 | `review_comparison_shows_filter_and_window_diff`, `review_comparison_lists_anomalies`, `review_comparison_rejects_unknown_review` | ✅ |

## T2 — Intelligence artificielle locale

| ID | Comportement observable | Traçabilité | Lot | Test(s) attendu(s) | État |
|---|---|---|---|---|---|
| T2-01 | Panneau conversationnel : streaming token par token, liste d'objets sources cliquables, mention explicite « réponse non sourcée » le cas échéant. | FR-022 ; NFR-011 | W6 | `ai_panel_streams_tokens`, `ai_panel_lists_clickable_sources`, `ai_panel_flags_unsourced_answer` | ⬜ |
| T2-02 | `MockProvider` scripté couvrant cinq recettes (impact, consommateurs, proposition de vue, recherche conceptuelle, explication de règle), et repli honnête listant les scénarios supportés. Aucune simulation présentée comme un LLM. | FR-022 ; FR-024 ; NFR-012 | W6 | `mock_provider_covers_five_recipes`, `mock_provider_fallback_is_honest` | ⬜ |
| T2-03 | UI de ChangeSet : diff côte à côte, Valider puis Appliquer distincts, détection STALE si une révision a bougé, aucune écriture sans confirmation. | FR-024 ; FR-025 | W6 | `changeset_ui_requires_validate_then_apply`, `changeset_ui_detects_stale_revision` | ⬜ |
| T2-04 | Proxy local optionnel `archifier-proxy` (binaire Rust, port 8787) : clé lue depuis une variable d'environnement, CORS limité à 127.0.0.1:4173, aucun prompt journalisé par défaut. Désactivé si absent. | NFR-011 ; NFR-013 | W6 | `proxy_reads_key_from_env`, `proxy_cors_is_restricted`, `proxy_does_not_log_prompts`, `proxy_is_disabled_when_absent` | ⬜ |
| T2-05 | Analyse d'impact déterministe (dépendances, temporalité, diagrammes et stories affectés) que l'IA ne fait que commenter. | FR-023 ; NFR-003 | W6 | `impact_analysis_is_deterministic`, `impact_analysis_lists_affected_diagrams_and_stories` | ⬜ |
| T2-06 | Journal d'audit des ChangeSets appliqués : acteur, opérations, avant/après. | FR-026 ; NFR-001 ; NFR-002 | W6 | `changeset_audit_records_actor_and_before_after` | ⬜ |

## T3 — Couverture des notations

| ID | Comportement observable | Traçabilité | Lot | Test(s) attendu(s) | État |
|---|---|---|---|---|---|
| T3-01 | Profil ArchiMate 3.2 étendu (~60 classes : junction, grouping, location, plateau, motivation complète) avec matrice de relations complétée. | FR-003 ; NFR-014 | W2 | `archimate_profile_has_expected_classes`, `archimate_relation_matrix_is_complete` | ⬜ |
| T3-02 | Infrastructure de migration 3.2 → 4.x : fichier de mapping, outil de diagnostic de compatibilité, aucune classe 4 déclarée. | NFR-014 | W2 | `archimate4_mapping_has_no_declared_class`, `migration_diagnostic_reports_compatibility` | ⬜ |
| T3-03 | BPMN : compléter le Core Subset annoncé (événements limites, associations de données, sous-processus), sans sortir du sous-ensemble documenté. | FR-003 | W2 | `bpmn_core_subset_is_complete`, `bpmn_out_of_subset_is_refused` | ⬜ |
| T3-04 | UML statique complet (Port, Property, Package, réalisation d'interface) avec contraintes testées. | FR-003 | W2 | `uml_static_profile_has_port_property_package`, `uml_interface_realization_is_validated` | ⬜ |
| T3-05 | Palette et inspecteur entièrement pilotés par le profil : ajouter une classe ne demande aucune modification de code. | FR-003 ; NFR-014 | W2 | `palette_reflects_runtime_profile`, `inspector_reflects_profile_properties` | ⬜ |

## T4 — Gouvernance des interfaces

| ID | Comportement observable | Traçabilité | Lot | Test(s) attendu(s) | État |
|---|---|---|---|---|---|
| T4-01 | Politique de cycle de vie : dépréciation, remplacement, date de retrait, politique de rupture, cohérence des fenêtres. | FR-008 ; BR-001 ; BR-002 | W5 | `lifecycle_retirement_window_is_consistent`, `breaking_policy_is_required`, `replacement_version_must_exist` | ⬜ |
| T4-02 | Contrat événementiel (AsyncAPI 2.x/3.x) en import/export, même modèle de conservation du brut que OpenAPI. | FR-020 ; FR-021 | W5 | `asyncapi_import_preserves_raw`, `asyncapi_export_is_reproducible`, `asyncapi_version_is_detected` | ⬜ |
| T4-03 | Revues : instantané de filtres complet, réouverture, anomalies étendues (retrait proche, rupture non consommée, consommateur orphelin). | FR-013 | W5 | `review_filter_snapshot_is_complete`, `review_reopens`, `review_extended_anomalies` | ⬜ |
| T4-04 | Centre de notifications local : échéances, expirations, ruptures, liens cassés, sans workflow d'approbation ; acquittements stockés dans `local_state` et jamais exportés. | FR-013 ; FR-027 | W5 | `notification_center_lists_deadlines_and_breakages`, `notification_center_has_no_approval_workflow`, `notification_dismissal_is_local_and_not_exported` | ⬜ |

## T5 — Recherche et navigation

| ID | Comportement observable | Traçabilité | Lot | Test(s) attendu(s) | État |
|---|---|---|---|---|---|
| T5-01 | Facettes, synonymes et historique de recherche local. | FR-030 | W5 | `search_facets_filter_results`, `search_synonyms_expand_query`, `search_history_is_local` | ⬜ |
| T5-02 | « Architecture à la date T » complète : interfaces, documentation, stories et diagrammes à l'instant choisi. | FR-029 | W5 | `architecture_at_date_includes_docs_stories_diagrams` | ⬜ |
| T5-03 | Épingles et fil d'Ariane persistant. | FR-028 | W5 | `pins_and_breadcrumb_survive_reload` | ⬜ |

## T6 — Qualité

| ID | Comportement observable | Traçabilité | Lot | Test(s) attendu(s) | État |
|---|---|---|---|---|---|
| T6-01 | Playwright E2E sur le build statique : les 4 chaînes de recette et les 8 scénarios Gherkin de la spec fonctionnelle, plus les 5 scénarios de collaboration. Les 13 scénarios sont listés dans `tests/e2e/README.md` et tracés un à un. | FR-001 → FR-039 | W7 | `tests/e2e/**` | ⬜ |
| T6-02 | Budgets : 500 nœuds sans blocage perceptible (virtualisation du canvas), ouverture de fiche < 100 ms en local, recherche < 50 ms. | NFR-005 ; NFR-006 ; NFR-007 | W7 | `canvas_500_nodes_is_responsive`, `object_sheet_under_100ms`, `search_under_50ms` | ⬜ |
| T6-03 | Accessibilité : navigation clavier complète, focus visible, contrastes AA, audit automatisé sans erreur critique. | NFR transverse | W7 | `keyboard_navigation_is_complete`, `focus_is_visible`, `axe_has_no_critical_error` | ⬜ |
| T6-04 | Chaînes centralisées : aucune chaîne d'interface en dur dans un composant. | NFR transverse | W7 | `ui_strings_are_centralized` | ⬜ |

## T7 — Collaboration temps réel, sans serveur

| ID | Comportement observable | Traçabilité | Lot | Test(s) attendu(s) | État |
|---|---|---|---|---|---|
| T7-00 | Spike CRDT borné (une journée, bloquant) : Yrs tient 20 000 caractères, 200 insertions concurrentes sans perte, mise à jour < 2 Ko, décodage < 30 ms, bundle < +900 Ko brut. Échec ⇒ bascule sur Automerge, jamais de CRDT maison. Mesures : 20 200 caractères, 17–21 o par frappe, décodage release 27–88 µs, delta WASM 296–332 Kio. | ADR-014 ; NFR-017 ; NFR-019 | W0 | `concurrent_200_insertions_into_20k_text_converge_without_loss`, `keystroke_update_stays_under_2_kib`, `decoding_20k_document_stays_under_30_ms`, `scripts/spike-bundle.ps1` (`spike_bundle_delta_under_900kb`), `insert_inside_surrogate_pair_is_refused` | ✅ |
| T7-01 | Deux plans séparés dans le code : `Repository::mutate` (gouvernance, verrou optimiste ; cible de nommage `WriteTx`) et `CollabEdit` (documentation, name, layout, blocs de Story, commentaires). Aucun chevauchement de chemin d'écriture, aucun commit mixte. | CR-001 ; CR-002 | W3 | `cr_001_governance_never_uses_crdt`, `cr_002_collab_cannot_publish_or_close_review`, `cr_002_collab_cannot_create_object` | ⬜ |
| T7-02 | `SyncTransport` + `LocalBroadcastTransport` : co-édition réelle entre deux fenêtres du même navigateur, payloads binaires, rattrapage par vecteur d'état. Moteur headless en W3, démonstration navigateur en W4. | FR-035 ; FR-036 ; FR-039 ; NFR-015 ; NFR-016 ; NFR-017 ; NFR-019 | W3/W4 | `t7_02_two_windows_converge`, `broadcast_transport_two_windows_converge`, `t7_02_state_vector_catch_up`, `t7_02_update_payload_is_bounded`, `concurrent_node_move_converges`, `concurrent_story_blocks_are_preserved`, `concurrent_comments_are_preserved`, `cr_005_same_field_converges`, `cr_006_order_independent`, `offline_edits_merge_on_reconnect`, `local_broadcast_transport_is_nominal`, `local_edit_is_immediate` | ⬜ |
| T7-03 | Présence éphémère : qui est connecté, qui édite quel objet, expiration à 30 s, jamais persistée, jamais exportée. | FR-031 ; CR-004 ; NFR-018 | W4 | `presence_expires_after_30s`, `presence_shows_edited_object`, `presence_leaves_on_close`, `presence_is_never_persisted`, `presence_is_never_exported`, `awareness_encoding_roundtrip` | ⬜ |
| T7-04 | Persistance du document CRDT dans un store `collab`, initialisation idempotente depuis le snapshot, compaction au repos, migration IndexedDB additive v1 → v2. | FR-035 ; NFR-016 ; NFR-020 | W3/W4 | `collab_doc_seed_is_idempotent`, `collab_doc_survives_reload`, `collab_doc_compacts_at_rest`, `collab_doc_loads_lazily`, `idb_collab_migration_is_additive`, `offline_state_is_not_presented_as_published` | ⬜ |
| T7-05 | Validation après fusion : diagnostic `MERGE_VIOLATION` local, objet marqué « non conforme », aucune suppression ni correction automatique. Les relations incompatibles relèvent de la gouvernance : seconde écriture refusée, aucune suppression. | FR-037 ; CR-003 | W3 | `merge_violation_is_local`, `merge_violation_does_not_delete`, `merge_violation_marks_object_non_compliant`, `incompatible_relations_are_refused_without_deletion` | ⬜ |
| T7-06 | Historique regroupé : une révision après 2 s d'inactivité, pas une par frappe ; plafond de 30 éditions ; contenu identique sans révision ; flush sur navigation et `visibilitychange` ; une seule rafale pour une période hors ligne. | FR-038 | W3 | `history_coalesces_burst_within_2s`, `history_flushes_on_navigation`, `history_caps_burst_at_30`, `history_skips_identical_content`, `history_flushes_on_visibilitychange`, `offline_edits_coalesce_into_one_revision` | ⬜ |
| T7-07 | `WebSocketTransport` derrière le même trait, non activé, testé contre un relais en mémoire. Prêt pour le lot 3, invisible pour l'utilisateur. | FR-039 | W3 | `ws_transport_uses_same_contract`, `ws_transport_is_not_wired_in_ui` | ⬜ |

## Matrice de traçabilité — collaboration

| Exigence | Points | Test bloquant |
|---|---|---|
| FR-031 Présence | T7-03 | `presence_expires_after_30s` |
| FR-032 Documentation co-éditée | T7-01, T7-02 | `t7_02_two_windows_converge` |
| FR-033 Layout co-édité | T7-01, T7-02 | `concurrent_node_move_converges` |
| FR-034 Story et commentaires co-édités | T7-01, T7-02 | `concurrent_story_blocks_are_preserved`, `concurrent_comments_are_preserved` |
| FR-035 Hors ligne puis fusion | T7-02, T7-04 | `offline_edits_merge_on_reconnect` |
| FR-036 Sans serveur | T7-02 | `local_broadcast_transport_is_nominal` |
| FR-037 Fusion non conforme signalée | T7-05 | `merge_violation_is_local` |
| FR-038 Historique regroupé | T7-06 | `history_coalesces_burst_within_2s` |
| FR-039 Relais préparé | T7-07 | `ws_transport_uses_same_contract` |
| CR-001 Gouvernance hors CRDT | T7-01 | `cr_001_governance_never_uses_crdt` |
| CR-002 Pas de publication/fermeture | T7-01 | `cr_002_collab_cannot_publish_or_close_review` |
| CR-003 Fusion ne supprime jamais | T7-05 | `merge_violation_does_not_delete` |
| CR-004 Présence éphémère | T7-03 | `presence_is_never_persisted` |
| CR-005 Même champ, valeur unique | T7-02 | `cr_005_same_field_converges` |
| CR-006 Ordre indifférent | T7-02 | `cr_006_order_independent` |
| CR-007 Hors ligne non publié | T7-02, T7-04 | `offline_state_is_not_presented_as_published` |
| NFR-015 Convergence | T7-02 | `t7_02_two_windows_converge` |
| NFR-016 Hors ligne | T7-02, T7-04 | `offline_edits_merge_on_reconnect` |
| NFR-017 Mise à jour bornée | T7-00, T7-02 | `t7_02_update_payload_is_bounded` |
| NFR-018 Présence non exportée | T7-03 | `presence_is_never_exported` |
| NFR-019 Latence locale | T7-00, T7-02 | `local_edit_is_immediate` |
| NFR-020 Chargement paresseux | T7-04 | `collab_doc_loads_lazily` |

## Découpage W0 → W7 et périmètres de fichiers

Règle : **un fichier n'a qu'un seul écrivain par vague**. Les fichiers cités sont les seuls modifiables par l'agent concerné. `Cargo.toml`, `Cargo.lock` et `assets/tokens.css` sont réservés à l'orchestrateur / UX.

| Vague | Contenu | Agents | Périmètre exclusif | Porte |
|---|---|---|---|---|
| **W0** | Audit, docs, spike CRDT T7-00 | Orchestrateur, analyste, dev moteur, testeur, relecteur | `docs/backlog-mvp2.md`, `technical-specifications-lot2.md`, `docs/adr/**`, `fixtures/mvp2/**`, `crates/collab/**` (spike uniquement), `crates/collab/tests/spike.rs`, `crates/collab/Cargo.toml` ; `Cargo.toml`/`Cargo.lock` réservés à l'orchestrateur | **G1** contrats gelés + spike vert |
| **W1** | Dette lot 1 T1-01 → T1-11 | Dev moteur, dev écrans/canvas, testeur | Repo : `crates/core/src/{openapi,contract_diff,markdown_export,history_diff,hierarchy}.rs`, `crates/repo/src/**`, `crates/repo/src/lib.rs` (écrivain unique) ; UI : `crates/ui/src/views/**`, `state.rs`, `boot.rs`, `router.rs`, `components.rs` ; tests : `crates/*/tests/**`, `fixtures/mvp2/openapi/**` | **G2** |
| **W2** | Notations T3-01 → T3-05 | Dev notations, testeur | `crates/metamodel/profiles/**`, `crates/metamodel/src/**`, `crates/core/src/validation.rs` (règles de métamodèle), `crates/ui/src/views/diagram.rs` (palette/inspecteur), `crates/metamodel/tests/**` | **G3** |
| **W3** | Collaboration cœur T7-01, 02, 04, 05, 06, 07 | Dev moteur, testeur | `crates/collab/**`, `crates/repo/src/collab.rs`, `crates/repo/src/idb_collab.rs`, `crates/repo/src/idb.rs`, `crates/repo/src/lib.rs` (intégration), `crates/repo/tests/collab*.rs` | **G4a** headless |
| **W4** | Collaboration UI + présence T7-03 | Dev UI, UX, testeur | `crates/ui/src/collab/**`, `crates/ui/src/views/{element,diagram,story,review}.rs`, `state.rs`, `boot.rs`, `components.rs`, `assets/tokens.css` (UX), `tests/e2e/collab.spec.ts` (smoke) | **G4b** deux fenêtres |
| **W5** | Gouvernance T4 + recherche T5 | Dev gouvernance, dev recherche, testeur | Gouvernance : `crates/core/src/{lifecycle,asyncapi,notifications}.rs`, `crates/repo/src/{lifecycle,notifications}.rs`, `crates/ui/src/views/{interface,review,notifications}.rs` ; Recherche : `crates/core/src/search.rs`, `crates/repo/src/search.rs`, `crates/ui/src/views/{search,home}.rs`, `crates/repo/tests/search*.rs` | **G5** |
| **W6** | IA locale T2 | Dev IA, dev changesets, testeur | IA : `crates/ai/**`, `crates/proxy/**`, `crates/ui/src/views/ai.rs` ; ChangeSets : `crates/core/src/impact.rs`, `crates/repo/src/audit.rs`, `crates/ui/src/views/changeset.rs` ; tests : `crates/ai/tests/**`, `crates/proxy/tests/**` ; ajout des membres au workspace par l'orchestrateur | **G6** |
| **W7** | Qualité T6 | Testeur, UX | `tests/e2e/**` (dont `collab.spec.ts`), `playwright.config.ts`, `package.json`, `assets/tokens.css`, `docs/ui-spec.md`, `crates/ui/src/i18n/**` | **G7** MVP2 accepté |

### Portes — critères

| Porte | Critère de franchissement |
|---|---|
| **G1** | Contrats C-01 → C-10 figés ; spike T7-00 vert (tests + `scripts/spike-bundle.ps1`) ; ADR-011/012/013/015/016 matérialisés ; dépendances approuvées. |
| **G2** | T1-01 → T1-11 ✅ ; `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, `wasm32` et build verts ; aucune régression du baseline de 72 tests MVP1 + 11 tests W0. |
| **G3** | T3-01 → T3-05 ✅ ; profil 3.2 étendu sans classe 4 ; couverture publiée dans `docs/metamodel-coverage.md`. |
| **G4a** | T7-01, 02, 04, 05, 06, 07 ✅ en headless ; `crates/collab` testable sans interface ; aucun accès UI. |
| **G4b** | T7-03 ✅ ; deux fenêtres éditent la même documentation et le même nœud et convergent ; présence visible et expirée ; zéro erreur console. |
| **G5** | T4-01 → T4-04 et T5-01 → T5-03 ✅ ; anomalies et notifications sans workflow d'approbation. |
| **G6** | T2-01 → T2-06 ✅ ; cinq recettes + repli honnête ; aucune clé dans le bundle ; proxy désactivé par défaut. |
| **G7** | T6-01 → T6-04 ✅ ; 41/41 points ; budgets et accessibilité verts ; démonstration des 4 chaînes + 8 Gherkin + 5 collaboration. |

> **G2 franchie le 18/11/2026** : T1-01 → T1-11 ✅, **253 tests**, fmt/clippy/wasm/build verts,
> bundle **2,76 Mo**. Rapports : `docs/w1a-report.md`, `docs/w1b-report.md`,
> `docs/w1c1-report.md`, `docs/w1c2-report.md`, `docs/w1-report.md`.

## Décisions G1

| Réf. | Point | Décision | Statut |
|---|---|---|---|
| O-01 | `RepositoryPort` n'existe pas dans le code : l'UI utilise `Repository` concret. | **Option B** : le port effectif est le type `Repository`, centralisé par `state::repo()` ; aucune vue n'accède au stockage. Voir `docs/adr/ADR-016-port-depot-type-repository.md`. Pas d'extraction de trait. | ✅ rendu |
| O-02 | `cargo fmt --check` et `clippy -D warnings` échouent au 18/11/2026. | Normalisation mécanique en W0, orchestrateur seul, tests verts avant et après ; aucun changement de comportement. | ✅ fait |
| O-03 | Deux `panic!` hors tests (`repo/lib.rs:509`, `ui/state.rs:29`). | Supprimés : profils illisibles ⇒ registre vide + `profile_error()` ; dépôt non installé ⇒ repli mémoire anti-panic documenté. | ✅ fait |
| O-04 | Le scénario Gherkin « relations incompatibles » relève de la gouvernance, pas de la CRDT. | **Option A** : seconde écriture refusée (`409`/validation), aucune relation supprimée ; `MERGE_VIOLATION` réservé aux vues collaboratives fusionnées. | ✅ rendu |
| O-05 | ADR-015 est cité dans `technical-specifications.md` §20 mais absent de `docs/adr/`. | **Matérialisé le 18/11/2026** : `docs/adr/ADR-015-transport-synchronisation.md`. | ✅ fait |
| O-06 | Parseur YAML et empreinte cryptographique non dépendants aujourd'hui. | **Autorisé** : `serde_yaml_ng = "0.10"` et `sha2 = "0.10"` (déjà verrouillé en 0.10.9). Voir `technical-specifications-lot2.md` §0.1. | ✅ rendu |
| O-07 | `yrs` 0.27/0.28 ne compilent pas sur Rust 1.94 (`if let` guards, E0658). | **Épinglé `0.25`** : compile et passe T7-00. Voir `technical-specifications-lot2.md` §0.1 et D-11. | ✅ fait |

## Hors périmètre — refus explicite

| Brique | Décision |
|---|---|
| RBAC / ABAC / SSO | ➖ refusé MVP2 |
| Relais réseau de collaboration activé | ➖ préparé par T7-07, non activé |
| Workflow d'approbation formel | ➖ refusé |
| Serveur applicatif de données / SQL / PostgreSQL | ➖ refusé |
| BPMN exécutable, diagrammes UML dynamiques | ➖ refusé |
| MCP Apps, AG-UI, A2UI | ➖ refusé |
| Embeddings côté serveur | ➖ refusé |
| Classe ArchiMate 4 inventée | ➖ interdit ; mapping seulement |

## Historique

| Version | Date | Changement |
|---|---|---|
| 0.1 | 18/11/2026 | Création du squelette : 41 points, 7 thèmes, contrats C-01 → C-10, découpage W0 → W7, portes G1 → G7. |
| 0.2 | 18/11/2026 | Arbitrages G1 : port de dépôt = type `Repository` (ADR-016), scénario relations = 409 + `MERGE_VIOLATION`, parseur YAML et `sha2` autorisés. W1 sans extraction de trait. |
| 0.3 | 18/11/2026 | G1 franchi : contrats C-01 → C-10 figés, spike T7-00 vert (Yrs 0.25), ADR-011/012/013 matérialisés, tests d'architecture, import persistant, reset via le port. |
| 0.4 | 18/11/2026 | W1a livré : T1-01/02/03 ✅, 22 tests OpenAPI, UI import/export/diff, bundle 2,29 Mo. |
| 0.5 | 18/11/2026 | W1b livré : T1-04/05/06/07 ✅, diff de révisions, export Markdown, liens invalides, graphe, 186 tests, bundle 2,57 Mo. |
| 0.6 | 18/11/2026 | W1c-1 livré : T1-09/10/11 ✅, hiérarchie, cadrage Story, épinglage, comparaison de revues, 230 tests, bundle 2,71 Mo. |
| 0.7 | 18/11/2026 | W1c-2 livré : T1-08 ✅, multi-onglets, 253 tests, bundle 2,76 Mo. W1 complet, G2 franchie. |
