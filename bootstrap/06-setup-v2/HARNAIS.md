# Le harnais heanV2 — spécification

> Ce qui transforme « un modèle qui code bien » en « un projet qui ne régresse pas ».
>
> Écrit le 2026-11-21. Les 3 couches du socle (TypeScript strict, IPC typée, CI) sont dans
> [`PLAN-DE-RECONSTRUCTION.md`](PLAN-DE-RECONSTRUCTION.md), phase 0. Ce document décrit **ce qu'il
> faut y ajouter** pour atteindre les quatre objectifs : qualité, moins de régressions par itération,
> évolutivité, robustesse pour ajouter des features.

## Le principe

Un harnais répond à trois questions, et une seule des trois est habituellement traitée :

1. **« Est-ce que ça compile ? »** → types, lint. V1 n'avait rien (`node --check`).
2. **« Est-ce que ça marche encore ? »** → tests. V1 en avait, mais 3 suites skippées et
   **3 invariants sur 19 couverts**.
3. **« Est-ce que la règle est encore respectée *partout*, y compris dans le code écrit demain ? »**
   → c'est celle qui manque presque toujours, et c'est elle qui fait la différence entre un projet
   qui tient 6 mois et un projet qui tient 3 ans.

La question 3 ne se traite pas avec des tests unitaires. Elle se traite avec des **tests
d'architecture**, des **property tests**, des **invariants runtime** et de la **génération**. C'est
l'essentiel de ce qui suit.

Deuxième principe, spécifique à un projet mené avec un modèle : **tout ce qui n'est pas vérifié par
une machine dérive.** Pas par négligence — parce que chaque session repart d'un contexte reconstitué.
V1 le démontre : 11 affirmations de la documentation étaient fausses au moment de l'audit, et le
meilleur document design du projet vivait hors du dépôt. Un modèle plus abouti lit mieux cette
documentation ; il ne la rend pas vraie.

---

## Les 12 ajouts, par rendement

### 1. Rendre l'audio testable — fixtures + backend « fake » 🔴 priorité absolue

**Le problème V1** : zéro test sur la chaîne audio. Le bit-perfect est une *promesse*, pas une
*assertion*. Toute vérification passe par un DAC branché et une oreille.

**L'ajout** : le trait `audio/backend.rs` existe déjà en V1 (il était prévu pour WASAPI). On
l'utilise pour un **backend de test qui écrit dans un fichier WAV** au lieu de parler au HAL. À
partir de là :

