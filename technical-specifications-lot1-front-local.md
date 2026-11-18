# Addendum — Lot 1 « Front autonome, back simulé en IndexedDB »

| | |
|---|---|
| **Document** | Addendum aux spécifications techniques v1.0 |
| **Version** | 1.2 |
| **Date** | 17 novembre 2026 |
| **Statut** | **Validé** — persistance IndexedDB et agent UX/design arbitrés |
| **Remplace** | Les §3, §15, §16.2, §17, §18 de `technical-specifications.md` **pour la durée du lot 1 uniquement** |
| **Ne remplace pas** | §4 à §14 (modèle de domaine, métamodèles, règles métier, temporalité, markdown, OpenAPI, diagrammes) : **intégralement conservés** |

---

## 1. Décision

Le lot 1 livre **l'intégralité du front**, sans serveur, avec une persistance **IndexedDB** dans le navigateur. L'API HTTP, PostgreSQL, les crates `store`/`search`/`ai` serveur et l'authentification réelle sont **différés** au lot 2.

IndexedDB plutôt que `localStorage` : le choix change trois choses en bien. On obtient de **vraies transactions ACID** (donc le chemin d'écriture du §6.3 du document principal est répliqué fidèlement, pas approché), un **quota d'un ordre de grandeur supérieur** (donc l'historique est conservé **intégralement**, NFR-002 n'est plus dégradé), et une **API asynchrone native** (donc l'UI est écrite dès le premier jour contre un dépôt qui ne répond pas instantanément).

Cette décision est bonne, pour une raison qui n'était pas prévue : `archifier-core` est un crate **sans I/O**. Toutes les règles (`BR-001..012`, `MM-*`, `DGM-*`, `DOC-*`, `STORY-*`), la temporalité, les URI `arch://`, le diff de contrat et le validateur de ChangeSet compilent tel quel en WASM. Le lot 1 n'est donc pas une maquette : c'est **le vrai moteur**, avec un adaptateur de stockage différent.

Ce que cela change, et ce que cela ne change pas :

| Élément | Lot 1 |
|---|---|
| `archifier-core` | **inchangé**, compilé en WASM. Aucune règle n'est réécrite en JS |
| `archifier-metamodel` | **inchangé**, profils TOML embarqués |
| `archifier-dto` | **inchangé** |
| Persistance | **IndexedDB** derrière un port `RepositoryPort` (+ `StorageEngine`) |
| Historique | **complet**, aucune troncature (NFR-002 respecté) |
| `archifier-store` (SQL) | **non construit** |
| `archifier-api`, `archifier-server` | **non construits** |
| Auth | simulée par un sélecteur d'utilisateur |
| IA | `MockProvider` déterministe, 100 % client |
| Recherche | index inversé en mémoire (pas de `tsvector`) |
| Recherche sémantique | **absente** (pas d'embeddings sans serveur) |
| Export/import de snapshot | **ajouté** (remplace la sauvegarde base) |

---

## 2. Le point de couture : `RepositoryPort`

Toute l'UI passe par un trait unique. Rien d'autre ne touche au stockage.

> **Amendement MVP2 (ADR-016, 18/11/2026).** Le trait `RepositoryPort` n'a pas été
> extrait au lot 1. Pour le MVP2, le port effectif est le type concret `Repository`,
> centralisé par `crates/ui/src/state.rs::repo()` ; aucune vue n'accède au stockage et
> `boot.rs` reste la seule racine de composition. Voir
> `docs/adr/ADR-016-port-depot-type-repository.md`.

```rust
// crates/repo/src/lib.rs
#[cfg(target_arch = "wasm32")]
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
#[cfg(not(target_arch = "wasm32"))]
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait RepositoryPort {
    // lectures
    fn get_object<'a>(&'a self, id: &'a ObjectId) -> BoxFut<'a, Result<RepositoryObject, RepoError>>;
    fn query<'a>(&'a self, q: &'a Query) -> BoxFut<'a, Result<Page<ObjectSummary>, RepoError>>;
    fn history<'a>(&'a self, id: &'a ObjectId) -> BoxFut<'a, Result<Vec<RevisionMeta>, RepoError>>;
    fn revision<'a>(&'a self, id: &'a ObjectId, n: Revision) -> BoxFut<'a, Result<Snapshot, RepoError>>;
    fn dependencies<'a>(&'a self, req: &'a DependencyRequest) -> BoxFut<'a, Result<Graph, RepoError>>;
    fn search<'a>(&'a self, q: &'a SearchQuery) -> BoxFut<'a, Result<SearchPage, RepoError>>;
    fn interfaces_at<'a>(&'a self, at: OffsetDateTime, f: &'a InterfaceFilter) -> BoxFut<'a, Result<Vec<InterfaceView>, RepoError>>;
    fn domain_overview<'a>(&'a self, id: &'a DomainId, at: OffsetDateTime) -> BoxFut<'a, Result<DomainOverview, RepoError>>;
    fn review_view<'a>(&'a self, id: &'a ObjectId, view: ReviewViewKind, at: OffsetDateTime) -> BoxFut<'a, Result<ReviewView, RepoError>>;
    fn story_play<'a>(&'a self, id: &'a ObjectId) -> BoxFut<'a, Result<Playback, RepoError>>;

    // écriture — unique point d'entrée, réplique l'étape §6.3 du doc principal
    fn write<'a>(&'a self, tx: WriteTx) -> BoxFut<'a, Result<WriteOutcome, RepoError>>;
    fn apply_changeset<'a>(&'a self, cs: &'a ChangeSet, confirmed: bool) -> BoxFut<'a, Result<Applied, RepoError>>;
}
```

Deux implémentations prévues :

| Implémentation | Statut | Crate |
|---|---|---|
| `LocalRepository` (index en mémoire + IndexedDB) | **lot 1** | `archifier-repo-local` |
| `HttpRepository` (les endpoints du §9) | lot 2 | `archifier-repo-http` |

**Règle initiale (lot 1) :** `archifier-ui` ne dépend **que** de `RepositoryPort`, jamais d'une implémentation concrète. Le câblage se fait au `main` du bundle, en une ligne. Un test d'architecture échoue si un composant importe `archifier_repo_local`.

> **Amendement MVP2 (ADR-016, 18/11/2026).** Le trait n'a pas été extrait. Le port effectif
> est le type `Repository`, via `state::repo()` ; aucune vue n'accède au stockage. Les tests
> d'architecture `ui_views_do_not_touch_storage` et `idb_store_is_only_known_by_boot`
> remplacent la règle « trait unique » pour le MVP2.

Conséquence directe : les signatures de `Query`, `WriteTx`, `ChangeSet`, `DomainOverview` sont **gelées dès F0**. C'est le contrat qui rend le lot 2 mécanique au lieu d'être une réécriture. Pour le MVP2, `WriteTx` reste une cible de nommage ; le chemin réel est `Repository::mutate`.

---

## 3. Persistance

### 3.1 Port de stockage

```rust
pub trait StorageEngine {
    fn get<'a>(&'a self, store: Store, key: &'a str) -> BoxFut<'a, Result<Option<Value>>>;
    fn put<'a>(&'a self, store: Store, key: &'a str, record: &'a Value) -> BoxFut<'a, Result<()>>;
    fn delete<'a>(&'a self, store: Store, key: &'a str) -> BoxFut<'a, Result<()>>;
    fn scan<'a>(&'a self, store: Store) -> BoxFut<'a, Result<Vec<Value>>>;
    fn query_index<'a>(&'a self, store: Store, index: &'a str, range: KeyRange)
        -> BoxFut<'a, Result<Vec<Value>>>;

    /// Transaction atomique sur les stores listés : tout ou rien.
    fn transaction<'a>(&'a self, scope: &'a [Store])
        -> BoxFut<'a, Result<Box<dyn StorageTxn + 'a>>>;

    fn estimate(&self) -> BoxFut<'_, Result<StorageEstimate>>;
    fn request_persistence(&self) -> BoxFut<'_, Result<bool>>;
}

pub trait StorageTxn {
    fn get<'a>(&'a self, store: Store, key: &'a str) -> BoxFut<'a, Result<Option<Value>>>;
    fn put<'a>(&'a self, store: Store, key: &'a str, record: &'a Value) -> BoxFut<'a, Result<()>>;
    fn delete<'a>(&'a self, store: Store, key: &'a str) -> BoxFut<'a, Result<()>>;
    /// Valide la transaction. Sans appel à `commit`, toutes les opérations sont annulées.
    fn commit(self: Box<Self>) -> BoxFut<'static, Result<()>>;
}
```

| Implémentation | Usage |
|---|---|
| `IdbEngine` (crate `idb`, repli `rexie`) | application |
| `MemoryEngine` (BTreeMap + transaction par copie-à-l'écriture) | tests natifs, rapides |
| `FailingEngine` (quota, `AbortError`, transaction avortée à mi-parcours) | tests d'erreur et de rollback |

**Unique subtilité de l'adaptateur** : une transaction IndexedDB se referme d'elle-même dès que la file de requêtes se vide. L'implémentation doit donc conserver le handle de transaction ouvert entre deux opérations, sinon la deuxième écriture échoue avec `TransactionInactiveError`. C'est le seul point où `IdbEngine` n'est pas un simple passe-plat, et il doit être couvert par un test dédié (`idb_transaction_stays_open`).

L'API exacte du crate retenu DOIT être vérifiée sur `docs.rs` au début de F0 ; en cas d'incompatibilité, `rexie` est le repli documenté.

### 3.2 Stores et index

Une base IndexedDB nommée `archifier`, versionnée (`db.version`), contenant **un object store par table logique** du schéma SQL (§7 du document principal), plus un store technique.

| Store | Clé | Index |
|---|---|---|
| `objects` | `id` | `by_type`, `by_domain`, `by_name` (`name_lower`), `by_lifecycle` |
| `object_revisions` | `id` | `by_object` (`object_id`), `by_object_rev` (`object_id`, `revision_no`) |
| `domains` | `object_id` | `by_slug` (unique), `by_name` (`name_lower`, unique partiel logique) |
| `model_elements` | `object_id` | `by_class` (`notation`, `metaclass`), `by_parent` |
| `relationships` | `object_id` | `by_source`, `by_target`, `by_type` |
| `interfaces` | `object_id` | `by_key` (`logical_interface_key`, **unique**), `by_provider` |
| `interface_versions` | `id` | `by_interface`, `by_available_from`, `by_status` |
| `interface_consumers` | `id` | `by_version`, `by_consumer_object`, `by_consumer_domain` |
| `diagrams` | `object_id` | `by_notation` |
| `diagram_usages` | `[diagram_id, object_id, usage_kind]` | `by_object`, `by_diagram` |
| `stories` | `object_id` | — |
| `reviews` | `object_id` | `by_domain`, `by_status` |
| `documentation_blocks` | `id` | `by_owner`, `by_owner_rev` (`owner_object_id`, `revision`) |
| `links` | `id` | `by_source`, `by_target`, `by_status` |
| `comments` | `id` | `by_target`, `by_author` |
| `changesets` | `id` | `by_status`, `by_origin` |
| `openapi_documents` | `id` | `by_fingerprint` (unique), `by_interface` |
| `audit_events` | `id` | `by_object`, `by_actor`, `by_created` |
| `users` | `id` | `by_email` (`email_lower`, unique) |
| `meta` | `key` | — (store technique : `schema`, `write_seq`, `app_version`, `last_good_seq`) |

Deux contraintes de plateforme qui ont un effet sur le modèle :

- **IndexedDB n'accepte pas les booléens comme clé d'index.** Toutes les colonnes indexées sont donc du texte (`lifecycle`, `status`, `notation`, `object_type`) — ce qui est déjà le cas du schéma SQL. Aucune colonne booléenne ne doit être indexée.
- Une contrainte d'unicité métier (`logical_interface_key`, email, slug) est portée par un **index unique** IndexedDB, pas seulement par la validation applicative. C'est la seule façon d'obtenir la garantie qu'offrait `CREATE UNIQUE INDEX` côté SQL.

### 3.3 Forme des enregistrements et format d'échange

Chaque enregistrement est **plat pour ce qui doit être indexé**, et porte l'agrégat complet sérialisé dans un champ `payload` :

```json
{
  "id": "01JABC...",
  "object_type": "INTERFACE",
  "name": "Customer API",
  "name_lower": "customer api",
  "responsible_domain_id": "01JPAY...",
  "lifecycle": "ACTIVE",
  "revision": 7,
  "created_at": "2026-09-15T08:12:00Z",
  "updated_at": "2026-11-02T16:40:00Z",
  "payload": "{\"object\":{...},\"versions\":[...],\"consumers\":[...]}"
}
```

- Les champs plats ne sont dupliqués hors de `payload` **que** s'ils doivent servir d'index. Toute autre donnée vit uniquement dans `payload`, pour éviter deux sources de vérité.
- `payload` contient exactement ce que `object_revisions.payload` contient côté SQL. L'export et le futur import PostgreSQL restent donc un mapping direct, sans transformation.

**Format d'échange** — le fichier `.archifier.json` (sauvegarde, fixtures de test, transfert vers le lot 2) reste un snapshot unique au format du schéma SQL, produit par un balayage de tous les stores :

```json
{
  "x-archifier-snapshot": 1,
  "schema": 1,
  "write_seq": 42,
  "written_at": "2026-11-17T10:00:00Z",
  "written_by": "01USR...",
  "app_version": "0.1.0",
  "data": {
    "users": [], "objects": [], "object_revisions": [], "domains": [],
    "model_elements": [], "relationships": [], "interfaces": [],
    "interface_versions": [], "interface_consumers": [], "diagrams": [],
    "diagram_usages": [], "stories": [], "reviews": [],
    "documentation_blocks": [], "links": [], "comments": [],
    "changesets": [], "openapi_documents": [], "audit_events": []
  }
}
```

Ce fichier est aussi le **format des fixtures Playwright** : chaque scénario est joué sur un snapshot injecté, jamais sur un état construit par l'UI. C'est ce qui rend les tests déterministes et rejouables au lot 2.

### 3.4 Protocole d'écriture

Une écriture = **une transaction IndexedDB**, qui réplique littéralement le chemin d'écriture du §6.3 du document principal :

```text
txn(scope = [objects, object_revisions, <table détail>, audit_events,
             links, diagram_usages, meta])            mode readwrite
 1. get(objects, id)                       vérifier la révision attendue (If-Match)
 2. archifier_core::validate_*             validation pure, aucune I/O
 3. put(<table détail>, état courant)      ex. model_elements, interfaces, diagrams
 4. put(objects, name, revision + 1, updated_at, updated_by)
 5. put(object_revisions, snapshot immuable)
 6. put(audit_events, opération, before_revision, after_revision)
 7. put(links, liens arch:// recalculés)
    put(diagram_usages, delta)
 8. put(meta, write_seq + 1, last_write_at)
 9. commit
```

- **Atomicité réelle** : si une opération échoue (`ConstraintError`, `QuotaExceededError`, abort), IndexedDB annule **toutes** les écritures de la transaction. C'est le gain décisif sur `localStorage`, où une écriture partielle était possible et devait être compensée par une copie de sauvegarde.
- `durability: "strict"` est demandé quand le navigateur le supporte, pour garantir le flush disque avant `commit`.
- `QuotaExceededError` ⇒ pas de reprise automatique, remontée `QUOTA-FULL` avec proposition d'export.
- `ConstraintError` sur `interfaces.by_key` ⇒ `422` `IFACE-DUP-001` (clé logique déjà utilisée).
- Révision divergente ⇒ abandon explicite de la transaction + `409` `OBJ-CONFLICT`, sans aucune écriture.
- **Debounce** : inutile sur les écritures unitaires. Il reste appliqué au seul déplacement de nœuds dans un diagramme (une écriture au `drop`, pas par frame).
- **Multi-onglets** : les transactions en écriture sont sérialisées par le navigateur, donc pas de corruption. Un `BroadcastChannel("archifier:changes")` diffuse `{ write_seq, object_ids }` ; les autres onglets invalident leur index mémoire et affichent `EXTERNAL_WRITE_DETECTED` (« un autre onglet a modifié les données, recharger »). Aucune fusion automatique — même philosophie que le `409` du document principal.

### 3.5 Quota, persistance et éviction

L'IndexedDB d'une origine dispose d'un quota de l'ordre de plusieurs centaines de mégaoctets (variable selon le navigateur et l'espace disque libre), sans commune mesure avec les 5–10 Mo de `localStorage`.

Trois conséquences, toutes favorables :

1. **L'historique est complet.** Aucune troncature de révisions : `LOCAL-002` disparaît, et `NFR-002` (aucun historique altéré silencieusement) n'est plus dégradé par le profil local. C'est un gain de conformité, pas seulement de capacité.
2. **Le bandeau « historique tronqué » est supprimé.** Il ne reste que l'avertissement de portée : « Démonstrateur local — les données restent dans ce navigateur. »
3. **Le garde-fou devient une simple surveillance.** `navigator.storage.estimate()` fournit `{ usage, quota }`, affiché dans `/settings` ; alerte non bloquante à 80 %, blocage explicite avec proposition d'export au-delà.

Point de vigilance réel — l'**éviction** :

- Au premier démarrage, `navigator.storage.persist()` est demandé. L'état (`granted` / `denied`) est affiché dans `/settings`, car un navigateur sous pression disque peut sinon supprimer la base sans préavis.
- Un marqueur minimal est conservé en `localStorage` (`archifier:last_seen_seq`). Si la base s'ouvre vide alors que le marqueur annonce des données, l'application affiche `STORAGE-EVICTED` et propose d'importer un snapshot — plutôt que de laisser croire à une perte silencieuse.
- L'export `.archifier.json` reste la seule sauvegarde réelle. Il est proposé, jamais imposé.

### 3.6 Budget de stockage

| Contenu | Taille unitaire | Volume de démo | Total |
|---|---|---|---|
| Objet + révision courante | ~0,8 Ko | 400 | ~320 Ko |
| Révisions supplémentaires (×4) | ~1,5 Ko | 1 600 | ~2,4 Mo |
| Diagramme (60 nœuds) | ~8 Ko | 12 | ~96 Ko |
| Documentation (2 Ko/moyen) | 2 Ko | 250 | ~500 Ko |
| Versions d'interface + contrats | ~4 Ko | 30 | ~120 Ko |

Total attendu : **~3,4 Mo** pour un jeu de démonstration crédible — soit moins de 2 % d'un quota IndexedDB courant. La marge autorise l'historique complet sans politique de purge.

Le test de charge de F7 porte donc sur un jeu volontairement agressif : **5 000 objets, 20 000 révisions, 50 diagrammes** (~50 Mo), avec mesure du temps d'hydratation de l'index mémoire au démarrage. Si ce test passe, la limite pratique du profil local est atteinte par le volume de données, pas par le stockage.

---

## 4. Index en mémoire (remplacement de la couche SQL)

Au démarrage, `archifier-repo-local` **hydrate** un index depuis IndexedDB (balayage des stores `objects`, `relationships`, `interface_versions`, `interface_consumers`, `diagram_usages`), puis le met à jour de façon incrémentale à chaque `write`. Aucun accès de lecture ne fait de parcours linéaire.

Une base ne se justifie pas pour les jointures : IndexedDB n'en fait pas, et un parcours complet de 5 000 objets coûte quelques dizaines de millisecondes. L'index mémoire est donc le moteur de requête, IndexedDB n'étant que le journal durable. Coût d'hydratation mesuré au test de charge F7 ; au-delà de 300 ms sur le jeu cible, l'hydratation devient paresseuse par type d'objet, sans changer l'API.

```rust
pub struct MemoryIndex {
    objects:             HashMap<ObjectId, RepositoryObject>,
    by_type:             BTreeMap<ObjectType, BTreeSet<ObjectId>>,
    by_domain:           BTreeMap<DomainId, BTreeSet<ObjectId>>,
    rel_out:             HashMap<ObjectId, Vec<ObjectId>>,   // ids de relationships
    rel_in:              HashMap<ObjectId, Vec<ObjectId>>,
    interface_by_key:    HashMap<String, ObjectId>,
    versions_by_iface:   BTreeMap<ObjectId, Vec<ObjectId>>,
    consumers_by_version: HashMap<ObjectId, BTreeSet<ObjectId>>,
    diagram_usages:      HashMap<ObjectId, BTreeSet<ObjectId>>,
    doc_tokens:          HashMap<String, BTreeSet<ObjectId>>, // index inversé
    revisions:           HashMap<ObjectId, Vec<RevisionMeta>>,
}
```

**Ordre déterministe obligatoire** : toute collection exposée à l'UI est triée par ULID croissant (donc approximativement par date de création). Jamais d'itération sur `HashMap` dans un rendu ou dans un export — sinon les tests Playwright deviennent instables et l'export Markdown cesse d'être reproductible (NFR-009).

### 4.1 Recherche lexicale côté client

| Élément | Implémentation | Écart avec le §10.4 |
|---|---|---|
| Normalisation | minuscules + suppression d'accents (table de translittération intégrée) | identique en intention |
| Racinisation | `rust-stemmers` (Snowball français) | remplace `to_tsvector('french')` |
| Pondération | titre ×3, noms de voisins ×1,5, documentation ×1 | approximation de `setweight(A/B)` |
| Fautes de frappe | distance de Levenshtein ≤ 2 sur les noms courts | remplace `pg_trgm` |
| Sémantique | **absent**, `mode` toujours `"lexical"` | `mode` ne ment pas : l'UI n'affiche jamais « sémantique » |

**Dégradation annoncée, jamais silencieuse.** `GET /api/meta/features` n'existe pas en local ; c'est un `FeatureFlags` en mémoire qui renvoie `semantic_search: false`, `ai_mode: "mock"`, `storage: "local"`. L'UI l'affiche dans `/settings`.

---

## 5. IA en mode local

Trois contraintes absolues :

1. **Aucune clé d'API dans le navigateur.** Jamais. Une clé serait visible par quiconque ouvre les DevTools. Le mode local n'appelle aucun fournisseur externe.
2. **Le mode simulé n'est pas un mensonge.** L'UI affiche « IA simulée (jeu de scénarios local) » et jamais « IA » tout court.
3. **Le comportement observable est identique au futur mode réel** : mêmes outils, même flux SSE-like, mêmes événements, même ChangeSet, même double confirmation.

`MockProvider` ne fait pas semblant de raisonner : il applique des **recettes déterministes** sur les outils réels.

| Question reconnue | Recette |
|---|---|
| « qu'est-ce qui casse si <interface> disparaît à <date> » | `resolve_interface_by_name` → `impact_analysis` → synthèse templatée avec liste d'IDs |
| « quels systèmes consomment <interface> » | `list_interfaces_at` → `traverse_graph(direction=in)` → liste sourcée |
| « propose-moi une vue de cet impact » | `impact_analysis` → `propose_changes` (CreateDiagram) → ChangeSet `DRAFT` |
| « où parle-t-on de <concept> » | `search_text` → résultats avec extraits |
| toute autre question | réponse explicite : « le mode local ne couvre que N scénarios », avec la liste |

Les tool calls sont **réellement exécutés** contre le dépôt en mémoire. C'est ce qui rend la chaîne de recette n°4 vérifiable : les IDs cités sont vrais, et le ChangeSet proposé serait applicable.

Le délai de streaming est simulé (25–60 ms par token, déterministe par graine) pour que l'UI soit développée contre un flux asynchrone réel dès le premier jour. Sans cela, on découvrirait au lot 2 que l'interface ne supporte pas le streaming.

---

## 6. Latence et erreurs simulées (non négociable)

Le piège classique : une UI écrite contre un dépôt synchrone en mémoire casse le jour où elle rencontre un réseau. Parade, activée **par défaut en développement** :

```toml
[dev]
fake_latency_ms = 40           # en plus de l'asynchronisme réel d'IndexedDB
fake_latency_jitter_ms = 30
fake_conflict_rate = 0.02      # injecte des OBJ-CONFLICT aléatoires
fake_failure_rate = 0.005      # injecte des erreurs transitoires
seed = 42                      # tout est reproductible
```

IndexedDB étant déjà asynchrone, la latence simulée n'est plus nécessaire à la **correction** de l'UI — elle reste utile pour **éprouver** les états de chargement et de conflit, qui sinon ne seraient jamais visibles en local.

- En `fake_latency_ms = 0` uniquement pendant les tests Playwright de performance.
- Les états `loading`, `empty`, `error`, `conflict` sont donc **tous** conçus dès F0, et non rafistolés plus tard.
- Un test `ui_resilience` vérifie que chaque écran affiche un état d'erreur exploitable et un bouton « réessayer ».

---

## 7. Plan de lots révisé (front uniquement)

```text
F0   Socle technique : workspace, core, metamodel, dto, repo-local, StorageEngine
     IndexedDB, migrations de schéma, i18n, shell vide
       ⇒ PORTE F0 : `cargo test -p archifier-core` vert, base créée et hydratée,
         transaction d'écriture rejouée avec rollback vérifié, app vide qui démarre

F0.5 Design system : tokens, primitives (Button, Field, Badge, Dialog, Table,
     EmptyState, ErrorState, Banner), grammaire visuelle de l'Atelier, catalogue
     d'intentions de vue, spécification des gestes et matrice d'états
       ⇒ PORTE F0.5 : primitives gelées, page de démonstration des composants,
         maquette de l'Atelier validée (accueil = designer, pas inventaire)

F1   Atelier de vues : palette, création sans modal, relations filtrées, inspecteur,
     zones, bandes, ports, undo/autosave, pan/zoom
F2   Fiches objet : rendu, propriétés, documentation Markdown, historique, diff
F3   VISUALISER : accueil domaine, interfaces, timeline + chevauchements, graphe, date T
F4   Revues (9 vues, anomalies, commentaires) + Stories (éditeur + lecteur, framing)
F5   Recherche globale + panneau IA simulée + diff de ChangeSet + validation
F6   Import/export OpenAPI + export Markdown/Mermaid + import/export de snapshot
F7   Recette : 4 chaînes + 8 scénarios Gherkin + performance + quota
       ⇒ PORTE F7 : MVP1 front accepté, prêt pour le lot 2 (branchement HTTP)
```

Dépendances : `F0 → F0.5 → {F1, F2, F3} → {F4, F5} → F6 → F7`. Les lots F1/F2/F3 partagent le shell mais écrivent dans des fichiers disjoints et consomment les primitives **gelées** de F0.5.

---

## 8. Équipe recommandée

### 8.1 Composition

| Rôle | Nombre | Périmètre d'écriture | Quand |
|---|---|---|---|
| **Orchestrateur / architecte** *(moi)* | 1 | `docs/`, contrats, arbitrages, portes | en continu |
| **Analyste fonctionnel** | 1 | `specs/**`, `fixtures/**`, `i18n/fr.json` | en amont de chaque vague |
| **Développeur « moteur & données »** | 1 | `crates/repo-local`, `crates/core` (lecture seule), recherche, import/export | F0–F6 |
| **Développeur « canvas & diagrammes »** | 1 | `crates/ui/src/canvas/**` | F1, F4 |
| **Développeur « écrans & parcours »** | 1 | `crates/ui/src/routes/**`, `components/**` | F2, F3, F4 |
| **Développeur « IA & changeset UI »** | 1 | `crates/ui/src/ai/**`, `crates/repo-local/mock_ai.rs` | F5 |
| **Testeur** | 1 | `tests/e2e/**`, `crates/*/tests/**` | en continu, à partir de F0 |
| **Relecteur / gardien d'architecture** | 1 | *aucun* (lecture seule) | à chaque porte |
| **UX / design system** | 1 | `assets/tokens.css`, `docs/ui-spec.md`, `crates/ui/src/components/primitives/**` | **F0.5**, puis revue aux portes |

Pic de parallélisme : **4 agents actifs simultanément** (3 devs + 1 testeur), plus l'UX en F0.5, l'analyste en amont et le relecteur aux portes.

### 8.2 Pourquoi pas plus d'agents

Les fichiers structurants deviennent des goulots : `Cargo.toml`, `crates/ui/src/app.rs`, `assets/tokens.css`, `fixtures/*.json`. Au-delà de 3 développeurs simultanés, le coût de réconciliation dépasse le gain de parallélisme, et un conflit de fusion sur `app.rs` coûte plus cher qu'un lot entier. J'ai dimensionné le découpage pour que trois agents n'écrivent jamais dans le même fichier dans la même vague.

### 8.3 Orchestration, pas « coopération »

Recommandation ferme : **architecture en étoile**, pas de dialogue entre agents.

Un subagent démarre avec un contexte vierge et ne partage aucune mémoire avec ses pairs. Si on leur demande de « coopérer », ils coopèrent par le texte, et deux agents qui négocient une interface produisent une troisième version, implicite, que personne ne teste. Ce qui coordonne réellement du code, c'est : un contrat figé, le compilateur, et des tests bloquants.

Le modèle qui fonctionne est celui d'un chantier : un chef d'orchestre qui détient le contexte global, des lots à périmètre exclusif, une porte de vérification entre chaque vague. Les agents ne se parlent pas ; ils parlent au contrat.

Bénéfice mesurable : un échec est **localisable** (un lot, une porte), alors qu'en coopération un échec est diffus et attribuable à personne.

### 8.4 Le rôle d'analyste mérite d'être reformulé

Ta proposition d'analyste fonctionnel est juste, mais son livrable n'est pas un document — la spec fonctionnelle et le doc technique existent déjà. Son livrable utile est **exécutable** :

- les critères d'acceptation par lot, en Gherkin, dérivés des 30 `FR-*` et des 12 `BR-*` ;
- les **jeux de données de référence** (`fixtures/*.json`, au format snapshot) qui incarnent chaque scénario : le domaine Paiements avec v1/v2 en migration, la chaîne multi-notation, la story à trois diagrammes ;
- les textes d'interface en français (libellés, messages d'erreur, états vides), centralisés dans `i18n/fr.json`.

Ces fixtures sont la pièce la plus rentable du dispositif : elles rendent chaque test Playwright déterministe et rejouable au lot 2 sans modification.

### 8.5 Mode « évaluation » du testeur

Le testeur ne doit pas écrire de code de production et ne doit pas tester ce que le développeur a décidé de tester. Sa règle : **il dérive ses tests des critères d'acceptation de la spec, jamais du code livré**. S'il ne comprend pas un comportement en lisant la spec, c'est un défaut de spec à remonter, pas un détail à deviner.

### 8.6 Livrables de l'agent UX/design

L'agent UX/design intervient **une fois**, en F0.5, puis repasse en revue aux portes. Ses livrables sont gelés à la sortie de F0.5, comme les contrats de données :

| Livrable | Fichier | Contenu attendu |
|---|---|---|
| Tokens | `assets/tokens.css` | couleurs (dont les 3 statuts temporels), typographie, espacements, rayons, élévations, durées, densité |
| Primitives | `crates/ui/src/components/primitives/**` | `Button`, `Field`, `Badge`, `Dialog`, `Table`, `Tabs`, `Banner`, `EmptyState`, `ErrorState`, `Skeleton`, `Toast` |
| Spécification d'interaction | `docs/ui-spec.md` | hiérarchie visuelle, comportement au survol/focus/actif, raccourcis clavier, densité des listes, règles de troncature |
| Grammaire visuelle de l’Atelier | `docs/ui-spec.md` | bandes de couches, zones de domaine, frontières et ports, encodage temporel, zoom sémantique, hiérarchie typographique |
| Catalogue d’intentions de vue | `docs/ui-spec.md`, `assets/view-templates/*.json` | les 7 intentions, leur palette, leurs bandes, leur état vide et leur exemple |
| Spécification des gestes | `docs/ui-spec.md` | déposer, relier, renommer, formaliser, cadrer, publier ; équivalents clavier et refus attendus |
| Matrice d'états | `docs/ui-spec.md` | pour chaque écran : `loading`, `empty`, `error`, `conflict`, `partial`, avec le libellé exact et l'action de reprise |
| Accessibilité | `docs/ui-spec.md` | focus visible, ordre de tabulation, contrastes minimaux, `prefers-reduced-motion`, navigation clavier du canvas |
| Page de démonstration | route `/_design` | tous les primitives et tous les états, en une page, pour revue visuelle |

Ce que l’UX fait, et qui est désormais central : il définit **l’Atelier comme point d’entrée**, la grammaire spatiale des vues, le catalogue d’intentions, les gestes de création et leurs refus. Il ne code pas les écrans, mais il arbitre ce qui distingue un outil d’architecture d’une CMDB : le spatial avant le tabulaire, le geste avant le formulaire, la vue avant l’inventaire.

Ce que l’UX ne fait pas, et qui reste volontaire : aucune logique métier, aucune donnée, aucun accès au dépôt, aucun composant métier. Il fournit le vocabulaire visuel et les règles d’interaction, pas l’implémentation.

**Règle de gel :** après F0.5, les développeurs *consomment* les primitives. Une évolution de primitive passe par une demande à l'UX et une nouvelle version, jamais par une édition directe dans le fichier d'un autre lot. Sans cette règle, chaque écran dérive et l'application finit avec quatre styles de boutons.

**Arbitrage esthétique par défaut**, si tu ne fournis pas d'identité de marque : interface dense, sobre, sans imagerie, typographie système, glyphes ArchiMate/BPMN/UML réduits à des formes géométriques étiquetées (aucun jeu d'icônes normatif n'est reproduit, faute de droits). C'est un choix de démonstrateur assumé, pas une proposition de charte.

---

## 9. Ce qui reste gelé dès F0

Sans ces contrats, le parallélisme est impossible. Ils sont signés à la porte F0 et ne bougent plus sans un ADR.

1. `RepositoryPort` et l'ensemble des types qu'il manipule (`Query`, `WriteTx`, `Page`, `Graph`, `DomainOverview`, `ReviewView`, `Playback`, `SearchPage`).
2. Le schéma IndexedDB : liste des stores, clés, index et leur caractère unique (§3.2).
3. Le format d'échange `.archifier.json` (§3.3), qui sert aussi de format de fixtures.
4. La liste des codes d'erreur (`BR-*`, `MM-*`, `DGM-*`, `DOC-*`, `STORY-*`, `CS-*`, plus les nouveaux `IDB-001..003`, `IFACE-DUP-001`, `QUOTA-080`, `QUOTA-FULL`, `EXTERNAL_WRITE_DETECTED`, `STORAGE-EVICTED`).
5. Les signatures de `archifier-core` : `validate_*`, `temporal_status`, `contract_diff`, `ArchUri`, `ChangeSet`.
6. La surface de `FeatureFlags`.
7. Les primitives UI et les tokens (§8.6), à la sortie de F0.5.

---

## 10. Risques du lot 1

| Risque | Probabilité | Impact | Parade |
|---|---|---|---|
| Éviction des données par le navigateur sous pression disque | faible | haut | `storage.persist()` demandé, état affiché, marqueur de détection, export snapshot |
| Adaptateur IndexedDB mal maîtrisé (transaction qui se referme, migrations de schéma) | moyenne | moyen | test dédié `idb_transaction_stays_open`, migrations versionnées testées, `MemoryEngine` pour les tests métier |
| Hydratation de l'index trop lente sur gros volume | faible | moyen | mesure F7, hydratation paresseuse par type au-delà de 300 ms |
| UI développée contre un dépôt sans latence, cassée au lot 2 | faible | moyen | asynchronisme natif d'IndexedDB + latence simulée par défaut (§6) |
| Dérive visuelle entre écrans | moyenne | moyen | primitives gelées en F0.5, revue UX à chaque porte, page `/_design` |
| Agents qui « inventent » une API de dépendance | moyenne | moyen | règle explicite : vérifier sur `docs.rs`, sinon s'arrêter et demander |
| Le mode local est pris pour le produit | moyenne | moyen | bandeaux permanents « démonstrateur local », `FeatureFlags` affichés |

---

## 11. Ce que le lot 1 ne prouve pas

Pour rester honnête sur la valeur de la démonstration :

- Il ne prouve **rien** sur les performances réelles client/serveur (NFR-005/006) : celles-ci dépendront de PostgreSQL et du réseau. Seul le NFR-007 (canvas) est réellement mesurable en local.
- Il ne prouve rien sur la concurrence multi-utilisateurs ni sur l'authentification.
- Il ne prouve rien sur la pertinence d'un vrai LLM : le mode simulé démontre le **protocole** (proposition → validation → confirmation), pas la qualité des réponses.
- Il prouve, en revanche, et c'est l'essentiel : que le modèle de domaine tient debout, que les règles ArchiMate/BPMN/UML sont implémentables dans le sous-ensemble annoncé, et que la temporalité des interfaces se lit correctement.

---

## 12. Décisions : état

### Tranchées

| # | Décision | Valeur retenue |
|---|---|---|
| 1 | Profil « local » : front seul, IA simulée | oui |
| 2 | Persistance | **IndexedDB** (transactions ACID, quota large) |
| 3 | Rétention de l'historique | **aucune troncature**, NFR-002 respecté |
| 4 | Agent UX/design | **oui, dédié**, en F0.5 puis en revue aux portes |
| 5 | Relecteur d'architecture distinct du testeur | oui, en lecture seule, aux portes |

### Encore ouvertes, non bloquantes pour F0

| # | Question | Proposition par défaut |
|---|---|---|
| A | Identité de marque (couleurs, typographie, logo) | thème sobre et dense, arbitré par l'UX, révisable après F7 |
| B | Jeu de glyphes ArchiMate/BPMN/UML | formes géométriques étiquetées, aucune reproduction de jeu normatif |
| C | Fournisseur LLM réel | hors lot 1 ; `MockProvider` jusqu'au lot 2 |

---

## 13. Écarts constatés à l'implémentation du lot 1

Ces trois écarts sont assumés, mesurés et documentés. Ils ne changent ni le contrat
`RepositoryPort`, ni le format d'échange `.archifier.json`, donc le lot 2 n'est pas
impacté.

| ID | Spécifié au § | Implémenté | Raison et conséquence |
|---|---|---|---|
| `ADR-011` | 21 object stores IndexedDB, un par table, avec index secondaires (§3.2) | **un seul enregistrement** `snapshot/current` contenant le document complet | L'index en mémoire est le moteur de requête (§4) ; IndexedDB n'est qu'un journal durable. Un `put` unique donne l'atomicité sans orchestration multi-stores. Coût : réécriture complète à chaque écriture (≈ 3 Mo au jeu de démo, quelques ms). Les index secondaires restent inutiles tant que l'hydratation est complète. À revoir si le volume dépasse ~50 Mo. |
| `ADR-012` | `leptos_router` en mode history avec repli SPA (§15.1) | **routage par fragment** `#/view/elements/01J…`, sans dépendance | Aucune réécriture d'URL côté serveur, donc l'application se sert depuis n'importe quel serveur statique. Les liens `arch://` deviennent de simples ancres, ce qui simplifie aussi le rendu Markdown. |
| `ADR-013` | Panneau IA simulé livré en F5 (§5, L-01→L-07) | **non livré** | Une clé d'API dans le navigateur serait publique. Le moteur (ChangeSet, validation déterministe, refus sans confirmation) est implémenté **et testé** ; seul le panneau conversationnel manque, en attendant un proxy serveur au lot 2. Aucune simulation trompeuse n'est affichée. |

Deux écarts mineurs supplémentaires :

- Les sous-ensembles BPMN et UML sont déclarés dans les profils, mais l'interface ne
  propose aujourd'hui que la création d'éléments ; les contraintes structurelles
  (`MM-BPMN-*`, `MM-UML-*`) sont spécifiées et partiellement implémentées via la
  matrice de relations, pas encore toutes activées.
- L'import/export OpenAPI est spécifié (§11) mais pas câblé dans l'interface du lot 1.
