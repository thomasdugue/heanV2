# Manifeste — provenance et exclusions

Traçabilité de chaque fichier du bootstrap. Source : dépôt `noirdesktop`, branche
`claude/refonte-fab-5-2-3901f8`, commit `02508ea`, le 2026-11-21.

## Révision du soir du 2026-11-21 — deux sources ajoutées

Le bootstrap du matin avait été constitué depuis `main` seul. Une vérification demandée par Thomas a
révélé trois chantiers hors de `main`, dont deux commités nulle part. D'où deux nouvelles sources :

| Source ajoutée | D'où | Ce qu'elle apporte ici |
|---|---|---|
| **commit `e12a200`**, branche `thomasdugue/design-migration-audit`, clone `noirdesktop-v1` | arbre de travail du workspace Conductor `yellowknife`, **non commité** jusqu'au 2026-11-21 | `02-design/reference/styles.css` + `views.js` + `index.html`, et donc `tokens.css`. **C'est la cible de parité** — [`../docs/adr/0004`](../docs/adr/0004-cible-de-parite.md). |
| **commit `b8c1019`**, branche `archive/wifi-sync-v3` | arbre de travail du worktree `nice-solomon`, **non commité** depuis avril 2026 | tout `05-postmortem/chantier-wifi-sync/` : 5 677 l. de Rust, 30 tests, le plan de design et le protocole de test annoté. |

Ce qui a changé dans les fichiers existants : `00-BRIEF-V2.md` (contrainte 2), `README.md`,
`01-produit/FEATURE-SET.md` (en-tête + §11), `02-design/ECARTS-DESIGN.md` (réécrit),
`02-design/reference/README.md` (réécrit), `04-invariants/INVARIANTS.md` (invariant 19),
`06-setup-v2/HARNAIS.md` (§2), `05-postmortem/archive-dap-sync/README.md` (renvoi).

L'ancien CSS distribué est conservé sous `02-design/reference/styles.css.beta-v0.2.0` — il n'est plus
la référence, mais il permet de mesurer ce que les testeurs vont voir changer.

## Documents rédigés pour ce bootstrap (n'existaient pas)

| Fichier | Construit à partir de |
|---|---|
| `README.md` | — |
| `00-BRIEF-V2.md` | la demande + les constats ci-dessous |
| `MANIFEST.md` | — |
| `01-produit/FEATURE-SET.md` | **lecture du code** (`lib.rs`, les 20 modules JS, `index.html`, `tauri.conf.json`, `Cargo.toml`) + spec + skills + CHANGELOG + HISTORY |
| `01-produit/UX-ANTI-REGRESSION.md` | spec section 13 (V001-V016) + HISTORY + git log (W001-W008) |
| `01-produit/NON-GOALS.md` | BETA_KNOWN_ISSUES + archive DAP + spec section 14 + recherche marché |
| `02-design/tokens.css` | **extrait de `src/styles.css`** (`:root`, `@font-face`) + variables runtime trouvées dans les JS |
| `02-design/ECARTS-DESIGN.md` | diff manuel `DESIGN.md` ↔ `styles.css` ↔ spec section 7.2 + lecture de la CSP |
| `03-architecture/ARCHITECTURE-V1.md` | `wc -l` sur tout le code + `grep` des statics + CLAUDE.md + skills |
| `03-architecture/API-IPC-V1.md` | `grep` du `generate_handler!` et des `emit`/`listen` |
| `03-architecture/PERSISTANCE.md` | `get_data_dir()` + grep des noms de fichiers + CLAUDE.md + skills |
| `03-architecture/code-reference/README.md` | jugement sur chaque fichier copié |
| `04-invariants/README.md` | synthèse des 6 skills + mémoire de l'agent `invariant-guardian` |
| `05-postmortem/DETTE-TECHNIQUE-V1.md` | l'ensemble des constats |
| `06-setup-v2/README.md` | — |
| `06-setup-v2/PLAN-DE-RECONSTRUCTION.md` | — |
| `06-setup-v2/HARNAIS.md` | les 5 causes racines du postmortem + les 19 invariants + les 24 comportements, retournés en mécanismes de vérification |
| `06-setup-v2/CLAUDE.md` | template dérivé de `noir-tauri/CLAUDE.md` |
| `06-setup-v2/ci/secrets.env.template` | noms de variables documentés dans `hean-release` et `hean-ops` (**aucune valeur réelle**) |

