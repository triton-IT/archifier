# ADR-021 — Migration de métamodèle ArchiMate 3.2 → 4.x : infrastructure sans classe inventée

| | |
|---|---|
| Statut | **Proposé pour W2** |
| Date | 18 novembre 2026 |
| Remplace | Aucun ADR ; précise le §13 de `technical-specifications-lot2.md` et supersède la formulation du fichier de mapping du §5 de `technical-specifications.md` |
| Portée | Fichier de mapping, diagnostic de compatibilité, rapport de couverture ; aucune migration automatique |
| Décisions liées | ADR-016 (écriture de gouvernance pour une future application) |

## Contexte

La spécification fonctionnelle est explicite : la cible produit est ArchiMate 4, mais
**aucun métamodèle ArchiMate 4 ne doit être inventé**. Au 18 novembre 2026, le constat du
17 novembre 2026 reste valable : The Open Group ne publie pas de spécification ArchiMate 4
stable. Le profil effectivement chargé est `archimate:3.2`, avec 42 classes et 11 relations
au début de W2.

Le MVP2 doit donc préparer la migration sans la simuler :

- un fichier de mapping doit exister et être chargeable ;
- un diagnostic doit dire, pour chaque objet, s'il est compatible, mappé, non mappé ou à
  revoir ;
- aucune classe `archimate:4.*` ne doit être déclarée dans un profil chargeable ;
- les identifiants doivent survivre à toute migration future.

Deux formulations coexistent dans les documents existants :

| Source | Chemin | Schéma |
|---|---|---|
| `technical-specifications.md` §5 | `profiles/mappings/archimate-3.2_to_4.x.toml` | `[[element_map]] from/to`, `[[relation_map]] from/to`, `[unmapped] policy = "keep" \| "block"` |
| `technical-specifications-lot2.md` §13 | `crates/metamodel/mappings/archimate-3.2-to-4.x.toml` | `source_profile`, `target_profile`, `status`, `classes = []`, `default_policy = "preserve"` |

Le présent ADR retient la seconde forme, plus récente et cohérente avec l'arborescence
réelle des crates. Le chemin et le schéma du §5 sont supersédés.

Le risque principal n'est pas technique, il est documentaire : un développeur zélé déclare
« juste une classe temporaire » pour tester la palette, et le produit se met à prétendre
connaître un standard qui n'existe pas. La porte doit échouer avant ce scénario.

## Décision

### 1. La cible est une étiquette, pas un profil chargeable

- Le profil source normative reste `archimate:3.2`. C'est le seul profil ArchiMate
  enregistré dans `MetamodelRegistry` au MVP2.
- `archimate:4.x` est un **identifiant de cible** dans le fichier de mapping. Il n'est pas
  enregistré comme profil, ne fournit aucune classe, aucune relation, aucune règle et
  n'alimente ni la palette ni l'inspecteur.
- Aucune classe, aucun `metaclass`, aucun identifiant d'élément ne commence par
  `archimate:4` dans un profil chargeable. Un test échoue si c'est le cas.
- Aucune classe 4 « temporaire », « de test » ou « derrière un feature flag » n'est admise.
  Le test d'architecture ne connaît pas d'exception.
- La palette et l'inspecteur continuent de lire le profil au runtime (T3-05) : ajouter une
  classe 3.2 dans un TOML suffit, sans modification de code. Ce mécanisme ne sert pas à
  déclarer une classe 4.

### 2. Un fichier unique, vide par construction

Chemin canonique :

```text
crates/metamodel/mappings/archimate-3.2-to-4.x.toml
```

Contenu initial et unique état valide au MVP2 :

```toml
# Mapping ArchiMate 3.2 → 4.x.
# Aucune classe cible n'est inventée : The Open Group n'a pas publié de
# spécification 4.x stable. Ce fichier est une infrastructure, pas un métamodèle.

source_profile = "archimate:3.2"
target_profile = "archimate:4.x"
status = "unpublished"
default_policy = "preserve"

classes = []
relations = []
```

Forme d'une entrée future, volontairement vide de tout identifiant cible :

```toml
# classes = [
#   { from = "<clé de classe source déclarée dans archimate:3.2>",
#     to = "<clé de classe cible publiée par The Open Group>",
#     review = "required" },
# ]
```

Règles du fichier :

- `source_profile` doit valoir `archimate:3.2` ; `target_profile` doit valoir
  `archimate:4.x`.
- `status` doit valoir `unpublished`. Toute autre valeur est refusée au chargement tant que
  la cible n'est pas publiée.
- `default_policy` doit valoir `preserve`. L'ancienne valeur `block` est refusée : elle
  impliquerait de supprimer ou de bloquer un objet non mappé, contraire à la préservation
  des identifiants et à l'esprit de `CR-003`.
