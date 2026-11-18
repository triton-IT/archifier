# ADR-017 — Proxy LLM local : optionnel, sans clé dans le bundle

| | |
|---|---|
| Statut | **Proposé pour W6** |
| Date | 18 novembre 2026 |
| Remplace | Aucun ADR ; précise l'invariant 3 de `technical-specifications-lot2.md` §0 et le point T2-04 du backlog |
| Portée | Frontière entre l'application statique et un fournisseur LLM ; binaire local uniquement |
| Décisions liées | ADR-016 (port de dépôt), ADR-018 (contrats externes) |

## Contexte

Le MVP2 doit pouvoir utiliser un LLM réel sans jamais embarquer de clé d'API dans
l'application statique. `MockProvider` reste le mode par défaut et suffit aux cinq recettes
annoncées ; le LLM réel est un **plus**, pas une dépendance de fonctionnement.

Trois contraintes du produit se conjuguent :

1. l'application est un bundle WASM statique, servi depuis un serveur de fichiers ; tout ce
   qui est dans `pkg/` est public ;
2. `NFR-013` interdit que les journaux contiennent des prompts sensibles par défaut ;
3. `NFR-011` limite le contexte envoyé au strict nécessaire.

Un appel direct du navigateur au fournisseur exposerait la clé à `pkg/`, à `localStorage`,
aux outils de développement et à toute extension de navigateur. Un serveur applicatif
central serait contraire à l'invariant « aucun serveur de données ». La seule voie
compatible est un **binaire local optionnel**, démarré explicitement par l'utilisateur,
qui détient la clé hors du navigateur.

Le proxy n'est pas un composant de sécurité contre un poste compromis : un processus local
malveillant peut lire l'environnement du même utilisateur. Il protège contre la fuite
**accidentelle** de la clé dans le dépôt, le bundle, les exports ou les sauvegardes.

## Décision

### 1. Un crate et un binaire séparés

| Élément | Décision |
|---|---|
| Nom | `archifier-proxy` |
| Forme | binaire Rust local, crate séparé (`crates/proxy`) |
| Port | **8787**, fixe |
| Dépendance de `archifier-ui` | **jamais** |
| Présence dans `pkg/` | **jamais** |
| Bind | `127.0.0.1` uniquement, jamais `0.0.0.0` |
| Démarrage | explicite, manuel ; aucun lancement automatique par l'UI |

Le port 8787 est fixe pour que la configuration de l'UI soit prévisible. Si le port est
occupé, le binaire échoue avec un message clair et un code de sortie non nul ; il ne
bascule pas silencieusement sur un autre port.

### 2. La clé vient de l'environnement, et de nulle part ailleurs

Nom canonique proposé : `ARCHIFIER_PROXY_API_KEY`.

Règles :

- la clé est lue **une fois au démarrage** depuis la variable d'environnement ;
- aucun argument de ligne de commande, aucun fichier de configuration, aucune valeur par
  défaut, aucun prompt interactif ;
- si la variable est absente ou vide, le binaire **refuse de démarrer** (code de sortie
  non nul, message qui ne contient pas la clé). L'UI traite l'absence de proxy comme le
  mode dégradé, pas comme une erreur bloquante ;
- la clé n'est jamais écrite sur disque, jamais incluse dans une réponse, jamais renvoyée
  au navigateur, jamais placée dans une URL ou un query string ;
- elle n'apparaît ni dans `Debug`, ni dans un message d'erreur, ni dans un journal ; le
  proxy ne journalise pas les en-têtes entrants ou sortants ;
- elle est utilisée uniquement pour construire l'en-tête d'authentification de l'appel
  sortant vers le fournisseur, côté serveur ;
- elle n'est jamais lue par `archifier-ui`, ni exposée par une variable `VITE_*`, ni
  stockée dans `localStorage`, `sessionStorage`, IndexedDB, le snapshot ou un export.

Aucun secret n'est requis pour utiliser le proxy depuis l'UI : le proxy est le seul
détenteur de la clé. Le fait qu'il écoute sur la boucle locale est la seule barrière
d'accès, et elle est explicitement insuffisante pour un déploiement distant.

### 3. CORS limité à l'origine de l'application

| Réglage | Valeur |
|---|---|
| Origine autorisée | `http://127.0.0.1:4173` **exactement** |
| Origine `http://localhost:4173` | refusée (origine distincte) |
| Origine absente | admise uniquement pour `GET /health`, jamais pour une complétion |
| Wildcard `*` | interdit |
| `Access-Control-Allow-Credentials` | absent ; aucun cookie, aucune session |
| Méthodes | `GET`, `POST`, `OPTIONS` |
| En-têtes autorisés | `content-type`, `accept` |
| Préflight | répondu, avec `Vary: Origin` |

Une requête d'une autre origine reçoit un refus explicite (403) sans en-tête CORS. Le
proxy ne « complète » jamais une origine par une autre : `localhost` et `127.0.0.1` sont
deux origines différentes, même si elles désignent la même machine. Si le serveur de
développement change de port, l'allowlist doit être modifiée par un changement explicite,
pas élargie à `*`.

Le bind loopback et l'allowlist CORS sont deux barrières indépendantes. Aucune des deux ne
suffit seule.

### 4. Aucun prompt journalisé par défaut

- Les journaux par défaut contiennent uniquement : horodatage, méthode, chemin, statut,
  latence, modèle demandé, nombre de jetons si le fournisseur le renvoie, identifiant de
  requête local.
- Les corps de requête et de réponse, les prompts, les complétions, les pièces jointes et
  les en-têtes d'authentification ne sont **jamais** journalisés.
