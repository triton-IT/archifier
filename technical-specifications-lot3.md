# Spécification technique — Lot 3 (MVP3) : l'Atelier de vues

| | |
|---|---|
| **Document** | Plan technique du MVP3 : écarts, architecture, vagues, portes |
| **Version** | 0.9 — 18/11/2026 : socle, grammaire, navigation, partage implémentés |
| **Date** | 18 novembre 2026 |
| **Statut** | **Contrats gelés** ; G0→G4 franchies, G5 partielle (E2E/perf), G6 documentée |
| **Entrées** | `functional-specifications.md`, `technical-specifications.md` §14.4/§15, ADR-022, `docs/contracts/*`, `docs/ui-spec.md`, `backlog-mvp1.md`, `docs/backlog-mvp2.md` |
| **Complète** | `technical-specifications.md` sans le réécrire : les écarts sont consignés au §2 |

> **Statut historique.** Ce plan décrit le MVP3 livré sous ADR-022, où `/` ouvrait
> l'Atelier. Depuis ADR-023/024/025, `/` ouvre le **portefeuille de produits**, `/map` ouvre
> la Carte de l'entreprise ; l'Atelier est accessible par `/atelier`, `/design/:id` et
> `/view/diagrams/:id`.

---

## 1. Objet

Transformer une application dont `/` est un inventaire de domaines en un **designer de
vues** dont `/` est l'Atelier. Le lot 3 ne réécrit pas le moteur : il ajoute un **bus de
commandes**, un **schéma de layout 2**, une **grammaire visuelle**, puis réorganise la
navigation autour de trois espaces (Atelier, Bibliothèque, Contrôle).

Ce qui ne change pas :

- `archifier-core` reste pur, sans I/O.
- `Repository` reste le port unique (ADR-016) ; évolutions additives seulement (C-09).
- Aucune migration destructive ; les snapshots existants restent lisibles.
- Aucune nouvelle dépendance produit (layout, graphe, icônes, virtualisation) sans ADR.

## 2. Registre des écarts

Chaque écart est **constaté**, **arbitré** et **tracé**. Aucun n'est corrigé en silence.

| # | Écart constaté | Source | Arbitrage | Impact |
|---|---|---|---|---|
| E-01 | §15.1/§15.2 décrivent un routage `history` avec repli SPA ; l'application est en **fragment** (`#/…`). | ADR-012 vs §15.1 | **ADR-012 maintenu.** Les chemins de §15.2 deviennent des chemins logiques (C-06). Changer de transport casserait les liens et n'apporte rien au produit statique. | Aucun code de routage serveur ; liens partagés en `#/` |
| E-02 | §14.4.2 ne définit ni `command_id`, ni `view_id`, ni `expected_revision`, ni les identités créées. | VD-03/VD-04 vs §14.4.2 | **Ajout additif** en C-01 : l'idempotence et le 409 exigent une identité de commande et d'objet. Aucun champ retiré. | Bus sérialisable, rejouable |
| E-03 | §14.4.1 ne modélise **ni ports ni frontières**, alors que FR-044 et VD-10 l'exigent. | FR-044 vs §14.4.1 | **Ports = nœuds `INTERFACE` ancrés** (`node.port`), frontière = zone `DOMAIN` (`boundary`). Aucune 14e commande : `PlaceDraft`/`PlaceExisting`/`Formalize` suffisent (C-01, C-02). | Rendu et validation cohérents |
| E-04 | Le profil `archimate:3.2` ne contient pas `Capability` (ni couche Stratégie). | C-03 §7, ADR-021 | **Aucune classe inventée.** L'intention `CAPABILITY` utilise `BusinessFunction`/`BusinessService`/`Product`/`Goal`. Ajouter la classe exige une évolution de profil versionnée. | Palette honnête, pas de « 4 » |
| E-05 | Le layout schéma 1 n'a pas de `grid` alors que §14.1 en montre un ; `collapsed` manque aussi. | §14.1 vs `model.rs` | **Ajout additif en schéma 2** (C-02 §3), avec défauts. | Migration sans perte |
| E-06 | L'accueil actuel affiche un bouton « Enregistrer » et un formulaire de domaine. | FR-050, ADR-022 | **Supprimés de l'Atelier** ; `Ctrl/Cmd+S` en secours ; autosave 800 ms. | VD-04 |
| E-07 | Aucun module i18n ; toutes les chaînes sont en dur. | VD-25 | **Socle i18n créé en W3 avant les écrans**, audit bloquant en W5. | Dette mesurée, pas masquée |
| E-08 | `arch://relationship` et `arch://version` produisent « page introuvable ». | VD-01 | **Deux routes ajoutées** (C-06 §5) + une lecture additive `interface_of_version` (C-09 §4.5). | Aucun lien cassé |
| E-09 | Aucun E2E, aucun `tests/e2e`. | VD-22 | **Infrastructure Playwright créée en W1**, scénarios en W5. | Preuve de recette |
| E-10 | La publication d'une vue n'existe pas ; `published_diagrams` compte les vues actives. | VD-13 | **`publication_status` additif sur `Diagram`** (C-02 §4) + `set_diagram_publication`. | Lien partageable, retrait possible |
| E-11 | Le rôle UX possède `route /_design`, mais `router.rs` est gelé en W1 et réservé à Navigation. | §6 de la mission vs §9.2 | **UX livre `primitives::gallery`** ; la route `#/_design` est **câblée par Navigation en W3** (une ligne de dispatch). Le périmètre d'écriture reste exclusif. | Aucun conflit de fichiers |
| E-12 | Les commandes `add_node`, `set_node_position`, `add_edge`, `remove_node` écrivent le layout hors bus. | C-01 §1 | **Conservées pour compatibilité**, plus appelées par l'Atelier ; test d'architecture bloquant. | Pas de double chemin d'écriture |

