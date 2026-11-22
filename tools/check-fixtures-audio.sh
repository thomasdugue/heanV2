#!/usr/bin/env bash
# APPARIEMENT (niveau 2) — l'inventaire des fixtures audio.
#
# Trois accords, dans les deux sens, parce que chacun s'est déjà défait quelque part :
#
#   1. README ↔ disque. Une fixture documentée mais absente fait échouer un test avec un
#      « fichier introuvable » au lieu d'un message utile. Un fichier présent mais non
#      documenté est un binaire dont plus personne ne sait à quoi il sert — et V1 a montré
#      ce que coûte une documentation qu'on ne peut pas croire.
#   2. Fixture ↔ test. Une fixture qu'aucun test ne lit n'est pas une fixture, c'est du
#      poids mort dans le dépôt. Le harnais ne doit pas accumuler de décor.
#   3. Bornes de poids. « Quelques secondes chacune » : une fixture qui grossit est une
#      fixture qu'on a remplacée par de la vraie musique sans le dire.
#
# Volontairement SANS ffprobe : la CI est un seul job macOS qui lance ./check, et rien ne
# garantit qu'ffmpeg y soit installé. Une vérification qui ne tourne pas en CI ne vérifie
# rien. La taille en octets suffit à voir une dérive.
set -uo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dossier="$root/fixtures/audio"
readme="$dossier/README.md"
tests="$root/src-tauri/tests"
fail=0

MIN_OCTETS=1000        # en dessous, le fichier ne contient pas d'audio
MAX_OCTETS=2000000     # au-dessus, ce n'est plus « quelques secondes »
MAX_TOTAL_KO=4096      # le dossier entier, goldens compris

[ -d "$dossier" ] || { echo "  fixtures/audio/ absent"; exit 1; }
[ -f "$readme" ]  || { echo "  fixtures/audio/README.md absent"; exit 1; }

# Les noms documentés : premier champ entre accents graves d'une ligne de tableau.
documentes=$(grep -oE '^\| `[^`]+`' "$readme" | sed 's/^| `//;s/`$//' | sort -u)
# Les fichiers réellement présents, chemins relatifs à fixtures/audio/.
presents=$(cd "$dossier" && find . -type f ! -name 'README.md' | sed 's|^\./||' | sort)

n_doc=$(printf '%s\n' "$documentes" | grep -c . || true)
[ "$n_doc" -ge 20 ] || { echo "  seulement $n_doc fixtures documentées, au moins 20 attendues"; fail=1; }

# 1a. Documentée mais absente.
while IFS= read -r nom; do
  [ -n "$nom" ] || continue
  [ -f "$dossier/$nom" ] || { echo "  documentée mais absente : $nom"; fail=1; }
done <<< "$documentes"

# 1b. Présente mais non documentée.
while IFS= read -r nom; do
  [ -n "$nom" ] || continue
  printf '%s\n' "$documentes" | grep -qxF "$nom" \
    || { echo "  présente mais absente du README : $nom"; fail=1; }
done <<< "$presents"

# 2. Chaque fixture est citée par au moins un test. Les goldens sont cités par leur nom de
#    base (`golden/` est reconstruit dans le code), d'où le dépouillement du préfixe.
if [ -d "$tests" ]; then
  while IFS= read -r nom; do
    [ -n "$nom" ] || continue
    grep -rqF "${nom#golden/}" "$tests" \
      || { echo "  aucun test ne lit : $nom (poids mort, ou test oublié)"; fail=1; }
  done <<< "$presents"
else
  echo "  src-tauri/tests/ absent : impossible de vérifier que les fixtures servent"; fail=1
fi

# 3. Bornes de poids.
while IFS= read -r nom; do
  [ -n "$nom" ] || continue
  o=$(stat -f %z "$dossier/$nom" 2>/dev/null || echo 0)
  if [ "$o" -lt "$MIN_OCTETS" ]; then
    echo "  $nom ne pèse que $o octets : contient-il vraiment de l'audio ?"; fail=1
  elif [ "$o" -gt "$MAX_OCTETS" ]; then
    echo "  $nom pèse $o octets (plafond $MAX_OCTETS) : « quelques secondes » a dérivé"; fail=1
  fi
done <<< "$presents"

total=$(du -sk "$dossier" | awk '{print $1}')
if [ "$total" -gt "$MAX_TOTAL_KO" ]; then
  echo "  fixtures/audio/ pèse ${total} Ko (budget ${MAX_TOTAL_KO} Ko)"
  echo "  → raccourcir les fixtures, ou relever le budget dans un ADR en justifiant"
  fail=1
fi

n=$(printf '%s\n' "$presents" | grep -c . || true)
if [ "$fail" -eq 0 ]; then
  echo "check-fixtures-audio : OK ($n fixtures, ${total} Ko, toutes documentées et lues)"
else
  echo "check-fixtures-audio : ÉCHEC"
fi
exit $fail
