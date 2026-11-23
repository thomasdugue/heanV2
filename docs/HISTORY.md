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
- **2026-11-21** (suite 2) : question posée sur l'impact de la stack sur la promesse de légèreté.
  Chiffres relevés sur V1 : frontend complet **960 Ko**, binaire Rust **16 Mo**, DMG **33 Mo** — le
  frontend pèse donc ~1 % de l'app distribuée, et l'écart binaire/archive est fait des ~92 dylibs
  Samba. Conclusion dans `docs/PERFORMANCE.md` : +~1 Mo (SQLite) moins ce que la minification fait
  gagner, mémoire neutre à meilleure, audio strictement inchangé. Trois pièges identifiés
  (réactivité sur 10 000 pistes, perte du pool de virtual scroll, **dérive npm**) ; les deux premiers
  deviennent des règles dans `CLAUDE.md`, le troisième un garde-fou : `tools/check-weight.sh`
  (budget `dist/` ≤ 600 Ko, ≤ 3 dépendances de production) branché dans `./check`.
- **2026-11-21** (suite 3) : V1 mesurée. Bibliothèque de référence : **3 443 pistes, 336 albums,
  91 % sur NAS, 97 % FLAC**, dossier de données de 889 Mo. Démarrage « prêt » **2,8 s** (processus
  visible à 90 ms). Mémoire au repos sur l'accueil : **235 Mo de footprint** (WebView 155, natif 53,
  GPU 20, réseau 7) pour **433 Mo de RSS cumulé**. Méthode outillée dans `tools/measure-app.sh` +
  `tools/webkit-pids.py` pour que V2 soit mesurée à l'identique. Budgets remplis dans
  `docs/PERFORMANCE.md`, rapport dans `docs/MESURES-V1.md`. Trois chiffres manquent (FPS, mémoire
  après défilement, recherche) : ils demandent une instrumentation dans l'app → à poser en phase 0.
  **Trois bugs V1 trouvés en mesurant** : l'auto-updater renvoie 404 depuis le lancement (prerelease
  + `/releases/latest`), le nettoyage de `smb_buffer` laisse 607 Mo, et le cache de thumbnails ne
  couvre que 11 % des covers.
- **2026-11-21** (suite 4) : tentative de mesure de la mémoire sous défilement — **invalide** :
  courbe plate sur 90 s (2 Mo de variation), aucune navigation dans la fenêtre. L'échantillonnage
  avait été démarré en même temps que l'envoi des consignes, donc avant qu'elles soient lisibles.
  Protocole corrigé : c'est l'opérateur qui lance le script. Observation secondaire conservée
  (`MESURES-V1.md` §6bis) : entre deux relevés du même processus à ~70 min d'intervalle et au repos,
  **+114 Mo de RSS / +20 Mo de footprint**, concentrés sur `WebKit.WebContent` (+69 Mo) et
  `WebKit.GPU` (+32 Mo), sans trace de scan dans le journal. Cause indéterminée — scan non
  journalisé, caches WebKit, ou fuite.
- **2026-11-21** (suite 5) : **la cible de parité était la mauvaise.** Thomas signale un écart entre
  l'app et ses dernières refontes de design. Vérification : **trois chantiers vivaient hors de
  `main`**, dont deux commités nulle part.
  1. **PR [#37](https://github.com/thomasdugue/noirdesktop/pull/37)** — redesign v3, 23 commits,
     +2 416 l., ouverte depuis le 28 mai, jamais mergée. Le checkout principal de Thomas était posé
     dessus : l'app qu'il lançait n'était pas celle qu'il avait distribuée.
  2. **WiFi Sync v3** — 5 677 l. de Rust (serveur HTTPS axum, WebUI servie au navigateur du DAP,
     ZIP streamés, certificat auto-signé, mDNS, token persistant), 30 tests, **non commité depuis
     avril 2026** et absent de toute branche, de tout stash et du clone de sauvegarde. Mis à l'abri
     sur `archive/wifi-sync-v3` (`b8c1019`), poussé.
  3. **Migration d'octobre** — le redesign v3 porté sur `main` d'octobre, **non commité**, dans deux
     workspaces Conductor. Elle corrige une régression que la PR #37 porte toujours :
     `--glass-lo/md/hi` utilisées **45 fois sans être définies**, donc ignorées en silence par le
     navigateur → bordures, survols et état actif de la sidebar invisibles. Commitée (`e12a200`),
     **pas encore poussée**.

  Conséquences : [`adr/0004`](adr/0004-cible-de-parite.md) fixe la parité sur `e12a200` ;
  `bootstrap/02-design/` réextrait depuis ce commit (+ `views.js`, parce que le redesign réécrit le
  markup et pas seulement le CSS) ; `tokens.css` passe à 64 variables sur **deux** blocs `:root` ;
  `ECARTS-DESIGN.md` réécrit autour des 3 designs et 6 sources ; **invariant 19** ajouté (« toute
  variable `var(--x)` utilisée doit être définie ») et transformé en test d'architecture dans
  `HARNAIS.md` §2 ; `chantier-wifi-sync/` créé dans le bootstrap ; préalables A1, A2 et A4 de
  `DEMARRER.md` réécrits — **A2 (les captures) devient bloquant**, puisque le design de référence
  n'a jamais été lancé par personne.

  **Et le prototype d'origine est perdu** : le redesign v3 est le portage d'un « prototype Hean v4 »
  livré dans un bundle `design_handoff`, qui vivait dans `/tmp/proto/player_test_1/`. Introuvable sur
  le disque. On reproduit le portage, pas le prototype.
- **2026-11-21** (suite 6) : protocole de capture écrit (`docs/PROTOCOLE-CAPTURES.md`) et dossier
  `captures/` créé. Trois constats pratiques en préparant le lancement : le workspace Conductor
  `yellowknife` **ne peut pas lancer l'app** (pas de `scripts/.env.local`, et `dev.sh` s'arrête net
  sans lui ; pas de cache de build Rust non plus → 10-20 min à froid), alors que le **checkout
  principal** a les deux (`.env.local` + 17 Go de `target/`) et peut récupérer la branche maintenant
  qu'elle est poussée — basculement vérifié propre, HEAD local contenu dans la branche distante. Et
  **la mesure sous défilement ne peut pas se greffer sur cette session** : elle doit tourner sur la
  build **release** (`/Applications/Hean.app`), puisque les chiffres de `MESURES-V1.md` viennent de
  là ; un build de dev n'a pas le même profil mémoire.
