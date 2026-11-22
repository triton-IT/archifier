# C-06 — Table des routes, alias et liens partagés

| | |
|---|---|
| **Réf.** | C-06 |
| **Version** | 1.0 — figé G0, 18/11/2026 |
| **Propriétaire** | Dev « Navigation & espaces » — `crates/ui/src/router.rs` |
| **Sources** | §15.2, §15.4 ; FR-040, FR-048, FR-051 ; VD-01, VD-13, VD-18, VD-19 ; ADR-012 |
| **Dépend de** | C-03 (`ViewIntent`), C-02 (`ViewFraming`) |

> **Avenant ADR-023 (18/11/2026), ADR-024 et ADR-025 (21/11/2026).** Le chemin `/` décrit ci-dessous comme « Atelier » devient le **portefeuille de produits** (`ProductQuery::home()`, ADR-025). La carte est servie par `/map` avec `MapQuery::map_default()` (scope et niveau Entreprise, lentille primaire `PRODUCT`). L'Atelier reste accessible par `/atelier`, `/design/:id` et `/view/diagrams/:id`. Les routes de la carte (`/map`, `/products/:id`, `/teams/:id`, `/value-streams/:id`, `/capabilities/:id`, `/applications/:id`, `/infrastructure/:id`, `/business-domains/:id`, `/portfolios/:id`) et leurs paramètres sont fixés par **C-10**. Les alias de domaine restent résolvables. Aucune route existante n'est supprimée.

## 1. Décision de transport : routage par fragment (inchangé)

Le produit reste en **routage par fragment** (`#/…`), conformément à ADR-012 : aucun
serveur n'a besoin de réécrire les URL, et un lien `arch://` se résout en ancre. Les
chemins de §15.2 sont donc des **chemins logiques** ; la forme réelle est `#/<chemin>`.

> **Écart consigné.** §15.1 et §15.2 décrivent un routage `history` avec repli SPA.
> ADR-012, non remplacé, impose le fragment. Le MVP3 conserve ADR-012 : changer de
> transport serait une migration d'URL cassant les liens partagés, sans bénéfice produit.
> Voir `technical-specifications-lot3.md` §2 (registre des écarts).

Conséquences :

- Toute route ci-dessous est atteignable par `#` + chemin.
- Un serveur statique suffit (`node scripts/serve.mjs 4173`).
- Les alias restent valides **sans redirection d'historique** : on peut coller un ancien
  lien et atterrir sur le bon écran ; les liens générés utilisent la forme canonique.

## 2. Table exacte

| Chemin logique | Fragment | Écran | Posture | Source de vérité |
|---|---|---|---|---|
| `/` | `#/` | **Portefeuille de produits**, filtrable (`ProductQuery::home()`) | lecture | C-10, ADR-025 |
| `/products` | `#/products` | Alias du portefeuille de produits | lecture | C-10, ADR-025 |
| `/map` | `#/map` | **Carte de l'entreprise** (`MapQuery::map_default()`) | lecture | C-10, ADR-023/024 |
| `/atelier` | `#/atelier` | Atelier : dernière vue éditée, sinon « Nouvelle vue » | conception | `localStorage: archifier:last_view` |
| `/design/new?intent=` | `#/design/new?intent=APPLICATION_LANDSCAPE` | Création depuis une intention (ou les 7 si absente) | conception | C-03, C-05 |
| `/design/:id` | `#/design/:id` | Atelier, édition | **conception** | C-01, C-02 |
| `/view/diagrams/:id` | `#/view/diagrams/:id` | Atelier, lecture partagée | **lecture** | C-01, C-02 |
| `/model/diagrams/:id` | `#/model/diagrams/:id` | **alias** de `/design/:id` | conception | §15.2 |
| `/library` | `#/library` | Bibliothèque, onglet Vues | — | §15.4 |
| `/library/views?owner=&intent=&notation=&state=` | `#/library/views?…` | Vues filtrées (couvertures) | — | VD-15 |
| `/library/domains/:id` | `#/library/domains/:id` | Synthèse de domaine | — | §15.4 |
| `/view/domains/:id` | `#/view/domains/:id` | **alias** de `/library/domains/:id` | — | §15.2 |
| `/search?q=` | `#/search?q=` | Recherche transversale | — | FR-016 |
| `/control` | `#/control` | Contrôle, onglet Anomalies | — | VD-16 |
| `/control/inventory` | `#/control/inventory` | Inventaire assumé | — | VD-16 |
| `/control/coverage` | `#/control/coverage` | Couverture des métamodèles | — | VD-16 |
| `/control/exports` | `#/control/exports` | Exports / imports | — | VD-16 |
| `/view/elements/:id` | `#/view/elements/:id` | Fiche d'élément | — | VD-19 |
| `/view/relationships/:id` | `#/view/relationships/:id` | Fiche de relation (type, extrémités) | — | VD-19, `arch://` |
| `/view/interfaces/:id` | `#/view/interfaces/:id` | Fiche d'interface | — | VD-19 |
| `/view/versions/:id` | `#/view/versions/:id` | Fiche d'interface, version ciblée | — | VD-19, `arch://` |
| `/view/stories/:id` | `#/view/stories/:id` | Lecteur de Story | — | VD-13 |
| `/view/reviews/:id` | `#/view/reviews/:id` | Revue | — | VD-13 |
| `/view/at/:timestamp` | `#/view/at/2027-08-01T00:00:00Z?view=<id>` | Lecture à la date T | lecture | VD-12 |
| `/graph/:id` | `#/graph/:id` | Graphe de dépendances (secondaire) | — | FR-028 |
| `/settings` | `#/settings` | Profil, fuseau, à propos | — | §15.2 |
| `/_design` | `#/_design` | Galerie du design system (interne) | — | VD-23 |
| `/_design/...` | `#/_design/<section>` | États, primitives, tokens | — | VD-23 |

