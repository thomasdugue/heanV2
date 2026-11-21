# Quel design reproduire, et dans quelle source le lire

> **À lire avant de reproduire le design.** Deux pièges, et le second a failli coûter cher :
> il existe **trois versions du design** de Hean, et **six documents** qui le décrivent, qui ne
> disent pas la même chose.
>
> Premier relevé le 2026-11-21 sur `02508ea`. **Révisé le même jour** après découverte des deux
> refontes non mergées : la cible de parité a changé.

## 0. Les trois versions du design, et laquelle est la cible

| Version | Où | Date | Statut |
|---|---|---|---|
| **Design « beta »** | `main` de noirdesktop, tag `v0.2.0-beta.1` | mai 2026 | Ce que les ~50 testeurs ont installé. **Pas la cible.** |
| **Redesign v3** | PR [#37](https://github.com/thomasdugue/noirdesktop/pull/37), branche `claude/kind-cerf-A9weP` | juin 2026 | Ouverte, jamais mergée. Portage du prototype Hean v4. **Visuellement cassée telle quelle** (voir plus bas). |
| **Migration d'octobre** 🎯 | commit `e12a200` de `thomasdugue/design-migration-audit` (clone `noirdesktop-v1`) | 2026-10-29 | **LA CIBLE DE PARITÉ DE V2.** Le redesign v3 porté sur `main` d'octobre, avec la régression CSS corrigée. |

C'est la troisième qui fait foi. Elle n'existait **que** dans l'arbre de travail d'un workspace
Conductor jusqu'à sa mise à l'abri le 2026-11-21 ; elle n'est toujours pas poussée sur GitHub.
Décision et raisonnement : [`../../docs/adr/0004-cible-de-parite.md`](../../docs/adr/0004-cible-de-parite.md).

### Pourquoi la PR #37 ne peut pas servir de référence

`--glass-lo`, `--glass-md` et `--glass-hi` y sont utilisées **45 fois** sans jamais être définies :
leur définition avait disparu avec le revert des Lots A-H. Une variable CSS non définie rend la
déclaration invalide « au moment du calcul » — le navigateur l'ignore **en silence**, même avec
`!important`, et aucun test ne le détecte. Résultat sur la PR #37 : **bordures, survols et état actif
de la sidebar invisibles**.

La migration d'octobre rétablit les quatre définitions. C'est de là que vient l'invariant n°19
([`../04-invariants/INVARIANTS.md`](../04-invariants/INVARIANTS.md)) : *toute variable `var(--x)`
utilisée doit être définie dans `:root`.* En V2, c'est un test d'architecture, pas une règle de
relecture — il est spécifié dans
[`../06-setup-v2/HARNAIS.md`](../06-setup-v2/HARNAIS.md).

### Le prototype d'origine est perdu

Le redesign v3 est un *« portage fidèle du prototype Hean v4, fourni dans le bundle design_handoff +
screenshots v4 »* (commit `8a1308e`). Un commentaire dans le CSS donne son emplacement :
`/tmp/proto/player_test_1/css/tokens.css + app.css`. **C'était un dossier temporaire** : le bundle
(des fichiers `.jsx` — `home.jsx` est cité) n'est ni dans le dépôt, ni sur le disque, ni dans aucune
archive. Recherché le 2026-11-21 : introuvable.

**Conséquence** : le CSS porté **est** devenu la référence du design, faute de mieux. Les écarts de
fidélité relevés dans les commits de juin (titres de carrousels, eyebrow au-dessus du titre, boutons
icon-only) ne sont plus vérifiables contre leur source. On reproduit le portage, pas le prototype.

## Les six sources, et leur fiabilité

| Source | Date | Taille | Fiabilité |
|---|---|---|---|
| **`reference/styles.css`** | 2026-10-29 | 12 517 l. | ⭐⭐⭐⭐⭐ **La cible.** Le CSS de la migration d'octobre. Vérité absolue sur les valeurs. |
| **`tokens.css`** | 2026-11-21 | 64 variables | ⭐⭐⭐⭐⭐ Extrait automatiquement du précédent (les **deux** blocs `:root` + `@font-face` + les 10 variables posées par JS). Point de départ. |
| **`reference/views.js`** | 2026-10-29 | 3 672 l. | ⭐⭐⭐⭐⭐ **Le markup réel**, classe par classe. Indispensable : le redesign v3 n'est pas « CSS-only », il réécrit la structure HTML (now-strip, page album, page artiste). |
| **`reference/noir-design-system.html`** | 2026-04-06 | 108 Ko, 28 sections | ⭐⭐⭐⭐ **La meilleure doc design du projet, et elle était hors du dépôt.** Composants avec aperçus live, anti-patterns, accessibilité, référence écran par écran. Exacte sur couleurs/rayons/ombres. **Antérieure au redesign v3** : ne connaît ni la now-strip, ni les nouvelles pages. |
| **`reference/styles.css.beta-v0.2.0`** | mai 2026 | 10 654 l. | ⭐⭐⭐ Le CSS **distribué**. Gardé pour une seule raison : savoir ce qui change entre ce que les testeurs voient et la cible. Ne pas y prendre de valeurs. |
| **`DESIGN-SYSTEM.md`** | ~2026-05 | 179 l. | ⭐⭐ Excellente sur les **concepts** 2026 (glass, ambient glow, watermarks). **Fausse sur 5 couleurs, une ombre, et la typographie.** |

> ⚠️ `noir-design-system.html` vivait dans
> `~/Documents/Thomas/Noir Design System/noir-design-system.html`, **hors du dépôt git**. Il n'était
> référencé que par une ligne du skill `hean-frontend`. Il est désormais copié dans
> [`reference/`](reference/) — sans quoi il aurait été perdu. **C'est le document à ouvrir en premier
> pour les composants**, en gardant en tête qu'il précède le redesign v3 de deux mois.

---

## 1. Couleurs — `DESIGN-SYSTEM.md` se trompe 5 fois

Le code et `noir-design-system.html` sont **d'accord sur toute la palette**. C'est `DESIGN-SYSTEM.md`
qui dérive :

| Token | `DESIGN-SYSTEM.md` | **Code + design system HTML** |
|---|---|---|
| `--color-bg-lighter` | `#111` | `#151515` (`#111` existe mais s'appelle `--color-bg-light`) |
| `--color-bg-surface` | `#161616` | **`#1a1a1a`** |
| `--color-bg-hover` | `#1a1a1a` | **`#222`** |
| `--color-bg-elevated` | `#1e1e1e` | **`#333`** |
| `--color-error` | `#f87171` | **`#ff4444`** |

La palette réelle est une **échelle de gris à 8 niveaux** : `#0a0a0a` → `#111` → `#151515` →
`#1a1a1a` → `#222` → `#2a2a2a` → `#333` → `#444`, plus 4 niveaux de texte (`#fff`, `#888`, `#666`,
`#555`) et 3 bordures (`#222`, `#333`, `#444`).

Sémantique, usage strictement limité : `--color-green: #4ade80` (**état « en lecture » uniquement**),
`--color-error: #ff4444` + `--color-error-muted: #ff6b6b`, `--color-success: #fff`.

---

## 2. Typographie — l'écart le plus important, et il est à trois voix

### Ce que fait le code

```css
body { font-family: var(--font-body); }   /* = 'Geist Mono', 'SF Mono', Monaco, Menlo */
```

**28 usages de `var(--font-body)`** (Geist Mono) contre **7 de `var(--font-sans)`** (DM Sans) dans
`styles.css`. L'app est monospace par défaut.

### Ce que dit le design system (l'intention)

Deux polices avec des rôles distincts, et DM Sans y est citée **29 fois** :

- **Geist Mono** — corps, métadonnées, éléments techniques (chemins, bitrate, taille) ;
- **DM Sans** — **la typographie d'affichage** : titres de section (22px/600, -0.02em), titre d'album
  (28px/700, -0.03em), nom d'artiste (32px/600), titre du fullscreen player. C'est la section
  « 2026 Oversized Display Typography ».

Donc l'intention n'est **pas** « DM Sans pour le corps » (ce que dit `DESIGN-SYSTEM.md`, et qui est
faux), mais « DM Sans pour les grands titres ». Et cette intention n'est appliquée que sur 7 règles :
**elle est largement non réalisée dans le code**.

### Et DM Sans ne se charge probablement pas du tout

`index.html` charge DM Sans depuis Google Fonts :

```html
<link href="https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;500;600&display=swap" rel="stylesheet">
```

Or la CSP de `tauri.conf.json` est `style-src 'self' 'unsafe-inline'` — **pas** de
`fonts.googleapis.com` — et **aucun `font-src`** n'est déclaré (donc `default-src 'self' data: blob:
noir: https://tauri.localhost`). La feuille Google Fonts et les `.woff2` de `fonts.gstatic.com`
devraient donc être **bloqués par la CSP** dans l'app packagée, et les 7 règles `--font-sans`
tomberaient sur `-apple-system`, c'est-à-dire **San Francisco**.

**Non vérifié visuellement** (il faut lancer l'app et regarder la console + la police calculée).
C'est la première chose à vérifier. Dans tous les cas, deux problèmes à corriger en V2 :
1. une police ne doit pas dépendre du réseau (appel à Google à chaque lancement = latence + fuite
   d'IP, incohérent avec le `send_default_pii: false` de Sentry) ;
2. si DM Sans est voulue, l'embarquer en `@font-face` local, comme Geist Mono (déjà propre :
   `GeistMono-Variable.woff2`, weight 100-900, `font-display: swap`, dans [`reference/`](reference/)).

### Échelle de tailles : le code a tout décalé de +1px

Le code porte le commentaire `/* Font sizes (+1 cran pour lisibilité) */`. Résultat :

| Token | Design system HTML | **Code** |
|---|---:|---:|
| `--fs-caption` | 9px | **10px** |
| `--fs-small` | 10px | **11px** |
| `--fs-body` | 11px | **12px** |
| `--fs-label` | 12px | **13px** |
| `--fs-subheading` | 13px | **14px** |
| `--fs-title` | 14px | **15px** |
| `--fs-heading` | 16px | **18px** (+2) |
| `--fs-section-title` | 22px | 22px |
| `--fs-page-title` | 28px | 28px |
| `--fs-hero` | 48px | 48px |

Les 7 petites tailles ont été augmentées après coup, les 3 grandes sont inchangées. **Le code gagne.**

### Et le redesign v3 a tranché, deux fois, dans des sens opposés

Le prototype Hean v4 portait les petites tailles du design system (9/10/12/13/14/16px). Les deux
portages n'en ont pas fait la même chose :

- **PR #37 (juin)** les redéfinit dans son bloc `:root` — donc toute l'app rétrécit de 1 à 2px.
- **Migration d'octobre (la cible)** les laisse de côté, avec la raison écrite dans le CSS :
  *« Les `--fs-*` du prototype ne sont PAS redéfinis ici : ils rétréciraient de 1-2px tout le texte
  de l'app (onboarding inclus). »*

**Pour V2, c'est la décision d'octobre qui s'applique** : une seule échelle typographique, celle du
`:root` (10/11/12/13/14/15/18/22/28/48). L'eyebrow de la home est à 10px, pas 9. Un seul endroit du
CSS fixe une taille en dur plutôt que par token — c'est à corriger, pas à reproduire.
`DESIGN-SYSTEM.md`, lui, n'en documente que 7 sur 10 (il ignore section-title, page-title, hero).

---

## 3. Rayons, ombres, glass

| Token | Spec de mars | `DESIGN-SYSTEM.md` | Design system HTML | **Code** |
|---|---|---|---|---|
| `--radius-sm` | 2px | — | 4px | **4px** |
| `--radius-md` | 6px | — | 8px | **8px** |
| `--radius-lg` | 8px | — | 12px | **12px** |
| `--radius-xl` | 8px | — | — | **n'existe plus** |
| `--shadow-lg` | — | `0 8px 24px /.5` | `0 12px 32px /.5` | **`0 12px 32px /.5`** |

`--shadow-sm` / `--shadow-md` et tous les easings (`--ease-out-expo`, `--ease-spring`,
`--ease-smooth`) et durées (0.15 / 0.2 / 0.3 s) sont conformes partout.

**Glassmorphism — divergence sur le flou.** Le code définit des tokens globaux
(`--glass-bg: rgba(17,17,17,0.78)`, `--glass-blur: blur(20px) saturate(150%)`,
`--glass-border: rgba(255,255,255,0.06)`), alors que le design system HTML ne définit **pas** de
tokens et spécifie un flou **par élément** — notamment **blur 44px** pour les panneaux latéraux
(Settings 380px, Queue 320px, EQ, Lyrics). À trancher : tokens globaux (plus simple) ou flou par
élément (plus fidèle à l'intention).

Appliqué sur : sidebar, player bar, modals, menus contextuels, toasts, search panel, panneaux
latéraux. **Jamais** sur les grilles d'albums, les listes de pistes, les pages album/artiste.

---

## 4. Ce que seul le design system HTML contient

Ses 28 sections couvrent ce qu'aucun autre document ne décrit. À ouvrir dans un navigateur — il y a
des aperçus live :

- **§11-17 Composants** : boutons (primary, secondary, icon, control), items de navigation, inputs,
  toggle switches, range sliders, badges, player bar, sidebar — avec leurs états.
- **§18 Carousels** : covers carrées (albums) vs cercles (artistes).
- **§19 Track list** : barre d'accent à gauche au hover/en lecture.
- **§20-21** Menus contextuels, modals.
- **§22-23 Animations et les 4 tendances 2026** : album-art color theming, progressive disclosure,
  spring physics, kinetic typography.
- **§24 Screen Reference** ⭐ — **le substitut le plus proche de captures d'écran** : pour chaque
  écran, le layout et les specs clés. Exemples : Albums = `grid auto-fill minmax(220px, 1fr)`, hover
  `translateY(-6px) scale(1.02)` + `--shadow-lg` ; Artistes = `minmax(160px, 1fr)`, covers
  circulaires, texte centré ; Artist detail = cover 280×280 circulaire ; Fullscreen = cover 220px,
  ring sur le bouton play, particules ; Queue = point vert sur la piste en cours, drag-to-reorder ;
  Lyrics = ligne courante en blanc, les autres en `#555`.
- **§25 Anti-patterns** : « jamais de gradient coloré », « jamais d'accent autre que blanc / vert
  (lecture) / rouge (erreur) », etc. **Ces règles protègent l'identité monochrome — à reprendre
  telles quelles.**
- **§26 Accessibilité** : focus states, touch targets, `prefers-reduced-motion`, ratios de contraste.
  **Le seul endroit du projet où l'accessibilité est traitée.**
- **§27 Checklist nouveau composant.**
- **§28 Variables CSS en copier-coller.**

⚠️ Il décrit aussi l'écran **DAP Sync** (3 étapes), feature archivée le 2026-04-22, soit 16 jours
après sa rédaction. À ignorer.

## Ce que seul `DESIGN-SYSTEM.md` contient

Les trois chantiers de mai 2026, postérieurs au design system HTML, et qui y sont bien décrits :

- **ambient glow** sous la player bar, intensité suivant la luminosité de la pochette
  (`--ambient-intensity` : 0.02 cover sombre → 0.06 cover claire) ;
- **watermarks dynamiques** sur les pages artiste et album : le nom en `clamp(140px, 22vw, 260px)`,
  teinté par un gradient 2 couleurs extraites de l'artwork via `background-clip: text` +
  `-webkit-text-fill-color: transparent` ;
- **bandeau gradient 6 stops** sur la page album, avec opacité **calculée** depuis la luminosité
  (10 % cover claire → 28 % cover sombre) ;
- le **pipeline d'extraction de couleur** : canvas 64×64 → `getImageData` → k-means 5 couleurs →
  `pickAmbientColor()` (score = saturation × brightness, pénalités near-black / near-white / grays) →
  primaire = la plus vibrante, secondaire = la suivante → luminosité rec601
  `(R*0.299 + G*0.587 + B*0.114)/255`.

Variables posées par JS (absentes de `:root`, listées en fin de `tokens.css`) :
`--color-ambient-r/g/b`, `--color-ambient-2-r/g/b`, `--ambient-luminosity`,
`--ambient-gradient-opacity`, `--ambient-watermark-opacity`, `--ambient-intensity`.

Et ses **7 règles d'usage** finales sont bonnes (monochrome sauf vert/rouge, glass sélectif,
animations fonctionnelles, profondeur par luminosité en 3 niveaux max, minimum 10px,
watermark = signature et non décoration, bandeau gradient sur les albums uniquement).

---

## 5. Méthode recommandée pour « le design à l'identique »

1. **Ouvrir `reference/noir-design-system.html` dans un navigateur** et le garder ouvert. C'est la
   carte des composants, des écrans, des anti-patterns et de l'accessibilité.
2. **Prendre les valeurs dans `tokens.css`**, jamais dans un document. En cas de conflit avec le
   design system HTML : le code gagne (c'est le cas sur les 7 petites tailles de police et le flou du
   glass).
2bis. **Prendre la structure dans `reference/views.js`.** Le design system HTML décrit les composants
   de mai ; la now-strip, la page album en hero + body + aside, la page artiste avec watermark et le
   menu contextuel du redesign v3 n'y sont pas. Leur markup n'existe que là.
3. **Lire `DESIGN-SYSTEM.md`** pour les trois chantiers de mai (ambient glow, watermarks, bandeau) et
   ses 7 règles d'usage. Ignorer ses valeurs de couleurs, son `--shadow-lg` et sa phrase sur DM Sans.
4. **Garder `reference/styles.css` et `reference/index.html`** comme référence pixel de dernier
   recours : seul endroit où le design est complet et vrai. Pour voir ce qui change par rapport à
   l'app installée chez les testeurs, differ contre `reference/styles.css.beta-v0.2.0`. À lire comme une maquette, pas comme une
   architecture (voir la dette CSS dans
   [`../05-postmortem/DETTE-TECHNIQUE-V1.md`](../05-postmortem/DETTE-TECHNIQUE-V1.md)).
5. **Trancher la question Geist Mono / DM Sans.** L'échelle de tailles, elle, est tranchée (voir
   §2). Reste le partage des rôles : (a) Geist Mono partout, comme l'app tourne probablement
   aujourd'hui, DM Sans étant vraisemblablement bloquée par la CSP ; (b) l'intention du design
   system — Geist Mono pour le corps et le technique, DM Sans **embarquée en local** pour les grands
   titres, cette fois réellement appliquée. Le redesign v3 pousse vers (b) : il charge DM Sans
   jusqu'au poids 900 pour le watermark artiste. C'est le seul écart qui change l'apparence
   générale : décision consciente, après avoir regardé l'app.
6. **Faire les captures d'écran de V1 avant de commencer.** Aucune n'existe dans le dépôt. Le
   §24 Screen Reference en tient lieu sur les specs, pas sur le rendu composé.
   → `reference/captures/`, à créer.