- **2026-11-21** (suite 7) : **l'observabilité de V1 est un décor** — trouvé par accident, en lisant
  trente secondes de sortie du build de dev lancé sur la cible de parité. **252 `println!` contre 3
  appels `tracing::`** dans le backend, et `logging.rs` ne capture pas stdout : l'app installée a
  produit **63 lignes de journal** en tout. Conséquence : les trois bugs trouvés en mesurant V1 sont
  tous **silencieux en production** alors que le build de dev les annonce en clair. Ajouté comme
  **6e cause racine** de `DETTE-TECHNIQUE-V1.md`, et comme règle de harnais (`println!` interdit dans
  le backend, test d'architecture, `HARNAIS.md` §2).

  Deux questions ouvertes résolues au passage :
  - **§5, les thumbnails à 11 %** : cause exacte trouvée. `get_cover_bytes_internal()` fait
    `if path.starts_with("smb://") { return None }`, et `generate_thumbnails_batch()` est appelé avec
    des chemins de pistes — dont **91 % sont en SMB**. D'où `0 generated, N failed` en 0 ms sur tous
    les lots. Les pochettes sont pourtant sur le disque (2 173 fichiers, 275 Mo), mais `COVER_CACHE`
    est indexé par chemin de piste alors qu'une pochette internet est nommée par `artiste|||album`.
    Correction pour V2 : la source d'un thumbnail est le fichier de pochette sur disque, jamais une
    ré-extraction depuis l'audio.
  - **§6bis, la dérive mémoire** : un scan réseau de fond tourne bien (`[Network Scan] lofty FLAC
    fallback…`). Il n'était pas journalisé parce que c'est un `println!`. Ce n'était pas « rien
    n'indique un scan », c'était « rien n'est journalisé ».
- **2026-11-21** (suite 8) : **la cible de parité a été lancée pour la première fois.** 8 captures
  dans `bootstrap/02-design/reference/captures/`, avec leur index.

  **Deux validations** : les `--glass-*` sont bien rétablies (bordures, survols et sidebar active
  visibles — la régression de la PR #37 n'est pas dans la cible), et **DM Sans se charge**.
  L'hypothèse « bloquée par la CSP » que j'avais posée le matin était **fausse** : le redesign v3
  applique déjà Geist Mono au technique et DM Sans aux grands titres, exactement l'intention du
  design system. Décision de Thomas : **davantage de DM Sans**, modification qu'il fait côté design
  — donc la cible de parité changera et `adr/0004` sera remplacé.

  **Trois défauts trouvés** :
  - **§9, l'ambient casse à chaque page album.** `getImageData()` lève une `SecurityError` parce que
    la pochette vient du protocole `noir://`, origine différente → canvas teinté. Le code retombe sur
    un **fallback base64**, c'est-à-dire exactement ce que `noir://` avait été introduit pour
    supprimer (700 Mo → 60 Ko de tas). Piste sérieuse pour les 155 Mo de la WebView. Correction
    proposée : extraire les couleurs côté Rust au scan.
  - **§10, 21 % de la bibliothèque est en double** : 342 groupes, 731 pistes, dont 83 groupes de
    vrais doublons sur le NAS (même morceau, deux nommages). Zéro doublon de chemin exact : c'est un
    choix de modèle, pas un bug. À trancher en phase 3.
  - La première vignette de la grille d'albums déborde de sa carte.

  Ces trois-là étaient visibles **en trente secondes d'usage**. Elles ne l'étaient pas en production,
  pour la raison du §8.
- **2026-11-21** (suite 9) : série de captures **mise en pause**, décision de Thomas. La typographie
  va changer sur tous les écrans avec sa passe « davantage de DM Sans » ; les capturer maintenant
  reviendrait à les refaire. Les huit existantes remplissent le rôle qui comptait aujourd'hui —
  valider la cible. Le préalable A2 passe donc de **bloquant** à **en pause** : plus rien n'attend,
  et la phase 0 (socle, `./check`, tests d'architecture, tri des invariants, 24 comportements en
  tests) ne touche pas au design — elle peut démarrer pendant la passe.
- **2026-11-22** : deux recommandations demandées par Thomas, écrites dans `06-setup-v2/`.
  - **`ENTRETIEN-DU-HARNAIS.md`** — comment le harnais reste à jour sans discipline. Trois niveaux
    de force : **généré** (un script le produit depuis le code, la CI échoue sur la dérive →
    impossible à périmer), **apparié** (deux sources, un test les compare), **réclamé** (un garde-fou
    bloque et pose une question fermée). Le hook `invariant-reminder.sh` de V1 est le modèle du
    niveau 3, à généraliser en « contrat de fin de tâche ». Plus un audit hebdomadaire pour ce
    qu'aucun diff ne montre — l'endpoint de mise à jour en 404 depuis le lancement en est l'exemple.
    Et pour « ce que j'oublie », la seule règle qui couvre l'inconnu : **tout défaut qui atteint
    Thomas est un trou dans le harnais**, et son correctif inclut la vérification qui l'aurait vu.
    C'est ainsi que les 19 invariants de V1 sont nés — l'erreur était de les écrire en prose plutôt
    qu'en tests.
  - **`ORCHESTRATION.md`** — quel modèle fait quoi. Thèse : **on ne délègue en sécurité que là où le
    harnais est fort**, donc les deux questions de Thomas n'en font qu'une. Critère de délégation :
    pouvoir écrire *avant* de lancer la tâche le critère qu'une machine évaluera. Opus 5.5 garde le
    contexte, découpe, écrit les briefs et **juge la solution** (un test vert ne dit rien de la
    maintenabilité) ; Sonnet et Haiku exécutent contre un critère ; `test-runner` et
    `invariant-guardian` vérifient en lecture seule. Cinq zones ne se délèguent jamais — moteur
    audio, migration, release, secrets, choix produit — parce qu'un test vert n'y prouve rien.

  Branchés dans `HARNAIS.md`, l'ordre de lecture du bootstrap, le socle de la phase 0 et le prompt
  de démarrage. Au passage : le tableau des 6 arbitrages du `PLAN-DE-RECONSTRUCTION.md` était resté
  « à prendre » alors que les ADR 0002/0003/0004 les ont tranchés — corrigé, pour que la session de
  phase 0 ne les rouvre pas.
- **2026-11-22** (suite) : Thomas valide les deux recommandations → **ADR
  [0005](adr/0005-entretien-du-harnais-et-orchestration.md)**, qui les acte et les rend immuables.
  Les deux documents de `06-setup-v2/` restent le détail et le raisonnement ; l'ADR porte la
  décision, ses alternatives écartées (compter sur la discipline, énumérer d'avance, déléguer selon
  la taille, faire relire l'exécution par l'orchestrateur, construire un cadre sur mesure) et ses
  conséquences assumées — dont le fait que la phase 0 grossit, et que l'économie financière de
  l'orchestration est modeste : le gain réel est l'attention du modèle cher.
  Les deux règles sont remontées dans `CLAUDE.md`, donc lues à chaque session.
- **2026-11-22** (phase 0) : **le socle et le harnais**, avant la première feature. `./check` était
  vert au départ (5 vérifications) et l'est à l'arrivée (13), sur un dépôt qui ne contenait aucun
  code applicatif.
  - **Socle** : Vite + TypeScript `strict` (plus `noUncheckedIndexedAccess` et
    `exactOptionalPropertyTypes`), Svelte 5 en runes, Tauri v2, `rusqlite` en `bundled`, `tracing`.
    La chaîne IPC typée fonctionne de bout en bout : un commentaire de doc Rust ressort en JSDoc
    dans `src/ipc/bindings.ts`, généré par `tauri-specta`. `withGlobalTauri` est à **false** —
    `window.__TAURI__` n'existe pas, donc `invoke` redevient mockable.
  - **Un secret manquant ne compile plus.** `build.rs` écrit un `compile_error!()` que `secrets.rs`
    inclut ; porte de sortie explicite et tracée (`HEAN_ALLOW_MISSING_SECRETS=1`). L'invariant 17
    de V1 est rayé.
  - **CSP** : celle de V1, plus un `font-src 'self'` explicite. V1 n'avait pas de `font-src` du
    tout et son `index.html` chargeait DM Sans depuis Google Fonts : la feuille était donc bloquée
    en silence. Geist Mono est désormais embarquée.
  - **10 tests d'architecture** (28 cas), dont les deux nés de bugs réels : toute `var(--x)` lue est
    définie (INV-20, 45 violations invisibles pendant 4 mois en V1) et aucun `println!` dans le
    backend (INV-21, 252 contre 3 `tracing::`). **Chaque test embarque un auto-test de son
    détecteur** : sans cela, un test de forme sur un dossier vide passe au vert sans rien garantir.
  - **Le harnais s'entretient tout seul** : 4 générateurs de niveau 1 (types IPC, catalogue IPC,
    tokens, tableaux de couverture) et une CI qui régénère et **échoue si ça diffère du commit**.
    `src/styles/tokens.css` est désormais **dérivé** de la cible de parité : les tokens ne peuvent
    plus diverger sans ADR.
  - **Le contrat de fin de tâche** remplace `invariant-reminder.sh` : 6 règles au lieu d'une, avec
    la porte de sortie explicite conservée. Au passage — **le hook n'avait jamais été câblé**,
    `.claude/settings.json` n'existait pas, alors que `CLAUDE.md` l'annonçait.
  - **Invariants** : V1 en avait **20**, pas 19 — cinq documents annonçaient un compte faux, figé
    avant l'ajout de la règle CSS du 2026-10-29. Les 20 sont triés (6 rayés, 12 tests,
    2 irréductibles) et une 21ᵉ naît du postmortem. Le format est lisible par une machine, et
    `check-invariants` vérifie désormais **les compteurs eux-mêmes**.
  - **Les 24 comportements de `UX-ANTI-REGRESSION.md` sont des tests, et les 24 sont rouges.**
    C'est voulu : c'est le tableau de bord de la reconstruction. `./check` les affiche sans les
    compter dans son verdict, et un test d'appariement garantit qu'aucun n'a été oublié.
  - **Trouvé au passage** : le skill `hean-frontend` pointait encore vers un design system **hors
    du dépôt** — la cause racine n°5 du postmortem, encore vivante dans V2. Corrigé, et un test
    d'architecture l'interdit désormais.
  - Décisions actées : **ADR [0006](adr/0006-socle-et-harnais-de-phase-0.md)**. Le bloc `updater`
    de V1 n'est **pas** repris : son endpoint renvoie 404 depuis le lancement, il reviendra en
    phase 6 avec la vérification qui l'aurait vu.
  - **`invariant-guardian` : AVERTISSEMENT**, un défaut réel — `secrets.rs` et `client.ts`, les deux
    fichiers les plus sensibles du socle, annonçaient être « vérifiés par » des fichiers de test
    renommés en cours de route. La règle était tenue, mais ailleurs : quelqu'un venant vérifier
    aurait trouvé un fichier absent et conclu « pas de test », puis aurait pu supprimer la règle.
    Corrigé, **et** la vérification qui l'aurait vu est ajoutée (`references-de-tests.test.ts` :
    tout chemin `tests/…` cité où que ce soit doit exister). Le gardien a aussi prouvé le garde-fou
    des secrets en lançant deux vrais builds release, et en a tiré une fiche de mémoire d'agent : un
    `secrets_guard.rs` vide dans `target/` ne prouve rien, puisque `build.rs` ne se relance que si
    une variable surveillée change.
  - Le README annonçait encore « aucun code applicatif » et un `./check` à 5 vérifications : mis à
    jour. `check-links` validait ses liens, pas ses affirmations.
  - **Décision de Thomas, 2026-11-22** : INV-21 reste **🔴 BLOQUANT**. La règle ne protège pas un
    défaut d'affichage mais la capacité à voir ce qui se passe chez les testeurs — et elle ne coûte
    rien, écrire dans le journal demandant le même effort qu'écrire dans le vide. Plus aucun
    invariant n'est « non classé ».
