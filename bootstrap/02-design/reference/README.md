# Référence visuelle V1

Les artefacts à garder ouverts pendant la reconstruction de l'UI. Ordre d'utilité :

| Fichier | Quoi | Comment s'en servir |
|---|---|---|
| **`noir-design-system.html`** | ⭐ Design system complet, 108 Ko, 28 sections, 2026-04-06 | **À ouvrir dans un navigateur.** Aperçus live de chaque composant, §24 « Screen Reference » (specs écran par écran), §25 anti-patterns, §26 accessibilité, §27 checklist nouveau composant. Vivait **hors du dépôt** — rapatrié ici. |
| **`styles.css`** | Le CSS réel servi à l'utilisateur, 10 654 l. | Référence pixel de dernier recours. Les valeurs de `:root` en sont extraites dans [`../tokens.css`](../tokens.css). |
| **`index.html`** | Le markup réel, 938 l. | Structure DOM, ids des panels/modals, et le `<link>` Google Fonts problématique (voir [`../ECARTS-DESIGN.md`](../ECARTS-DESIGN.md) §2). |
| **`GeistMono-Variable.woff2`** | La police embarquée, weight 100-900 | À copier tel quel dans V2. |
| **`prototype-onboarding.html`** | Prototype des 6 étapes d'onboarding, 48 Ko | Le parcours complet, avant intégration. |
| **`mockup-home.html`** | Maquette de la page d'accueil, 46 Ko | Antérieure à l'implémentation ; l'écart avec le résultat final est instructif. |
| **`splash-preview.html`** | Écran de démarrage | |
| **`dmg-window.html`** | Fenêtre du DMG d'installation, 700×390 | Utilisée par `generate-dmg.py`. Réutilisable tel quel. |
| **`app_logo.svg`** | Logo Hean | |

## Ce qui manque : `captures/`

**Aucune capture d'écran de V1 n'existe dans le dépôt.** C'est le trou le plus gênant de ce
bootstrap : le CSS seul ne dit pas à quoi ressemble le résultat composé, et le §24 du design system
donne les specs, pas le rendu.

À faire avant de commencer V2 — lancer l'app et capturer, vers `captures/` :

```bash
cd /Users/tsunami25/Documents/Thomas/noirdesktop/noir-tauri && ./scripts/dev.sh
```

Liste à couvrir : Home (haut + chaque carousel) · Albums · Artistes · Pistes (avec multi-sélection) ·
détail album · détail artiste (avec loose tracks) · Playlist · Discovery mix · Fullscreen player ·
Fullscreen lyrics · panneaux Queue / Track Info (lecture + édition) / Settings (chaque section) / EQ /
Lyrics · menus contextuels track et album · modals playlist / confirm / feedback / bulk edit /
découverte NAS · onboarding (6 étapes) · sidebar repliée (< 900 px) · module d'indexation en cours ·
player bar avec le badge bit-perfect et l'ambient glow.
