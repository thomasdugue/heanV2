---
name: hean-invariants
description: Règles « ne JAMAIS casser » de heanV2, triées en trois piles (rayées par construction / devenues des tests / irréductibles). Charger AVANT toute modif de code et avant toute review de diff. Contient la gravité de chaque règle, le pourquoi (la régression qu'elle a coûtée en V1), et la couverture mécanique de chacune.
---

# hean-invariants — ce qui ne doit jamais casser (V2)

> **Ce fichier est lu par une machine.** `tools/check-invariants.mjs` en extrait chaque
> `### INV-NN` avec ses quatre champs et vérifie qu'aucune règle n'est orpheline (sans test)
> et qu'aucun test ne cite une règle disparue. Garder le format exactement tel quel.

## Le tri, et pourquoi il compte

V1 avait **20 invariants** en prose (ses propres documents en annoncent 19 — le compte
datait d'avant l'ajout de la règle CSS le 2026-10-29, et rien ne vérifiait ce nombre : c'est
la dérive de documentation du postmortem, en miniature). **3 sur 20 étaient couverts par un
test.** Les 17 autres reposaient sur la vigilance de quelqu'un.

L'erreur de V1 n'était pas de découvrir ces règles après coup — chacune a été payée par une
régression réelle, et c'est le legs le plus précieux du projet. L'erreur était de les écrire
en **prose** : une phrase dans un document ne peut pas échouer. Un test, si.

D'où les trois piles :

| Pile | Ce que ça veut dire | Combien |
|---|---|---|
| **rayé** | L'architecture de V2 rend la faute inexprimable. Plus de règle à retenir. | 6 |
| **test** | La règle est (ou sera) une vérification exécutable. Elle a quitté les têtes. | 17 |
| **irréductible** | Comportement de CoreAudio, de WebKit ou de Gatekeeper. Aucun test ne le prouve. | 2 + 7 pièges |

**La mesure du succès : la pile « irréductible » ne grossit pas, et la pile « test » se
remplit.** Une règle `couverture : planifiée` est une dette visible, pas un oubli.

### Champs obligatoires

- `pile` — `rayé`, `test` ou `irréductible`.
- `gravité` — 🔴 BLOQUANT (perte de données, app inutilisable, risque légal) ou
  🟡 AVERTISSEMENT (défaut visuel ou gênant, réparable). **Une règle nouvelle sans gravité
  est BLOQUANTE par défaut** jusqu'à ce que Thomas la classe.
- `couverture` — `sans objet` (pile rayé), `test <nom>`, `architecture <nom>`,
  `planifiée (phase N)` ou `manuelle`.
- `pourquoi` — la régression que la règle a coûtée. **Sans le pourquoi, la règle finit
  supprimée par quelqu'un qui la trouve arbitraire.**

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
  release partait alors sans Sentry et sans feedback, sans aucune erreur — invisible jusque
  chez les testeurs. V1 s'en protégeait avec un wrapper `release.sh` qu'il suffisait
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
- couverture : planifiée (phase 5) — conteneur Samba + proxy à injection de panne
- pourquoi : `read_file_to_temp_progressive()` doit retenter avec une connexion fraîche si
  `open_with()` échoue. En V1 les tests `network_source` étaient `#[ignore]` : la règle ne
  se vérifiait qu'avec un vrai NAS, dans le bon état.

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
  se passe chez les testeurs. Un défaut visuel se voit et se répare ; être aveugle ne se voit
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
- couverture : test `demarrage.rs` — la séquence réelle de `run()`, rejouée en entier
- pourquoi : **règle née le 2026-11-23 d'une relecture, et mesurée.** `run()` ouvrait la
  bibliothèque **quoi qu'il arrive**, y compris juste après un échec de migration — et ouvrir
  la bibliothèque crée le dossier de données. Donc un seul fichier de pochette illisible, ou
  un disque momentanément plein, et le lancement suivant trouvait un dossier « déjà peuplé »
  par la base vide que V2 venait de créer lui-même : il renonçait **définitivement**. Les
  572 Mo du dossier de V1 restaient en place, invisibles, et la reprise n'ayant jamais lieu,
  les pistes supprimées revenaient au premier scan — **INV-06 violé à travers la migration,
  de façon permanente, chez ~50 testeurs**.
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

### INV-25 — La copie d'une migration est forcée sur le disque avant que l'original soit supprimé
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
  **Et le coût est mesuré, pour qu'il ne serve pas de prétexte** : 90 ms sur le dossier réel de
  Thomas (2 444 fichiers, 283 Mo, le 2026-11-25). La règle ne porte donc pas sur « combien de
  fichiers synchroniser » — ça se négocie — mais sur l'**ordre** : durable, puis irréversible.
  D'où la forme de la couverture, un test de forme : la propriété ne s'observe qu'en coupant le
  courant au bon millième de seconde, mais l'ordre des appels, lui, se lit. (Gravité 🔴 par
  défaut, comme INV-24, en attendant que Thomas la classe — c'est une perte de données.)

---

## Pile 3 — irréductibles

Ces règles décrivent le comportement de CoreAudio, de WebKit, de dyld et de Gatekeeper, pas
celui de notre code. Aucun test ne les prouve : elles restent écrites, avec leur gravité.
C'est **le legs le plus précieux de V1** — sept pièges qui ne se déduisent d'aucune
documentation Apple et qui coûteraient les mêmes semaines à redécouvrir.

### INV-13 — Mosaïque 28×28 : positionnement absolu, jamais CSS Grid
- pile : irréductible
- gravité : 🟡 AVERTISSEMENT
- couverture : manuelle — piège PLAT-5, détecté a posteriori par la régression visuelle
- pourquoi : bug WebKit. Dans une grille de 28×28 px, une cellule vide résout `height: 100%`
  contre le conteneur et non contre son grid area. **Corrigé 4 fois** avant d'abandonner
  Grid pour `calc(50% - 0.5px)`. La même grille en 96×96 ne pose aucun problème.

### INV-18 — dylibbundler doit être suivi du fix des LC_RPATH dupliqués
- pile : irréductible
- gravité : 🔴 BLOQUANT
- couverture : manuelle — piège PLAT-6, vérifiable dans le pipeline de release (phase 6)
- pourquoi : `dylibbundler` ajoute un `LC_RPATH` à chaque dylib, mais ~66 des 92 dylibs Samba
  de Homebrew en avaient déjà un → doublons → **dyld refuse de charger depuis Big Sur**.
  Sans la passe `install_name_tool -delete_rpath` puis re-signature, l'app crash au lancement
  sur tout Mac sans Homebrew, donc chez tous les testeurs.

### Les cinq autres pièges de plateforme

Ils n'ont jamais eu de numéro d'invariant parce qu'ils vivaient dans les « décisions
techniques actées ». Ils sont de la même nature, et ils restent valables en V2.

- **PLAT-1 — AirPlay n'existe dans CoreAudio que tant qu'il est le défaut système.** Dès
  qu'il perd ce statut, macOS tue la session et le device disparaît du HAL.
  `set_system_default_device` sur un device stale **accepte silencieusement sans rien
  faire**. D'où la stratégie de préservation : quand on quitte AirPlay, on ne change pas le
  défaut système, l'audio est routé par assignement AudioUnit explicite. 🔴 BLOQUANT.
- **PLAT-2 — `libsmbclient` est un singleton au niveau du process.** Deux `SmbClient`
  concurrents → EINVAL. Ce n'est pas un choix d'architecture, c'est une contrainte de la
  bibliothèque C : tout passe par un mutex unique. 🔴 BLOQUANT.
- **PLAT-3 — Les media keys ne s'attrapent qu'avec `MPRemoteCommandCenter`.** Apple Music
  intercepte F7/F8/F9 en priorité sur les global shortcuts Tauri. Ne jamais les réajouter.
  🟡 AVERTISSEMENT.
- **PLAT-4 — Le drag & drop HTML5 ne fonctionne pas dans la WebView.** Implémentation
  manuelle mousedown/mousemove/mouseup obligatoire. 🟡 AVERTISSEMENT.
- **PLAT-7 — Tauri notarise l'app mais pas le DMG.** Sans `xcrun notarytool submit` explicite
  sur le DMG, Gatekeeper le refuse même si l'app à l'intérieur est notarisée. Notariser
  **et** stapler les deux. 🔴 BLOQUANT.

(PLAT-5 = INV-13, PLAT-6 = INV-18.)

---

## Règles de travail sur le code

Reprises de V1, elles ont fait leurs preuves. Elles étaient dans `CLAUDE.md` ; elles sont
ici depuis le 2026-11-22, parce que ce skill est chargé avant toute modif de code — donc
exactement quand elles servent — et que `CLAUDE.md` doit rester lisible en entier.

- **Ne jamais modifier l'audio quand on travaille sur l'UI ou la bibliothèque. Et
  inversement.** La règle de zone ne vise que les chemins **Rust** de l'audio ; les fichiers
  frontend du player sont de l'UI malgré leur nom.
- **Aucun fichier ne dépasse ~400 lignes.** Si ça arrive, c'est que le découpage est à
  revoir. `./check` le refuse. (V1 : `lib.rs` 4 870 lignes, `views.js` 3 512,
  `styles.css` 10 654.)
- **Toute donnée affichée passe par un échappement**, guillemets compris. C'est le
  comportement W006 : un tag contenant `"` était tronqué à la sauvegarde.

### Éprouver un détecteur plutôt que le supposer

Un test de forme sur un dossier encore vide passe au vert sans rien garantir : il produit de
la confiance sans en mériter. Tout détecteur se prouve donc en lui donnant la faute qu'il est
censé voir.

- **Les tests d'architecture embarquent leur auto-test** : chacun lance son détecteur sur un
  échantillon fautif écrit à côté. C'est la forme à préférer — elle tourne à chaque `./check`.
- **Les vérifications en script** (`tools/check-*.sh|mjs`) n'ont pas cette forme : on les
  éprouve **à la main, par injection**, avant de les committer. `check-adr.mjs`,
  `check-comptes.mjs` et les règles CSS/`println!` l'ont toutes été.
- **Un comportement porté entre dans le verdict.** Les 24 comportements de
  `UX-ANTI-REGRESSION.md` sont rouges par construction tant que la feature n'existe pas :
  c'est le tableau de bord, et `./check` ne le compte pas dans son verdict. Mais dès qu'un
  comportement est **porté**, la même indulgence devient un angle mort — il pourrait
  régresser sans faire rougir `./check`, et le tableau de bord relabelliserait la régression
  en dette (« il en reste 19 à porter »). Tenu par `tools/check-comportements.mjs` : un
  comportement porté doit passer, un comportement à porter doit échouer. Né le 2026-11-22,
  relevé par `invariant-guardian` à la relecture de la phase 2 — la phase qui venait d'ouvrir
  le trou en portant les six premiers. Rattraper ça par le protocole de session serait de la
  discipline, ce que l'ADR 0005 refuse par principe.
- **Et un test porté doit contenir une assertion.** Le corollaire, sans quoi la règle
  précédente se contourne sans le vouloir : `it('V005 — …', () => {})` serait compté porté
  **et** vert. Tenu par `contrat-complet.test.ts`.
- **Un auto-test ne remplace pas la matière réelle.** Un auto-test prouve que le détecteur voit
  la faute **qu'on lui a inventée**. Il ne prouve pas qu'il voit le code **réel** : les deux
  assertions ne se remplacent pas. Payé sur INV-06, dont le détecteur cherchait `\bTrack\b` là où
  specta émet `Track_Serialize[]` — zéro commande attrapée sur huit, règle 🔴 désarmée dès
  l'arrivée de la première matière réelle, **auto-test vert pendant tout ce temps**. Un détecteur
  qui dérive sa matière d'un **artefact généré** (`src/ipc/bindings.ts`, `tokens.css`,
  `docs/generated/`) est le plus exposé : le format change sans que personne décide de le changer.
  D'où, depuis le 2026-11-23, l'assertion « il trouve au moins N choses réelles » là où la matière
  est légitime — et sa qualité principale est de **se périmer d'elle-même** le jour où cette
  matière disparaît. Mesuré : casser un détecteur partagé laisse ses interdictions **vertes** et ne
  fait rougir que cette assertion-là.
