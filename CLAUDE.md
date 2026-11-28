# CLAUDE.md — heanV2

> Lu **en entier** à chaque session, donc tenu sous 150 lignes (vérifié) : le détail vit dans
> `.claude/skills/`. Leçon V1 — son `CLAUDE.md` avait fini trop gros pour être lu jusqu'au bout.

## Faits qui ne se redécouvrent pas

**Aucun utilisateur.** La bêta V1 n'a **jamais** été distribuée. Les « ~50 testeurs » qu'annoncent
`bootstrap/` et les ADR 0001, 0003, 0004 et 0010 n'ont jamais existé ([ADR 0017](docs/adr/0017-ce-que-thomas-a-tranche-le-25-novembre.md)).
Conséquence : **aucune compatibilité descendante à tenir.** Changer un format, un identifiant ou un
nom de service ne coûte qu'à Thomas, une fois.

## Décisions

Toute décision structurante va dans [`docs/adr/`](docs/adr/) — numérotée, **immuable**, avec les
alternatives écartées **et pourquoi** (sans le pourquoi, la règle finit supprimée par quelqu'un qui
la trouve arbitraire). On ne modifie pas un ADR : on en écrit un nouveau qui le remplace.

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
| Entretien du harnais | **par mécanisme, jamais par discipline** (ADR [0005](docs/adr/0005-entretien-du-harnais-et-orchestration.md)) : généré > apparié > réclamé. Tout défaut qui atteint Thomas repart avec la vérification qui l'aurait vu | une phrase dans un document ne peut pas échouer ; un test, si |

**Svelte 5 uniquement** : les runes (`$state`, `$derived`, `$effect`), jamais les stores Svelte 4
(`writable`, `$:`). Beaucoup d'exemples en ligne sont encore en Svelte 4 — vérifier la doc officielle.

**Ce qui reste à trancher** est tenu à jour dans [l'index des ADR](docs/adr/README.md), avec la
phase que chaque question bloque.

## Skills — quoi charger avant de travailler

| Zone touchée | Skill |
|---|---|
| **Toute modif de code, toute review de diff** | `hean-invariants` |
| Moteur audio, EQ, resampler, devices/AirPlay, media keys | `hean-audio` |
| NAS/SMB, streaming réseau | `hean-smb` |
| Vues, covers, playlists, design | `hean-frontend` |
| Feedback, Sentry, logs, worker, dashboard | `hean-ops` |
| Build release, signature, notarisation, updater | `hean-release` |
| Déléguer une tâche à un sous-agent | `hean-orchestration` |

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

```bash
npm run dev        # Vite seul (pas de fenêtre native)
npm run tauri dev  # l'app complète
npm run gen        # régénère tout ce qui est généré (./check échoue si ça diffère du commit)
git config core.hooksPath tools/git-hooks   # une fois par machine : ./check --fast au commit
./scripts/release.sh --test   # DMG signé, local, sans secrets ni notarisation — sans --test : distribuable
```

## Architecture

```
src/            frontend · ipc/ (SEUL à parler à Tauri) · styles/tokens.css (GÉNÉRÉ)
src-tauri/      backend  · commands/ (toute commande IPC) · secrets.rs (SEUL à lire un secret)
tests/          architecture/ (forme du code) · ux/ (les 24 comportements) · unit/
tools/          check-*.sh|mjs et gen/ — tout ce que ./check lance
docs/generated/ catalogues produits par npm run gen — ne jamais éditer à la main
```

**Trois points d'entrée uniques**, tenus par des tests d'architecture : l'IPC, les secrets, la liste
de pistes — ce qui fait disparaître « n'oublie pas de répéter ce filtre partout ».

## Règles de travail

- **Répondre toujours en français.**
- **Un prompt = une tâche.** Ne pas toucher à ce qui est hors scope.
- **Une session, un périmètre** — [ADR 0013](docs/adr/0013-une-session-un-perimetre.md). Le brief nomme
  les fichiers tenus ; le reste se remonte. **Une session n'en démarre pas une autre** : un travail
  trouvé en chemin devient une ligne dans « Bugs connus » ou une décision ouverte, et Thomas décide.
  **Jamais deux sessions dans le même fichier** — celle qui le découvre s'arrête, elle n'écrase pas ;
  exception nommée : les journaux en ajout seul (`docs/HISTORY.md`, l'index des ADR).
  **Jamais deux sessions qui écrivent ; autant qu'on veut qui lisent.**
- **Porter d'abord, améliorer ensuite.** Redessiner un sous-système en le portant depuis V1 exige
  un défaut **mesuré** (une ligne de [`docs/MESURES-V1.md`](docs/MESURES-V1.md)), pas une intuition.
- **Un ADR n'existe que pour une décision de Thomas**, ou une qu'il pourrait vouloir annuler. Le
  reste est un message de commit. **Une page maximum.**
- **Les commentaires disent pourquoi, jamais quoi.** Un commentaire ne compile pas : rien ne le
  contredit quand il devient faux.

Les règles qui ne valent que pour le code — zone audio/UI, 400 lignes par fichier, échappement de
toute donnée affichée, et les trois règles de performance — sont dans le skill
[`hean-invariants`](.claude/skills/hean-invariants/SKILL.md), chargé avant toute modif de code.

## Invariants

Source de vérité : [`.claude/skills/hean-invariants/SKILL.md`](.claude/skills/hean-invariants/SKILL.md).
Tableau à jour, **généré** : [`docs/generated/invariants.md`](docs/generated/invariants.md).

**29 règles**, les 20 de V1 plus neuf nées depuis : 1. **Rayées (6)** — inexprimables par
construction. 2. **Devenues des tests (21)**, dont **12 déjà actives** et **9 « planifiées »**,
dette visible que `./check` recompte. 3. **Irréductibles (2 + 7 pièges de plateforme)** —
CoreAudio, WebKit, dyld, Gatekeeper : aucun test ne les prouvera jamais, et ce sont les seules
que le skill te fait encore lire.

## Protocole de session — deux formats

**Passe** — correction, ajustement, réglage. `./check --fast`, commit direct, pas d'ADR.

**Chantier** — une feature, un portage. `test-runner` au début (si c'est rouge **avant** toi, le
dire et corriger d'abord) ; `invariant-guardian` sur le diff à la fin, un verdict BLOQUANT se corrige
avant la PR ; `./check` entier ; PR. Un test qui passait et échoue = **régression**, à corriger avant
de terminer.

**Dans les deux cas** : lire ce fichier + le skill de la zone au début ; à la fin, « Bugs connus »
à jour, une entrée de **15 lignes maximum** dans `docs/HISTORY.md` (vérifié), et toute règle née
ajoutée à `hean-invariants` **avec sa gravité** (+ les comptes ci-dessus).

## Bugs connus

**Un fichier modifié sur le NAS depuis une autre machine n'est pas repris** si le serveur ne
répond pas à `readdirplus` : il manque un « rescan complet d'une source » (ADR 0016 §4).

Repris de V1, à ne pas réintroduire — l'endpoint de mise à jour renvoyait 404 depuis le lancement
(bloc `updater` repris en phase 6, ADR 0006), et `logging.rs` ne capturait pas stdout (INV-21).
