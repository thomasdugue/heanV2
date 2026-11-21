# 0001 — Réécrire Hean de zéro plutôt que refactorer V1

- **Date** : 2026-11-21
- **État** : Acceptée
- **Décideur** : Thomas

## Contexte

Hean v0.2.0-beta.1 fonctionne : DMG signé et notarisé distribué à ~50 testeurs, audio bit-perfect,
gapless sur NAS, AirPlay géré. Le problème n'est pas la qualité du résultat, c'est le **coût marginal
de chaque changement**, qui monte.

Constats mesurés sur le code (détail dans
[`../../bootstrap/05-postmortem/DETTE-TECHNIQUE-V1.md`](../../bootstrap/05-postmortem/DETTE-TECHNIQUE-V1.md)) :

- **16 076 lignes de JavaScript sans types ni linter** — la seule analyse statique est `node --check`
  (syntaxe). Les bugs les plus coûteux de V1 sont des noms de champs faux, un `.size` sur un Array, un
  glyphe Unicode divergent entre deux copies de la même fonction.
- **Des fichiers trop gros pour être raisonnés** : `lib.rs` 4 870 lignes (86 commandes + 15 états
  globaux), `views.js` 3 512, `styles.css` 10 654 en un seul fichier.
- **Sept caches globaux concurrents**, chacun persisté dans son propre JSON, modifiés en même temps
  par le scan de fond, les écritures de métadonnées et les commandes IPC — sans transaction.
- **19 invariants, dont 8 (42 %) protègent des faiblesses structurelles** et non des spécificités de
  la plateforme. Quatre décrivent le même problème de cohérence de caches ; quatre protègent une
  vignette de 28 pixels, corrigée quatre fois.
- **3 invariants sur 19 sont couverts par un test.** Les 16 autres reposent sur la vigilance.
- **La documentation a dérivé** : 11 affirmations fausses au moment de l'audit, et le meilleur
  document design du projet vivait hors du dépôt git.

## Décision

Réécrire l'application dans un nouveau dépôt (`heanV2`), avec **parité fonctionnelle et visuelle
stricte** comme contrainte, et un harnais de vérification posé **avant** la première feature.

Le savoir de V1 est transporté, pas le code : les invariants, les 24 comportements anti-régression,
les 7 pièges de plateforme (CoreAudio/AirPlay, singleton libsmbclient, media keys, drag WebView,
petits CSS Grid WebKit, LC_RPATH dyld, notarisation du DMG) et le design system.

Exception : les modules qui encodent du savoir sur des APIs hostiles sont **portés, pas réécrits** —
`coreaudio_backend.rs`, `coreaudio_stream.rs`, `media_controls.rs`, tout `network/`. Les réécrire,
c'est racheter les mêmes bugs.

## Alternatives écartées

| Alternative | Pourquoi écartée |
|---|---|
| **Refactorer V1 progressivement** | Les trois problèmes structurels (absence de types, fichiers monolithiques, caches concurrents) se tiennent entre eux : introduire TypeScript dans 16 000 lignes sans bundler, ou passer en SQLite sans toucher aux 86 commandes, ne se fait pas par incréments livrables. Le refactoring aurait la même ampleur que la réécriture, sans le bénéfice d'un harnais neuf. |
| **Ne rien changer et continuer** | Tenable à court terme — l'app marche. Écarté parce que le coût par changement augmente et que la densité d'invariants le montre. |
| **Changer seulement de modèle d'assistance** (un modèle plus abouti sur la base V1) | Un modèle plus abouti écrit mieux et tient plus de contexte, mais il ne peut pas vérifier un nom de champ sans compilateur, et il ne rend pas vraie une documentation qui a dérivé. Le gain vient du harnais ; le modèle l'amplifie. |
| **Changer de framework applicatif** (Electron, SwiftUI, natif) | Tauri v2 + Rust + CoreAudio direct est démontré en production et c'est ce qui rend le bit-perfect et le hog mode possibles. Aucune raison de le rouvrir. |

## Conséquences acceptées

- **Le risque principal devient la perte de features**, pas l'architecture. D'où le contrat de parité
  en 124 points ([`../../bootstrap/01-produit/FEATURE-SET.md`](../../bootstrap/01-produit/FEATURE-SET.md))
  et le mode parité V1 ↔ V2 du harnais.
- **Les beta-testeurs restent sur V1** jusqu'à ce que V2 atteigne la parité.
- **La migration des données utilisateur** (bibliothèque, playlists, historique, sources NAS,
  tables SQLite de DAP Sync) devra être traitée explicitement.
- **DAP Sync reste hors périmètre** pour V2, mais son code (~12 400 lignes) est conservé dans
  [`../../bootstrap/05-postmortem/archive-dap-sync/`](../../bootstrap/05-postmortem/archive-dap-sync/).
- **Une période à deux bases de code**, le temps de la reconstruction.
