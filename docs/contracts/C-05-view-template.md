# C-05 — `ViewTemplate` : schéma JSON d'un modèle de vue

| | |
|---|---|
| **Réf.** | C-05 |
| **Version** | 1.0 — figé G0, 18/11/2026 |
| **Propriétaire** | UX — `assets/view-templates/*.json` |
| **Sources** | FR-046, FR-054, NFR-027 ; VD-14, VD-17 ; §15.4 « Nouvelle vue » |
| **Dépend de** | C-02 (layout), C-03 (intentions), C-04 (croquis) |

## 1. Rôle

Un modèle de vue est un **échafaudage** : bandes, zones, cadrage, et au plus quelques
croquis d'exemple. Il ne contient **jamais** d'objet du référentiel, jamais d'`object_id`,
jamais de domaine figé. Il matérialise l'état vide utile exigé par NFR-027.

Un fichier par intention :

```text
assets/view-templates/context.json
assets/view-templates/application-landscape.json
assets/view-templates/process.json
assets/view-templates/deployment.json
assets/view-templates/capability.json
assets/view-templates/migration.json
assets/view-templates/interface-review.json
```

## 2. Schéma (version 1)

```json
{
  "schema": 1,
  "template_version": "1.0.0",
  "id": "application-landscape",
  "intent": "APPLICATION_LANDSCAPE",
  "title_key": "template.application_landscape.title",
  "summary_key": "template.application_landscape.summary",
  "notation": "ARCHIMATE",
  "scope": { "kind": "DOMAIN" },
  "grid": { "size": 8, "snap": true },
  "layers": [
    { "id": "business", "label_key": "layer.business", "y": 0, "h": 200, "tone": "business" },
    { "id": "application", "label_key": "layer.application", "y": 216, "h": 340, "tone": "application" },
    { "id": "technology", "label_key": "layer.technology", "y": 572, "h": 200, "tone": "technology" }
  ],
  "zones": [
    { "id": "z1", "kind": "DOMAIN", "label_key": "template.zone.supplier_domain",
      "x": 48, "y": 232, "w": 520, "h": 308, "layer_id": "application",
      "boundary": true, "tone": "domain-1" }
  ],
  "framing": { "zoom": 0.9, "x": 0, "y": 0, "focus": [], "dim_others": false },
  "drafts": [
    { "id": "d1", "label_key": "template.application_landscape.draft.service",
      "shape": "RECT", "x": 96, "y": 288, "w": 176, "h": 72,
      "suggested_metaclass": "ApplicationService", "status": "SKETCH",
      "layer_id": "application", "zone_id": "z1" }
  ],
  "example": {
    "steps": ["place", "rename", "connect", "formalize"]
  }
}
```

## 3. Champs — règle opposable

