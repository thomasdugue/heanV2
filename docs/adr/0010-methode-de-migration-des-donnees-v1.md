# 0010 — La méthode de migration des données de V1

- **Date** : 2026-11-22
- **État** : Acceptée
- **Décideur** : **Thomas pour la priorité**, le 2026-11-22 : la migration des données avant
  les écrans, pour sécuriser la bibliothèque des beta-testeurs d'abord. **Les choix de
  méthode ci-dessous sont ceux de la session**, pris à l'intérieur du périmètre déjà tranché
  par l'[ADR 0003](0003-dossier-de-donnees.md) — ils ne lui ont pas été soumis. La même
  distinction que pour l'[ADR 0009](0009-bibliotheque-cle-dalbum-et-source-de-verite.md), et
  pour la même raison : « Thomas a décidé ceci » est ce qui fait qu'une règle ne se
  rediscute pas, donc ça ne s'écrit que quand c'est vrai.
- **Suit** : [0003](0003-dossier-de-donnees.md) (le dossier de données et la migration),
  [0009](0009-bibliotheque-cle-dalbum-et-source-de-verite.md) (la bibliothèque)

## Contexte

L'[ADR 0003](0003-dossier-de-donnees.md) a tranché le **quoi** : les données passent de
`~/Library/Application Support/noir/` à `hean/`, par une migration one-shot au premier
lancement, parce que ~50 beta-testeurs y ont bibliothèque, playlists, favoris, historique
d'écoute et sources NAS. Il a aussi écrit la conséquence acceptée la plus importante : « une
migration non testée est une perte de données chez 50 personnes. Elle se teste sur une copie
réelle d'un dossier `noir/` complet, pas sur des fixtures. »

Cet ADR tranche le **comment**. Il s'appuie sur une mesure du dossier réel, faite le
2026-11-22 : **572 Mo, 2 446 fichiers**, dont `covers/` 275 Mo (2 175 fichiers),
`smb_buffer/` **289 Mo (3 fichiers)**, `thumbnails/` 2,1 Mo, `logs/` 28 Ko, `feedback/`
20 Ko. Deux surprises dans ce relevé, toutes deux des défauts de V1 :

- **la moitié du dossier est un cache jetable.** `smb_buffer/` contient les fichiers
  temporaires du téléchargement progressif SMB, que V1 devait nettoyer à chaque lancement
  (comportement W008, correctif de V1 au commit `eaee676`) : il en restait trois, dont un de
  147 Mo, datés de la veille ;
- **`logs/` garde des fichiers de juillet** alors que la rotation est annoncée à 7 jours ;
- et **`dap_sync.db` n'existe pas**, là où l'ADR 0003 le décrit comme « laissé en place
  volontairement pour préserver les configurations des beta-testeurs ». Le fichier est resté
  dans la documentation après avoir disparu du disque — ou il n'a jamais existé chez Thomas.
  La migration le prend en charge s'il est là, et ne s'en étonne pas s'il manque.

## Décisions

### 1. Copier à côté, vérifier, échanger, puis supprimer

Quatre temps, et chacun ferme un mode de défaut précis :

1. **Copier dans `hean.en-cours-de-migration/`**, voisin du dossier d'arrivée, et non
   directement dans `hean/`. Une copie interrompue — application tuée, machine endormie,
   disque plein — laisse alors un dossier provisoire et **pas de `hean/` du tout** : le
   lancement suivant repart proprement. En copiant directement dans `hean/`, une
   interruption laisserait une bibliothèque à moitié présente, **indistinguable d'une
   bibliothèque complète**. C'est le cœur de la décision.
2. **Vérifier** la copie contre la source (`migration/verification.rs`). C'est ce qui
   autorise l'étape 4.
3. **Échanger** : un `rename` du provisoire vers `hean/`. Les deux dossiers sont dans
   `Application Support`, donc sur le même volume, donc l'opération est atomique — il
   n'existe aucun instant où `hean/` est à moitié là.
4. **Supprimer l'original**, et seulement là.

