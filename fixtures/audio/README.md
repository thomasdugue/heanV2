# Fixtures audio

L'actif le plus réutilisable du projet. Quelques secondes chacune, versionnées.
Chaque fixture correspond à **un bug réel de V1** — c'est ce qui justifie sa présence.

| Fixture | Pourquoi |
|---|---|
| `flac-24-192.flac` | bit-perfect haute résolution : le cas d'usage central |
| `flac-16-44.flac` | changement de sample rate entre deux pistes |
| `alac.m4a` · `wav.wav` · `aiff.aiff` · `mp3-320.mp3` · `aac.m4a` · `vorbis.ogg` | un par format supporté |
| `gapless-a.flac` + `gapless-b.flac` | paire conçue pour qu'une discontinuité à la jonction soit audible et mesurable |
| `tags-nfd.flac` | métadonnées en NFD — la clé d'album doit normaliser en NFC (invariant 1) |
| `artist-feat.flac` | artiste « X feat. Y » — ne doit pas créer un album distinct (régression W003) |
| `accents.flac` | caractères accentués dans les tags et le nom de fichier |
| `no-cover.flac` | déclenche le fallback internet du pipeline de covers (niveau 4) |
| `huge-cover.flac` | cover 3000×3000 embarquée : génération de thumbnail |
| `empty-tags.flac` | aucun tag : le fallback sur le nom de fichier doit marcher |
| `corrupt.flac` | fichier tronqué : doit produire une erreur propre, pas un panic |

## Génération

Les fixtures ne sont pas encore produites. Elles se fabriquent avec `ffmpeg` à partir d'un signal de
test (sinus, silence) plutôt qu'avec de la vraie musique — pas de question de droits, et un signal
connu permet des assertions sur le spectre et le RMS.

Le script de génération est à écrire en phase 1, en même temps que le backend audio de test
(voir [`../../bootstrap/06-setup-v2/HARNAIS.md`](../../bootstrap/06-setup-v2/HARNAIS.md), §1).

## Golden files

Les sorties de référence (`golden/`) sont produites **une fois**, vérifiées à l'oreille et au
spectre, puis figées. Un test qui échoue sur un golden file est soit une régression, soit un
changement voulu — dans ce second cas, le golden est régénéré **dans le même commit**, pour que le
diff montre ce qui a changé dans le son.