- `decode(fixture.flac) → wav` comparé octet par octet à un golden file : **« bit-perfect » devient
  un test** (EQ off + volume 100 % ⇒ sortie identique à l'entrée).
- l'EQ, le resampler et le volume se testent sur des signaux connus (sinus, silence, DC) avec des
  assertions sur le spectre ou le RMS ;
- les transitions gapless se testent en vérifiant qu'il n'y a pas de discontinuité à la jonction ;
- tout ça tourne **en CI, sans matériel**.

**Les fixtures** (`fixtures/audio/`, versionnées, quelques secondes chacune) sont l'actif le plus
réutilisable du projet : un fichier par format supporté, un 24/192, un 16/44.1, une paire gapless,
un fichier aux métadonnées en **NFD**, un avec un artiste « feat. », un avec des caractères accentués,
un sans cover embarquée, un avec une cover 3000×3000, un fichier corrompu, un fichier à tags vides.
Chacune de ces fixtures correspond à un bug réel de V1.

**Coût** : moyen. **Gain** : la zone la plus risquée et la plus chère à déboguer devient vérifiable
en continu.

### 2. Tests d'architecture (fitness functions) 🔴

**Le problème V1** : l'invariant 6 dit « le filtre `excluded_paths` doit être vérifié à **chaque**
point d'entrée — et dans tout nouveau point d'entrée ». Une règle formulée comme « n'oublie pas de
répéter ça partout » sera violée. C'est le modèle de 3 invariants V1.

**L'ajout** : des tests qui portent sur la **forme du code**, pas sur son comportement.
`dependency-cruiser` (TS) + un test `cargo` qui parse l'arbre, ou simplement des tests qui grep avec
des règles explicites :

- aucun import circulaire (ce qui rend le médiateur `app.js` inutile — il n'existait que pour ça) ;
- **une seule fonction au monde construit une liste de pistes** : `library/repository`. Tout autre
  fichier qui lit le store de pistes directement fait échouer le test. → l'invariant 6 disparaît ;
- aucun accès au store hors des actions ;
- `audio/` n'importe jamais `ui/`, et réciproquement → la « règle de zone » de V1, qui reposait sur la
  discipline, devient mécanique ;
- aucun secret lu autrement que par le module dédié ;
- aucune chaîne de couleur en dur dans le CSS hors du fichier de tokens → la dérive design devient
  impossible ;
- **toute variable `var(--x)` utilisée est définie** → c'est l'invariant 19, et c'est le test qui
  rend le mieux de toute cette liste. Une variable CSS non définie rend la déclaration invalide « au
  moment du calcul » : le navigateur l'ignore **sans erreur**, même avec `!important`. Donc ni la
  relecture, ni les tests de comportement, ni le type-checking ne l'attrapent. En V1, `--glass-lo` /
  `--glass-md` / `--glass-hi` ont été utilisées **45 fois sans être définies** sur toute une PR de
  refonte : bordures, survols et état actif de la sidebar invisibles, découverts quatre mois plus
  tard. Le test tient en vingt lignes : collecter les `--x:` déclarées, collecter les `var(--x)`
  utilisées, échouer sur la différence (en tolérant les valeurs de repli `var(--x, 100)`) ;
- **aucun `println!` / `eprintln!` dans le backend** → tout passe par `tracing`. V1 en comptait
  **252**, contre **3** appels `tracing::`, et `logging.rs` ne capture pas stdout : 63 lignes de
  journal pour toute la vie de l'app installée. Les trois bugs trouvés le 2026-11-21 étaient tous
  annoncés en clair par le build de dev et **invisibles en production**. Détail :
  [`../../docs/MESURES-V1.md`](../../docs/MESURES-V1.md) §8 ;
- aucun fichier > 400 lignes.

**Coût** : faible. **Gain** : énorme, et croissant avec la taille du projet. C'est le seul mécanisme
qui contraint le code **futur**.

### 3. Une machine à états explicite pour la lecture 🔴

**Le problème V1** : `playbackContext` vaut `'album' | 'library' | 'playlist' | null`, avec le
commentaire « `null` → comportement conservateur ». Croisé avec shuffle (3 modes), repeat (3 modes),
la queue, le gapless et `tracksViewOrder`, ça fait une combinatoire que personne ne tient en tête — et
un bug connu en V1 : « si `ui.tracksViewOrder` est vide (la vue Pistes n'a jamais été rendue), la
transition gapless laisse `currentTrackIndex` inchangé ». L'état de la lecture dépendait d'un effet
de bord du rendu d'une vue.

**L'ajout** : un reducer typé avec une union exhaustive (ou XState), où le compilateur **refuse** un
`switch` incomplet. Plus de `null` « conservateur » : un état par cas réel. Et la décision « quelle
piste après celle-ci ? » devient une **fonction pure** `nextTrack(state) → TrackId | Stop`, testable
exhaustivement : 3 contextes × 3 shuffle × 3 repeat × queue vide/pleine × fin d'album/pas = ~100 cas,
tous couverts par une table de tests.

**Coût** : faible (c'est un refactoring de conception, pas d'outillage). **Gain** : supprime une
famille entière de bugs, et rend trivial l'ajout d'un mode (crossfade, radio, ReplayGain).

### 4. Property-based testing sur les normalisations 🟠

**Le problème V1** : la clé d'album. Invariant 1 (un album = une entrée, `normalizeKey()` = trim +
NFC), régression W003 (dédup par `artist+album` au lieu d'`album` seul → les variations « feat. »
créaient de faux albums), lookup à 4 niveaux dans la home parce que deux formats de clé coexistaient.
Trois bugs, une seule cause : une normalisation implicite, écrite plusieurs fois.

**L'ajout** : `fast-check` (TS) et `proptest` (Rust) sur toutes les fonctions de normalisation. On ne
teste plus des exemples, on teste des **propriétés** :

- `normalizeKey` est idempotente : `f(f(x)) == f(x)` ;
- deux chaînes NFC/NFD équivalentes donnent la même clé, pour **toute** chaîne Unicode générée ;
- N pistes d'un même album, quels que soient leurs artistes, produisent exactement 1 entrée ;
- le tri des pistes est total et stable (pas de permutation selon l'ordre d'entrée) ;
- la dédup de mosaïque est invariante par permutation de la playlist.

Et, structurellement : **un type `AlbumKey` avec un constructeur unique**. Si la seule façon d'obtenir
une `AlbumKey` est `AlbumKey::new(raw)`, l'invariant 1 n'est plus une règle, c'est le système de types.

**Coût** : faible. **Gain** : tue la famille de bugs la plus récurrente de V1.

### 5. Le mode parité V1 ↔ V2 🟠 — la réponse au vrai risque

**Le problème** : le risque n°1 de V2 n'est pas technique, c'est de perdre des features sans le voir.
Une checklist de 124 lignes aide, mais elle est cochée à la main.

**L'ajout** : un script jetable dans V1 qui **dumpe son état en JSON** (liste des albums avec leurs
clés, l'ordre de tri de chaque vue, les groupements d'artistes, la liste des pistes exclues, les mixes
de la home, les covers résolues par niveau de fallback). Le même dump dans V2. Puis un `diff`.

Lancé sur **ta vraie bibliothèque** — pas sur des fixtures — ça prouve la parité fonctionnelle sur
les données qui comptent, et ça remonte exactement les écarts : « V1 trouve 1 284 albums, V2 en
trouve 1 286, voici les deux en trop ». C'est le test que je voudrais avoir en premier.

**Coût** : faible (deux scripts et un diff). **Gain** : transforme « j'espère que c'est pareil » en
« voici les 2 écarts ».

### 6. Un test par invariant, et un garde anti-orphelin 🟠

**Le problème V1** : 3 invariants sur 19 couverts par un test. Les 16 autres reposaient sur la
vigilance humaine, puis sur celle d'un agent.

**L'ajout** : convention de nommage (`INV-07_metadata_in_file.test.ts`) et un **script de CI qui
compare la liste des invariants du fichier de référence à la liste des tests qui les citent**. Un
invariant sans test fait échouer la CI — ou doit être explicitement marqué
`coverage: architecture` / `coverage: manual` avec la raison. Les 7 pièges de plateforme resteront
`manual` : c'est légitime, mais c'est alors **écrit**, et la liste des règles non automatisées est
visible au lieu d'être implicite.

**Coût** : faible. **Gain** : ferme le trou structurel de V1 et empêche qu'il se réouvre.

### 7. Régression visuelle sur les composants 🟠

**Le problème V1** : la mosaïque 28×28 corrigée **4 fois**. Les bugs de layout (`min-width: 0`, le
`grid-column: 1 / -1` des loose tracks, la sidebar sous 900 px) ne se voient qu'à l'œil, et ne se
revoient que par hasard.

**L'ajout** : une page de composants (comme le `noir-design-system.html` de V1, mais **dans le
dépôt** et **générée**), capturée automatiquement état par état, avec diff par pixel contre une
référence. Chaque composant × chaque état (vide, 1, 2, 3, 4 covers / hover / actif / en lecture /
désactivé / texte long / texte avec guillemets) × les deux largeurs critiques (900 px et au-dessus).