## Copies à l'identique

> Seule modification apportée aux copies : **les liens relatifs ont été réécrits** pour rester
> valides dans l'arborescence du bootstrap (9 liens, dans 6 fichiers, pointant vers
> `docs/archive/dap-sync/`, `BETA_KNOWN_ISSUES.md` et des fichiers source V1). Le texte est intact.
> Plus un addendum explicitement signalé en fin de `05-postmortem/BUGS-CONNUS-V1.md`.

| Ici | Source V1 |
|---|---|
| `01-produit/GUIDE-TESTEURS-V1.md` | `docs/BETA_TESTERS_GUIDE.md` |
| `01-produit/RELEASE-NOTES-V1.md` | `docs/RELEASE_NOTES_v0.2.0-beta.1.md` |
| `01-produit/recherche/market-viability.md` | `research/Noir Desktop- Market Viability…md` |
| `01-produit/recherche/pricing-gtm.md` | `research/pricing_gtm_analysis.md` |
| `01-produit/recherche/gtm-playbook-solo-dev.md` | `research/compass_artifact_wf-…md` |
| `01-produit/recherche/ancien-claude-md-fevrier.md` | `research/CLAUDE_Feature Analysis.md` — malgré son nom, c'est une **ancienne copie du CLAUDE.md** (février), pas une analyse de features. Utile pour voir l'évolution du projet. |
| `02-design/DESIGN-SYSTEM.md` | `DESIGN.md` |
| `02-design/reference/index.html`, `styles.css` | `noir-tauri/src/` |
| `02-design/reference/GeistMono-Variable.woff2` | `noir-tauri/src/fonts/` |
| `02-design/reference/app_logo.svg` | racine |
| `02-design/reference/splash-preview.html` | `noir-tauri/src/` |
| `02-design/reference/mockup-home.html` | `docs/noir-home-mockup.html` |
| `02-design/reference/prototype-onboarding.html` | `noir-tauri/docs/onboarding-prototype.html` |
| `02-design/reference/dmg-window.html` | `noir-tauri/scripts/` |
| `02-design/reference/noir-design-system.html` | ⚠️ **`~/Documents/Thomas/Noir Design System/noir-design-system.html` — hors du dépôt git.** 108 Ko, 2026-04-06, 28 sections. Référencé par une seule ligne du skill `hean-frontend`. Rapatrié ici, sans quoi il aurait été perdu. **C'est la meilleure doc design du projet.** |
| `03-architecture/SPEC-V1-COMPLETE.md` | `noir-tauri/docs/NOIR_DESKTOP_SPEC.md` (8 mars 2026, 1 324 l.) |
| `03-architecture/SPEC-V1-ancienne-fevrier.md` | `docs/NOIR_DESKTOP_SPEC.md` (723 l., sous-ensemble) |
| `03-architecture/code-reference/rust/**` | `noir-tauri/src-tauri/src/` (audio/, network/, + 7 fichiers) |
| `03-architecture/code-reference/js/**` | `noir-tauri/src/` (state, app, utils, drag, error-tracking) |
| `03-architecture/code-reference/Cargo.toml.v1`, `tauri.conf.json.v1` | `noir-tauri/src-tauri/` |
| `04-invariants/INVARIANTS.md` | `noir-tauri/.claude/skills/hean-invariants/SKILL.md` |
| `04-invariants/SAVOIR-*.md` | les 5 autres skills `hean-*` |
| `05-postmortem/HISTORIQUE-SESSIONS.md` | `docs/HISTORY.md` |
| `05-postmortem/CHANGELOG-V1.md` | `CHANGELOG.md` |
| `05-postmortem/BUGS-CONNUS-V1.md` | `docs/BETA_KNOWN_ISSUES.md` **+ un addendum ajouté en fin de fichier** |
| `05-postmortem/archive-dap-sync/` | `docs/archive/dap-sync/` (7 fichiers) **+ le code complet de la feature**, extrait du tag `dap-sync-archive-2026-04` le 2026-11-21 : 6 973 l. de Rust, 2 957 l. de JS, 2 482 l. de CSS, le diff de retrait et les extraits de câblage. Voir `archive-dap-sync/README-CODE.md`. Les 5 pistes d'amélioration de `TODOS.md` y sont reprises. |
| `06-setup-v2/agents/`, `hooks/` | `.claude/agents/`, `.claude/hooks/` |
| `06-setup-v2/ci/ci.yml` | `.github/workflows/ci.yml` |
| `06-setup-v2/ci/cloudflare-worker.js`, `generate-dmg.py`, `release.sh.v1`, `dev.sh.v1` | `noir-tauri/scripts/` |
| `06-setup-v2/ci/beta-bug-report.md` | `.github/ISSUE_TEMPLATE/` |
| `06-setup-v2/SPEC-SUITE-DE-TESTS-V1.md` | `docs/SPEC-test-suite.md` |
| `06-setup-v2/GUIDE-LAUNCH-BETA-V1.md` | `docs/BETA_LAUNCH_GUIDE.md` |

