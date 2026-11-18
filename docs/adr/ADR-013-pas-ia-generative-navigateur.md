# ADR-013 — Pas de panneau IA générative dans le navigateur au lot 1

| | |
|---|---|
| Statut | **Accepté pour le lot 1**, maintenu au MVP2 (le proxy est préparé en W6) |
| Date | 17 novembre 2026 (rétro-ADR matérialisé le 18/11/2026) |
| Portée | Panneau IA, clés d'API, fournisseur de modèle |
| Décisions liées | ADR-017 (proxy LLM local), ADR-016 (port de dépôt) |

## Contexte

La spécification fonctionnelle demande un panneau conversationnel, des réponses sourcées
et des propositions de ChangeSet. Un appel direct à un fournisseur LLM depuis le navigateur
exigerait une clé d'API dans le bundle ou dans `localStorage`. Une clé embarquée est visible
par tout utilisateur ; une clé en `localStorage` est accessible à tout script de l'origine.

Le lot 1 est un front statique sans serveur. Il n'existe aucun endroit sûr pour héberger une
clé.

## Décision

- **Aucune clé d'API n'est présente dans le bundle, les sources, les fixtures ou
  `localStorage`.**
- Le panneau IA générative n'est **pas livré** au lot 1.
- Le moteur conserve les contrats nécessaires : `ChangeSet`, validation déterministe,
  confirmation explicite, analyse d'impact.
- Un `MockProvider` déterministe pourra être livré au MVP2 (T2-02) à condition d'annoncer
  qu'il s'agit de recettes scriptées, jamais d'un LLM.
- Un LLM réel passera exclusivement par `archifier-proxy` (ADR-017), binaire local, clé lue
  dans une variable d'environnement, désactivé par défaut.

## Conséquences

Positives :

- aucune fuite de secret par construction ;
- le produit reste honnête : l'absence d'IA est annoncée, pas simulée ;
- le chemin ChangeSet est déjà testé côté moteur.

Négatives, acceptées :

- pas de réponse générative au lot 1 ;
- l'utilisateur doit lancer un proxy local pour utiliser un LLM réel au MVP2 ;
- la recherche sémantique reste indisponible sans embeddings.

## Ce que cet ADR ne couvre pas

Le choix du fournisseur LLM, la journalisation, la modération, le coût, la conformité RGPD
et l'hébergement du proxy au lot 3.

## Références

- `README.md`, section « Limites assumées du lot 1 ».
- `docs/adr/ADR-017-proxy-llm-local.md`.
- `technical-specifications.md` §13.
