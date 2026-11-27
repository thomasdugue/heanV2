# Démarrer la construction de heanV2

Marche à suivre pour lancer la phase 0 avec un autre modèle (Fable 5.2 ou autre). Écrit le
2026-11-21, une fois les six arbitrages pris.

> **Révisé le soir du 2026-11-21.** Deux refontes de design non mergées et un chantier WiFi Sync
> complet ont été retrouvés hors de `main`, dont deux qui n'étaient commités nulle part. La cible de
> parité a changé (voir [`adr/0004`](adr/0004-cible-de-parite.md)), et les préalables A1, A2 et A4
> avec elle.

---

## A. Cinq préalables, avant d'écrire du code

Aucun n'est long. Chacun, oublié, coûte cher plus tard.

### A1. Mettre la cible de parité à l'abri 🔴

**La cible est tranchée** : le commit `e12a200` (redesign v3 porté sur main d'octobre) —
[`adr/0004`](adr/0004-cible-de-parite.md). Ce n'est **pas** `v0.2.0-beta.1`, comme le disait la
première version de ce document.

Ce qui reste à faire tient en une ligne : **ce commit n'est pas poussé.** Il ne vit que dans le clone
local `noirdesktop-v1`, sur la branche `thomasdugue/design-migration-audit`. Tant qu'il n'est pas sur
GitHub, une panne de disque emporte la référence visuelle de V2.

```bash
cd $HOME/conductor/workspaces/noirdesktop-v1/yellowknife && git push -u origin thomasdugue/design-migration-audit
```

Reste aussi à choisir un régime pour V1 pendant la reconstruction — moins urgent qu'il n'y paraît,
puisque le travail de design sur V1 est arrêté depuis octobre :

| Régime | Conséquence |
|---|---|
| **Gel total** | Les testeurs attendent. Le plus simple, le plus honnête, et il faut le leur dire. |
| **Hotfix critiques seulement** (crash, perte de données) | Raisonnable. Chaque hotfix s'ajoute explicitement au contrat de parité comme une ligne supplémentaire. **L'auto-updater cassé entre dans cette catégorie** : aucun testeur n'a jamais reçu de mise à jour ([`MESURES-V1.md`](MESURES-V1.md) §6). |
| **V1 continue d'évoluer** | À éviter. La cible bouge, et le travail de parité est à refaire en continu. |

### A2. Capturer la cible visuellement 🟠 ~~bloquant~~ — **fait pour la validation, en pause pour la référence**

**Le 2026-11-21 à 18 h 51, la cible a été lancée pour la première fois.** Huit captures, **retirées
du dépôt le 2026-11-27** avant la bascule en public ([ADR 0020](adr/0020-les-captures-sortent-du-depot.md)) :
ce qu'elles montraient, écran par écran, est consigné dans
[`../bootstrap/02-design/reference/captures/`](../bootstrap/02-design/reference/captures/README.md).

Ces captures avaient deux rôles. **Le premier est rempli** : la cible est validée — les `--glass-*`
sont bien rétablies, DM Sans se charge, la structure du redesign v3 tient. Et trois défauts ont été
trouvés au passage (`MESURES-V1.md` §9 et §10).

**Le second — servir de référence visuelle à V2 — est en pause**, volontairement : Thomas fait une
passe « davantage de DM Sans » côté design, qui changera la typographie de tous les écrans. La série
sera reprise **en entier** après, et la cible de parité changera avec elle (nouvel ADR remplaçant
`0004`).

**Ce que ça débloque** : plus rien n'attend. La phase 0 ne touche pas au design — socle, `./check`,
tests d'architecture, tri des invariants, traduction des 24 comportements en tests. Elle peut
démarrer pendant la passe de design.

Ce qui suit reste valable pour reprendre la série.

⚠️ **À prendre sur le bon commit.** Pas sur l'app installée, pas sur `main` : sur `e12a200`.

**Le protocole complet est dans [`PROTOCOLE-CAPTURES.md`](PROTOCOLE-CAPTURES.md)** — lancement, les
30 captures dans l'ordre d'un parcours, et la vérification typographique à faire dans la même
session. En deux lignes :

```bash
cd $HOME/Documents/Thomas/noirdesktop && git checkout thomasdugue/design-migration-audit
```

```bash
cd $HOME/Documents/Thomas/noirdesktop/noir-tauri && ./scripts/dev.sh
```

C'est bien le **checkout principal** et pas le workspace Conductor : lui seul a `scripts/.env.local`
(sans quoi `dev.sh` s'arrête net) et 17 Go de cache de build Rust.

Destination : **locale, hors de git** tant que le dépôt est public ([ADR 0020](adr/0020-les-captures-sortent-du-depot.md)).
La liste des écrans à couvrir est dans
[`../bootstrap/02-design/reference/README.md`](../bootstrap/02-design/reference/README.md).

Urgence réelle : ces captures doivent être prises **depuis l'état actuel** de ta machine
(bibliothèque scannée, caches chauds, NAS configuré, playlists réelles). Cet état est local et
unique — il n'est pas dans git.

Et si le rendu ne te plaît pas, c'est maintenant qu'il faut le dire : la cible de parité se change
par un nouvel ADR, pas en cours de route.

### A3. Mesurer V1 🟠

Les budgets de performance du harnais doivent être des **faits**, pas des souhaits. À mesurer sur V1,
avant de commencer : temps de démarrage jusqu'à UI utilisable avec la vraie bibliothèque, images par
seconde du virtual scroll de la vue Pistes, empreinte mémoire après avoir fait défiler ~1 000 covers,
temps de réponse de la recherche. Noter les chiffres dans
[`../bootstrap/06-setup-v2/HARNAIS.md`](../bootstrap/06-setup-v2/HARNAIS.md) §8.

### A4. Vérifier la typographie 🟠

Dix minutes, et ça débloque le dernier arbitrage de design.

**Déjà tranché** par la cible de parité : l'échelle de tailles. Une seule, celle du `:root`
(10/11/12/13/14/15/18/22/28/48). Le prototype v4 en portait une plus petite, et la migration
d'octobre a explicitement refusé de l'appliquer — la raison est écrite dans le CSS : *« ils
rétréciraient de 1-2px tout le texte de l'app (onboarding inclus) »*.

**Reste ouvert** : le partage des rôles entre Geist Mono et DM Sans. Lancer la cible, inspecter la
police **calculée** sur le corps de texte et sur un titre de section, et regarder la console pour une
erreur CSP sur `fonts.googleapis.com`. L'hypothèse est que DM Sans ne se charge pas du tout dans
l'app packagée — or le redesign v3 la charge jusqu'au poids 900 pour le watermark artiste, donc le
design la suppose présente. Détail du raisonnement :
[`../bootstrap/02-design/ECARTS-DESIGN.md`](../bootstrap/02-design/ECARTS-DESIGN.md) §2.

### A5. Rapatrier les secrets 🟢

Le nouveau dépôt n'a aucun secret. Avant la première release (phase 7, pas avant) :
`scripts/.env.local` avec les 6 valeurs — modèle dans
[`../bootstrap/06-setup-v2/ci/secrets.env.template`](../bootstrap/06-setup-v2/ci/secrets.env.template).
Ne pas découvrir ça le jour de la release.

---

## B. Changer de modèle

Le modèle se choisit **par session**, pas par dépôt.

1. Ouvrir une nouvelle session Claude Code sur `~/Documents/Thomas/heanV2`.
2. `/model` → choisir Fable 5.2 (`claude-fable-5-2`).
3. Vérifier que la session a bien chargé le dépôt : `CLAUDE.md` doit apparaître dans le contexte, et
   les skills `hean-*` doivent être listés comme disponibles.

Si Fable 5.2 n'apparaît pas dans `/model`, c'est une question de disponibilité sur le compte, pas de
configuration du dépôt — le harnais est indépendant du modèle.

**Un point de méthode qui compte plus que le choix du modèle** : ce dépôt contient 114 fichiers de
brief. Un modèle lâché dessus sans consigne va vouloir tout lire, et brûler son contexte avant
d'écrire une ligne. L'ordre de lecture est donc **explicite dans le prompt de démarrage** ci-dessous :
quatre fichiers, pas cent quatorze. Le reste se charge à la demande, par zone, via les skills.

---

## C. Prompt de démarrage de la phase 0

À coller tel quel dans la nouvelle session :

> Projet : heanV2, réécriture de Hean (lecteur de musique audiophile macOS, Tauri v2 + Rust).
> Contrainte : même design et même feature set que V1, base technique saine.
>
> Lis dans cet ordre, et **seulement** ces fichiers pour commencer :
> 1. `CLAUDE.md`
> 2. `bootstrap/00-BRIEF-V2.md`
> 3. `docs/adr/0002-stack-technique.md` et `docs/adr/0003-dossier-de-donnees.md`
> 4. `bootstrap/06-setup-v2/HARNAIS.md`
>
> La stack est tranchée, ne la rouvre pas : TypeScript strict + Vite, Svelte 5 (runes uniquement,
> jamais les stores Svelte 4), tauri-specta pour l'IPC, SQLite via rusqlite, données dans
> `~/Library/Application Support/hean/` avec migration depuis `noir/`.
>
> La cible de parité visuelle est tranchée aussi : le commit `e12a200` (redesign v3), dont le CSS et
> le markup sont dans `bootstrap/02-design/reference/`. Pas la version distribuée aux testeurs. Ne la
> rouvre pas non plus — c'est `docs/adr/0004-cible-de-parite.md`.
>
> On fait la **phase 0 et rien d'autre** — aucune feature. Dans cet ordre :
>
> 1. Le socle : `package.json`, `vite.config.ts`, `tsconfig.json` en `strict`, `src-tauri/` avec
>    `Cargo.toml` et `tauri.conf.json` (reprends la CSP de
>    `bootstrap/03-architecture/code-reference/tauri.conf.json.v1`, qui est bonne, **sauf** le trou
>    Google Fonts), `build.rs` qui échoue avec `compile_error!()` si un secret compile-time manque en
>    profil release, et le câblage tauri-specta qui génère les types TS.
> 2. Brancher dans `./check` : `tsc --noEmit`, le lint, les tests frontend, `cargo clippy -- -D
>    warnings`, `cargo test --locked`. `./check` doit rester le point d'entrée unique.
> 2bis. L'entretien du harnais — lis `bootstrap/06-setup-v2/ENTRETIEN-DU-HARNAIS.md` et pose ses
>    deux pièces de phase 0 : les **générateurs de niveau 1** (types TS, catalogue IPC, tokens) avec
>    la CI qui échoue sur la dérive, et le **contrat de fin de tâche** — le hook qui réclame ce qui
>    manque quand un diff touche une zone. Le hook `invariant-reminder.sh` de V1 en est le modèle :
>    généralise-le, et garde sa porte de sortie explicite.
> 2ter. L'orchestration — lis `bootstrap/06-setup-v2/ORCHESTRATION.md` et pose la politique de
>    routage (un `.claude/agents/` par rôle avec son modèle), le gabarit de brief à six points, et la
>    liste en dur des zones jamais déléguées. Règle à respecter toi-même dès cette phase : **tu ne
>    délègues une tâche que si tu peux écrire son critère de réussite exécutable avant de la
>    lancer.**
> 3. Les tests d'architecture (`HARNAIS.md` §2) : pas d'imports circulaires, un seul constructeur de
>    liste de pistes, `audio/` n'importe jamais `ui/`, aucune couleur en dur hors du fichier de
>    tokens.
> 4. Trier les 19 invariants de `bootstrap/04-invariants/INVARIANTS.md` en trois piles — rayés
>    (impossibles par construction, avec la justification écrite), devenus des tests, irréductibles —
>    dans un nouveau `.claude/skills/hean-invariants/SKILL.md` propre à V2. Puis le script
>    anti-orphelin de `HARNAIS.md` §6.
> 5. Traduire les 24 comportements de `bootstrap/01-produit/UX-ANTI-REGRESSION.md` en tests. Ils
>    échouent tous : c'est normal, c'est le tableau de bord du projet.
>
> Terminé quand `./check` est vert sauf sur les tests de comportement, que le fichier d'invariants de
> V2 existe et est rempli, et qu'un commit est en place. **Ne commence pas la phase 1** (moteur audio).
>
> Protocole : `test-runner` au début et à la fin, `invariant-guardian` sur le diff avant la PR, une
> entrée dans `docs/HISTORY.md`, un ADR par décision structurante. Réponds en français.

---

## D. Et ensuite

L'ordre des phases est dans
[`../bootstrap/06-setup-v2/PLAN-DE-RECONSTRUCTION.md`](../bootstrap/06-setup-v2/PLAN-DE-RECONSTRUCTION.md).
Deux rappels qui comptent :

- **Phase 1 = le moteur audio, pas l'UI.** C'est contre-intuitif et c'est pour ça que c'est écrit :
  l'audio est la partie où une erreur de conception est irrattrapable, et le reste en dépend. Les
  modules CoreAudio et SMB se **portent**, ne se réécrivent pas — voir
  [`../bootstrap/03-architecture/code-reference/README.md`](../bootstrap/03-architecture/code-reference/README.md).
- **Une phase ne commence pas avant que la précédente soit verte.** Les critères de sortie sont
  écrits pour ça. C'est le seul garde-fou contre le scénario « 80 % de six phases et rien qui
  marche ».
