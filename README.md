# Archifier — la carte produit de l'entreprise

Archifier est un outil d'architecture d'entreprise qui se parcourt comme une carte :
**de la big picture à l'infrastructure**, en passant par les value streams, les produits,
les applications et les interfaces.

| | |
|---|---|
| **Cible produit** | v0.4 — portefeuille de produits à l'accueil, carte de l'entreprise, ontologie produit, zoom sémantique (ADR-023/024/025) |
| **Implémentation présente** | v0.4 première tranche : portefeuille filtrable, carte L0–L4, ontologie native, pages d'objet, création inline ; Atelier MVP3 conservé |
| **Statut de ce dépôt** | **La carte produit est implémentée et testée** ; les lentilles avancées, l'impact et la migration guidée restent à approfondir. |
| **Front** | Rust + Leptos 0.8 → WebAssembly |
| **Moteur** | `archifier-core` : règles, temporalité, `arch://`, bus `ViewCommand` |
| **Métamodèles** | profils TOML externes (ArchiMate 3.2, BPMN 2.0 Core, UML 2.5.1, natif) |
| **Persistance** | IndexedDB, un enregistrement atomique par écriture |
| **Routage** | fragment (`#/…`), ADR-012 |

## Le changement de cap

Le MVP3 ouvrait sur un **Atelier de vues** et demandait de choisir une intention, puis de
nommer un domaine. Ce parcours est trop guidé : il ne répond pas d'abord à la question
« à quoi ressemble mon entreprise ? », et le mot « domaine » mélange propriété,
classification et regroupement.

La cible v0.4 renverse le parcours (ADR-023) :

1. `/` ouvre le **portefeuille de produits** filtrable (ADR-025) ; la carte est sur `#/map` ;
2. on zoome sur un **value stream**, un **produit**, une **application**, puis une
   **infrastructure** ;
3. les **interfaces** sont visibles à chaque niveau, avec fournisseur, consommateurs,
   versions et état ;
4. les **lentilles** (valeur, capacité, organisation, interface, temps, risque…) changent
   la lecture sans déplacer la géographie ;
5. l'**Atelier** reste la surface d'édition des vues authored, atteinte depuis la carte.

Le domaine devient un **domaine métier** : une classification. La responsabilité est portée
par une **équipe** qui possède des **produits** et opère des **applications**.

| Avant (MVP3) | Après (cible v0.4) |
|---|---|
| Accueil = Atelier, choix d'une intention | Accueil = portefeuille de produits filtrable |
| Domaine = pivot de propriété et de navigation | Produit + équipe ; domaine = classification |
| Intention de vue = préalable | Lentille de carte ou action de détail |
| Interface = flèche ou domaine fournisseur | Interface = objet versionné, fournisseur/consommateurs |
| Application = composant ArchiMate | Application = système utilisé par des produits ; composant = partie |
| Une vue = un diagramme | Une carte = projection ; une vue = récit authored |

## La cible v0.4 en un écran

L'accueil est le **portefeuille de produits** filtré (`#/`) ; la carte ci-dessous est
`#/map`.

```text
L0  Entreprise        quartiers = value streams / portefeuilles
                      bâtiments = produits
                      utilités  = produits plateformes
                      routes    = interfaces agrégées
 │
 ├─ L1  Portefeuille  produits du périmètre, plateformes, interfaces produit à produit
 │
 ├─ L2  Produit       frontière, ports, applications, interfaces applicatives
 │
 ├─ L3  Application   composants, services, interfaces, données, déploiement
 │
 └─ L4  Infrastructure nœuds, environnements, réseaux, applications déployées
```

| Dimension | Lentille | Question |
|---|---|---|
| Valeur | `VALUE` | Quel parcours, quelles étapes, quels produits ? |
| Capacité | `CAPABILITY` | Quelles capacités, couvertes par quoi, où sont les trous ? |
| Produit | `PRODUCT` | Quel portefeuille, quel cycle de vie, quelle criticité ? |
| Organisation | `ORGANIZATION` | Qui possède quoi, où sont les dépendances d'équipe ? |
| Applicatif | `APPLICATION` | Quelles applications, quels composants, quel âge ? |
| Interface | `INTERFACE` | Qui expose/consomme quoi, quelles versions ? |
| Technologie | `TECHNOLOGY` | Quelles plateformes, quelle obsolescence ? |
| Données | `DATA` | Quels objets, quels flux, quelle classification ? |
| Temps | `TIME` | Où en est-on à T, que se passe-t-il ? |
| Risque / coût | `RISK` | Où sont les risques, coûts, dettes ? |

