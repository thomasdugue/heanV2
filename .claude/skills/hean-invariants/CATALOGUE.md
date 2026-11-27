# Catalogue des invariants — les 29 fiches

<!-- Lu par `tools/check-invariants.mjs`, `tools/check-comptes.mjs` et `tools/gen/couverture.mjs`,
     conjointement avec SKILL.md. Garder le format `### INV-NN` + les quatre champs. -->

**Ne pas ouvrir ce fichier pour écrire du code.** Les règles qu'il décrit sont tenues par des
tests : `./check` t'arrêtera si tu en casses une, et c'est tout ce dont une session a besoin.
Il s'ouvre pour **ajouter une règle, en modifier une, ou comprendre pourquoi une vérification
vient de refuser un diff**. Ce qui ne peut être tenu par aucun test — les pièges de plateforme —
est resté dans [`SKILL.md`](SKILL.md), qui est lu avant chaque modification de code.

Le tri en trois piles, les quatre champs obligatoires et la gravité sont définis dans
[`SKILL.md`](SKILL.md). Vue générée et à jour :
[`docs/generated/invariants.md`](../../../docs/generated/invariants.md).

---

## Pile 1 — rayées par construction

Ces six règles n'existent plus. Elles ne sont pas « respectées » : elles sont inexprimables.
La justification est obligatoire — c'est elle qui permet de rouvrir le sujet si
l'architecture change.

### INV-06 — Suppression de pistes = PERMANENTE
- pile : rayé
- gravité : 🔴 BLOQUANT
- couverture : sans objet — architecture `une-seule-source-de-pistes.test.ts`
- pourquoi : en V1 la règle disait « le filtre `excluded_paths` doit être vérifié à CHAQUE
  point d'entrée, et dans tout nouveau point d'entrée » — trois points d'entrée listés à la
  main. Une règle formulée comme « n'oublie pas de répéter ça partout » sera violée au
  quatrième point d'entrée. En V2 il n'y a **qu'un** constructeur de liste de pistes
  (`library/repository`), donc le filtre n'a qu'un endroit où vivre, et un test
  d'architecture refuse tout appel direct ailleurs.

### INV-08 — METADATA_CACHE : UPDATE, jamais REMOVE
- pile : rayé
- gravité : 🔴 BLOQUANT
- couverture : sans objet — SQLite, un seul écrivain, transactions (ADR 0002)
- pourquoi : supprimer une entrée au lieu de la mettre à jour créait une race fatale — le
  scan de fond relisait le fichier audio pas encore écrit par lofty, réinsérait les anciennes
  métadonnées, et **la corruption persistait indéfiniment**. Cette race supposait sept caches
  globaux modifiés concurremment sans transaction. Il n'y a plus de cache à supprimer.

### INV-09 — start_background_scan() : re-application du cache de métadonnées
- pile : rayé
- gravité : 🔴 BLOQUANT
- couverture : sans objet — SQLite, un seul écrivain, transactions (ADR 0002)
- pourquoi : il fallait re-synchroniser les métadonnées depuis un snapshot pris **avant** le
  lock de `TRACKS_CACHE` « pour éviter le deadlock ». Une règle dont l'énoncé mentionne un
  ordre de prise de verrous décrit un défaut de conception, pas un invariant métier.

### INV-10 — METADATA_CACHE : jamais de donnée périmée
- pile : rayé
- gravité : 🔴 BLOQUANT
- couverture : sans objet — SQLite, un seul écrivain, transactions (ADR 0002)
- pourquoi : troisième face du même problème que INV-08 et INV-09. Les éditions de
  l'utilisateur pouvaient être perdues par un scan concurrent.

### INV-14 — Mosaïque de playlist : rebuild automatique après chargement async des covers
- pile : rayé
- gravité : 🟡 AVERTISSEMENT
- couverture : sans objet — Svelte 5 : le DOM découle de l'état (ADR 0002)
- pourquoi : les covers arrivaient en async dans des cellules déjà déconnectées par un
  rebuild concurrent. Le correctif V1 tenait en trois pièces, dont un garde
  `_thumbsRefreshScheduled` contre la boucle infinie et un rebuild planifié via
  `requestAnimationFrame`. Avec un modèle de composants réactif, il n'y a ni cellule
  déconnectée ni rebuild manuel : toute la famille disparaît.

### INV-17 — option_env!() pour TOUS les secrets compile-time
- pile : rayé
- gravité : 🔴 BLOQUANT
- couverture : sans objet — `src-tauri/build.rs` + `compile_error!()`, et architecture
  `backend.test.ts`