---

## Volontairement exclu

| Exclu | Pourquoi |
|---|---|
| **`noir-tauri/scripts/.env.local`** | Contient les vrais secrets (DSN Sentry, secret du worker, identifiants Apple, clé de signature). Seuls les **noms** de variables sont documentés, dans `06-setup-v2/ci/secrets.env.template`. |
| **`lib.rs` (4 870 l.)** | C'est le fichier à ne pas reproduire. Sa surface est inventoriée dans `API-IPC-V1.md`, ses états globaux dans `ARCHITECTURE-V1.md`, et le postmortem explique pourquoi. Consultable dans le dépôt V1. |
| **`views.js`, `playback.js`, `playlists.js`, `panels.js`, `renderer.js`, `onboarding.js`, `library.js`, `network.js`, `shortcuts.js`, `fullscreen-player.js`, `eq.js`, `search.js`, `lyrics.js`, `feedback.js`, `auto-update.js`** (~14 000 l.) | Leur **comportement** est capturé dans `FEATURE-SET.md` + `UX-ANTI-REGRESSION.md`, leur **rendu** dans `02-design/reference/`. Copier le code inciterait à le porter tel quel, ce qui est précisément ce qu'on veut éviter. |
| **`noir/`** (racine du dépôt) | Le prototype Electron initial (`main.js`, `renderer.js`, `styles.css`, ~4 fichiers). Abandonné bien avant Tauri, aucune valeur pour V2. |

| **`noir-tauri/src/test-regression.html`** (1 577 l.) | Harnais de test manuel V1, lié à l'implémentation V1. Le protocole utile est dans `SPEC-SUITE-DE-TESTS-V1.md`. |
| **`docs/prompt-test-suite-noir.md`** | Prompt de génération de la suite de tests V1 — artefact de process, pas de connaissance produit. |
| **Le dashboard dev** (`docs/index.html`, `noir-tauri/dashboard/index.html`) et **`scripts/sprint-planner.js`** | Outillage hors-app, réutilisable tel quel depuis le dépôt V1 ; ne conditionne pas le démarrage de V2. Documenté dans `04-invariants/SAVOIR-OPS.md`. `sprint-planner.js` référence un modèle et un chargement de codebase à revoir de toute façon. |
| **Les workflows `feedback-agent.yml`, `sprint-plan.yml`, `sprint-synthesis.yml`** | Liés au dashboard, même raison. Seul `ci.yml` est copié. |
| **`noir-tauri/src/__tests__/`** (15 fichiers Jest) | À relire dans le dépôt V1 avant d'écrire ceux de V2, mais les porter n'aurait pas de sens (3 suites skippées faute de pouvoir mocker `invoke`). Les 2 qui gardent un invariant sont nommés dans `03-architecture/code-reference/README.md`. |
| **`src-tauri/icons/`, `gen/`, `target/`, `node_modules/`** | Binaires, artefacts de build, dépendances. |
| **Les captures d'écran** | **Il n'y en a aucune dans le dépôt.** C'est le trou le plus gênant de ce bootstrap : voir `README.md`, « Deux choses à faire avant de commencer ». |

