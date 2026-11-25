#!/usr/bin/env bash
# La vérification que l'ADR 0003 exige : la migration, sur une COPIE RÉELLE d'un dossier
# `noir/` complet, et pas sur des fixtures.
#
# « Une migration non testée est une perte de données chez 50 personnes. Elle se teste sur
# une copie réelle d'un dossier noir/ complet, pas sur des fixtures : c'est la seule façon
# de rencontrer les cas tordus (chemins NFD, fichiers de cache corrompus, dossier
# partiellement écrit). » — ADR 0003, conséquences acceptées.
#
# Ce script est DÉLIBÉRÉMENT hors de `./check` : il ne tourne pas en CI, parce que la CI n'a
# pas de dossier `noir/`, et une vérification qui ne peut pas tourner là où elle est lancée
# ne vérifie rien. Il se lance à la main, une fois, et son résultat est consigné dans
# `docs/HISTORY.md` — comme les mesures audio de la phase 1.
#
# TROIS GARDE-FOUS, parce que ce script manipule les vraies données de quelqu'un :
#   1. il travaille sur une COPIE, dans un dossier temporaire ;
#   2. il refuse de désigner comme cible le vrai dossier `hean/` ;
#   3. il ne touche JAMAIS à l'original — il ne fait que le lire.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

SOURCE_REELLE="${1:-$HOME/Library/Application Support/noir}"
BAC="$(mktemp -d "${TMPDIR:-/tmp}/hean-migration-reelle.XXXXXX")"
trap 'rm -rf "$BAC"' EXIT

if [ ! -d "$SOURCE_REELLE" ]; then
  echo "pas de dossier V1 à $SOURCE_REELLE — rien à vérifier ici."
  echo "→ relancer ce script sur une machine qui en a un, ou lui passer un chemin en argument."
  exit 2
fi
case "$SOURCE_REELLE" in
  *"/Application Support/hean") echo "refus : ça, c'est le dossier de V2."; exit 1 ;;
esac

echo "=== source réelle : $SOURCE_REELLE"
du -sh "$SOURCE_REELLE" | awk '{print "    poids  : " $1}'
find "$SOURCE_REELLE" -type f | wc -l | awk '{print "    fichiers : " $1}'

echo "=== copie dans le bac (l'original n'est pas touché)"
cp -Rp "$SOURCE_REELLE" "$BAC/noir" || { echo "la copie de travail a échoué"; exit 1; }
AVANT_FICHIERS=$(find "$BAC/noir" -type f | wc -l | tr -d ' ')
AVANT_OCTETS=$(find "$BAC/noir" -type f -exec stat -f %z {} + | awk '{s+=$1} END {print s+0}')
IGNORES=$(find "$BAC/noir/smb_buffer" -type f 2>/dev/null | wc -l | tr -d ' ')

echo "=== migration du bac : $BAC/noir → $BAC/hean"
cargo run --quiet --locked --manifest-path src-tauri/Cargo.toml --example migrer -- \
  "$BAC/noir" "$BAC/hean" || { echo "la migration a échoué"; exit 1; }

echo "=== constat"
# Ni le témoin ni la base ne sont des fichiers MIGRÉS : le premier est écrit par la
# migration, la seconde par la reprise de configuration qui suit.
APRES_FICHIERS=$(find "$BAC/hean" -type f ! -name 'migration-depuis-noir.json' ! -name 'library.db' | wc -l | tr -d ' ')
echo "    avant  : $AVANT_FICHIERS fichiers, $AVANT_OCTETS octets (dont $IGNORES ignorés volontairement)"
echo "    après  : $APRES_FICHIERS fichiers"
fail=0
[ "$APRES_FICHIERS" -eq "$((AVANT_FICHIERS - IGNORES))" ] || { echo "    ÉCART de compte"; fail=1; }
# INV-26 / ADR 0014 : la source DOIT survivre. Cette ligne vérifiait l'inverse jusqu'au
# 2026-11-25 — « l'ancien dossier n'a pas été supprimé » était une faute, c'est devenu la règle,
# sur décision de Thomas (« Oui, garde-le »). Et on vérifie qu'il est intact, pas seulement là.
[ -d "$BAC/noir" ] || { echo "    l'ancien dossier a été supprimé : INV-26 est violé"; fail=1; }
[ -f "$BAC/noir/playlists.json" ] || { echo "    l'ancien dossier a été vidé"; fail=1; }
[ -f "$BAC/hean/migration-depuis-noir.json" ] || { echo "    témoin absent"; fail=1; }

# La reprise a-t-elle ramené les dossiers de bibliothèque ? Sans eux, le testeur migré
# trouve une bibliothèque vide (ADR 0010).
RACINES=$(grep -o '"[^"]*"' "$SOURCE_REELLE/config.json" 2>/dev/null | wc -l | tr -d ' ')
[ -f "$BAC/hean/library.db" ] && echo "    base créée, configuration reprise (voir la ligne Reprise ci-dessus)" || { echo "    pas de base : la reprise n'a pas tourné"; fail=1; }
echo "    ($RACINES chaînes dans le config.json d'origine)"

# Les fichiers qui ne se reconstruisent pas : comparés à l'original, pas à la copie.
for f in playlists.json listening_history.json network_sources.json config.json; do
  if [ -f "$SOURCE_REELLE/$f" ]; then
    if cmp -s "$SOURCE_REELLE/$f" "$BAC/hean/$f"; then
      echo "    $f : identique à l'original"
    else
      echo "    $f : DIFFÉRENT de l'original"; fail=1
    fi
  fi
done

echo "=== l'original, après coup"
[ -d "$SOURCE_REELLE" ] && echo "    toujours là : $(find "$SOURCE_REELLE" -type f | wc -l | tr -d ' ') fichiers" || { echo "    DISPARU — c'est grave"; fail=1; }

if [ "$fail" -eq 0 ]; then echo "=== VÉRIFICATION RÉELLE : OK"; else echo "=== VÉRIFICATION RÉELLE : ÉCHEC"; fi
exit $fail
