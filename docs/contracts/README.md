# Contrats G0 — Atelier de vues (MVP3)

| | |
|---|---|
| **Document** | Registre des contrats gelés à la porte G0 |
| **Version** | 1.0 — 18 novembre 2026 |
| **Statut** | **FIGÉ — G0** (signé par l'orchestrateur) |
| **Entrées** | ADR-022, `functional-specifications.md` (FR-040..054, NFR-021..030), `technical-specifications.md` (§14.4, §15.0..15.6) |
| **Sortie** | 9 contrats opposables, référencés par les vagues W1 → W6 |

## Règle de gel

Un contrat de ce dossier est **opposable**. Un subagent ne le modifie pas : il signale
l'écart dans son rapport de porte. Toute évolution passe par un ADR et une révision
mineure du contrat (`1.0` → `1.1`) qui liste les champs ajoutés et les tests ajoutés.

Aucun contrat ne peut **retirer** un champ ou renommer une variante entre deux vagues :
seules les évolutions **additives** sont autorisées, avec valeur par défaut.

## Les neuf contrats

| Réf. | Contrat | Fichier | Propriétaire d'écriture |
|---|---|---|---|
| **C-01** | `ViewCommand` : énumération, champs, idempotence, plan d'effets, codes | [`C-01-view-command.md`](C-01-view-command.md) | Dev Atelier (core) |
| **C-02** | `DiagramLayout` schéma 2 : champs, défauts, migration depuis le schéma 1 | [`C-02-layout-schema-2.md`](C-02-layout-schema-2.md) | Dev Atelier (core) |
| **C-03** | `ViewIntent` : 7 variantes et leur configuration | [`C-03-view-intent.md`](C-03-view-intent.md) | UX + orchestrateur |
| **C-04** | `DraftStatus` : `SKETCH \| FORMALIZED \| VALID` et transitions | [`C-04-draft-status.md`](C-04-draft-status.md) | Dev Inspecteur |
| **C-05** | `ViewTemplate` : schéma JSON versionné d'un modèle de vue | [`C-05-view-template.md`](C-05-view-template.md) | UX |
| **C-06** | Table des routes, alias, paramètres de lien partagé | [`C-06-routes.md`](C-06-routes.md) | Dev Navigation |
| **C-07** | Tokens : bandes, zones, frontières, ports, temporalité, statuts | [`C-07-tokens.md`](C-07-tokens.md) | UX |
| **C-08** | Codes `DSG-001..005`, `REV-409`, codes réutilisés | [`C-08-codes.md`](C-08-codes.md) | Orchestrateur |
| **C-09** | Surface `Repository` : gel + évolutions additives approuvées | [`C-09-repository-surface.md`](C-09-repository-surface.md) | Orchestrateur |

## Ce que ces contrats ne couvrent pas

- L'algorithme de disposition automatique, le routage avancé des arêtes (hors MVP3).
- La collaboration temps réel sur le layout (ADR-014) : C-01 prépare l'idempotence,
  elle ne l'active pas.
- La charte de marque, les polices, les icônes : C-07 fixe les **noms** de tokens,
  pas un système graphique complet.
- La migration des données existantes : aucune migration destructive n'est autorisée
  (C-02 décrit une migration additive et idempotente).

## Vérification de conformité (porte G0)

```powershell
cargo test --workspace
cargo check -p archifier-ui --target wasm32-unknown-unknown
```

À G0, ces commandes doivent rester vertes **sans modification de code** : les contrats
sont des documents, pas encore du code. Les tests squelettes sont livrés en W0 par le
testeur, en `#[ignore]` explicite tant que le code n'existe pas, puis dé-ignorés vague
par vague. Un test squelette ne prouve rien : il réserve le nom.

## Journal de gel

| Date | Révision | Décision |
|---|---|---|
| 18/11/2026 | 1.0 | Création des neuf contrats, alignés sur ADR-022 et §14.4/§15 |
