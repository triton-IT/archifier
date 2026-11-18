# ADR-018 — Contrats externes : OpenAPI et AsyncAPI, brut conservé et export reproductible

| | |
|---|---|
| Statut | **Accepté pour W1a et W5** |
| Date | 18 novembre 2026 |
| Remplace | Aucun ADR ; matérialise le contrat C-09 et précise les §10 et §11 de `technical-specifications-lot2.md` |
| Portée | Import, conservation, empreinte, diff et export des contrats OpenAPI et AsyncAPI |
| Décisions liées | ADR-016 (port de dépôt), ADR-019 (cycle de vie des interfaces) |

## Contexte

Le MVP1 conserve un `contract_snapshot` normalisé, mais pas le document d'origine. Cette
perte a deux conséquences :

- un export ne peut pas restituer les extensions, commentaires, ancres et choix de
  présentation du document importé ;
- l'idempotence d'un réimport repose sur une normalisation dont la stabilité n'est pas
  prouvée.

Le MVP2 doit importer OpenAPI 3.0, 3.1 et 3.2, ainsi qu'AsyncAPI 2.x et 3.x, en JSON comme
en YAML, sans transformer l'outil en compilateur de spécifications. Il doit aussi garantir
qu'un même document ne crée pas une nouvelle version à chaque import, et que deux exports
d'une même révision sont identiques octet à octet (`NFR-009`).

Trois tensions doivent être tranchées explicitement :

1. **fidélité vs normalisation** : conserver le brut et produire un modèle normalisé sont
   deux besoins différents, pas deux implémentations du même besoin ;
2. **idempotence vs extensions** : si l'export ajoute des `x-architecture-*`, un réimport
   naïf calcule une autre empreinte et croit à un nouveau contrat ;
3. **contrat vs référentiel** : un canal AsyncAPI ou un chemin OpenAPI n'est pas un objet
   de la palette ; le contrat reste un document, pas une nouvelle classe de référentiel.

## Décision

### 1. Trois champs additifs, aucun changement d'identité

```rust
pub struct InterfaceVersion {
    // … champs existants, inchangés …
    #[serde(default)]
    pub contract_format: Option<ContractFormat>,
    #[serde(default)]
    pub contract_raw: Option<String>,
    #[serde(default)]
    pub contract_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContractFormat {
    OpenApi30,
    OpenApi31,
    OpenApi32,
    AsyncApi2,
    AsyncApi3,
}
```

- Les trois champs sont **additifs** et `serde(default)` : un snapshot existant désérialise
  avec `None`, sans migration de numéro et sans réécriture.
- `contract_snapshot` reste le modèle normalisé utilisé par le diff et l'export généré.
  `contract_raw` conserve le texte exact du document importé.
- Aucun identifiant d'interface, de version, de domaine ou de relation n'est dérivé du
  contrat ni de son empreinte. `contract_hash` n'est jamais une clé primaire.
- Une version publiée reste immuable : un contenu différent produit une **proposition** de
  nouvelle version, jamais une réécriture silencieuse de la version existante (`BR-010`).

### 2. Formats et détection de version

| Famille | Versions acceptées | Détection | Refus |
|---|---|---|---|
| OpenAPI | 3.0.x, 3.1.x, 3.2.x | champ racine `openapi` | `OAS-001` |
| AsyncAPI | 2.x, 3.x | champ racine `asyncapi` | `ASYNC-001` |
| Swagger 2.0, OpenAPI 4+, AsyncAPI 1.x/4+ | aucune | — | `OAS-001` / `ASYNC-001` |
| Document illisible, ambigu ou non conforme au format déclaré | — | — | `OAS-002` / `ASYNC-002` |

Règles :

- La version est détectée sur le **contenu**, pas sur l'extension du fichier ni sur un
  choix de l'utilisateur.
- Un document qui contient à la fois `openapi` et `asyncapi` est ambigu et refusé
  (`OAS-002` ou `ASYNC-002` selon la famille attendue), même si l'un des deux champs est
  vide.
- Les versions `-rc`, `-beta` et les versions d'un majeur non listé sont refusées : on
  n'importe pas un contrat dont la sémantique n'est pas stabilisée.
- Le format retenu est stocké dans `contract_format`. Il est cohérent avec
  `Interface.contract_type` : `OpenApi` pour OpenAPI, `Event` pour AsyncAPI.

### 3. Le document brut est conservé tel quel

- `contract_raw` contient le texte **exact** reçu : mêmes espaces, mêmes commentaires,
  même ordre de clés, mêmes fins de ligne (`LF` ou `CRLF`), mêmes extensions `x-*`.
