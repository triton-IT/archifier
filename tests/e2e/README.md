# E2E — chaîne de conception

Ces tests dérivent de `functional-specifications.md` (FR-040..054, Gherkin) et de
`docs/ui-spec.md`. Ils ne corrigent jamais le produit : un échec est un signalement.

## Prérequis

```powershell
.\scripts\build.cmd
node scripts\serve.mjs 4173
npx playwright test tests/e2e
```

`npx` doit pouvoir installer Playwright ; sur un poste où l'exécution des scripts
npm est désactivée, utiliser un interpréteur Node avec Playwright installé
globalement, ou exécuter la chaîne manuellement (voir ci-dessous).

## Scénarios couverts

| Scénario | Fichier | Points |
|---|---|---|
| `/` ouvre l'Atelier, pas un inventaire | `design-chain.spec.ts` | VD-01 |
| Déposer, nommer, formaliser, relier, frontière, port | idem | VD-02..VD-11 |
| Curseur temporel lisible sans fiche | idem | VD-12 |
| Publier, partager, rouvrir le lien | idem | VD-13 |
| Bibliothèque et Contrôle secondaires | idem | VD-15, VD-16 |
| `DSG-001` : formalisation incomplète | idem | VD-07 |
| `DSG-003` : publication avec croquis | idem | C-04 |
| `arch://` non cassé | idem | VD-01 |

## Exécution manuelle équivalente (vérifiée le 18/11/2026)

1. Ouvrir `http://127.0.0.1:4173/` : l'Atelier s'affiche.
2. Choisir « Applicatif d'un domaine », domaine Paiements, « Créer cette vue ».
3. Cliquer un composant dans la palette : le croquis est sélectionné, l'inspecteur
   propose type et domaine.
4. `F2` / double-clic : renommer sur place.
5. « Formaliser » : le croquis devient un nœud `VALIDÉ`.
6. Outil Relation, cliquer deux nœuds : la barre ne propose que les relations
   autorisées ; « Relier » crée l'arête.
7. « Zone de domaine », sélectionner la zone, choisir une interface, « Rattacher ».
8. Champ date `2027-08-01` : le port passe « Expirée » sans déplacement.
9. « Publier » avec un croquis : `DSG-003` ; retirer le croquis, publier : succès.
10. « Copier le lien », le rouvrir : posture lecture, zoom et curseur restitués.
11. Bibliothèque : couvertures ; Contrôle : anomalies et inventaire, aucune création.
12. Recharger : la vue, le cadrage et les croquis sont restaurés.

## Limite connue

Le binaire Playwright n'est pas installé dans l'environnement de développement
courant (`npm` bloqué par la politique d'exécution PowerShell). Les scénarios
ci-dessus ont été rejoués dans le navigateur intégré, avec les mêmes sélecteurs.
