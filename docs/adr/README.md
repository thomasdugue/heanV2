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
| [0014](0014-la-migration-conserve-le-dossier-de-v1.md) | La migration conserve le dossier de V1 moins son cache : « Oui, garde-le », « Oui, jette le cache » | Acceptée |
| [0015](0015-nom-de-service-keychain-pour-les-mots-de-passe-smb.md) | Nom de service Keychain pour les mots de passe SMB : `com.tdugue.hean.smb` et lui seul, le nom de V1 n'est plus lu | Acceptée |
| [0016](0016-la-part-reseau-de-la-bibliotheque.md) | La part réseau de la bibliothèque : SMB lu directement, et un scan par périmètre | Acceptée |
| [0017](0017-ce-que-thomas-a-tranche-le-25-novembre.md) | Ce que Thomas a tranché le 25 novembre : pas d'utilisateurs, le son d'abord, et le harnais se gèle | Acceptée |
| [0018](0018-le-bit-perfect-est-un-etat-affiche.md) | Le bit-perfect est un état affiché ; le repli de fréquence est permis s'il s'affiche | Acceptée |
| [0019](0019-le-depot-passe-en-public.md) | Le dépôt passe en public le temps de la construction de V2 ; données sensibles masquées, DSN à révoquer | Acceptée |

**Décisions encore ouvertes**, à transformer en ADR dès qu'elles sont prises :

