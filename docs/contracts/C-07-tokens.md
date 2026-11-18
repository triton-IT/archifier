# C-07 — Tokens : bandes, zones, frontières, ports, temporalité, statuts

| | |
|---|---|
| **Réf.** | C-07 |
| **Version** | 1.0 — figé G0, 18/11/2026 |
| **Propriétaire** | UX — `assets/tokens.css` |
| **Sources** | NFR-028, §15.5, §15.6 ; VD-09, VD-10, VD-12, VD-23 ; ADR-022 §4 |
| **Dépend de** | C-02, C-04 |

## 1. Règle

Aucune couleur, taille, ombre, durée ou épaisseur n'est écrite en dur dans un composant.
Tout passe par une variable de `assets/tokens.css`. `assets/app.css` **consomme** ces
variables et n'en définit plus aucune en local.

Un token est **gelé** dès G0. Un ajout est une révision mineure de C-07 ; un renommage est
interdit (il casse les composants gelés).

Les valeurs ci-dessous sont des **valeurs initiales** ; l'UX peut ajuster une valeur sans
changer le nom, à condition que les tests de contraste (§9) restent verts.

## 2. Couleurs de base (conservées)

```css
--bg, --surface, --surface-2, --ink, --ink-soft, --muted,
--line, --line-strong, --accent, --accent-soft,
--ok, --ok-soft, --warn, --warn-soft, --danger, --danger-soft,
--archive, --archive-soft
```

Valeurs actuelles conservées à l'identique (aucune régression visuelle des écrans lot 1/2).

## 3. Bandes de couches

| Token | Rôle | Valeur initiale |
|---|---|---|
| `--layer-band-gap` | Espace vertical entre deux bandes | `16px` |
| `--layer-band-radius` | Rayon des bandes | `8px` |
| `--layer-label-size` | Taille du libellé de bande | `12px` |
| `--layer-label-weight` | Graisse du libellé | `600` |
| `--layer-label-tracking` | Interlettrage | `0.04em` |
| `--layer-collapsed-h` | Hauteur d'une bande repliée | `32px` |
| `--layer-label-pad` | Marge interne du libellé | `12px` |
| `--layer-business-bg` | Fond Métier | `#eef2fb` |
| `--layer-business-fg` | Texte Métier | `#28374f` |
| `--layer-business-accent` | Filet Métier | `#5b6ea8` |
| `--layer-application-bg` | Fond Applicatif | `#e9f5ef` |
| `--layer-application-fg` | Texte Applicatif | `#1e4433` |
| `--layer-application-accent` | Filet Applicatif | `#2f7d5a` |
| `--layer-technology-bg` | Fond Technologique | `#fdf0e6` |
| `--layer-technology-fg` | Texte Technologique | `#5c3612` |
| `--layer-technology-accent` | Filet Technologique | `#a9622a` |
| `--layer-physical-bg` | Fond Physique | `#f3ecfa` |
| `--layer-physical-fg` | Texte Physique | `#402659` |
| `--layer-physical-accent` | Filet Physique | `#7a4fa3` |
| `--layer-motivation-bg` | Fond Motivation | `#fff5e3` |
| `--layer-motivation-fg` | Texte Motivation | `#5d3f00` |
| `--layer-motivation-accent` | Filet Motivation | `#a97a12` |
| `--layer-implementation-bg` | Fond Mise en œuvre | `#eef1f4` |
| `--layer-implementation-fg` | Texte Mise en œuvre | `#333d49` |
| `--layer-implementation-accent` | Filet Mise en œuvre | `#62748a` |
| `--layer-process-bg` | Fond Processus (BPMN) | `#eaf1fb` |
| `--layer-process-fg` | Texte Processus | `#233a5c` |
| `--layer-process-accent` | Filet Processus | `#3c6aa6` |
| `--layer-flow-bg` | Fond Flux (BPMN) | `#f2f7fd` |
| `--layer-flow-fg` | Texte Flux | `#2b4a70` |
| `--layer-flow-accent` | Filet Flux | `#5b86b8` |
| `--layer-data-bg` | Fond Données | `#eef6f2` |
| `--layer-data-fg` | Texte Données | `#1f4a38` |
| `--layer-data-accent` | Filet Données | `#3d8a63` |
| `--layer-annotation-bg` | Fond Annotation | `#f6f5ef` |
| `--layer-annotation-fg` | Texte Annotation | `#4a4736` |
| `--layer-annotation-accent` | Filet Annotation | `#8a8360` |
| `--layer-structure-bg` | Fond Structure (UML) | `#eef3f7` |
| `--layer-structure-fg` | Texte Structure | `#26404f` |
| `--layer-structure-accent` | Filet Structure | `#4f7488` |
| `--layer-deployment-bg` | Fond Déploiement (UML) | `#fdf0e6` |
| `--layer-deployment-fg` | Texte Déploiement | `#5c3612` |
| `--layer-deployment-accent` | Filet Déploiement | `#a9622a` |
| `--layer-behavior-bg` | Fond Comportement (UML) | `#f4eefb` |
| `--layer-behavior-fg` | Texte Comportement | `#43265c` |
| `--layer-behavior-accent` | Filet Comportement | `#7d54a5` |

