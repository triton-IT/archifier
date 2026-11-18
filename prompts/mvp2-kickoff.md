# Mission : Archifier v0.2 (« MVP 2 ») — application locale, IndexedDB, temps réel

Tu reprends un projet **existant et fonctionnel**. Objectif : livrer le MVP 2 sans
régression, en conservant la persistance **IndexedDB dans le navigateur**, **zéro serveur
applicatif** pour le cœur du produit, et en ajoutant l'**édition collaborative temps réel
qui fonctionne d'abord sans serveur**.

Travaille en français. Ne me demande pas de valider chaque fichier : livre des lots
vérifiés.

---

## 1. Lis d'abord, dans cet ordre, avant d'écrire une ligne

1. `functional-specifications.md` — la source normative du métier. La section
   « Collaboration temps réel (MVP2) » est la plus récente : FR-031 à FR-039,
   CR-001 à CR-007, NFR-015 à NFR-020.
2. `technical-specifications.md` — le modèle, les règles, les métamodèles, l'API cible.
   Le **§21** décrit l'architecture de collaboration ; lis-le avant de concevoir quoi que
   ce soit.
3. `technical-specifications-lot1-front-local.md` — **les écarts assumés** (ADR-011
   snapshot unique, ADR-012 routage par fragment, ADR-013 pas d'IA). Ils sont délibérés et
   motivés : ne les « corrige » pas sans mesure contraire.
4. `backlog-mvp1.md` — l'état réel livré, avec la ventilation livré / partiel / non livré.
5. `README.md` — compilation, lancement, limites.
6. `docs/adr/ADR-014-collaboration-temps-reel.md` — la décision CRDT, déjà prise.
   Applique-la.

Ne réécris pas ces documents. Tu les **complètes** (voir §8).

## 2. État vérifié au 18/11/2026

- Workspace Cargo, 4 crates : `core` (moteur pur, sans I/O, compilé aussi en WASM),
  `metamodel` (profils TOML externes), `repo` (index mémoire + IndexedDB), `ui`
  (Leptos 0.8, rendu client, routage par fragment).
- 30 fichiers Rust, ~9 500 lignes. `cargo test --workspace` → **72 tests verts**.
- Bundle WASM ~1,95 Mo. Aucune dépendance de routage, aucune dépendance d'aléa système.
- **Fonctionne** : domaines, référentiel multi-notation (ArchiMate 3.2 partiel, BPMN Core,
  UML statique), relations natives transverses, interfaces + versions + frise temporelle +
  chevauchements + anomalies, canvas SVG (pan/zoom/palette/déplacement), documentation
  Markdown versionnée + liens `arch://`, stories (chapitres, blocs, transitions), revues à
  onglets + commentaires, recherche lexicale, export/import JSON.
- **Conçu mais non implémenté** : collaboration temps réel (ADR-014, §21). Rien n'existe
  encore dans le code : ni CRDT, ni transport, ni présence.
- **Non livré** : panneau IA, import/export OpenAPI dans l'UI, diff de versions et de
  révisions, graphe de dépendances dédié, diagnostic de liens cassés, multi-onglets.
- Commandes :
  ```powershell
  cargo test --workspace
  cargo check -p archifier-ui --target wasm32-unknown-unknown
  .\scripts\build.ps1
  node scripts\serve.mjs 4173
  ```
- Environnement : Rust 1.94, cible `wasm32-unknown-unknown` installée,
  `wasm-bindgen-cli` 0.2.130, Node 24. Pas de Docker, pas de PostgreSQL — et ce n'est pas
  nécessaire.

**Première action concrète** : exécute `cargo test --workspace` et `.\scripts\build.ps1`
pour confirmer cet état avant de modifier quoi que ce soit. Si un chiffre diverge,
signale-le au lieu de « réparer » silencieusement.

## 3. Contraintes non négociables

1. **Persistance IndexedDB uniquement.** Pas de serveur de données, pas de SQL, pas de
   système de fichiers côté application. Le snapshot reste le format d'échange.
2. **Aucune clé d'API dans le bundle.** Si tu ajoutes un LLM réel, il passe par un proxy
   local explicite (T2-04), désactivé par défaut. Jamais de clé en dur, jamais en
   `localStorage`.
3. **Aucune classe ArchiMate « 4 » inventée.** Le profil 3.2 est la seule base normative
   disponible. La préparation 4.x se fait par infrastructure de mapping.
4. `archifier-core` reste **sans I/O**. `archifier-ui` ne parle qu'à `RepositoryPort` :
   jamais d'accès direct à IndexedDB depuis une vue.
5. UI en **français**, routage par fragment, pas de framework CSS, pas d'icônes normatives
   reproduites.
6. Pas de `unwrap` / `expect` / `panic!` / `todo!` hors tests. Erreurs typées.
7. **Ne casse aucun des 72 tests existants.** Toute modification de comportement existant
   exige un ADR et une mise à jour du backlog.
8. Pas de serveur en tâche de fond persistante : démarre, vérifie, arrête. Si tu dois
   laisser quelque chose tourner, demande-le.
9. **Collaboration à deux plans, jamais mélangés.** Gouvernance (domaines, éléments,
   relations, interfaces, versions, contrats, revues, ChangeSets) = écriture unique,
   verrou optimiste, `409`. Collaboration (documentation, `name`, layout, blocs de Story,
   commentaires, présence) = CRDT. La CRDT ne crée jamais d'objet, ne publie jamais de
   version, ne change jamais un statut de revue. Les deux plans n'écrivent jamais dans la
   même transaction.
10. **La synchronisation fonctionne sans serveur avant de fonctionner avec.**
    `BroadcastChannel` est le mode nominal du MVP2. Tout code qui exige un relais réseau
    pour fonctionner est hors périmètre.

## 4. Périmètre MVP 2 — contrat (41 points, 7 thèmes)

Chaque ligne est un **comportement observable** avec au moins un test. C'est la définition
autoritative du MVP 2 : si une ligne manque, le MVP 2 n'est pas livré.

### T1 — Dette du lot 1 (bloquante, à faire en premier)

- **T1-01** Import OpenAPI 3.0/3.1/3.2 avec aperçu, diff, idempotence par empreinte
  canonique et conservation du document brut.
- **T1-02** Export OpenAPI 3.1, réservé aux interfaces applicatives HTTP, avec
  `x-architecture-*` et reproductibilité octet-à-octet.
- **T1-03** Diff de deux versions d'interface : compatible / rupture / inconnu, rupture
  confirmée explicitement par l'utilisateur.
- **T1-04** Diff de deux révisions d'objet, champ par champ, à partir des snapshots.
- **T1-05** Export Markdown d'un domaine et d'un objet : arborescence, liens relatifs
  valides, SVG + complément Mermaid, reproductible.
- **T1-06** Diagnostic des liens `arch://` cassés ou archivés, avec extrait de contexte et
  accès au propriétaire.
- **T1-07** Graphe de dépendances dédié : direction, profondeur 1 à 6, chemins affichés.
- **T1-08** Multi-onglets : `BroadcastChannel`, détection d'écriture externe, dialogue
  recharger / comparer / forcer, aucune fusion automatique. *(Prépare T7, ne le remplace
  pas : ici on détecte, là on fusionne.)*
- **T1-09** Hiérarchie parent/enfant exposée (couloirs BPMN, ports UML) : profondeur 5,
  cycles interdits, parent archivé signalé.
- **T1-10** Cadrage complet des blocs Story (zoom, focus, atténuation) et épinglage de
  révision pour figer un récit.
- **T1-11** Comparaison de deux revues : filtres, fenêtres temporelles, anomalies.

### T2 — Intelligence artificielle locale

- **T2-01** Panneau conversationnel : streaming token par token, liste d'objets sources
  cliquables, mention explicite « réponse non sourcée » le cas échéant.
- **T2-02** `MockProvider` scripté couvrant cinq recettes (impact, consommateurs,
  proposition de vue, recherche conceptuelle, explication de règle), et un repli honnête
  listant les scénarios supportés. Aucune simulation présentée comme un LLM.
- **T2-03** UI de ChangeSet : diff côte à côte, Valider puis Appliquer distincts,
  détection `STALE` si une révision a bougé, aucune écriture sans confirmation.
- **T2-04** Proxy local **optionnel** `archifier-proxy` (binaire Rust, port 8787) : clé lue
  depuis une variable d'environnement, CORS limité à `127.0.0.1:4173`, aucun prompt
  journalisé par défaut. Désactivé si absent.
- **T2-05** Analyse d'impact **déterministe** (dépendances, temporalité, diagrammes et
  stories affectés) que l'IA ne fait que commenter.
