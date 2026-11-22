#!/usr/bin/env bash
# APPARIEMENT (niveau 2) — la liste des zones jamais déléguées existe à deux endroits :
# tools/zones-interdites.txt (la source, lue par le hook de fin de tâche) et le skill
# hean-orchestration (lu par un modèle en début de session). Si les deux divergent, le
# modèle lit une liste périmée sans que personne le sache. Ce test l'interdit.
#
# Ce que ce script ne fait PAS, et c'est assumé : il ne peut pas savoir QUEL modèle a écrit
# une ligne. Le garde-fou qui réclame, lui, est le hook .claude/hooks/contrat-de-fin-de-tache.sh.
set -uo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_zones="$root/tools/zones-interdites.txt"
skill="$root/.claude/skills/hean-orchestration/SKILL.md"
fail=0

ids=$(grep -v '^\s*#' "$source_zones" | grep -v '^\s*$' | cut -d'|' -f1 | tr -d ' ' | sort)
n=$(printf '%s\n' "$ids" | grep -c . )
[ "$n" -ge 5 ] || { echo "  $n zones listées, au moins 5 attendues (ADR 0005)"; fail=1; }

if [ -f "$skill" ]; then
  for id in $ids; do
    grep -q "zone:$id" "$skill" || { echo "  zone « $id » absente du skill hean-orchestration"; fail=1; }
  done
  for id in $(grep -oE 'zone:[a-z-]+' "$skill" | cut -d: -f2 | sort -u); do
    printf '%s\n' "$ids" | grep -qx "$id" || { echo "  le skill cite « $id », absente de tools/zones-interdites.txt"; fail=1; }
  done
else
  echo "  .claude/skills/hean-orchestration/SKILL.md absent"; fail=1
fi

if [ "$fail" -eq 0 ]; then echo "check-zones : OK ($n zones, skill en accord)"; else echo "check-zones : ÉCHEC"; fi
exit $fail