Bonus : c'est aussi le mécanisme qui prouve « le design à l'identique » — on capture V1 une fois, on
s'en sert de référence.

**Coût** : moyen (les baselines demandent de l'entretien). **Gain** : la seule défense contre les
régressions visuelles, qui sont la moitié des régressions d'une app comme celle-ci.

### 8. Des budgets de performance en CI 🟠

**Le problème V1** : quatre phases d'optimisation (RAF idle, HTTP async, protocole `noir://`,
recyclage DOM) avec des gains mesurés — **et rien qui les protège**. Rien n'empêche un `innerHTML`
dans une boucle de scroll de tout annuler.

**L'ajout** : une bibliothèque de test générée (10 000 pistes synthétiques, c'est une fixture) et un
fichier de budgets que la CI fait respecter :

| Mesure | Budget V1 constaté |
|---|---|
| démarrage jusqu'à UI utilisable, 10 k pistes | à mesurer sur V1 d'abord |
| recherche sur 10 k pistes | < 16 ms |
| images par seconde du virtual scroll | 60 |
| heap après 1 000 covers chargées | ~60 Ko d'URLs (et non 700 Mo de base64) |
| temps de rendu d'une vue (p95) | à mesurer |

Mesurer V1 **avant** de commencer, pour que les budgets soient des faits et non des souhaits.

