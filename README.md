# Archifier — MVP3 (v0.3) : l'Atelier de vues

Archifier est un **designer de vues** d'architecture d'entreprise. Le point d'entrée
n'est ni un tableau de bord, ni un inventaire : `/` ouvre l'**Atelier**. Le référentiel
se remplit comme conséquence du geste de conception, jamais comme préalable administratif
(ADR-022).

| | |
|---|---|
| Front | Rust + Leptos 0.8 (rendu client) → WebAssembly |
| Moteur | `archifier-core` : règles, temporalité, `arch://`, bus `ViewCommand` |
| Métamodèles | profils TOML externes (ArchiMate 3.2, BPMN 2.0 Core, UML 2.5.1, natif) |
| Persistance | IndexedDB, un enregistrement atomique par écriture |
| Bundle | **3,22 Mo** de WebAssembly |
| Tests | **309 tests verts** (`cargo test --workspace`) |
| Routage | fragment (`#/…`), ADR-012 |

## Démarrage

Prérequis : Rust ≥ 1.88 (toolchain épinglée), cible `wasm32-unknown-unknown`,
`wasm-bindgen-cli` 0.2.130, Node ≥ 20.

```powershell
.\scripts\build.cmd
node scripts\serve.mjs 4173
```

Puis <http://127.0.0.1:4173/>. Le premier lancement ouvre un référentiel
**vide** : l'Atelier propose de nommer le premier domaine, puis de créer la
première vue.

## Vérifications

```powershell
cargo test --workspace                      # 309 tests
cargo check -p archifier-ui --target wasm32-unknown-unknown
.\scripts\build.cmd
node tests\e2e\run.mjs                      # si Playwright est installé
```

## Ce que fait le produit

### L'Atelier (`/`)

- Ouvre la dernière vue éditée, sinon l'écran « Nouvelle vue » : **sept intentions**
  (contexte, applicatif, processus, déploiement, capacité, migration, revue d'interface),
  chacune avec un aperçu spatial, une palette, des bandes, un état vide et un modèle.
- `#/design/new?intent=…`, `#/design/:id` (conception), `#/view/diagrams/:id` (lecture
  partagée). Alias `#/model/diagrams/:id` et `#/view/domains/:id` conservés.

### Concevoir depuis la vue

- **Bus de commandes** (`ViewCommand`) validé par `archifier-core` avant projection :
  pose de croquis, objet existant, renommage, formalisation, connexion, reconnexion,
  déplacement, redimensionnement, zone, cadrage, curseur temporel, publication,
  suppression de représentation.
- **Croquis → brouillon → objet valide** : badge et motif permanents ; un croquis n'est
  jamais publié (`DSG-003`) ; une relation interdite est refusée avant création
  (`DSG-002`) ; un port sans interface est refusé (`DSG-004`) ; supprimer une forme ne
  supprime jamais l'objet (`DSG-005`).
- **Grammaire visuelle** : bandes de couches, zones de domaine, frontières, ports,
  versions, états temporels (plein / pointillé / fantôme + texte), zoom sémantique,
  aimantation aux guides.
- **Inspecteur non bloquant** : propriétés issues du profil, domaine, documentation,
  liens, historique. Aucun formulaire modal dans un parcours de création.
- **Curseur temporel** : ré-encode `SCHEDULED` / `ACTIVE` / `EXPIRED` sans déplacer une
  forme et sans écrire de donnée métier.
- **Édition fluide** : annulation/rétablissement (100 entrées), enregistrement immédiat
  après chaque commande, indicateur `Enregistré · révision N`, `Ctrl/Cmd+S` en secours,
  conflit `REV-409` avec recharger / comparer / forcer.
- **Clavier** : `V` `E` `R` `Z` `N` `F` `F2` `Échap`, `Ctrl/Cmd+K`, `Ctrl/Cmd+Z`,
  `Ctrl/Cmd+Maj+Z`, `Suppr`.
- **Partage** : lien avec cadrage et curseur, export SVG et Markdown, ajout à une Story,
  capture dans une Revue, publication / retrait.

### Espaces secondaires

- **Bibliothèque** (`#/library`) : couvertures de vues (titre, intention, vignette de
  cadrage, propriétaire, formes, croquis), filtres, recherche, synthèse de domaine.
- **Contrôle** (`#/control`) : anomalies cliquables, inventaire assumé, couverture des
  métamodèles, exports. Aucune création depuis le Contrôle.
- **Fiches** : élément, relation, interface, version, Story, Revue — atteintes depuis une
  vue, la Bibliothèque, la recherche ou `arch://`. Jamais imposées au lancement.
- **`/_design`** : galerie des tokens, primitives et états.

## Architecture

```text
crates/core        moteur pur : modèle, validation, view_command, temporalité
crates/metamodel   profils TOML + registre de relations
crates/repo        index mémoire, IndexedDB, recherche, cartes de vue
crates/collab      spike CRDT headless
crates/ui          Leptos : design/*, canvas/*, inspector/*, routes/*, library, control
```

Règles vérifiées par `crates/ui/tests/architecture.rs` :

- `canvas/**` ne lit pas le dépôt et n'écrit jamais le layout ;
- `inspector/**` ne fait aucune écriture dépôt ;
- toute écriture de layout passe par `design/commands.rs` ;
- la surface `Repository` reste gelée (six méthodes additives : C-09) ;
- `IdbStore` n'est connu que de `boot.rs`.

## Contrats et documentation

- `docs/adr/ADR-022-atelier-de-vues-point-entree.md` — décision structurante.
- `docs/contracts/` — neuf contrats gelés à G0 (commandes, schéma 2, intentions,
  statuts, modèles, routes, tokens, codes, surface dépôt).
- `docs/ui-spec.md` — grammaire, gestes, états, intentions.
- `docs/backlog-mvp3.md` — 26 points VD-*, traçabilité et état réel.
- `technical-specifications-lot3.md` — écarts, architecture, vagues, portes.

## Limites assumées

- **E2E Playwright non exécuté dans cet environnement** : `npm` est bloqué par la
  politique PowerShell. La chaîne de conception a été rejouée dans le navigateur intégré
  et les mêmes sélecteurs sont dans `tests/e2e/design-chain.spec.ts`.
- **Budgets de performance non mesurés** : les fixtures 200/500 formes existent, le
  protocole est décrit, la mesure instrumentée reste à exécuter (VD-20).
- **i18n partielle sur les écrans historiques** : l'Atelier, la Bibliothèque, le Contrôle,
  les erreurs et les intentions passent par `i18n/fr.json` (test dédié) ; les fiches
  lot 1/2 conservent des libellés en dur (VD-25 partiel).
- **Pas de disposition automatique** ni de routage avancé des arêtes.
- **Pas d'IA générative** : aucune clé dans le navigateur (ADR-013).
- **Collaboration temps réel** : commandes idempotentes prêtes, UI non activée (ADR-014).
- **Sécurité simulée** : identités commutables, pas de RBAC.
- Les données sont propres à ce navigateur et à cette origine.
