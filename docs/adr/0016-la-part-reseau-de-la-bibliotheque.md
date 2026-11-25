# 0016 — La part réseau de la bibliothèque : SMB lu directement, et un scan par périmètre

- **Date** : 2026-11-25
- **État** : Acceptée
- **Décideur** : la session, sur une décision de produit de **Thomas** le 2026-11-25 — « oui,
  le NAS » — c'est-à-dire *que* la part réseau soit visible, pas *comment*. La forme ci-dessous
  appartient à la session, comme pour les ADR 0010 et 0012. Le seul point que Thomas doit
  trancher a son propre ADR : [0015](0015-nom-de-service-keychain-pour-les-mots-de-passe-smb.md).

## Le problème, en un chiffre

**91 % de la bibliothèque de Thomas est sur son NAS.** 3 137 pistes sur 3 443
([`docs/MESURES-V1.md`](../MESURES-V1.md) §1). Au premier lancement de V2 le 2026-11-25, l'app a
repris la configuration de V1 et affiché **305 pistes** — toutes locales, et c'était le bon
chiffre pour ce qu'elle savait faire. Les 3 137 autres n'apparaissaient nulle part.

## Décision

### 1. SMB se lit directement, par `libsmbclient`, et pas à travers un volume monté

V2 parle à `libsmbclient` (crate `pavao`), comme V1. Une piste distante est désignée par une
**URI**, dans le format hérité de V1 :

```
smb://{id de la source}/{partage}/{chemin dans le partage, sans slash initial}
```

Pour la source de Thomas (`id` `dcdd24af…`, partage `music`, `remote_path` `/LOSSLESS/complete`),
cela donne `smb://dcdd24af…/music/LOSSLESS/complete/<album>/<piste>.flac`. Le format est
**épinglé par un test qui rejoue ses valeurs réelles** (`src-tauri/tests/reseau/uri.rs`) : les
3 137 chemins de son cache V1 portent déjà ce préfixe, et une URI assemblée autrement rendrait
sa bibliothèque invisible deux fois — d'abord en V2, puis en revenant à V1.

> **L'écart apparent, résolu.** Le cache de V1 annonce le préfixe `…/music/LOSSLESS` alors que
> `remote_path` vaut `/LOSSLESS/complete`. Il n'y a pas d'écart : `/LOSSLESS` est simplement un
> préfixe plus court que `/LOSSLESS/complete`. Lu dans `walk_smb_audio_files` de V1, le chemin
> part de `source.remote_path` et descend ; l'URI concatène `share` puis ce chemin. Vérifié
> aussi dans l'autre sens : **3 137 des 3 137 entrées** commencent par
> `smb://dcdd24af…/music/LOSSLESS/complete/`.

**L'alternative écartée mérite d'être écrite, parce qu'elle est tentante** : monter le partage
dans le Finder et le scanner comme un dossier ordinaire (`/Volumes/music/…`). Ça ne demanderait
ni `libsmbclient`, ni Keychain, ni mutex — et V1 s'en servait déjà pour la lecture audio. Mais
l'identité d'une piste **est son chemin** (`library/types.rs`), et un chemin de montage n'est
pas stable : macOS écrit `/Volumes/music 1` quand `music` est déjà pris, et le même NAS donne
donc deux bibliothèques. Pire, le volume démonté, toutes les pistes deviennent « disparues » et
le scan suivant les **efface**. Une URI qui ne dépend que de la source est stable par
construction.

### 2. Un scan ne conclut la disparition d'une piste que **dans son périmètre** — INV-27

C'est la règle née de cette session, et c'est un 🔴.

`Depot::appliquer` effaçait toute piste connue qu'il n'avait pas revue. Avec deux provenances,
un scan de disque aurait donc effacé les **3 137 pistes du NAS** à son premier passage, et un
scan du NAS les 305 pistes locales. En V1 les deux scans écrivaient dans le même cache global et
la règle était « n'oublie pas de filtrer » — la formulation qu'INV-06 existe pour ne plus
écrire.

Le périmètre devient donc un **type** (`library::scan::Perimetre`), passé à l'application du
scan, et le dépôt ne peut pas l'oublier : il n'y a pas de variante « tout ». Tenu par
`src-tauri/tests/reseau/perimetre.rs`, qui mélange les deux provenances dans une même base et
scanne chacune à son tour.

### 3. Les métadonnées d'une piste distante sont lues **dans le fichier distant**, pas reprises de V1

Le `network_scan_cache.json` de V1 est migré et contient les métadonnées des 3 137 pistes : s'en
servir aurait rendu le premier scan quasi instantané. **Écarté**, pour trois raisons mesurées :

