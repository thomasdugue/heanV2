# heanV2

Réécriture complète de **Hean**, lecteur de musique audiophile pour macOS : même design, même feature
set, base technique saine.

**État : phase 0 faite ; phase 1 entamée par ce qui rend l'audio vérifiable.** Le socle compile et
démarre, le filet anti-régression tourne, et **« bit-perfect » n'est plus une promesse** : un test
décode une fixture FLAC vers un WAV à travers un backend de test et compare le résultat *octet par
octet* à un fichier écrit par ffmpeg, donc par un décodeur indépendant du nôtre — en CI, sans
matériel ([ADR 0007](docs/adr/0007-format-interne-audio-et-preuve-du-bit-perfect.md)). Le moteur
CoreAudio lui-même, l'égaliseur et le gapless ne sont pas portés.

La **bibliothèque** existe depuis la phase 2 : scan du disque, métadonnées lues et écrites dans le
fichier audio, rangement dans SQLite sous transaction, schéma versionné
([ADR 0009](docs/adr/0009-bibliotheque-cle-dalbum-et-source-de-verite.md)). Elle n'a pas encore
d'interface. Le reste du contrat de parité est à porter, et les comportements anti-régression encore
rouges le sont exprès : leur compte est le tableau de bord du projet.

---

## Démarrer

```bash
./check
```

Vert, build compris. `check` est le point d'entrée **unique** : identique en local, en pre-commit et
en CI — la CI est un seul job qui lance `./check` et rien d'autre. Et « identique » veut dire
identique **jusqu'aux versions des outils** : `package-lock.json` fige celles de npm,
[`rust-toolchain.toml`](rust-toolchain.toml) celle de Rust, et `./check` vérifie que la version qui
tourne est bien celle qui est épinglée ([ADR 0008](docs/adr/0008-epingler-la-chaine-doutils.md)).
Sans ce dernier point, la promesse était à moitié fausse, et elle l'a été jusqu'au 2026-11-22. Il installe lui-même les dépendances npm si elles manquent.

Pour voir l'application :

```bash
npm run tauri dev
```

---

## Organisation du dépôt

| Chemin | Contenu |
|---|---|
| `bootstrap/` | **Tout le contexte repris de V1** — brief, contrat de parité (124 features), design de la cible, invariants, postmortem, chantiers repris (MTP + WiFi Sync), plan de reconstruction, spécification du harnais. Autonome : [`bootstrap/README.md`](bootstrap/README.md) donne l'ordre de lecture. |
| `CLAUDE.md` | Guide de travail, lu en entier à chaque session. Reste **sous 150 lignes** (vérifié par `./check`) : le détail va dans un skill. |
| `src/`, `src-tauri/` | Le socle. `src/ipc/` est le **seul** module qui parle à Tauri ; `src-tauri/src/secrets.rs` le seul qui lit un secret. Des tests d'architecture le garantissent. |
| `tests/` | `architecture/` (la forme du code) · `ux/` (les 24 comportements anti-régression) · `unit/`. |
| `docs/generated/` | Catalogues **générés** par `npm run gen` — IPC, tokens, couverture des invariants et des comportements. Ne jamais les éditer : `./check` échoue si ils diffèrent du code. |
| `.claude/skills/` | 7 skills `hean-*` — le savoir par zone, dont `hean-invariants` (les 27 règles) et `hean-orchestration` (quand déléguer). |
| `.claude/agents/` | `test-runner` (lance `./check`, résumé court) · `invariant-guardian` (relit un diff contre les invariants). Les deux en lecture seule. |
| `.claude/hooks/` | `contrat-de-fin-de-tache.sh` — réclame ce qui manque quand un diff touche une zone, avec une porte de sortie explicite. |
| `docs/adr/` | Un fichier par décision, numéroté, immuable. |
| `docs/MESURES-V1.md` | Les mesures de référence de V1 : échelle de la bibliothèque, démarrage, mémoire par processus, et deux bugs trouvés en mesurant. |
| `docs/PERFORMANCE.md` | Les chiffres réels de V1, l'impact de la stack sur poids/mémoire/vitesse, et les 3 règles qui protègent la promesse de légèreté. |
| `docs/HISTORY.md` | Journal des sessions, une entrée par session, en bas. |
| `tools/` | Les vérifications appelées par `./check`. |
| `fixtures/` | Fixtures de test — notamment [`fixtures/audio/`](fixtures/audio/README.md), l'actif le plus réutilisable du projet. |

## Ce que `./check` vérifie aujourd'hui

- **liens markdown** — tout lien relatif résout *(leçon V1 : le meilleur document design du projet
  vivait hors du dépôt git)* ;