| Champ | Obligatoire | Règle |
|---|---|---|
| `schema` | oui | Entier, **1** pour cette version. Toute autre valeur : `TPL-002`. |
| `template_version` | oui | SemVer. Une évolution de contenu incrémente la mineure ; un changement de schéma incrémente la majeure et `schema`. |
| `id` | oui | Slug stable, unique, identique au nom de fichier. |
| `intent` | oui | Doit égaler l'intention du catalogue (C-03). |
| `title_key`, `summary_key` | oui | Clés i18n, jamais de phrase en dur (VD-25). |
| `notation` | oui | Notation de la vue créée. |
| `scope.kind` | oui | `DOMAIN` (défaut), `GLOBAL`, `MULTI_DOMAIN`. Le domaine concret est choisi à l'instanciation. |
| `grid` | non | Défaut `{ "size": 8, "snap": true }`. |
| `layers` | oui | Au moins une bande ; `id` uniques ; `y` croissants ; `h > 0`. |
| `zones` | non | `DOMAIN` sans `domain_id` (injecté à l'instanciation), `GROUP` sans domaine. |
| `framing` | non | Cadrage initial. Ne contient jamais de `focus` (aucun nœud n'existe). |
| `drafts` | non | **0 à 4** croquis d'exemple, toujours `SKETCH`. |
| `example.steps` | non | Sous-ensemble ordonné de `place`, `rename`, `connect`, `formalize`, `share`. |

**Interdits structurels** (validés au chargement) :

1. tout champ `object_id`, `relationship_id`, `interface_id`, `version_id` ;
2. un domaine nommé en dur (`domain_id`, `domain_name`) ;
3. une classe absente du profil ciblé (`TPL-003`) ;
4. une couleur littérale (`#rrggbb`, `rgb()`), un `style_override` non vide ;
5. une taille de police ou une ombre : tout passe par les tokens (C-07) ;
6. un croquis de statut `FORMALIZED` ou `VALID`.

## 4. Instanciation

`apply_template(template, domain_id, title, now) -> (Diagram, DiagramLayout)` — pure côté
core, écriture côté dépôt par l'UI :

1. crée la vue (`create_diagram`) avec la notation et le domaine choisis ;
2. copie `layers`, `grid`, `zones`, `framing` ;
3. remplace chaque zone `DOMAIN` sans domaine par `domain_id` (une seule zone) ;
4. copie les `drafts` en conservant leurs identifiants locaux ;
5. résout chaque `label_key` par l'i18n au moment du rendu, **pas** à l'instanciation :
   changer la langue ne réécrit pas le layout ;
6. ne crée **aucun** `ModelElement`, `Relationship` ou `Interface`.

Un modèle ne peut pas être instancié deux fois avec le même `domain_id` et le même titre
sans confirmation : `TPL-004` (avertissement, pas un blocage).

## 5. Codes

| Code | Sévérité | Règle |
|---|---|---|
| `TPL-001` | erreur | Champ obligatoire absent ou type invalide. |
| `TPL-002` | erreur | `schema` inconnu. |
| `TPL-003` | erreur | `suggested_metaclass` absente du profil ciblé. |
| `TPL-004` | avertissement | Instanciation homonyme dans le même domaine. |

Le chargement d'un modèle invalide **ne bloque pas l'application** : l'intention reste
proposée avec un état vide générique et l'anomalie est reportée au Contrôle (VD-16).

## 6. Contenu attendu des sept modèles (contrat de recette)

| Modèle | Bandes | Zones | Croquis d'exemple | Cadrage |
|---|---|---|---|---|
| `context` | motivation, business, application, technology | 0 | 1 acteur, 1 objectif | 0.9 |
| `application-landscape` | business, application, technology | 1 DOMAIN fournisseur | 1 service, 1 composant | 0.9 |
| `process` | process, flow, data, annotation | 1 GROUP « Couloir client » | 1 tâche, 1 événement de début | 1.0 |
| `deployment` | structure, deployment | 2 GROUP « Production », « Recette » | 1 nœud, 1 artefact | 0.9 |
| `capability` | motivation, business, implementation | 0 | 1 capacité métier, 1 objectif | 0.9 |
| `migration` | business, application, technology, implementation | 1 DOMAIN | 1 composant v1, 1 composant v2, 1 lot | 0.8 |
| `interface-review` | business, application | 1 DOMAIN fournisseur, 1 DOMAIN consommateur | 1 port, 2 consommateurs | 0.9 |

Les libellés sont des clés i18n ; les valeurs françaises sont fournies par l'analyste
dans `i18n/fr.json`.

## 7. Tests obligatoires (noms figés)

| Test | Prouve |
|---|---|
| `seven_templates_load_and_match_intents` | VD-14 |
| `template_contains_no_object_reference` | C-05 §3 |
| `template_unknown_schema_is_refused` | TPL-002 |
| `template_unknown_class_is_refused` | TPL-003 |
| `template_instantiation_creates_no_object` | C-05 §4 |
| `template_instantiation_injects_domain_into_zone` | VD-09 |
| `template_drafts_are_sketches` | C-04 |
| `template_labels_are_i18n_keys` | VD-25 |
