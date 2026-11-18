# ADR-016 — Port de dépôt : le type `Repository` centralisé, sans trait séparé au MVP2

| | |
|---|---|
| Statut | **Accepté pour le MVP2** |
| Date | 18 novembre 2026 |
| Remplace | La formulation « toute l'UI passe par un trait unique `RepositoryPort` » de `technical-specifications-lot1-front-local.md` §2, **pour le MVP2 uniquement** |
| Portée | Frontière UI ↔ dépôt ↔ stockage |
| Décisions liées | ADR-011 (snapshot unique), ADR-014 (collaboration CRDT) |

## Contexte

L'addendum du lot 1 annonçait deux contrats :

1. un trait `RepositoryPort` unique, implémenté par `LocalRepository` puis par un futur `HttpRepository` ;
2. un type `WriteTx` comme point d'entrée unique d'écriture.

L'état vérifié au 18/11/2026 est différent :

- `RepositoryPort` **n'existe pas** comme trait. `archifier-ui` utilise `archifier_repo::Repository`, type concret, via `state::repo()`.
- `WriteTx` **n'existe pas** comme type. Le chemin d'écriture est `Repository::mutate`, privé, qui valide via `archifier-core` et `archifier-metamodel`, puis persiste atomiquement le snapshot.
- Aucune vue n'importe `idb`, `SnapshotStore` ou un moteur de stockage. Le seul point qui connaît `IdbStore` est `crates/ui/src/boot.rs`.
- Les 72 tests du baseline passent sans ce trait.

Deux options étaient possibles :

| Option | Contenu | Coût |
|---|---|---|
| **A** | Extraire un trait `RepositoryPort` en W1-00, faire retourner `Rc<dyn RepositoryPort>` à `state::repo()` | Refactor mécanique large (une quinzaine de fichiers de vues + signatures génériques à adapter), sans bénéfice fonctionnel immédiat |
| **B** | Acter que le type `Repository` **est** le port, centralisé par `state::repo()` | Aucun refactor, mais couplage de l'UI au type concret |

L'arbitrage du 18/11/2026 retient **l'option B**.

## Décision

### 1. Le port est le type `Repository`

L'API publique de `archifier_repo::Repository` est le contrat de dépôt du MVP2. Elle est gelée pour la durée du lot 2 : un dev ne la modifie pas pour faire passer son propre code sans demande d'évolution à l'orchestrateur.

### 2. Un seul point d'accès depuis l'UI

- `crates/ui/src/state.rs` expose `repo()` et reste le **seul** point d'accès de l'UI au dépôt.
- Aucune vue n'importe `archifier_repo::Repository`, `archifier_repo::idb`, `SnapshotStore` ou `CollabStore`.
- Les vues appellent `repo().méthode()` ; elles ne construisent jamais de dépôt.

### 3. `boot.rs` est la racine de composition

`crates/ui/src/boot.rs` est la racine de composition **normale** et le seul fichier autorisé à connaître :

- `IdbStore` / `MemoryStore` pour le démarrage réel ;
- le repli en mémoire si IndexedDB est indisponible ;
- le chargement initial du snapshot.

**Exception anti-panic documentée.** `state::repo()` peut construire un dépôt vide en mémoire si `install` n'a pas encore eu lieu. Ce n'est pas une racine de composition fonctionnelle : c'est un garde de dernier recours pour qu'une vue ne panique jamais avant le montage. Le test `memory_fallback_is_documented_exception` vérifie que `MemoryStore` n'apparaît que dans `boot.rs` et `state.rs`.

Le lot 2 ajoute `CollabStore` et `IdbCollabStore` ; ils sont wired dans `boot.rs` et dans `crates/repo`, jamais dans une vue.

### 4. Les frontières de stockage restent explicites

| Frontière | Rôle | Visible depuis l'UI ? |
|---|---|---|
| `SnapshotStore` | snapshot unique IndexedDB (ADR-011) | non |
| `CollabStore` (lot 2) | documents CRDT, store `collab` | non |
| `Repository` | index mémoire, règles, lectures, écritures de gouvernance, collaboration | oui, via `state::repo()` |

### 5. `WriteTx` reste une cible de nommage, pas un type du MVP2

Le chemin d'écriture de gouvernance reste `Repository::mutate` :

```text
Repository::mutate
  ├─ validation archifier-core / archifier-metamodel
  ├─ mutation de l'index en mémoire
  ├─ réindexation
  └─ un enregistrement atomique du snapshot
```

Le nom `WriteTx` peut être introduit plus tard, lors du lot 3, si un adaptateur HTTP ou un journal d'écriture l'exige. Il ne sera pas ajouté « pour le nom » au MVP2.

### 6. Pas d'extraction de trait en W1

W1 ne contient pas de lot « W1-00 extraction de `RepositoryPort` ». Les devs travaillent directement contre l'API publique de `Repository`. Toute demande d'évolution de cette API passe par l'orchestrateur.

## Conséquences

### Positives

- Aucun refactor large avant la dette du lot 1 ; le baseline de 72 tests reste la référence.
- Le risque de fusion entre devs diminue : `state.rs` et `boot.rs` restent les seuls points de couture.
- La règle opérationnelle qui compte est préservée : **aucune vue n'accède au stockage**.
- Le lot 2 peut ajouter `CollabStore` sans toucher aux vues.

### Négatives, acceptées

- L'UI est couplée au type concret `Repository` ; un futur `HttpRepository` ne sera pas substituable par simple changement de type.
- La formulation « trait unique » de l'addendum lot 1 n'est plus vraie littéralement. Elle est amendée par le présent ADR.
- Un adaptateur futur devra soit exposer la même API publique, soit être caché derrière une façade évaluée dans un ADR ultérieur.

### Mesures de compensation

1. **Test d'architecture** `ui_views_do_not_touch_storage` : aucune occurrence de `indexed_db`, `local_storage`, `idb`, `SnapshotStore`, `CollabStore`, `Repository::new` ou `archifier_repo::Repository` dans `crates/ui/src/views/**`.
2. **Test d'architecture** `idb_store_is_only_known_by_boot` : `IdbStore` n'apparaît que dans `boot.rs` et `crates/repo`.
3. **Test d'architecture** `memory_fallback_is_documented_exception` : `MemoryStore` n'apparaît que dans `boot.rs` et `state.rs` (repli anti-panic).
4. **Revue de porte** : toute nouvelle méthode publique de `Repository` utilisée par l'UI est ajoutée avec un test et documentée dans le contrat de dépôt.
5. **Interdiction maintenue** : une vue ne reçoit jamais un `Rc<dyn SnapshotStore>`, un handle IndexedDB ou un `Repository` construit localement.

## Ce que cet ADR ne couvre pas

- L'adaptateur HTTP du lot 3 et la substitution d'implémentation.
- L'authentification et l'autorisation du dépôt.
- La migration du snapshot vers un serveur.
- Le nom `WriteTx` et sa future extraction.

## Références

- `technical-specifications-lot1-front-local.md` §2 — contrat initial `RepositoryPort`.
- `technical-specifications.md` §21.1 — chemin d'écriture `CollabEdit`.
- `technical-specifications-lot2.md` §15 — contrat C-08 mis à jour.
- `docs/backlog-mvp2.md` — décision O-01.
- ADR-011 — snapshot unique IndexedDB.
- ADR-014 — collaboration CRDT.