- **taille de `CLAUDE.md`** ≤ 150 lignes · **taille des fichiers source** ≤ 400 lignes
  *(leçon V1 : `lib.rs` 4 870, `views.js` 3 512, `styles.css` 10 654 en un seul fichier)* ;
- **invariants** — chaque règle a une pile, une gravité, une couverture et un *pourquoi* ; une règle
  sans test échoue, un test qui cite une règle disparue aussi, et **les compteurs annoncés sont
  vérifiés** *(leçon V1 : cinq documents annonçaient 19 invariants, il y en avait 20)* ;
- **zones jamais déléguées** — la liste ne peut pas diverger entre ses deux copies ;
- **chaîne d'outils** — la version de Rust qui compile est bien celle qui est épinglée
  *(leçon du 2026-11-22 : la CI tournait avec un clippy plus récent que la machine de Thomas, donc
  le même code était vert en local et rouge en CI)* ;
- **fixtures audio** — les 24 fixtures et leurs goldens sont appariés au README dans les deux sens,
  bornés en poids, et **chacune doit être lue par au moins un test** : un binaire commité que
  personne ne lit n'est pas une fixture, c'est du décor ;
- **lint** (oxlint) · **types** (`tsc` en `strict` + `svelte-check` en échec sur avertissement)
  *(cause racine n°1 de V1 : 16 076 lignes de JS sans analyse statique)* ;
- **tests d'architecture** — sur la *forme* du code : aucun import circulaire, un seul
  constructeur de liste de pistes, les zones ne se traversent pas, aucune couleur en dur hors des
  tokens, toute `var(--x)` lue est définie, aucun `println!` dans le backend ;
- **build** — le frontend compile *(un `./check` vert sur un projet qui ne compile pas serait
  exactement la fausse confiance que ce harnais existe pour empêcher)* ;
- **clippy `-D warnings`** et **`cargo test --locked`** ;
- **artefacts générés** — régénérés et comparés au commit : la dérive de documentation devient
  impossible *(cause racine n°5 : 11 affirmations fausses au moment de l'audit)* ;
- **poids et dépendances** — budget sur `dist/` hors sourcemaps et sur les dépendances npm de
  production *(V1 en avait zéro : pas de bundler, donc pas de porte d'entrée)* ;
- **comptes annoncés** — tout nombre écrit en prose sur le dépôt doit correspondre à la réalité
  *(leçon du 2026-11-22 : ce README a annoncé « 29 cas » de tests d'architecture là où il y en avait
  32 — vrai à l'écriture, faux un commit plus tard)*.

Hors verdict, en **tableau de bord** : les 24 comportements de `UX-ANTI-REGRESSION.md`. Chacun reste
rouge tant que la feature correspondante n'est pas portée, et `./check` affiche le compte à chaque
passage — il se déduit du nombre de tests rouges, pour qu'aucun nombre écrit à la main ne puisse le
faire mentir.

La liste des vérifications à ajouter, dans l'ordre, est en commentaire en tête de
[`check`](check) et spécifiée dans [`bootstrap/06-setup-v2/HARNAIS.md`](bootstrap/06-setup-v2/HARNAIS.md).

## Décidé, et qui ne se rouvre pas

Tauri v2 + Rust · CoreAudio direct (**jamais** CPAL) · bit-perfect sans resampling · macOS 11+
uniquement · hors Mac App Store (aucun entitlement sandbox n'existe pour l'accès exclusif à un device
de sortie) · distribution directe, licence perpétuelle.

## Stack tranchée

**TypeScript strict + Vite · Svelte 5 (runes) · tauri-specta · SQLite · données dans `hean/`**
— avec Tauri v2, Rust et CoreAudio direct qui ne se rouvrent pas.
Le raisonnement et les alternatives écartées : [`docs/adr/`](docs/adr/).

**Cible de parité visuelle** : le redesign v3, commit `e12a200` — pas la version distribuée aux
testeurs. Trois designs de Hean existent, deux étaient hors de `main` :
[`docs/adr/0004`](docs/adr/0004-cible-de-parite.md).

Reste ouvert : le partage Geist Mono / DM Sans, et cinq points listés dans
[`docs/adr/README.md`](docs/adr/README.md).

## Avant d'écrire du code

**[`docs/DEMARRER.md`](docs/DEMARRER.md)** — cinq préalables (figer la cible de parité, capturer V1,
le mesurer, vérifier la typographie, rapatrier les secrets) et comment changer de modèle.

Le préalable le plus important n'est pas technique : **figer la cible**. Si V1 continue d'évoluer
pendant la reconstruction, la parité est une cible mobile et le travail ne se termine jamais.

---

Hean V1 (v0.2.0-beta.1, distribuée à ~50 testeurs) reste dans
[`thomasdugue/noirdesktop`](https://github.com/thomasdugue/noirdesktop).
