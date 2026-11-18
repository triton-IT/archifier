# ADR-019 — Cycle de vie des interfaces : dépréciation, retrait et notifications dérivées

| | |
|---|---|
| Statut | **Proposé pour W5** |
| Date | 18 novembre 2026 |
| Remplace | Aucun ADR ; matérialise les décisions D-08/D-09 et le §12 de `technical-specifications-lot2.md` |
| Portée | `InterfaceVersion`, fenêtres temporelles, politique de rupture, centre de notifications local |
| Décisions liées | ADR-016 (écriture de gouvernance), ADR-018 (contrats et diff) |

## Contexte

`InterfaceVersion` possède déjà `available_from`, `available_until`, `breaking_change`,
`supersedes_version_id` et un statut éditorial `DRAFT | PUBLISHED | WITHDRAWN`. Les règles
existantes sont claires :

- `BR-001` : une version publiée définit `available_from` et `available_until` ;
- `BR-002` : `available_until > available_from` ;
- `BR-007` : le statut éditorial est distinct du statut temporel ;
- `BR-009` : plusieurs versions coexistent pendant une migration ;
- `BR-010` : une rupture est marquée puis confirmée par l'utilisateur.

Il manque la dimension **planifiée** du retrait : savoir qu'une version est dépréciée,
quand elle sera retirée, par quoi elle est remplacée, et ce qu'il advient d'une rupture
pendant cette fenêtre. Il manque aussi un centre de notifications qui signale les
échéances sans introduire un workflow d'approbation, explicitement refusé par le produit.

Deux pièges doivent être évités :

1. ajouter un statut `DEPRECATED` au statut éditorial, ce qui mélangerait `BR-007` et
   créerait une transition d'état à maintenir ;
2. persister des notifications ou des acquittements dans le snapshot, ce qui en ferait des
   données partagées, exportées et éventuellement synchronisées par la CRDT.

La dépréciation est une **propriété temporelle déclarée** de la version, pas un statut
éditorial. Les notifications sont des **faits dérivés** de l'état courant et de l'horloge,
pas des objets du référentiel.

## Décision

### 1. Champs additifs, portés par `InterfaceVersion`

```rust
pub struct InterfaceVersion {
    // … champs existants, inchangés …
    #[serde(default)]
    pub deprecation: Option<DeprecationPolicy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeprecationPolicy {
    #[serde(with = "time::serde::rfc3339")]
    pub announced_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub retirement_at: OffsetDateTime,
    pub replacement_version_id: Option<ObjectId>,
    pub breaking_policy: BreakingPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BreakingPolicy {
    Announce,
    RequireConfirmation,
    Forbid,
}
```

- `deprecation` est `Option` et `serde(default)` : un snapshot existant désérialise avec
  `None`, sans migration de numéro et sans réécriture.
- `breaking_policy` n'a pas de valeur par défaut à l'intérieur de `DeprecationPolicy` :
  dès qu'une dépréciation est déclarée, la politique est explicite (`IFACE-013`).
- Aucun champ n'est ajouté au `RepositoryObject` ni à `Interface`. La dépréciation est
  celle d'une **version**, pas celle de l'interface entière.
- Aucun statut `DEPRECATED` n'est ajouté à `PublicationStatus`. Le statut éditorial reste
  `DRAFT`, `PUBLISHED` ou `WITHDRAWN`, et le statut temporel reste calculé (`BR-007`).

### 2. Sémantique temporelle

Les fenêtres existantes sont semi-ouvertes : `[available_from, available_until)`. Un
`None` signifie « pas de borne ». La dépréciation ajoute une fin planifiée :

```text
fin_effective = min(available_until si défini, retirement_at)
fenêtre_effective = [available_from, fin_effective)
période_de_dépréciation = [announced_at, retirement_at)
```

Règles :

- `retirement_at` est le premier instant où la version n'est **plus disponible** ; la borne
  est exclusive, comme `available_until`.
- `announced_at` est l'instant où la dépréciation est annoncée et devient visible. Il peut
  précéder `available_from` (version planifiée dont le retrait est déjà annoncé).
- `announced_at` doit être **strictement antérieur** à `retirement_at`. Une annonce et un
  retrait au même instant relèvent d'un retrait éditorial (`WITHDRAWN`), pas d'une
  dépréciation planifiée.