**Coût** : moyen. **Gain** : la perf cesse d'être un chantier qu'on refait.

### 9. La documentation générée, et la CI qui échoue sur la dérive 🟠

**Le problème V1** : cause racine n°5 du postmortem. 11 affirmations fausses, trois chemins de données
documentés dont deux faux, et un document de référence hors du dépôt.

**L'ajout** : tout ce qui est déductible du code est **généré** dans des fichiers commités — liste des
commandes IPC, catalogue des events avec leur payload, tokens CSS, surface de l'état, liste des
fixtures, tableau de couverture des invariants. La CI régénère et **échoue si le résultat diffère du
commit**. On n'écrit à la main que ce qui ne se déduit pas : les invariants et les décisions.

Et une règle d'architecture : **aucune référence hors du dépôt**, jamais.

**Coût** : faible. **Gain** : la documentation redevient fiable, donc utilisable sans vérification —
ce qui change le coût de chaque session.

### 10. Un seul point d'entrée : `./check` 🟢

**Le problème V1** : l'agent `test-runner` devait connaître quatre commandes différentes, dans trois
répertoires, avec un cas particulier si `node_modules` était absent et un timeout de 600 s pour la
première compilation Rust.

**L'ajout** : `./check` (et `./check --fast`) qui fait **tout**, identique en local, en pre-commit et
en CI. Une seule chose à savoir, une seule chose à maintenir, et la CI ne peut pas diverger du local.
Plus un pre-commit (lefthook) sur le format, le lint, les types et les tests affectés.

**Coût** : quasi nul. **Gain** : c'est ce qui fait que les 9 points précédents tournent vraiment.

### 11. Des invariants vérifiés à l'exécution 🟢