- pourquoi : `option_env!()` renvoie `None` **en silence** si la variable manque. Le binaire
  release partait alors sans Sentry et sans feedback, sans aucune erreur — invisible jusqu'à
  l'exécution du binaire livré. V1 s'en protégeait avec un wrapper `release.sh` qu'il suffisait
  d'oublier une fois. En V2 **le compilateur refuse** : un build release sans secrets ne
  compile pas. Porte de sortie explicite et tracée : `HEAN_ALLOW_MISSING_SECRETS=1`.

---

## Pile 2 — devenues des tests

La règle a quitté la tête de quelqu'un. `couverture : planifiée (phase N)` veut dire que le
test n'existe pas encore parce que le code qu'il protège n'existe pas encore — c'est une
dette **visible**, que `./check` recompte à chaque passage.

### INV-01 — Un album = une seule entrée
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : test `cle.rs` — type `AlbumKey` à constructeur unique + proptests, et
  architecture `une-seule-cle-dalbum.test.ts` côté frontend
- pourquoi : trois bugs, une seule cause — une normalisation (trim + NFC) écrite plusieurs
  fois. L'invariant lui-même, la régression W003 (dédup par `artist+album` au lieu d'`album`
  seul : les variations « feat. » créaient de faux albums) et un lookup à quatre niveaux dans
  la home parce que deux formats de clé coexistaient. Si la seule façon d'obtenir une
  `AlbumKey` est `AlbumKey::new(raw)`, ce n'est plus une règle, c'est le système de types.
  **Branchée sur un test le 2026-11-22 (phase 2).** Trois mécanismes, et c'est leur
  empilement qui compte : (1) le type `AlbumKey` n'a qu'un constructeur, sans `From<String>`
  ni champ public, donc aucune clé non normalisée ne peut entrer dans la base ; (2) des
  proptests éprouvent l'idempotence et l'équivalence NFC/NFD sur **toute** chaîne Unicode
  générée, plus les deux fixtures réelles `tags-nfd.flac` et `artist-feat.flac` ; (3) un
  test d'architecture refuse toute normalisation Unicode écrite à la main dans `src/` **et
  dans `tests/`** — un faux backend qui normaliserait serait une deuxième implémentation
  arrivée par la porte du harnais.

### INV-02 — Pas de duplication visuelle sur la home
- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : planifiée (phase 3) — test de comportement sur « Recently Played »
- pourquoi : la section dédupliquait par chemin de fichier, donc un même album écouté par
  plusieurs pistes apparaissait plusieurs fois.

### INV-03 — SMB : connexion périmée = retry automatique
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : test reprise.rs
- pourquoi : une opération SMB doit retenter avec une connexion fraîche si la première
  échoue — un NAS ferme une session inactive, et `libsmbclient` ne s'en aperçoit qu'à
  l'opération suivante. En V1 les tests `network_source` étaient `#[ignore]` : la règle ne se
  vérifiait qu'avec un vrai NAS, dans le bon état.
  **Elle n'attend plus la phase 5, et c'est la leçon à retenir de son branchement** : la
  *réaction* à une connexion périmée a été séparée de sa *cause*. `reessayer_une_fois` ne
  connaît ni SMB ni le réseau — deux fermetures, l'opération et le rafraîchissement — donc
  elle s'éprouve par **injection de panne**, sans conteneur Samba et sans NAS allumé. Ce qui
  demandait un vrai serveur était la cause, pas la réaction ; une couverture « planifiée »
  peut donc cacher un découpage à faire plutôt qu'un outil à installer.
  Quatre cas éprouvés, et le troisième est celui qu'on oublie : la première tentative échoue
  puis la seconde réussit ; un échec persistant **ne boucle pas** (une boucle « jusqu'à ce que
  ça marche » transformerait un NAS éteint en attente infinie — W005 pris par le mauvais
  bout) ; une reconnexion qui échoue elle-même remonte **sa** cause, sinon l'utilisateur lit
  « session périmée » alors que son mot de passe est refusé ; et un succès du premier coup ne
  reconnecte jamais, donc le chemin normal — 3 137 fichiers — ne paie pas la règle.
  **Et ce garde-fou a rattrapé un autre défaut sans le signaler, le 2026-11-25** : la
  connexion en cache était réutilisée dès que l'hôte et le partage correspondaient, **sans
  égard pour les identifiants**, donc corriger un mot de passe mal tapé n'avait aucun effet.
  Le scan finissait quand même par passer — au coup d'après, par cette reprise-ci. Thomas a
  saisi son mot de passe quatre fois en croyant se tromper. Leçon à garder : **une reprise
  qui réussit silencieusement masque la cause**, et un `WARN` par tentative perdue n'a pas
  suffi à la faire voir. La correction est dans `client.rs` (l'empreinte des identifiants
  entre dans la question « est-ce la bonne connexion ? »), et une opération qui a échoué
  deux fois ne reste plus en cache.