- **Un corpus vide est une erreur, par mécanisme.** Deuxième question, celle d'avant : *est-ce que
  je regarde quelque chose ?* Un détecteur pointé sur un dossier renommé ne trouve rien et passe au
  vert. La garde vit dans le lecteur partagé (`tests/architecture/_lecture.ts`), pas dans chaque
  test, donc un détecteur écrit demain en hérite sans que personne y pense. Un vide légitime —
  la zone n'existe pas encore — s'écrit `{ videPrevu: '<raison, et quelle phase le remplira>' }`,
  et une raison trop courte est refusée.
  **Un silence expliqué n'est pas un angle mort ; un silence non écrit en est un.** Même doctrine
  pour un détecteur muet faute de matière (INV-04 avant la phase 5, Svelte 4 avant la phase 3) :
  le commentaire dit pourquoi il est muet et quelle phase le rendra bavard.
- **La liste de ce qu'on surveille oublie ; la liste de ce qu'on exclut couvre d'avance.** Troisième
  variante, et c'est le **sens de l'échec** qui compte. `check-sizes.sh` énumérait les extensions à
  mesurer (`.ts .rs .css …`) : il ne lisait donc **ni shell ni Python**, et un rapport Python de
  **569 lignes est passé vert** — le premier fichier du dépôt à franchir la limite, dans une langue
  que le détecteur ne lisait pas. Inversé le 2026-11-23 : on mesure **tout, sauf une liste
  d'exceptions écrite** (binaires, tables de données). Un binaire nouveau fait alors rougir et
  quelqu'un écrit l'exception ; un langage nouveau ne passe plus en silence, ce qui ne réveillait
  personne. Même traitement pour `aucun-chemin-machine.test.ts`, à qui manquaient `.py`, `.json` et
  `.txt` : il lit désormais **tout le suivi, moins les binaires**, qui sont écrits avec leur
  raison. Les deux détecteurs appliquent donc le même principe — et c'est voulu : garder une
  liste d'un côté et l'inverser de l'autre donnerait une incohérence que quelqu'un « simplifierait »
  plus tard en la trouvant arbitraire. **La liste par énumération est ce qui a créé les trois trous**
  (shell, Python, et `tools/git-hooks/pre-commit`, qu'aucune recherche par extension ne pouvait
  attraper, faute d'en avoir une).
- **Un corpus dérivé de `git ls-files` est identique en local et en CI, par construction.** C'est le
  choix de `check-adr.mjs`, repris le 2026-11-23 par `check-sizes.sh` et par l'assertion de
  couverture ci-dessus. Un corpus lu sur le disque embarque ce qui traîne — un `.DS_Store`, un
  fichier de travail — donc il peut rougir chez Thomas et passer en CI : le défaut du 2026-11-22,
  vu de l'autre bout. Éprouvé : un fichier de 999 lignes non suivi par git est bien ignoré.
- ⚠️ **Nettoyer l'injection : arbre propre avant, ou copie.** `git checkout <fichier>` pour
  retirer une faute injectée **efface aussi le travail non commité du même fichier**. Payé le
  2026-11-22 : deux modifications perdues, repérées seulement parce qu'un renvoi s'était remis
  à dire l'ancien numéro.

### Performance — détail dans [`docs/PERFORMANCE.md`](../../../docs/PERFORMANCE.md)

La promesse du produit est une app légère et rapide. Trois règles la protègent ; les deux
premières sont des pièges de Svelte 5, la troisième est le seul risque réel.

1. **La bibliothèque n'est jamais réactive.** `$state.raw` (ou hors de tout état réactif)
   pour la liste de pistes. La réactivité ne couvre que l'état d'UI : vue, tri, sélection,
   filtre. Envelopper 10 000 pistes dans des signaux multiplie la mémoire et le coût de
   chaque mutation.
2. **Le virtual scroll reste explicite** : pool fixe de 60 nœuds recyclés, item à 48 px,
   délégation d'événements. C'est ce qui tient 60 fps sur 10 000+ pistes en V1 ; aucun
   framework ne fera mieux en rendant 10 000 lignes.
3. **Zéro dépendance npm de production sans ADR** — ce qu'elle apporte, son poids mesuré,
   pourquoi on ne l'écrit pas à la main. V1 en avait zéro ; `./check` applique un budget.

## Décisions techniques actées

Elles ne sont pas des invariants : ce sont des choix, et leur place est dans
[`docs/adr/`](../../../docs/adr/) — dont l'[index](../../../docs/adr/README.md) est la liste à
jour, vérifiée par `tools/check-adr.mjs` (aucune décision acceptée mais invisible). **Cette
phrase portait la liste à la main, et elle annonçait « les sept ADR en vigueur » en en citant
sept alors qu'il y en avait neuf** — 0008 et 0009 manquaient. Un nombre écrit à la main que
personne ne recompte est un mensonge en attente (ADR 0005) : la liste est donc remplacée par
le renvoi vers l'index, qui est tenu par une vérification. **Un ADR ne se modifie pas : on en
écrit un nouveau.**

Rappel des interdits qui en découlent et qui ne se rouvrent pas : **jamais CPAL** ·
**bit-perfect = pas de resampling** · **macOS 11+ seulement, hors Mac App Store** ·
**Svelte 5 en runes, jamais les stores Svelte 4** · **aucune dépendance npm de production
sans ADR**.

## Mémoire de l'agent de relecture

> La règle de zone « audio **et** UI dans le même diff → AVERTISSEMENT » ne vise que les
> chemins **Rust** de l'audio. Les fichiers frontend du player sont de l'UI malgré leur nom.
> *(Faux positif rencontré lors de la relecture du correctif shuffle, 2026-10-29.)*
