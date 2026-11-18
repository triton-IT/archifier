# Archifier — la carte produit de l'entreprise

Archifier est un outil d'architecture d'entreprise qui se parcourt comme une carte :
**de la big picture à l'infrastructure**, en passant par les value streams, les produits,
les applications et les interfaces.

| | |
|---|---|
| **Cible produit** | v0.4 — carte de l'entreprise, ontologie produit, zoom sémantique (ADR-023) |
| **Implémentation présente** | MVP3 — Atelier de vues (ADR-022), suite de tests MVP3 |
| **Statut de ce dépôt** | Les spécifications de la cible v0.4 sont écrites ; **la carte produit n'est pas encore codée.** |
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

1. `/` ouvre la **Carte de l'entreprise** ;
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
| Accueil = Atelier, choix d'une intention | Accueil = Carte de l'entreprise, zoom |
| Domaine = pivot de propriété et de navigation | Produit + équipe ; domaine = classification |
| Intention de vue = préalable | Lentille de carte ou action de détail |
| Interface = flèche ou domaine fournisseur | Interface = objet versionné, fournisseur/consommateurs |
| Application = composant ArchiMate | Application = système utilisé par des produits ; composant = partie |
| Une vue = un diagramme | Une carte = projection ; une vue = récit authored |

## La cible v0.4 en un écran

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
- `functional-specifications.md` § « Cible v0.4 — Le produit, la carte et le zoom » —
  ontologie, règles `PRD-*`, carte, lentilles, pages, interfaces, migration, FR-055..081,
  scénarios d'acceptation, trajectoire.
- `docs/ui-spec.md` v2.0 — grammaire de la carte, cinq niveaux, lentilles, pages
  produit/équipe/value stream/capacité/application/infrastructure, états, accessibilité.
- `docs/contracts/C-10-produit-carte-navigation.md` — objets, cardinalités, projection
  `MapQuery → MapProjection`, routes, agrégation, codes `MAP-*`/`PRD-*`, tests nommés.
- `technical-specifications.md` §15.0/§15.2 — avenant : la carte devient `/`, l'Atelier
  reste la surface d'édition.
- `docs/contracts/` — contrats MVP3 gelés (C-01..C-09), conservés ; C-10 est additif.

## État implémenté (MVP3)

Le code présent dans ce dépôt implémente le MVP3, pas encore la cible v0.4.

### L'Atelier (`#/design/:id`)

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

## Démarrage (MVP3)

Prérequis : Rust ≥ 1.88 (toolchain épinglée), cible `wasm32-unknown-unknown`,
`wasm-bindgen-cli` 0.2.130, Node ≥ 20.

```powershell
.\scripts\build.cmd
node scripts\serve.mjs 4173
```

Puis <http://127.0.0.1:4173/>. Le premier lancement ouvre un référentiel **vide** :
l'Atelier propose de nommer le premier domaine, puis de créer la première vue.

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

La cible v0.4 ajoutera un module de **projection de carte** (requête, agrégation, ancres)
distinct du layout authored. Il ne réutilise pas `DiagramLayout` : la carte a sa propre
géométrie stable (C-10 §6 et §10).

## Limites assumées

- **La carte produit n'est pas implémentée** : les specs et contrats de la cible v0.4 sont
  écrits, le code reste MVP3.
- **E2E Playwright non exécuté dans cet environnement** : `npm` est bloqué par la
  politique PowerShell. La chaîne de conception a été rejouée dans le navigateur intégré.
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