### INV-04 — SMB : un échec Keychain remonte une erreur, jamais un mot de passe vide
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : architecture `backend.test.ts`
- pourquoi : `retrieve_password().unwrap_or_default()` fait tenter au NAS une connexion avec
  un mot de passe vide — donc un **risque de verrouillage du compte** de l'utilisateur, pour
  une erreur qui ne s'affiche nulle part. Le test refuse `unwrap_or_default()` sur toute
  ligne qui parle d'identifiant.

### INV-05 — SMB : l'erreur du thread de download remonte à l'utilisateur
- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : planifiée (phase 5) — conteneur Samba
- pourquoi : sans récupérer `LAST_DOWNLOAD_ERROR` quand `bytes_written == 0`, l'utilisateur
  voit un message générique au lieu de la cause réelle.

### INV-07 — Une métadonnée éditée est écrite dans le fichier audio
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : test `edition.rs` — écriture lofty, puis relecture du fichier
- pourquoi : le cache est un accélérateur, **la source de vérité est le fichier audio**.
  SQLite règle la cohérence entre caches (INV-08/09/10) mais ne dit rien de ça : si on
  n'écrit que dans la base, la métadonnée disparaît dès que l'utilisateur ouvre le fichier
  ailleurs. C'est pour cette raison que cette règle n'est pas rayée avec ses voisines.
  **Branchée sur un test le 2026-11-22 (phase 2).** Elle est devenue un ORDRE D'ÉCRITURE :
  le fichier d'abord, puis la base, et la base n'est alimentée qu'avec ce que la **relecture
  du fichier** a rendu (`library/metadonnees.rs`). Si l'écriture échoue, la base n'a pas
  bougé ; si lofty normalise une valeur à sa façon, c'est sa version qui entre dans la base,
  donc la base ne peut pas annoncer autre chose que le fichier. Le test édite une vraie
  fixture copiée en zone temporaire, **supprime la base**, rescanne, et retrouve la valeur :
  elle ne peut venir que du fichier.

### INV-11 — Mosaïque de playlist : déduplication par album seul
- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : planifiée (phase 3) — property test « invariant par permutation »
- pourquoi : dédupliquer par `artist+album` fait réapparaître la même pochette N fois dans
  une mosaïque dès qu'un album a des variations « feat. ». Régression du 2026-03-16.

### INV-12 — Mosaïque de playlist : le fallback internet est obligatoire
- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : planifiée (phase 4) — test des 4 niveaux du pipeline de covers
- pourquoi : sans le 4ᵉ niveau (`fetch_internet_cover`), tout album dont la pochette ne vient
  que de MusicBrainz/Deezer affiche une cellule noire.

### INV-15 — Boutons « Add to Queue » / « Add to Playlist » au survol
- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : planifiée (phase 4) — régression visuelle, état `hover`
- pourquoi : les règles CSS d'opacité ont déjà été supprimées une fois. Un état de survol ne
  se revoit que par hasard — c'est exactement ce que la capture état par état attrape.

### INV-16 — Sentry : before_send jette les events si le toggle est éteint
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : planifiée (phase 6) — test unitaire avec `SENTRY_ENABLED = false`
- pourquoi : sans ce test, couper le toggle dans Settings ne stoppe pas l'envoi des events
  déjà dans le hub. C'est un sujet **RGPD**, pas un confort. Les tests `scrubs_*` de V1
  couvraient l'anonymisation des chemins mais pas le toggle : il fallait « vérifier
  visuellement après chaque modif », c'est-à-dire ne pas vérifier.

### INV-19 — Bouton shuffle : une seule source pour l'icône et le mode
- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : planifiée (phase 3) — un seul module exporte le glyphe et le cycle de mode
- pourquoi : le statut collé au texte du bouton (`'⤮ᴬ'`) décalait l'icône, et la logique
  recopiée dans `shortcuts.js` affichait **un autre glyphe** (⤢ au lieu de ⤮). Ce n'est pas
  un bug d'affichage, c'est un symptôme de duplication. Régression du 2026-10-29.

