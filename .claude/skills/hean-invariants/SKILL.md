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
| **test** | La règle est (ou sera) une vérification exécutable. Elle a quitté les têtes. | 14 |
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
- couverture : test `bibliotheque.rs` — écriture lofty, puis relecture du fichier
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
  `build.rs` est hors scope : ses `println!("cargo::…")` sont le protocole de Cargo.

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
[`docs/adr/`](../../../docs/adr/). Les sept ADR en vigueur : réécrire de zéro (0001), la
stack (0002), le dossier de données (0003), la cible de parité visuelle (0004), l'entretien
du harnais et l'orchestration (0005), le socle de phase 0 (0006), le format interne audio et
la preuve du bit-perfect (0007). **Un ADR ne se modifie pas : on en écrit un nouveau.**

Rappel des interdits qui en découlent et qui ne se rouvrent pas : **jamais CPAL** ·
**bit-perfect = pas de resampling** · **macOS 11+ seulement, hors Mac App Store** ·
**Svelte 5 en runes, jamais les stores Svelte 4** · **aucune dépendance npm de production
sans ADR**.

## Mémoire de l'agent de relecture

> La règle de zone « audio **et** UI dans le même diff → AVERTISSEMENT » ne vise que les
> chemins **Rust** de l'audio. Les fichiers frontend du player sont de l'UI malgré leur nom.
> *(Faux positif rencontré lors de la relecture du correctif shuffle, 2026-10-29.)*
