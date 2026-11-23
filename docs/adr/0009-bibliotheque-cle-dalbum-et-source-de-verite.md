# 0009 — La bibliothèque : schéma SQLite, clé d'album typée, source de vérité des métadonnées

- **Date** : 2026-11-22
- **État** : Acceptée
- **Décideur** : **session de phase 2** — choix techniques pris à l'intérieur d'un périmètre
  déjà tranché par l'[ADR 0002](0002-stack-technique.md) et par le brief de phase 2.
  **Non soumis à Thomas.** Il n'a arbitré aucun des quatre points ci-dessous, et l'écrire
  ici est le seul moyen de garder distinguable ce qu'il a réellement décidé — pour INV-21 et
  INV-22, par exemple — de ce qu'une session a décidé pour lui. Ce que ce champ doit dire en
  général, quand une session décide à l'intérieur d'un brief qu'il a approuvé, est une
  question ouverte devant lui ; la formulation d'ici suivra sa réponse.
- **Suit** : [0002](0002-stack-technique.md) (SQLite, `rusqlite` en `bundled`),
  [0003](0003-dossier-de-donnees.md) (le dossier de données)

## Contexte

L'[ADR 0002](0002-stack-technique.md) a tranché **SQLite** pour le store de bibliothèque, en
annonçant deux conséquences acceptées : « il faut concevoir un schéma et des migrations
versionnées ». La phase 2 les écrit. Trois décisions en sont sorties, et chacune ferme une
famille de régressions de V1 plutôt qu'un bug.

Le contexte chiffré, pour mémoire : V1 maintenait **sept caches globaux** persistés chacun
dans son JSON, modifiés concurremment par le scan de fond, les écritures lofty et les
commandes IPC, sans transaction. Quatre invariants classés BLOQUANTS décrivaient le même
défaut. Et une seule fonction mal partagée — `normalizeKey()`, trim + NFC, recopiée à
plusieurs endroits — a produit **trois** bugs : l'invariant 1, la régression W003 et un
lookup à quatre niveaux dans la home.

## Décisions

### 1. La clé d'album est un TYPE, pas une convention

`AlbumKey` est un newtype dont le seul constructeur est `AlbumKey::new(brut)`, qui fait trim
+ NFC. Pas de `From<String>`, pas de champ public, pas de constructeur de secours : il n'y a
**aucun chemin** par lequel une clé non normalisée puisse entrer dans la base.

Ce qui entre dans la clé : **l'album seul**. Pas l'artiste — c'est exactement la régression
W003, où les variations « feat. » créaient de faux albums. Ce qui n'y entre pas : la casse et
la ponctuation, laissées telles quelles, parce que replier la casse fusionnerait deux albums
réellement distincts et que la parité avec V1 est la cible ([ADR 0004](0004-cible-de-parite.md)).

**Et la clé est construite dans UN seul langage.** Le frontend la reçoit dans le champ
`cle_album` de chaque piste et ne la recalcule jamais. Un test d'architecture refuse toute
normalisation Unicode écrite à la main dans `src/` **et dans `tests/`** : un faux backend de
test qui normaliserait serait une deuxième implémentation arrivée par la porte du harnais.

**Écarté** : une fonction `normaliser_cle(&str) -> String` bien documentée. C'est exactement
ce que V1 avait. Une fonction peut ne pas être appelée ; un type ne peut pas ne pas exister.

### 2. La source de vérité d'une métadonnée est le fichier audio — c'est un ORDRE D'ÉCRITURE

INV-07 devient une mécanique : écrire le fichier avec lofty, **relire le fichier**, et
n'alimenter la base qu'avec ce que la relecture a rendu. Trois propriétés en découlent :

- si l'écriture du fichier échoue, la base n'a pas bougé ;
- si lofty normalise une valeur à sa façon, c'est **sa** version qui entre dans la base, donc
  la base ne peut pas annoncer autre chose que le fichier ;
- le contraire — base à jour, fichier non écrit — est devenu impossible à écrire sans
  supprimer du code.

