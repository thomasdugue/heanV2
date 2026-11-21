# heanV2

Réécriture complète de **Hean**, lecteur de musique audiophile pour macOS : même design, même feature
set, base technique saine.

**État : aucun code applicatif.** Ce dépôt contient pour l'instant le brief, le savoir repris de V1 et
le harnais. C'est volontaire — la phase 0 consiste à poser le socle et le filet anti-régression
*avant* la première ligne de code.

---

## Démarrer

```bash
./check
```

Vert. Les vérifications grandissent avec le projet ; `check` est le point d'entrée unique, identique
en local, en pre-commit et en CI.

Puis, dans une session Claude Code ouverte sur ce dépôt :

> Lis `bootstrap/00-BRIEF-V2.md`, `bootstrap/05-postmortem/DETTE-TECHNIQUE-V1.md`,
> `bootstrap/06-setup-v2/HARNAIS.md` et `bootstrap/06-setup-v2/PLAN-DE-RECONSTRUCTION.md`.
> On est en phase 0 : pose-moi les 6 arbitrages un par un, avec ta recommandation, et n'écris aucun
> code avant qu'ils soient tranchés.

---

## Organisation du dépôt

| Chemin | Contenu |
|---|---|
| `bootstrap/` | **Tout le contexte repris de V1** — brief, contrat de parité (124 features), design system, invariants, postmortem, plan de reconstruction, spécification du harnais. Autonome : [`bootstrap/README.md`](bootstrap/README.md) donne l'ordre de lecture. |
| `CLAUDE.md` | Guide de travail, lu en entier à chaque session. Reste **sous 150 lignes** (vérifié par `./check`) : le détail va dans un skill. |
| `.claude/skills/` | Les 6 skills `hean-*` repris de V1 — le savoir par zone. Leurs chemins de fichiers sont ceux de V1 et sont à adapter au fur et à mesure. |
| `.claude/agents/` | `test-runner` (lance tout, résumé court) · `invariant-guardian` (relit un diff contre les invariants). |
| `.claude/hooks/` | `invariant-reminder.sh` — propose d'ajouter une règle quand du code change sans que les invariants bougent. |
| `docs/adr/` | Un fichier par décision, numéroté, immuable. |
| `docs/HISTORY.md` | Journal des sessions, une entrée par session, en bas. |
| `tools/` | Les vérifications appelées par `./check`. |
| `fixtures/` | Fixtures de test — notamment [`fixtures/audio/`](fixtures/audio/README.md), l'actif le plus réutilisable du projet. |

## Ce que `./check` vérifie aujourd'hui

- **liens markdown** — tout lien relatif résout ; aucune référence hors du dépôt
  *(leçon V1 : le meilleur document design du projet vivait hors du dépôt git)* ;
- **taille de `CLAUDE.md`** — ≤ 150 lignes *(leçon V1 : il avait fini par être tronqué à la lecture)* ;
- **taille des fichiers source** — ≤ 400 lignes *(leçon V1 : `lib.rs` 4 870, `views.js` 3 512,
  `styles.css` 10 654 en un seul fichier)* ;
- **skills** — frontmatter exploitable.

La liste des vérifications à ajouter, dans l'ordre, est en commentaire en tête de
[`check`](check) et spécifiée dans [`bootstrap/06-setup-v2/HARNAIS.md`](bootstrap/06-setup-v2/HARNAIS.md).

## Décidé, et qui ne se rouvre pas

Tauri v2 + Rust · CoreAudio direct (**jamais** CPAL) · bit-perfect sans resampling · macOS 11+
uniquement · hors Mac App Store (aucun entitlement sandbox n'existe pour l'accès exclusif à un device
de sortie) · distribution directe, licence perpétuelle.

## À trancher en phase 0

Langage frontend · framework UI · IPC typée générée · store de bibliothèque · nom du dossier de
données · typographie. Les options et mes recommandations sont dans
[`bootstrap/06-setup-v2/PLAN-DE-RECONSTRUCTION.md`](bootstrap/06-setup-v2/PLAN-DE-RECONSTRUCTION.md).

## Deux choses à faire avant d'écrire du code

1. **Capturer V1 visuellement** — aucune capture d'écran n'existe. Sans elles, « le design à
   l'identique » n'est pas vérifiable. → `bootstrap/02-design/reference/captures/`
2. **Mesurer V1** — démarrage avec 10 000 pistes, images par seconde du virtual scroll, mémoire après
   1 000 covers, temps de recherche. Sinon les budgets de performance du harnais seront des souhaits
   au lieu de faits.

Dans les deux cas : V1 est la seule référence, et elle ne sera plus dans le même état plus tard.

---

Hean V1 (v0.2.0-beta.1, distribuée à ~50 testeurs) reste dans
[`thomasdugue/noirdesktop`](https://github.com/thomasdugue/noirdesktop).
