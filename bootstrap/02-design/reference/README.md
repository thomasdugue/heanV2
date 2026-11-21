# Référence visuelle V1

Les artefacts à garder ouverts pendant la reconstruction de l'UI.

> ⚠️ **Ce dossier contient deux versions du CSS.** `styles.css` est la **cible de parité** (migration
> d'octobre, commit `e12a200`) ; `styles.css.beta-v0.2.0` est ce que les testeurs ont installé.
> Pourquoi ce n'est pas la même chose : [`../ECARTS-DESIGN.md`](../ECARTS-DESIGN.md) §0.

Ordre d'utilité :

| Fichier | Quoi | Comment s'en servir |
|---|---|---|
| **`styles.css`** 🎯 | Le CSS de la **cible de parité**, 12 517 l., 2026-10-29 | Référence absolue sur les valeurs. Les deux blocs `:root` en sont extraits dans [`../tokens.css`](../tokens.css). |
| **`views.js`** 🎯 | Le **markup réel**, 3 672 l., 2026-10-29 | Indispensable : le redesign v3 réécrit la structure HTML (now-strip, page album hero/body/aside, page artiste, menu contextuel). Ce markup n'est décrit nulle part ailleurs. |
| **`noir-design-system.html`** ⭐ | Design system complet, 108 Ko, 28 sections, 2026-04-06 | **À ouvrir dans un navigateur.** Aperçus live de chaque composant, §24 « Screen Reference » (specs écran par écran), §25 anti-patterns, §26 accessibilité, §27 checklist nouveau composant. Vivait **hors du dépôt** — rapatrié ici. **Antérieur au redesign v3** : ne connaît pas les écrans refaits. |
| **`index.html`** | Le markup de la coquille, 938 l. | Structure DOM, ids des panels/modals, et le `<link>` Google Fonts problématique (voir [`../ECARTS-DESIGN.md`](../ECARTS-DESIGN.md) §2). Charge DM Sans jusqu'au poids 900 pour le watermark artiste. |
| **`GeistMono-Variable.woff2`** | La police embarquée, weight 100-900 | À copier tel quel dans V2. |
| **`styles.css.beta-v0.2.0`** | Le CSS **distribué** aux testeurs, 10 654 l. | Une seule utilité : `diff` contre `styles.css` pour savoir ce qui change entre l'app installée et la cible. Ne pas y prendre de valeurs. |
| **`index.html.beta-v0.2.0`** | Idem pour le markup de la coquille | Idem. |
| **`prototype-onboarding.html`** | Prototype des 6 étapes d'onboarding, 48 Ko | Le parcours complet, avant intégration. |
| **`mockup-home.html`** | Maquette de la page d'accueil, 46 Ko | Antérieure à l'implémentation ; l'écart avec le résultat final est instructif. |
| **`splash-preview.html`** | Écran de démarrage | |
| **`dmg-window.html`** | Fenêtre du DMG d'installation, 700×390 | Utilisée par `generate-dmg.py`. Réutilisable tel quel. |
| **`app_logo.svg`** | Logo Hean | |

## Ce qui n'est pas là, et ne le sera jamais

Le **prototype Hean v4**, dont le redesign v3 est le portage. Il a vécu dans
`/tmp/proto/player_test_1/` pendant la session de juin 2026 — un dossier temporaire. Le bundle
`design_handoff` (des fichiers `.jsx`) a disparu avec. Recherché sur tout le disque le 2026-11-21 :
introuvable. `styles.css` et `views.js` ci-dessus **sont** devenus la référence du design, faute de
mieux.

## Ce qui manque encore : `captures/`

**Aucune capture d'écran de V1 n'existe dans le dépôt.** C'est le trou le plus gênant de ce
bootstrap : le CSS seul ne dit pas à quoi ressemble le résultat composé, et le §24 du design system
donne les specs, pas le rendu — et il précède le redesign v3.

⚠️ **Les captures doivent être prises sur la cible de parité**, pas sur `main` ni sur l'app installée.
Le worktree qui porte le bon commit :

Protocole complet : [`../../../docs/PROTOCOLE-CAPTURES.md`](../../../docs/PROTOCOLE-CAPTURES.md).

```bash
cd /Users/tsunami25/Documents/Thomas/noirdesktop && git checkout thomasdugue/design-migration-audit
```

```bash
cd /Users/tsunami25/Documents/Thomas/noirdesktop/noir-tauri && ./scripts/dev.sh
```

Vérifier avant de lancer que `git log -1 --format=%h` vaut bien `e12a200`.

Liste à couvrir : Home (eyebrow + now-strip + chaque carousel) · Albums · Artistes · Pistes (avec
multi-sélection) · détail album (hero + body + aside) · détail artiste (avec watermark et loose
tracks) · Playlist · Discovery mix · Fullscreen player · Fullscreen lyrics · panneaux Queue / Track
Info (lecture + édition) / Settings (chaque section, en drawer) / EQ / Lyrics · menus contextuels
track et album · modals playlist / confirm / feedback / bulk edit / découverte NAS · onboarding
(6 étapes) · sidebar repliée (< 900 px) · module d'indexation en cours · player bar avec le badge
bit-perfect et l'ambient glow.
