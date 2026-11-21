# Decision records

Un fichier par décision, numéroté, **immuable**. On ne modifie pas un ADR : on en écrit un nouveau
qui le remplace (`Remplace : 0003`).

Chaque ADR dit : le contexte, la décision, les alternatives écartées **et pourquoi**, les
conséquences acceptées.

**Pourquoi ce format** — en V1, les « décisions techniques actées » s'accumulaient dans une section
de skill qui grossissait. On y perdait le *pourquoi*, donc la règle finissait par être supprimée par
quelqu'un qui la trouvait arbitraire. Un ADR garde la trace du raisonnement, pas seulement de la
conclusion.

| # | Décision | État |
|---|---|---|
| [0001](0001-reecrire-hean-de-zero.md) | Réécrire Hean de zéro plutôt que refactorer V1 | Acceptée |
| [0002](0002-stack-technique.md) | Stack : TypeScript strict + Vite, Svelte 5, tauri-specta, SQLite | Acceptée |
| [0003](0003-dossier-de-donnees.md) | Données dans `hean/` avec migration depuis `noir/` | Acceptée |
| [0004](0004-cible-de-parite.md) | La cible de parité est le redesign v3, commit `e12a200` | Acceptée |

**Décisions encore ouvertes**, à transformer en ADR dès qu'elles sont prises :

| Sujet | Bloque | Où c'est documenté |
|---|---|---|
| Typographie — Geist Mono partout, ou + DM Sans embarquée en local | phase 4 | [`../../bootstrap/02-design/ECARTS-DESIGN.md`](../../bootstrap/02-design/ECARTS-DESIGN.md) §2. L'échelle de tailles, elle, est tranchée par [`0004`](0004-cible-de-parite.md). |
| Régime de V1 pendant la reconstruction (gel / hotfix seulement / évolution) | rien | [`0004`](0004-cible-de-parite.md) — la cible, elle, est tranchée |
| WiFi Sync dans le périmètre de V2, et à quelle phase | rien | [`../../bootstrap/05-postmortem/chantier-wifi-sync/README.md`](../../bootstrap/05-postmortem/chantier-wifi-sync/README.md) |
| V2 = mise à jour de V1 (même bundle identifier) ou app distincte | phase 7 | [`0003`](0003-dossier-de-donnees.md) |
| Nom de service Keychain pour les mots de passe SMB | phase 3 | [`0003`](0003-dossier-de-donnees.md) |
| `noir://` → `hean://` et `NOIR_WORKER_*` → `HEAN_WORKER_*` | phase 2 / 7 | [`0003`](0003-dossier-de-donnees.md) |
