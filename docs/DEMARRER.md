# Démarrer la construction de heanV2

Marche à suivre pour lancer la phase 0 avec un autre modèle (Fable 5.2 ou autre). Écrit le
2026-11-21, une fois les six arbitrages pris.

---

## A. Cinq préalables, avant d'écrire du code

Aucun n'est long. Chacun, oublié, coûte cher plus tard.

### A1. Figer la cible de parité 🔴

**C'est le point le plus important, et il n'était pas dans le plan.**

V1 a ~50 beta-testeurs actifs, un Sentry qui remonte des crashs, un dashboard, des issues GitHub. La
reconstruction va durer des semaines. Pendant ce temps, si V1 continue d'évoluer, **la cible de
parité bouge pendant qu'on la vise** — et on ne finit jamais.

Décision à prendre maintenant, et à écrire dans un ADR :

> La parité se mesure contre **v0.2.0-beta.1** (tag + le commit `02508ea`), pas contre `main` de
> `noirdesktop`.

Puis choisir un régime pour V1 pendant la reconstruction :

| Régime | Conséquence |
|---|---|
| **Gel total** | Les testeurs attendent. Le plus simple, le plus honnête, et il faut le leur dire. |
| **Hotfix critiques seulement** (crash, perte de données) | Raisonnable. Chaque hotfix s'ajoute explicitement au contrat de parité comme une ligne supplémentaire. |
| **V1 continue d'évoluer** | À éviter. La cible bouge, et le travail de parité est à refaire en continu. |

### A2. Capturer V1 visuellement 🔴

**Aucune capture d'écran n'existe.** Le CSS seul ne dit pas à quoi ressemble le résultat composé, et
`bootstrap/02-design/reference/noir-design-system.html` §24 donne les specs, pas le rendu. Sans
captures, la contrainte n°2 du brief — « même design » — n'est **pas vérifiable**.

```bash
cd /Users/tsunami25/Documents/Thomas/noirdesktop/noir-tauri && ./scripts/dev.sh
```

Destination : `bootstrap/02-design/reference/captures/`. La liste des écrans à couvrir est dans
[`../bootstrap/02-design/reference/README.md`](../bootstrap/02-design/reference/README.md).

Urgence réelle : ces captures doivent être prises **depuis l'état actuel** de ta machine
(bibliothèque scannée, caches chauds, NAS configuré, playlists réelles). Cet état est local et
unique — il n'est pas dans git.

### A3. Mesurer V1 🟠

Les budgets de performance du harnais doivent être des **faits**, pas des souhaits. À mesurer sur V1,
avant de commencer : temps de démarrage jusqu'à UI utilisable avec la vraie bibliothèque, images par
seconde du virtual scroll de la vue Pistes, empreinte mémoire après avoir fait défiler ~1 000 covers,
temps de réponse de la recherche. Noter les chiffres dans
[`../bootstrap/06-setup-v2/HARNAIS.md`](../bootstrap/06-setup-v2/HARNAIS.md) §8.

### A4. Vérifier la typographie 🟠

Dix minutes, et ça débloque le dernier arbitrage. Lancer V1, inspecter la police **calculée** sur le
corps de texte et sur un titre de section, et regarder la console pour une erreur CSP sur
`fonts.googleapis.com`. Détail du raisonnement :
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
> On fait la **phase 0 et rien d'autre** — aucune feature. Dans cet ordre :
>
> 1. Le socle : `package.json`, `vite.config.ts`, `tsconfig.json` en `strict`, `src-tauri/` avec
>    `Cargo.toml` et `tauri.conf.json` (reprends la CSP de
>    `bootstrap/03-architecture/code-reference/tauri.conf.json.v1`, qui est bonne, **sauf** le trou
>    Google Fonts), `build.rs` qui échoue avec `compile_error!()` si un secret compile-time manque en
>    profil release, et le câblage tauri-specta qui génère les types TS.
> 2. Brancher dans `./check` : `tsc --noEmit`, le lint, les tests frontend, `cargo clippy -- -D
>    warnings`, `cargo test --locked`. `./check` doit rester le point d'entrée unique.
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
