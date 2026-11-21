# Les 4 sources du design V1, et laquelle croire

> **À lire avant de reproduire le design.** Il existe **quatre** descriptions du design de Hean, et
> elles ne disent pas la même chose. Reproduire « à l'identique » en partant de la mauvaise donnerait
> un résultat visiblement différent de l'app actuelle.
>
> Relevé fait le 2026-11-21 sur le commit `02508ea`.

## Les quatre sources

| Source | Date | Taille | Fiabilité |
|---|---|---|---|
| **`reference/styles.css`** | à jour | 10 654 l. | ⭐⭐⭐⭐⭐ **C'est le code servi à l'utilisateur.** Vérité absolue sur les valeurs. |
| **`tokens.css`** | 2026-11-21 | 116 l. | ⭐⭐⭐⭐⭐ Extrait automatiquement du précédent (`:root` + `@font-face`). À utiliser comme point de départ. |
| **`reference/noir-design-system.html`** | 2026-04-06 | 108 Ko, 28 sections | ⭐⭐⭐⭐ **La meilleure doc design du projet, et elle était hors du dépôt.** Composants avec aperçus live, anti-patterns, accessibilité, référence écran par écran. Valeurs de couleurs/rayons/ombres **exactes**. Décalée de +1px sur l'échelle typographique et sur le flou du glass. |
| **`DESIGN-SYSTEM.md`** | ~2026-05 | 179 l. | ⭐⭐ Excellente sur les **concepts** 2026 (glass, ambient glow, watermarks). **Fausse sur 5 couleurs, une ombre, et la typographie.** |

> ⚠️ `noir-design-system.html` vivait dans
> `~/Documents/Thomas/Noir Design System/noir-design-system.html`, **hors du dépôt git**. Il n'était
> référencé que par une ligne du skill `hean-frontend`. Il est désormais copié dans
> [`reference/`](reference/) — sans quoi il aurait été perdu. **C'est le document à ouvrir en premier
> pour reconstruire l'UI.**

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
3. **Lire `DESIGN-SYSTEM.md`** pour les trois chantiers de mai (ambient glow, watermarks, bandeau) et
   ses 7 règles d'usage. Ignorer ses valeurs de couleurs, son `--shadow-lg` et sa phrase sur DM Sans.
4. **Garder `reference/styles.css` et `reference/index.html`** comme référence pixel de dernier
   recours : seul endroit où le design est complet et vrai. À lire comme une maquette, pas comme une
   architecture (voir la dette CSS dans
   [`../05-postmortem/DETTE-TECHNIQUE-V1.md`](../05-postmortem/DETTE-TECHNIQUE-V1.md)).
5. **Trancher la question typographique.** Trois options : (a) Geist Mono partout, comme l'app
   tourne probablement aujourd'hui ; (b) l'intention du design system — Geist Mono pour le corps et
   le technique, DM Sans **embarquée** pour les grands titres, cette fois réellement appliquée ;
   (c) autre chose. C'est le seul écart qui change l'apparence générale : il mérite une décision
   consciente, prise après avoir regardé l'app.
6. **Faire les captures d'écran de V1 avant de commencer.** Aucune n'existe dans le dépôt. Le
   §24 Screen Reference en tient lieu sur les specs, pas sur le rendu composé.
   → `reference/captures/`, à créer.