## La cible documentaire

- `docs/adr/ADR-023-produit-carte-entreprise.md` — décision structurante : produit, carte,
  zoom, lentilles, place d'ArchiMate/BPMN/UML.
- `docs/adr/ADR-024-produit-pivot-multi-centre.md` — le produit est le pivot de la carte,
  pas le centre du référentiel.
- `docs/adr/ADR-025-accueil-portefeuille-produits.md` — l'accueil est le portefeuille de
  produits filtrable ; la carte reste `#/map`.
- `functional-specifications.md` § « Cible v0.4 — Le produit, la carte et le zoom » —
  ontologie, règles `PRD-*`, portefeuille, carte, lentilles, pages, interfaces, migration,
  FR-055..082, scénarios d'acceptation, trajectoire.
- `docs/ui-spec.md` v2.2 — portefeuille de produits, carte, cinq niveaux, lentilles, pages
  produit/équipe/value stream/capacité/application/infrastructure, états, accessibilité.
- `docs/contracts/C-10-produit-carte-navigation.md` — objets, cardinalités, portefeuille
  (`ProductQuery`), projection `MapQuery → MapProjection`, routes, agrégation, codes
  `MAP-*`/`PRD-*`, tests nommés.
- `technical-specifications.md` §15.0/§15.2 — avenant : l'accueil devient le portefeuille
  (`/`), la carte passe sur `/map`, l'Atelier reste la surface d'édition.
- `docs/contracts/` — contrats MVP3 gelés (C-01..C-09), conservés ; C-10 est additif.

## État implémenté (v0.4 + MVP3)

L'accueil est le **portefeuille de produits** (`#/`), filtrable (`ProductQuery::home()`,
ADR-025). La **Carte de l'entreprise** est sur `#/map` (`MapQuery::map_default()`). L'Atelier
reste la surface d'édition des vues authored.

### Le portefeuille de produits (`#/`)

- liste de cartes produit avec recherche et filtres : équipe, type, cycle de vie,
  criticité, value stream, capacité, application, niveau/exposition d'interface,
  complétude ;
- chaque carte montre l'équipe accountable, le value stream, les compteurs de capacités,
  applications et interfaces, les signaux et les manques ;
- filtres sérialisés dans l'URL (`#/?team=…&lifecycle=…&capability=…`) ;
- actions : ouvrir la fiche produit, voir le produit sur la carte, créer un produit.

### La carte v0.4 (`#/map`)

- `#/map` ouvre la carte L0 (scope et niveau Entreprise, lentille primaire Produit) ; à L0,
  les seuls nœuds sont des produits, les quartiers sont le contexte ;
- cinq niveaux : `ENTERPRISE`, `PORTFOLIO`, `PRODUCT`, `APPLICATION`,
  `INFRASTRUCTURE` ;
- dix lentilles : valeur, capacité, produit, organisation, applicatif, interface,
  technologie, données, temps, risque ;
- quartiers = value streams / plateformes / non rattaché / environnements ;
- lignes agrégées depuis les consommations d'interface, dépliables dans le panneau ;
- lien profond `#/map?scope=…&level=…&lens=…&date=…&focus=…` ;
- liste accessible des objets et recherche intégrée ;
- création inline non modale : produit, équipe, value stream, application, interface,
  capacité, nœud ;
- pages d'objet : produit, équipe, value stream, étape, capacité, application,
  composant, interface, infrastructure, domaine métier ;
- bouton « Charger un exemple » pour un référentiel de démonstration (assurance),
  jamais chargé automatiquement et jamais destructif.