- **T2-06** Journal d'audit des ChangeSets appliqués : acteur, opérations, avant/après.

### T3 — Couverture des notations

- **T3-01** Profil ArchiMate 3.2 étendu (~60 classes : junction, grouping, location,
  plateau, éléments de motivation complets) avec matrice de relations complétée.
- **T3-02** Infrastructure de migration 3.2 → 4.x : fichier de mapping, outil de diagnostic
  de compatibilité, aucune classe 4 déclarée.
- **T3-03** BPMN : compléter le Core Subset annoncé (événements limites, associations de
  données, sous-processus), sans sortir du sous-ensemble documenté.
- **T3-04** UML statique complet (Port, Property, Package, réalisation d'interface) avec
  contraintes testées.
- **T3-05** Palette et inspecteur entièrement pilotés par le profil : ajouter une classe ne
  demande **aucune** modification de code.

### T4 — Gouvernance des interfaces

- **T4-01** Politique de cycle de vie : dépréciation, remplacement, date de retrait,
  politique de rupture, cohérence des fenêtres.
- **T4-02** Contrat événementiel (AsyncAPI 2.x/3.x) en import/export, même modèle de
  conservation du brut que OpenAPI.
- **T4-03** Revues : instantané de filtres complet, réouverture, anomalies étendues (retrait
  proche, rupture non consommée, consommateur orphelin).
- **T4-04** Centre de notifications local : échéances, expirations, ruptures, liens cassés,
  sans workflow d'approbation.

### T5 — Recherche et navigation

- **T5-01** Facettes, synonymes et historique de recherche local.
- **T5-02** « Architecture à la date T » complète : interfaces, documentation, stories et
  diagrammes à l'instant choisi.
- **T5-03** Épingles et fil d'Ariane persistant.

### T6 — Qualité

- **T6-01** Playwright E2E sur le build statique : les 4 chaînes de recette et les 8
  scénarios Gherkin de la spec fonctionnelle, plus les 5 scénarios de collaboration.
- **T6-02** Budgets : 500 nœuds sans blocage perceptible (virtualisation du canvas),
  ouverture de fiche < 100 ms en local, recherche < 50 ms.
- **T6-03** Accessibilité : navigation clavier complète, focus visible, contrastes AA, audit
  automatisé sans erreur critique.
- **T6-04** Chaînes centralisées : aucune chaîne d'interface en dur dans un composant.

### T7 — Collaboration temps réel, sans serveur

- **T7-00** Spike CRDT borné (une journée, bloquant) : Yrs tient 20 000 caractères, 200
  insertions concurrentes sans perte, mise à jour < 2 Ko, décodage < 30 ms, bundle
  < +900 Ko brut. Échec ⇒ bascule sur Automerge, jamais de CRDT maison.
- **T7-01** Deux plans séparés dans le code : `WriteTx` (gouvernance, verrou optimiste) et
  `CollabEdit` (documentation, `name`, layout, blocs de Story, commentaires). Aucun
  chevauchement de chemin d'écriture, aucun commit mixte.
- **T7-02** `SyncTransport` + `LocalBroadcastTransport` : co-édition réelle entre deux
  fenêtres du même navigateur, payloads binaires, rattrapage par vecteur d'état.
- **T7-03** Présence éphémère : qui est connecté, qui édite quel objet, expiration à 30 s,
  jamais persistée, jamais exportée.
- **T7-04** Persistance du document CRDT dans un store `collab`, initialisation idempotente
  depuis le snapshot, compaction au repos.
- **T7-05** Validation après fusion : diagnostic `MERGE_VIOLATION` local, objet marqué « non
  conforme », **aucune** suppression ni correction automatique.
- **T7-06** Historique regroupé : une révision après 2 s d'inactivité, pas une par frappe.
- **T7-07** `WebSocketTransport` derrière le même trait, non activé, testé contre un relais
  en mémoire. Prêt pour le lot 3, invisible pour l'utilisateur.

## 5. Hors périmètre — refus explicite

RBAC/ABAC, SSO, **relais réseau de collaboration** (T7-07 le prépare, ne l'active pas),
workflow d'approbation formel, serveur applicatif de données, BPMN exécutable, diagrammes
UML dynamiques (State Machine, Activity, Sequence), MCP Apps, AG-UI, A2UI, embeddings côté
serveur. Si tu penses qu'une de ces briques est indispensable, tu l'écris dans un ADR et tu
attends l'arbitrage.

## 6. Méthode : orchestration en étoile

Pas de « coopération » entre agents. Un contrat figé, le compilateur, des tests bloquants.
Chaque agent a un périmètre de fichiers **exclusif**.

### Équipe

| Rôle | Périmètre d'écriture |
|---|---|
| Orchestrateur (toi) | contrats, ADR, arbitrages, portes ; seul à modifier les backlogs |
| Analyste fonctionnel | `specs/**`, `fixtures/**`, critères d'acceptation |
| UX / design system | `assets/tokens.css`, `docs/ui-spec.md`, `components/primitives/**` |
| Dev « moteur & données » | `crates/core`, `crates/repo`, `crates/collab`, migrations de snapshot |
| Dev « canvas & vues » | `crates/ui/src/canvas/**`, routes de diagramme |
| Dev « écrans & parcours » | `crates/ui/src/views/**` hors canvas |
| Dev « IA & changesets » | `crates/ai/**`, `crates/proxy/**`, panneau IA |
| Testeur | `tests/e2e/**`, `crates/*/tests/**` — **jamais** de code de production |
| Relecteur | lecture seule ; refuse une porte si une règle de §3 est violée |

**Maximum 4 agents actifs simultanément** (3 devs + 1 testeur). Au-delà, `app.rs`,
`Cargo.toml` et `tokens.css` deviennent des goulots et le coût de fusion dépasse le gain.

### Vagues et portes

```text
W0  Audit + documents + spike CRDT (T7-00)                 → G1 contrats gelés
W1  T1 dette du lot 1                (2 devs en parallèle) → G2
W2  T3 notations                     (1 dev + testeur)     → G3
W3  T7 collaboration — cœur CRDT     (1 dev)               → G4a
W4  T7 collaboration — UI + présence (1 dev)               → G4b
W5  T4 gouvernance + T5 recherche    (2 devs)              → G5
W6  T2 IA locale                     (2 devs)              → G6
W7  T6 qualité                       (testeur + UX)        → G7 MVP2 accepté
```

W3 et W4 sont **séparées volontairement** : la collaboration est le seul chantier qui
touche `repo` et `ui` en même temps. Séparer le moteur CRDT headless (testable sans
interface) du branchement UI réduit le risque de fusion, au prix d'un aller-retour.

Règles de porte : une vague ne démarre pas si la précédente n'est pas verte. Un échec doit
être localisable à un lot, sinon la porte est mal découpée. Après W4, la démonstration
attendue est concrète : **deux fenêtres côte à côte éditent la même documentation et le
même nœud de diagramme, et convergent sans perte.**

## 7. Règles de collaboration

1. Lire la fiche de lot et les contrats gelés **avant** d'écrire.
2. Ne jamais modifier `archifier-core`, `RepositoryPort` ou le format de snapshot pour faire
   passer son propre code : demande d'évolution à l'orchestrateur. Exception explicite : le
   lot T7 ajoute `CollabEdit`, il ne modifie pas `WriteTx`.
3. Ne jamais inventer une API de dépendance. Si la signature n'est pas certaine, vérifie sur
   `docs.rs`, sinon arrête-toi et demande.
4. Un lot = ses tests. Chaque code de règle (`BR-*`, `MM-*`, `CS-*`, `CR-*`, nouveau
   `OAS-*`) a un test nommé d'après lui.
5. Pas de dépendance nouvelle sans accord explicite. Yrs est autorisé par ADR-014 ; le reste
   passe par un ADR.
6. Migrations de snapshot **additives uniquement**, avec numéro consommé définitivement. Le
   store `collab` est nouveau : il ne remplace rien.
7. Un rapport de fin de lot court et factuel : fichiers, tests, règles couvertes, points
   ouverts, commandes exécutées et leur résultat.

## 8. Livrables documentaires

- `docs/backlog-mvp2.md` : les 41 points ci-dessus avec identifiants, traçabilité
  `FR-*`/`NFR-*`, lot et état.
- `technical-specifications-lot2.md` : adaptations du modèle (AsyncAPI, politiques de cycle
  de vie, mapping de métamodèle, proxy, schéma CRDT détaillé), écarts et décisions.
- `docs/adr/` : un ADR par décision structurante nouvelle (au minimum : proxy LLM, AsyncAPI,
  migration de métamodèle, virtualisation du canvas). ADR-014 et ADR-015 existent déjà.
- Mise à jour de `README.md` et création de `docs/metamodel-coverage.md` (le rapport est déjà
  produit par `coverage_report()` dans `archifier-metamodel`).

## 9. Portes de qualité — commandes exactes

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p archifier-ui --target wasm32-unknown-unknown
.\scripts\build.ps1
npx playwright install chromium   # une fois
npx playwright test
```

Budgets bloquants : bundle ≤ 4 Mo gzip (marge CRDT) ; 500 nœuds à 60 fps ; fiche < 100 ms ;
recherche < 50 ms ; latence perçue d'une frappe collaborative < 16 ms ; zéro erreur console
dans un parcours complet.

## 10. Ce que tu ne dois pas faire

- Ajouter un serveur de données « parce que c'est plus simple ».
- Remplacer IndexedDB par un backend au premier problème de quota.
- Faire passer le plan de gouvernance par la CRDT, même « temporairement ».
- Simuler une IA sans le dire, ou afficher « IA » quand c'est un jeu de recettes.
- Marquer un point du backlog comme fait sans test qui le prouve.
- Livrer sans avoir ouvert l'application et cliqué dans les parcours concernés — pour T7,
  cela veut dire **deux fenêtres ouvertes en même temps**.

## 11. Première réponse attendue

1. Confirmation de l'état (tests, build) ou signalement d'un écart.
2. `docs/backlog-mvp2.md` et `technical-specifications-lot2.md` squelettés, avec les
   contrats à figer listés — en particulier le schéma CRDT par champ et la signature de
   `SyncTransport`.
3. Le découpage W0→W7 avec les périmètres de fichiers.
4. Rien d'autre : pas de code avant la porte G1.