### INV-20 — Toute variable `var(--x)` utilisée est définie
- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : architecture `css-variables-definies.test.ts`
- pourquoi : **la règle qui rend le plus de toute la liste.** Une variable CSS non définie
  rend la déclaration invalide « au moment du calcul » : le navigateur l'ignore **sans
  erreur**, même avec `!important`. Ni la relecture, ni les tests de comportement, ni le
  type-checking ne l'attrapent. En V1, `--glass-lo` / `--glass-md` / `--glass-hi` ont été
  utilisées **45 fois sans définition** sur toute une PR de refonte : bordures, survols et
  état actif de la sidebar invisibles pendant **quatre mois**. Le test tient en vingt lignes.

### INV-21 — Aucun `println!` / `eprintln!` dans le backend
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : architecture `backend.test.ts`
- pourquoi : **règle née le 2026-11-22**, du postmortem de V1. V1 comptait **252 `println!`
  contre 3 appels `tracing::`**, et son `logging.rs` ne capturait pas stdout : l'app
  installée a produit **63 lignes de journal dans toute sa vie**. Les trois bugs trouvés le
  2026-11-21 — dont la mise à jour automatique qui renvoie 404 depuis le lancement, et qui
  n'a donc jamais fonctionné pour aucun testeur — étaient tous annoncés en clair par le build
  de dev et **invisibles en production**. **Classée 🔴 BLOQUANT par Thomas le 2026-11-22** :
  ce que cette règle protège n'est pas un défaut d'affichage, c'est la capacité à voir ce qui
  se passe chez l'utilisateur. Un défaut visuel se voit et se répare ; être aveugle ne se voit
  pas. Et le coût de la règle est nul — écrire dans le journal demande exactement le même
  effort qu'écrire dans le vide. Toute exception passera par un ADR.
  **Deux dossiers sont hors du scan, la liste est assertée, et la dérogation est tracée par
  l'[ADR 0011](../../../docs/adr/0011-derogation-inv-21-pour-les-exemples.md)** (2026-11-23) :
  `build.rs`, dont les `println!("cargo::…")` sont le protocole de Cargo, et
  `src-tauri/examples/`, dont les exemples se lancent à la main — sans sortie ils n'ont aucun
  intérêt, et ils ne sont pas dans le binaire livré, donc ils ne peuvent pas rendre l'app
  aveugle chez un testeur, ce que la règle protège. `backend.test.ts` refuse maintenant tout
  `.rs` posé hors des dossiers scannés sans dérogation écrite : en ajouter une demande de
  toucher au test, au lieu de se produire par omission.

### INV-22 — L'intégrité d'un fichier audio ne se vérifie jamais avec le décodeur
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : planifiée (phase 5) — contrôle hors décodeur : durée annoncée par le conteneur contre trames réellement rendues
- pourquoi : **règle née le 2026-11-22 d'une mesure**, et **classée 🔴 BLOQUANT par Thomas
  le 2026-11-22** — classée, pas « bloquante par défaut » : une règle classée avec son
  pourquoi ne se rediscute pas. Sur les trois façons dont un fichier audio s'abîme dans la
  vraie vie, **une seule produit une erreur** : un en-tête STREAMINFO incohérent. Des trames
  abîmées sont sautées en silence — **un seul octet suffit à faire disparaître 4 608 trames**,
  un dixième de seconde de musique — et un fichier tronqué, c'est-à-dire la copie interrompue
  depuis le NAS, joue simplement plus court. Le `README` des fixtures de phase 0 affirmait
  l'inverse (« doit produire une erreur propre ») : la mesure l'a démenti, et personne ne
  pouvait le savoir avant d'essayer.
  **Le raisonnement de Thomas** : ce qui est en jeu n'est pas un désagrément, c'est **perdre
  de la musique sans le savoir**. Un testeur dont la bibliothèque NAS s'est mal recopiée
  écoutera des morceaux tronqués **en croyant écouter le master** — et ne signalera rien,
  puisqu'il n'y a rien à voir. C'est la suite de son raisonnement sur INV-21 : un défaut qui
  se voit se répare, un défaut invisible se propage. Et la règle ne coûte presque rien à
  tenir : vérifier l'intégrité à côté du décodage, c'est quelques lignes.
  **« Ça s'est décodé » ne prouvera jamais « le fichier est entier. »** La prémisse, elle, est
  déjà épinglée par les trois tests de `decodage.rs` — la section qui les porte cite la règle,
  et le parcours des citations couvre désormais `src-tauri/tests/`. Si une mise à jour de
  symphonia se mettait à signaler ces cas, ces tests échoueraient : cet échec serait la bonne
  nouvelle.

