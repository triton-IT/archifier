# ADR-015 — Transport de synchronisation : local d'abord, relais préparé

| | |
|---|---|
| Statut | **Accepté pour le MVP2** |
| Date | 18 novembre 2026 |
| Remplace | La ligne « ADR-015 — Transport de synchronisation » du tableau §20 de `technical-specifications.md`, qui n'était qu'une intention ; matérialise le contrat C-03 |
| Portée | Frontière entre le moteur collaboratif (`crates/collab`) et le canal de diffusion ; aucun choix d'écran, aucune persistance |
| Décisions liées | ADR-014 (CRDT Yrs), ADR-016 (port de dépôt) |

## Contexte

Le MVP2 doit satisfaire deux exigences qui semblent opposées :

- `FR-036` : la synchronisation entre deux fenêtres d'un même navigateur est le mode
  nominal, sans serveur ;
- `FR-039` : le relais réseau doit être préparé derrière un contrat unique, activable sans
  modifier les écrans.

L'état vérifié au 18/11/2026 est explicite : `docs/adr/ADR-015-transport-synchronisation.md`
est absent, alors qu'ADR-015 est cité par le tableau §20 de `technical-specifications.md`
et attendu par la porte G1. Aucun code de transport n'existe.

Le risque à éviter est qu'un choix de medium (`BroadcastChannel`, WebSocket, canal de test)
remonte dans le moteur CRDT ou dans l'UI. Si le moteur connaît `BroadcastChannel`, le
relais réseau devient une refonte ; si l'UI connaît le WebSocket, le mode sans serveur
devient une option. Le transport doit donc être une frontière étroite, remplaçable, et
sans capacité métier.

Deux contraintes techniques bornent la décision :

- `NFR-017` interdit d'envoyer le document complet à chaque frappe ; le rattrapage doit
  être différentiel ;
- `BroadcastChannel` ne franchit ni l'origine, ni le profil de navigateur, ni la machine,
  et n'offre aucune frontière de sécurité. Ces limites sont assumées au MVP2.

## Décision

### 1. Un seul trait, trois implémentations

Le contrat C-03 de `technical-specifications-lot2.md` §5 fait foi. Il ajoute au brouillon
du §21.4 de `technical-specifications.md` l'identifiant d'abonnement et le désabonnement,
nécessaires pour qu'une vue puisse s'inscrire et se retirer sans fuite.

```rust
pub type PeerId = String;          // identifiant de session, jamais persisté
pub type DocId = String;           // "doc:<ULID>"
pub type SubscriptionId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Disconnected,
    Local,
    Connected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncMessage {
    Update { doc: DocId, from: PeerId, payload: Vec<u8> },
    StateVector { doc: DocId, from: PeerId, payload: Vec<u8> },
    Awareness { from: PeerId, payload: Vec<u8> },
}

pub trait SyncTransport {
    fn send(&self, message: SyncMessage) -> Result<(), SyncError>;
    fn subscribe(&self, on: Box<dyn Fn(SyncMessage) + 'static>) -> SubscriptionId;
    fn unsubscribe(&self, id: SubscriptionId);
    fn peers(&self) -> Vec<PeerId>;
    fn status(&self) -> SyncStatus;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SyncError {
    #[error("transport indisponible")]
    Unavailable,
    #[error("message trop volumineux : {0} octets")]
    TooLarge(usize),
    #[error("encodage invalide")]
    Encoding,
    #[error("transport fermé")]
    Closed,
}
```

Règles du trait :

- `send` ne bloque jamais l'UI et ne rend jamais une écriture locale dépendante du réseau.
  Un échec de `send` est signalé à l'appelant, mais ne défait jamais l'édition déjà
  appliquée au document CRDT.
- Une implémentation peut mettre en file un envoi ; la file est bornée. Un dépassement
  produit `SyncError::Unavailable`, jamais une croissance illimitée en mémoire.
- `subscribe` rend un identifiant stable. `unsubscribe` est idempotent : un identifiant
  inconnu est ignoré sans erreur.
- Un callback n'est jamais invoqué de façon réentrante depuis `send`, et aucun verrou
  interne n'est tenu pendant son exécution. Une exception ou un panic dans un abonné ne
  doit pas rendre le transport inutilisable pour les autres.
