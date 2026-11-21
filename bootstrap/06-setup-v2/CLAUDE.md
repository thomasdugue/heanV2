# CLAUDE.md — heanV2

> **Template.** À poser à la racine de heanV2 et à compléter au fur et à mesure. Les blocs
> `⟦À TRANCHER⟧` attendent une décision (voir `heanv2-bootstrap/06-setup-v2/PLAN-DE-RECONSTRUCTION.md`,
> phase 0). Les blocs `⟦À REMPLIR⟧` se remplissent quand le code existe.
>
> Ce fichier est volontairement **court** : il est lu en entier à chaque session. Le détail vit dans
> `.claude/skills/`. Si ce fichier dépasse ~150 lignes, c'est qu'il faut déplacer quelque chose dans
> un skill. (Leçon V1 : le `CLAUDE.md` était devenu trop gros pour être lu intégralement, et
> l'historique des sessions a dû en être extrait.)

## Projet

**heanV2** — réécriture de Hean, lecteur de musique audiophile macOS. Objectif : **même design, même
feature set** que Hean v0.2.0-beta.1, sur une base technique saine.

Référence de parité : `heanv2-bootstrap/01-produit/FEATURE-SET.md` (liste à cocher).
Postmortem de V1 : `heanv2-bootstrap/05-postmortem/DETTE-TECHNIQUE-V1.md`.

## Stack

⟦À TRANCHER⟧
- Framework : Tauri v2 (décidé, ne se rouvre pas)
- Backend : Rust (décidé)
- Audio : CoreAudio direct via `coreaudio-rs` + `coreaudio-sys` — **jamais CPAL** (décidé)
- Frontend : ⟦TypeScript strict + ? (Svelte / Solid / Lit / vanilla + Vite)⟧
- IPC : ⟦tauri-specta (types générés) ?⟧
- Store bibliothèque : ⟦SQLite via rusqlite ?⟧
- Dossier de données : ⟦`~/Library/Application Support/hean/` + migration depuis `noir/` ?⟧

## Skills — quoi charger avant de travailler

| Zone touchée | Skill |
|---|---|
| **Toute modif de code, toute review de diff** | `hean-invariants` |
| Moteur audio, EQ, resampler, devices/AirPlay, media keys | `hean-audio` |
| NAS/SMB, streaming réseau | `hean-smb` |
| Vues, covers, playlists, design | `hean-frontend` |
| Feedback, Sentry, logs, worker, dashboard | `hean-ops` |
| Build release, signature, notarisation, updater | `hean-release` |

**Agents** : `test-runner` (lance tout, résumé court, ne corrige rien) · `invariant-guardian` (relit
un diff contre les invariants, verdict BLOQUANT / AVERTISSEMENT / OK, lecture seule).

Historique des sessions : `docs/HISTORY.md` — **une entrée par session, en bas.**

## Commands

⟦À REMPLIR⟧

```bash
# dev
# type-check
# lint
# tests frontend
# tests Rust
# release signée + notarisée
```

## Architecture

⟦À REMPLIR au fur et à mesure — garder cette section sous 20 lignes, le détail va dans un skill⟧

## Règles de travail (reprises de V1, elles ont fait leurs preuves)

- **Ne jamais modifier l'audio quand tu travailles sur l'UI ou la bibliothèque. Et inversement.**
  La règle de zone ne vise que les chemins Rust de l'audio ; les fichiers frontend du player sont de
  l'UI malgré leur nom.
- **Un prompt = une tâche.** Ne pas toucher à ce qui est hors scope.
- **Aucun fichier ne dépasse ~400 lignes.** Si ça arrive, c'est que le découpage est à revoir.
- **Toute donnée affichée passe par un échappement** (y compris les guillemets).
- **Répondre toujours en français.**

## Invariants

Source de vérité : `.claude/skills/hean-invariants/SKILL.md`. Résumé d'une ligne par règle ici.

Au démarrage de V2, les 19 invariants de V1 sont triés en trois piles :
1. **Rayés** — devenus impossibles par construction (avec la justification écrite).
2. **Devenus des tests** — la règle est dans la suite de tests, plus dans la tête de quelqu'un.
3. **Irréductibles** — pièges de plateforme (CoreAudio/AirPlay, singleton libsmbclient, media keys,
   drag WebView, petits CSS Grid WebKit, LC_RPATH dyld, notarisation du DMG). Ceux-là restent des
   règles écrites, avec leur gravité.

⟦À REMPLIR : le résumé numéroté, au fur et à mesure⟧

## Protocole de session

**Début** : (1) lire ce fichier + le skill de la zone · (2) lancer l'agent `test-runner` — noter
X pass / Y fail. Si des tests échouent **avant** de commencer, le signaler : on corrige d'abord.

**Fin** : relancer `test-runner` et comparer au début, puis `invariant-guardian` sur le diff
(un verdict BLOQUANT = corriger avant la PR). Un test qui passait et échoue = **régression**, à
corriger avant de terminer. Mettre à jour « Bugs connus » ci-dessous, ajouter une entrée dans
`docs/HISTORY.md`, et si une nouvelle règle est née, l'ajouter à `hean-invariants` **avec sa gravité**
(+ une ligne au résumé ci-dessus). Un hook `Stop` le rappelle quand du code a changé sans que les
invariants bougent.

## Bugs connus

⟦À REMPLIR⟧
