# 0007 — Le format interne du pipeline audio, et la preuve du bit-perfect

- **Date** : 2026-11-22
- **État** : Acceptée
- **Décideur** : Thomas
- **Suit** : [0002](0002-stack-technique.md), [0005](0005-entretien-du-harnais-et-orchestration.md), [0006](0006-socle-et-harnais-de-phase-0.md)

## Contexte

L'[ADR 0002](0002-stack-technique.md) acte **CoreAudio direct, jamais CPAL** et
**bit-perfect = aucun rééchantillonnage**. Il ne dit pas sous quelle forme les échantillons
circulent entre le décodeur et la sortie, ni comment on *prouve* que la promesse est tenue.

En V1, « bit-perfect » était une promesse : aucun test ne portait sur la chaîne audio, et
toute vérification passait par un DAC branché et une oreille. Le problème n'est pas la
difficulté, c'est l'asymétrie : **un défaut de bit-perfect ne fait échouer aucun test.** Ça
joue, un peu moins bien. Il peut donc vivre indéfiniment sans que personne le voie — ce qui
est exactement pourquoi le moteur audio est l'une des zones qui ne se délèguent jamais
([`tools/zones-interdites.txt`](../../tools/zones-interdites.txt)).

Le pipeline documenté de V1 décode en **`f32` entrelacé**. Cette décision-là est l'objet de
l'ADR, parce qu'elle commande tout le reste.

## Décisions

### 1. Le format interne est `i32` mis à l'échelle sur 32 bits, et non `f32`

Un échantillon circule dans la chaîne sous la forme de sa valeur native **décalée à gauche**
pour occuper toute la plage de `i32` : `i32 = natif << (32 - profondeur)`. Le retour est le
décalage arithmétique inverse.

`f32` n'était pas faux : il représente exactement tout entier jusqu'à 2²⁴, donc 16 et 24 bits
y survivent intacts. Mais il faut **le démontrer**, en raisonnant sur la mantisse, et la
démonstration tombe au premier fichier 32 bits entier. Le décalage, lui, est exact par
construction, se vérifie d'un coup d'œil, et son aller-retour est testé de façon
**exhaustive** en 16 bits — les 65 536 valeurs, pas un échantillonnage.

Et il tombe sur la convention de symphonia : ses tampons `S32` portent déjà l'échantillon
décalé. Le chemin FLAC — celui du bit-perfect — ne fait donc **aucune conversion du tout**.

**Écarté** : `f32` interne comme en V1 (il faut prouver l'exactitude au lieu de la
constater). **Écarté aussi** : garder les échantillons dans leur profondeur natale avec un
type par profondeur (trois chemins de code à maintenir pour supprimer un décalage).

### 2. Le bypass n'est pas une optimisation : c'est le chemin bit-perfect

En V1 l'égaliseur était « bypassé si tous les gains ≈ 0 dB », présenté comme un gain de
performance. C'est en réalité la seule chose qui fasse du bit-perfect autre chose qu'un
espoir : quand la chaîne est transparente, les échantillons ne sont ni copiés, ni convertis,
ni multipliés. Il n'y a aucune occasion d'arrondi **parce qu'il n'y a aucune opération**.

D'où le refus de toute tolérance : le volume doit valoir **exactement** 1,0. Un « 99,9999 %
≈ 100 % » ferait prendre le chemin transparent pour un réglage que l'utilisateur a réellement
modifié — l'app annoncerait bit-perfect en ignorant le curseur. C'est aussi ce qui s'explique
le plus simplement : bit-perfect si, et seulement si, le volume est à 100 %.

### 3. La référence est produite par un décodeur indépendant

Le test compare la sortie **octet par octet** à un fichier commité que **ffmpeg** a écrit.
Comparer notre sortie à elle-même ne prouverait rien ; la comparer à celle d'un tiers prouve
qu'aucun bit ne change entre le fichier et la sortie.

Conséquence : notre WAV doit avoir exactement la forme de celui d'ffmpeg, donc
`WAVE_FORMAT_EXTENSIBLE` au-delà de 16 bits (en-tête de 68 octets) et PCM canonique en
dessous (44 octets). Ce n'est pas un détail cosmétique : écrire un en-tête canonique en
24 bits obligerait à comparer les flux en sautant les en-têtes, c'est-à-dire à écrire
soi-même la logique dont dépend le test, et à ne plus rien prouver d'indépendant.