Le bulk edit écrit les fichiers un par un (c'est le disque, on ne peut pas faire autrement)
mais met la base à jour **en une seule transaction**. Si une écriture échoue au milieu, la
transaction est annulée ; les fichiers déjà écrits ont une date de modification nouvelle,
donc le scan différentiel les rattrape. L'asymétrie est voulue dans ce sens-là.

**Écarté** : écrire la base d'abord « pour que l'interface réponde tout de suite ». C'est le
défaut de V1 : la métadonnée disparaissait dès que l'utilisateur ouvrait le fichier ailleurs,
et il n'avait aucun moyen de comprendre pourquoi.

### 3. Un schéma strict, des migrations versionnées dès la première

`PRAGMA user_version` porte le numéro de version — un entier dans l'en-tête du fichier, donc
pas de table d'état à garder cohérente. Chaque migration tourne **dans une transaction** :
soit le schéma avance d'une version complète, soit il ne bouge pas. Les tables sont
déclarées `STRICT` : SQLite accepterait sinon un texte dans une colonne entière, et l'erreur
de type se découvrirait trois écrans plus loin, sous forme de donnée absurde.

**Une base écrite par une version plus récente de Hean est refusée**, pas « réparée ». Ne pas
démarrer est un défaut visible ; perdre des données en silence ne l'est pas — c'est le même
raisonnement que celui de Thomas sur INV-21 et INV-22.

Le **mécanisme** (appliquer une version) et le **contenu** (le schéma) sont séparés :
`migrations.rs` d'un côté, `repository.rs` de l'autre. Ce n'est pas de l'esthétique — c'est
le seul fichier qui a le droit de nommer la table des chemins exclus (INV-06), et séparer
garde la règle mécanique au lieu de déclarative.

**Conséquence sur la délégation** : `src-tauri/src/library/migrations.rs` rejoint la zone
`migration-donnees` de [`tools/zones-interdites.txt`](../../tools/zones-interdites.txt). Une
migration de schéma qui perd une colonne perd les mêmes données qu'une copie ratée ; le
raisonnement de l'[ADR 0005](0005-entretien-du-harnais-et-orchestration.md) s'applique mot
pour mot.

### 4. Un seul écrivain : `Mutex<Depot>`

L'état partagé du backend est **une base, une connexion, un verrou**. C'est ce qui raye
INV-08, 09, 10 et 14. Ajouter un second état mutable côté Rust rouvrirait ces quatre portes,
et c'est BLOQUANT.

Côté frontend, le symétrique : un objet d'état **gelé** (`Object.freeze`), dont le contenu se
mute en place. V013 (« `library = {}` cassait la référence vue par les autres modules »)
n'est plus une règle à retenir : en module ES, donc en mode strict, la réassignation lève une
`TypeError`.

Le scan reste **séquentiel**. V1 le parallélisait avec rayon (B02), et il faudra le refaire —
mais pas avant qu'une mesure le réclame : la parallélisation de V1 est arrivée avec les sept
caches concurrents, et c'est elle qui a rendu les invariants 8, 9 et 10 nécessaires.

## Dépendances ajoutées

| Dépendance | Rôle | Pourquoi pas à la main |
|---|---|---|
| `lofty` 0.22 | lire **et écrire** les tags | C'est la bibliothèque de V1, et écrire un tag FLAC/MP4/ID3 à la main est un projet à soi seul. Épinglée en 0.22 : les suivantes exigent rustc 1.89, au-delà de la `rust-version` du crate ([ADR 0008](0008-epingler-la-chaine-doutils.md)). |
| `unicode-normalization` 0.1 | NFC dans `AlbumKey::new` | La normalisation Unicode se fait avec les tables d'Unicode, pas avec des règles devinées. |
| `proptest` 1 (dev) | property tests Rust | [HARNAIS.md](../../bootstrap/06-setup-v2/HARNAIS.md) §4. |
| `fast-check` 4 (dev) | property tests TypeScript | idem. Dépendance de **développement** : la règle « zéro dépendance npm de production sans ADR » n'est pas entamée, et `./check` recompte les dépendances de production à chaque passage. |

Pas de `rayon` (scan séquentiel), pas de `walkdir` (une descente récursive tient en trente
lignes et n'a pas besoin d'être configurable), pas de `tempfile` en dépendance directe (la
fonction de huit lignes du test dit ce qu'elle nettoie).

## Conséquences acceptées

- **Le frontend ne sait pas normaliser, et c'est voulu.** Toute comparaison de titres devra
  passer par le backend ou par une clé déjà reçue. Si un jour une vue a besoin de comparer
  deux chaînes sans aller-retour IPC, la réponse sera **une commande**, pas une fonction
  recopiée en TypeScript.
- **Les champs de l'IPC sont en `snake_case`** (`cle_album`, `duree_ms`), parce que les types
  sont générés depuis le Rust et qu'on ne retouche pas un fichier généré. Le défaut de V1
  n'était pas la convention de nommage, c'était que les noms n'étaient **vérifiés nulle part**.
- **Pas de `u64` ni d'`i64` dans les types IPC** : `specta_typescript` refuse de les exporter,
  parce qu'ils ne tiennent pas dans un `number` JavaScript. La contrainte est bonne à prendre.
  Taille de fichier et date de modification restent en `i64` **dans la base**, où elles
  servent au scan différentiel et non à l'affichage.
- **L'identité d'une piste est son chemin de fichier.** La question « un morceau logique avec
  N fichiers ? » reste ouverte et datée de la phase 3 (21 % de la bibliothèque de Thomas est
  en double, [`../MESURES-V1.md`](../MESURES-V1.md) §10).
- **Les erreurs IPC remontent en texte**, pas en type. Une erreur typée serait mieux ; elle
  attendra d'avoir un affichage d'erreur à nourrir (phase 3). Ce qui est acquis dès maintenant
  est qu'aucune erreur ne se perd : chacune est journalisée par `tracing` avant de partir.
- **Un fichier abîmé entre quand même dans la bibliothèque.** Mesuré en phase 2, et contre
  l'attente : lofty accepte les trois fixtures de fichiers abîmés, en-tête STREAMINFO
  incohérent compris — là où symphonia, le décodeur, refuse ce dernier. Pour
  `corrupt-entete.flac`, qui dure une seconde, le conteneur annonce **plus de 71 minutes** et
  lofty le rapporte sans broncher. Le contrôle d'intégrité est INV-22, daté de la phase 5 ;
  d'ici là un test fixe la mesure, et s'il se mettait à échouer parce que lofty refuse ces
  fichiers, **cet échec serait la bonne nouvelle**.

## Ce que cette phase ne fait pas

- **Le pipeline de pochettes** (INV-12, 4 niveaux de fallback) — sa propre session.
- **La migration des données de V1** (`noir/` → `hean/`, [ADR 0003](0003-dossier-de-donnees.md))
  — zone jamais déléguée, fixtures tirées de vraies données, sa propre session.
- **Toute interface.** Le frontend s'arrête au dépôt et au regroupement, c'est-à-dire à ce
  qu'il faut pour que les six comportements soient vérifiables.
