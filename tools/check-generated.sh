#!/usr/bin/env bash
# La CI régénère les artefacts de niveau 1 et ÉCHOUE si le résultat diffère du commit.
# C'est le mécanisme qui rend la dérive de documentation impossible (cause racine n°5 du
# postmortem : 11 affirmations fausses, dont trois chemins de données documentés faux).
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

CIBLES=(src/ipc/bindings.ts src/styles/tokens.css src/styles/tokens.generated.ts docs/generated)

bash tools/gen/all.sh > /dev/null || { echo "  la régénération a échoué"; echo "check-generated : ÉCHEC"; exit 1; }

if ! git diff --quiet -- "${CIBLES[@]}" || [ -n "$(git ls-files --others --exclude-standard -- "${CIBLES[@]}")" ]; then
  echo "  des artefacts générés diffèrent du commit :"
  git diff --stat -- "${CIBLES[@]}" | sed 's/^/    /'
  git ls-files --others --exclude-standard -- "${CIBLES[@]}" | sed 's/^/    (non suivi) /'
  echo "  → lancer 'npm run gen' et commiter le résultat"
  echo "check-generated : ÉCHEC"
  exit 1
fi
echo "check-generated : OK (${#CIBLES[@]} cibles à jour)"
