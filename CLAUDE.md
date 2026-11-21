# CLAUDE.md — heanV2

> **Template.** À poser à la racine de heanV2 et à compléter au fur et à mesure. Les blocs
> `⟦À TRANCHER⟧` attendent une décision (voir `heanv2-bootstrap/06-setup-v2/PLAN-DE-RECONSTRUCTION.md`,
> phase 0). Les blocs `⟦À REMPLIR⟧` se remplissent quand le code existe.
>
> Ce fichier est volontairement **court** : il est lu en entier à chaque session. Le détail vit dans
> `.claude/skills/`. Si ce fichier dépasse ~150 lignes, c'est qu'il faut déplacer quelque chose dans
> un skill. (Leçon V1 : le `CLAUDE.md` était devenu trop gros pour être lu intégralement, et
> l'historique des sessions a dû en être extrait.)

## Décisions

Toute décision structurante va dans [`docs/adr/`](docs/adr/) — un fichier numéroté, immuable, avec le
contexte, les alternatives écartées **et pourquoi**. On ne modifie pas un ADR : on en écrit un nouveau
qui le remplace. Leçon V1 : les « décisions actées » s'accumulaient dans un skill, on y perdait le
*pourquoi*, donc la règle finissait par être supprimée par quelqu'un qui la trouvait arbitraire.

## Projet

**heanV2** — réécriture de Hean, lecteur de musique audiophile macOS. Objectif : **même design, même
feature set** que Hean v0.2.0-beta.1, sur une base technique saine.

Référence de parité : `heanv2-bootstrap/01-produit/FEATURE-SET.md` (liste à cocher).
Postmortem de V1 : `heanv2-bootstrap/05-postmortem/DETTE-TECHNIQUE-V1.md`.

## Stack — tranchée le 2026-11-21 (ADR [0002](docs/adr/0002-stack-technique.md), [0003](docs/adr/0003-dossier-de-donnees.md))

| | Choix | Pourquoi, en une ligne |
|---|---|---|
| Framework | **Tauri v2** | démontré en production |
| Backend | **Rust** | idem |
| Audio | **CoreAudio direct** (`coreaudio-rs` + `coreaudio-sys`) — **jamais CPAL** | seul chemin vers le bit-perfect et le hog mode |
| Frontend | **TypeScript strict + Vite** | cause racine n°1 de V1 : 16 076 lignes de JS sans types |
| UI | **Svelte 5 (runes)** | compilé, CSS scopé, remplace l'état partagé par référence |
| IPC | **tauri-specta** (types générés depuis le Rust) | un renommage de commande casse la compilation du front |
| Store bibliothèque | **SQLite** (`rusqlite`, `bundled`) | transactions → 4 invariants bloquants disparaissent |
| Dossier de données | **`~/Library/Application Support/hean/`** + migration one-shot depuis `noir/` | nom cohérent, testeurs préservés |
| Cible de parité visuelle | **le redesign v3, commit `e12a200`** (ADR [0004](docs/adr/0004-cible-de-parite.md)) — **pas** la beta distribuée | trois designs de Hean existent, deux étaient hors de `main` ; celui-ci est le seul complet et non cassé |

**Svelte 5 uniquement.** Les runes (`$state`, `$derived`, `$effect`) et non les stores Svelte 4
(`writable`, `$:`). Beaucoup d'exemples en ligne sont encore en Svelte 4 : si un doute, vérifier la
doc officielle Svelte 5, ne pas mélanger les deux syntaxes.

**Reste à trancher** : la typographie (Geist Mono partout, comme le fait V1, ou Geist Mono +
DM Sans embarquée pour les grands titres). Décision reportée jusqu'à vérification sur V1 — rien en
phase 0 n'en dépend. Voir [`bootstrap/02-design/ECARTS-DESIGN.md`](bootstrap/02-design/ECARTS-DESIGN.md) §2.

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

```bash
./check            # TOUT : liens, tailles, skills, types, tests. Identique en local et en CI.
./check --fast     # seulement les vérifications rapides (pre-commit)
```

`./check` est le point d'entrée **unique**. Quand une vérification manque, on l'ajoute dedans — on ne
crée pas une deuxième commande. La liste de ce qui reste à y brancher est en commentaire en tête du
fichier [`check`](check), chacune annotée de sa phase.

⟦À REMPLIR au fur et à mesure : dev, release signée + notarisée⟧

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

## Règles de performance (voir [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md))

La promesse du produit est une app légère, peu gourmande, aux performances redoutables. Trois règles
la protègent — les deux premières sont des pièges de Svelte 5, la troisième est le seul risque réel :

1. **La bibliothèque n'est jamais réactive.** `$state.raw` (ou hors de tout état réactif) pour la
   liste de pistes. La réactivité ne couvre que l'état d'UI : vue, tri, sélection, filtre. Envelopper
   10 000 pistes dans des signaux multiplie la mémoire et le coût de chaque mutation.
2. **Le virtual scroll reste explicite** : un pool fixe de 60 nœuds recyclés, item à 48 px,
   délégation d'événements. C'est ce qui tient 60 fps sur 10 000+ pistes en V1 ; aucun framework ne
   fera mieux en rendant 10 000 lignes.
3. **Zéro dépendance npm de production sans ADR** — ce qu'elle apporte, son poids mesuré, pourquoi on
   ne l'écrit pas à la main. V1 en avait zéro ; `./check` applique un budget.

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
