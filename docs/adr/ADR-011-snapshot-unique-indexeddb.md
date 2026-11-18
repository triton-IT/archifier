# ADR-011 — Persistance IndexedDB : un snapshot unique plutôt que 21 object stores

| | |
|---|---|
| Statut | **Accepté pour le lot 1**, maintenu au MVP2 |
| Date | 17 novembre 2026 (rétro-ADR matérialisé le 18/11/2026) |
| Portée | Persistance locale du référentiel |
| Décisions liées | ADR-014 (collaboration), ADR-016 (port de dépôt) |

## Contexte

La spécification technique initiale prévoyait 21 object stores IndexedDB, un par table,
avec des index secondaires, pour reproduire le schéma PostgreSQL. Le lot 1 est un front
autonome : l'index de lecture vit en mémoire, et le seul besoin de durabilité est de
retrouver le référentiel complet au rechargement.

Un découpage en 21 stores aurait imposé des transactions multi-stores, une gestion de
migration par table et un risque de snapshot partiellement écrit. Le volume cible du MVP1
tient dans un enregistrement unique.

## Décision

- La base `archifier` contient le store `snapshot`, clé `current`, valeur JSON.
- **Une écriture = un enregistrement atomique** : `Repository::mutate` sérialise le
  `Dataset` complet et appelle `SnapshotStore::save` une seule fois.
- L'historique complet est conservé dans ce snapshot ; aucune troncature.
- Au lot 2, le store `collab` et le store `local_state` sont ajoutés à côté, sans
  transformer le store `snapshot`.
- Le snapshot reste le format d'échange (export/import JSON).

## Conséquences

Positives :

- rollback trivial : une écriture échouée ne laisse pas de transaction partielle ;
- migration de schéma unique et lisible ;
- export/import direct, sans jointure.

Négatives, acceptées :

- écriture du document complet à chaque mutation ; le volume MVP reste compatible ;
- pas de requête IndexedDB par champ : les index sont reconstruits en mémoire ;
- le snapshot n'est pas adapté à un très gros référentiel ; une migration vers un stockage
  plus fin serait un ADR séparé.

## Ce que cet ADR ne couvre pas

Le store CRDT `collab` (ADR-014), le store `local_state`, les quotas, la compaction et la
sauvegarde serveur.

## Références

- `technical-specifications-lot1-front-local.md` §3.
- `crates/repo/src/idb.rs`, `crates/repo/src/storage.rs`.