Un écart **nouveau** découvert en cours de vague est ajouté ici par l'orchestrateur, avec
un ADR s'il change une décision produit ou un contrat.

## 3. Architecture cible

```text
crates/core
├─ model.rs              + champs additifs schéma 2 (C-02)
├─ validation.rs         + DGM-007..015 (C-08)
├─ view_command.rs       ViewCommand, ViewWorld, validate, CommandReceipt (C-01)
├─ ids.rs                inchangé
└─ temporal.rs           inchangé

crates/metamodel           inchangé (aucune classe ajoutée)

crates/repo
├─ lib.rs                + 5 méthodes additives (C-09 §4)
└─ types.rs              + ViewFilter, ViewCard, MiniNode

crates/ui/src
├─ app.rs                coque : Atelier · Bibliothèque · Contrôle (W3)
├─ router.rs             routes C-06 + alias (W1 seam, puis W3)
├─ state/
│  └─ last_view.rs       localStorage `archifier:last_view` (C-09 §6)
├─ design/
│  ├─ commands.rs        adaptateur ViewWorld + exécution des effets
│  ├─ history.rs         undo/redo ≥ 100, autosave 800 ms, 409
│  ├─ tools.rs           outil actif, raccourcis
│  ├─ selection.rs       sélection simple/multiple, focus, cadrage
│  └─ formalize.rs       SKETCH → FORMALIZED → VALID (C-04)
├─ canvas/
│  ├─ mod.rs             Canvas, culling, transform racine
│  ├─ layers.rs          bandes
│  ├─ zones.rs           zones, frontières, groupes
│  ├─ node.rs            formes, glyphes, badges
│  ├─ port.rs            ports, versions, consommateurs
│  ├─ edge.rs            arêtes orthogonales
│  └─ minimap.rs         minimap
├─ inspector/
│  ├─ mod.rs             panneau non bloquant
│  ├─ properties.rs      propriétés générées par le profil
│  ├─ documentation.rs   Markdown
│  ├─ links.rs           liens entrants/sortants
│  └─ history.rs         révisions et diff
├─ library/              couvertures, filtres, synthèse de domaine
├─ control/              anomalies, inventaire, couverture, exports
├─ components/
│  ├─ primitives/        boutons, champs, badges, dialogues (gelés)
│  └─ gallery.rs         galerie `/_design`
├─ routes/
│  ├─ atelier.rs         `/`, `/design/new`
│  ├─ design.rs          `/design/:id`, `/view/diagrams/:id` (Canvas)
│  ├─ library.rs
│  ├─ control.rs
│  └─ design_system.rs   `/_design`
└─ i18n/
   ├─ mod.rs             clés typées, `t(key)`
   └─ fr.rs              catalogue (ou `i18n/fr.json` inclus)
```

Règles de dépendance :

