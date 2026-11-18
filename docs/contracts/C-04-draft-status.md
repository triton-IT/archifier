# C-04 — `DraftStatus` : croquis, brouillon formalisé, objet valide

| | |
|---|---|
| **Réf.** | C-04 |
| **Version** | 1.0 — figé G0, 18/11/2026 |
| **Propriétaire** | Dev « Inspecteur & formalisation » — `crates/ui/src/design/formalize.rs` |
| **Sources** | FR-047, NFR-029, DSG-001/003, §14.4.1, §14.4.3 ; VD-07, VD-11 ; ADR-022 §3 |
| **Dépend de** | C-01 (`Formalize`, `DraftShape`), C-02 (`drafts[]`), C-08 |

## 1. Les trois états

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DraftStatus { Sketch, Formalized, Valid }
```

| État | Ce qui existe | Ce qui est visible | Publiable | Exportable |
|---|---|---|---|---|
| `SKETCH` | Une entrée `layout.drafts[]`. **Aucun objet** du référentiel. | Forme au trait pointillé, badge « Croquis », libellé en italique. | Non (`DSG-003`) | Non |
| `FORMALIZED` | Un `ModelElement` + un nœud `nodes[]`. Propriétés requises **incomplètes**. | Forme pleine, badge « Brouillon », propriété manquante signalée. | Non (`DSG-001`) | Oui, avec mention « brouillon » |
| `VALID` | Le même objet, propriétés requises complètes. | Forme pleine, badge « Validé », aucune alerte. | Oui | Oui |

NFR-029 impose que la distinction soit **visible en permanence** : chaque forme porte son
badge et son motif, sans survol et sans ouverture de fiche.

## 2. Où vit l'état

Le croquis est **local** : il n'existe que dans `layout.drafts[]`. À la formalisation, il
n'est pas « copié » : il est **remplacé** par un nœud qui **réutilise son identifiant**.

```text
avant : drafts[ d1 { status: SKETCH } ]
après : nodes[  d1 { kind: ELEMENT, object_id: 01J… } ]
```

Conséquences, toutes testées :

1. Aucune duplication de géométrie.
2. Un identifiant de forme reste stable toute la vie de la vue.
3. `FORMALIZED` et `VALID` sont **calculés** depuis l'objet, jamais stockés dans le
   layout : supprimer une propriété requise fait retomber un `VALID` en `FORMALIZED`
   immédiatement, sans commande de statut.
4. Un layout relu ne contient jamais `drafts[].status != SKETCH`. Le type accepte les
   trois valeurs en lecture (compatibilité ascendante) ; un croquis non-`SKETCH` lu
   produit l'avertissement `DGM-015` et est traité comme `SKETCH`.

## 3. Définition de `VALID`

Un objet est `VALID` si et seulement si :

1. il existe, est actif, et son domaine responsable existe et est actif ;
2. sa classe appartient au profil de sa notation (`MM-ELEM-001` sinon) ;
3. toutes les propriétés requises par le profil sont présentes et non vides
   (`required_properties_missing` → liste vide) ;
4. pour une interface : elle possède au moins une version, et la version représentée
   appartient à l'interface (`DSG-004`).

Aucune autre condition (documentation, propriétaire, couleur) n'entre dans `VALID`.

## 4. Transitions autorisées

| De | Vers | Déclencheur | Garde |
|---|---|---|---|
| — | `SKETCH` | `PlaceDraft` | géométrie valide (`DGM-005`) |
| `SKETCH` | `FORMALIZED` | `Formalize` | métamodèle + domaine (`DSG-001`), classe du profil (`MM-ELEM-001`) |
| `SKETCH` | `VALID` | `Formalize` | idem + propriétés requises complètes ; **raccourci autorisé** si le geste les fournit |
| `FORMALIZED` | `VALID` | édition de propriétés | `required_properties_missing` devient vide |
| `VALID` | `FORMALIZED` | suppression d'une propriété requise | recalcul immédiat |
| `FORMALIZED` | `SKETCH` | **annulation** de la formalisation | uniquement via l'historique ; restaure le croquis, retire le nœud, **conserve l'objet créé** |
| tout | — | `DeleteRepresentation` | retire la représentation ; ne change aucun statut d'objet (`DSG-005`) |

**Interdits :**

- `SKETCH → VALID` sans `Formalize` (aucun objet implicite).
- `VALID → SKETCH` par édition : il faut annuler la formalisation.
- Publication d'une vue contenant un `SKETCH`.
- Export d'un croquis comme objet (le SVG de la vue exclut les croquis et l'annonce).
- Suppression d'objet par changement de statut.

## 5. Publication

`PublishView` exige :

1. aucun `SKETCH` dans la vue → sinon `DSG-003`, avec la liste des identifiants ;
2. tous les nœuds `VALID` → sinon `DSG-001`, avec pour chacun le champ manquant ;
3. les zones `DOMAIN` référencent des domaines actifs → sinon `DGM-008`.

Un refus de publication ne modifie **rien** : la vue reste locale, les croquis restent
là, l'indicateur d'enregistrement reste `Enregistré · révision N`. L'utilisateur peut
« Formaliser les N croquis restants » depuis le message — c'est une action de la vue,
pas un modal.

## 6. Formalisation différée et liens

- Le croquis peut rester indéfiniment à l'état `SKETCH` ; il est sauvegardé et restauré
  (VD-04, §14.4.3).
- Un croquis ne peut pas être l'extrémité d'une relation : aucune classe n'existe, donc
  aucune règle de métamodèle n'est évaluable. `Connect` renvoie `DSG-002` avec
  `detail = "DRAFT_NOT_FORMALIZED"`.
- L'UI ne montre pas un refus sec : elle met en avant l'action « Formaliser puis relier ».
  C'est un **geste de plus**, pas un modal, et il reste dans le budget de trois gestes
  puisque la formalisation fait partie du parcours normal (VD-07).

## 7. Inspecteur : ce qu'il montre par état

| Section | `SKETCH` | `FORMALIZED` | `VALID` |
|---|---|---|---|
| Nom | éditable sur place (`F2`) | éditable | éditable |
| Type | proposition (`suggested_metaclass`), choix dans le profil | affiché, modifiable | affiché |
| Domaine | proposition (`suggested_domain_id`), choix | affiché, modifiable | affiché |
| Propriétés | aucune (pas d'objet) | générées depuis le profil, manquantes marquées | générées, complètes |
| Documentation | désactivée avec explication | éditable | éditable |
| Liens | « Aucun lien tant que le croquis n'est pas formalisé » | liens entrants/sortants | idem |
| Historique | « Croquis local, non versionné » | révisions de l'objet | révisions |

L'inspecteur est un panneau latéral **non bloquant** : il ne recouvre jamais le canevas,
ne capture jamais le focus de force, et `Échap` le referme en conservant la sélection
(VD-11, NFR-023).

## 8. Tests obligatoires (noms figés)

| Test | Prouve |
|---|---|
| `place_draft_creates_sketch_without_object` | VD-07 |
| `sketch_is_persisted_and_restored` | VD-04 |
| `sketch_is_not_searchable` | §14.4.3 |
| `sketch_cannot_be_published` | **DSG-003** |
| `sketch_is_excluded_from_svg_export` | DSG-003 |
| `formalize_creates_element_and_replaces_draft` | FR-047 |
| `formalize_incomplete_properties_is_formalized_not_valid` | NFR-029 |
| `valid_requires_profile_required_properties` | DSG-001 |
| `removing_required_property_downgrades_to_formalized` | C-04 §2 |
| `undo_formalize_restores_sketch_and_keeps_object` | DSG-005 |
| `port_formalization_creates_interface_and_version` | **DSG-004** |
| `port_without_interface_is_refused` | DSG-004 |
| `sketch_status_is_never_persisted_as_formalized` | DGM-015 |
| `inspector_does_not_cover_canvas` | VD-11, NFR-023 |
