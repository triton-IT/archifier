# Fixtures W1b — export, diff et diagnostic

Deux jeux, tous deux des instantanés `archifier_core::Dataset` complets chargés
par `Repository::from_json` :

- `dataset.json` : jeu principal (diff, export, liens, graphe) ;
- `dataset-unreadable-diagram.json` : diagramme dont une coordonnée viole §5.7,
  pour l'erreur `EXPORT-002` côté dépôt.

## `dataset.json`

### Domaines

| Objet | Identifiant | Rôle |
|---|---|---|
| `Paiements` (`slug = paiements`) | `01K8ZP4N…0001` | domaine principal, propriétaire de ses objets |
| `Vide` (`slug = vide`) | `…000W` | domaine **sans aucun objet** : l'export doit produire un `index.md` valide |

### Objets de `Paiements`

| Objet | Identifiant | Rôle |
|---|---|---|
| `Order Service v2` | `…0002` | objet à **2 révisions** (`revisions["1"]`, `revisions["2"]`), source de la documentation à trois liens |
| `Pricing Service` | `…0003` | voisin direct (relation `serving`), cible du lien valide, 2e nœud du diagramme |
| `Tarification` | `…0004` | niveau 2 de la chaîne de dépendances |
| `Grille tarifaire` | `…0005` | niveau 3 |
| `Référentiel prix` | `…0006` | niveau 4 |
| `Réplication prix` | `…0007` | niveau 5 |
| `Journal prix` | `…0008` | niveau 6 |
| `Archive prix` | `…0009` | niveau 7 (au-delà de la borne) |
| `Purge prix` | `…000A` | niveau 8 (au-delà de la borne) |
| `Legacy Billing` | `…000B` | objet **archivé** : bandeau d'export, cible du lien archivé, cible archivée du graphe |
| `Facturation` | `…000X` | objet actif relié à `Legacy Billing` : vérifie le drapeau `lifecycle` du graphe |
| `Vue Paiements` | `…000N` | diagramme : 2 nœuds, 1 arête (`serving`) |
| `Récit de la commande` | `…000P` | Story : blocs Markdown, Diagramme, Élément — présente dans le jeu, export Story reporté en W1c (§5.1) et non exigé par les tests |

### Documentation de `Order Service v2`

| Lien | Statut attendu |
|---|---|
| `arch://element/…0003` | `OK` — cible active |
| `arch://element/…000C` | `BROKEN` — ULID valide, objet absent ; **cité deux fois** dans la documentation (lignes distinctes) pour le dédoublonnage |
| `arch://element/…000B` | `ARCHIVED` — cible archivée **après** écriture du lien |

Le cache `links` reflète ces trois entrées, sauf pour le lien archivé : il porte
`status = OK`, statut périmé qui force `broken_links()` à **recalculer** depuis le
`lifecycle` courant (§5.4). Le test vérifie explicitement ce cache périmé.

La première occurrence du lien cassé est une ligne de 315 caractères : la fenêtre
de contexte (80 avant + 80 après, `…` si rogné) doit rester bornée et conserver
l'URI et son libellé.

### Graphe

- Chaîne sortante `Order Service v2 → Pricing Service → … → Purge prix` (8 arêtes) :
  profondeur 3 atteint `Grille tarifaire`, profondeur 6 s'arrête à `Journal prix`
  avec `truncated = true`.
- `Pricing Service` a un prédécesseur (`Order Service v2`) et un successeur
  (`Tarification`) : base du test `BOTH` étiqueté.
- `Facturation → Legacy Billing` : arête active vers une cible archivée, incluse
  et signalée par `lifecycle = ARCHIVED`.

## `dataset-unreadable-diagram.json`

Domaine `Export invalide` (`slug = invalide`) avec un diagramme à deux nœuds dont
le second a `w = 0.0`. JSON ne peut pas porter `NaN`/`Infinity` : la coordonnée
**non finie** est construite en mémoire dans le test (noyau pur), tandis que ce
fichier couvre la même branche `EXPORT-002` côté dépôt (§5.7).

## Dates

Tous les `created_at` / `updated_at` sont figés (janvier–mars 2026). Aucune date
de génération : l'horloge des tests (`NOW_MS = 1_795_000_000_000`, 2026-11-18) ne
doit jamais apparaître dans un bundle exporté.