- W6 n'implémente aucun mode qui journalise les prompts. Un futur mode de débogage
  explicite, local et documenté exigerait un ADR distinct ; il ne serait jamais actif par
  défaut.
- Les erreurs renvoyées à l'UI sont normalisées : code, message court, identifiant de
  requête. Elles ne recopient ni le prompt, ni la réponse du fournisseur, ni la clé.
- Le proxy est sans état : aucun historique de conversation, aucun cache de prompt, aucune
  base locale. Le flux de réponse est relayé, pas conservé.

### 5. Désactivé si absent, jamais requis

| Situation | Comportement |
|---|---|
| Proxy non démarré | mode `MockProvider`, bandeau « proxy local indisponible », aucune complétion réseau |
| Proxy démarré sans clé | refus de démarrer ; l'UI le voit comme absent |
| Proxy démarré avec clé | disponible après activation explicite de l'utilisateur |
| Proxy arrêté en cours de session | l'appel en cours échoue proprement ; retour au mode dégradé, sans perte de travail |

- L'UI ne contacte pas le proxy au démarrage. Elle ne fait aucune requête réseau tant que
  l'utilisateur n'a pas activé le mode LLM réel.
- À l'activation, une seule vérification `GET /health` est permise. En cas d'échec, l'UI
  affiche le mode dégradé et ne réessaie pas en boucle.
- Aucune télémétrie, aucun ping périodique, aucune requête de découverte réseau.
- Le proxy n'est jamais une condition de démarrage, de chargement ou d'export de
  l'application.

### 6. Surface réseau minimale

| Route | Rôle | Clé requise | Origine requise |
|---|---|---|---|
| `GET /health` | disponibilité et version du protocole | non | oui, sauf diagnostic local sans `Origin` |
| `POST /v1/chat` | complétion en streaming | oui, côté serveur | oui, allowlist exacte |

- Le corps de requête est limité (plafond proposé : 1 Mio) et le flux de réponse est
  relayé sans mise en mémoire intégrale.
- Un délai d'expiration borne chaque appel ; le proxy n'ouvre pas de connexion persistante
  vers le fournisseur au-delà de la requête.
- La requête de l'UI ne peut pas désigner une URL amont arbitraire : l'adresse du
  fournisseur est fixée par la configuration du proxy, jamais par le corps de la requête.
  Cela évite de transformer le proxy en relais SSRF.
- Le modèle demandé fait partie de la requête ; le proxy ne décide pas à la place de l'UI
  et ne propose pas de liste de modèles.

### 7. Interdictions vérifiées par des tests

| Interdiction | Test |
|---|---|
| Aucune clé dans le bundle | `proxy_reads_key_from_env`, `no_key_like_string_in_pkg` |
| Aucune clé dans `localStorage` / `sessionStorage` | `no_key_in_browser_storage` |
| CORS non élargi | `proxy_cors_is_restricted` |
| Aucun prompt dans les journaux | `proxy_does_not_log_prompts` |
| Absence du proxy = mode dégradé | `proxy_is_disabled_when_absent` |
| Le proxy n'est pas une dépendance de l'UI | `proxy_is_not_a_ui_dependency` |
| Aucune requête réseau au démarrage | `no_network_before_explicit_activation` |

Le test `no_key_like_string_in_pkg` cherche les motifs `sk-`, `Bearer ` et
`ARCHIFIER_PROXY_API_KEY` dans `pkg/`. Il ne remplace pas une revue, mais il attrape la
fuite la plus probable.

## Conséquences

### Positives

- La clé ne peut pas fuir par le bundle, un export ou une sauvegarde de l'application.
- Le mode par défaut reste entièrement local et fonctionnel sans proxy.
- Le proxy est un crate séparé : il n'alourdit ni le WASM, ni le temps de démarrage.
- La surface réseau est petite, loopback, à origine unique, et sans état.
- L'absence de journalisation des prompts respecte `NFR-013` par construction.

### Négatives, acceptées

- L'utilisateur doit démarrer un binaire et définir une variable d'environnement : ce
  n'est pas une expérience « clé en main ».
- L'origine exacte `127.0.0.1:4173` est rigide ; `localhost` ou un autre port sont refusés.
- L'absence de journal de prompt rend le diagnostic d'une réponse de mauvaise qualité plus
  difficile ; on préfère cette gêne à une fuite.
- Le proxy n'est pas un composant multi-utilisateur, ni un service distant, ni une
  frontière de sécurité contre un poste compromis.
- Le port fixe peut entrer en conflit avec un autre outil local ; l'échec est explicite,
  mais l'utilisateur doit libérer le port.

## Ce que cet ADR ne couvre pas

- Le choix du fournisseur, la liste des modèles, la qualité des réponses et la facturation.
- La rotation, la révocation et le stockage durable de la clé.
- Le déploiement distant, le TLS, l'authentification multi-utilisateur et le quota.
- La politique de contenu, le filtrage, la modération et la conformité des prompts.
- La journalisation d'audit des appels LLM (acteur, objet, décision).
- Le mode de débogage qui afficherait un prompt ; il exigerait un ADR dédié.
- Les embeddings, le RAG et tout stockage vectoriel.

## Références

- `functional-specifications.md` — `NFR-011`, `NFR-012`, `NFR-013`.
- `technical-specifications-lot2.md` §0 invariant 3 et §14.2 — proxy `archifier-proxy`.
- `docs/backlog-mvp2.md` — T2-04, porte G6, périmètre W6 `crates/proxy/**`.
- `prompts/mvp2-kickoff.md` §10 — interdiction de simuler une IA sans le dire.
- ADR-016 — port de dépôt et frontière UI / stockage.