- `classes` et `relations` sont des tableaux. Au MVP2, ils sont vides.
- Une entrée `from` dupliquée est refusée. Une entrée sans `to` est refusée. Une entrée
  `to` qui ne correspond à aucune classe du profil cible est refusée dès que ce profil
  existe.
- Le chargement refuse les champs inconnus : une faute de frappe dans un mapping ne doit
  pas être ignorée silencieusement.
- Un fichier non vide est refusé tant que le profil cible n'est pas enregistré.
  « Enregistrer un profil » est un acte normatif, pas une commodité de développement.

### 3. Le mapping est une donnée, pas un profil ni du code

- Le mapping ne crée aucune classe, ne modifie aucune règle de relation et ne remplace pas
  le profil source.
- Il ne contient pas de code exécutable, pas d'expression, pas de script. C'est une table
  de correspondance déclarative.
- Il ne duplique pas les libellés, glyphes, couches ou propriétés : ceux-ci restent dans le
  profil cible, lorsqu'il existera.
- Le charger ne modifie pas `MetamodelRegistry`, ne change pas le profil par défaut de la
  notation et n'affecte pas les objets existants.
- Le mapping est versionné avec le code, relu comme une ressource normative et couvert par
  des tests. Ce n'est pas un réglage utilisateur.

### 4. Diagnostic en lecture seule

Le diagnostic prend un snapshot, le profil source, le mapping et, si disponible, le profil
cible. Il produit un rapport par objet et par relation, sans jamais écrire.

Statuts, par ordre de priorité :

| Statut | Signification | Condition |
|---|---|---|
| `requires_review` | une décision humaine est nécessaire | cible non publiée, mapping marqué `review = "required"`, risque sémantique (changement de couche, classe abstraite devenue concrète, règle de relation incompatible), ou relation dont une extrémité est à revoir |
| `mapped` | une correspondance explicite et valide existe | entrée `classes` valide, classe cible présente dans le profil cible |
| `compatible` | la classe source existe à l'identique dans la cible | profil cible publié, aucune mapping nécessaire, sémantique identique |
| `unmapped` | aucune correspondance n'est déclarée | profil cible publié, aucune entrée, aucune identité de classe |

Précisions :

- Aujourd'hui, le profil cible n'existe pas : tous les objets et relations ressortent en
  `requires_review`, avec la raison `target_unpublished`. C'est le résultat honnête. On ne
  certifie pas `compatible` ni `mapped` sans cible.
- `unmapped` n'est pas une erreur : avec `default_policy = "preserve"`, l'objet conserve sa
  classe source et son identifiant. Le diagnostic le dit explicitement.
- Une relation est `requires_review` si l'une de ses extrémités l'est, ou si le couple
  source/relation/cible n'est pas autorisé par les règles du profil cible.
- Le rapport est trié par identifiant d'objet, puis par identifiant de relation. Il ne
  contient ni horodatage, ni numéro de build, ni ordre dépendant d'une `HashMap`.
- Le rapport peut être affiché dans l'UI et produit en Markdown ou JSON. Il n'est pas
  persisté dans le snapshot et ne crée pas de `MergeDiagnostic`.
- Aucun objet n'est migré automatiquement, même si son statut est `compatible` ou `mapped`.
  Le diagnostic ne fait que constater.

### 5. Préservation des identifiants

- Les identifiants d'objets, de relations, d'interfaces, de versions, de diagrammes, de
  stories, de revues et de ChangeSets ne sont **jamais** réécrits par le diagnostic ni par
  une future migration.
- Un objet `unmapped` conserve sa classe source, son `metamodel_version` source et son
  identifiant. Il n'est ni supprimé, ni converti en une classe générique, ni dupliqué.
- Une future migration, si elle est décidée, devra être une écriture de gouvernance
  explicite : proposition de `ChangeSet`, avant/après, validation déterministe,
  confirmation humaine, puis application. Elle ne passera jamais par la CRDT
  (`CR-001`, `CR-002`).
- Le changement éventuel de `metamodel_version` ou de `metaclass` est l'objet même de cette
  future migration, pas un effet de bord du diagnostic. Il sera tracé comme une révision
  d'objet, avec un acteur et une date.
- Aucun bouton « migrer tout » n'est livré en W2. Le lot W2 livre l'infrastructure et le
  diagnostic, pas la bascule.

### 6. Rapport de couverture

- `docs/metamodel-coverage.md` est généré par `coverage_report()` de
  `archifier-metamodel`, puis publié. Il n'est jamais édité à la main.
