# Fixtures W1c-1

Instantanés `Dataset` complets, chargés par `Repository::from_json` avec une
horloge figée (`NOW_MS = 1_795_000_000_000`, 18 novembre 2026).

| Fichier | Objet | Tests |
|---|---|---|
| `hierarchy.json` | Chaîne BPMN de 5 arêtes (racine 0, nœud profond 5), parent archivé + enfant feuille, branche `HIER-005`, Native/Grouping exclus, composants ArchiMate/UML, diagramme non hiérarchisable | `crates/repo/tests/hierarchy_acceptance.rs` |
| `story-framing.json` | Trois récits : cadrage `zoom 2.0` + focus + dim (opacité 0.35), épinglage objets + version d'interface, référence sans instantané (`STORY-003`) | `crates/repo/tests/story_acceptance.rs` |
| `reviews.json` | Trois revues (deux domaines + une même domaine à fin seule), filtres distincts, `Shared API` expirée à deux dates pour distinguer les anomalies par `detail` | `crates/repo/tests/review_compare_acceptance.rs` |

Points d'attention :

- Les instantanés de révision de `story-framing.json` sont minimaux : seul leur
  **existence** est requise par §7.3. `Pricing Service` (`…402`) n'en a pas,
  volontairement, pour le cas `STORY-003`.
- `Payment API` a trois versions : `1.0.0` publiée (explicite dans un bloc),
  `2.0.0` publiée (défaut du bloc sans version) et `3.0.0` brouillon (jamais
  épinglée).
- Les identifiants sont des ULID valides de 26 caractères. Aucune validation
  métier n'est appliquée au chargement : la fixture peut porter un parent
  archivé, une chaîne de profondeur 5 ou des éléments `NATIVE`/`Grouping`.

Plan de test détaillé : `docs/w1c-test-plan.md`.