### INV-23 — Un libellé destiné à l'affichage ne vient jamais d'un `{:?}`
- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : test `format.rs` (cible `bibliotheque`) — table `FileType` → libellé, une assertion par fixture audio
- pourquoi : **règle née le 2026-11-23 de la passe de vérité**, c'est-à-dire du premier
  passage du code sur la vraie bibliothèque de Thomas. `library/metadonnees.rs` remplissait
  le champ `format` avec `format!("{:?}", fichier.file_type())` : le **`Debug`** d'un type de
  lofty, envoyé tel quel vers l'interface. Résultat sur ses 40 MP3 : `Mpeg`, là où la V1
  écrivait `MP3` et où le fichier dit `mp3`. Seul écart sur 182 où la V1 avait raison.
  Deux raisons de la retenir, et la seconde est la vraie. D'abord c'est une rupture de
  parité visible par l'utilisateur, et la parité est la cible (ADR 0004). Ensuite **la valeur
  change avec la version de la dépendance** : renommer un variant d'énumération chez lofty
  est un changement anodin de leur côté, et personne ici ne le verrait — aucun test ne porte
  sur une chaîne que rien n'épingle. C'est le mode de défaut d'INV-21 déplacé d'un cran :
  non pas un défaut invisible, mais un défaut dont la cause est hors du dépôt.
  Ce qu'une table explicite donne en plus de la correction : elle **épingle** la
  correspondance, donc une montée de version qui renomme un variant fait échouer la
  compilation ou le test au lieu de changer ce que Thomas lit à l'écran.
  Et la règle vaut au-delà du champ `format` : `{:?}` est fait pour un journal, pas pour un
  œil. Tout libellé que l'interface montre se décide ici, explicitement.

