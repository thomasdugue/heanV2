#!/usr/bin/env bash
# Chaque skill doit avoir un frontmatter exploitable (name + description).
set -u
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fail=0; count=0
for f in "$root"/.claude/skills/*/SKILL.md; do
  [ -e "$f" ] || continue
  count=$((count+1))
  head -1 "$f" | grep -qx -- '---' || { echo "  pas de frontmatter : ${f#$root/}"; fail=1; continue; }
  grep -qE '^name: ' "$f"        || { echo "  frontmatter sans 'name' : ${f#$root/}"; fail=1; }
  grep -qE '^description: ' "$f" || { echo "  frontmatter sans 'description' : ${f#$root/}"; fail=1; }
done
if [ "$fail" -eq 0 ]; then echo "check-skills : OK ($count skills)"; else echo "check-skills : ÉCHEC"; fi
exit $fail