| Règle | Test |
|---|---|
| `core` ne dépend d'aucune I/O, ni de `leptos`, ni de `repo` | `cargo tree`, revue |
| `canvas/**` ne connaît ni `Repository` ni `save_layout` | `canvas_never_writes_layout_directly` |
| `inspector/**` ne connaît pas `Repository` en écriture | `inspector_never_writes_repository` |
| une seule fonction exécute les `RepoEffect` | `every_layout_write_goes_through_commands` |
| aucune chaîne d'interface hors i18n | `no_hardcoded_string_in_ui` |
| aucun token recopié dans un composant | `no_hardcoded_token_in_components` |

## 4. Contrats G0

Voir `docs/contracts/README.md`. Rappel : C-01 `ViewCommand`, C-02 schéma 2, C-03
intentions, C-04 statuts, C-05 modèles, C-06 routes, C-07 tokens, C-08 codes, C-09 dépôt.
Toute évolution est additive et documentée dans le journal du contrat.

## 5. Vagues, agents et périmètres exclusifs

**Règle absolue :** un fichier n'a qu'un écrivain par vague. Une vague ne démarre pas si la
précédente n'est pas verte. Maximum **4 agents actifs**.

### W0 — Cadrage (3 agents) → G0

| Rôle | Périmètre d'écriture exclusif | Livrables |
|---|---|---|
| Orchestrateur | `docs/contracts/**`, `docs/backlog-mvp3.md`, `technical-specifications-lot3.md`, `docs/ui-spec.md` (base), ADR | contrats, backlog, plan, registre d'écarts |
| UX | `assets/tokens.css`, `assets/app.css` (consommation), `assets/view-templates/*.json`, `crates/ui/src/components/primitives/**` | tokens, 7 modèles, primitives, maquette de `/` |
| Analyste | `fixtures/views/*.json`, `i18n/fr.json`, `docs/acceptance/*.md` | 7 fixtures, libellés, Gherkin, jeu « chaîne de conception » |

**Preuve G0** : contrats figés ; maquette de `/` ; 7 modèles valides (C-05) ; 7 fixtures ;
tests squelettes `#[ignore]` nommés ; `cargo test --workspace` toujours à 253 verts.

### W1 — Socle (3 agents) → G1

| Rôle | Périmètre exclusif | Livrables |
|---|---|---|
| Dev Atelier | `crates/core/src/view_command.rs`, `crates/core/src/model.rs`, `crates/core/src/validation.rs`, `crates/ui/src/design/{commands,history,tools,selection}.rs`, `crates/ui/src/state/last_view.rs`, `crates/repo/src/{lib,types}.rs` | bus, schéma 2 + migration, undo/autosave, outils, 5 méthodes additives |
| Dev Canvas | `crates/ui/src/canvas/**`, `crates/ui/src/routes/design.rs` | rendu SVG schéma 2, bandes/zones/ports (lecture), gestes de base |
| Testeur | `tests/e2e/**`, `crates/*/tests/**` | infra Playwright, squelettes dé-ignorés, tests bus/migration |

**Seam de routage W1 (signé orchestrateur)** : une seule modification additive de
`router.rs` + `views/mod.rs` ajoute `Route::Atelier`, `Route::Design`, `Route::DesignNew`
et leur dispatch. Aucune route existante ne change. Ensuite `router.rs` est gelé jusqu'en
W3.

**Preuve G1** : déposer une forme, la nommer, annuler, rouvrir → tout est restauré ; aucun
modal ; `REV-409` sans perte.

### W2 — Grammaire (3 agents) → G2

| Rôle | Périmètre exclusif | Livrables |
|---|---|---|
| Dev Canvas | `crates/ui/src/canvas/**` | bandes éditables, zones, aimantation, frontières, ports, temporalité, culling, minimap, zoom sémantique |
| Dev Inspecteur | `crates/ui/src/inspector/**`, `crates/ui/src/design/formalize.rs` | inspecteur non bloquant, propriétés de profil, cycle de statut, `DSG-001/004` |
| UX (revue) | `docs/ui-spec.md` (corrections), `assets/tokens.css` (valeurs) | revue de grammaire, corrections de tokens, aucun code métier |

**Preuve G2** : démo 3 composants, 1 frontière, 2 ports, 1 migration v1→v2 lue au curseur
temporel, géométrie inchangée.

### W3 — Entrée (3 agents) → G3