### INV-24 — Rien n'est créé dans le dossier de données avant que la migration ait rendu son verdict
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : test `echec_rattrapable.rs` (cible `demarrage`) — la séquence réelle de `run()`, rejouée en entier
- pourquoi : **règle née le 2026-11-23 d'une relecture, et mesurée.** `run()` ouvrait la
  bibliothèque **quoi qu'il arrive**, y compris juste après un échec de migration — et ouvrir
  la bibliothèque crée le dossier de données. Donc un seul fichier de pochette illisible, ou
  un disque momentanément plein, et le lancement suivant trouvait un dossier « déjà peuplé »
  par la base vide que V2 venait de créer lui-même : il renonçait **définitivement**. Les
  572 Mo du dossier de V1 restaient en place, invisibles, et la reprise n'ayant jamais lieu,
  les pistes supprimées revenaient au premier scan — **INV-06 violé à travers la migration,
  de façon permanente. **La bêta V1 n'ayant jamais été distribuée, la seule bibliothèque en jeu est celle de Thomas** — ce qui ne rend pas la perte moins définitive.
  **Ce qui rend la règle générale** : ce n'est pas « attention à l'ordre dans `run()` », c'est
  qu'un échec doit rester rattrapable, donc qu'on n'écrit rien tant qu'on ne sait pas. Ne pas
  démarrer est réparable — les données sont intactes, relancer réessaie ; démarrer ne l'est
  pas. L'ADR 0010 avait accepté l'inverse (« un échec n'empêche pas de démarrer ») sur un
  raisonnement juste et une propriété fausse ; l'ADR 0012 le remplace.
  **Et la raison pour laquelle une PR entière ne l'a pas vu** : le test censé prouver la
  propriété appelait la migration deux fois de suite, sans l'ouverture de la base que `run()`
  fait entre les deux. Il concluait « le lancement suivant peut retenter » au sujet d'un
  programme qui n'existe pas. D'où la forme de la couverture : la séquence est une fonction
  (`demarrage::preparer`) et le test l'appelle **en entier**. Un test qui saute une étape de
  la séquence réelle n'éprouve pas le produit. C'est la leçon fondatrice du projet — une
  phrase dans un document ne peut pas échouer — retrouvée dans le code le plus sensible qu'il
  contienne.
  **« Créé » et non « écrit », et la nuance est mesurée** : juger si une base existante est
  vierge demande de l'ouvrir, donc d'appliquer les migrations de schéma qui lui manquent —
  une écriture, avant la migration du dossier. La conséquence est nulle (une base non vierge
  arrête la migration de toute façon, et une base vierge est effacée juste après), mais
  l'énoncé devait coller au code plutôt que l'inverse. Relevé en relecture du 2026-11-23.

### INV-25 — La copie d'une migration est sur le disque avant que le témoin ne la certifie
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : architecture `migration-durable.test.ts` — l'ordre des quatre étapes irréversibles
- pourquoi : **règle née le 2026-11-25 de la relecture de la PR #8, et mesurée.** La migration
  copiait, vérifiait, échangeait, puis supprimait `noir/` — dans le bon ordre, et sans jamais
  rien forcer sur le disque. Or `std::fs::copy` et `std::fs::rename` rendent la main dès que le
  **cache du système** est à jour, pas le disque ; entre les deux il y a quelques secondes, sans
  borne garantie. Une extinction brutale dans cette fenêtre — panique noyau, coupure de courant,
  batterie à plat — laissait au redémarrage un `hean/` avec son témoin, des fichiers vides
  dedans, et plus de `noir/` du tout : **playlists, historique d'écoute et sources NAS perdus,
  et plus aucun code pour savoir qu'ils avaient existé.**
  **Ce qui rend la règle sérieuse n'est pas sa probabilité, c'est son exposition** : ce chemin
  ne s'exécute qu'une seule fois par testeur, juste après une mise à jour — c'est-à-dire
  précisément au moment où l'on redémarre sa machine — et il n'y a pas de deuxième chance,
  puisque l'original a disparu. C'est la même famille qu'INV-22 : un défaut qui ne se voit pas
  ne se signale pas.
  **⚠️ La gravité a changé le 2026-11-25, le jour même, et il faut le dire plutôt que de laisser
  le 🔴 se périmer** : Thomas a décidé que la migration ne supprime plus `noir/` (ADR 0014,
  INV-26). Les octets ne sont donc plus perdus — ils sont dans l'ancien dossier, toujours là. Ce
  qui reste, et qui justifie encore la règle : le témoin **certifierait une copie trouée**, donc
  l'application démarrerait sur une bibliothèque incomplète **sans jamais remigrer**, et son
  propriétaire n'aurait aucune raison de soupçonner que ses données entières sont à côté.
  Réparable à la main par quelqu'un qui sait lire un dossier `Application Support` ; invisible
  pour tous les autres. Tant que V2 n'a aucun utilisateur, la bibliothèque en jeu est celle de
  Thomas seul — ce qui change l'ampleur, pas la nature. À Thomas de dire si cela reste
  🔴 ou devient 🟡 — la règle, elle, ne bouge pas : elle coûte 90 ms.
  **Et le coût est mesuré, pour qu'il ne serve pas de prétexte** : 90 ms sur le dossier réel de
  Thomas (2 444 fichiers synchronisés — les 2 443 migrés plus le témoin, écrit juste avant —
  283 Mo, le 2026-11-25). Il est si bas parce que la copie est un **clone** APFS : il n'y a
  presque aucune page sale à pousser (voir `migration/durabilite.rs`, qui dit aussi ce que
  `sync_data` et `sync_all` font vraiment sur Apple — les deux sont `F_FULLFSYNC`). La règle ne porte donc pas sur « combien de
  fichiers synchroniser » — ça se négocie — mais sur l'**ordre** : durable, puis irréversible.
  D'où la forme de la couverture, un test de forme : la propriété ne s'observe qu'en coupant le
  courant au bon millième de seconde, mais l'ordre des appels, lui, se lit. (Gravité 🔴 par
  défaut, comme INV-24, en attendant que Thomas la classe — c'est une perte de données.)

### INV-26 — La migration ne supprime pas le dossier de V1, sauf son cache de téléchargement
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : architecture `migration-durable.test.ts` — l'appel de suppression ne doit pas reparaître
- pourquoi : **règle née le 2026-11-25 d'une décision de Thomas, dans ses mots : « Oui,
  garde-le ».** Question posée : « Je fais garder ton dossier V1 au lieu de le supprimer ? » Il a
  choisi le filet de retour arrière vers la V1. L'ADR 0014 l'acte et remplace sur ce point
  l'ADR 0010, dont la suppression était le quatrième temps.
  **Une seule exception, nommée, et elle vient de lui aussi** : « Je fais supprimer le cache de
  289 Mo en migrant, et garder le reste ? » → **« Oui, jette le cache »**. Donc `noir/smb_buffer/`
  est supprimé après un échange réussi et vérifié, et **lui seul**. Le périmètre est écrit dans
  `migration/cache_v1.rs`, avec la raison : trois autres fichiers de `noir/` portent le mot
  *cache* (`tracks_cache.json`, `added_dates_cache.json`, `network_scan_cache.json`) et ne sont
  pas jetables — c'est l'état de la bibliothèque V1, dont les 3 443 pistes de Thomas. Les
  supprimer viderait le filet qu'on vient de décider de garder, pour 4 Mo. Une suppression chez
  quelqu'un ne s'étend pas par déduction : ce qui n'a pas été arbitré n'est pas couvert.
  **Ce n'est pas qu'une préférence : la prémisse de l'ADR 0010 était fausse.** Il écartait
  « garder l'original au cas où » parce que ce serait « 283 Mo dupliqués sur le disque ». Mesuré
  le 2026-11-25 : sur APFS, `std::fs::copy` **clone** — copier 400 Mo ne consomme aucun espace —
  donc les fichiers migrés partagent leurs blocs avec `hean/`. Ce que `noir/` occupe encore en
  propre est ce qui n'a PAS été migré, c'est-à-dire son cache SMB : **289 Mio en trois fichiers,
  sur 572 Mio apparents**. La décision a donc été prise sur un chiffre juste.
  **Pourquoi un test et pas une ligne de documentation.** La suppression existait, elle rendait
  la main plus propre, et un « nettoyage » de phase 3 la remettrait sans que personne ne la
  relise comme une décision — c'est la forme même de régression que ce projet existe pour
  fermer. Deux mécanismes, et c'est voulu : un test de forme refuse que l'appel reparaisse
  (`migration-durable.test.ts`), et un test de comportement rejoue la séquence de démarrage
  **deux fois** pour établir ce que garder l'original ne casse pas — la source survit intacte, et
  le témoin seul empêche de remigrer 283 Mo à chaque lancement
  (`src-tauri/tests/demarrage/retour_arriere.rs`, plus l'idempotence dans `migration.rs`).
  **Un échec de la suppression du cache n'est pas fatal** : un cache qui résiste coûte 289 Mio,
  pas une donnée, et faire échouer une migration pour de la place disque serait l'inverse de ce
  que l'ADR 0012 a établi. Avertissement au journal, migration réussie (test dans
  `migration.rs`, qui provoque la panne en retirant le droit d'écrire sur `noir/`).
  **Et le filet doit se savoir** : une ligne de journal dit où les données d'origine sont
  restées. Un retour arrière dont personne ne connaît l'existence n'en est pas un — c'est le
  raisonnement d'INV-21 appliqué à une bonne nouvelle. S'il faut de l'interface pour le dire,
  c'est une question ouverte, pas un acquis.

### INV-27 — Un scan ne conclut la disparition d'une piste que dans son périmètre
- pile : test
- gravité : 🔴 BLOQUANT
- couverture : test perimetre.rs
- pourquoi : `Depot::appliquer` efface toute piste connue qu'il n'a pas revue — c'est ce qui
  fait disparaître de la bibliothèque un fichier que l'utilisateur a supprimé. Avec deux
  provenances dans la même base, le premier scan de disque aurait donc effacé les **3 137
  pistes réseau** de Thomas (91 % de sa bibliothèque), et le premier scan réseau ses 305
  pistes locales. En V1 les deux scans écrivaient dans le même cache global et la règle était
  « n'oublie pas de filtrer » : la forme de règle qu'INV-06 existe pour ne plus écrire.
  **La règle est un type, pas une phrase.** `scan::Perimetre` n'a **aucune variante « tout »** :
  le dépôt ne peut pas oublier de restreindre, puisqu'il n'a pas le choix de ne pas le faire.
  Le périmètre d'une source est le **préfixe** de ses URI, slash final compris — sans ce slash,
  une source dont l'identifiant commence comme celui d'une autre effacerait les pistes de sa
  voisine, et le test l'éprouve avec `dcdd24af` contre `dcdd24af-bis`.
  **Et le volet qui a demandé le plus de travail est le second**, trouvé en écrivant le test et
  non en relisant le code : un dossier que le serveur **refuse de lister** n'est pas un dossier
  vide. Le parcours le journalisait et rien n'en tenait compte, donc un partage momentanément
  inaccessible — ou une racine de disque éjectée, et Thomas n'en a qu'une — faisait **effacer
  ses pistes**. C'est exactement ce que B17 dit depuis V1 (« un dossier absent ne doit pas faire
  perdre la bibliothèque entière »), et la conséquence n'en avait jamais été tirée. D'où
  `scan::Conclusion` : un parcours qui a sauté quelque chose rend `Partielle`, et rien ne
  disparaît ce tour-là. Le seul coût est de voir une suppression au scan **suivant**.
  Né le 2026-11-25 en branchant la part réseau (ADR 0016 §2). Les deux volets sont éprouvés
  dans la même base, avec les deux provenances mélangées : un test qui n'en mettrait qu'une
  passerait au vert avec ou sans la règle.

### INV-28 — Un chiffre montré à l'utilisateur dit sur place s'il est mesuré ou estimé
- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : architecture `chiffres-annonces.test.ts`
- pourquoi : **règle de Thomas, née le 2026-11-26 de trois cas en une soirée.** L'interface
  annonçait « environ deux minutes » pour le premier scan réseau ; le vrai a duré
  **36 minutes** ([ADR 0016](../../../docs/adr/0016-la-part-reseau-de-la-bibliotheque.md) §3),
  et l'estimation était fausse d'un facteur 17 parce qu'elle comptait le débit là où le coût
  est la latence. Le même soir, l'[ADR 0015](../../../docs/adr/0015-nom-de-service-keychain-pour-les-mots-de-passe-smb.md)
  chiffrait le coût de l'option B à « une saisie de mot de passe », **deux fois de suite**,
  avant qu'un relevé du trousseau ne le mesure à **zéro**.
  Ce que les trois cas ont en commun n'est pas l'erreur — un chiffre faux se corrige — mais
  qu'**en lisant, on ne pouvait pas savoir que le chiffre n'avait jamais été mesuré**. Un
  chiffre dont l'origine est muette se recopie, et il se recopie avec autorité : « environ
  deux minutes » est passé du calcul de l'ADR au commentaire du composant puis au texte
  affiché, sans que personne mente à aucune étape.
  🟡 et non 🔴 : rien n'est perdu, rien n'est illisible. Mais le dommage est réel et il porte
  sur la confiance — une fenêtre qui ne bouge pas pendant une demi-heure quand on a promis
  deux minutes **est** une panne, pour qui la regarde. C'est le raisonnement d'INV-21 déplacé
  vers l'utilisateur : un défaut qui ne se signale pas coûte plus que lui-même.
  **La fenêtre est la ligne**, et le détecteur se contourne en écrivant « mesuré » sans avoir
  mesuré : les deux choix sont argumentés en tête du test, avec les formes essayées avant.
  Corollaire pour les documents, qui n'a pas de détecteur et n'en aura pas : un nombre qu'on
  n'a pas mesuré s'écrit **estimé**, avec la méthode d'estimation — c'est elle qui a permis de
  voir, après coup, que les 41 ms par fichier venaient d'un calcul de débit.

---

### INV-30 — Un contrôle dont la valeur est sondée ne se relit pas pendant qu'on le manipule

- pile : test
- gravité : 🟡 AVERTISSEMENT
- couverture : planifiée (phase 4) — **et la matière existe déjà**, les deux curseurs de
  `Lecture.svelte` : ce qui manque est un détecteur qui distingue « valeur sondée » de
  « valeur qu'on vient de poser », et c'est une question de conception, pas d'absence de
  corpus. Dire « faute de matière » ici serait la faute d'INV-04 du 2026-11-25
- pourquoi : né le 2026-11-27, sur la **première épreuve du moteur de son par Thomas**. La
  barre de position liait son `value` à l'état de lecture, qui est **sondé** et non poussé :
  chaque relevé réécrivait la valeur du curseur sous le doigt, le pouce poussait, le sondage
  ramenait. Le déplacement n'était pas lent ni imprécis — **il était impossible à viser**, et
  l'étape 5 du protocole a échoué sur le matériel de Thomas alors que le moteur, lui, faisait
  son travail : le saut était calculé juste et appliqué juste. Le défaut vivait entièrement
  dans l'interface.
  **Ce qui le rendait invisible à la relecture est la vraie leçon**, et c'est elle qui vaut
  une règle : le curseur de volume, dix lignes plus bas, a **exactement la même forme** et
  fonctionne. Il fonctionne parce que la valeur qu'il relit est celle qu'on vient d'y poser —
  elle ne bouge pas d'elle-même. Un relecteur qui compare les deux conclut que la forme est
  bonne. **Un deuxième contrôle de la même forme qui marche ne prouve rien sur le premier** :
  ce qui les sépare n'est pas leur code, c'est de savoir si la valeur relue évolue sans
  qu'on y touche. La question à se poser devant un contrôle lié à un état sondé est donc
  celle-là, et pas « est-ce que ça ressemble à ce qui marche ».
  🟡 et non 🔴 : rien n'est perdu et l'app reste utilisable. Mais le coût réel est le même
  qu'INV-28 et qu'INV-21 — une fonction qui a l'air présente et ne l'est pas se signale
  moins qu'une panne franche, et c'est Thomas qui l'a payée, à la main, en premier.
  Correctif : pendant le déplacement c'est le doigt qui commande, et le sondage reprend la
  main à l'échéance qui suit le relâchement.

---

