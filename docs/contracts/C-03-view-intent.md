# C-03 — `ViewIntent` : sept intentions et leur configuration

| | |
|---|---|
| **Réf.** | C-03 |
| **Version** | 1.0 — figé G0, 18/11/2026 |
| **Propriétaires** | UX (contenu), orchestrateur (gel) |
| **Sources** | FR-046, FR-048, §14.4.1, §15.4, §15.6 ; VD-06, VD-14 ; ADR-022 §4 |
| **Dépend de** | C-02, C-05, C-07 |

## 1. Catalogue fermé

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViewIntent {
    Context,
    ApplicationLandscape,
    Process,
    Deployment,
    Capability,
    Migration,
    InterfaceReview,
}
```

L'intention **pilote la palette, les bandes, l'état vide et les invitations**. Elle ne
valide **jamais** le métamodèle : une classe autorisée par le profil reste autorisée,
même hors palette. Inversement, une classe absente du profil est refusée même si elle
figure dans la palette (`MM-ELEM-001`).

Une vue sans intention ne peut pas être créée (`DGM-014`). Une vue migrée reçoit une
intention par défaut (C-02 §5) et une invitation à la préciser.

## 2. Configuration opposable

Chaque intention porte un `ViewIntentConfig` :

```rust
pub struct ViewIntentConfig {
    pub intent: ViewIntent,
    pub label_key: &'static str,
    pub tagline_key: &'static str,
    pub notation: DiagramNotation,
    pub layers: Vec<LayerSpec>,
    pub palette: Vec<PaletteSpec>,      // (couche, [metaclass])
    pub empty_state_key: &'static str,
    pub empty_actions: Vec<EmptyAction>,// MODELE | EXAMPLE | IMPORT | DEMO
    pub template_id: &'static str,      // assets/view-templates/<id>.json
    pub boundary_default: bool,         // frontières visibles d'emblée
    pub temporal_default: bool,         // curseur temporel ouvert à la création
    pub scope_default: ScopeKind,
}

pub struct LayerSpec { pub id: &'static str, pub label_key: &'static str,
                       pub h: f64, pub order: u32 }