- Le texte est stocké en UTF-8. Un BOM UTF-8, un encodage UTF-16 ou un encodage non UTF-8
  est refusé à l'import (`OAS-002` / `ASYNC-002`) : accepter puis retirer un BOM rendrait
  la promesse « même texte » ambiguë. Le refus est explicite, jamais silencieux.
- Un import YAML ne doit contenir qu'**un seul document**. Les flux multi-documents
  (`---` répétés) sont refusés (`ASYNC-002` / `OAS-002`) : l'empreinte d'un flux n'a pas
  de sens unique.
- Les balises personnalisées et les constructions qui ne se projettent pas en JSON sont
  refusées. Les ancres et alias YAML standard sont résolus ; les limites anti-abus
  s'appliquent (plafond proposé : 2 Mio de texte, profondeur 128).
- Aucun `$ref` externe n'est résolu, ni par HTTP, ni par fichier. Un `$ref` non résolu
  produit `OAS-030` / un statut « inconnu » au diff, jamais une requête réseau. Cela évite
  un SSRF et rend l'import reproductible hors ligne.
- Le brut est affiché comme du **texte**, jamais injecté comme HTML. Il n'est ni exécuté,
  ni interprété par un moteur de template.

### 4. Empreinte canonique SHA-256

`contract_hash` est l'empreinte SHA-256, en 64 caractères hexadécimaux minuscules, de la
forme canonique du document. Le `sourceFingerprint` de la spécification fonctionnelle
correspond à ce champ ; il n'existe pas deux empreintes concurrentes. Forme canonique :

```text
document brut
  └─ décodage JSON ou YAML vers un modèle de données JSON
       └─ suppression récursive des clés réservées « x-architecture-* »
            └─ canonicalisation JSON (RFC 8785, JCS)
                 └─ SHA-256 sur les octets UTF-8
```

Règles :

- La canonicalisation trie les clés d'objet, supprime les espaces insignifiants et fixe la
  représentation des nombres et des chaînes selon RFC 8785. Deux documents YAML et JSON
  sémantiquement identiques produisent la même empreinte.
- Le namespace `x-architecture-*` est **réservé à Archifier** et exclu de l'empreinte, à
  tous les niveaux. C'est ce qui rend l'aller-retour export → réimport idempotent : les
  extensions ajoutées par l'export ne créent pas un nouveau contrat.
- Toutes les autres extensions `x-*` sont incluses dans l'empreinte. Deux documents qui
  diffèrent par une extension métier sont deux contrats différents.
- L'empreinte est une **identité de contenu**, pas une signature, pas une preuve
  d'authenticité et pas une autorisation. Une collision SHA-256 n'est pas traitée comme un
  risque opérationnel au MVP2.
- L'idempotence est évaluée par le couple `(contract_format, contract_hash)` à l'échelle
  du référentiel. Si l'empreinte est déjà connue, l'import renvoie l'interface et la version
  existantes (`OAS-050`) et ne crée rien. La réutilisation d'une interface par un autre
  domaine est une opération d'attachement par identifiant, pas un réimport.

### 5. Import : proposer, jamais publier

```text
lire le texte
  └─ détecter le format et la version
       └─ parser et normaliser vers contract_snapshot
            └─ calculer contract_hash
                 ├─ empreinte déjà connue → OAS-050, aucun changement
                 └─ empreinte nouvelle → proposition de version + diff
                      └─ confirmation utilisateur → écriture de gouvernance
```

- `serde_yaml_ng 0.10` est le parseur YAML approuvé en G1. JSON est traité par le parseur
  JSON du workspace ; si le contenu n'est pas du JSON strict, on tente YAML.
- Un import ne publie jamais une version, ne clôt jamais une revue et ne modifie jamais un
  statut (`CR-002`). Il produit une proposition qui passe par le verrou optimiste et la
  confirmation humaine (`BR-010`).
- `OAS-050` n'est pas une erreur : c'est un no-op idempotent. L'UI peut afficher
  « document déjà importé » et proposer l'interface et la version existantes.
- Une empreinte déjà connue ne crée ni interface, ni version, ni attachement. Pour réutiliser
  une interface dans un autre domaine, on attache l'interface par son identifiant, on ne
  réimporte pas son contrat.
- Les doublons de clés YAML sont refusés. En JSON, la règle du parseur est documentée et
  le brut reste la référence en cas d'ambiguïté.

### 6. Export : deux modes, deux garanties

