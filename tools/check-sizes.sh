#!/usr/bin/env bash
# Limite dure de taille de fichier : un fichier trop gros n'est plus raisonnable.
# Leçon V1 : lib.rs 4 870 lignes, views.js 3 512, styles.css 10 654 en un seul fichier.
#
# ⚠️ CE DÉTECTEUR A EU UN TROU, jusqu'au 2026-11-23. Il listait les extensions à mesurer
# (.ts .tsx .js .rs .css .mjs .svelte) et ne lisait donc **ni shell ni Python**. Un rapport
# Python de 569 lignes est passé VERT : le premier fichier du dépôt à franchir la limite, et
# il l'a franchie dans une langue que le détecteur ne lit pas. Relevé par invariant-guardian
# en relecture de la « passe de vérité », transmis par cette session-là.
#
# **D'où l'inversion : on mesure tout, sauf une liste d'exceptions écrite.** Une règle tenue
# par la liste de ce qu'elle surveille oublie la langue qu'on ajoutera demain ; une règle
# tenue par la liste de ce qu'elle exclut la couvre d'avance. Le SENS DE L'ÉCHEC change, et
# c'est là tout l'intérêt : un binaire nouveau fait rougir, et quelqu'un écrit l'exception —
# au lieu qu'un langage nouveau passe en silence, ce qui ne réveille personne.
#
# Et on ne lit que les fichiers **SUIVIS PAR GIT**, comme `check-adr.mjs`. C'est ce qui rend
# le corpus identique en local et en CI : sans ça, un `.DS_Store` ou un fichier de travail
# rendrait `./check` rouge chez Thomas et vert en CI — le défaut du 2026-11-22, vu de l'autre
# bout. Ça règle aussi d'un coup les dépôts imbriqués et les binaires que macOS sème.
set -u
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root" || exit 1
limit=400

# Les zones, PLUS le point d'entrée lui-même : `./check` applique la règle aux autres et y
# échappait, n'étant dans aucun dossier scanné. Trouvé en mesurant, le 2026-11-23.
ZONES=(src src-tauri/src tests tools check)

# Ce qui n'est pas du code à lire d'un bout à l'autre. Chaque exception a sa raison :
#   - les binaires : `wc -l` n'y veut rien dire, et le compte serait du bruit ;
#   - `zones-interdites.txt` : une table de données lue par un script, pas un fichier qu'on
#     lit d'un bout à l'autre — la limite n'y protège rien ;
#   - les FICHIERS DE VERROUILLAGE de dépendances (`Cargo.lock`, `package-lock.json`) :
#     **tranché par Thomas le 2026-11-23** (option A), sur une propriété vérifiable et non
#     par confort — un lock est écrit par un outil et régénéré, jamais relu, alors que la
#     leçon de V1 porte sur ce qu'on RELIT (`lib.rs` à 4 870 lignes, `views.js` à 3 512).
#     Le motif vise le nom de fichier **où qu'il soit**, donc les trois locks du dépôt sont
#     couverts de la même façon : `src-tauri/Cargo.lock` (5 265 lignes),
#     `tools/passe-de-verite/Cargo.lock` (5 109) et `package-lock.json` (2 180). Pas
#     seulement celui qui rougissait.
#
# ⚠️ ET UN TROU QUI RESTE, écrit ici pour qu'il ne se redécouvre pas : deux de ces trois
# locks échappent AUSSI à la mesure pour une raison **différente** — ils sont hors des zones
# ci-dessus (`package-lock.json` est à la racine, `src-tauri/Cargo.lock` à la racine de
# `src-tauri/`). L'exception les couvre par le nom, donc le jour où les zones s'élargissent
# rien ne change pour eux ; mais **les ZONES sont elles aussi une liste**, avec la même
# faiblesse que celle des extensions qu'on vient de supprimer. Voir la liste « reste à
# brancher » en tête de `check` : c'est sa propre tâche, et elle n'est pas gratuite.
EXCEPTIONS='\.(woff2?|ttf|otf|png|jpe?g|gif|webp|icns|ico|flac|wav|mp3|m4a|aiff?|pdf)$|^tools/zones-interdites\.txt$|(^|/)(Cargo\.lock|package-lock\.json)$'

mesures=$(git ls-files -- "${ZONES[@]}" | grep -vE "$EXCEPTIONS")
n=$(printf '%s' "$mesures" | grep -c . || true)

# Un détecteur qui ne regarde RIEN passe au vert sans rien garantir. Si `git ls-files` ne
# rend plus rien — dossier renommé, exécution hors dépôt — c'est un échec, pas un succès.
if [ "$n" -eq 0 ]; then
  echo "  corpus vide : aucune des zones (${ZONES[*]}) ne contient de fichier suivi par git"
  echo 'check-sizes : ÉCHEC'
  exit 1
fi

fail=0
while IFS= read -r f; do
  [ -n "$f" ] || continue
  lignes=$(wc -l < "$f" | tr -d ' ')
  if [ "$lignes" -gt "$limit" ]; then
    echo "  trop gros ($lignes lignes > $limit) : $f"
    fail=1
  fi
done <<< "$mesures"

if [ "$fail" -eq 0 ]; then
  echo "check-sizes : OK ($n fichiers mesurés, limite $limit lignes)"
else
  echo 'check-sizes : ÉCHEC'
fi
exit $fail
