# Historique des sessions

Une entrée par session, **en bas**. Ce que la session a fait, ce qu'elle a cassé, ce qu'elle a appris.

Leçon V1 : cet historique vivait dans `CLAUDE.md` jusqu'à ce que le fichier devienne trop gros pour
être lu en entier. Il a sa place ici, pas là.

- **2026-11-21** : création du dépôt. Brief, savoir repris de V1 et harnais de départ importés depuis
  `thomasdugue/noirdesktop` (dossier `heanv2-bootstrap`, 114 fichiers). Installé : `CLAUDE.md`,
  les 6 skills `hean-*`, les agents `test-runner` et `invariant-guardian`, le hook
  `invariant-reminder.sh`, `./check` (liens markdown, taille de `CLAUDE.md`, taille des fichiers,
  skills) et la CI qui l'exécute. ADR 0001 (réécrire de zéro). Aucun code applicatif. `./check` vert.
  **Prochaine étape : les 6 arbitrages de la phase 0.**
- **2026-11-21** (suite) : les 6 arbitrages de la phase 0 pris. **TypeScript strict + Vite**,
  **Svelte 5 (runes)**, **tauri-specta**, **SQLite (rusqlite)**, données dans
  `~/Library/Application Support/hean/` avec migration one-shot depuis `noir/`. La typographie est
  reportée jusqu'à vérification sur V1 (rien en phase 0 n'en dépend). ADR 0002 et 0003 écrits,
  `CLAUDE.md` complété (118/150 lignes), index des ADR enrichi des 5 décisions encore ouvertes.
  Ajout de `docs/DEMARRER.md` : 5 préalables avant de coder — dont **figer la cible de parité**, qui
  manquait au plan — la procédure de changement de modèle et le prompt de démarrage de la phase 0.
  Toujours aucun code applicatif. `./check` vert.