- `peers()` est une information **best effort** pour la présence ; ce n'est pas un registre
  d'autorisation. Il peut être vide alors que `status()` vaut `Local`.
- `status()` vaut `Local` pour `BroadcastChannel`, `Connected` pour un relais réellement
  ouvert, `Disconnected` sinon.

### 2. Trois messages, aucun message de gouvernance

| Variante | `doc` | Contenu du `payload` | Persisté ? |
|---|---|---|---|
| `Update` | requis | encodage binaire Yrs v1, différentiel | oui, dans le store `collab` |
| `StateVector` | requis | vecteur d'état Yrs v1 | non, éphémère |
| `Awareness` | absent | JSON UTF-8 de présence (§6) | jamais (`CR-004`) |

- Le transport ne décode **pas** le `payload`. Il transporte une enveloppe et des octets ;
  le codec Yrs reste dans `crates/collab`. C'est ce qui permet de tester le transport sans
  CRDT et de changer d'implémentation sans changer d'encodage.
- Aucune donnée de gouvernance ne transite par ce canal (`CR-001`) : pas de version, pas
  de contrat, pas de statut de revue, pas de relation. Le seul contenu admis est celui du
  §3 de `technical-specifications-lot2.md`.
- `from` est le `PeerId` de la session émettrice. Il sert à ignorer son propre écho et à
  attribuer une présence ; il n'est jamais persisté, exporté ni projeté dans le snapshot.

### 3. Enveloppe binaire versionnée

Tous les messages sont encodés dans une enveloppe binaire. Un canal ne doit jamais
interpréter un payload métier comme une structure de contrôle.

```text
magic      : 4 octets  "AFR1"
kind       : u8        1 = Update, 2 = StateVector, 3 = Awareness
doc_len    : u16 LE    longueur de doc_id en octets UTF-8 (0 pour Awareness)
doc        : doc_len octets
from_len   : u16 LE    longueur de peer_id en octets UTF-8
from       : from_len octets
payload_len: u32 LE
payload    : payload_len octets
```

Règles de validation, toutes sans panic :

- `magic` différent de `AFR1` ⇒ `SyncError::Encoding` ; un magic inconnu n'est jamais
  interprété comme une version future tolérante.
- `kind` inconnu ⇒ `SyncError::Encoding`. On échoue fermé, on n'ignore pas silencieusement.
- `doc_len = 0` est refusé pour `Update` et `StateVector` ; `doc` et `from` doivent être de
  l'UTF-8 valide. `from_len = 0` est refusé.
- Toute longueur qui déborde du tampon, tout octet final non consommé et tout dépassement
  d'entier ⇒ `SyncError::Encoding`.
- Limite dure : **1 Mio par message**. Au-delà ⇒ `SyncError::TooLarge(taille)`. Un `Update`
  de plus de 64 Kio est journalisé en mode debug, sans être refusé avant la limite dure.
- Aucune compression ni fragmentation au MVP2. Une évolution incompatible changerait le
  magic (`AFR2`), pas la sémantique d'un champ existant.
- Le décodage est déterministe et sans allocation proportionnelle à une longueur annoncée
  mais absente du tampon.

### 4. `LocalBroadcastTransport` : le mode nominal

| Caractéristique | Valeur |
|---|---|
| Canal | `BroadcastChannel("archifier:collab:v1")` |
| Encodage | enveloppe binaire ci-dessus, `ArrayBuffer` / `Uint8Array` |
| `status()` | `Local` si le canal est ouvert, `Disconnected` sinon |
| Portée | même origine, même profil de navigateur, même machine |
| Sécurité | aucune frontière ; le canal est lisible par tout script de la même origine |

Règles :

- Le nom versionné `archifier:collab:v1` est le nom **canonique**. ADR-014 §3 cite la forme
  historique `archifier:collab` ; son amendement C-03 (18/11/2026) et le §5.2 du lot 2
  fixent `:v1`. `technical-specifications.md` §21.4 conserve encore la forme non versionnée.
  Le présent ADR confirme `archifier:collab:v1` comme seule constante de canal.
- Seuls des messages binaires sont postés. Aucun objet structuré, aucun `Dataset`, aucun
  `ChangeSet`.
