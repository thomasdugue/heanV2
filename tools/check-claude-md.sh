#!/usr/bin/env bash
# CLAUDE.md doit rester lisible EN ENTIER à chaque session.
# Leçon V1 : il avait fini par être tronqué à la lecture, et l'historique a dû en être extrait.
set -u
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
limit=150
n=$(wc -l < "$root/CLAUDE.md" | tr -d ' ')
if [ "$n" -gt "$limit" ]; then
  echo "  CLAUDE.md fait $n lignes (> $limit) : déplacer du contenu dans un skill .claude/skills/"
  echo "check-claude-md : ÉCHEC"
  exit 1
fi
echo "check-claude-md : OK ($n/$limit lignes)"
