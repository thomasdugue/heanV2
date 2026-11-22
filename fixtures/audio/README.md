# Fixtures audio

L'actif le plus réutilisable du projet. Quelques secondes chacune, versionnées, **2 Mo en
tout**. Chaque fixture correspond à **un piège réel de V1** — c'est ce qui justifie sa
présence, et `tools/check-fixtures-audio.sh` refuse qu'une fixture existe sans être citée
par un test. Un binaire commité que personne ne lit n'est pas une fixture, c'est du poids
mort.

> **Ce tableau est lu par une machine.** Le script d'inventaire en extrait chaque nom de
> fichier entre accents graves, dans la première colonne, et vérifie l'accord dans les deux
> sens : aucune fixture manquante, aucun fichier non documenté. Garder le format tel quel.

## Bit-perfect — les deux fixtures centrales

| Fixture | Pourquoi |
|---|---|
| `flac-16-44.flac` | 1 s, 16 bits / 44,1 kHz. La référence du test octet par octet. |
| `flac-24-192.flac` | 0,5 s, 24 bits / 192 kHz. Le cas d'usage central, et celui où une troncature à 16 bits serait inaudible sur la plupart des morceaux. |

Les deux portent un signal **à canaux différents** : gauche un sinus à 997 Hz, droite un
sinus à 1 327 Hz mêlé de bruit à graine fixe. Un signal où les deux canaux seraient
identiques ne ferait échouer ni une inversion gauche/droite, ni un entrelacement raté ; et
sans le bruit, les bits de poids faible resteraient à zéro et une troncature passerait
inaperçue.

## Un fichier par format annoncé comme supporté

| Fixture | Pourquoi |
|---|---|
| `wav.wav` | PCM nu — et le seul format qui soit à la fois entrée et sortie du harnais. |
| `aiff.aiff` | PCM gros-boutiste : l'ordre des octets est inversé par rapport au WAV. |
| `alac.m4a` | Sans perte, mais **ne déclare pas ses canaux** dans le conteneur (voir plus bas). |
| `mp3-320.mp3` | Avec perte, décodé en flottant. |
| `aac.m4a` | Avec perte, et ne déclare pas ses canaux non plus. |
| `vorbis.ogg` | Avec perte, et son premier paquet Ogg est **vide** — un décodeur naïf y voit une fin de fichier. |

WMA, DSD, Opus et MQA ne sont pas supportés : les features correspondantes sont absentes de
`Cargo.toml`, ce qui rend l'absence de support vérifiable au lieu d'accidentelle.

## Signaux connus — pour le volume et l'égaliseur

| Fixture | Pourquoi |
|---|---|
| `sine-997.flac` | Sinus pur : permet des assertions sur le spectre. 997 Hz n'est harmonique ni de 44 100 ni de 192 000. |
| `silence.flac` | Exactement nul. Tout traitement qui le rend non nul ajoute du bruit. |
| `dc.flac` | Constante continue : révèle un filtre qui déplace le zéro. |

## Gapless

| Fixture | Pourquoi |
|---|---|
| `gapless-a.flac` | Première moitié de `flac-16-44.flac`, coupée à l'échantillon exact. |
| `gapless-b.flac` | Seconde moitié. Concaténées, les deux redonnent **exactement** le fichier entier : toute discontinuité à la jonction vient donc du moteur, pas des fixtures. |

La paire n'a pas de golden propre : la référence de la jonction **est**
`golden/flac-16-44.wav`. Deux goldens identiques seraient deux sources de vérité pour un
même fait.

## Pièges de métadonnées

| Fixture | Pourquoi |
|---|---|
| `tags-nfd.flac` | Album et artiste en **NFD** (« Café » écrit `e` + accent combinant), titre en NFC dans le même fichier. La clé d'album doit normaliser (INV-01). |
| `artist-feat.flac` | Artiste « Zoé Léger feat. Hean », même album que la précédente. Dédupliquer par `artiste + album` recréerait un faux album : régression W003. |
| `accents-éàü.flac` | Accents dans les tags **et** dans le nom de fichier. |
| `no-cover.flac` | Aucune pochette : déclenche le 4ᵉ niveau du pipeline de covers (INV-12). |
| `huge-cover.flac` | Pochette 3000 × 3000 embarquée. Elle ajoute une piste image au conteneur — `default_track()` pourrait la prendre pour l'audio. |
| `empty-tags.flac` | Aucun tag : le repli sur le nom de fichier doit marcher, et l'absence de métadonnées ne doit pas empêcher de jouer. |

## Fichiers abîmés — trois cas, trois comportements

Mesuré sur symphonia, et **contre l'attente** : sur les trois façons dont un fichier audio
s'abîme réellement, **une seule produit une erreur**. La version phase 0 de ce document
annonçait « doit produire une erreur propre » pour la troncature — c'était faux, et personne
ne pouvait le savoir avant d'essayer.

| Fixture | Pourquoi |
|---|---|
| `corrupt-entete.flac` | Bloc STREAMINFO retourné bit à bit. **Refusé dès la sonde**, avec un message : le seul des trois qui échoue. |
| `corrupt-trames.flac` | 400 octets retournés au milieu des trames. **Aucune erreur** : symphonia saute les trames invalides. Un seul octet abîmé suffit à faire disparaître un bloc entier — 4 608 trames, un dixième de seconde — en silence. |
| `corrupt-tronque.flac` | Coupé au tiers, comme une copie interrompue depuis le NAS. **Aucune erreur** : le fichier joue plus court, et rien ne le signale. |

**Conséquence de conception : l'intégrité d'un fichier ne se déduit pas du décodeur.** Elle
devra se vérifier autrement — la durée annoncée par le conteneur contre le nombre de trames
réellement rendues — le jour où la lecture depuis le NAS arrivera.

## Goldens

| Référence | Pourquoi |
|---|---|
| `golden/flac-16-44.wav` | Sortie attendue en 16 bits. En-tête PCM canonique de 44 octets. |
| `golden/flac-24-192.wav` | Sortie attendue en 24 bits. En-tête `WAVE_FORMAT_EXTENSIBLE` de 68 octets — au-delà de 16 bits, un WAV ne s'écrit pas en PCM canonique. |

Les goldens sont produits par **ffmpeg**, donc par un décodeur FLAC et un encodeur WAV
indépendants des nôtres. C'est ce qui donne sa force au test : comparer notre sortie à
elle-même ne prouverait rien.

Un test qui échoue sur un golden est soit une régression, soit un changement voulu — dans ce
second cas le golden est régénéré **dans le même commit**, pour que le diff montre ce qui a
changé dans le son.

## Fabrication

```bash
./tools/make-fixtures-audio.sh
```

Script **one-shot** : les fixtures sont commitées, pas régénérées à chaque `./check`.
L'encodeur FLAC d'ffmpeg n'est pas garanti reproductible bit à bit d'une version à l'autre,
et une fixture régénérée en CI ferait échouer la comparaison octet par octet pour une raison
fausse — or un test qui échoue pour une raison fausse finit par être désarmé. Fabriquées avec
ffmpeg 8.0.1.

Le signal est synthétique plutôt que de la vraie musique : aucune question de droits, et un
signal connu permet des assertions sur le spectre et le RMS. Détail de chaque commande dans
le script ; les trois variantes de fichier abîmé sont produites par
[`../../tools/abimer-fixture.py`](../../tools/abimer-fixture.py).

Pourquoi ces fixtures existent, et ce qu'elles remplacent :
[`../../bootstrap/06-setup-v2/HARNAIS.md`](../../bootstrap/06-setup-v2/HARNAIS.md) §1.
