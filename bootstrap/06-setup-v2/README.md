# Setup heanV2 — ce qui s'installe tel quel

Ce dossier contient l'outillage réutilisable de V1 et le plan de démarrage.

## À installer dans heanV2

| Source ici | Destination | Adaptation nécessaire |
|---|---|---|
| [`CLAUDE.md`](CLAUDE.md) | racine du dépôt | compléter les blocs `⟦À TRANCHER⟧` / `⟦À REMPLIR⟧` |
| [`../04-invariants/*.md`](../04-invariants/) | `.claude/skills/hean-*/SKILL.md` | ce sont déjà des `SKILL.md` avec frontmatter. **Re-cheminer les références de fichiers** (elles pointent vers l'arborescence V1) et trier les invariants en trois piles (rayés / devenus tests / irréductibles) |
| [`agents/test-runner.md`](agents/test-runner.md) | `.claude/agents/` | adapter les commandes de test à la stack V2 |
| [`agents/invariant-guardian.md`](agents/invariant-guardian.md) | `.claude/agents/` | adapter le chemin du skill d'invariants et la liste des chemins « zone audio » |
| [`hooks/invariant-reminder.sh`](hooks/invariant-reminder.sh) | `.claude/hooks/` | adapter les deux `grep -E` de chemins |
| [`ci/ci.yml`](ci/ci.yml) | `.github/workflows/` | ajouter `tsc --noEmit` + lint, garder `cargo test --locked` sur macOS, protéger `main` |
| [`ci/cloudflare-worker.js`](ci/cloudflare-worker.js) | `scripts/` | **réutilisable tel quel** (routes `/feedback` + proxy `/sentry/*`) |
| [`ci/generate-dmg.py`](ci/generate-dmg.py) + `../02-design/reference/dmg-window.html` | `scripts/` | réutilisables tels quels |
| [`ci/secrets.env.template`](ci/secrets.env.template) | `scripts/.env.local` (gitignored, chmod 600) | remplir avec les valeurs réelles |
| [`ci/beta-bug-report.md`](ci/beta-bug-report.md) | `.github/ISSUE_TEMPLATE/` | tel quel |

## À lire, pas à installer

| Fichier | Pourquoi |
|---|---|
| [`PLAN-DE-RECONSTRUCTION.md`](PLAN-DE-RECONSTRUCTION.md) | **Commencer par là.** Les 6 arbitrages de la phase 0, puis les 8 phases avec leurs critères de sortie. |
| [`HARNAIS.md`](HARNAIS.md) | **À lire juste après.** Les 12 ajouts au harnais (fixtures + backend audio fake, tests d'architecture, machine à états, property tests, mode parité V1↔V2, budgets de perf, doc générée…), ce qu'il ne faut pas ajouter, et la table « chaque faiblesse V1 → ce qui la rend impossible ». Détermine une partie du contenu de la phase 0. |
| [`SPEC-SUITE-DE-TESTS-V1.md`](SPEC-SUITE-DE-TESTS-V1.md) | Le protocole de tests manuels de V1 : à rejouer en phase 8 pour la recette de parité. |
| [`GUIDE-LAUNCH-BETA-V1.md`](GUIDE-LAUNCH-BETA-V1.md) | Le guide narratif complet du lancement beta. Partiellement périmé (antérieur au worker Cloudflare) ; le runbook à jour est [`../04-invariants/SAVOIR-RELEASE.md`](../04-invariants/SAVOIR-RELEASE.md), section du haut. |
| [`ci/release.sh.v1`](ci/release.sh.v1) / [`ci/dev.sh.v1`](ci/dev.sh.v1) | Les wrappers V1. À relire pour comprendre ce qu'ils validaient — mais en V2 la validation des secrets doit se faire dans `build.rs` avec `compile_error!()`, pas dans un script bash qu'on peut oublier de lancer. |

## Non copié ici

- **Le dashboard dev** (`docs/index.html` → GitHub Pages, et `noir-tauri/dashboard/index.html`) et
  **`scripts/sprint-planner.js`** : outillage hors-app (issues GitHub + crashs Sentry fusionnés,
  génération de plans). Réutilisables tels quels depuis le dépôt V1, mais ils ne conditionnent pas
  le démarrage de V2 — et `sprint-planner.js` référence un modèle (`claude-opus-4-7`) et un
  chargement de 583 Ko de codebase à revoir. Documentés dans
  [`../04-invariants/SAVOIR-OPS.md`](../04-invariants/SAVOIR-OPS.md).
- **Les workflows `feedback-agent.yml` / `sprint-plan.yml` / `sprint-synthesis.yml`** : liés au
  dashboard, même raison.
- **`scripts/.env.local`** : contient les vrais secrets. **Volontairement exclu.** Seuls les noms de
  variables sont documentés dans [`ci/secrets.env.template`](ci/secrets.env.template).
- **Les tests V1** (`src/__tests__/`, 15 fichiers Jest) : à relire dans le dépôt V1 avant d'écrire
  ceux de V2. `AlbumGrouping.test.js` et `ShuffleButton.test.js` sont les deux seuls qui gardent
  réellement un invariant ; 3 suites sont skippées parce que `invoke` n'est pas mockable — un
  problème à régler dès la phase 0 de V2.