- L'émetteur ignore tout message dont `from` est son propre `PeerId`. `BroadcastChannel`
  ne renvoie normalement pas à l'émetteur, mais le transport ne dépend pas de cette
  garantie.
- Si `BroadcastChannel` est absent ou si le constructeur échoue, le transport passe en
  `Disconnected`. L'édition locale continue ; l'UI affiche « synchronisation inactive ».
  Aucun repli réseau n'est tenté.
- Le canal n'est ni une file durable, ni un journal : un message émis sans abonné est
  perdu. Le rattrapage par vecteur d'état (§7) est la seule compensation admise.
- `peers()` est dérivé des `Awareness` récents et des `from` observés, avec expiration.
  Aucun protocole de jointure ou de départ distinct n'est ajouté.
- L'UI affiche les limites : même origine, même profil, aucune sécurité. On ne présente
  jamais ce mode comme un partage distant.

### 5. `InProcessTransport` : les tests, pas la production

`InProcessTransport` partage un bus en mémoire entre plusieurs instances et permet de
contrôler :

- l'ordre d'arrivée, y compris l'inversion et la duplication ;
- la perte de messages ;
- la partition et la reconnexion d'un pair ;
- le retard, sans attente murale : les tests avancent par étapes explicites ;
- l'expiration de présence.

Il sert aux tests de convergence, d'indépendance à l'ordre (`CR-006`), de rattrapage, de
partitions et de reprise hors ligne. Il n'est pas exposé à l'UI et ne constitue pas un
mode utilisateur. Ses aides de test (partition, perte, horloge) restent derrière
`#[cfg(test)]` ou une feature de test, jamais dans le chemin WASM de production.

### 6. `WebSocketTransport` : préparé, non activé

- Le transport dépend d'un `RelaySocket` injecté (trait), pas d'une URL codée en dur. Le
  relais réel, l'authentification et la reconnexion sont du lot 3.
- Il utilise la **même** enveloppe, les mêmes `SyncMessage` et le même trait. Le remplacement
  de `LocalBroadcastTransport` ne change ni le moteur, ni la projection, ni les écrans.
- `status()` ne vaut `Connected` que si la socket est ouverte et la poignée de main
  terminée ; sinon `Disconnected`.
- Aucun chemin UI ne l'instancie. Aucun drapeau d'activation n'est exposé au MVP2. Un test
  d'architecture vérifie que `crates/ui` ne le mentionne pas.
- Les tests du MVP2 l'exercent contre un relais en mémoire pour prouver que le contrat est
  réellement médium-agnostique. Une socket réelle n'est pas nécessaire à la porte G4a.

### 7. Rattrapage par vecteur d'état

```text
ouverture d'un document éditable
  └─ le pair envoie StateVector { doc, from, payload }
       └─ chaque pair qui possède le document répond
            Update { doc, from, payload = delta depuis le vecteur reçu }
                 └─ si le delta est vide, aucune réponse n'est envoyée
```

Règles :

1. Un `StateVector` ne reçoit jamais un autre `StateVector` en réponse : pas de boucle.
2. Les réponses sont coalescées par document et par tour de boucle ; un pair ne répond
   pas deux fois au même vecteur.
3. Un `Update` ne renvoie jamais le document complet à chaque frappe. La taille d'un lot
   reste bornée par `NFR-017`.
4. En cas de partition, les mises à jour locales restent dans le document et dans le store
   `collab`. À la reconnexion, l'échange de vecteurs suffit ; aucun journal de messages
   n'est rejoué.
5. Un `Update` appliqué deux fois est sans effet supplémentaire : l'idempotence vient du
   CRDT, pas du transport.
6. La présence n'est pas rattrapée. Un pair qui arrive attend le prochain battement
   (`Awareness` toutes les 10 s) ; il ne rejoue pas l'historique de présence.
7. Si le delta nécessaire dépasse 1 Mio, le transport rend `TooLarge`. Le message n'est ni
   tronqué ni fragmenté : l'UI signale que la synchronisation de ce document est impossible
   et le document local reste intact. Un découpage futur exigera une évolution d'enveloppe.

### 8. Modes dégradés

