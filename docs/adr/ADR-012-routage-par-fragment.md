# ADR-012 — Routage par fragment, sans dépendance de routeur

| | |
|---|---|
| Statut | **Accepté pour le lot 1**, maintenu au MVP2 |
| Date | 17 novembre 2026 (rétro-ADR matérialisé le 18/11/2026) |
| Portée | Navigation de l'interface |
| Décisions liées | ADR-011 (persistance locale), ADR-016 (port de dépôt) |

## Contexte

La spécification technique prévoyait `leptos_router` en mode history, avec un repli SPA
côté serveur. Le lot 1 est un bundle statique servi par n'importe quel serveur de fichiers :
il n'existe pas de serveur applicatif pour réécrire les URL inconnues vers `index.html`.
Une URL profonde `https://…/view/elements/<id>` renverrait donc 404.

Le produit doit rester servable depuis un simple répertoire, y compris `file://` ou un
serveur statique minimal.

## Décision

- Le routage est porté par le **fragment** `#/...`.
- `crates/ui/src/router.rs` définit `Route`, `parse_hash` et `to_hash` ; aucune dépendance
  de routage externe.
- Les liens internes utilisent `#/view/<type>/<id>` ; les exports Markdown utilisent des
  liens relatifs vers des fichiers générés, pas des routes applicatives.
- L'écoute de `hashchange` est installée dans `App::new`.
- Une route inconnue affiche un écran « page introuvable » avec retour aux domaines.

## Conséquences

Positives :

- aucun serveur à configurer, aucun repli SPA ;
- le bundle est servable par `node scripts/serve.mjs` ou tout serveur statique ;
- les liens sont copiables et survivent au rechargement.

Négatives, acceptées :

- les URL ne sont pas « propres » pour le SEO et le partage social ;
- le fragment n'est pas envoyé au serveur, donc pas de rendu serveur par route ;
- les liens `arch://` doivent être résolus vers un fragment, pas vers une URL HTTP.

## Ce que cet ADR ne couvre pas

Le référencement, le rendu serveur, les redirections 301 et les liens profonds sans
fragment.

## Références

- `crates/ui/src/router.rs`.
- `technical-specifications-lot1-front-local.md` §13, ADR-012.
