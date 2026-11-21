#!/usr/bin/env bash
# Mesure la mémoire réelle d'une app Tauri : processus natif + les 3 processus WebKit
# (WebContent, GPU, Networking) qu'il fait naître. Le RSS seul surcompte les pages
# partagées : on lit aussi phys_footprint, qui est la mémoire réellement attribuée.
#
# POURQUOI CET OUTIL : les budgets de V2 n'ont de sens que comparés à V1 avec la MÊME
# méthode. Ne pas mesurer V1 avec Activity Monitor et V2 avec autre chose.
#
# Usage :
#   ./tools/measure-app.sh noir-tauri 60      # V1 (binaire de Hean v0.2.0-beta.1)
#   ./tools/measure-app.sh hean 60            # V2, plus tard
#
# Protocole de référence (à reproduire à l'identique entre V1 et V2) :
#   1. app fermée, lancer ce script → il attend le process
#   2. laisser 10 s au repos sur la vue d'accueil           → « repos »
#   3. aller sur Albums, défiler toute la grille sans pause  → « après défilement »
#   4. taper 3 requêtes dans la recherche                    → « après recherche »
#   Noter les 3 paliers. Le script imprime la courbe, c'est toi qui lis les paliers.
set -uo pipefail
PROC="${1:-noir-tauri}"
DUR="${2:-60}"

pid_of() { pgrep -x "$PROC" | head -1; }
sum_footprint() {
  local main=$1 total=0 v
  for p in $main $(python3 "$(dirname "${BASH_SOURCE[0]}")/webkit-pids.py" "$main"); do
    v=$(footprint -p "$p" 2>/dev/null | awk '/^[[:space:]]*phys_footprint:/{print $2; exit}')
    u=$(footprint -p "$p" 2>/dev/null | awk '/^[[:space:]]*phys_footprint:/{print $3; exit}')
    [ -z "${v:-}" ] && continue
    case "$u" in KB) v=$(echo "$v/1024" | bc -l);; GB) v=$(echo "$v*1024" | bc -l);; esac
    total=$(echo "$total + $v" | bc -l)
  done
  printf "%.0f" "$total"
}
sum_rss() {
  local main=$1 pids
  pids="$main $(python3 "$(dirname "${BASH_SOURCE[0]}")/webkit-pids.py" "$main")"
  ps -axo pid,rss | awk -v L="$pids" 'BEGIN{split(L,a," ");for(i in a)k[a[i]]=1} k[$1]{s+=$2} END{printf "%.0f", s/1024}'
}

echo "en attente du processus $PROC…"
for _ in $(seq 1 120); do PID=$(pid_of); [ -n "${PID:-}" ] && break; sleep 0.5; done
[ -z "${PID:-}" ] && { echo "processus $PROC introuvable — lance l'app"; exit 1; }
echo "pid $PID · échantillonnage toutes les 2 s pendant ${DUR} s"
echo
printf "%7s %12s %14s\n" "t (s)" "RSS cumulé" "footprint"
t=0
while [ "$t" -le "$DUR" ]; do
  [ -z "$(pid_of)" ] && { echo "(processus terminé)"; break; }
  printf "%7s %9s Mo %11s Mo\n" "$t" "$(sum_rss "$PID")" "$(sum_footprint "$PID")"
  sleep 2; t=$((t+2))
done
echo
echo "processus comptés (natif + WebKit nés après lui) :"
for p in $PID $(python3 "$(dirname "${BASH_SOURCE[0]}")/webkit-pids.py" "$PID"); do
  ps -p "$p" -o pid=,rss=,comm= | awk '{n=$3; sub(/.*\//,"",n); printf "  %7s %7.0f Mo  %s\n",$1,$2/1024,n}'
done