| Rôle | Périmètre exclusif | Livrables |
|---|---|---|
| Dev Navigation | `crates/ui/src/{app.rs,router.rs,routes/*}`, `library/**`, `control/**`, `state/**` sauf `last_view.rs`, `i18n/**` | routes C-06, `/` Atelier, 7 intentions, Bibliothèque, Contrôle, onboarding, `/_design` câblé, fiche |
| Dev Inspecteur | `crates/ui/src/inspector/**`, `design/formalize.rs` | fiche objet complète, liens `arch://`, historique, publication assistée |
| Analyste | `i18n/fr.json`, `docs/acceptance/*.md`, `fixtures/views/*.json` | libellés définitifs, scénarios, données de démo |

**Preuve G3 (MVP3)** : `/` ouvre l'Atelier ; les 7 intentions créent une vue ; Bibliothèque et
Contrôle séparés ; aucun inventaire au lancement.

### W4 — Partage (3 agents) → G4

| Rôle | Périmètre exclusif | Livrables |
|---|---|---|
| Dev Navigation | `routes/*`, `app.rs`, `library/**`, `control/**`, `i18n/**` | lien partagé, export SVG/Markdown, Story/Revue, palette de commandes |
| Dev Atelier | `design/{commands,history,tools,selection}.rs`, `core/src/view_command.rs` | cadrage comme commande, publication, export via commandes, `Ctrl/Cmd+Maj+P/L` |
| Testeur | `tests/e2e/**`, `crates/*/tests/**` | E2E lien partagé, cadrage, publication, Story/Revue |

**Preuve G4** : lien restitue le cadrage ; vue ajoutée à une Story ; export SVG et
Markdown identiques au cadrage.

### W5 — Qualité (3 agents) → G5

| Rôle | Périmètre exclusif | Livrables |
|---|---|---|
| Testeur | `tests/e2e/**`, `crates/*/tests/**` | 5 scénarios Gherkin, chaîne de conception, budgets, a11y, non-régression |
| UX | `docs/ui-spec.md`, `assets/tokens.css`, `components/primitives/**`, `components/gallery.rs` | galerie complète, corrections de contraste, revue visuelle |
| Relecteur | *aucun fichier* (lecture seule) | verdict GO/NO-GO, violations référencées |

**Preuve G5** : E2E verts, budgets tenus, a11y sans erreur critique, 253 tests + nouveaux
verts.

### W6 — Recette (3 agents) → G6

| Rôle | Périmètre exclusif | Livrables |
|---|---|---|
| Orchestrateur | `docs/**`, `README.md`, `backlog-mvp3.md` | backlog à jour, README, traçabilité |
| Relecteur | lecture seule | verdict final, liste des écarts résiduels |
| Testeur | `tests/e2e/**` | chronométrage de la chaîne, rapport de recette |

**Preuve G6** : chaîne de conception < 3 min, sans modal ni aide ; 26 points verts.

## 6. Portes — commandes de vérification

```powershell
# à chaque porte
cargo test --workspace
cargo check -p archifier-ui --target wasm32-unknown-unknown
.\scripts\build.cmd

# portes G3+
node scripts\serve.mjs 4173
node tests\e2e\run.mjs            # Playwright sur le build statique

# mesure du bundle
(Get-Item pkg\archifier_ui_bg.wasm).Length / 1MB
```

| Porte | Critère bloquant | Preuve archivée |
|---|---|---|
| G0 | contrats, maquette, 7 modèles, fixtures, squelettes | `docs/contracts/`, `docs/ui-spec.md`, `assets/view-templates/`, `fixtures/views/` |
| G1 | pose/nom/annulation/restauration, zéro modal | tests `design::*` + E2E `e2e_first_shape` |
| G2 | 3 composants, 1 frontière, 2 ports, migration temporelle | E2E `e2e_design_grammar` + capture |
| G3 (MVP3) | `/` Atelier, 7 intentions, espaces séparés | E2E `e2e_home_is_atelier` |
| G4 | lien + cadrage, Story, export | E2E `e2e_shared_link`, fichiers exportés |
| G5 | budgets, a11y, non-régression | rapport testeur |
| G6 | chaîne < 3 min, docs à jour | rapport recette + `README.md` |

## 7. Stratégie de test

