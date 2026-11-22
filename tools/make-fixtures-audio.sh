#!/usr/bin/env bash
# Fabrique les fixtures audio de fixtures/audio/. Script ONE-SHOT : les fixtures sont
# COMMITÉES, pas régénérées à chaque passage de ./check.
#
# Pourquoi commiter des binaires plutôt que de les régénérer : l'encodeur FLAC d'ffmpeg
# n'est pas garanti reproductible bit-à-bit d'une version à l'autre. Une fixture régénérée
# en CI ferait échouer la comparaison octet par octet pour une raison fausse — et un test
# qui échoue pour une raison fausse finit par être désarmé. Les fixtures sont donc figées,
# et tools/check-fixtures-audio.sh vérifie seulement qu'elles sont toutes là.
#
# Le signal est synthétique (sinus + bruit à graine fixe) et non de la vraie musique :
# aucune question de droits, et un signal connu permet des assertions sur le spectre.
#
#   ./tools/make-fixtures-audio.sh          refabrique tout
#
# Prérequis : ffmpeg (fabriqué avec la 8.0.1). Les goldens sont produits par ffmpeg, donc
# par un décodeur INDÉPENDANT du nôtre : c'est ce qui donne sa force au test bit-perfect.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
OUT=fixtures/audio
GOLD=$OUT/golden
mkdir -p "$GOLD"
command -v ffmpeg >/dev/null || { echo "ffmpeg absent"; exit 1; }
Q="-hide_banner -loglevel error -y"
# -fflags +bitexact -map_metadata -1 : aucun tag « encoder », sinon le fichier changerait
# à chaque mise à jour d'ffmpeg sans que le son change.
EXACT="-fflags +bitexact -map_metadata -1"

# --- le signal de référence -------------------------------------------------------------
# Canaux VOLONTAIREMENT différents : gauche 997 Hz (fréquence de test audio standard, non
# harmonique de 44 100 ni de 192 000), droite 1 327 Hz + bruit à -34 dBFS. Un signal où
# L == R ne ferait échouer ni une inversion de canaux ni un entrelacement raté.
# Le bruit sert à exercer les bits de poids faible : sans lui, un sinus pur laisse les
# derniers bits à zéro et une troncature 24 → 16 bits passerait inaperçue.
signal() { # $1 = rate, $2 = durée
  printf -- '-f lavfi -i sine=frequency=997:sample_rate=%s:duration=%s ' "$1" "$2"
  printf -- '-f lavfi -i sine=frequency=1327:sample_rate=%s:duration=%s ' "$1" "$2"
  printf -- '-f lavfi -i anoisesrc=sample_rate=%s:duration=%s:amplitude=0.02:seed=1789:color=white ' "$1" "$2"
}
MIX='[0:a]volume=0.7[l];[1:a]volume=0.5[r0];[r0][2:a]amix=inputs=2:normalize=0[r];[l][r]join=inputs=2:channel_layout=stereo[out]'

stereo() { # $1=rate $2=durée $3=sample_fmt $4=sortie
  # shellcheck disable=SC2046
  ffmpeg $Q $(signal "$1" "$2") -filter_complex "$MIX" -map '[out]' \
    -c:a flac -sample_fmt "$3" $EXACT "$4"
}
mono() { # $1=durée $2=sortie  (+ arguments de métadonnées passés ensuite)
  local d=$1 out=$2; shift 2
  ffmpeg $Q -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=$d" \
    -c:a flac -sample_fmt s16 $EXACT "$@" "$out"
}

echo "→ bit-perfect (les deux fixtures centrales)"
# 1 s en 16/44.1 : le golden pèse 176 Ko. 0,5 s en 24/192 : le golden pèse 576 Ko — à
# 1 s il en pèserait 1,2 Mo pour ne rien prouver de plus (0,5 s = 24 paquets FLAC).
stereo 44100  1.0 s16 "$OUT/flac-16-44.flac"
stereo 192000 0.5 s32 "$OUT/flac-24-192.flac"

echo "→ goldens (produits par ffmpeg : un décodeur indépendant du nôtre)"
ffmpeg $Q -i "$OUT/flac-16-44.flac"  -c:a pcm_s16le $EXACT "$GOLD/flac-16-44.wav"
ffmpeg $Q -i "$OUT/flac-24-192.flac" -c:a pcm_s24le $EXACT "$GOLD/flac-24-192.wav"

echo "→ un fichier par format supporté (0,5 s : prouver le décodage ne demande pas plus)"
SRC="$OUT/flac-16-44.flac"
ffmpeg $Q -i "$SRC" -t 0.5 -c:a pcm_s16le  $EXACT "$OUT/wav.wav"
ffmpeg $Q -i "$SRC" -t 0.5 -c:a pcm_s16be  $EXACT "$OUT/aiff.aiff"
ffmpeg $Q -i "$SRC" -t 0.5 -c:a alac       $EXACT "$OUT/alac.m4a"
ffmpeg $Q -i "$SRC" -t 0.5 -c:a libmp3lame -b:a 320k $EXACT "$OUT/mp3-320.mp3"
ffmpeg $Q -i "$SRC" -t 0.5 -c:a aac        -b:a 256k $EXACT "$OUT/aac.m4a"
# L'encodeur Vorbis natif d'ffmpeg est marqué expérimental (-strict -2) : libvorbis n'est
# pas toujours compilé. Pour une fixture de DÉCODAGE, le flux produit est valide.
ffmpeg $Q -i "$SRC" -t 0.5 -c:a vorbis -strict -2 -q:a 6 $EXACT "$OUT/vorbis.ogg"