| Mode | Source | Extensions `x-architecture-*` | Garantie |
|---|---|---|---|
| **Brut** | `contract_raw` | non ajoutées | identique octet à octet au document importé |
| **Normalisé** | `contract_snapshot` | ajoutées | identique octet à octet pour une même révision |

**Export brut.** Il restitue `contract_raw` sans reformatage. Il est refusé si le brut est
absent (snapshot MVP1) ; l'UI explique alors que seul l'export normalisé est disponible. Il
ne prétend pas ajouter les extensions d'Archifier : ce serait changer le document.

**Export normalisé.** Il est réservé aux interfaces éligibles (§8) et produit :

- un JSON UTF-8, clés triées lexicalement à tous les niveaux, indentation de 2 espaces,
  fins de ligne `LF`, saut de ligne final ;
- aucune date d'export, aucun nom d'hôte, aucun numéro de build, aucun identifiant de
  session ;
- les extensions `x-architecture-*` suivantes, valeurs issues du modèle :

| Extension | Valeur |
|---|---|
| `x-architecture-interface-id` | identifiant de l'interface |
| `x-architecture-version-id` | identifiant de la version |
| `x-architecture-domain-id` | domaine responsable |
| `x-architecture-logical-key` | clé logique de l'interface |
| `x-architecture-version` | version sémantique de l'interface |
| `x-architecture-generated-by` | constante `archifier`, sans version ni horodatage |

Deux exports normalisés de la même révision sont identiques, y compris sur deux machines
et deux builds différents. Toute donnée volatile qui entrerait dans le document est
considérée comme un défaut, pas comme une commodité.

Le modèle normalisé ne couvre que les champs connus. Les extensions `x-*` inconnues, les
commentaires et les constructions non modélisées ne sont garantis que par l'export brut.
L'UI ne doit donc pas présenter l'export normalisé comme une copie fidèle.

### 7. AsyncAPI : même contrat, sémantique événementielle

- `Interface.contract_type = Event`, `contract_format = AsyncApi2 | AsyncApi3`.
- Même conservation que OpenAPI : `contract_raw`, `contract_snapshot`, `contract_hash`.
- Le modèle normalisé couvre `info`, `servers`, `channels`, `operations`, `messages`,
  `schemas`, `security` et les bindings connus. Les bindings inconnus restent dans le brut.