- il est **incomplet** — 42 des 3 137 entrées n'ont ni codec ni durée (V1 n'a pas su les lire) ;
- il porte **l'interprétation de V1**, et la passe de vérité du 2026-11-23 a comparé 306
  fichiers champ par champ : sur **182 écarts**, un seul donnait raison à V1 (le nom de format,
  d'où INV-23). Reprendre ce cache, c'est importer 181 écarts connus sur 91 % de la
  bibliothèque ;
- la source de vérité d'une métadonnée est **le fichier audio** (INV-07). Un cache d'une autre
  application n'en est pas une.

Le coût est assumé, et il est désormais **mesuré** — cette version remplace l'estimation que
cet ADR portait, parce qu'il demandait lui-même « la mesure réelle se fera au premier scan de
Thomas, et c'est elle qu'il faudra écrire ici ». Seul ce chiffre change ; la décision, elle, est
intacte. Le relevé complet est dans [`../HISTORY.md`](../HISTORY.md), au 2026-11-25.

| | estimé | **mesuré** |
|---|---|---|
| par fichier | ~41 ms | **~690 ms** |
| 3 137 pistes | ~2 minutes | **36 minutes** |

**Et l'estimation était fausse d'un facteur 17 pour une raison qui vaut d'être écrite** : elle
comptait le **débit** (1 Mio à 36 Mo/s) là où le coût réel est la **latence**. Un en-tête de
1 Mio n'est pas un transfert, c'est un aller-retour — ouvrir, lire, fermer — sur un lien où
chaque aller-retour se paie, et une centaine de fichiers (MP3, M4A) sont lus en entier par
dessus. Le raisonnement « combien d'octets ? » ne prédit rien ici ; c'est « combien
d'allers-retours ? » qui compte. Les scans suivants ne relisent rien (§4), donc ces 36 minutes
sont payées **une fois**.

**Et l'en-tête ne suffit pas pour tous les formats.** Une durée se lit dans un bloc d'en-tête
pour FLAC (STREAMINFO), WAV (taille du chunk `data`) et AIFF (COMM) — mais MP3 sans en-tête
Xing la *déduit de la taille du fichier*, et OGG Vorbis la lit sur la **dernière** page. Lire
l'en-tête de ces trois-là donnerait une durée **plausible et fausse**, c'est-à-dire le pire mode
de défaut. Ils sont donc lus **en entier**. Chez Thomas cela fait 82 MP3 + 19 M4A + 0 OGG sur
3 443 pistes, soit 3 %. La règle est tenue par un test qui tronque les fixtures et compare
(`src-tauri/tests/reseau/entete.rs`) : il rougirait si lofty changeait de comportement, et il se
périme de lui-même le jour où un format quitte la liste.

### 4. Le scan différentiel d'une source distante est **par présence**, pas par empreinte

`libsmbclient` via pavao 0.2 ne rend **ni taille ni date** en listant un dossier (`SmbDirent`
ne porte que le nom et le type ; vérifié aussi dans le cache de V1 : `size: 0, modified: 0` sur
les 3 137 entrées). Demander un `stat` par fichier rajouterait 3 137 allers-retours réseau — la
faute que l'on cherche à éviter.

Donc : **une piste distante déjà dans la base n'est pas relue.** Taille et date sont stockées à
zéro, ce qui fait que la comparaison d'empreintes existante conclut « inchangée » sans cas
particulier.

**Conséquence acceptée, et elle se voit** : un fichier modifié *sur le NAS depuis une autre
machine* ne sera pas repris. Pour le reprendre, il faut aujourd'hui exclure la piste puis
rescanner — ce qui est le mauvais geste, puisque l'exclusion est permanente (INV-06). Un
« rescan complet d'une source » est donc une fonction à ajouter, et elle est notée comme telle
dans `CLAUDE.md` § Bugs connus plutôt que devinée ici.

### 5. Le mutex unique, et ce qu'il impose à l'IPC

`libsmbclient` est un **singleton de process** (PLAT-2, 🔴) : deux clients concurrents rendent
`EINVAL`. Toute opération passe par un `Mutex` unique dans `network::client`.

Cela a une conséquence sur la commande IPC, et c'est la raison de sa forme en trois temps : un
scan dure des minutes, donc il ne tient **pas** le verrou de la bibliothèque pendant ce temps.

1. verrou court sur le dépôt : lire la source et les chemins déjà connus ;
2. **sans verrou** : parcours SMB et lecture des métadonnées ;
3. verrou court sur le dépôt : appliquer, en une transaction.

Sans ce découpage, `list_tracks` serait bloqué pendant tout le scan et la fenêtre gèlerait —
c'est le comportement **W005**, qui reste à porter mais dont la cause principale est fermée ici.

### 6. Une connexion périmée est retentée une fois — INV-03 passe de « planifiée » à tenue

La règle attendait la phase 5 et un conteneur Samba. Elle ne l'attend plus : la logique de
reprise est une fonction à deux fermetures (`network::client::reessayer_une_fois`), donc elle
s'éprouve **par injection de panne**, sans NAS et sans conteneur
(`src-tauri/src/network/client.rs`, tests). Ce qui demande toujours un vrai serveur est la
*cause* d'une connexion périmée, pas la réaction à cette cause.

## Ce qui est explicitement hors de cet ADR

- **La lecture audio depuis le NAS** (téléchargement progressif, `SmbProgressiveFile`, gapless
  SMB). Le backend de sortie CoreAudio n'existe pas : « le son n'est pas encore branché » est
  écrit dans la fenêtre. Du streaming SMB ne serait démontrable par rien.
- **Pochettes et vignettes depuis SMB.** La correction de V1 est déjà écrite
  ([`docs/MESURES-V1.md`](../MESURES-V1.md) §5) : la source d'une vignette est le fichier de
  pochette sur le disque, jamais une ré-extraction. Phase 4.
- **La découverte mDNS/Bonjour.** La source de Thomas est migrée avec son nom d'hôte ; la
  découverte ne lui rend rien aujourd'hui.
- **Le nom de service Keychain** : [ADR 0015](0015-nom-de-service-keychain-pour-les-mots-de-passe-smb.md).

## Alternatives écartées

| Alternative | Pourquoi écartée |
|---|---|
| **Monter le partage et le scanner comme un dossier** | Voir §1 : l'identité d'une piste deviendrait un chemin de montage, donc instable, et un volume démonté ferait effacer 91 % de la bibliothèque. |
| **Reprendre les 3 137 pistes du cache de V1 au démarrage** (sans NAS, sans mot de passe) | C'est la plus tentante : la bibliothèque complète apparaîtrait au lancement, NAS éteint. Écartée pour §3 (42 trous, 181 écarts connus, INV-07) **et** parce que la demande était « voir sa source, déclencher un scan, et que les pistes arrivent » — un remplissage silencieux au démarrage ne prouve pas que le scan fonctionne. Reste une option si le NAS éteint devient un vrai problème d'usage ; à ce moment-là ce sera un ADR, pas une improvisation. |
| **`stat` par fichier pour obtenir taille et date** | +3 137 allers-retours réseau pour un scan différentiel qui, de toute façon, n'a pas à relire ce qu'il connaît. Le coût est certain, le gain hypothétique. |
| **Porter le parseur FLAC natif de V1** (`parse_flac_blocks`, ~200 lignes) | V1 l'a écrit parce que lofty 0.18 échouait sur un FLAC tronqué à 512 Ko (les offsets du SEEKTABLE pointent au-delà). Écarté pour l'instant : un en-tête de 1 Mio plus un **repli sur le fichier entier** donne le même résultat sans deuxième implémentation de FLAC à maintenir. Si le repli se déclenche souvent, la mesure le dira — et c'est elle qui justifiera les 200 lignes, pas la crainte. |
| **Un scan réseau asynchrone avec événements de progression** | Ce qu'il faut à terme. Hors périmètre ici : la coquille est volontairement laide et n'a pas de barre de progression à nourrir, et un canal d'événements inutilisé serait du code à jeter. La fenêtre annonce la durée attendue à la place. |

## Conséquences acceptées

- **`brew install samba` devient nécessaire pour compiler.** Le `build.rs` de `pavao-sys`
  panique si `pkg-config` ne trouve pas `smbclient`. La CI l'installe en une étape ; la machine
  de Thomas l'a déjà, depuis V1.
- **Le binaire se lie à une dylib de Homebrew** (`/opt/homebrew/opt/samba/lib/libsmbclient…`).
  C'est exactement ce que INV-18 / PLAT-6 décrivent : sans `dylibbundler` **suivi du retrait des
  LC_RPATH dupliqués** et d'une re-signature, l'app crashera au lancement **sur tout Mac sans
  Homebrew** — c'est-à-dire sur toute machine qui n'est pas celle de développement. La dette
  est à la phase 6, elle est écrite, et elle n'était pas nouvelle : c'est le même piège que V1
  a payé.
- **Un premier scan dure des minutes**, et la fenêtre le dit avant de commencer.
- **Un mot de passe de NAS est demandé une fois par session**, au premier scan. Il n'est jamais
  lu au démarrage (V009).
