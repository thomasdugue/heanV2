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