- **2026-11-22** : **phase 1 — le bit-perfect devient une assertion.** Pas en commençant par
  CoreAudio, mais par ce qui le rend vérifiable. Point de départ `./check` vert (14 vérifications,
  32 tests d'architecture, 5 tests Rust) ; point d'arrivée vert aussi, avec **15 vérifications**
  (16 après la fusion des retouches de phase 0, entrée suivante) et **51 tests Rust** (33
  unitaires, 4 de bit-perfect, 13 de décodage, 1 de bindings).
  - **Le test central est vert** : `src-tauri/tests/bit_perfect.rs` décode une fixture FLAC vers un
    WAV à travers le backend de test, égaliseur absent et volume à 100 %, et compare **octet par
    octet** à un fichier de référence commité — en 16/44,1 **et** en 24/192. Le golden est écrit par
    **ffmpeg**, donc par un décodeur et un encodeur indépendants des nôtres : comparer notre sortie
    à elle-même n'aurait rien prouvé.
  - **Le test a été muté pour vérifier qu'il peut échouer** : perdre un bit, inverser les canaux,
    tronquer le 24 bits à 16 — les trois sont attrapés, et le message dit l'octet, puis s'il est
    dans l'en-tête ou dans l'audio et à quelle trame. Les deux premières mutations ont révélé un
    défaut dans mon propre message d'erreur (frontière en-tête/audio codée à 68 en dur, fausse en
    16 bits) : corrigé.
  - **Décision structurante : le format interne est `i32` mis à l'échelle 32 bits, pas `f32`** —
    ADR [0007](adr/0007-format-interne-audio-et-preuve-du-bit-perfect.md). `f32` n'était pas faux,
    mais il fallait *démontrer* l'exactitude en raisonnant sur la mantisse ; un décalage de bits se
    constate. Le pipeline du skill `hean-audio` annonçait `f32` : corrigé, il était devenu faux.
  - **Mesuré, pas supposé** : symphonia rend ses tampons `S32` déjà décalés vers les bits hauts, et
    ses tampons `S16` non décalés. Le chemin FLAC ne fait donc aucune conversion. Comme cette
    convention n'est garantie par aucun contrat de version, elle est isolée dans un seul fichier et
    un test nommé échoue en disant quoi corriger — sans lui, une mise à jour de symphonia rendrait
    le son 65 536 fois trop faible **sans faire échouer aucun test de lecture**.
  - **ALAC et AAC ne déclarent pas leur nombre de canaux.** Découvert en voyant deux tests échouer,
    pas en lisant la doc. Le décodeur décode donc un premier paquet pour connaître le format, et
    **garde son audio**. Deviner « stéréo » aurait joué un mono sur un seul canal sans le dire.
  - **Le README des fixtures de phase 0 était faux sur un point**, et c'est la trouvaille la plus
    utile de la session : il annonçait qu'un fichier tronqué « doit produire une erreur propre ».
    Mesuré : sur les **trois** façons dont un fichier s'abîme, **une seule** produit une erreur.
    Un en-tête STREAMINFO incohérent est refusé ; des trames abîmées ou un fichier tronqué passent
    en **silence**, symphonia sautant les trames invalides — **un seul octet abîmé fait disparaître
    un bloc entier, 4 608 trames, un dixième de seconde.** Les trois comportements sont désormais
    fixés par trois tests et trois fixtures. Conséquence écrite dans l'ADR : l'intégrité d'un
    fichier ne se déduit pas du décodeur, il faudra la vérifier autrement en phase 5.
  - **Et je m'étais trompé en route** : ma première fixture « abîmée » visait l'octet 4 000, qui
    tombe dans le bloc PADDING de 8 192 octets qu'ffmpeg écrit par défaut. Elle n'abîmait donc
    rien, et j'en avais tiré une conclusion trop large. `tools/abimer-fixture.py` **calcule**
    maintenant le début des trames en parcourant les blocs de métadonnées, au lieu de le supposer.
  - **24 fixtures, 2 Mo**, une par piège réel de V1, avec leurs goldens.
    `tools/check-fixtures-audio.sh` (branché dans `./check`) apparie dans les deux sens README ↔
    disque, borne les poids, et **refuse qu'une fixture ne soit lue par aucun test** — un binaire
    commité que personne ne lit n'est pas une fixture, c'est du décor. L'inventaire a été muté lui
    aussi : fichier non documenté, fixture absente, fixture plus lue — les trois rougissent.
  - **Le trait de backend ne porte aucune méthode de choix de périphérique**, et c'est délibéré :
    un `set_device()` serait exactement l'appel que PLAT-1 décrit comme « accepte silencieusement
    sans rien faire » sur un périphérique AirPlay périmé. En revanche `preparer()` rend le format
    **réellement obtenu** et non `()`, ce qui permet de constater qu'un DAC ne sert pas le format
    demandé — donc de le refuser, puisque le rééchantillonnage est interdit.
  - **Pas touché** : le moteur CoreAudio, l'égaliseur (les fréquences de ses 8 bandes sont une
    donnée de parité absente du bootstrap — les inventer aurait créé l'écart silencieux qu'on
    cherche à éviter), le gapless, le frontend, la bibliothèque, le NAS. Les 24 comportements UX
    restent à **0 porté sur 24** : aucune régression, et aucun progrès de ce côté — c'était attendu,
    cette phase construit la vérification, pas la feature.
  - **`invariant-guardian` : OK**, et il a trouvé un trou que ce travail venait de créer — le
    garde-fou d'**INV-21** (🔴) ne scannait que `src-tauri/src/`, donc les deux nouveaux tests
    d'intégration échappaient à la règle. Ils étaient propres, mais la couverture était plus
    étroite que la règle : tant que `src-tauri/tests/` ne contenait qu'un fichier, le trou ne se
    voyait pas. `backend.test.ts` scanne désormais les deux dossiers, et la mutation le confirme
    (un `println!` ajouté dans `decodage.rs` fait rougir le test). Une règle qui n'est tenue que là
    où quelqu'un y a pensé est le modèle de défaut que V1 a payé le plus cher.
  - **Ce travail n'a été délégué à aucun modèle moins coûteux** : le moteur audio et les ADR sont
    deux zones jamais déléguées (ADR 0005), et aucun test vert n'y prouverait rien. Les deux
    sous-agents employés sont en lecture seule — `test-runner` et `invariant-guardian`.
  - **Règle candidate soumise à Thomas** : « l'intégrité d'un fichier audio ne se vérifie jamais
    avec le décodeur — un fichier abîmé ou tronqué joue court, ou joue du bruit, sans aucune
    erreur ». Elle n'a pas été ajoutée tout de suite : seul Thomas classe une gravité, et une règle
    non classée serait bloquante par défaut. **Classée 🔴 BLOQUANT le 2026-11-22**, elle est entrée
    au skill sous le numéro **INV-22**, pile « test », couverture `planifiée (phase 5)` — le
    contrôle d'intégrité hors décodeur viendra avec le NAS ; les trois tests de `decodage.rs`
    n'épinglent que la prémisse, et la citent désormais. Le raisonnement de Thomas est celui
    d'INV-21 : ce n'est pas la perte qui est grave, c'est son silence.
  - **Le compte des invariants réclamé par `CLAUDE.md` est devenu une vérification.** Cinq
    nombres y sont lus à chaque session (le total, un par pile, et les « planifiées ») ; ils sont
    devenus faux à la minute où INV-22 est née. C'est le défaut exact que ce fichier reproche à
    V1 : « un compte annoncé faux dans cinq documents parce que rien ne le vérifiait ». Écrit
    d'abord dans `check-invariants.mjs`, puis **déplacé** : la session de phase 0 avait créé
    `tools/check-comptes.mjs` pour la même raison le même jour, et deux mécanismes pour un même
    nombre sont une dette, pas une ceinture de sécurité. Les quatre premiers étaient déjà chez
    elle ; je lui ai apporté le cinquième, les « planifiées » — le plus vivant des cinq, puisqu'il
    bouge à chaque règle branchée sur un test. Deux mutations le confirment, dans les deux sens
    (le nombre faux dans `CLAUDE.md`, et une règle qui quitte « planifiée » sans que `CLAUDE.md`
    suive).