## 4. Zones de domaine

| Token | Rôle | Valeur initiale |
|---|---|---|
| `--zone-domain-bg` | Fond par défaut d'une zone domaine | `#f7f9fd` |
| `--zone-domain-border` | Bordure | `#9fb2cc` |
| `--zone-domain-border-width` | Épaisseur | `2px` |
| `--zone-domain-radius` | Rayon | `8px` |
| `--zone-domain-label-size` | Libellé | `13px` |
| `--zone-domain-label-weight` | Graisse | `600` |
| `--zone-domain-selected` | Bordure sélectionnée | `#2f6feb` |
| `--zone-domain-selected-width` | Épaisseur | `2.5px` |
| `--zone-group-bg` | Fond d'un groupe visuel | `#fbfbfd` |
| `--zone-group-border` | Bordure | `#c6cedb` |
| `--zone-group-dash` | Pointillé SVG | `6 4` |
| `--zone-palette-1` | Teinte domaine 1 | `#e8f0fe` |
| `--zone-palette-2` | Teinte domaine 2 | `#e6f4ec` |
| `--zone-palette-3` | Teinte domaine 3 | `#fdf1dc` |
| `--zone-palette-4` | Teinte domaine 4 | `#f3ecfa` |
| `--zone-palette-5` | Teinte domaine 5 | `#fdecec` |
| `--zone-palette-6` | Teinte domaine 6 | `#e7f3f7` |
| `--zone-palette-ink` | Texte sur teinte | `#1b2430` |

Une zone `DOMAIN` porte **toujours** son libellé et son teintage `domain-N` ; la couleur
n'est jamais le seul indicateur d'appartenance (NFR-028).

## 5. Frontières et ports

| Token | Rôle | Valeur initiale |
|---|---|---|
| `--boundary-stroke` | Trait de frontière | `#6b7787` |
| `--boundary-width` | Épaisseur | `1.5px` |
| `--boundary-dash` | Pointillé SVG | `8 5` |
| `--boundary-active-stroke` | Frontière survolée / cible de port | `#2f6feb` |
| `--port-size` | Diamètre d'un port | `16px` |
| `--port-fill` | Fond | `#ffffff` |
| `--port-stroke` | Contour | `#2f6feb` |
| `--port-stroke-width` | Épaisseur | `2px` |
| `--port-active-fill` | Port avec version active | `#e6f4ec` |
| `--port-selected-fill` | Port sélectionné | `#e8f0fe` |
| `--port-hit-pad` | Zone de clic élargie | `8px` |
| `--port-version-size` | Taille du libellé de version | `10px` |
| `--port-consumer-stroke` | Trait de consommation traversante | `#17724b` |
| `--port-consumer-dash` | Pointillé de consommation | `4 3` |