---

## Avertissements sur les sources

| Source | Fiabilité |
|---|---|
| `03-architecture/SPEC-V1-COMPLETE.md` | **Périmée mais précieuse.** Datée du 8 mars 2026, elle décrit la v0.1.0 : 82 commandes (il y en a 86), 29 variables CSS (60), 7 813 lignes de CSS (10 654), radius 2/6/8 px (4/8/12), et elle ignore les trois chantiers design de 2026 (glassmorphism, ambient glow, watermarks). En revanche, ses sections 3 (chaîne audio), 4.9 (NAS), 7.x (UI détaillée), 13 (anti-régression) et 15 (patterns) sont les plus complètes qui existent. **À lire pour les mécanismes, pas pour les chiffres.** |
| `02-design/DESIGN-SYSTEM.md` | **Fiable sur le concept, fausse sur les valeurs.** Les sections glassmorphism, ambient glow et watermarks dynamiques (avec le pipeline d'extraction de couleur) sont exactes et irremplaçables. 5 couleurs, une ombre et surtout la typographie divergent du code. → `ECARTS-DESIGN.md`. |
| `02-design/reference/noir-design-system.html` | **La meilleure source design, et elle était hors du dépôt.** Valeurs de couleurs, rayons et ombres **exactes**. Décalée de +1px sur les 7 petites tailles de police (le code les a augmentées après coup) et sur le flou du glass (44px par élément vs token global 20px). Décrit l'écran DAP Sync, archivé 16 jours après sa rédaction : à ignorer. |
| `04-invariants/SAVOIR-RELEASE.md` | **Deux pipelines dans un fichier.** La section du haut est la bonne ; les « Étapes 0 à 5 » datent d'avant le worker Cloudflare et mentionnent `NOIR_GITHUB_FEEDBACK_TOKEN`, qui n'existe plus. Le fichier le signale lui-même en encadré. Laissé intact pour ne rien perdre. |
| `04-invariants/SAVOIR-FRONTEND.md` | Deux chemins faux : les thumbnails et covers internet sont dans `~/Library/Application Support/noir/`, pas `~/.noir_desktop/`. Sa palette de couleurs est celle de `DESIGN-SYSTEM.md`, donc fausse sur 3 valeurs. En revanche, c'est lui qui signalait l'existence du design system hors dépôt — **désormais rapatrié** dans `02-design/reference/`. |
| `06-setup-v2/GUIDE-LAUNCH-BETA-V1.md` | Antérieur au worker Cloudflare, partiellement périmé. Le runbook à jour est `SAVOIR-RELEASE.md`. |
| `01-produit/recherche/*` | Analyses de mars-avril 2026 (marché, prix, GTM). Contexte produit, pas des faits techniques ; les prix et les concurrents ont pu bouger. |
| `02-design/reference/styles.css` et `views.js` | **La cible de parité — et elle n'a jamais été lancée.** Le corps de la PR #37 le dit : « l'agent ne peut pas lancer l'app Tauri […] le rendu visuel est à confirmer sur ton Mac ». La migration d'octobre n'a pas été lancée davantage. Ce sont donc des sources **fiables sur le code et invérifiées sur le rendu**. Les captures d'écran (§A2 de `DEMARRER.md`) sont la première validation. |
| `05-postmortem/chantier-wifi-sync/code/` | **Non compilé depuis avril 2026.** 30 tests existent mais n'ont pas tourné depuis. Les versions des dépendances ajoutées (axum 0.7, rustls 0.23, rcgen 0.13…) sont à revérifier. Le `cargo check` est le premier geste de toute reprise. |
| `05-postmortem/chantier-wifi-sync/TEST-PROTOCOL-V2.2.md` | **Contient les réponses de Thomas écrites à la main** dans le fichier (« OK », « j'ai une alerte… », « je ne sais pas comment faire »). C'est la seule preuve directe que la feature tournait — et la trace de deux frictions d'UX. Ne pas « nettoyer » ce fichier : les annotations sont l'information. |
