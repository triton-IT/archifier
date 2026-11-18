# C-08 — Codes : `DSG-001..005`, `REV-409` et codes réutilisés

| | |
|---|---|
| **Réf.** | C-08 |
| **Version** | 1.0 — figé G0, 18/11/2026 |
| **Propriétaire** | Orchestrateur (gel) ; implémentation : core, repo, UI |
| **Sources** | `functional-specifications.md` § « Formalisation progressive », §14.2, §15.3 ; VD-03, VD-08, VD-10, VD-16 |
| **Dépend de** | C-01, C-02, C-04 |

## 1. Règle

Un code est **stable**, **affiché** et **testé par son nom**. L'interface n'invente pas de
message libre : elle affiche la clé i18n associée (`error.<code>`), complétée par un
`detail` (type fautif, champ manquant, identifiant). Un code n'est jamais traduit dans le
code Rust : seul `i18n/fr.json` porte la phrase.

Un refus :

1. laisse la vue **intacte** ;
2. n'écrit **aucun** objet ;
3. affiche le code à côté du geste concerné (inspecteur ou barre d'état), pas dans un modal ;
4. propose l'action de réparation quand elle existe.

## 2. `DSG-001` — Formalisation incomplète

| | |
|---|---|
| Déclencheur | `Formalize` sans `metaclass` ou `domain_id` valide ; `PublishView` avec un nœud dont les propriétés requises du profil sont incomplètes. |
| Message | « Cet objet ne peut pas être publié : le champ « {champ} » est requis par le profil {profil}. » |
| Détail | `path` du champ, métaclasse, profil. |
| Réparation | L'inspecteur met le champ en évidence et lui donne le focus ; aucun modal. |
| Test | `formalize_requires_metaclass_and_domain`, `publish_view_incomplete_element_is_refused`, `dsg_001_names_missing_field`. |

## 3. `DSG-002` — Relation non autorisée

| | |
|---|---|
| Déclencheur | `Connect` ou `Reconnect` dont le triplet (source, relation, cible) n'est pas autorisé par le métamodèle chargé ; `Connect` depuis/vers un croquis (`detail = DRAFT_NOT_FORMALIZED`). |
| Message | « La relation « {relation} » n'est pas autorisée entre {source} et {cible} dans {profil}. » |
| Détail | triplet fautif, profil, raison (`NOT_DECLARED`, `WRONG_DIRECTION`, `CROSS_LAYER`, `DRAFT_NOT_FORMALIZED`). |
| Effet | Aucune relation, aucune arête. La sélection reste sur la source. |
| Test | `connect_forbidden_relation_is_rejected_before_creation`, `reconnect_forbidden_relation_keeps_edge_intact`, `connect_from_sketch_is_refused_until_formalized`. |

## 4. `DSG-003` — Croquis non publiable

| | |
|---|---|
| Déclencheur | `PublishView` avec au moins un `SKETCH` ; export d'un croquis comme objet. |
| Message | « {n} croquis ne peuvent pas être publiés. Formalisez-les ou retirez leur représentation. » |
| Détail | liste des `DraftId`. |
| Effet | Publication refusée. **L'export SVG de la vue reste possible** en excluant les croquis, avec la mention « {n} croquis exclus ». Le croquis lui-même n'est jamais exporté. |
| Test | `sketch_cannot_be_published`, `sketch_is_excluded_from_svg_export`, `dsg_003_lists_draft_ids`. |

## 5. `DSG-004` — Port sans interface

| | |
|---|---|
| Déclencheur | `Formalize` d'un croquis `PORT` sans `interface_id` ni création d'interface dans le même geste ; `PlaceExisting` d'un `INTERFACE` sur une frontière avec `version_id` n'appartenant pas à l'interface. |
| Message | « Un port doit référencer une interface existante ou en créer une dans le même geste. » |
| Détail | zone, côté, version demandée. |
| Effet | Aucun port, aucune interface. |
| Test | `formalize_port_without_interface_is_refused`, `port_formalization_creates_interface_and_version`, `port_version_must_belong_to_interface`. |

## 6. `DSG-005` — Suppression de représentation

| | |
|---|---|
| Déclencheur | `DeleteRepresentation` sur un nœud, une arête, une zone ou un croquis. |
| Message | « La forme a été retirée de la vue. L'objet reste dans le référentiel. » |
| Effet | Retrait de la représentation uniquement. Aucun `archive`, aucune suppression d'objet, aucune suppression de relation. Un nœud retiré ne supprime pas la relation qu'il représentait. |
| Réparation | L'inspecteur de l'objet propose « Archiver l'objet » — action explicite et confirmée, hors du geste de suppression de forme. |
| Test | `delete_representation_keeps_object`, `delete_representation_keeps_relationship`, `delete_node_does_not_archive_element`. |

## 7. `REV-409` — Conflit de révision

| | |
|---|---|
| Déclencheur | `expected_revision` ne correspond pas à la révision courante de la vue (autre onglet, autre poste, rejeu tardif). |
| Message | « La vue a été modifiée ailleurs. Vos croquis sont conservés. » |
| Effet | Aucune écriture. La vue locale et les croquis restent en mémoire et dans le layout local. |
| Options | **Recharger** (perd les modifications locales après confirmation), **Comparer** (diff champ par champ), **Forcer** (réémet sans révision attendue). Jamais de fusion automatique. |
| Test | `stale_revision_returns_conflict_without_data_loss`, `conflict_offers_reload_compare_force`, `conflict_never_auto_merges`. |

Le dialogue réutilise le composant multi-onglets existant (lot 2, `ExternalWrite`) ; aucun
second mécanisme de conflit n'est créé.

## 8. Autres codes

| Code | Sévérité | Déclencheur | Test |
|---|---|---|---|
| `CMD-001` | erreur | `command_id` réutilisé avec un corps différent | `same_command_id_with_other_body_is_refused` |
| `CMD-002` | erreur | Commande de création sans identité d'objet (`Formalize`/`Connect`), géométrie non finie, ou `expected_revision` négatif | `creation_without_object_id_is_refused` |
| `DGM-000` | erreur | Vue inexistante ou archivée | `command_on_unknown_view_is_refused` |
| `DGM-001` | erreur | Nœud référençant un objet inexistant (existant) | `dgm_001_still_applies` |
| `DGM-002` | erreur | Identifiant de forme dupliqué (existant) | `duplicate_node_id_is_refused` |
| `DGM-005` | erreur | Géométrie non finie ou dimensions ≤ 0 (existant) | `invalid_geometry_is_refused` |
| `DGM-006` | erreur | Plus de 1 000 nœuds (existant) | `max_nodes_is_enforced` |
| `DGM-007` | erreur | Croquis dupliqué ou géométrie invalide | `duplicate_draft_id_is_refused` |
| `DGM-008` | erreur | Zone `DOMAIN` sans domaine ou domaine inconnu | `zone_domain_must_exist` |
| `DGM-008-W` | avertissement | Zone `DOMAIN` sur domaine archivé | `zone_on_archived_domain_warns` |
| `DGM-009` | erreur | `zone_id` / `layer_id` inconnu | `unknown_zone_reference_is_refused` |
| `DGM-010` | avertissement | Bandes chevauchantes | `overlapping_layers_warn` |
| `DGM-011` | avertissement | Port hors de sa frontière | `port_outside_boundary_warns_without_blocking` |
| `DGM-012` | avertissement | Curseur temporel illisible | `invalid_temporal_cursor_warns` |
| `DGM-013` | erreur | `schema > 2` | `unknown_future_schema_is_refused` |
| `DGM-014` | erreur | Vue créée sans intention | `view_without_intent_is_refused` |
| `DGM-015` | avertissement | Croquis lu avec un statut non `SKETCH` | `sketch_status_is_never_persisted_as_formalized` |
| `LINK-001` | erreur | Lien `arch://` vers un identifiant inexistant | `arch_uri_to_unknown_id_is_reported` |
| `LINK-002` | avertissement | Paramètre de lien invalide (zoom, encodage) | `invalid_share_parameter_is_ignored` |
| `LINK-003` | avertissement | `focus` contient un nœud inconnu | `unknown_focus_node_is_ignored_not_fatal` |
| `TPL-001` | erreur | Modèle de vue invalide (champ obligatoire) | `template_missing_field_is_refused` |
| `TPL-002` | erreur | `schema` de modèle inconnu | `template_unknown_schema_is_refused` |
| `TPL-003` | erreur | Classe de modèle absente du profil | `template_unknown_class_is_refused` |
| `TPL-004` | avertissement | Instanciation homonyme | `template_duplicate_instantiation_warns` |
| `PALETTE-EMPTY` | avertissement | Configuration d'intention vide après intersection | `empty_palette_is_reported_to_control` |
| `MM-ELEM-001` | erreur | Métaclasse absente du profil (existant) | `unknown_metaclass_is_refused` |
| `MM-ELEM-002` | erreur | Métaclasse abstraite instanciée (existant) | `abstract_class_is_refused` |
| `MM-PROP-001` | erreur | Propriété inconnue du profil (existant) | `unknown_property_is_refused` |
| `MM-REL-002` | erreur | Relation d'un objet vers lui-même (existant) | `self_relationship_is_refused` |
| `MM-REL-003` | erreur | Type de relation absent de la notation (existant) | `unknown_relation_type_is_refused` |
| `MM-PROFILE-001` | erreur | Profil ou version de métamodèle inconnu (existant) | `unknown_profile_is_refused` |
| `OBJ-002` | erreur | Nom obligatoire vide (existant) | `empty_name_is_refused` |
| `IFACE-002..005` | erreur | Invariants d'interface et de version (existants) | conservés |
| `DOM-001..003` | erreur | Domaine : nom, archivage, dépendances (existants) | conservés |
| `STORE-001..004` | erreur | Persistance IndexedDB (existants) | conservés |
| `BR-001`, `BR-002` | erreur | Règles métier transverses (existantes) | conservés |

## 9. Priorité d'affichage

Quand plusieurs codes surviennent dans un même geste, l'UI affiche **le premier dans
l'ordre de validation** (C-01 §4), et regroupe les autres dans « Détails » :

```text
CMD-001/CMD-002 > REV-409 > DGM-000 > DGM-002/005/007 > DSG-002 > DSG-001 > DSG-004
               > MM-* > DGM-008/009 > avertissements
```

`DSG-003` ne s'applique qu'à la publication et à l'export, jamais à une commande de pose.

## 10. Tests transverses

| Test | Prouve |
|---|---|
| `every_error_code_has_i18n_key` | VD-25 |
| `no_error_message_is_hardcoded_in_ui` | VD-25 |
| `rejection_leaves_layout_unchanged` | C-01 §1 |
| `rejection_writes_no_object` | C-01 §1 |
| `codes_are_displayed_next_to_the_gesture` | NFR-023 |
| `conflict_never_auto_merges` | VD-04 |
