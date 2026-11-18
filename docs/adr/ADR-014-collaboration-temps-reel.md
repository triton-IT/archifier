# ADR-014 — Édition collaborative temps réel, sans serveur au MVP2

| | |
|---|---|
| Statut | Accepté pour le MVP2 |
| Date | 18 novembre 2026 |
| Remplace | La ligne « Édition collaborative CRDT — futur » du tableau « Après MVP1 » |
| Portée | Plan collaboratif uniquement (documentation, nom, layout, blocs de Story, commentaires, présence) |

## Contexte

Le MVP1 est mono-utilisateur de fait : la persistance locale ne partage rien. Le produit
cible est explicitement collaboratif (« référentiel collaboratif »), et le MVP2 doit
introduire l'édition simultanée **sans rendre un serveur obligatoire**, puisque le lot 2
reste une application statique dont la source de vérité est IndexedDB.

Deux exigences se contredisent en apparence : « plusieurs personnes éditent en même temps »
et « aucun serveur ». La sortie de ce dilemme est de séparer ce qui converge de ce qui doit
être arbitré.

## Décision

### 1. Deux plans, deux algos

| Plan | Contenu | Sémantique |
|---|---|---|
| Gouvernance | domaines, éléments, relations, interfaces, versions, contrats, revues, ChangeSets | écriture unique, verrou optimiste `revision` + `If-Match`, `409` |
| Collaboration | documentation Markdown, `name`, layout de diagramme, blocs de Story, commentaires, présence | CRDT, convergence automatique |

Un contrat d'interface n'est pas un paragraphe : il exige une confirmation humaine
(`BR-010`, `FR-025`). Inversement, bloquer sur un `409` deux personnes qui tapent dans le
même document serait une faute d'ergonomie. Les deux plans ne partagent ni chemin
d'écriture, ni garantie.

**Conséquence dure** : le CRDT ne peut jamais créer un objet, publier une version ou
changer un statut de revue (`CR-002`).

### 2. CRDT : Yrs (implémentation Rust de Yjs)

| Option | Verdict |
|---|---|
| **Yrs** | retenu — `Y.Text` éprouvé pour le texte riche, vecteurs d'état compacts, WASM natif |
| Automerge | repli si le spike échoue — modèle plus lourd, empreinte mémoire supérieure |
| Loro | écarté pour le MVP2 — écosystème plus jeune sur le texte |
| CRDT maison | refusé — réimplémenter un texte CRDT correct est un projet, pas une tâche |

**Porte de sortie du spike** (une journée, bloquante) :

1. un `Y.Text` de 20 000 caractères supporte 200 insertions concurrentes sans perte ;
2. l'encodage d'une frappe reste sous 2 Ko ;
3. le décodage d'un document de 20 000 caractères reste sous 30 ms ;
4. le bundle WASM augmente de moins de 900 Ko brut.

Si l'un de ces points échoue, on bascule sur Automerge et on documente l'écart. On ne
réécrit pas un CRDT maison.

### 3. Transport : local d'abord

Un seul trait `SyncTransport`, trois implémentations :

| Implémentation | Statut | Moyen |
|---|---|---|
| `LocalBroadcastTransport` | **nominal MVP2** | `BroadcastChannel("archifier:collab")`, messages binaires |
| `InProcessTransport` | tests | canal en mémoire, ordre et partitions contrôlables |
| `WebSocketTransport` | préparé, non activé | relais réseau, même encodage |

`BroadcastChannel` donne une **vraie** co-édition entre deux fenêtres du même navigateur :
c'est la démonstration de `FR-036`, et elle ne dépend d'aucun serveur. Ses limites sont
assumées et affichées : même origine, même profil, aucune frontière de sécurité.

> **Amendement MVP2 (contrat C-03, `technical-specifications-lot2.md` §5).** Le canal
> exact est `archifier:collab:v1`. Le trait `SyncTransport` inclut `SubscriptionId`,
> `unsubscribe`, `peers` et `status` ; C-03 fait foi en cas de divergence avec ce tableau.

### 4. Persistance

Un store IndexedDB dédié (`collab`), un enregistrement binaire par document
(`doc:<object_id>`), séparé du snapshot unique d'`ADR-011`. Le snapshot conserve la
documentation et le layout pour l'export, mais la **source de vérité** du texte et du
layout devient le document CRDT. Initialisation idempotente depuis le snapshot si le
document est absent.

### 5. Fusion non conforme : signaler, jamais corriger

Un CRDT converge, il ne valide pas. Après application d'une mise à jour distante, le
moteur revalide la vue concernée. Une violation produit un diagnostic `MERGE_VIOLATION`
local, non persisté, et marque l'objet « non conforme » dans l'interface. **Aucune
suppression, aucune correction automatique** (`CR-003`, `FR-037`).

### 6. Historique

Les rafales d'édition sont regroupées : une révision après 2 s d'inactivité, pas une par
frappe (`FR-038`). L'historique reste celui d'une action humaine.

## Conséquences

Positives :

- collaboration réelle disponible sans serveur, donc démontrable immédiatement ;
- le relais réseau est un ajout d'implémentation, pas une refonte ;
- la gouvernance conserve ses garanties fortes, sans compromis.

Négatives, acceptées :

- dépendance lourde (Yrs) et temps de démarrage à surveiller ;
- deux chemins d'écriture à maintenir, donc deux classes de tests ;
- un état convergé peut être métier-invalide : c'est signalé, pas empêché ;
- `BroadcastChannel` ne franchit ni le profil de navigateur ni la machine.

## Ce que cet ADR ne couvre pas

Le relais réseau, l'authentification et l'autorisation du plan collaboratif, le chiffrement
de bout en bout, la conformité RGPD du contenu partagé, et la résolution **sémantique** des
conflits.

## Références

- `functional-specifications.md` — section « Collaboration temps réel (MVP2) », FR-031 à FR-039, CR-001 à CR-007.
- `technical-specifications.md` — §21.
- `technical-specifications-lot1-front-local.md` — ADR-011 (snapshot unique), ADR-012 (routage par fragment).