**Écarté** : le `mv` sec, c'est-à-dire un `rename` de `noir/` vers `hean/`. Instantané,
atomique, zéro espace disque — et sans aucun moyen de revenir si V2 se révèle incapable de
lire ce qu'il a déplacé. L'ADR 0003 l'avait déjà écarté en une phrase ; le mettre en œuvre
confirme la raison.

**Écarté aussi** : garder l'original « au cas où ». Ce serait 283 Mo dupliqués sur le disque
de chaque testeur, pour toujours, et deux dossiers dont personne ne saurait lequel fait foi.

### 2. L'inventaire est explicite : `smb_buffer/` ne se migre pas

La liste de ce qui se migre est écrite (`migration/inventaire.rs`), elle n'est pas « tout le
dossier ». Un `cp -R` aurait copié 289 Mo de cache SMB pour que V2 les supprime au premier
lancement — et si V2 oubliait de les supprimer à son tour, on aurait **transporté le bug de
V1 avec ses données**. Les fichiers cachés (`.DS_Store`) ne se migrent pas non plus : ce ne
sont pas des données de l'application.

Tout le reste suit, `logs/` et `feedback/` compris : les journaux sont lus par « Attach
recent logs » et les retours sont conservés en local exprès.

### 3. Déplacer les fichiers ne suffit pas : la configuration de V1 est **reprise**

Trouvé en écrivant les tests, et c'est le défaut le plus grave de cette phase : **V2 ne lit
aucun JSON de V1.** Une migration qui ne fait que déplacer des octets laisserait donc le
testeur, au premier lancement, devant :

- une bibliothèque **vide** — ses dossiers de musique sont dans `config.json`, que rien ne
  relit, et il devrait tous les rajouter à la main ;
- et **ses pistes supprimées de retour** au premier scan, parce que la liste d'exclusion de
  V1 vit aussi dans ce fichier. C'est **INV-06 (🔴 BLOQUANT) violé à travers la migration** :
  « une piste supprimée ne revient jamais » ne vaut pas que dans une version.

`library/reprise_v1.rs` reprend donc ces deux listes vers la base, **une seule fois**,
garanti par une table `reprises`. Rejouer la reprise à chaque démarrage ressusciterait les
réglages que l'utilisateur a changés depuis dans V2 — typiquement une piste qu'il a cessé
d'exclure.

Ce module ne contient **aucun SQL** : il passe par l'API publique du dépôt
(`ajouter_racine`, `exclure`). Ce n'est pas de l'élégance, c'est INV-06 — un seul fichier au
monde nomme la table des chemins exclus, et le test d'architecture refuse le contraire. Même
les tests composent leur `config.json` à partir des constantes du dépôt plutôt que d'écrire
le nom de la clé en clair : un détecteur qui se mettrait à signaler des données de test
finirait désarmé.

**Ce qui n'est PAS repris, et pourquoi** : les playlists, les favoris et l'historique
d'écoute. Leurs fichiers sont migrés et attendent les phases qui créeront leurs tables ;
inventer leur forme maintenant serait deviner. **La bibliothèque elle-même n'est pas reprise
non plus** : elle se rescanne depuis les dossiers retrouvés, ce qui est plus sûr que de
traduire `tracks_cache.json` dans un schéma différent — et c'est automatique, donc le
testeur ne voit qu'un scan au premier lancement, pas une bibliothèque à reconstruire.

**Mesure sur les données réelles de Thomas** : 1 dossier de bibliothèque repris, 0 exclusion
(sa liste est vide). Un testeur qui aurait supprimé des pistes, lui, les verrait rester
supprimées.

### 4. La vérification est à deux niveaux, et c'est un arbitrage assumé

- **chaque fichier par sa taille** : c'est ce qui attrape une copie tronquée, le cas réel ;
- **les fichiers irremplaçables par leur contenu**, octet par octet : `.json` et `.db`.

Pourquoi pas tout : une taille identique ne prouve pas un contenu identique — un secteur
abîmé ne change pas la longueur d'un fichier — mais comparer les 275 Mo de `covers/` au
contenu coûterait plusieurs secondes de lecture au premier lancement pour protéger des
données **régénérables**. Une pochette perdue se retélécharge ; une playlist perdue ne
revient pas. Les écarts sont rendus **tous ensemble**, et nommés : qui doit comprendre ce qui
est arrivé à ses données a besoin de leur étendue, pas d'un symptôme.