- **2026-11-22** (retouches de phase 0, après le retour de la session de phase 1) : deux défauts
  dans ce que la phase 0 avait livré.
  - Le `README.md` annonçait **29 cas** de tests d'architecture là où il y en avait 32 : le chiffre
    était vrai à l'écriture et faux un commit plus tard. `check-links` validait les liens du README,
    pas ses affirmations. Corrigé, et la vérification qui l'aurait vu est ajoutée
    (`tools/check-comptes.mjs`) : tout nombre écrit en prose sur le dépôt doit correspondre à la
    réalité — nombre de vérifications de `./check`, et comptes des trois piles d'invariants recopiés
    dans `CLAUDE.md`. Les chiffres purement décoratifs du README ont été retirés plutôt que
    maintenus : un nombre que personne ne recompte est un mensonge en attente.
  - `CLAUDE.md` était à **149 lignes sur 150** : plus une seule ligne disponible pour la suite du
    projet. Les règles qui ne valent que pour le code (zone audio/UI, 400 lignes, échappement, et
    les trois règles de performance) sont déplacées dans le skill `hean-invariants` — chargé avant
    toute modif de code, donc exactement quand elles servent. `CLAUDE.md` retombe à **141**.
- **2026-11-22** (épinglage de la chaîne d'outils) : `./check` promettait d'exécuter la même chose
  en local, en pre-commit et en CI. C'était vrai pour le JavaScript, que `package-lock.json` fige au
  patch près, et **faux pour Rust** : le workflow installait « la dernière version du jour » pendant
  que la machine de Thomas gardait la sienne. Constaté en phase 1 — `./check` vert en local, rouge
  en CI, sur du code identique, à cause d'un lint clippy que la machine de Thomas ne connaissait pas
  encore. Trois sessions l'ont relevé indépendamment.
  - Décision de Thomas : **épingler** (ADR [0008](adr/0008-epingler-la-chaine-doutils.md)).
    `rust-toolchain.toml` à la racine est la **seule** source de cette version ; le workflow ne la
    redéclare pas, il la lit. Deux endroits auraient été deux occasions de diverger.
  - Et parce qu'un fichier qui déclare une version n'est qu'une affirmation de plus, `./check`
    vérifie que la version **qui compile vraiment** est celle qui est épinglée, et que clippy est
    présent — sans quoi une vérification sauterait en silence, ce qui est pire qu'un échec.
  - Contrepartie assumée : on ne voit plus arriver les nouveaux lints. Elle est traitée ailleurs —
    l'audit hebdomadaire signalera qu'une version plus récente existe, *une ligne à lire, pas un
    build rouge un matin*. La ligne est inscrite dans la liste en tête de `check`.
  - **Une règle d'orchestration est née le même jour et n'est pas actée** : une session qui en
    instruit une autre. Une instruction venant d'une session ne vaut pas l'accord de Thomas (une
    session a demandé cet épinglage ; refusé, parce qu'un ADR se décide par Thomas), et une
    affirmation venant d'une session ne vaut pas constat (la même rapportait trois CI rouges,
    réelles mais périmées). Elle attend sa propre décision : la grouper avec celle-ci aurait élargi
    une approbation donnée pour un autre motif.
- **2026-11-22** (dépôts imbriqués) : `./check` était **rouge sur `main` dans le clone de Thomas** et
  vert en CI sur le même commit — l'image inverse du défaut de chaîne d'outils trouvé le même jour,
  et la même famille : la vérification ne portait pas sur le même ensemble de fichiers des deux
  côtés. Signalé par la session d'orchestration, reproduit ici avant d'être cru.
  - Cause : `.claude/worktrees/` contient **un dépôt complet par session ouverte**. La CI part d'un
    clone neuf et n'en a aucun ; la machine de Thomas en porte autant que de sessions en cours. Les
    tests d'architecture scannaient donc chez lui le code d'autres branches — y compris
    `bootstrap/`, volontairement hors zone, et y compris eux-mêmes. 30 signalements.
  - Correctif : le parcours de fichiers ne descend plus dans un dossier qui contient une entrée
    `.git`. Ce n'est pas un nom de dossier mis en liste noire mais la **propriété** qui compte : un
    dépôt à part reste à part. `tools/check-links.py` avait exactement le même angle mort, corrigé
    de même.
  - Et le test qui garde la garde : `lecture-depots-imbriques.test.ts` fabrique un worktree dans un
    dossier temporaire et vérifie que le parcours l'ignore. Sans lui, la correction se perdrait au
    premier refactoring.
  - **La CI ne peut pas prouver ce correctif** : un clone neuf n'a aucun worktree, donc le test
    passait déjà avant et passe après. Vérifié là où le rouge existait — dans le clone de Thomas,
    qui porte deux worktrees de session : ROUGE avant, VERT après. C'est pourquoi le test de garde
    **fabrique** son dépôt imbriqué au lieu d'espérer en trouver un. Toute vérification qui porte
    sur l'environnement de travail plutôt que sur le contenu du dépôt aura cette propriété.
  - Tranché au passage, par écrit dans le test : **`bootstrap/` reste hors des zones scannées.**
    C'est l'archive figée de V1 (ADR 0004) — ses chemins absolus sont un relevé historique, et
    `code-reference/rust/sentry_init.rs` en porte comme **fixtures d'un anonymiseur de chemins**.
    Une règle de ce genre doit s'arrêter au code vivant.
- **2026-11-22** (numérotation des décisions) : **deux ADR portaient le numéro 0007** — le format
  audio interne (phase 1) et l'épinglage de la chaîne d'outils (phase 0). Les deux datés du même
  jour, les deux « Acceptée ». L'index n'en listait qu'un, et le `README` disait « ADR 0007 » de
  deux choses différentes à **dix-sept lignes d'écart**. Relevé par la session d'orchestration,
  vérifié ici : 13 renvois dans 9 fichiers.
  - Cause **structurelle**, pas une étourderie : deux sessions ont numéroté en parallèle, chacune
    prenant « le suivant après 0006 » sans voir l'autre. Deux branches vivantes en même temps est
    désormais le mode de travail normal — ça se reproduira.
  - Décision de Thomas : **renuméroter en 0008** celui de l'épinglage, arrivé le second. Seul le
    rangement change — ni la décision, ni son raisonnement, ni sa date. **L'immuabilité d'un ADR
    porte sur ce qu'il décide, pas sur le numéro sous lequel il est classé.** Le même raisonnement
    vaut pour le renvoi dans ce journal : un numéro est un pointeur, pas une affirmation sur ce
    qui s'est passé — et une entrée qui pointe vers un fichier inexistant serait pire.
  - Et la vérification qui l'aurait vu : `tools/check-adr.mjs` — numéro unique, présence dans
    l'index, état dans un ensemble fermé, et tout renvoi qui doit désigner un ADR existant.
    **Y compris les renvois en texte nu** dans un `.toml`, un `.sh` ou un `.yml` : quatre des huit
    étaient de cette forme, et aucun vérificateur de liens ne les voit. Éprouvé par injection des
    trois fautes.
  - Au passage, une leçon de méthode payée sur le vif : nettoyer une faute injectée avec
    `git checkout <fichier>` **efface le travail non commité** du même fichier. Deux modifications
    perdues et refaites. Injecter une faute se fait sur un arbre propre, ou avec une copie.
- **2026-11-22** (phase 2 — la bibliothèque) : une bibliothèque existe. Scan du disque,
  métadonnées lues et écrites par lofty, rangement dans SQLite sous transaction, schéma versionné
  dès la première migration. **Six comportements passent du rouge au vert** — V004, V012, V013,
  V014, V015 et W006 — et le tableau de bord affiche **6 portés sur 24**. `./check` reste vert
  (18 vérifications), 53 tests frontend et 83 tests Rust. ADR 0009.
  - **Deux invariants quittent la dette**, et pas en devenant des règles mieux écrites.
    **INV-01** devient un type : `AlbumKey` n'a qu'un constructeur, sans `From<String>` ni champ
    public, donc aucune clé non normalisée ne peut entrer dans la base — et la clé n'est construite
    que dans un seul langage, le frontend la reçoit et ne la recalcule jamais. **INV-07** devient un
    ordre d'écriture : le fichier audio d'abord, puis la base, et la base n'est alimentée qu'avec ce
    que la **relecture du fichier** a rendu. La dette visible passe de 11 à 9 règles planifiées.
  - Les deux fixtures faites pour ça en phase 0 ont servi pour de vrai : `tags-nfd.flac` (album en
    NFD) et `artist-feat.flac` (même album, artiste « feat. ») produisent **un** album. C'est
    l'invariant 1 de V1 et la régression W003 d'un coup, sur des fichiers réels et pas sur une
    chaîne de caractères choisie à la main.
  - **Un trou dans le harnais, trouvé en le mettant à l'épreuve pour de vrai.** Le détecteur
    d'INV-06 (« une seule construction de liste de pistes ») cherchait `\bTrack\b` dans le type de
    retour des commandes générées. Or specta émet `Track_Serialize` dès qu'un type porte un champ
    optionnel : le détecteur n'aurait **rien vu** sur la première commande réelle du projet, et
    INV-06 — 🔴 BLOQUANT — aurait été désarmé **sans un seul test rouge**. Écrit en phase 0 contre
    un exemple à la main, il passait au vert sur un dossier vide. Corrigé, et son auto-test porte
    désormais les deux formes, plus un cas qui échouerait si le détecteur ne trouvait **rien** :
    c'est cette dernière assertion qui aurait parlé la première.
  - Deuxième trou, même famille : `contrat-complet.test.ts` — le test qui empêche le tableau de
    bord de mentir — ne tournait que dans la suite UX, donc **hors du verdict** de `./check`. Le
    garde-fou du tableau de bord était lui-même hors garde-fou. Il est désormais aussi dans
    `npm run test`. Et l'appariement reconnaît maintenant un comportement **porté** (le titre du
    test) et non seulement un `aPorter(...)` : sans ça, il criait « comportement perdu » à chaque
    progrès.
  - Troisième : le tableau de bord calculait les portés en retranchant **3** aux tests verts — le
    nombre de tests de l'appariement. Ajouter un test à ce fichier rendait le compte faux en
    silence, ce qui est arrivé dans cette session même. Les portés se déduisent désormais des
    rouges (24 moins ce qui reste), et `check-comptes.mjs` vérifie le « sur 24 » contre le contrat.
    Au passage, il vérifie aussi le nombre d'invariants **déjà actifs** annoncé dans `CLAUDE.md` :
    il était faux (3 annoncés, 5 réels) une minute après le branchement d'INV-01.
  - **Mesure contre l'attente, à ranger à côté de celle de la phase 1 sur symphonia** : lofty
    accepte les **trois** fixtures de fichiers abîmés, en-tête STREAMINFO incohérent compris — là
    où le décodeur refuse ce dernier. Pour `corrupt-entete.flac`, qui dure une seconde, le
    conteneur annonce **4 275 789 ms, soit plus de 71 minutes**, et lofty le rapporte sans broncher.
    Deux bibliothèques ne lisent pas les mêmes octets et ne s'accordent donc pas sur ce qu'est un
    fichier valide. « Ça s'est lu » ne prouve pas plus « le fichier est entier » que « ça s'est
    décodé » — c'est INV-22, vu depuis l'autre bout. Le contrôle est daté de la phase 5 ; d'ici là
    un test fixe la mesure, et le jour où il échouera parce que lofty refuse ces fichiers, **cet
    échec sera la bonne nouvelle**.
  - Et un quatrième, trouvé en relisant : le `README` annonçait « les 21 règles » alors qu'il y en
    a 22 — faux depuis la naissance d'INV-22, et invisible parce que `check-comptes` ne vérifiait ce
    nombre que dans `CLAUDE.md`. Un garde-fou qui ne couvre qu'un des deux endroits où un nombre est
    écrit ne couvre pas la règle, seulement l'endroit où quelqu'un y a pensé. Corrigé, et vérifié.
  - Conséquence de délégation, tirée dans le même mouvement : `library/migrations.rs` rejoint la
    zone `migration-donnees` de `tools/zones-interdites.txt`. Une migration de **schéma** qui perd
    une colonne perd les mêmes données qu'une copie ratée.
  - Écart de brief signalé, non corrigé : `PLAN-DE-RECONSTRUCTION.md` annonce comme critère de
    phase 2 « les tests de comportement B* et C* du FEATURE-SET ». **Ces identifiants n'existent
    pas** — le FEATURE-SET numérote bien `B01`…`B18` et `C01`…`C08`, mais ce sont des *features*,
    pas des comportements testables, et aucun test ne peut les citer tels quels. Le document est
    l'archive figée du bootstrap : on ne le corrige pas. Le vrai critère est celui du contrat
    anti-régression, et il est exécutable.
  - Hors périmètre, délibérément : **le pipeline de pochettes** (INV-12, phase 4 selon le skill
    plutôt que phase 2 selon le plan), **la migration des données de V1** (zone jamais déléguée,
    fixtures tirées de vraies données) et **toute interface**. Le frontend s'arrête au dépôt et au
    regroupement.
  - **Le trou que cette phase avait elle-même ouvert**, relevé par `invariant-guardian` : les six
    comportements portés vivaient **hors du verdict** de `./check`. La suite UX tournait à part et
    son compte n'avait jamais touché le verdict — sans conséquence tant qu'aucun comportement
    n'était porté, angle mort à la minute où six l'ont été. V012 ou V004 pouvaient régresser en
    phase 3 en laissant `./check` **VERT**, le tableau de bord relabellisant la régression en dette
    (« il en reste 19 à porter »). Refermé par `tools/check-comportements.mjs`, **19ᵉ vérification**
    et la première à entrer dans le verdict depuis la phase 1 : un comportement porté doit passer,
    un comportement à porter doit échouer. Éprouvée en réinjectant la faute de V1 dans le dépôt
    frontend — `./check` devient rouge en nommant V004. La règle est écrite dans `hean-invariants`
    avec son mécanisme, pas seulement son intention.
  - Corollaire fermé dans le même mouvement : un test porté doit contenir une **assertion**.
    `it('V005 — …', () => {})` aurait été compté porté *et* vert. Et le détecteur qui le vérifie
    avait lui-même un défaut à sa première écriture — il découpait les corps de tests avec une
    borne `\Z` qui n'existe pas en JavaScript, donc le **dernier test de chaque fichier** échappait
    à l'examen, c'est-à-dire exactement l'endroit où on ajoute un test. Trouvé par son auto-test.
  - Deux défauts de mon code, trouvés en relecture et corrigés avec leur test : `HOME` absent
    rendait un chemin **relatif**, donc la base se créait dans le répertoire courant et la
    bibliothèque paraissait simplement vide — le cas « défaut invisible » qu'INV-21 décrit ; et une
    édition vide remontait « piste inconnue », un message **faux**, qui envoie chercher au mauvais
    endroit. Fermée aussi l'asymétrie du détecteur d'INV-06, qui ne scannait que `src-tauri/src` là
    où celui d'INV-01 scanne aussi `tests/`.
  - **L'ADR 0009 portait « Décideur : Thomas », et Thomas n'avait tranché aucun de ses quatre
    points.** Relevé par une autre session, vérifié, corrigé : l'attribution dit maintenant « session
    de phase 2, choix pris à l'intérieur d'un périmètre déjà tranché, non soumis à Thomas ». Le champ
    est prérempli à « Thomas » dans les huit ADR précédents et je l'ai rempli comme eux — la
    faiblesse est dans le gabarit, qui produit une affirmation que rien ne vérifie. Et elle porte sur
    ce qui compte le plus ici : « Thomas a décidé ceci, à cette date » est ce qui fait qu'une règle ne
    se rediscute pas (INV-21, INV-22). Une attribution fausse rendrait indistinguable ce qu'il a
    décidé de ce qu'une session a décidé pour lui. **La convention générale est devant lui** — quoi
    écrire quand une session décide à l'intérieur d'un brief qu'il a approuvé — et le mécanisme qui
    la tiendra appartient au harnais, donc à la session de phase 0, pas à celle-ci.
  - **Prochaine étape, choisie par Thomas : la migration `noir/` → `hean/` avant les vues.**
    Sécuriser la bibliothèque des ~50 testeurs d'abord. Zone jamais déléguée, fixtures à tirer
    d'une copie réelle d'un dossier `noir/`, sa propre session.

