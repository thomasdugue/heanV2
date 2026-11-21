#!/usr/bin/env bash
# Budget de poids et de dépendances. Inerte jusqu'à ce qu'un build existe.
#
# Pourquoi : le frontend de V1 pesait 960 Ko, soit ~1 % de l'app distribuée (DMG 33 Mo) —
# donc le framework n'est PAS le risque. Le risque, c'est la dérive npm : V1 avait ZÉRO
# dépendance de production côté frontend parce qu'il n'y avait pas de bundler. Ouvrir
# package.json ouvre la porte. Voir docs/PERFORMANCE.md.
set -uo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_BUDGET_KB=600   # V1 servait 960 Ko non minifiés ; V2 doit faire mieux
DEPS_BUDGET=3        # Svelte est une devDependency : il disparaît au build
fail=0

if [ -d "$root/dist" ]; then
  kb=$(du -sk "$root/dist" | cut -f1)
  if [ "$kb" -gt "$DIST_BUDGET_KB" ]; then
    echo "  dist/ pèse ${kb} Ko (budget ${DIST_BUDGET_KB} Ko)"
    echo "  → alléger, ou relever le budget dans un ADR en justifiant"
    fail=1
  else
    echo "check-weight : dist/ ${kb}/${DIST_BUDGET_KB} Ko"
  fi
else
  echo "check-weight : pas de dist/ (rien à mesurer pour l'instant)"
fi

if [ -f "$root/package.json" ]; then
  n=$(python3 -c "import json;print(len(json.load(open('$root/package.json')).get('dependencies',{})))" 2>/dev/null || echo 0)
  if [ "$n" -gt "$DEPS_BUDGET" ]; then
    echo "  ${n} dépendances de production (budget ${DEPS_BUDGET})"
    echo "  → toute dépendance de production supplémentaire demande un ADR :"
    echo "    ce qu'elle apporte, son poids mesuré, pourquoi on ne l'écrit pas à la main"
    fail=1
  else
    echo "check-weight : ${n}/${DEPS_BUDGET} dépendances de production"
  fi
fi

[ "$fail" -eq 0 ] || echo "check-weight : ÉCHEC"
exit $fail
