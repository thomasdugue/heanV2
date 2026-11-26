---
name: hean-invariants
description: Règles « ne JAMAIS casser » de heanV2 — les pièges de plateforme qu'aucun test ne prouvera, et les règles de travail sur le code (zones, 400 lignes, échappement, performance). Charger AVANT toute modif de code et avant toute review de diff. Le catalogue des 28 fiches est dans CATALOGUE.md, à n'ouvrir que pour ajouter ou modifier une règle.
---

# hean-invariants — ce qui ne doit jamais casser (V2)

> **Ce fichier et [`CATALOGUE.md`](CATALOGUE.md) sont lus par une machine.**
> `tools/check-invariants.mjs` en extrait chaque `### INV-NN` avec ses quatre champs et
> vérifie qu'aucune règle n'est orpheline et qu'aucun test ne cite une règle disparue.
> Garder le format exactement tel quel.

## Ce que tu as sous les yeux, et ce qui n'y est pas

**Règle du partage : ce qu'un test applique ne s'écrit pas pour être lu.**

Les 28 fiches d'invariants vivent dans [`CATALOGUE.md`](CATALOGUE.md). Tu n'as pas à les lire
pour écrire du code : elles sont tenues par des tests, et `./check` t'arrêtera si tu en casses
une. Ouvre le catalogue pour **ajouter une règle, en modifier une, ou comprendre un refus**.

Ce qui est resté ici est ce qu'aucune vérification ne rattrapera jamais : les **pièges de
plateforme** (CoreAudio, WebKit, dyld, Gatekeeper) et les **règles de travail sur le code**.
Ce sont les seules choses que tu dois tenir dans ta tête.

Vue générée et à jour : [`docs/generated/invariants.md`](../../../docs/generated/invariants.md).

## Le tri, et pourquoi il compte

V1 avait **20 invariants** en prose (ses propres documents en annonçaient 19 : rien ne
vérifiait ce nombre). **3 sur 20 étaient couverts par un test.** Les 17 autres reposaient sur
la vigilance de quelqu'un.

L'erreur de V1 n'était pas de découvrir ces règles après coup — chacune a été payée par une
régression réelle, et c'est le legs le plus précieux du projet. L'erreur était de les écrire
en **prose** : une phrase dans un document ne peut pas échouer. Un test, si.

| Pile | Ce que ça veut dire | Combien |
|---|---|---|
| **rayé** | L'architecture de V2 rend la faute inexprimable. Plus de règle à retenir. | 6 |
| **test** | La règle est (ou sera) une vérification exécutable. Elle a quitté les têtes. | 20 |
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
- **Avant d'appeler un comportement de V1 un défaut : lire la chaîne entière.** Le mécanisme
  est en Rust, mais **l'honnêteté du mécanisme est souvent en JS**. Citer `fichier:ligne` des
  deux côtés — `src-tauri/src/` **et** `noir-tauri/src/*.js`. Sans ces deux références, ce
  n'est pas un constat, c'est une impression : ne pas l'écrire dans un brief, et surtout ne
  demander à personne de « corriger ». Payé le 2026-11-26 : le repli de fréquence du DAC,
  déclaré « altération silencieuse » après lecture du seul Rust, alors que le lecteur affiche
  fréquence entrante et sortante depuis toujours (ADR 0018). Corollaire demandé par Thomas :
  **quand il décrit un comportement de V1, le vérifier comme n'importe quelle affirmation et
  lui dire ce qui diffère** — il décrit de mémoire un produit de 30 000 lignes. C'est la
  condition de « porter d'abord, améliorer sur un défaut mesuré » : sans mesure des deux
  bouts, « mesuré » ne veut rien dire.

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
- **Un détecteur de mots doit parler la langue du code qu'il lit.** Quatrième variante, et la
  plus bête — donc celle qui durera le plus longtemps si elle n'est pas écrite. Le détecteur
  d'INV-04 (🔴) ne retenait une ligne que si elle portait `password|keychain|credential|secret` :
  **que de l'anglais**, dans un projet dont tout le code est en français. Le jour où le vrai
  module du trousseau est arrivé — `network/identifiants.rs`, `mot_de_passe()`,
  `lire_du_trousseau()` — **aucun de ses appels n'était vu**, et la faute de V1 réécrite en
  français (`identifiants::mot_de_passe(&source.id).unwrap_or_default()`) passait au vert.
  Pire : l'assertion « il voit la matière réelle » ajoutée le même jour **passait aussi**, parce
  qu'elle injectait la faute sur `security_framework::passwords::get_generic_password` — un nom
  d'API tierce, donc anglais. Elle prouvait que le détecteur voit la bibliothèque, pas qu'il voit
  **les appelants**, qui sont le site réaliste de la faute. Deux conséquences écrites : la liste
  de mots porte les deux langues, et **une injection se fait chez l'appelant**, pas à la
  frontière d'une dépendance. Relevé par `invariant-guardian` le 2026-11-25.
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
- **Un détecteur qui argumente sa fenêtre doit interdire, par une assertion, la fenêtre qu'il a
  écartée.** 🟡 Cinquième variante, et la première qui porte sur un **réglage** du détecteur
  plutôt que sur son corpus ou son vocabulaire. La fenêtre est la partie qu'on élargit **de
  bonne foi** — « mais le commentaire juste au-dessus dit mesuré » — et aucun auto-test ne peut
  la voir, puisqu'ils tiennent tous sur une seule ligne de texte. Mesuré le 2026-11-26 sur
  INV-28, dont les lignes d'argumentation (« la fenêtre est la ligne ») n'étaient tenues par
  rien : l'élargir au bloc ou à ±2 lignes laissait les quatre assertions **vertes**, et la
  règle retombait dans l'état exact qui l'avait fait naître. Deux lignes d'assertion ont suffi.
  Relevé par `invariant-guardian`, qui a rejoué les mutations plutôt que de me croire.
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
**jamais de rééchantillonnage par confort, par optimisation ou par défaut de conception — le
repli de fréquence est permis quand le périphérique ne sait pas faire autrement, et il
s'affiche** (ADR 0018 ; la formule « bit-perfect = pas de resampling » était un interdit sans
source, et l'ADR 0002 ne dit pas ça) · **macOS 11+ seulement, hors Mac App Store** ·
**Svelte 5 en runes, jamais les stores Svelte 4** · **aucune dépendance npm de production
sans ADR**.