Routes **supprimées** : aucune. `#/` devient le **portefeuille de produits** ; la carte se
déplace sur `#/map`. L'Atelier se déplace sur `#/atelier` et conserve `#/design/:id` /
`#/view/diagrams/:id`.

## 3. Type de route (contrat)

```rust
pub enum Route {
    Atelier,
    ViewNew { intent: Option<ViewIntent> },
    ViewEditor { id: ObjectId, link: Option<ViewLink> },
    ViewReader { id: ObjectId, link: Option<ViewLink> },
    Library { tab: LibraryTab, query: LibraryQuery },
    DomainSummary { id: ObjectId },
    Search { q: String },
    Control { tab: ControlTab },
    Element { id: ObjectId },
    Relationship { id: ObjectId },
    Interface { id: ObjectId, version: Option<ObjectId> },
    Story { id: ObjectId },
    Review { id: ObjectId },
    ViewAt { at: OffsetDateTime, view: Option<ObjectId> },
    Graph { id: ObjectId },
    DesignSystem,
    Settings,
    NotFound(String),
}

pub struct ViewLink {
    pub zoom: Option<f64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub focus: Vec<String>,     // NodeId
    pub dim: bool,
    pub caption: Option<String>,
    pub t: Option<OffsetDateTime>, // curseur temporel du lien
}
```

`LibraryTab` ∈ `Views | Domains | Objects | Interfaces | Stories | Reviews`.
`ControlTab` ∈ `Anomalies | Inventory | Coverage | Exports`.

## 4. Paramètres de lien partagé

Forme **canonique** générée par « Copier le lien » d'une vue en lecture :

```text
#/view/diagrams/01J…?zoom=0.9&x=120&y=40&focus=n1,n4&dim=1&t=2027-08-01T00:00:00Z&caption=Migration%20v1
```

| Paramètre | Type | Défaut | Règle |
|---|---|---|---|
| `zoom` | décimal `0.25..4` | absent = cadrage persisté | Hors bornes : ignoré, avertissement `LINK-002`. |
| `x`, `y` | décimal | `0` | Pan en coordonnées monde, avant `zoom`. |
| `focus` | `NodeId` séparés par `,` | vide | Un id inconnu est ignoré individuellement (`LINK-003`), les autres s'appliquent. |
| `dim` | `0`/`1` | `0` | Atténuation des non-focus. |
| `t` | RFC 3339 | absent | Curseur temporel du lien ; n'écrit **aucune** donnée. |
| `caption` | texte URL-encodé | vide | Légende de cadrage, affichée en lecture. |

Règles :

1. Un lien partagé ouvre **toujours** la posture lecture, même si le demandeur est
   l'auteur : c'est un contrat de lecture, pas d'édition.
2. Le cadrage du lien **prime** sur le cadrage persisté, sans le réécrire. Fermer le lien
   ne modifie pas la vue.
3. Le cadrage est restitué **à l'identique** (NFR-030) : zoom, pan, focus, atténuation,
   légende, curseur.
4. `focus` est canonicalisé (ordre lexicographique, doublons retirés) pour produire des
   liens reproductibles.