Preuves : `cargo test -p archifier-core map::`,
`cargo test -p archifier-repo --test map_acceptance`, `cargo test -p archifier-ui`.
Décisions et écarts : `docs/v0.4/decision-log.md`,
`docs/v0.4/implementation-report.md`.

### L'Atelier (`#/atelier`, `#/design/:id`)

- Bus de commandes (`ViewCommand`) validé par `archifier-core` avant projection : pose de
  croquis, objet existant, renommage, formalisation, connexion, reconnexion, déplacement,
  redimensionnement, zone, cadrage, curseur temporel, publication, suppression de
  représentation.
- **Croquis → brouillon → objet valide** : badge et motif permanents ; un croquis n'est
  jamais publié (`DSG-003`) ; une relation interdite est refusée avant création
  (`DSG-002`) ; un port sans interface est refusé (`DSG-004`) ; supprimer une forme ne
  supprime jamais l'objet (`DSG-005`).
- **Grammaire visuelle** : bandes de couches, zones de domaine, frontières, ports,
  versions, états temporels (plein / pointillé / fantôme + texte), zoom sémantique,
  aimantation aux guides.
- **Inspecteur non bloquant** : propriétés issues du profil, domaine, documentation,
  liens, historique.
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

- **Bibliothèque** (`#/library`) : couvertures de vues, filtres, recherche, synthèse de
  domaine.
- **Contrôle** (`#/control`) : anomalies cliquables, inventaire assumé, couverture des
  métamodèles, exports.
- **Fiches** : élément, relation, interface, version, Story, Revue — atteintes depuis une
  vue, la Bibliothèque, la recherche ou `arch://`.
- **`/_design`** : galerie des tokens, primitives et états.

## Démarrage

Prérequis : Rust ≥ 1.88 (toolchain épinglée), cible `wasm32-unknown-unknown`,
`wasm-bindgen-cli` 0.2.130, Node ≥ 20.

```powershell
.\scripts\build.cmd
node scripts\serve.mjs 4173
```

Puis <http://127.0.0.1:4173/>. Le premier lancement ouvre la **carte de l'entreprise**
sur un référentiel **vide**, en lecture **Produits** : créez un produit (son équipe
responsable est demandée juste après), ou chargez l'exemple. Aucun formulaire de domaine
n'est demandé.

```powershell
cargo test --workspace                      # suite MVP3
cargo check -p archifier-ui --target wasm32-unknown-unknown
node tests\e2e\run.mjs                      # si Playwright est installé
```

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

La carte v0.4 utilise un module de **projection de carte** (requête, agrégation, ancres)
distinct du layout authored. Il ne réutilise pas `DiagramLayout` : la carte a sa propre
géométrie stable (C-10 §6 et §10).

## Limites assumées

- **Ancres manuelles** : la commande et la persistance existent, le glisser-déposer
  n'est pas encore exposé dans la carte.
- **Lentilles superposées** : la projection accepte plusieurs lentilles, l'UI applique
  une lentille primaire ; l'encodage cumulé complet reste à raffiner.
- **Groupe multi-sociétés** : aucune `OrganizationUnit` native n'est encore projetée ;
  une société n'est pas un périmètre de carte à ce jour (ADR-024 §5).
- **E2E Playwright non exécuté dans cet environnement** : `npm` est bloqué par la
  politique PowerShell. Le parcours a été rejoué dans le navigateur intégré.
- **Budgets de performance non mesurés** : les fixtures 200/500 formes existent, le
  protocole est décrit, la mesure instrumentée reste à exécuter (VD-20).
- **i18n partielle sur les écrans historiques** : l'Atelier, la Bibliothèque, le Contrôle,
  les erreurs et les intentions passent par `i18n/fr.json` ; les fiches lot 1/2 conservent
  des libellés en dur (VD-25 partiel).
- **Pas de disposition automatique** ni de routage avancé des arêtes.
- **Pas d'IA générative** : aucune clé dans le navigateur (ADR-013).
- **Collaboration temps réel** : commandes idempotentes prêtes, UI non activée (ADR-014).
- **Sécurité simulée** : identités commutables, pas de RBAC.
- Les données sont propres à ce navigateur et à cette origine.