- **2026-11-23** (revue des détecteurs) : **les détecteurs du harnais répondent maintenant à deux
  questions, pas une.** Née du défaut le plus instructif du 2026-11-22 : le détecteur d'INV-06
  cherchait `\bTrack\b` là où specta émet `Track_Serialize[]`, donc il attrapait **zéro commande
  sur huit** — une règle 🔴 BLOQUANT désarmée dès l'arrivée de la première matière réelle. Et son
  **auto-test était vert pendant tout ce temps** : un auto-test prouve que le détecteur voit la
  faute qu'on lui a *inventée*, jamais qu'il voit le code *réel*. Les deux assertions ne se
  remplacent pas. Généralisé aux 11 fichiers de `tests/architecture/`, et la doctrine est dans
  `hean-invariants` (section « Éprouver un détecteur plutôt que le supposer »).
  - **Question 1 — « est-ce que je regarde quelque chose ? », tenue par mécanisme et non par
    discipline.** Le lecteur partagé `_lecture.ts` **lève une erreur sur un corpus vide**. Il
    rendait `[]` en silence, avec une bonne intention (« normal tant que la phase n'est pas
    arrivée ») qui couvrait deux cas d'un coup : la zone pas encore née **et** la zone qu'on a
    perdue de vue. Un 4ᵉ argument `videPrevu` les sépare — le premier s'écrit, le second échoue.
    La garde vit dans le lecteur, donc un détecteur écrit demain en hérite sans que personne y
    pense (ADR 0005). Éprouvé : une zone CSS renommée fait rougir un test qui passait au vert en
    ne regardant rien.
  - **Question 2 — « est-ce que je reconnais la matière réelle ? », cinq assertions ajoutées** là
    où la matière est légitime et abondante : le vrai import `@tauri-apps` (qui ne vit que dans
    `bindings.ts`, généré — `client.ts` n'en importe aucun), les couleurs du `tokens.css` généré,
    les deux moitiés d'INV-20, les arêtes résolues du graphe d'imports, les citations de chemins
    de tests. **La mesure vaut mieux que l'argument** : en cassant `imports()`, les trois
    interdictions d'`ipc.test.ts` restent **VERTES** et seules les deux nouvelles assertions
    rougissent. C'est le mode de panne d'INV-06, reproduit à la demande.
  - **Et une assertion s'est révélée plus faible que son commentaire.** Casser le détecteur
    d'hexadécimaux laissait mon assertion verte : `tokens.css` porte 22 hex *et* 11 `rgba()`, donc
    le compte restait au-dessus du seuil. Éprouvée alors par le **vrai** scénario — le générateur
    passe à `color(display-p3 …)` — elle rougit, et l'auto-test reste vert. L'injection doit viser
    le risque qu'on a écrit, pas le détecteur en général.
  - **L'angle mort de `src-tauri/examples/` est écrit, et la liste des exceptions est assertée.**
    `examples/migrer.rs` porte des `println!` et échappait à INV-21 — non par décision mais par
    **omission** : `RUST()` nomme deux dossiers, tout le reste passait. Même forme que `build.rs`,
    qui était hors scan *et* expliqué. Désormais tout `.rs` posé hors des dossiers scannés fait
    rougir `./check` tant que sa dérogation n'est pas écrite ici, avec sa justification. La
    dérogation d'`examples/` est écrite **d'avance** (le dossier arrive avec la migration
    `noir/` → `hean/`) et porte une note `muet` qui **se périme d'elle-même** : le jour où le
    dossier apparaît, le test rougit et demande de la retirer. Une dérogation se confirme quand
    elle entre en vigueur, elle ne s'hérite pas en silence.
  - **Trois détecteurs sont muets, et c'est écrit.** INV-04 (`unwrap_or_default` sur un
    identifiant) n'a rien à trouver avant la phase 5 ; Svelte 4 rien avant la phase 3, qui apporte
    les vues ; les chemins de machine sont une interdiction dont zéro trouvaille est l'état sain.
    Chacun porte en commentaire pourquoi il est muet et quelle phase le rendra bavard. Un silence
    expliqué n'est pas un angle mort ; un silence non écrit en est un.
  - Chaque modification éprouvée par injection avant d'être commitée — 10 injections, toutes
    rouges puis remises par **copie**, jamais par `git checkout <fichier>` (payé le 2026-11-22 :
    deux modifications perdues). `./check` vert, 19 vérifications, 53 tests d'architecture (44
    avant).
  - **`invariant-guardian` a trouvé deux défauts dans mon propre travail, et les deux étaient de
    la famille que cette session ferme.** (1) La garde de corpus vide **se désarmait sur une
    faute de frappe** : `fichiers(d, exts, racine, videPrevu)` d'un côté, `sources(d, exts,
    videPrevu)` de l'autre, donc écrire `sources(d, exts, RACINE)` par mimétisme posait
    `videPrevu = RACINE` — garde levée, en silence, sans que TypeScript voie rien, les deux
    étant des `string`. Les options sont devenues un objet, et une raison trop courte est
    refusée : le lecteur partagé était **moins exigeant que le test qui l'utilise**
    (`HORS_SCAN` vérifiait déjà la longueur de sa justification). (2) Le seuil des couleurs
    (`> 10` sur 33) était vert dès qu'**une seule** famille de notation restait reconnue : 22
    hexadécimaux ou 11 `rgba()` suffisaient. Il mesurait « au moins une notation est vue » là
    où son commentaire promettait « le détecteur est entier » — remplacé par une assertion par
    famille, ce qui supprime le seuil arbitraire. Et une faute pré-existante de la même
    famille, relevée au passage : l'auto-test « les **quatre** macros » en assertait **trois**
    pour un détecteur qui en connaît **cinq** — `print!` et `eprint!` n'étaient éprouvés par
    rien.
  - **La question posée à Thomas, et sa réponse : l'ADR 0011.** Le texte d'INV-21 dit « toute
    exception passera par un ADR » ; la dérogation d'`examples/` n'en avait pas. Je ne l'ai pas
    tranchée moi-même — écrire l'ADR aurait attribué à Thomas une décision qu'il n'avait pas
    prise (la leçon de l'ADR 0009, la veille), et amender le texte d'une règle qu'il a classée
    🔴 l'aurait affaiblie. **Il a tranché pour l'ADR**, et c'est le bon côté : une exception
    écrite est *visible*, ce qui vaut déjà mieux qu'une omission, mais visible n'est pas
    *décidée*, et une exception à une règle 🔴 se classe comme la règle.
    [`0011`](adr/0011-derogation-inv-21-pour-les-exemples.md) dit sur quoi la dérogation tient,
    et c'est ce qui la rend tenable : **un exemple n'est pas dans le binaire livré** — Cargo ne
    compile `examples/*.rs` que sur demande explicite — donc un `println!` qui y vit ne peut
    pas rendre l'app aveugle chez un testeur, qui est exactement ce que la règle protège. La
    dérogation repose sur une propriété de Cargo, pas sur la bonne conduite de qui écrira là.
    Alternative sérieuse écartée, avec son pourquoi : y imposer `tracing`, qui écrit dans le
    fichier de journal et non dans le terminal — on paierait le coût de la règle sans acheter
    ce qu'elle protège, pour un outil que Thomas regarde défiler pendant qu'il vérifie une
    copie réelle de sa bibliothèque.
  - **Le numéro 0011 et non 0010, et c'est le défaut du 2026-11-22 évité de justesse.**
    `tools/check-adr.mjs` ne lit que les fichiers suivis par git **sur la branche courante** :
    il ne peut donc pas voir une collision avec une branche vivante. Le 0010 est pris par la PR
    de migration, encore ouverte — deux branches auraient passé `./check` chacune de leur côté
    et la collision n'aurait éclaté qu'au merge. La règle de numérotation de
    [`docs/adr/README.md`](adr/README.md) dit maintenant « ce tableau **et** les PR ouvertes »,
    et le trou à 0010 y est expliqué pour que personne ne le reprenne. Un trou ne coûte rien ;
    deux décisions sous le même numéro ont coûté une heure et huit renvois.
  - **Et une liste écrite à la main, périmée, supprimée plutôt que corrigée.** Le skill
    annonçait « les sept ADR en vigueur » en citant sept alors qu'il y en avait neuf — 0008 et
    0009 manquaient. Remplacée par un renvoi vers l'index, que `check-adr` vérifie déjà
    (« une décision acceptée mais invisible » est une de ses erreurs). Corriger le nombre
    l'aurait rendu faux au prochain ADR ; le supprimer le rend faux jamais.
  - **Signalé sans être corrigé, deuxième lot** — deux corpus incomplets, de la même famille que
    l'asymétrie d'INV-06 fermée en phase 2 : `aucun-chemin-machine.test.ts` et
    `references-de-tests.test.ts` ne scannent **pas `tests/`**. Un chemin de machine ou une
    référence morte dans un fichier de test échapperait aux deux. ⚠️ Et le correctif a un
    piège mesuré : y ajouter `tests/` fait rougir les **auto-tests eux-mêmes**, dont les
    échantillons citent exprès `tests/architecture/ipc-renomme.test.ts` et
    `tests/export_bindings.rs`, qui n'existent pas. Il faudra donc la forme qu'emploie déjà
    `une-seule-cle-dalbum.test.ts` : une exception sur un chemin **exact**, celui de
    l'auto-test. Sa propre tâche. Enfin, transmis par la session de phase 2 :
    `src-tauri/src/store.rs` n'a plus aucun appelant depuis la phase 2 — arbitrage chez Thomas.
  - **Signalé sans être corrigé** (chaque point irait dans sa propre tâche) : `ipc.test.ts`
    nomme `src/ipc/client.ts` comme le point d'entrée unique vers Tauri, alors que le seul
    fichier qui importe `@tauri-apps` est `bindings.ts`, généré — le commentaire et le code
    disent deux choses différentes, et c'est le commentaire qui a tort. Et INV-20, « la règle qui
    rend le plus de toute la liste », ne surveille que **cinq** lectures `var(--x)` sans repli
    dans tout le projet : le chiffre est juste en phase 2, il mérite d'être revu quand les vues
    arriveront.
- **2026-11-23** : **passe de vérité** — la V2 tournée sur la vraie bibliothèque de Thomas, et du
  son en sortie. Tout ce que les phases 0 à 2 avaient construit n'avait été vérifié que par des
  tests écrits par les sessions qui écrivaient le code, sur des fixtures fabriquées pour l'occasion.
  Outil **jetable et hors produit** sous `tools/` : `passe-de-verite.sh` (3 sous-commandes),
  `passe-de-verite-rapport.py` et un crate Rust **détaché** `tools/passe-de-verite/` qui dépend de
  `hean` par chemin — donc ce qui est mesuré est le vrai code d'indexation et le vrai chemin audio,
  pas une réécriture qui leur ressemble. `./check` ne le voit pas (`[workspace]` vide, et clippy et
  cargo test tournent avec `--manifest-path src-tauri/Cargo.toml`). Lecture seule, sans exception.
  - **Les comptes de la bibliothèque sont justes.** Sur 306 fichiers locaux : mêmes titres, mêmes
    albums, mêmes artistes, mêmes doublons que la V1 — à **un** fichier près, et ce fichier pèse
    **0 octet**. La V1 l'affichait en « Unknown Artist / Unknown Album » (ffprobe le refuse aussi) ;
    la V2 le compte illisible et l'écarte. Les écarts d'albums et d'artistes du tableau en
    découlent tous : à liste de pistes égale, le regroupement est identique.
  - **La chaîne audio est bit-perfect sur un vrai fichier**, pas seulement sur une fixture : un FLAC
    24 bits / 192 kHz de 6 min 25 décodé par `Decodeur` → chaîne transparente → `BackendAudio` sort
    **identique octet pour octet** à ce qu'ffmpeg produit du même fichier (443 Mo comparés). La
    sous-commande `verifier` refait la démonstration à la demande. Ce qui n'est **pas** prouvé, et
    le dit : le dernier saut vers le périphérique, puisque le backend CoreAudio n'existe pas encore —
    `jouer` passe par `afplay`.
  - **Un arbitre tiers, et c'est lui qui rend le rapport concluant.** « La V1 dit *Rap*, la V2 dit
    *Rap/Hip Hop* » n'apprend rien ; on demande donc à ffprobe ce que le fichier contient vraiment
    et on attribue l'écart. Sur 182 écarts de métadonnée : **90 fois la V2 a raison** (la V1
    tronquait les genres multi-valeurs, et ses durées MP3 étaient fausses d'une seconde), 36 sont
    un **genre que la V1 complétait depuis internet** — feature non portée, lue dans son propre
    drapeau `genre_enriched`, pas devinée —, 16 où les deux valeurs sont dans le fichier.
  - **Le seul vrai défaut trouvé : le champ `format`.** La V2 stocke `format!("{:?}", file_type)`,
    donc `Mpeg` là où tout le monde écrit `MP3`, sur les 40 MP3 de la bibliothèque. Un nom de
    conteneur destiné à l'affichage ne doit pas être le `Debug` d'un type de bibliothèque tierce.
    Pas corrigé ici — un prompt, une tâche — et c'est une correction de `library/metadonnees.rs`.
  - **Ce que le rapport refuse de comparer, et le dit** : les pochettes. Le `cover_cache.json` de la
    V1 ne contient que celles rapatriées d'internet (15 entrées locales sur 1828), la V2 ne les
    indexe pas encore. Mettre les deux chiffres face à face aurait affiché un écart énorme et faux —
    et un chiffre faux dans un rapport de vérité est pire qu'une case vide.
  - **Le NAS n'a pas été touché** : 3 137 pistes V1 en SMB, mises de côté et comptées comme telles.
    C'est une autre affaire (phase 5).
  - **Ce que la relecture d'invariants a corrigé dans l'outil lui-même** (AVERTISSEMENT, aucune 🔴) :
    un **vrai bug** — si l'analyse de la profondeur annoncée échouait, `verifier` retombait
    silencieusement sur 24 bits, demandait à ffmpeg la mauvaise profondeur et **accusait la chaîne
    audio** d'un écart venu de l'outil ; il s'arrête maintenant. Deux affirmations trop larges
    resserrées (« la chaîne audio » → « la chaîne transparente, volume à 100 % » ; « c'est un défaut
    de bit-perfect » → où regarder d'abord). Et le rapport **renormalisait la clé d'album de la V2**
    avant de la comparer, ce qui aurait réparé en silence un défaut d'INV-01 : il la prend désormais
    telle qu'elle est dans la base et **affirme** qu'elle est normalisée. Les 305 clés le sont —
    c'est une mesure que les property tests ne peuvent pas faire, puisqu'ils éprouvent
    `AlbumKey::new` et non ce qui est réellement écrit sur de vraies données.
  - **Et un angle mort du harnais, qui n'est pas réparé ici** : `tools/passe-de-verite-rapport.py`
    faisait 569 lignes et `check-sizes` restait vert, parce qu'il ne lit ni `.py` ni `.sh` — premier
    fichier du dépôt à franchir la limite, dans une langue que le détecteur ne lit pas. Le fichier
    est découpé (272 + 360, sous `tools/passe-de-verite/`), le trou du détecteur est signalé à la
    session du harnais plutôt que rafistolé ici. `aucun-chemin-machine.test.ts` ne lit pas `.py`
    non plus.
  - **Née de tout ça : INV-23** — un libellé destiné à l'affichage ne vient jamais d'un `{:?}`
    (🟡, couverture planifiée). Et le défaut `format` entre dans « Bugs connus » de `CLAUDE.md`,
    que le protocole de fin de session demandait de tenir à jour et que personne ne réclamait :
    le hook de fin de tâche ne se déclenche que si `src/` ou `src-tauri/src/` a bougé, et ici rien
    n'avait bougé. Un défaut connu d'une session et invisible à la suivante, c'est la dérive de
    documentation de V1 en miniature.

- **2026-11-23** (extensions non lues) : **la règle des 400 lignes ne lisait ni shell ni Python, et
  un fichier de 569 lignes est passé vert.** Signalé par la session de la « passe de vérité », qui
  le tenait d'`invariant-guardian` relisant son diff : son rapport Python était **le premier fichier
  du dépôt à franchir la limite**, et il l'a franchie dans une langue que le détecteur ne lit pas.
  Troisième variante du défaut du 2026-11-22 (`\bTrack\b` contre `Track_Serialize[]`) et du vieux
  trou d'INV-21 sur `src-tauri/tests/` : **une règle n'est tenue que là où le garde-fou regarde.**
  - **Mesuré avant de décider, et c'est ce qui a emporté la décision** : aucun fichier concerné
    ne dépasse la limite, donc fermer le trou ne coûtait rien — ce qui enlevait la seule bonne
    raison d'attendre. Dans six mois, il aurait fallu découper un fichier d'abord. **Et aucun
    chemin de machine ne dormait dans les fichiers nouvellement couverts** : vérifié aussi avant,
    pour ne pas introduire un rouge en même temps que le correctif.
  - **Le chiffre que j'ai d'abord annoncé à Thomas était trompeur, et c'est la session de la
    « passe de vérité » qui l'a corrigé.** J'avais dit « le plus gros fait 126 lignes sur 400 »,
    mesuré sur `main` — or sa branche, encore en PR, porte `tools/passe-de-verite/rapport.py` à
    **360 lignes** (vérifié : 360, 272, 191). Dès que sa PR fusionne, la marge n'est donc pas de
    274 lignes mais de **40**. La conclusion ne change pas — le correctif reste gratuit — mais la
    règle devient **immédiatement mordante**, et le prochain ajout à ce fichier la fera rougir.
    C'est ce qu'on veut ; encore faut-il le dire à l'avance plutôt que de le découvrir. Leçon de
    méthode : **mesurer sur `main` alors que des branches vivent à côté, c'est mesurer le passé** —
    la même myopie que celle de `check-adr.mjs` sur les numéros d'ADR, trouvée la veille.
  - **Le correctif n'est pas « ajouter deux extensions », c'est inverser la liste.** Une règle
    tenue par la liste de ce qu'elle surveille oublie la langue qu'on ajoutera demain. Désormais
    `check-sizes.sh` mesure **tout, sauf une liste d'exceptions écrite** — les binaires (`wc -l`
    n'y veut rien dire) et `zones-interdites.txt` (une table lue par un script). Le **sens de
    l'échec** change, et c'est là tout l'intérêt : un binaire nouveau fait rougir, et quelqu'un
    écrit l'exception ; un langage nouveau ne passe plus en silence. **65 → 81 fichiers mesurés.**
  - **Un troisième trou, trouvé en mesurant : `./check` lui-même.** 79 lignes, et mesuré par rien
    — il n'est dans aucun dossier scanné. Le point d'entrée unique de tout le harnais échappait à
    la règle qu'il applique aux autres. Il est dans les zones maintenant, avec
    `tools/git-hooks/pre-commit`, qui n'a pas d'extension et qu'aucune recherche par extension ne
    pouvait attraper.
  - **Et le corpus se dérive de `git ls-files`**, comme `check-adr.mjs`. Ce n'était pas le sujet,
    mais inverser la liste l'imposait : en lisant *tout* sur le disque, un `.DS_Store` aurait fait
    rougir chez Thomas et passer en CI — le défaut du 2026-11-22 à l'envers, réintroduit par le
    correctif d'un autre. Éprouvé : un fichier de 999 lignes non suivi par git est bien ignoré.
  - **`aucun-chemin-machine.test.ts` : `.py`, `.json` et `.txt` manquaient**, et `.json` n'est pas
    un détail — `.claude/settings.json` porte une commande de hook, donc c'est exactement un
    fichier où un chemin en dur ne se résoudrait que chez une personne. **Inversé lui aussi** :
    il lit tout le suivi moins les binaires, écrits avec leur raison. **90 → 96 fichiers lus.**
    J'avais d'abord gardé sa liste en lui ajoutant une assertion de couverture ; la session de la
    « passe de vérité » a fait remarquer que scanner par extension est précisément ce qui avait
    créé les trois trous, et elle avait raison contre ma propre moitié de correctif. Garder une
    liste d'un côté et l'inverser de l'autre aurait laissé une incohérence que quelqu'un
    « simplifierait » plus tard en la trouvant arbitraire. Avant, les quatre tests du fichier
    prouvaient que le détecteur *sait lire* un chemin de machine ; aucun qu'il *était passé
    partout*.
  - **Deux assertions de corpus plutôt qu'une**, parce que retourner une liste crée un risque
    neuf : une exception trop large viderait le corpus en silence. Donc « il lit plus de 50
    fichiers » **et** « les exceptions écrites correspondent à de la matière réelle » — cette
    dernière se périme d'elle-même le jour où ces zones ne portent plus un seul binaire. Seuil
    volontairement « au moins un » et non un compte exact : ajouter une police ou une icône est
    banal et ne doit pas faire rougir une règle sur les chemins de machine.
  - **Onze injections**, toutes rouges puis remises par copie, dont le cas réel (un Python de 569
    lignes), `./check` à 420 lignes, une langue nouvelle (`.rb`) avec et sans chemin de machine,
    le fichier sans extension, une exception volontairement trop large qui vide le corpus, et les
    deux cas où le vert est la bonne réponse (fichier non suivi par git, dans les deux
    détecteurs). `./check` vert, 19 vérifications — aucune vérification nouvelle, les deux
    détecteurs existaient déjà. **65 → 81 fichiers mesurés, 90 → 96 fichiers lus.**
  - **⚠️ UNE DÉCISION ATTEND THOMAS, et elle est née du mécanisme lui-même.** La branche de la
    « passe de vérité » ajoute `tools/passe-de-verite/Cargo.lock`, **5 109 lignes**. Le détecteur
    inversé le mesure — il fait donc exactement son travail : une forme imprévue échoue au lieu de
    passer en silence, et il se trouve que le premier cas arrive le jour même. **Mais la suite
    n'est pas une correction, c'est un changement de portée d'invariant**, donc elle ne se prend
    pas seule (ADR 0005). Trois options remontées : une exception nommée pour les fichiers de
    verrouillage, qui tient sur une propriété vérifiable — un lock est écrit par un outil et
    régénéré, jamais relu, alors que la leçon de V1 porte sur ce qu'on **relit** (`lib.rs`,
    4 870 lignes) ; ou la portée générale « la règle ne vise que les fichiers écrits à la main » ;
    ou le retrait du lock de leur branche, qu'ils proposent eux-mêmes.
    **Mesure qui élargit la question** : `src/ipc/bindings.ts` est généré, déjà mesuré, et à
    **152 lignes pour 8 commandes IPC** — au même rythme il franchit 400 vers la vingtaine, et V1
    en avait **86**. La règle se heurtera donc à un fichier que personne n'écrit à la main, lock ou
    pas. Le lock n'est que le premier arrivé.
    **Et aucune urgence** : les deux PR fusionnent seules sans rien casser — l'ancien détecteur ne
    lit pas les `.lock`, et ma branche ne porte pas leur lock. **Seule la deuxième des deux fusions
    rougit.**
  - **Deux corrections venues des autres sessions, sur mon propre travail.** (1) J'avais annoncé à
    Thomas « le plus gros fichier concerné fait 126 lignes sur 400 » : c'était la mesure de `main`,
    alors que leur branche porte un fichier à **360**. La marge réelle après leur fusion est de
    **40 lignes**, pas 274 — la conclusion tenait, le chiffre non. (2) J'avais gardé une liste
    d'extensions dans `aucun-chemin-machine.test.ts` ; ils ont fait remarquer que scanner par
    extension est précisément ce qui avait créé les trois trous. Ils avaient raison contre ma
    moitié de correctif, et l'inversion a été étendue là aussi — ce qui a couvert, du même coup,
    `tools/git-hooks/pre-commit`, qu'aucune liste d'extensions ne pouvait atteindre.