| Niveau | Emplacement | Portée |
|---|---|---|
| Unitaire moteur | `crates/core/src/**` (`#[cfg(test)]`) | commandes, projection, migration, validation |
| Intégration dépôt | `crates/repo/tests/**` | 5 méthodes additives, 409, publication, cartes de vue |
| Unitaire UI | `crates/ui/tests/**` (natif) | outils, sélection, historique, i18n, modèles |
| Architecture | `crates/ui/tests/architecture.rs` | frontières de modules, surface dépôt, chaînes/tokens en dur |
| E2E | `tests/e2e/**` | 5 scénarios Gherkin, chaîne, lien, refus, temporalité |

Règles :

1. Un test dérive de la spec, jamais du code livré. Un comportement incompris est un
   défaut de spec, remonté — pas deviné.
2. Le nom du test est le nom du contrat (`connect_forbidden_relation_is_rejected_before_creation`).
3. Un test squelette `#[ignore]` réserve un nom ; il ne prouve rien et ne compte pas.
4. Le testeur ne corrige jamais le code pour faire passer un test.

### 7.1 Fixtures

- `fixtures/views/<intent>.json` : 7 snapshots de vue, domaine **Paiements**, migration
  v1/v2, 3 à 12 formes.
- `fixtures/views/heavy-200.json`, `heavy-500.json` : budgets de performance.
- `fixtures/views/legacy-v1.json` : migration schéma 1 → 2.
- `fixtures/views/dsg-cases.json` : `DSG-001..005`, `REV-409`.
- `fixtures/views/design-chain.json` : jeu rejouable de la chaîne de conception.

## 8. Mesure des budgets (VD-20)

| Budget | Méthode |
|---|---|
| Première forme < 5 s | Playwright : `navigationStart` → apparition du premier nœud croquis dans le DOM. 5 exécutions, médiane. |
| Création ≤ 3 gestes | comptage d'événements instrumentés (`pointerdown`, `keydown` utiles) dans l'E2E. |
| Vue 200 formes < 1 s | ouverture de `heavy-200.json` : navigation → canevas stable (2 frames sans mutation). |
| 500 formes, 30/60 fps | `requestAnimationFrame` pendant 3 s de pan/zoom scripté ; compte des frames > 50 ms et des fps moyens. |
| Aucune frame > 50 ms | même captation ; toute frame > 50 ms échoue. |
| Bundle ≤ 3,5 Mo | taille du `.wasm` après `build.cmd` (2,76 Mo au départ). |

Les mesures sont consignées dans le rapport de porte, avec la machine et la version du
navigateur. Un budget non mesuré n'est pas « tenu ».

## 9. Accessibilité (VD-21)

- Audit automatisé `axe-core` injecté par Playwright (dépendance de **test** uniquement).
- Checklist clavier manuelle : tous les gestes du §11 de `docs/ui-spec.md`.
- Vérification des contrastes sur les paires de tokens déclarées (C-07 §10).
- `prefers-reduced-motion` simulé, transitions à 0 ms.
- Aucun état vide sans action.

## 10. Migration et retour arrière

- **Données** : le schéma 2 est additif ; un binaire antérieur lit un layout schéma 2 sans
  erreur (les champs inconnus sont ignorés) et conserve `nodes`/`edges`. Aucune migration
  destructive.
- **Layout** : migration idempotente, une seule révision d'audit, aucune géométrie modifiée.
- **Retour arrière** : `git revert` de la vague ; les vues migrées restent lisibles. En cas
  de doute, export de snapshot avant migration (bouton du Contrôle).
- **Stockage** : aucun nouveau store IndexedDB ; le layout reste dans le snapshot unique.
  `localStorage` ne porte que des préférences (C-09 §6).

## 11. Risques techniques

| Risque | Probabilité | Impact | Mitigation |
|---|---|---|---|
| Le bus et le layout divergent | moyenne | haut | une seule projection, tests d'architecture, aucun `save_layout` hors `design::commands` |
| Le rendu 500 formes casse | moyenne | haut | culling ADR-020, groupes SVG séparés, mesure W1 puis W5 |
| La migration de layout perd une forme | faible | critique | diff de géométrie, fixture `legacy-v1.json`, idempotence |
| L'i18n arrive après les écrans | haute | moyen | socle i18n en W3 **avant** les écrans, audit bloquant W5 |
| Les alias de routes cassent des liens | moyenne | moyen | table C-06, tests de résolution `arch://` |
| Le rôle UX et la route `/_design` entrent en conflit | moyenne | faible | E-11 : galerie UX, câblage Navigation |
| Le budget 3 gestes est dépassé par la formalisation | moyenne | moyen | formalisation différée, test de comptage |