**Écarté** : un golden produit par notre propre code et relu « à l'oreille » une fois
(c'est ce que le README de phase 0 envisageait ; ça prouve la stabilité, pas la justesse).

### 4. Les fixtures sont commitées, pas régénérées

L'encodeur FLAC d'ffmpeg n'est pas garanti reproductible bit à bit d'une version à l'autre.
Une fixture régénérée en CI ferait échouer la comparaison pour une raison fausse — et un test
qui échoue pour une raison fausse finit par être désarmé. Les 24 fixtures pèsent 2 Mo en
tout ; [`tools/check-fixtures-audio.sh`](../../tools/check-fixtures-audio.sh) vérifie dans
les deux sens qu'aucune ne manque, qu'aucun binaire n'est non documenté, et **qu'aucune
fixture n'est ignorée de tous les tests** — une fixture que personne ne lit n'est pas une
fixture, c'est du poids mort.

### 5. Le trait de backend ne porte aucune méthode de choix de périphérique

La tentation serait un `set_device(id) -> Result<()>`. Ce serait un piège : d'après **PLAT-1**
(🔴), un périphérique AirPlay n'existe dans CoreAudio que tant qu'il est le défaut système, et
`set_system_default_device` sur un périphérique périmé **accepte silencieusement sans rien
faire**. Un setter qui rend `Ok(())` alors que rien n'a eu lieu est précisément la dégradation
invisible que ce projet refuse. La stratégie de préservation AirPlay n'est pas exprimable
comme un setter ; elle appartient au backend CoreAudio, pas à une interface que le backend
WAV devrait faire semblant d'implémenter.

En revanche `preparer` rend le format **réellement obtenu** et non `()`. Un `Ok(())` laisserait
croire qu'un périphérique joue à 192 kHz alors qu'il peut accepter la demande et tourner à
48 kHz. Rendre le format obtenu permet de **constater** l'écart, donc de le refuser — puisque
le bit-perfect interdit de rééchantillonner.

### 6. Un paquet indécodable fait échouer le décodage ; il n'est pas sauté

Sauter produirait du bruit en silence. Le décodage s'arrête et remonte une erreur nommée.

### 7. Quand le conteneur ne déclare pas le format, il se déduit du premier paquet

**Mesuré** : ALAC et AAC ne déclarent pas leur nombre de canaux
(`codec_params.channels` vaut `None`). L'information n'existe qu'après le décodage d'un
premier paquet. Refuser ces fichiers serait perdre deux des sept formats annoncés ; deviner
« stéréo » serait jouer un mono sur un seul canal sans le dire. Le décodeur décode donc un
premier paquet et **garde son audio** pour le rendre au premier appel suivant.

De même, une profondeur absente (formats avec perte, ALAC) retombe sur une valeur
**explicite et documentée**, 24 bits, et non sur un défaut implicite : un fichier avec perte
n'a pas d'échantillon d'origine à préserver, donc la question de la fidélité ne se pose pas.

## Conséquences acceptées

- **La convention d'échelle de symphonia est un détail d'implémentation, pas un contrat.**
  Elle n'est écrite dans aucune garantie de version. Une mise à jour pourrait la changer, et
  le son sortirait alors 65 536 fois trop faible **sans qu'aucun test de lecture n'échoue**.
  Mitigation : elle est isolée dans un seul fichier
  ([`convert.rs`](../../src-tauri/src/audio/convert.rs)) et un test nommé
  `la_convention_dechelle_de_symphonia_est_celle_quon_croit` échoue avec un message qui dit
  quoi corriger, au lieu de laisser le test bit-perfect annoncer « les octets diffèrent ».

- **L'intégrité d'un fichier audio ne se déduit pas du décodeur.** Mesuré, et contre
  l'attente : sur les trois façons dont un fichier s'abîme réellement, **une seule produit
  une erreur**. Un en-tête STREAMINFO incohérent est refusé ; mais des trames abîmées ou un
  fichier tronqué passent en **silence** — symphonia saute les trames invalides, et un seul
  octet abîmé suffit à faire disparaître un bloc entier, 4 608 trames, un dixième de seconde.
  Le README des fixtures de phase 0 annonçait l'inverse. Les trois comportements sont
  désormais fixés par trois tests. Il faudra vérifier l'intégrité autrement — la durée
  annoncée par le conteneur contre le nombre de trames réellement rendues — le jour où la
  lecture depuis le NAS arrivera (phase 5), parce que c'est là que le cas devient courant.

- **2 Mo de binaires dans le dépôt**, dont 750 Ko de goldens. C'est le prix du 24/192 : son
  golden pèse 576 Ko pour une demi-seconde. Un budget les plafonne à 4 Mo.

- **L'égaliseur n'est pas implémenté.** Les fréquences de ses huit bandes sont une donnée de
  parité avec V1 que le bootstrap ne contient pas, et les inventer créerait exactement l'écart
  silencieux que la reconstruction cherche à éviter. `est_transparente()` ne couvre donc que
  le volume aujourd'hui, et devra exiger **aussi** que l'égaliseur soit plat quand il
  arrivera. C'est écrit dans [`chain.rs`](../../src-tauri/src/audio/chain.rs).

- **`decoder_entierement` charge tout en mémoire.** C'est une commodité de test, pas le
  chemin de lecture : le moteur consommera `Decodeur::suivant()`, qui rend des blocs, et un
  test vérifie qu'il en rend bien plusieurs.

- **L'encodeur Vorbis d'ffmpeg est marqué expérimental** (`-strict -2`), `libvorbis` n'étant
  pas toujours compilé. Pour une fixture de **décodage**, le flux produit est valide.