- `retirement_at >= available_from` lorsqu'`available_from` est défini (`IFACE-010`).
- `retirement_at <= available_until` lorsqu'`available_until` est défini (`IFACE-011`).
  Pour une version publiée, `BR-001` garantit qu'il l'est. Pour un brouillon, la règle ne
  s'applique que si la borne existe.
- Une date de retrait dans le passé est acceptée pour un import historique : elle décrit
  un fait, elle ne déclenche aucune suppression.
- Les comparaisons se font en UTC. Les dates sont sérialisées en RFC 3339 avec offset ;
  aucun calcul en heure locale, aucun effet de changement d'heure.
- Le passage de `retirement_at` ne change **aucun** champ persisté. Il change uniquement
  le statut temporel dérivé et les notifications.

### 3. Politique de rupture

`breaking_change` reste le fait : la version contient une rupture, détectée manuellement ou
par le diff de contrat. `breaking_policy` est la **réponse autorisée** à ce fait pendant la
fenêtre de dépréciation. `BR-010` s'applique dans tous les cas : la rupture est marquée et
confirmée par l'utilisateur ; aucune politique ne rend une rupture silencieuse.

| Politique | Rupture autorisée ? | Confirmation requise | Effet |
|---|---|---|---|
| `Announce` | oui | confirmation standard `BR-010` | la rupture est annoncée dans le centre de notifications |
| `RequireConfirmation` | oui, après confirmation renforcée | `BR-010` + confirmation nominative des consommateurs impactés | sans confirmation, la publication de la rupture est refusée (`IFACE-013`) |
| `Forbid` | non | aucune confirmation ne suffit | toute rupture est refusée pendant `[announced_at, retirement_at)` (`IFACE-013`) |

Précisions :

- `Announce` ne veut pas dire « rupture automatique ». C'est le régime actuel, rendu
  visible : `BR-010` reste la porte.
- `RequireConfirmation` ajoute une confirmation qui nomme les consommateurs impactés, ou
  qui déclare explicitement qu'aucun consommateur n'est déclaré. Un simple « OK » générique
  ne suffit pas. Cette confirmation est une action de publication, pas un workflow
  persistant : aucun statut « en attente » n'est stocké.
- `Forbid` impose de publier une **nouvelle version** (ou une nouvelle interface) plutôt
  que de casser la version dépréciée. Une rupture déjà publiée avant `announced_at` n'est
  pas réécrite : la politique ne vaut que pour les publications suivantes.
- Une version sans `deprecation` conserve le régime `BR-010` standard. L'absence de
  politique n'est pas une politique `Announce` implicite.
- `replacement_version_id` est indépendant de `BreakingPolicy` : une dépréciation peut
  annoncer un remplaçant sans imposer de rupture.

### 4. Cohérence des fenêtres et du remplacement

| Code | Règle | Conséquence |
|---|---|---|
| `IFACE-010` | `announced_at < retirement_at` et `retirement_at >= available_from` (si défini) | erreur de validation |
| `IFACE-011` | `retirement_at <= available_until` (si défini) | erreur de validation |
| `IFACE-012` | `replacement_version_id` existe, appartient à la même interface, n'est pas la version elle-même, et la chaîne de remplacement est acyclique | erreur de validation |
| `IFACE-013` | une rupture sur une version dépréciée exige une politique explicite ; `Forbid` refuse, `RequireConfirmation` exige la confirmation | publication refusée |
| `IFACE-014` | une version dépréciée encore consommée sans acquittement local est signalée | notification |
| `IFACE-015` | un consommateur orphelin est signalé | notification |
| `IFACE-016` | une rupture non consommée par les consommateurs de la version précédente est signalée | notification |

Précisions :

- « Consommateur orphelin » : `consumer_object_id` absent ou archivé, `consumer_domain_id`
  absent, ou version cible absente. C'est un lien cassé, pas une faute de l'utilisateur.
- « Rupture non consommée » : une version `breaking_change = true` est publiée, et au moins
  un consommateur de la version précédente (désignée par `supersedes_version_id` ou par la
  chaîne de remplacement) n'a pas déclaré consommer la nouvelle. Le signal est informatif ;
  il ne bloque ni la publication ni le retrait.
- Un remplacement non publié à `retirement_at` ne rend pas la fenêtre invalide, mais produit
  une notification : le retrait laisserait un trou de couverture.