| Situation | Comportement | Ce qui est préservé |
|---|---|---|
| `BroadcastChannel` absent | `Disconnected`, bandeau, édition locale seule | document local, flush IndexedDB |
| `send` échoue | erreur rendue à l'appelant, aucun rollback | édition locale, état CRDT |
| message illisible | message ignoré avec diagnostic de transport, jamais de panic | autres pairs, document local |
| pair en retard | rattrapage par `StateVector` | convergence, sans blocage UI |
| document trop volumineux | `TooLarge`, message explicite | document local, aucune perte silencieuse |
| relais non activé | aucun chemin réseau | mode nominal local |

Un transport indisponible ne bloque jamais l'édition locale. C'est une exigence produit,
pas une tolérance d'implémentation.

### 9. Invariants vérifiés par des tests nommés

| Invariant | Test |
|---|---|
| Le transport ne connaît ni gouvernance ni structure du payload | `sync_transport_is_media_agnostic` |
| L'enveloppe fait un aller-retour exact | `envelope_roundtrip_is_exact` |
| Les magics, types et longueurs invalides sont refusés sans panic | `envelope_rejects_invalid_magic_and_lengths` |
| Deux fenêtres convergent par BroadcastChannel | `t7_02_two_windows_converge` |
| Le rattrapage par vecteur d'état ne renvoie que le delta | `t7_02_state_vector_catch_up` |
| Un `Update` reste borné | `t7_02_update_payload_is_bounded` |
| Le WebSocket respecte le même contrat | `ws_transport_uses_same_contract` |
| Le WebSocket n'est pas câblé dans l'UI | `ws_transport_is_not_wired_in_ui` |
| La présence n'est jamais persistée | `presence_is_never_persisted` |
| Un transport indisponible ne bloque pas l'édition | `transport_unavailable_does_not_block_local_edit` |

## Conséquences

### Positives

- La collaboration réelle entre deux fenêtres est disponible sans serveur, donc
  démontrable dès le MVP2 (`FR-036`).
- Le relais réseau est un ajout d'implémentation derrière un contrat déjà testé, pas une
  refonte du moteur ni des écrans (`FR-039`).
- Le moteur CRDT ne dépend d'aucune API de navigateur ; il reste testable en natif.
- Les modes dégradés sont explicites et n'entraînent jamais de perte d'édition locale.

### Négatives, acceptées

- `BroadcastChannel` ne franchit ni le profil, ni la machine, ni une frontière de sécurité.
- Le canal ne garantit ni l'ordre, ni la livraison, ni la durabilité ; le CRDT et le
  rattrapage compensent, mais un message perdu retarde la convergence jusqu'au prochain
  échange de vecteurs.
- Le plafond de 1 Mio rend un très gros delta impossible à rattraper en un message ; le
  comportement est explicite, mais imparfait.
- Le nom de canal versionné diverge de la forme historique d'ADR-014 §3 (amendée depuis)
  et de `technical-specifications.md` §21.4 (non aligné) ; le présent ADR tranche en faveur
  de `:v1`.
- `WebSocketTransport` non activé est du code testé mais non observé en production : il
  peut diverger du futur relais réel.

## Ce que cet ADR ne couvre pas

- L'implémentation et l'exploitation du relais réseau, son déploiement et sa supervision.
- L'authentification, l'autorisation et la révocation d'un pair sur le relais.
- Le chiffrement de bout en bout et la conformité RGPD du contenu partagé.
- Le partage entre profils de navigateur ou machines différentes.
- La politique de présence (battement, expiration, rendu), couverte par le contrat C-04.
- La fragmentation, la compression et le transport de documents dépassant la limite dure.
- Les garanties de livraison, d'ordre ou de reprise après crash du canal.

## Références

- `functional-specifications.md` — section « Collaboration temps réel (MVP2) », `FR-035`,
  `FR-036`, `FR-039`, `NFR-015` à `NFR-017`, `CR-001`, `CR-004`, `CR-006`.
- `technical-specifications.md` §20 (ligne ADR-015) et §21.4 (transport).
- `technical-specifications-lot2.md` §5 — contrat C-03 `SyncTransport`.
- `docs/backlog-mvp2.md` — T7-02, T7-07, décision O-05.
- ADR-014 — édition collaborative temps réel, CRDT Yrs.
- ADR-016 — port de dépôt et racine de composition.
