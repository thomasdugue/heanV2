#!/usr/bin/env bash
# Limite dure de taille de fichier : un fichier trop gros n'est plus raisonnable.
# Leçon V1 : lib.rs 4 870 lignes, views.js 3 512, styles.css 10 654 en un seul fichier.
set -u
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
limit=400
fail=0
for dir in src src-tauri/src; do
  [ -d "$root/$dir" ] || continue
  while IFS= read -r f; do
    n=$(wc -l < "$f" | tr -d ' ')
    if [ "$n" -gt "$limit" ]; then
      echo "  trop gros ($n lignes > $limit) : ${f#$root/}"
      fail=1
    fi
  done < <(find "$root/$dir" -type f \( -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.rs' -o -name '*.css' \) -not -path '*/node_modules/*')
done
if [ "$fail" -eq 0 ]; then echo "check-sizes : OK (limite $limit lignes)"; else echo "check-sizes : ÉCHEC"; fi
exit $fail
