# Decision records

Un fichier par décision, numéroté, **immuable**. On ne modifie pas un ADR : on en écrit un nouveau
qui le remplace (`Remplace : 0003`).

Chaque ADR dit : le contexte, la décision, les alternatives écartées **et pourquoi**, les
conséquences acceptées.

**Pourquoi ce format** — en V1, les « décisions techniques actées » s'accumulaient dans une section
de skill qui grossissait. On y perdait le *pourquoi*, donc la règle finissait par être supprimée par
quelqu'un qui la trouvait arbitraire. Un ADR garde la trace du raisonnement, pas seulement de la
conclusion.

**Numérotation.** Le numéro suivant se prend en regardant ce tableau, et une collision est
possible dès que deux branches vivent en même temps — c'est arrivé le 2026-11-22, deux sessions
ayant pris « le suivant après 0006 » sans se voir. `tools/check-adr.mjs` le refuse désormais :
numéro unique, présence dans ce tableau, état dans un ensemble fermé, et tout renvoi en texte nu
qui doit désigner exactement un fichier — ce qu'aucun vérificateur de liens ne voit.

| # | Décision | État |
|---|---|---|
| [0001](0001-reecrire-hean-de-zero.md) | Réécrire Hean de zéro plutôt que refactorer V1 | Acceptée |
| [0002](0002-stack-technique.md) | Stack : TypeScript strict + Vite, Svelte 5, tauri-specta, SQLite | Acceptée |
| [0003](0003-dossier-de-donnees.md) | Données dans `hean/` avec migration depuis `noir/` | Acceptée |
| [0004](0004-cible-de-parite.md) | La cible de parité est le redesign v3, commit `e12a200` | Acceptée |
| [0005](0005-entretien-du-harnais-et-orchestration.md) | Le harnais s'entretient par mécanisme ; la délégation suit le harnais | Acceptée |
| [0006](0006-socle-et-harnais-de-phase-0.md) | Le socle de phase 0 et la forme du harnais | Acceptée |
| [0007](0007-format-interne-audio-et-preuve-du-bit-perfect.md) | Format interne audio `i32` à l'échelle 32 bits ; le bit-perfect devient une assertion | Acceptée |
| [0008](0008-epingler-la-chaine-doutils.md) | La chaîne d'outils Rust est épinglée — `./check` est vraiment identique partout | Acceptée |
| [0009](0009-bibliotheque-cle-dalbum-et-source-de-verite.md) | La bibliothèque : schéma SQLite strict et versionné, clé d'album typée, le fichier audio comme source de vérité | Acceptée |

**Décisions encore ouvertes**, à transformer en ADR dès qu'elles sont prises :

| Sujet | Bloque | Où c'est documenté |
|---|---|---|
| **Nouvelle cible de parité** après la passe « davantage de DM Sans » que Thomas fait côté design | phase 4 | Remplacera [`0004`](0004-cible-de-parite.md). Vérifié le 2026-11-21 : DM Sans **se charge** et le redesign v3 l'applique déjà aux grands titres ; la direction est d'élargir sa part. Méthode de réextraction : [`../../bootstrap/02-design/ECARTS-DESIGN.md`](../../bootstrap/02-design/ECARTS-DESIGN.md) §2. |
| Identité d'une piste : par chemin de fichier, ou morceau logique avec N fichiers ? | phase 3 | 21 % de la bibliothèque est en double — [`../MESURES-V1.md`](../MESURES-V1.md) §10 |
| Régime de V1 pendant la reconstruction (gel / hotfix seulement / évolution) | rien | [`0004`](0004-cible-de-parite.md) — la cible, elle, est tranchée |
| WiFi Sync dans le périmètre de V2, et à quelle phase | rien | [`../../bootstrap/05-postmortem/chantier-wifi-sync/README.md`](../../bootstrap/05-postmortem/chantier-wifi-sync/README.md) |
| V2 = mise à jour de V1 (même bundle identifier) ou app distincte | phase 7 | [`0003`](0003-dossier-de-donnees.md) |
| Nom de service Keychain pour les mots de passe SMB | phase 3 | [`0003`](0003-dossier-de-donnees.md) |
| `noir://` → `hean://` et `NOIR_WORKER_*` → `HEAN_WORKER_*` | phase 2 / 7 | [`0003`](0003-dossier-de-donnees.md) — **pas tranché en phase 2** : la bibliothèque n'a besoin d'aucun protocole custom, le sujet revient avec les pochettes (phase 4) et les secrets (phase 7) |