echo "→ paire gapless (coupée à l'échantillon exact dans un même signal continu)"
# La propriété testable : concaténer A et B redonne EXACTEMENT le signal d'origine. Toute
# discontinuité à la jonction vient donc de notre chaîne, pas des fixtures.
ffmpeg $Q -i "$SRC" -t 0.5          -c:a flac -sample_fmt s16 $EXACT "$OUT/gapless-a.flac"
ffmpeg $Q -i "$SRC" -ss 0.5 -t 0.5  -c:a flac -sample_fmt s16 $EXACT "$OUT/gapless-b.flac"
# Pas de golden propre à la paire : A et B sont découpés dans flac-16-44.flac, donc la
# référence de la jonction EST golden/flac-16-44.wav. Deux goldens identiques seraient deux
# sources de vérité pour un même fait — exactement ce que ce projet cherche à éviter.

echo "→ signaux connus pour l'EQ et le volume"
# HARNAIS.md §1 : « l'EQ, le resampler et le volume se testent sur des signaux connus
# (sinus, silence, DC) avec des assertions sur le spectre ou le RMS ».
ffmpeg $Q -f lavfi -i "sine=frequency=997:sample_rate=44100:duration=0.5" \
  -c:a flac -sample_fmt s16 $EXACT "$OUT/sine-997.flac"
ffmpeg $Q -f lavfi -i "anullsrc=sample_rate=44100:channel_layout=mono" -t 0.5 \
  -c:a flac -sample_fmt s16 $EXACT "$OUT/silence.flac"
# DC : une constante. `aeval` donne une valeur fixe sur toute la durée.
ffmpeg $Q -f lavfi -i "aevalsrc=0.5:sample_rate=44100:duration=0.5:channel_layout=mono" \
  -c:a flac -sample_fmt s16 $EXACT "$OUT/dc.flac"

echo "→ pièges de métadonnées (chacun est un bug réel de V1)"
# NFD : « Café » écrit e + U+0301 et non é. La clé d'album doit normaliser en NFC (INV-01).
NFD_ALBUM=$(python3 -c 'import unicodedata;print(unicodedata.normalize("NFD","Café Noir"))')
NFD_ARTIST=$(python3 -c 'import unicodedata;print(unicodedata.normalize("NFD","Zoé Léger"))')
mono 0.5 "$OUT/tags-nfd.flac" -metadata album="$NFD_ALBUM" -metadata artist="$NFD_ARTIST" -metadata title="Première"
# « feat. » : la régression W003 créait un album distinct par variation d'artiste.
mono 0.5 "$OUT/artist-feat.flac" -metadata album="Café Noir" \
  -metadata artist="Zoé Léger feat. Hean" -metadata title="Deuxième"
# Accents en NFC dans les tags ET dans le nom de fichier.
mono 0.5 "$OUT/accents-éàü.flac" -metadata album="Été" -metadata artist="Noël" -metadata title="Où ça ?"
# Aucune cover : déclenche le 4ᵉ niveau du pipeline de covers (INV-12).
mono 0.5 "$OUT/no-cover.flac" -metadata album="Sans Pochette" -metadata artist="Hean"
# Aucun tag du tout : le fallback sur le nom de fichier doit marcher.
mono 0.5 "$OUT/empty-tags.flac"

echo "→ cover 3000×3000 embarquée (génération de thumbnail)"
TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
ffmpeg $Q -f lavfi -i "gradients=s=3000x3000:n=3:seed=7:d=1" -frames:v 1 -q:v 6 "$TMP/cover.jpg"
mono 0.5 "$TMP/base.flac" -metadata album="Grande Pochette" -metadata artist="Hean"
ffmpeg $Q -i "$TMP/base.flac" -i "$TMP/cover.jpg" -map 0:a -map 1:v \
  -c:a copy -c:v copy -disposition:v attached_pic $EXACT "$OUT/huge-cover.flac"

echo "→ fichiers abîmés — TROIS cas, qui ne se comportent pas pareil"
# Mesuré sur symphonia, et contre l'attente : seul un en-tête abîmé produit une erreur.
# Des trames abîmées ou un fichier tronqué passent en SILENCE. Le détail est en tête de
# tools/abimer-fixture.py, et chaque comportement est fixé par un test.
python3 "$(dirname "${BASH_SOURCE[0]}")/abimer-fixture.py" "$SRC" \
  "$OUT/corrupt-entete.flac" "$OUT/corrupt-trames.flac" "$OUT/corrupt-tronque.flac"

echo
echo "fixtures produites :"
du -ch "$OUT"/* "$GOLD"/* 2>/dev/null | tail -1
ls -1 "$OUT" "$GOLD"