- L'absence de `replacement_version_id` n'est pas une erreur : une version peut être retirée
  sans successeur. Elle est signalée si la version est encore consommée, pas comme une
  incohérence de fenêtre.
- Les fenêtres de dépréciation de deux versions peuvent se chevaucher : `BR-009` autorise
  explicitement la coexistence pendant une migration. Aucune règle n'interdit ce
  chevauchement.
- Aucune règle n'oblige à retirer une version à `retirement_at`. La date est une
  planification, pas un déclencheur d'archivage.

### 5. Statut dérivé, jamais persisté

Le statut éditorial et les dates restent les seules données stockées. L'UI et les
notifications calculent :

| Notion dérivée | Calcul | Persisté ? |
|---|---|---|
| `deprecated` | `now >= announced_at && now < fin_effective` | non |
| `retirement_due` | `now >= announced_at` et `retirement_at - now <= 30 jours` et `now < retirement_at` | non |
| `retired` | `now >= fin_effective` | non |
| `replacement_missing` | un remplacement est déclaré, mais absent, archivé ou non disponible à `retirement_at` | non |

- Le seuil de 30 jours est une constante de produit proposée, révisable par mesure ou par
  décision UX. Il n'est pas stocké dans le snapshot.
- `TemporalStatus` existant (`SCHEDULED | ACTIVE | EXPIRED`) est calculé à partir de la
  fenêtre effective, donc de `retirement_at` lorsqu'il est défini. Une version dépréciée
  encore active reste `ACTIVE` ; elle n'invente pas un quatrième statut temporel.
- Aucune tâche de fond, aucun réveil planifié, aucun job de minuit ne change l'état. Le
  calcul se fait à l'ouverture de l'écran, à la demande, et reste déterministe pour une
  même horloge.

### 6. Centre de notifications : dérivé, sans workflow

Le centre de notifications est une **liste calculée** :

| Notification | Source | Acquittable localement |
|---|---|---|
| Dépréciation annoncée | `announced_at <= now < retirement_at` | oui |
| Retrait proche | `retirement_due` | oui |
| Version retirée | `retired` | oui |
| Version dépréciée encore consommée | `IFACE-014` | oui |
| Consommateur orphelin | `IFACE-015` | oui |
| Rupture non consommée | `IFACE-016` | oui |
| Remplacement indisponible au retrait | `replacement_missing` | oui |

Règles :

- Aucune notification n'est un objet du snapshot, une ligne de base, un `ChangeSet` ou un
  événement persisté. La liste est recalculée depuis le snapshot et `now`.
- L'ordre est déterministe : échéance croissante, puis code de règle, puis identifiant.
- Aucun workflow d'approbation, aucun statut « approuvé », aucun « en attente de
  validation », aucune transition à maintenir.
- Une notification ne bloque jamais une publication, un retrait ou un export. Elle informe.
- Une notification ne modifie jamais l'objet source : acquitter n'est pas approuver, et
  masquer n'est pas corriger.
- L'UI affiche le code de règle, l'objet concerné, l'échéance et un lien vers la fiche. Les
  notifications ne sont ni exportées, ni incluses dans un snapshot, ni projetées.

### 7. Acquittements strictement locaux

- Les acquittements et masquages sont stockés dans le store IndexedDB `local_state`
  (décision D-08), additif, séparé du snapshot et du store `collab`.
- Ils ne sont jamais exportés, jamais synchronisés par la CRDT, jamais envoyés à un pair et
  jamais inclus dans une sauvegarde de référentiel.
- La clé d'acquittement contient `rule_code`, `version_id` et une empreinte du fait
  (révision de l'objet, date d'échéance, ensemble des consommateurs). Si le fait change, la
  clé change : la notification réapparaît. Un acquittement ne masque pas une nouvelle
  rupture.
- Un changement de profil de navigateur, de poste ou de stockage local réaffiche les
  notifications. C'est cohérent avec leur nature locale.
- L'acquittement n'a ni auteur, ni horodatage exporté, ni valeur d'audit. Ce n'est pas une
  décision de gouvernance.

### 8. Écriture de gouvernance uniquement

- `DeprecationPolicy` est écrite par `Repository::mutate`, avec `revision` et `If-Match` ;
  un conflit produit `409` et le dialogue recharger / comparer / forcer.
- Aucune variante de `CollabEdit` ne peut créer, modifier ou retirer une dépréciation. La
  CRDT ne change jamais un statut de revue, une version ou une rupture (`CR-001`,
  `CR-002`).
