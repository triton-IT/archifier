# Backlog fonctionnel MVP1 — Archifier v0.1

| | |
|---|---|
| **Document** | Backlog de référence pour l'implémentation |
| **Version** | 1.0 |
| **Date** | 17 novembre 2026 |
| **Entrée** | `functional-specifications.md`, `technical-specifications.md`, `technical-specifications-lot1-front-local.md` |
| **Périmètre** | Lot 1 : front complet, back simulé en IndexedDB |
| **Volume** | **81 points fonctionnels**, 30 exigences `FR-*`, 14 exigences `NFR-*` |

### État au 17/11/2026 (livraison du lot 1)

Synthèse honnête de ce qui est effectivement livré et testé, par lot. La colonne
`État` ligne à ligne reste à tenir à jour pendant la phase d'orchestration.

| Lot | Livré | Partiel | Non livré |
|---|---|---|---|
| F0 socle | A-02, A-04, A-05, P-05 | A-01 (identités seedées, pas de conflit inter-onglets), A-03 (révisions oui, dialogue de conflit non), A-06 | — |
| F1 canvas | C-01→C-05, D-01, D-02, F-01, F-03→F-06 | C-06 (parent/enfant non exposé), F-02 (relations par listes, pas par tirage) | — |
| F2 fiches | G-01→G-04, J-02, O-01, O-03 | B-02 (archivage de domaine incomplet), O-02 (snapshots stockés, diff non rendu) | — |
| F3 visualiser | B-01→B-04, D-03, E-01→E-07, J-01, J-03, J-05 | E-08 (comparaison non rendue), E-09 (filtres d'interface incomplets), J-04 (pas de sélecteur de date T) | D-04 (graphe dédié) |
| F4 revues/stories | H-03, I-01, I-03, I-04 | H-01/H-05 (lecteur seulement, pas d'éditeur), H-02 (cadrage partiel), H-04, I-02 (7 vues sur 9, entrantes/sortantes manquantes), I-05 | — |
| F5 recherche/IA | K-03, K-04 | K-01 (index incomplet : descriptions/contrats), K-02 (filtres domaine/notation/catégorie absents) | L-01→L-07 (ADR-013) |
| F6 export | P-01 | G-05 (comptage des liens cassés, page de diagnostic absente) | M-01→M-04, N-01→N-03 |
| F7 robustesse | — | P-02 | P-03, P-04 |

> **Révision W0 (18/11/2026).** Cette synthèse corrige des mentions « livré » surévaluées
> constatées par l'audit qualité. Les points partiels restent dans le périmètre W1/W5 ;
> aucun code n'a été retiré. `P-01` est désormais couvert par un test de persistance de
> l'import (`crates/repo/tests/storage_boundary.rs`).

**72 tests automatisés du lot 1** couvrent le moteur et les scénarios d'acceptation
(`cargo test --workspace`) ; W0 ajoute 11 tests (spike CRDT, frontière de stockage,
architecture) pour un total de 83. Tous les codes `BR-*`, `OBJ-*`, `MM-*`, `CS-*`, `STORY-002`
cités ci-dessus ont au moins un test dédié.

## Comment ce backlog est utilisé
- Chaque ligne est un **comportement observable et testable**, pas une tâche technique.
- Un point n'est `✅ fait` que si : le code existe, **au moins un test automatisé** le couvre, et aucun état `todo!`/`unimplemented!` ne subsiste dans son périmètre.
- **Seul l'orchestrateur modifie la colonne `État`.** Un agent signale l'avancement dans son rapport de fin de lot ; il ne réécrit pas ce fichier.
- Un point qui révèle une ambiguïté de spécification est marqué `⛔` avec la question posée en marge du lot, jamais tranché silencieusement par un agent.
- Les lots `F0` à `F7` sont décrits dans l'addendum lot 1, §7.

### Légende

| Symbole | État |
|---|---|
| ⬜ | à faire |
| 🟦 | en cours |
| ✅ | fait et testé |
| ⛔ | bloqué (ambiguïté ou dépendance) |

### Décisions actives

| Décision | Valeur retenue |
|---|---|
| Persistance | **IndexedDB** (transactions ACID, quota large, historique complet) |
| Historique | **complet**, aucune troncature (NFR-002 respecté) |
| Front | Rust / Leptos CSR → WASM |
| Back | simulé derrière `RepositoryPort`, aucune API HTTP au lot 1 |
| IA | `MockProvider` déterministe, outils réellement exécutés, zéro clé navigateur |
| Équipe | orchestrateur + analyste + UX/design + 3 devs + testeur + relecteur |

---

## A. Socle et identité — lot F0

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| A-01 | Comptes locaux simulés avec sélecteur d'utilisateur, pour attribuer chaque écriture à un auteur nommé. | — | F0 | ⬜ |
| A-02 | Identifiants ULID opaques et stables, indépendants du nom, de la notation, du domaine et des représentations. | FR-004 | F0 | ⬜ |
| A-03 | Verrou optimiste par révision, avec dialogue recharger / comparer / forcer en cas de conflit. | — | F0 | ⬜ |
| A-04 | Archivage au lieu de suppression, l'objet reste résolvable et passe en lecture seule. | — | F0 | ⬜ |
| A-05 | Bascule MODÉLISER / VISUALISER permanente conservant l'objet courant, sans différence de droits. | FR-015 | F0 | ⬜ |
| A-06 | Bandeau « démonstrateur local » indiquant que les données restent dans le navigateur. | — | F0 | ⬜ |

## B. Domaines

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| B-01 | Création, renommage, description Markdown et archivage des domaines. | FR-001 | F2 | ⬜ |
| B-02 | Archivage refusé avec la liste des objets bloquants si des objets actifs dépendent du domaine. | FR-001 | F2 | ⬜ |
| B-03 | Page domaine VISUALISER : diagrammes publiés, interfaces exposées et consommées, dépendances, Stories, compteurs. | FR-001 | F3 | ⬜ |
| B-04 | Domaine responsable obligatoire sur chaque élément, relation, diagramme et interface, affiché partout. | FR-002 | F2 | ⬜ |

## C. Référentiel multi-notation

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| C-01 | Palette ArchiMate 3.2 de 42 classes couvrant métier, applicatif, technologique, physique, motivation et migration. | FR-003 | F1 | ⬜ |
| C-02 | Palette BPMN 2.0 Core : processus, participants, couloirs, activités, événements, gateways, flux, données, annotations. | FR-003 | F1 | ⬜ |
| C-03 | Palette UML 2.5.1 statique : composants, classes, nœuds, artefacts, acteurs, cas d'usage. | FR-003 | F1 | ⬜ |
| C-04 | Validation de métamodèle bloquante, avec code explicite pour toute classe ou relation hors du sous-ensemble annoncé. | FR-003 | F1 | ⬜ |
| C-05 | Propriétés sémantiques typées déclarées par le profil, les clés inconnues étant refusées. | — | F1 | ⬜ |
| C-06 | Hiérarchie parent/enfant par notation, profondeur limitée à 5, cycles interdits. | — | F1 | ⬜ |

## D. Relations et graphe

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| D-01 | Relations intra-notation : composition, agrégation, affectation, réalisation, service, accès, influence, déclenchement, flux, association, spécialisation. | FR-003 | F1 | ⬜ |
| D-02 | Relations transverses natives (REFINES, IMPLEMENTS, REALIZES, DESCRIBES, EXPOSES, CONSUMES, TRACE_TO) sans jamais fusionner deux objets. | FR-003 | F1 | ⬜ |
| D-03 | Dépendances entrantes et sortantes par parcours borné de 1 à 6 sauts, 3 par défaut, avec chemins affichés. | FR-011 | F3 | ⬜ |
| D-04 | Graphe navigable des dépendances d'un élément ou d'une interface, cliquable de proche en proche. | FR-011 | F3 | ⬜ |

## E. Interfaces et temporalité

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| E-01 | Interface objet de premier rang : identité, clé logique stable, catégorie BUSINESS/APPLICATION/TECHNICAL, domaine fournisseur. | FR-007 | F3 | ⬜ |
| E-02 | Versions d'interface en SemVer avec statut éditorial DRAFT/PUBLISHED/WITHDRAWN distinct du statut temporel. | FR-008 | F3 | ⬜ |
| E-03 | Publication avec fenêtre de disponibilité en intervalle semi-ouvert, refusée et motivée (BR-002) si la fenêtre est invalide. | FR-008 | F3 | ⬜ |
| E-04 | Statut temporel calculé SCHEDULED / ACTIVE / EXPIRED selon la date, jamais stocké en double. | FR-029 | F3 | ⬜ |
| E-05 | Versions parallèles autorisées pendant une migration, avec période de chevauchement mise en évidence. | FR-008 | F3 | ⬜ |
| E-06 | Déclaration de consommation d'une version précise par un domaine, un élément ou une interface, sans copie. | FR-009 | F3 | ⬜ |
| E-07 | Frise temporelle par version avec repère « maintenant » et chevauchements hachurés. | FR-008 | F3 | ⬜ |
| E-08 | Comparaison de deux versions avec diff classé compatible / rupture / inconnu, la rupture restant à confirmer. | FR-012 | F3 | ⬜ |
| E-09 | Recherche et filtres d'interfaces par domaine, catégorie, état, consommateur, version et fenêtre temporelle. | FR-010 | F3 | ⬜ |

## F. Diagrammes

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| F-01 | Canvas avec grille, pan, zoom, sélection, déplacement et cadrage automatique. | — | F1 | ⬜ |
| F-02 | Création de relations par tirage, avec types proposés filtrés par le métamodèle chargé. | FR-003 | F1 | ⬜ |
| F-03 | Occurrences multiples d'un même objet dans un ou plusieurs diagrammes, sans duplication de l'objet. | FR-005 | F1 | ⬜ |
| F-04 | Référence inter-domaines : un diagramme de B place l'élément ou l'interface de A, et le domaine affiché reste A. | FR-006 | F1 | ⬜ |
| F-05 | Recherche du référentiel avant création pour proposer les objets existants sans bloquer l'utilisateur. | FR-005 | F1 | ⬜ |
| F-06 | Persistance du layout (positions, dimensions, waypoints, styles limités, annotations) en une écriture au drop. | — | F1 | ⬜ |

## G. Documentation et liens

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| G-01 | Documentation Markdown sur tout objet : domaine, élément, relation, interface, diagramme, Story, revue. | FR-016 | F2 | ⬜ |
| G-02 | Éditeur Markdown avec aperçu, CommonMark étendu, HTML filtré par liste blanche. | FR-016 | F2 | ⬜ |
| G-03 | Liens internes typés `arch://` en forme canonique par ID, résolus vers une URL applicative. | FR-017 | F2 | ⬜ |
| G-04 | Blocs documentaires versionnés, chaque enregistrement créant une révision consultable. | FR-016 | F2 | ⬜ |
| G-05 | Diagnostic des liens cassés avec extrait de contexte et accès au propriétaire du lien. | FR-027 | F6 | ⬜ |

## H. Stories

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| H-01 | Composition par chapitres et blocs Markdown, diagramme, vue, élément, interface, transition. | FR-018 | F4 | ⬜ |
| H-02 | Cadrage par bloc de diagramme : zoom, éléments en avant, éléments atténués, légende, sans modifier la source. | FR-018 | F4 | ⬜ |
| H-03 | Lecteur avec progression, sommaire, transitions qui ouvrent le bloc cible et surlignent l'élément. | FR-018 | F4 | ⬜ |
| H-04 | Références par ID avec épinglage optionnel d'une révision pour figer un récit historique. | FR-018 | F4 | ⬜ |
| H-05 | Story éditable en MODÉLISER et jouable en VISUALISER. | FR-018 | F4 | ⬜ |

## I. Revues

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| I-01 | Revue pérenne : titre, domaine, fenêtre temporelle, photographie de filtres, auteur, statut, résumé Markdown. | FR-013 | F4 | ⬜ |
| I-02 | Neuf vues : synthèse, exposées, consommées, entrantes, sortantes, calendrier, contrats, consommateurs, détail. | FR-013 | F4 | ⬜ |
| I-03 | Anomalies détectées, dont DEPENDENCY_ON_EXPIRED_INTERFACE, chacune cliquable vers l'objet concerné. | FR-013 | F4 | ⬜ |
| I-04 | Commentaires Markdown horodatés et attribués, sans workflow d'approbation. | FR-014 | F4 | ⬜ |
| I-05 | Réouverture d'une revue fermée et comparaison avec une revue antérieure. | FR-013 | F4 | ⬜ |

## J. VISUALISER

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| J-01 | Accueil par domaine comme point d'entrée unique, sans parcours imposé. | — | F3 | ⬜ |
| J-02 | Fiche objet complète : identité, domaine, propriétés, documentation, dépendances, usages, historique. | — | F2 | ⬜ |
| J-03 | Navigation inverse depuis un élément vers les diagrammes, Stories, interfaces et domaines qui l'utilisent. | FR-028 | F3 | ⬜ |
| J-04 | Vue « architecture à la date T » recalculée selon les fenêtres de disponibilité effectives. | FR-029 | F3 | ⬜ |
| J-05 | Zoom sémantique du domaine vers l'élément puis la relation, sans perdre le fil. | FR-028 | F3 | ⬜ |

## K. Recherche

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| K-01 | Recherche plein texte sur noms, descriptions, documentation et contrats, avec extraits et score. | FR-030 | F5 | ⬜ |
| K-02 | Filtres par type d'objet, domaine, notation et catégorie. | FR-030 | F5 | ⬜ |
| K-03 | Tolérance aux fautes de frappe par distance d'édition sur les noms. | FR-030 | F5 | ⬜ |
| K-04 | Recherche sémantique explicitement annoncée comme indisponible en mode local, jamais simulée. | FR-030 | F5 | ⬜ |

## L. IA et propositions

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| L-01 | Question en langage naturel avec réponse streamée et liste des objets sources cliquables. | FR-022 | F5 | ⬜ |
| L-02 | Signalement explicite d'une réponse non sourcée, jamais de traçabilité feinte. | FR-022 | F5 | ⬜ |
| L-03 | Analyse d'impact avec dépendances directes et transitives, diagrammes et Stories affectés, notes de risque calculées. | FR-023 | F5 | ⬜ |
| L-04 | Proposition de modèle, relation, diagramme, documentation ou Story, présentée en ChangeSet et aperçu. | FR-024 | F5 | ⬜ |
| L-05 | Validation déterministe avant application : métamodèle, cardinalités, dates, existence des IDs, concurrence. | FR-025 | F5 | ⬜ |
| L-06 | Double confirmation Valider puis Appliquer, rendant toute écriture silencieuse impossible. | FR-025 | F5 | ⬜ |
| L-07 | Mode IA simulée annoncé, exécutant réellement les outils sur le dépôt, sans clé d'API dans le navigateur. | FR-022 | F5 | ⬜ |

## M. OpenAPI

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| M-01 | Import 3.0/3.1/3.2 couvrant titre, version, serveurs, opérations, paramètres, corps, réponses, schémas, sécurité. | FR-020 | F6 | ⬜ |
| M-02 | Conservation systématique du document source brut en plus du modèle normalisé. | FR-020 | F6 | ⬜ |
| M-03 | Idempotence par empreinte canonique et extensions `x-architecture-*`, sans seconde interface créée. | FR-020 | F6 | ⬜ |
| M-04 | Export OpenAPI 3.1 réservé aux interfaces applicatives HTTP, sans faux document pour une interface métier. | FR-021 | F6 | ⬜ |

## N. Export documentaire

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| N-01 | Export Markdown d'un domaine en arborescence navigable avec liens relatifs valides. | FR-019 | F6 | ⬜ |
| N-02 | Export d'un objet en bundle minimal incluant ses voisins directs pour que tous les liens résolvent. | FR-019 | F6 | ⬜ |
| N-03 | Exports reproductibles à révision identique, diagrammes en SVG avec complément Mermaid. | FR-019 | F6 | ⬜ |

## O. Historique et audit

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| O-01 | Historique par objet avec auteur, date, numéro de révision et résumé de changement. | FR-026 | F2 | ⬜ |
| O-02 | Comparaison de deux révisions par diff structurel champ par champ. | FR-026 | F2 | ⬜ |
| O-03 | Journal d'audit des opérations, acteur et état avant/après, non altérable. | FR-026 | F2 | ⬜ |

## P. Robustesse locale

| ID | Fonctionnalité | FR | Lot | État |
|---|---|---|---|---|
| P-01 | Sauvegarde et restauration par export/import d'un fichier de snapshot complet. | — | F6 | ⬜ |
| P-02 | Garde-fou de quota à 80 %, puis blocage explicite avec export proposé. | — | F7 | ⬜ |
| P-03 | Détection d'éviction ou de base illisible, avec bandeau explicite et proposition d'import, sans perte silencieuse. | — | F7 | ⬜ |
| P-04 | Détection d'écriture concurrente entre onglets avec invitation à recharger, sans fusion automatique. | — | F7 | ⬜ |
| P-05 | États chargement, vide, erreur et conflit systématiquement conçus, avec action de reprise. | — | F0 | ⬜ |

---

## Matrice de traçabilité — exigences fonctionnelles

| Exigence | Points du backlog | Couverture |
|---|---|---|
| FR-001 Créer/modifier/archiver des domaines | B-01, B-02, B-03 | complète |
| FR-002 Domaine responsable obligatoire | B-04 | complète |
| FR-003 Éléments ArchiMate/BPMN/UML | C-01, C-02, C-03, D-01, D-02, F-02 | complète |
| FR-004 Identifiant stable global | A-02 | complète |
| FR-005 Représentations multiples sans duplication | F-03, F-05 | complète |
| FR-006 Référencer un objet d'un autre domaine | F-04 | complète |
| FR-007 Trois catégories d'interface | E-01 | complète |
| FR-008 Publier une version d'interface | E-02, E-03, E-05, E-07 | complète |
| FR-009 Déclarer une consommation | E-06 | complète |
| FR-010 Rechercher/filtrer les interfaces | E-09 | complète |
| FR-011 Dépendances entrantes et sortantes | D-03, D-04 | complète |
| FR-012 Comparer deux versions d'interface | E-08 | complète |
| FR-013 Revue d'interfaces, vues synthétique et détaillée | I-01, I-02, I-03, I-05 | complète |
| FR-014 Commentaires et observations | I-04 | complète |
| FR-015 Basculer MODÉLISER / VISUALISER | A-05 | complète |
| FR-016 Documentation Markdown de tout objet | G-01, G-02, G-04 | complète |
| FR-017 Liens internes typés | G-03 | complète |
| FR-018 Composer une Story | H-01, H-02, H-03, H-04, H-05 | complète |
| FR-019 Exporter en Markdown navigable | N-01, N-02, N-03 | complète |
| FR-020 Importer un document OpenAPI | M-01, M-02, M-03 | complète |
| FR-021 Exporter une interface en OpenAPI | M-04 | complète |
| FR-022 Interroger en langage naturel avec IDs sources | L-01, L-02, L-07 | complète |
| FR-023 Analyse d'impact par IA | L-03 | complète |
| FR-024 Proposition IA de modèle/documentation/Story | L-04 | complète |
| FR-025 Aperçu, validation déterministe, confirmation | L-05, L-06 | complète |
| FR-026 Historique et comparaison de révisions | O-01, O-02, O-03 | complète |
| FR-027 Liens internes cassés | G-05 | complète |
| FR-028 Navigation vers les usages d'un élément | J-03, J-05 | complète |
| FR-029 Architecture à la date T | J-04, E-04 | complète |
| FR-030 Recherche plein texte | K-01, K-02, K-03, K-04 | complète |

## Matrice de traçabilité — exigences non fonctionnelles

| Exigence | Points du backlog | Mode de vérification |
|---|---|---|
| NFR-001 Révision traçable par utilisateur et date | A-03, O-01 | test d'écriture + audit |
| NFR-002 Historique non altérable silencieusement | O-03, O-02 | test d'immutabilité des révisions |
| NFR-003 Validation déterministe côté moteur | C-04, L-05 | tests unitaires partagés |
| NFR-004 Erreur LLM sans contournement des règles | L-05, L-06 | test de tentative de contournement |
| NFR-005 Fiche objet p95 < 1 s | J-02, B-03 | mesure F7 |
| NFR-006 Recherche p95 < 2 s | K-01 | mesure F7 |
| NFR-007 500 représentations sans blocage | F-01 | mesure F7 sur diagramme chargé |
| NFR-008 Dates UTC, rendu local | transverse (A-03, E-04, E-07) | test de fuseau |
| NFR-009 Exports reproductibles | N-03 | comparaison octet-à-octet |
| NFR-010 Sauvegarde préservant IDs et historique | P-01 | aller-retour de snapshot |
| NFR-011 Données minimales envoyées à l'IA | L-07 | inspection du contexte d'appel |
| NFR-012 Fournisseurs IA non source de vérité | L-05, L-06 | test d'écriture refusée |
| NFR-013 Journaux sans contrats ni prompts | transverse | test de capture de logs |
| NFR-014 Métamodèle extensible sans migration d'IDs | C-04 | chargement d'un profil alternatif |

---

## Dépendances entre lots

```text
F0 (socle) ─┬─► F1 (canvas)     ─┐
            ├─► F2 (fiches)     ─┼─► F4 (revues + stories)
            └─► F3 (visualiser) ─┘
                                   └─► F5 (recherche + IA) ─► F6 (import/export) ─► F7 (recette)
```

| Lot | Points | Contenu |
|---|---|---|
| F0 | 7 | A-01 → A-06, P-05 |
| F1 | 14 | C-01 → C-06, D-01, D-02, F-01 → F-06 |
| F2 | 11 | B-01, B-02, B-04, G-01 → G-04, J-02, O-01 → O-03 |
| F3 | 16 | B-03, D-03, D-04, E-01 → E-09, J-01, J-03 → J-05 |
| F4 | 10 | H-01 → H-05, I-01 → I-05 |
| F5 | 11 | K-01 → K-04, L-01 → L-07 |
| F6 | 9 | G-05, M-01 → M-04, N-01 → N-03, P-01 |
| F7 | 3 | P-02, P-03, P-04 |
| **Total** | **81** | |

---

## Hors périmètre MVP1

- RBAC, ABAC, restrictions de modification par domaine.
- Workflow d'approbation obligatoire, statuts d'approbation formels.
- SSO et connecteurs d'identité externes.
- Édition collaborative temps réel (CRDT).
- Moteur BPMN exécutable, simulation de processus.
- Diagrammes UML dynamiques (State Machines, Activity, Sequence).
- Conformité BPMN/UML intégrale hors sous-ensemble annoncé.
- Base vectorielle séparée et embeddings (recherche sémantique).
- MCP Apps, AG-UI, A2UI.
- Connecteurs applicatifs tiers autres que l'import/export OpenAPI.
- API HTTP réelle, PostgreSQL, authentification serveur (lot 2).

---

## Historique

| Version | Date | Changement |
|---|---|---|
| 1.0 | 17/11/2026 | Création : 81 points, matrices FR/NFR, affectation aux lots F0–F7 |