#[derive(Debug, Clone, Copy)]
pub struct PaletteSpec {
    pub layer_id: &'static str,
    /// Classes explicitement masquées pour cette intention. Vide à G0.
    pub hide: &'static [&'static str],
}
```

Hauteurs par défaut (px monde, grille 8) :

| Couche | Hauteur |
|---|---|
| `business` | 240 |
| `application` | 280 |
| `technology` | 240 |
| `physical` | 200 |
| `motivation` | 180 |
| `implementation` | 200 |
| `process` (BPMN) | 360 |
| `flow` (BPMN) | 240 |
| `data` (BPMN) | 160 |
| `annotation` (BPMN) | 120 |
| `structure` (UML) | 260 |
| `deployment` (UML) | 260 |
| `behavior` (UML) | 200 |

L'espacement entre bandes est de `--layer-band-gap` (C-07). Les bandes sont ordonnées
par `order` croissant et se déplacent ensemble lors de la réorganisation.

**Palette = couche entière, filtrée par le profil.** Une intention cite des **couches**,
pas une liste blanche de classes : toutes les classes concrètes de la couche dans le
profil chargé sont proposées, moins `hide` (vide à G0) et moins les classes abstraites.
Les tableaux des §3 à §9 listent les classes dont la **présence à G0** est testée ; une
classe non listée reste disponible. C'est ce qui rend VD-06 vrai : ajouter une classe dans
un profil TOML la fait apparaître dans la palette de l'intention qui cite sa couche,
**sans modification de code**.

---

## 3. `CONTEXT` — Contexte

| | |
|---|---|
| Libellé | **Contexte** |
| Accroche | « Qui est concerné, ce qui change, ce que le système doit accomplir. » |
| Notation | `ARCHIMATE` |
| Modèle | `assets/view-templates/context.json` |
| Portées | `motivation`, `business`, `application` |
| Frontières | non |
| Temporel | non |
| Exemple | « Contexte de la refonte Paiements » |

**Bandes** : `motivation` (180), `business` (240), `application` (260), `technology` (200).

**Palette** (dérivée des profils chargés, aucune classe inventée) :

| Couche | Classes |
|---|---|
| Motivation | `Stakeholder`, `Driver`, `Assessment`, `Goal` |
| Métier | `BusinessActor`, `BusinessRole`, `BusinessService`, `Product`, `BusinessObject` |
| Applicatif | `ApplicationComponent`, `ApplicationService` |
| Technologique | `Node`, `TechnologyService` |

**État vide** : « Posez d'abord les acteurs et les objectifs, puis les composants qui les
servent. Un contexte se lit de haut en bas : pourquoi, pour qui, avec quoi. »
Actions : **Exemple**, **Partir d'un modèle**, **Importer une vue**.

---

## 4. `APPLICATION_LANDSCAPE` — Applicatif d'un domaine

| | |
|---|---|
| Libellé | **Applicatif d'un domaine** |
| Accroche | « Les composants du domaine, leurs services, leurs dépendances. » |
| Notation | `ARCHIMATE` |
| Modèle | `assets/view-templates/application-landscape.json` |
| Frontières | oui (une zone `DOMAIN` par domaine du périmètre) |
| Temporel | non (activable) |
| Exemple | « Paysage applicatif Paiements » |

**Bandes** : `business` (200), `application` (340), `technology` (200).

**Palette** :

| Couche | Classes |
|---|---|
| Métier | `BusinessActor`, `BusinessRole`, `BusinessService`, `BusinessProcess`, `BusinessObject` |
| Applicatif | `ApplicationComponent`, `ApplicationCollaboration`, `ApplicationService`, `ApplicationInterface`, `ApplicationProcess`, `ApplicationDataObject` |
| Technologique | `Node`, `SystemSoftware`, `TechnologyService` |

**État vide** : « Déposez les composants du domaine dans la bande applicative, nommez-les,
puis reliez-les. Les services exposés deviendront des ports sur la frontière du domaine. »
Actions : **Exemple**, **Ouvrir une vue existante**, **Partir d'un modèle**.

C'est l'intention de référence de la chaîne de conception (VD-17, VD-22).

---

## 5. `PROCESS` — Processus

| | |
|---|---|
| Libellé | **Processus** |
| Accroche | « Le déroulé de bout en bout, ses tâches, ses événements, ses données. » |
| Notation | `BPMN` |
| Modèle | `assets/view-templates/process.json` |
| Frontières | non (couloirs = groupes) |
| Temporel | non |
| Exemple | « Encaissement d'une commande » |

**Bandes** : `process` (360), `flow` (240), `data` (160), `annotation` (120).

**Palette** :

| Couche | Classes |
|---|---|
| Processus | `Process`, `Participant`, `Lane`, `SubProcess`, `CallActivity`, `Task`, `UserTask`, `ServiceTask`, `ManualTask`, `BusinessRuleTask` |
| Flux | `StartEvent`, `EndEvent`, `IntermediateCatchEvent`, `IntermediateThrowEvent`, `ExclusiveGateway`, `ParallelGateway`, `InclusiveGateway` |
| Données | `DataObject`, `DataStore` |
| Annotation | `TextAnnotation` |

**État vide** : « Commencez par l'événement de début, ajoutez les tâches dans l'ordre, puis
marquez celles qui sont automatisées. Un couloir par acteur, pas un couloir par outil. »
Actions : **Exemple**, **Partir d'un modèle**.

---

## 6. `DEPLOYMENT` — Déploiement

| | |
|---|---|
| Libellé | **Déploiement** |
| Accroche | « Où tournent les composants, sur quels nœuds, avec quels artefacts. » |
| Notation | `UML` |
| Modèle | `assets/view-templates/deployment.json` |
| Frontières | oui (environnements = zones `GROUP`) |
| Temporel | non |
| Exemple | « Production Paiements » |

**Bandes** : `structure` (200), `deployment` (340).

**Palette** :

| Couche | Classes |
|---|---|
| Structure | `Component`, `Artifact`, `Package` |
| Déploiement | `Node`, `Device`, `ExecutionEnvironment` |

**État vide** : « Placez les nœuds d'exécution, puis glissez les composants déployés
à l'intérieur. Une zone par environnement, jamais une couleur par environnement seule. »
Actions : **Exemple**, **Importer un snapshot**.

---

## 7. `CAPABILITY` — Capacité

| | |
|---|---|
| Libellé | **Capacité** |
| Accroche | « Ce que l'entreprise sait faire, pour qui, et ce qui le prouve. » |
| Notation | `ARCHIMATE` |
| Modèle | `assets/view-templates/capability.json` |
| Frontières | non |
| Temporel | non |
| Exemple | « Capacités de la chaîne Paiements » |

**Bandes** : `motivation` (180), `business` (320), `implementation` (220).

**Palette** :

| Couche | Classes |
|---|---|
| Motivation | `Goal`, `Driver`, `Assessment` |
| Métier | `Product`, `BusinessService`, `BusinessFunction`, `BusinessProcess`, `BusinessObject` |
| Mise en œuvre | `WorkPackage`, `Deliverable` |

**État vide** : « Cartographiez ce que l'entreprise sait faire comme des services rendus,
puis rattachez les objectifs et les lots de travail. »

> **Écart documenté (à ne pas corriger dans l'UI).** Le profil `archimate:3.2` livré
> expose 42 classes et **ne contient pas** `Capability`, `ValueStream`, `CourseOfAction`
> ni `Resource` (couche Stratégie absente). L'intention `CAPABILITY` exprime donc la
> capacité avec `BusinessFunction` / `BusinessService` / `Product`, sans inventer de
> classe « 4 ». Ajouter `Capability` exige une évolution de profil versionnée (ADR-021)
> et un test de couverture, pas une exception codée dans l'Atelier (VD-06).

---

## 8. `MIGRATION` — Migration

| | |
|---|---|
| Libellé | **Migration** |
| Accroche | « L'état aujourd'hui, l'état cible, les lots qui font la bascule. » |
| Notation | `ARCHIMATE` |
| Modèle | `assets/view-templates/migration.json` |
| Frontières | oui |
| Temporel | **oui**, ouverte à la date du jour |
| Exemple | « Migration v1 → v2 de Payment API » |

**Bandes** : `business` (180), `application` (260), `technology` (200),
`implementation` (220).

**Palette** :

| Couche | Classes |
|---|---|
| Métier | `BusinessActor`, `BusinessProcess`, `BusinessService` |
| Applicatif | `ApplicationComponent`, `ApplicationService`, `ApplicationInterface` |
| Technologique | `Node`, `SystemSoftware` |
| Mise en œuvre | `WorkPackage`, `Deliverable` |

**État vide** : « Posez l'existant, puis l'état cible. Le curseur temporel montre ce qui
est planifié, actif ou expiré à la date choisie, sans déplacer une seule forme. »
Actions : **Exemple**, **Ouvrir la vue temporelle**.

C'est l'intention de la démonstration de temporalité (G2, VD-12).

---

## 9. `INTERFACE_REVIEW` — Revue d'interface

| | |
|---|---|
| Libellé | **Revue d'interface** |
| Accroche | « Une frontière, des ports, des versions, des consommateurs. » |
| Notation | `ARCHIMATE` |
| Modèle | `assets/view-templates/interface-review.json` |
| Frontières | **oui**, obligatoires |
| Temporel | **oui** |
| Exemple | « Payment API : v1 et v2 en transition » |

**Bandes** : `business` (220), `application` (320).

**Palette** :

| Couche | Classes |
|---|---|
| Métier | `BusinessActor`, `BusinessRole`, `BusinessInterface`, `BusinessService` |
| Applicatif | `ApplicationInterface`, `ApplicationComponent`, `ApplicationService` |

**État vide** : « Dessinez la frontière du domaine fournisseur, posez un port par
interface exposée, puis les consommateurs de l'autre côté. Chaque port porte une version
et un état temporel lisibles sans ouvrir de fiche. »
Actions : **Exemple**, **Ouvrir une revue existante**.

---

## 10. Dérivation de la palette — règle VD-06

La palette affichée est calculée à chaque rendu :

```text
palette(intent) = ⋃ couches(citées par l'intention)
                    ∩ classes concrètes(profils chargés)
                    − hide(intention)
```

- Une classe **ajoutée** dans une couche d'un profil TOML apparaît automatiquement dans
  la palette de toute intention qui cite cette couche. Aucune modification de code, aucun
  redéploiement de configuration.
- Une classe **retirée** du profil disparaît de la palette ; elle n'est pas remplacée par
  un bouton mort.
- `hide` est une liste de données, vide à G0. Elle sert à retirer une classe jugée hors
  sujet pour une intention précise, jamais à constituer une liste blanche.
- Une classe abstraite (`abstract = true` dans le profil) n'est jamais proposée.
- Une palette vide n'existe pas : si l'intersection est vide, la palette affiche la
  couche entière du profil et un avertissement `PALETTE-EMPTY` (défaut de configuration,
  signalé au Contrôle, jamais à l'utilisateur final).

## 11. Tests obligatoires (noms figés)

| Test | Prouve |
|---|---|
| `seven_intents_are_exposed_with_template` | VD-14 |
| `intent_palette_is_derived_from_profile_layers` | VD-06 |
| `profile_class_addition_appears_without_code_change` | VD-06 |
| `removed_profile_class_disappears_from_palette` | VD-06 |
| `intent_never_bypasses_metamodel_validation` | C-03 §1 |
| `each_intent_has_empty_state_with_action` | NFR-027, VD-14 |
| `each_intent_template_opens_with_its_layers` | VD-14 |
| `view_without_intent_is_refused` | DGM-014 |
| `capability_intent_uses_existing_classes_only` | ADR-021, C-03 §7 |
