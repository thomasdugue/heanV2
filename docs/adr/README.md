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

⚠️ **Mais `check-adr.mjs` ne voit que la branche courante** : il lit les fichiers suivis par git,
donc il ne peut pas détecter une collision avec une branche vivante tant qu'elle n'a pas fusionné.
**Le numéro se prend donc en regardant ce tableau ET les PR ouvertes.** C'est arrivé une seconde
fois le 2026-11-23, et cette fois sans dégât : 0010 était déjà pris par la PR de migration des
données de V1, encore ouverte et donc invisible pour `check-adr.mjs`. Les deux branches auraient
passé `./check` chacune de son côté, et la collision n'aurait éclaté qu'au merge. La décision
suivante a donc pris **0011**, et les deux numéros sont arrivés intacts — c'est l'état qu'on lit
aujourd'hui dans le tableau. Un trou temporaire dans la numérotation ne coûte rien ; deux
décisions sous le même numéro ont coûté une heure et huit renvois à corriger.

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
| [0010](0010-methode-de-migration-des-donnees-v1.md) | La méthode de migration des données de V1 : copier à côté, vérifier, échanger, puis supprimer | Acceptée |
| [0011](0011-derogation-inv-21-pour-les-exemples.md) | Les exemples en ligne de commande écrivent sur stdout : dérogation à INV-21 | Acceptée |
| [0012](0012-un-echec-de-migration-doit-rester-rattrapable.md) | Un échec de migration doit rester rattrapable : rien n'est créé dans le dossier de données avant son verdict | Acceptée |
| [0013](0013-une-session-un-perimetre.md) | Une session, un périmètre ; et aucune session n'en démarre une autre | Acceptée |
| [0014](0014-la-migration-conserve-le-dossier-de-v1.md) | La migration conserve le dossier de V1 : « Oui, garde-le » | Acceptée |

**Décisions encore ouvertes**, à transformer en ADR dès qu'elles sont prises :

| Sujet | Bloque | Où c'est documenté |
|---|---|---|
| **Nouvelle cible de parité** après la passe « davantage de DM Sans » que Thomas fait côté design | phase 4 | Remplacera [`0004`](0004-cible-de-parite.md). Vérifié le 2026-11-21 : DM Sans **se charge** et le redesign v3 l'applique déjà aux grands titres ; la direction est d'élargir sa part. Méthode de réextraction : [`../../bootstrap/02-design/ECARTS-DESIGN.md`](../../bootstrap/02-design/ECARTS-DESIGN.md) §2. |
| Identité d'une piste : par chemin de fichier, ou morceau logique avec N fichiers ? | phase 3 | 21 % de la bibliothèque est en double — [`../MESURES-V1.md`](../MESURES-V1.md) §10 |
| **Le cache SMB de `noir/` vaut-il d'être supprimé, lui ?** Thomas a tranché le 2026-11-25 que le dossier de V1 est conservé ([`0014`](0014-la-migration-conserve-le-dossier-de-v1.md), « Oui, garde-le »). Son coût réel est **289 Mio, et c'est entièrement son `smb_buffer/`** — trois fichiers FLAC téléchargés depuis le NAS, un **cache reconstructible** que V1 était censé purger à chaque lancement (W008, correctif `eaee676`) et qu'il n'a pas purgé. Le supprimer ne perdrait aucune donnée et ramènerait le retour arrière à un coût presque nul. Mais « garde-le » répondait à « garder ton dossier V1 », pas à « garder tout sauf le cache » : c'est une autre question. | rien | [`0014`](0014-la-migration-conserve-le-dossier-de-v1.md), section « question ouverte » ; mesure du 2026-11-25 sur le dossier réel |
| Régime de V1 pendant la reconstruction (gel / hotfix seulement / évolution) | rien | [`0004`](0004-cible-de-parite.md) — la cible, elle, est tranchée |
| WiFi Sync dans le périmètre de V2, et à quelle phase | rien | [`../../bootstrap/05-postmortem/chantier-wifi-sync/README.md`](../../bootstrap/05-postmortem/chantier-wifi-sync/README.md) |
| V2 = mise à jour de V1 (même bundle identifier) ou app distincte | phase 7 | [`0003`](0003-dossier-de-donnees.md) |
| Nom de service Keychain pour les mots de passe SMB | phase 3 | [`0003`](0003-dossier-de-donnees.md) |
| **La règle des 400 lignes vise-t-elle les fichiers écrits par un outil ?** Thomas a tranché l'étape (2026-11-23, option A : une exception nommée pour les fichiers de verrouillage, dans `tools/check-sizes.sh`). La question de fond reste ouverte. | quand `src/ipc/bindings.ts` approchera 400 lignes | **Déclencheur chiffré** : `bindings.ts` est généré et mesuré, à **152 lignes pour 8 commandes IPC** — environ 19 lignes par commande, et V1 en avait **86**. Il franchira donc 400 vers la vingtaine de commandes : la question se reposera d'elle-même, sans que personne la rouvre. **Deux formes envisagées, et leurs défauts, pour ne pas les redécouvrir.** (B) « la règle ne vise que l'écrit à la main » : cohérent, mais « généré » n'est pas vérifiable sans une liste à entretenir — la forme même qui a créé les trous du 2026-11-23. (D) « l'exception reconnaît les fichiers qui s'annoncent générés » : auto-entretenue, puisque les cinq fichiers générés du dépôt le déclarent déjà en tête — mais ⚠️ **deux faiblesses mesurées** : les formulations divergent (trois phrasés, deux langues, et le marqueur n'est pas toujours en première ligne), donc il faudrait d'abord un **marqueur canonique** émis par `npm run gen` ; et keyer sur le **contenu** ouvre une porte, puisque coller le marqueur en tête d'un fichier écrit à la main suffirait à échapper à la règle — ce qui se voit dans un diff, mais existe. |
| `noir://` → `hean://` et `NOIR_WORKER_*` → `HEAN_WORKER_*` | phase 2 / 7 | [`0003`](0003-dossier-de-donnees.md) — **pas tranché en phase 2** : la bibliothèque n'a besoin d'aucun protocole custom, le sujet revient avec les pochettes (phase 4) et les secrets (phase 7) |