- Une modification de politique est une révision d'objet comme une autre : elle est
  historisée, attribuée à un utilisateur et jamais silencieuse.
- Les champs sont additifs : aucune migration de snapshot n'est nécessaire, aucun numéro
  de version de snapshot n'est consommé.
- Une proposition de `ChangeSet` peut regrouper plusieurs dépréciations, mais chaque
  version est validée séparément et l'écriture reste une écriture de gouvernance unique.

### 9. Invariants vérifiés par des tests nommés

| Invariant | Test |
|---|---|
| Les fenêtres de retrait sont cohérentes | `lifecycle_retirement_window_is_consistent` |
| Une politique est obligatoire dès qu'une dépréciation existe | `breaking_policy_is_required` |
| Le remplacement existe et appartient à la même interface | `replacement_version_must_exist` |
| Une chaîne de remplacement cyclique est refusée | `replacement_cycle_is_refused` |
| `Forbid` refuse une rupture pendant la fenêtre | `forbid_policy_refuses_breaking_change` |
| `RequireConfirmation` exige une confirmation nominative | `require_confirmation_names_consumers` |
| Une version dépréciée encore consommée est signalée | `deprecated_version_still_consumed_is_reported` |
| Un consommateur orphelin est signalé | `orphan_consumer_is_reported` |
| Une rupture non consommée est signalée | `unconsumed_breaking_change_is_reported` |
| La dépréciation ne change pas le statut éditorial | `deprecation_does_not_change_publication_status` |
| L'acquittement est local et non exporté | `notification_ack_is_local_and_not_exported` |
| L'acquittement ne modifie pas la source | `notification_ack_does_not_mutate_source` |
| Un snapshot sans dépréciation reste lisible | `snapshot_without_deprecation_deserializes` |
| La dépréciation n'est jamais accessible par la CRDT | `deprecation_policy_is_governance_only` |

## Conséquences

### Positives

- Une version peut annoncer sa fin de vie, son remplaçant et sa politique de rupture sans
  créer un nouveau statut ni une nouvelle machine à états.
- Les notifications sont recalculables, donc jamais désynchronisées d'un snapshot restauré
  ou importé.
- L'acquittement local ne pollue ni le référentiel, ni les exports, ni la synchronisation.
- `BR-007` est préservé : la dépréciation est temporelle, pas éditoriale.
- `BR-009` est préservé : plusieurs versions peuvent se chevaucher pendant une migration.
- Aucune migration de snapshot n'est nécessaire.

### Négatives, acceptées

- L'absence de notification persistée signifie qu'une notification peut réapparaître après
  un changement de poste ou de profil. C'est le prix de la nature locale.
- Le seuil de 30 jours est arbitraire et devra être validé par l'usage.
- `RequireConfirmation` ajoute une étape UX qui peut être perçue comme lourde ; elle est
  réservée aux versions dépréciées qui cassent.
- Les règles `IFACE-014` à `IFACE-016` sont informatives : un utilisateur peut ignorer une
  alerte et retirer une version encore consommée. Le produit ne l'empêche pas.
- Une date de retrait dans le passé ne déclenche rien : un import historique peut produire
  une notification « déjà retiré » sans action automatique.

## Ce que cet ADR ne couvre pas

- Le workflow d'approbation formel, les rôles et les circuits de validation, refusés au
  MVP2.
- L'archivage automatique, la suppression d'une version ou la migration forcée des
  consommateurs.
- La politique de versionnement sémantique et la numérotation des versions.
- Le contenu du diff de contrat et la classification des ruptures, couverts par ADR-018.
- La synchronisation multi-utilisateur des acquittements et des notifications.
- Les notifications par e-mail, webhook ou canal externe.
- La conformité RGPD des journaux de notification.

## Références

- `functional-specifications.md` — `FR-008`, `FR-013`, `FR-027`, `BR-001`, `BR-002`,
  `BR-007`, `BR-009`, `BR-010`, `NFR-001`, `NFR-002`.
- `technical-specifications-lot2.md` §12 — politique de cycle de vie et centre de
  notifications ; décisions D-08 et D-09.
- `docs/backlog-mvp2.md` — T4-01, T4-03, T4-04.
- ADR-016 — `Repository::mutate`, verrou optimiste et frontière de stockage.
- ADR-018 — import, diff et rupture de contrat.