5. Aucun paramètre ne transporte de secret, de sélection d'édition ni d'historique.

## 5. Alias et liens `arch://`

| `arch://` | Route résolue | Note |
|---|---|---|
| `arch://domain/{id}` | `#/library/domains/{id}` | Changement de cible : ancienne `#/view/domains/{id}` reste valide. |
| `arch://element/{id}` | `#/view/elements/{id}` | Inchangé. |
| `arch://relationship/{id}` | `#/view/relationships/{id}` | **Route ajoutée** : elle manquait et produisait « page introuvable ». |
| `arch://interface/{id}` | `#/view/interfaces/{id}` | Inchangé. |
| `arch://version/{id}` | `#/view/versions/{id}` | **Route ajoutée** : résout l'interface porteuse puis cible la version. |
| `arch://story/{id}` | `#/view/stories/{id}` | Inchangé. |
| `arch://review/{id}` | `#/view/reviews/{id}` | Inchangé. |
| `arch://diagram/{id}` | `#/view/diagrams/{id}` | Lecture ; `ArchUri::route()` reste inchangé. |

Un lien `arch://` vers un objet archivé s'ouvre avec un bandeau « objet archivé » ; il
n'est **pas** une erreur 404. Seul un identifiant inexistant produit `LINK-001`.

## 6. Règles de rendu de la coque

- `/` ne rend **jamais** une liste de domaines, un tableau de bord ni un inventaire : c'est
  le portefeuille de produits. Tests bloquants : `home_opens_product_catalog`,
  `home_deep_link_restores_product_filters`.
- La Bibliothèque et le Contrôle sont accessibles par la barre latérale, jamais imposés.
- La coque ne contient plus « Domaines » comme entrée de navigation principale ; les
  entrées principales sont **Produits · Carte · Atelier**, puis **Bibliothèque · Contrôle ·
  Recherche · Réglages**.
- La bascule conception ↔ lecture est un **geste de barre d'outils** qui navigue de
  `#/design/:id` à `#/view/diagrams/:id` (et retour) en **sérialisant le cadrage courant**
  dans le lien. La sélection est conservée en mémoire de session par `view_id`.
- L'écran introuvable propose « Retour à l'accueil » et « Rechercher », jamais « Retour
  aux domaines ».
- `NotFound` conserve la chaîne demandée dans `detail` pour le diagnostic, jamais dans le
  titre affiché.

## 7. Parsing — ordre et robustesse

1. Retirer le `#` initial ; couper à `?` ; découper le chemin sur `/`.
2. Décoder les pourcentages (`%xx`) ; `+` reste un `+` (un fragment n'est pas un
   formulaire). Un encodage invalide : paramètre ignoré, `LINK-002`.
3. Appliquer les alias (`model` → `design`, `view/domains` → `library/domains`).
4. Résoudre la route ; un identifiant invalide produit `NotFound`, jamais un panic.
5. Parser la requête en `BTreeMap<String, String>` (ordre stable).
6. Une route inconnue produit `NotFound` avec la chaîne d'origine.

Aucune route ne dépend de l'ordre d'insertion d'une `HashMap` : les liens générés sont
déterministes (test `share_link_is_reproducible`).

## 8. Tests obligatoires (noms figés)

| Test | Prouve |
|---|---|
| `home_opens_product_catalog` | ADR-025 : `/` ouvre le portefeuille de produits, pas l'Atelier ni un inventaire |
| `home_deep_link_restores_product_filters` | ADR-025 : `#/?team=…&lifecycle=…&sort=…` restaure les filtres |
| `map_opens_enterprise_map` | ADR-023/025 : `/map` ouvre la carte en lecture Produits |
| `design_route_parses` / `design_new_route_parses` | VD-01 |
| `model_diagrams_alias_maps_to_design` | VD-01 |
| `view_domains_alias_maps_to_library_domain` | VD-01 |
| `relationship_route_resolves_arch_uri` | VD-01 |
| `version_route_resolves_to_interface` | VD-01 |
| `share_link_restores_framing` | VD-13 |
| `share_link_is_reproducible` | C-06 §4 |
| `share_link_opens_read_posture` | FR-040 |
| `unknown_focus_node_is_ignored_not_fatal` | LINK-003 |
| `posture_switch_preserves_framing_and_selection` | FR-048 |
| `library_and_control_are_separate_from_atelier` | VD-01, VD-15, VD-16 |
| `not_found_offers_home` | §15.4 |