**L'ajout** : une fonction `invariant(cond, msg)` qui **lève en développement** et **remonte à Sentry
en production** (sans interrompre l'utilisateur). Posée aux endroits où un invariant est vérifiable à
chaud :

- après `groupAlbums()` : aucune clé normalisée en double ;
- après un chargement de pistes : aucun chemin présent dans `excluded_paths` ;
- après une transition gapless : la piste courante a changé ;
- après une écriture de métadonnées : le fichier relu contient bien les nouvelles valeurs.

Ça attrape ce que les tests ne voient pas : les vraies données, les cas limites réels, chez les
testeurs. V1 avait déjà Sentry avec anonymisation — autant s'en servir pour ça.

**Coût** : quasi nul. **Gain** : le filet de dernier recours, sur la vraie bibliothèque.

### 12. Un serveur SMB jetable, avec injection de panne 🟢

**Le problème V1** : les tests `network_source` sont `#[ignore]`. La régression W005 (l'UI gèle quand
le NAS est injoignable) et le piège des dialogues Keychain au démarrage ne se voient qu'avec un vrai
NAS, dans le bon état.

**L'ajout** : un conteneur Samba lancé par les tests d'intégration, et un proxy TCP qui injecte
latence, coupure en milieu de transfert, et connexion périmée. Les invariants SMB (retry sur
connexion périmée, propagation de l'erreur de download, singleton) deviennent des tests. En CI
macOS c'est compliqué ; en local non, et c'est déjà beaucoup mieux que `#[ignore]`.

**Coût** : moyen. **Gain** : trois invariants bloquants passent de « manuel » à « automatique ».

---

## Pour ajouter une feature sans casser le reste

Les trois éléments qui rendent l'ajout de features sûr, et qui ne sont pas des tests :

- **Un ADR par décision** (`docs/adr/0001-store-sqlite.md`), numéroté, immuable, avec le contexte et
  les alternatives écartées. En V1, les « décisions techniques actées » s'accumulaient dans une
  section de skill qui grossissait ; on perdait le *pourquoi*, donc la règle finissait par être
  supprimée par quelqu'un qui la trouvait arbitraire.
- **Un skill « ajouter une feature »** : la checklist exacte — où vont les fichiers, quels tests
  écrire, mettre à jour le fichier de parité, ADR si décision, invariant si la régression est
  possible. C'est ce qui rend une session reproductible quel que soit le modèle.
- **Des feature flags** pendant toute la reconstruction. Ils permettent de livrer V2 par morceaux,
  de garder un chemin V1 en parallèle, et de comparer côte à côte au lieu de basculer d'un coup.

Et deux garde-fous repris de V1, qui ont fait leurs preuves : le **CLAUDE.md sous 150 lignes**
(vérifié par la CI — il avait fini par être trop gros pour être lu en entier) et le **hook `Stop`**
qui propose d'ajouter un invariant quand du code change sans que le fichier d'invariants bouge.

---

## Ce que je n'ajouterais pas

Un harnais trop lourd ne tourne plus, et un harnais qui ne tourne plus est pire que pas de harnais.

| À éviter | Pourquoi |
|---|---|
| **Un objectif de couverture** (80 %, 100 %) | Mesure la mauvaise chose. 100 % de couverture sur `views.ts` n'aurait attrapé aucun des bugs de V1. La question est « quel invariant est protégé », pas « quel pourcentage de lignes est exécuté ». |
| **Des tests e2e pour tout** | Lents, fragiles, et ils cassent pour de mauvaises raisons. Réserve-les à ce qui n'est testable qu'ainsi : drag & drop, virtual scroll, raccourcis clavier, le parcours d'onboarding. Une dizaine, pas cent. |
| **Du mocking systématique** | Un mock encode une hypothèse sur du code que tu ne contrôles pas. Préfère un **vrai** backend de test (le WAV, le conteneur Samba) : il attrape les écarts de comportement, pas seulement les appels. |
| **Une architecture hexagonale / DDD complète** | Sur une app solo de 30 000 lignes, le coût cérémoniel dépasse le gain. Les tests d'architecture (point 2) donnent 80 % du bénéfice pour 5 % du coût. |
| **Un monorepo avec packages** | Inutile pour une app macOS unique. À reconsidérer le jour où un backend Windows arrive. |
| **Un design system en bibliothèque publiée** | La page de composants générée (point 7) suffit et ne coûte rien à maintenir. |
| **Tout mettre en phase 0** | Les points 1, 2, 3, 6, 9, 10 sont en phase 0 — ils contraignent le code à écrire. Les points 4, 5, 11 arrivent avec la bibliothèque (phase 2). Les points 7, 8, 12 arrivent avec l'UI et le NAS (phases 3-5). Mettre les douze au départ bloquerait le projet avant la première ligne. |

---

## Ce que chaque faiblesse de V1 devient

| Faiblesse V1 | Ce qui la rend impossible |
|---|---|
| 9 bugs de noms de champs (onboarding, `result.id`) | IPC typée générée (socle) |
| `.size` sur un Array (V014) | TypeScript strict (socle) |
| Secret manquant en silence (inv. 16) | `build.rs` + `compile_error!()` (socle) |
| Invariant 1 + W003 + lookup à 4 niveaux (clé d'album) | type `AlbumKey` à constructeur unique + property tests (**4**) |
| Invariant 6 (`excluded_paths` répété partout) | test d'architecture : un seul constructeur de liste de pistes (**2**) |
| Invariants 7, 8, 9, V015 (cohérence des caches) | SQLite + transactions (socle) |
| Bug gapless / `tracksViewOrder` vide | machine à états + `nextTrack` pure (**3**) |
| Invariant 19 (shuffle dupliqué avec un autre glyphe) | test d'architecture : une seule source par comportement (**2**) |
| Invariants 10-13 (mosaïque corrigée 4 fois) | régression visuelle état par état (**7**) |
| V013 (ne jamais réassigner l'état) | store immuable (socle) |
| Bit-perfect non vérifiable | backend audio fake + golden files (**1**) |
| 3 suites Jest skippées (`invoke` non mockable) | client IPC typé à implémentation substituable (socle) |
| Tests SMB `#[ignore]` | conteneur Samba + injection de panne (**12**) |
| W005 (UI gelée, NAS offline) | idem (**12**) |
| 4 phases de perf non protégées | budgets en CI (**8**) |
| 11 affirmations de doc fausses | documentation générée + CI (**9**) |
| Design system hors du dépôt | règle d'architecture : aucune référence externe (**9**) |
| Perte de features en V2 | mode parité V1 ↔ V2 (**5**) |

Il reste, irréductible, les **7 pièges de plateforme** de
[`../04-invariants/README.md`](../04-invariants/README.md). Ceux-là resteront des règles écrites,
avec leur gravité, et c'est normal : ils décrivent le comportement de CoreAudio, de WebKit et de
Gatekeeper, pas celui de ton code.