Un port affiche un libellé de version (`v2`) **au zoom de détail** (C-07 §8) et toujours
un titre accessible (`aria-label` = nom d'interface + version + état).

## 6. Encodage temporel (motif + texte, jamais couleur seule)

| Token | Rôle | Valeur initiale |
|---|---|---|
| `--temporal-active-stroke` | Contour `ACTIVE` | `#1f6f4a` |
| `--temporal-active-fill` | Fond `ACTIVE` (plein) | `#ffffff` |
| `--temporal-active-pattern` | Motif SVG | `none` |
| `--temporal-scheduled-stroke` | Contour `SCHEDULED` | `#2f6feb` |
| `--temporal-scheduled-fill` | Fond | `#f5f9ff` |
| `--temporal-scheduled-pattern` | Motif SVG | `dash 6 4` |
| `--temporal-expired-stroke` | Contour `EXPIRED` | `#6b7787` |
| `--temporal-expired-fill` | Fond | `#f4f5f7` |
| `--temporal-expired-pattern` | Motif SVG | `dash 2 4` |
| `--temporal-expired-opacity` | Opacité du fantôme | `0.55` |
| `--temporal-expired-strike-width` | Épaisseur de la barre | `1.5px` |
| `--temporal-overlap-fill` | Chevauchement | `repeating-linear-gradient(45deg, #fdf1dc 0 6px, #fbe6c6 6px 12px)` |
| `--temporal-badge-active-bg` / `-fg` | Badge | `#e6f4ec` / `#0f5132` |
| `--temporal-badge-scheduled-bg` / `-fg` | Badge | `#e8f0fe` / `#1d4ed8` |
| `--temporal-badge-expired-bg` / `-fg` | Badge | `#eceff3` / `#3c4654` |
| `--temporal-legend-size` | Légende | `11px` |

Règle absolue : chaque forme temporelle porte un **libellé d'état** lisible (« Active »,
« Planifiée », « Expirée ») en plus du motif. Un test a11y vérifie l'absence de couleur
seule (VD-21).

## 7. Croquis, brouillon, validé, publié

| Token | Rôle | Valeur initiale |
|---|---|---|
| `--draft-fill` | Fond d'un croquis | `#fbfbfd` |
| `--draft-stroke` | Contour | `#6b7787` |
| `--draft-dash` | Pointillé SVG | `4 3` |
| `--draft-label-style` | Style du libellé | `italic` |
| `--draft-badge-bg` / `-fg` | Badge « Croquis » | `#eceff3` / `#3c4654` |
| `--status-formalized-bg` / `-fg` | Badge « Brouillon » | `#fdf1dc` / `#7a4600` |
| `--status-valid-bg` / `-fg` | Badge « Validé » | `#e6f4ec` / `#0f5132` |
| `--status-published-bg` / `-fg` | Badge « Publié » | `#e8f0fe` / `#1d4ed8` |
| `--status-withdrawn-bg` / `-fg` | Badge « Retiré » | `#eceff3` / `#3c4654` |
| `--status-missing-fg` | Propriété requise manquante | `#b42318` |
| `--status-missing-bg` | Fond du signal | `#fdecea` |

## 8. Densité, zoom sémantique, géométrie

| Token | Rôle | Valeur initiale |
|---|---|---|
| `--zoom-aggregate-max` | En dessous : agrégats par bande/domaine | `0.55` |
| `--zoom-detail-min` | Au-dessus : ports, versions, propriétés | `1.15` |
| `--zoom-min` / `--zoom-max` | Bornes du zoom | `0.25` / `4` |
| `--node-min-w` / `--node-min-h` | Taille minimale d'une forme | `120px` / `56px` |
| `--node-radius` | Rayon d'une forme | `8px` |
| `--node-radius-pill` | Forme arrondie | `999px` |
| `--node-title-size` | Nom | `13px` |
| `--node-title-weight` | Graisse | `600` |
| `--node-type-size` | Type | `11px` |
| `--node-domain-size` | Domaine | `10px` |
| `--node-status-size` | État temporel | `10px` |
| `--node-icon-size` | Glyphe | `16px` |
| `--cull-margin` | Marge de virtualisation | `200px` |
| `--minimap-size` | Largeur de la minimap | `168px` |
| `--inspector-width` | Largeur de l'inspecteur | `320px` |
| `--palette-width` | Largeur de la palette | `232px` |
| `--toolbar-height` | Hauteur de la barre d'outils | `48px` |
| `--statusbar-height` | Hauteur de la barre d'état | `28px` |