| Sujet | Bloque | Où c'est documenté |
|---|---|---|
| **Nouvelle cible de parité** après la passe « davantage de DM Sans » que Thomas fait côté design | phase 4 | Remplacera [`0004`](0004-cible-de-parite.md). Vérifié le 2026-11-21 : DM Sans **se charge** et le redesign v3 l'applique déjà aux grands titres ; la direction est d'élargir sa part. Méthode de réextraction : [`../../bootstrap/02-design/ECARTS-DESIGN.md`](../../bootstrap/02-design/ECARTS-DESIGN.md) §2. |
| Identité d'une piste : par chemin de fichier, ou morceau logique avec N fichiers ? | phase 3 | 21 % de la bibliothèque est en double — [`../MESURES-V1.md`](../MESURES-V1.md) §10 |
| **Cible de parité : les 124 features, ou une cible réduite à ~85-90 ?** Critère proposé par l'audit : « un utilisateur passe à V2 sans perdre ce qu'il utilise ». Deezer est dedans et WiFi Sync dehors, le reste attend | rien : **Thomas a explicitement reporté la question jusqu'à ce que le son sorte des enceintes** | [`0017`](0017-ce-que-thomas-a-tranche-le-25-novembre.md) §2 |
| **Une seule liste des zones Rust vivantes, au lieu de cinq ?** Chaque garde-fou porte la sienne, un dossier qui naît entre dans trois listes sur cinq, et l'oubli est invisible. Vu trois fois. Gravité suggérée 🟡 : le dommage est indirect — un détecteur aveugle — mais il désarme des règles 🔴 | rien ; gelé jusqu'au jalon son ([`0017`](0017-ce-que-thomas-a-tranche-le-25-novembre.md) §5) | mémoire de l'agent de relecture, `les-listes-de-zones-du-harnais` |
| **La flèche de l'écart de fréquence pointe toujours vers le bas**, alors que le repli choisit en priorité une fréquence *supérieure* : un 44,1 kHz joué à 48 s'affiche « 48 kHz ↓ ». Les deux chiffres restent visibles, donc l'information n'est pas fausse | rien : cosmétique, non mesuré comme gênant | [`0018`](0018-le-bit-perfect-est-un-etat-affiche.md) |
| Régime de V1 pendant la reconstruction (gel / hotfix seulement / évolution) | rien | [`0004`](0004-cible-de-parite.md) — la cible, elle, est tranchée |
| V2 = mise à jour de V1 (même bundle identifier) ou app distincte | phase 7 | [`0003`](0003-dossier-de-donnees.md) |
| **Que faire quand le NAS est éteint ?** Aujourd'hui les pistes réseau n'apparaissent qu'après un scan, donc une bibliothèque à 91 % réseau est quasi vide sans le NAS. Reprendre les 3 137 pistes du cache de V1 au démarrage la rendrait visible hors ligne, au prix de 42 trous et de 181 écarts de métadonnées connus | rien aujourd'hui ; deviendra urgent si Thomas travaille souvent NAS éteint | [`0016`](0016-la-part-reseau-de-la-bibliotheque.md) § alternatives écartées |
| **Un scan réseau de vingt minutes dans une seule transaction.** Tout l'inventaire est appliqué à la fin, ce qui garantit qu'une bibliothèque n'est jamais à moitié écrite — et fait perdre vingt minutes si le réseau coupe à la 3 000ᵉ piste. Deux pistes : appliquer par lots (un dossier, un commit), ou garder la transaction unique et ajouter une reprise. Le choix change ce que « jamais à moitié écrite » veut dire | rien aujourd'hui ; chaque premier scan d'un gros NAS | [`0016`](0016-la-part-reseau-de-la-bibliotheque.md) §5, et la mesure du 2026-11-25 dans [`../HISTORY.md`](../HISTORY.md) |
| **V2 n'écrit aucun journal sur disque.** `src-tauri/src/observability.rs` n'installe qu'un `fmt::layer()` vers stderr : en `npm run tauri dev` tout va dans le terminal, et en bundle signé sans terminal il n'y aura **rien**. Et ce n'est pas un appender mal réglé — `tracing-appender` n'est pas déclaré dans `src-tauri/Cargo.toml` (seuls `tracing` et `tracing-subscriber` le sont), donc il n'existe aujourd'hui aucun moyen d'écrire un fichier. Le commentaire en tête de ce module reproche justement à V1 d'avoir eu des bugs « annoncés en clair en dev et invisibles en production » — V2 a aujourd'hui la même propriété, en pire | rien aujourd'hui ; bloquera tout diagnostic dès qu'une build signée tourne (phase 6), et a **déjà** mordu : la mesure du scan de 36 min, les quatre saisies de mot de passe et l'écriture du trousseau de 01:07:35Z n'existent que dans un défilement de terminal qui disparaîtra | relevé le 2026-11-26 en cherchant à expliquer cette écriture, cf. [`0015`](0015-nom-de-service-keychain-pour-les-mots-de-passe-smb.md) § relevé du trousseau. Zone `hean-ops`. **Pas mis dans « Bugs connus » de `CLAUDE.md` : le fichier est à 149 lignes sur 150**, donc l'échappatoire que l'ADR 0013 prescrit est pleine |
| **Rescan complet d'une source réseau.** `libsmbclient` ne rend taille et date qu'avec `readdirplus` ; là où le serveur ne le supporte pas, une piste déjà connue n'est jamais relue, donc un fichier modifié depuis une autre machine reste invisible | rien aujourd'hui | [`0016`](0016-la-part-reseau-de-la-bibliotheque.md) §4 |
| **La règle des 400 lignes vise-t-elle les fichiers écrits par un outil ?** Thomas a tranché l'étape (2026-11-23, option A : une exception nommée pour les fichiers de verrouillage, dans `tools/check-sizes.sh`). La question de fond reste ouverte. | quand `src/ipc/bindings.ts` approchera 400 lignes | **Déclencheur chiffré** : `bindings.ts` est généré et mesuré, à **152 lignes pour 8 commandes IPC** — environ 19 lignes par commande, et V1 en avait **86**. Il franchira donc 400 vers la vingtaine de commandes : la question se reposera d'elle-même, sans que personne la rouvre. **Deux formes envisagées, et leurs défauts, pour ne pas les redécouvrir.** (B) « la règle ne vise que l'écrit à la main » : cohérent, mais « généré » n'est pas vérifiable sans une liste à entretenir — la forme même qui a créé les trous du 2026-11-23. (D) « l'exception reconnaît les fichiers qui s'annoncent générés » : auto-entretenue, puisque les cinq fichiers générés du dépôt le déclarent déjà en tête — mais ⚠️ **deux faiblesses mesurées** : les formulations divergent (trois phrasés, deux langues, et le marqueur n'est pas toujours en première ligne), donc il faudrait d'abord un **marqueur canonique** émis par `npm run gen` ; et keyer sur le **contenu** ouvre une porte, puisque coller le marqueur en tête d'un fichier écrit à la main suffirait à échapper à la règle — ce qui se voit dans un diff, mais existe. |
| **INV-29 — aucun justificatif en clair dans un fichier suivi par git ?** Un détecteur sur `git ls-files` cherchant les motifs de justificatifs (mot de passe app-specific Apple, DSN Sentry, Team ID, en-tête de clé privée PEM), avec une liste d'**exclusions** écrite plutôt qu'une énumération d'extensions. Gravité suggérée 🔴 : compromission de compte | rien aujourd'hui ; redevient urgent à chaque nouveau secret écrit dans le dépôt | **né d'un défaut mesuré le 2026-11-26** : le masquage a retiré le DSN et le Team ID du même fichier et laissé, *entre les deux lignes éditées*, un mot de passe app-specific Apple en clair — le seul des trois qui authentifie. Le motif de recherche butait sur le guillemet ouvrant. Une relecture à l'œil a vu deux justificatifs sur trois ; le troisième n'avait pas de mécanisme. INV-17 couvre les secrets *à la compilation*, rien ne couvre les secrets *dans l'arbre*. ⚠️ Le harnais est gelé ([`0017`](0017-ce-que-thomas-a-tranche-le-25-novembre.md)) : c'est à Thomas de dire si ce défaut franchit le seuil |
| **Les 8 captures de `bootstrap/02-design/reference/captures/` montrent la bibliothèque réelle** — pochettes sous droits, noms d'artistes et noms de playlists personnels. Elles sont la cible de parité visuelle ([`0004`](0004-cible-de-parite.md)), donc les flouter abîme ce à quoi elles servent et les retirer prive la phase 4 de sa référence | **la bascule en public** ([`0019`](0019-le-depot-passe-en-public.md)) | relevé le 2026-11-26 en masquant le reste. Trois formes : les garder telles quelles, flouter les seules pochettes (la mise en page, la typo et les couleurs survivent ; les titres non), ou sortir le dossier du dépôt et le garder en local |
| `noir://` → `hean://` et `NOIR_WORKER_*` → `HEAN_WORKER_*` | phase 2 / 7 | [`0003`](0003-dossier-de-donnees.md) — **pas tranché en phase 2** : la bibliothèque n'a besoin d'aucun protocole custom, le sujet revient avec les pochettes (phase 4) et les secrets (phase 7) |