### 5. Aucune estimation de place disque avant de commencer

**Écarté**, et c'est contre-intuitif : un pré-contrôle « reste-t-il 283 Mo ? » ajoute du code
et une façon de se tromper (quel volume ? quelle marge ? et si le disque se remplit entre le
contrôle et la copie ?) sans rien garantir de plus. Un disque plein fait échouer la copie, et
cet échec est traité comme tous les autres : le provisoire est effacé, l'original n'a pas
bougé, le lancement suivant retentera. **Une estimation peut être fausse ; un `ENOSPC` ne
peut pas.**

### 6. Le témoin est la seule source de vérité sur « est-ce déjà passé ? »

`migration-depuis-noir.json`, écrit dans le dossier d'arrivée **avant l'échange** — donc il
fait partie du dossier à l'instant où celui-ci devient `hean/`. L'ordre compte : écrit après,
une interruption entre les deux laisserait une bibliothèque complète que le lancement suivant
croirait non migrée, et qu'il refuserait de toucher en la trouvant « déjà peuplée ».

Il porte le compte des fichiers et des octets, pas seulement un « c'est fait » : un témoin
muet ne permet pas de constater après coup qu'il manque quelque chose.

**Et si `hean/` existe déjà avec des données mais sans témoin**, la migration **ne fait
rien** et le dit. Deux bibliothèques ne se fusionnent pas : ce serait le scénario le plus
destructeur de tous, et le seul irréversible.

### 7. La vérification sur données réelles est un outil à part, hors de `./check`

`tools/verifier-migration-reelle.sh` travaille sur une **copie** du vrai dossier, dans un bac
temporaire, et refuse de désigner le vrai `hean/` comme cible. Il est **délibérément hors de
`./check`** : la CI n'a pas de dossier `noir/`, et une vérification qui ne peut pas tourner là
où elle est lancée ne vérifie rien — elle donnerait seulement l'illusion d'être couverte.
Elle se lance à la main, et son résultat est consigné, comme les mesures audio de la phase 1.

**Résultat du 2026-11-22**, sur les données réelles de Thomas : 2 446 fichiers en entrée,
**2 443 migrés** (291 Mo), 3 ignorés (`smb_buffer/`), témoin écrit, ancien dossier supprimé
dans le bac, `playlists.json`, `listening_history.json`, `network_sources.json` et
`config.json` **identiques à l'original**, reprise de configuration **1 dossier de
bibliothèque / 0 exclusion** — et l'original intact, 2 446 fichiers.

## Conséquences acceptées

- **Les entrées Keychain ne sont pas touchées.** Renommer un dossier ne les déplace pas, et
  le nom de service que V2 emploiera est un point ouvert daté de la phase 3 (ADR 0003). Ne
  rien faire laisse les mots de passe en place ; s'y prendre à moitié les perdrait.
- **Un pic transitoire de ~283 Mo** sur le disque du testeur, le temps de la copie.
- **La migration peut échouer sans empêcher l'application de démarrer.** L'échec est
  journalisé en `error!` et la tentative reprend au lancement suivant. Démarrer sur une
  bibliothèque vide serait pire que démarrer sur l'ancien dossier intact.
- **Un lien symbolique n'est pas suivi** : il pourrait pointer hors du dossier de données.
- **Mesure à garder, parce qu'elle vaut au-delà de la migration** : sur **APFS, un nom de
  fichier en NFD et le même en NFC désignent UN SEUL fichier** — le système compare les noms
  après normalisation tout en conservant les octets qu'on lui donne. Un test qui posait les
  deux écritures de « Café Noir » en attendant deux fichiers a trouvé la seconde écrasant la
  première. Conséquence pour le projet : la normalisation NFC d'INV-01 porte sur les **clés
  d'album**, que nous comparons dans SQLite, et **non sur les chemins**, que le système
  compare pour nous. Deux problèmes qui se ressemblent, un seul des deux est le nôtre.