## 9. Typographie, espacement, élévation, mouvement

| Token | Rôle | Valeur initiale |
|---|---|---|
| `--text-xs` … `--text-xl` | Échelle typographique | `11px`, `12px`, `13px`, `15px`, `18px` |
| `--leading-tight` / `--leading-normal` | Interlignes | `1.25` / `1.5` |
| `--weight-regular` / `--weight-medium` / `--weight-strong` | Graisses | `400` / `500` / `600` |
| `--space-1` … `--space-8` | Espacement (base `--step` 4px) | `4, 8, 12, 16, 24, 32, 48, 64` |
| `--shadow-canvas` | Ombre de forme | `0 1px 2px rgba(20,30,45,.06), 0 4px 12px rgba(20,30,45,.04)` |
| `--shadow-inspector` | Ombre de panneau | `-2px 0 12px rgba(20,30,45,.08)` |
| `--shadow-popover` | Ombre de palette de commandes | `0 12px 32px rgba(20,30,45,.18)` |
| `--motion-fast` | Sélection, survol | `120ms` |
| `--motion-base` | Apparition de lien | `160ms` |
| `--motion-slow` | Cadrage demandé | `220ms` |
| `--ease-standard` | Courbe | `cubic-bezier(.2,.6,.2,1)` |
| `--ease-emphasis` | Courbe du cadrage | `cubic-bezier(.3,0,.1,1)` |

`prefers-reduced-motion: reduce` force `--motion-fast/base/slow` à `0ms` et supprime toute
transition non demandée. Aucune animation décorative.

## 10. Focus, contraste, cibles

| Token | Rôle | Valeur initiale |
|---|---|---|
| `--focus-ring` | Anneau de focus | `#2f6feb` |
| `--focus-width` | Épaisseur | `2px` |
| `--focus-offset` | Décalage | `2px` |
| `--tap-target` | Cible minimale | `44px` |
| `--hit-pad` | Marge de clic des formes | `6px` |
| `--contrast-min-text` | Seuil texte | `4.5` |
| `--contrast-min-ui` | Seuil interface | `3.0` |

Toute couleur de texte sur fond teinté doit atteindre **4.5:1** ; tout trait signifiant
(bordure de port, contour d'état) **3:1**. Un test automatisé calcule les ratios des
paires déclarées et échoue sinon.

## 11. Ce qui n'est pas un token

- Une couleur choisie par l'utilisateur pour une forme : elle n'existe pas. Seul
  `style_override` à 4 clés est accepté par le modèle (§14.1), et l'UI de l'Atelier ne
  l'expose pas au MVP3.
- Les couleurs d'un domaine dans le référentiel : le domaine n'a pas de couleur stockée ;
  sa teinte de zone est un `tone` de vue, dérivé d'un ordre stable de domaine.
- Les icônes : aucune icône normative n'est reproduite ; les glyphes restent ceux du
  profil (`glyph_family`), rendus par les primitives.

## 12. Tests obligatoires (noms figés)

| Test | Prouve |
|---|---|
| `no_hardcoded_color_in_components_css` | C-07 §1 |
| `tokens_define_every_grammar_family` | VD-23 |
| `token_text_pairs_meet_aa` | VD-21 |
| `reduced_motion_disables_transitions` | NFR-028, VD-21 |
| `temporal_state_has_text_and_pattern` | NFR-028 |
| `status_badges_cover_three_draft_states` | NFR-029 |
| `design_gallery_renders_all_tokens_and_states` | VD-23 |