- W2 étend le rapport pour inclure, par profil : classes, relations, couples autorisés,
  lacunes connues, et pour `archimate:4.x` : statut `unpublished`, nombre de classes
  déclarées (zéro), nombre d'entrées de mapping (zéro) et objets `requires_review`.
- Le rapport ne liste aucune classe 4. Il liste l'**absence** de classe 4 comme un fait.
- Un test `metamodel_coverage_is_up_to_date` régénère le rapport et le compare au fichier
  versionné. Une modification de profil qui n'est pas répercutée fait échouer la porte.
- Le rapport reste un document de couverture, pas une preuve de conformité à un standard
  non publié.

### 7. Invariants vérifiés par des tests nommés

| Invariant | Test |
|---|---|
| Le mapping ne déclare aucune classe | `archimate4_mapping_has_no_declared_class` |
| Aucun profil chargeable ne contient de classe 4 | `no_archimate4_class_in_loadable_profiles` |
| Un mapping non vide est refusé tant que la cible est non publiée | `mapping_rejects_non_empty_classes_while_unpublished` |
| La politique `block` est refusée | `mapping_rejects_block_policy` |
| Le diagnostic rapporte la compatibilité | `migration_diagnostic_reports_compatibility` |
| Le diagnostic préserve les identifiants | `migration_diagnostic_preserves_ids` |
| Le diagnostic n'écrit jamais | `migration_diagnostic_is_read_only` |
| Le diagnostic est déterministe | `migration_diagnostic_is_deterministic` |
| Une relation dont une extrémité est à revoir est signalée | `mapping_relation_endpoints_must_map` |
| Le rapport de couverture est à jour | `metamodel_coverage_is_up_to_date` |
| La palette lit le profil au runtime | `palette_reflects_runtime_profile` |
| L'inspecteur lit le profil au runtime | `inspector_reflects_profile_properties` |

## Conséquences

### Positives

- Le produit peut dire « cible ArchiMate 4 » sans prétendre connaître un standard non
  publié.
- Le fichier de mapping existe, il est versionné, relu et testé ; le jour de la
  publication, il reste à le remplir et à lever le verrou, pas à créer l'infrastructure.
- Les identifiants sont protégés par conception : le diagnostic ne touche à rien et une
  future migration est une écriture explicite.
- Le rapport de couverture rend les lacunes visibles au lieu de les masquer.
- Le schéma unique du fichier met fin à la divergence entre le §5 de
  `technical-specifications.md` et le §13 de `technical-specifications-lot2.md`.

### Négatives, acceptées

- Tant que la cible n'est pas publiée, le diagnostic est pessimiste : tout est
  `requires_review`. C'est peu spectaculaire, mais honnête.
- Le fichier de mapping est inerte au MVP2 ; il ne sert qu'à prouver que le chargeur et le
  diagnostic fonctionnent.
- Le chemin et le schéma du §5 de `technical-specifications.md` deviennent obsolètes ; une
  mise à jour documentaire hors de cet ADR sera nécessaire.
- Le rapport de couverture doit être régénéré à chaque évolution de profil, ce qui ajoute
  une étape à la porte.
- Une migration réelle restera un chantier humain, même après publication : le mapping ne
  décide pas des équivalences sémantiques à la place des architectes.

## Ce que cet ADR ne couvre pas

- Le contenu réel du mapping 3.2 → 4.x : aucune classe cible n'est proposée ici.
- L'application de la migration, la réécriture des snapshots et la bascule de profil.
- La conformité formelle à une future spécification The Open Group.
- Le mapping inverse 4.x → 3.2.
- La palette et l'inspecteur 4.x, qui n'existent pas tant que le profil cible n'est pas
  publié.
- La migration des documents CRDT : la CRDT ne porte ni classe, ni type, ni identifiant.
- Les profils BPMN et UML, qui ne sont pas concernés par ce mapping.

## Références

- `functional-specifications.md` — section « Statut d'ArchiMate 4 », décision v0.1 sur la
  cible et l'interdiction d'inventer un métamodèle.
- `technical-specifications.md` §5 — profils, `MetamodelRegistry`, ancien chemin de mapping
  supersédé ; §20 pour le journal des ADR.
- `technical-specifications-lot2.md` §13 — migration de métamodèle, fichier canonique,
  `status = "unpublished"`, `default_policy = "preserve"`.
- `docs/backlog-mvp2.md` — T3-01, T3-02, T3-05, porte G3.
- `crates/metamodel/src/lib.rs` — `Profile::parse`, `MetamodelRegistry::coverage`,
  `coverage_report()`.
- `crates/metamodel/profiles/archimate-3.2.toml` — profil source, 42 classes et 11
  relations à l'ouverture de W2.