- La détection de version lit `asyncapi`. Les erreurs sont `ASYNC-001` (version non
  supportée), `ASYNC-002` (document illisible ou ambigu) et `ASYNC-003` (identifiant de
  canal, d'opération ou de message dupliqué).
- Aucun canal, message ou schéma ne devient un objet du référentiel. La palette, la
  recherche d'objets et les relations n'en contiennent aucune trace.
- L'export normalisé est reproductible et porte les mêmes `x-architecture-*`. Il est refusé
  pour une interface qui n'est pas événementielle (`ASYNC-060`, code proposé par cet ADR).

### 8. Aucun faux document pour une interface non concernée

| Interface | OpenAPI | AsyncAPI |
|---|---|---|
| `Application` + `contract_type = OpenApi` | autorisé | refusé |
| `Application` ou `Technical` + `contract_type = Event` | refusé | autorisé |
| `Business`, quel que soit le contrat | refusé (`OAS-060`) | refusé (`ASYNC-060`) |
| `contract_type` `None` ou `Other` | refusé | refusé |
| `Technical` + `contract_type = OpenApi` | refusé (`OAS-060`) | refusé |

Un export refusé est un **résultat explicite**, pas un document vide, pas un document
synthétique et pas un fichier nommé `.yaml` sans contenu. L'UI désactive l'action et
explique la règle. C'est l'application directe de la spécification fonctionnelle : on ne
fabrique pas un faux OpenAPI pour une interface business ou un faux AsyncAPI pour une
interface qui n'est pas événementielle.

### 9. Diff et rupture

- Le diff est calculé sur les `contract_snapshot` normalisés, jamais sur le texte brut.
  Deux documents identiques mais formatés différemment ne produisent pas de faux diff.
- `OAS-001` à `OAS-003` couvrent les erreurs de format et les doublons d'`operationId` ;
  `OAS-010` à `OAS-014` classent les ruptures ; `OAS-020` et `OAS-021` les changements
  compatibles ; `OAS-030` et `OAS-031` les cas inconnus.
- Une rupture exige une confirmation explicite (`BR-010`, `OAS-040`). Elle n'est jamais
  appliquée parce que « le diff est vert ».
- Un `$ref` non résolu reste « inconnu » ; il n'est jamais traité comme compatible par
  défaut.
- Le diff AsyncAPI suit la même règle : suppression de canal ou de message = rupture ;
  ajout de champ optionnel = compatible ; `$ref` non résolu = inconnu.

### 10. Invariants vérifiés par des tests nommés

| Invariant | Test |
|---|---|
| OpenAPI 3.0, 3.1 et 3.2 sont détectés | `oas_import_3_0_3_1_3_2` |
| Le brut est conservé exactement | `oas_raw_document_is_preserved` |
| L'import est idempotent par empreinte | `oas_import_is_idempotent_by_fingerprint` |
| Une empreinte connue ne crée pas une seconde interface | `same_fingerprint_does_not_create_second_interface` |
| L'export normalisé est reproductible | `oas_export_is_byte_reproducible` |
| L'export est refusé hors HTTP applicatif | `oas_export_refuses_non_http_interface` |
| L'export brut est identique au brut importé | `oas_raw_export_is_byte_identical` |
| Les extensions Archifier ne changent pas l'empreinte | `contract_hash_ignores_archifier_extensions` |
| YAML et JSON du même document partagent l'empreinte | `yaml_and_json_share_fingerprint` |
| AsyncAPI 2.x et 3.x sont détectés | `asyncapi_version_is_detected` |
| AsyncAPI conserve le brut et exporte de façon reproductible | `asyncapi_import_preserves_raw`, `asyncapi_export_is_reproducible` |
| Un `$ref` externe n'est jamais récupéré | `external_ref_is_never_fetched` |
| Le brut n'est jamais interprété comme HTML | `contract_raw_is_never_rendered_as_html` |
| Un snapshot sans brut reste lisible | `snapshot_without_contract_raw_deserializes` |

## Conséquences

### Positives

- Un contrat importé peut être restitué tel quel, extensions et commentaires compris.
- L'idempotence ne dépend ni du formatage, ni du format JSON/YAML, ni des extensions
  ajoutées par Archifier.
- L'export normalisé est reproductible entre runs, machines et builds (`NFR-009`).
- Aucun faux document n'est produit pour une interface non HTTP ou non événementielle.
- L'import est hors ligne et sans `$ref` réseau, donc sans SSRF ni dépendance à un serveur
  distant.
- Les snapshots existants restent lisibles : les trois champs sont additifs.

### Négatives, acceptées

- Le snapshot grossit du texte brut complet ; le plafond de 2 Mio et le quota IndexedDB
  deviennent des limites d'usage réelles.
- Maintenir deux exports (brut et normalisé) et deux garanties est plus coûteux qu'un seul
  chemin, mais c'est la seule façon d'être à la fois fidèle et reproductible.
- L'export normalisé perd les extensions inconnues et les commentaires ; c'est documenté et
  l'export brut reste disponible.
- La canonicalisation RFC 8785 sur un document issu de YAML demande une conversion
  prudente des scalaires, des dates et des ancres ; le brut reste la référence en cas de
  doute.
- Refuser un BOM, un flux multi-documents ou une balise personnalisée est une rigidité
  assumée ; elle évite des empreintes ambiguës.
- `ASYNC-003` et `ASYNC-060` sont proposés par cet ADR et doivent être ratifiés avec le jeu
  de codes de la W5.

## Ce que cet ADR ne couvre pas

- La validation sémantique complète d'OpenAPI ou d'AsyncAPI, la génération de code, les
  serveurs mock et les tests de conformité.
- La résolution des `$ref` externes, les registres de schémas et les catalogues d'API.
- La signature, la provenance, la confiance et le scan de secrets dans les contrats.
- Les formats binaires (protobuf, Avro), GraphQL et les extensions propres à un fournisseur.
- Le versionnement sémantique des interfaces et la politique de rupture, couverts par
  ADR-019.
- La compression ou la déduplication du brut dans IndexedDB.
- L'import d'un contrat comme source d'objets de référentiel : les canaux et messages
  restent dans le document.

## Références

- `functional-specifications.md` — import/export OpenAPI, `FR-012`, `FR-020`, `FR-021`,
  `BR-010`, `NFR-009`, section « Statut d'ArchiMate 4 » pour la frontière des notations.
- `technical-specifications-lot2.md` §0.1 (dépendances approuvées), §10 (OpenAPI),
  §11 (AsyncAPI), décision D-06/D-07.
- `docs/backlog-mvp2.md` — T1-01 à T1-03, T4-02, contrat C-09.
- ADR-016 — écriture de gouvernance par `Repository`, jamais par une vue.
- ADR-019 — cycle de vie, dépréciation et politique de rupture.