## 12. Définition de « terminé »

1. La démonstration en 10 étapes de la mission passe sans manipulation cachée, en < 3 min.
2. Les 26 points VD-* ont chacun un test nommé et vert.
3. `cargo test --workspace` reste vert, y compris les 253 tests d'origine et les scénarios
   lot 1/2.
4. `cargo check -p archifier-ui --target wasm32-unknown-unknown` et `.\scripts\build.cmd`
   sont verts ; le bundle reste ≤ 3,5 Mo.
5. Aucun modal dans un parcours de création ; aucun inventaire au lancement.
6. `docs/ui-spec.md`, `docs/backlog-mvp3.md`, `README.md` et `docs/contracts/*` sont à jour ;
   ADR-022 est référencé.
7. Le relecteur a rendu un verdict **GO** pour G1 → G6.

## 13. Commandes de référence

```powershell
cargo test --workspace
cargo check -p archifier-ui --target wasm32-unknown-unknown
.\scripts\build.cmd
node scripts\serve.mjs 4173
# puis http://127.0.0.1:4173/
```

## 14. Journal

| Date | Révision | Décision |
|---|---|---|
| 18/11/2026 | 0.1 | Création : registre des écarts, architecture, vagues W0→W6, portes |
| 18/11/2026 | 0.9 | Implémentation : bus, schéma 2, canevas, inspecteur, navigation, partage |

## 15. État d'implémentation au 18/11/2026

### 15.1 Livré et vérifié

| Domaine | Preuve |
|---|---|
| Bus `ViewCommand` (13 commandes, `ViewWorld`, projection, migration) | **20 tests core** nommés (`place_draft_*`, `connect_*`, `publish_*`, `migration_*`, `temporal_cursor_*`) |
| Schéma 2 + migration v1→v2 | `migration_preserves_node_geometry`, `migration_is_idempotent` ; vues seed migrées à l'ouverture (4 nœuds, 3 arêtes, 3 bandes) |
| Surface dépôt additive | `view_cards_acceptance.rs` (8 tests), `REV-409` sans écriture |
| Routage et alias | 9 tests `router::tests` (`home_parses_as_atelier`, alias, lien partagé reproductible) |
| Modèles de vue | 5 tests `tests/templates.rs` (7 modèles, aucun `object_id`, instanciation sans objet) |
| Tokens et design system | 3 tests `tests/tokens.rs` (familles, 13 couches, reduced-motion) ; `/_design` |
| i18n | 4 tests `tests/i18n_keys.rs` (clés littérales, codes d'erreur, 7 intentions, libellés de couches) |
| Architecture | 8 tests `tests/architecture.rs` (canvas sans dépôt, inspecteur sans écriture, layout via commandes, surface gelée) |
| Non-régression | **307 tests** verts, dont les 253 d'origine |

### 15.2 Vérifié dans le navigateur (chaîne de conception)

Ouverture de `/` → Atelier ; création d'une vue Applicatif (domaine Paiements) ;
pose de composants ; renommage ; formalisation (`VALIDÉ`) ; relation `serving` ;
zone de domaine ; port rattaché à `Payment API` ; curseur au 01/08/2027 (port
« Expirée ») ; `DSG-001` (type vide) ; `DSG-003` (publication avec croquis) ;
publication réussie après retrait des croquis ; lien partagé restituant posture
lecture, zoom et curseur ; Bibliothèque (couvertures) ; Contrôle (2 anomalies,
aucune création) ; `/` reprend la dernière vue ; Story enrichie d'un bloc `Vue`.

### 15.3 Non prouvé automatiquement (à ne pas déclarer « fait »)

| Point | Ce qui manque |
|---|---|
| VD-04 | Test automatisé du couple undo/redo + conflit UI (le 409 dépôt et le rejeu core sont testés) |
| VD-05 / VD-21 | Test DOM des raccourcis globaux et audit axe |
| VD-15 / VD-16 / VD-17 / VD-18 / VD-19 | Tests E2E des écrans (parcours manuel seulement) |
| VD-20 | Mesure instrumentée des budgets (fixtures 200/500 présentes) |
| VD-22 | Exécution Playwright (spec écrite, binaire absent) |
| VD-25 | Écrans historiques (fiches lot 1/2) encore partiellement en dur |

Aucun de ces points n'est marqué `✅` dans `docs/backlog-mvp3.md`.
