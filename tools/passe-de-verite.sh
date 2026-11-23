#!/usr/bin/env bash
# OUTIL TEMPORAIRE — « passe de vérité ». À jeter quand l'interface existera.
#
# Hors produit et hors ./check : ce script n'est appelé par rien d'autre que par Thomas, et
# il ne compile pas avec l'application (tools/passe-de-verite/Cargo.toml est détaché).
#
#   ./tools/passe-de-verite.sh bibliotheque [dossier]   scanne et compare à la V1
#   ./tools/passe-de-verite.sh jouer [fichier]          fait sortir du son
#   ./tools/passe-de-verite.sh verifier [fichier]       compare le décodage à celui d'ffmpeg
#
# LECTURE SEULE sur la musique et sur les données de la V1. Rien n'est modifié, déplacé ni
# supprimé chez l'utilisateur : la base SQLite jetable et le PCM décodé vont dans un dossier
# temporaire, jamais dans le dossier de données de l'application.
#
# Aucun chemin de machine n'est écrit ici (tests/architecture/aucun-chemin-machine.test.ts).
# Le dossier à scanner vient, dans cet ordre : de l'argument, de $HEAN_BIBLIOTHEQUE, ou de
# ce que la V1 a elle-même enregistré comme bibliothèque dans son config.json.
set -uo pipefail
racine_depot="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
crate="$racine_depot/tools/passe-de-verite"
donnees_v1="${HEAN_DONNEES_V1:-$HOME/Library/Application Support/noir}"
travail="${TMPDIR:-/tmp}/hean-passe-de-verite"
mkdir -p "$travail"

# Le binaire se construit dans le target de src-tauri : les dépendances y sont déjà
# compilées, donc on ne les recompile pas pour un outil jetable.
export CARGO_TARGET_DIR="$racine_depot/src-tauri/target"
binaire="$CARGO_TARGET_DIR/release/passe-de-verite"

batir() {
  echo "→ construction de l'outil (silencieuse si rien n'a changé)"
  # --release, et pas debug : symphonia et rusqlite en debug mettraient des minutes là où
  # le release met des secondes, et un diagnostic qu'on n'a pas envie de relancer ne sert
  # plus à rien.
  #
  # HEAN_ALLOW_MISSING_SECRETS=1 : en profil release, `src-tauri/src/secrets.rs` exige par
  # `compile_error!` les secrets de compilation (Sentry, worker). C'est voulu — un binaire
  # distribué sans eux serait muet. Cet outil n'est pas distribué, ne touche à aucun secret
  # et n'en a aucun usage ; la variable est l'échappatoire que secrets.rs documente
  # lui-même pour exactement ce cas.
  HEAN_ALLOW_MISSING_SECRETS=1 \
    cargo build --release --quiet --manifest-path "$crate/Cargo.toml" || exit 1
}

trouver_racine() {
  if [ -n "${1:-}" ]; then echo "$1"; return; fi
  if [ -n "${HEAN_BIBLIOTHEQUE:-}" ]; then echo "$HEAN_BIBLIOTHEQUE"; return; fi
  # Le dernier recours n'est pas une devinette : c'est le dossier que la V1 a enregistré
  # elle-même. Il est lu dans SES données, pas écrit dans le dépôt.
  python3 - "$donnees_v1/config.json" <<'PY'
import json, sys
try:
    chemins = json.load(open(sys.argv[1]))["library_paths"]
except Exception:
    chemins = []
print(chemins[0] if chemins else "")
PY
}

# La piste par défaut est prise dans le DERNIER SCAN, donc parmi ce que la bibliothèque V2
# a réellement indexé — pas par un `find`, qui court-circuiterait justement ce qu'on teste.
# La plus haute résolution disponible, d'une durée raisonnable : c'est le cas d'usage de
# Thomas, et celui où un défaut de chaîne s'entendrait le plus.
resoudre_piste() {
  if [ -n "${1:-}" ]; then echo "$1"; return; fi
  [ -f "$travail/v2.json" ] || { echo ""; return; }
  python3 - "$travail/v2.json" <<'PY'
import json, sys
pistes = json.load(open(sys.argv[1]))["pistes"]
bons = [p for p in pistes if 60_000 <= (p.get("duree_ms") or 0) <= 420_000] or pistes
bons.sort(key=lambda p: (p.get("bits") or 0, p.get("echantillonnage") or 0), reverse=True)
print(bons[0]["path"] if bons else "")
PY
}

cas="${1:-bibliotheque}"; shift || true

case "$cas" in
  bibliotheque)
    racine="$(trouver_racine "${1:-}")"
    # Deux pannes différentes, deux messages différents : « je n'ai pas de chemin » et
    # « ton chemin n'existe pas » n'appellent pas la même chose de la part de Thomas.
    if [ -z "$racine" ]; then
      echo "Je ne sais pas où est ta bibliothèque."
      echo "Donne-la-moi : ./tools/passe-de-verite.sh bibliotheque \"/chemin/vers/ta/musique\""
      exit 2
    fi
    if [ ! -d "$racine" ]; then
      echo "Ce dossier n'existe pas (ou n'est pas un dossier) :"
      echo "  $racine"
      if [ -z "${1:-}" ] && [ -z "${HEAN_BIBLIOTHEQUE:-}" ]; then
        echo "Il vient du config.json de la V1 — ta bibliothèque a dû bouger depuis."
        echo "Donne-moi le bon : ./tools/passe-de-verite.sh bibliotheque \"/le/bon/chemin\""
      fi
      exit 2
    fi
    if [ ! -d "$donnees_v1" ]; then
      echo "Données de la V1 introuvables dans : $donnees_v1"
      echo "Si elles sont ailleurs : HEAN_DONNEES_V1=\"/chemin\" ./tools/passe-de-verite.sh …"
      exit 2
    fi
    batir
    echo "→ scan de « $racine » par le code d'indexation de la V2"
    # Un nom neuf à chaque lancement ($$ = numéro de processus) : le scanner refuse
    # d'écraser un fichier existant, et c'est ce qui rend « ne supprime rien » exact plutôt
    # que « ne supprime rien d'important ».
    "$binaire" scanner "$racine" "$travail/jetable-$$.db" "$travail/v2.json" || exit 1
    echo
    python3 "$racine_depot/tools/passe-de-verite-rapport.py" \
      "$travail/v2.json" "$donnees_v1" "$travail/detail.txt"
    ;;

  jouer)
    fichier="$(resoudre_piste "${1:-}")"
    [ -n "$fichier" ] || { echo "Lance d'abord : ./tools/passe-de-verite.sh bibliotheque"; exit 2; }
    [ -f "$fichier" ] || { echo "Fichier introuvable : $fichier"; exit 2; }
    batir
    pcm="$travail/ecoute.wav"
    echo "→ décodage par le chemin audio de la V2 (chaîne transparente)"
    echo "   $fichier"
    "$binaire" jouer "$fichier" "$pcm" || exit 1
    echo
    echo "→ lecture. Le décodage et la chaîne sont ceux de la V2 ; le dernier saut vers ta"
    echo "   sortie audio est fait par afplay, parce que le backend CoreAudio de la V2"
    echo "   n'existe pas encore. Donc : tu entends la V2 décoder, pas encore la V2 jouer."
    echo
    afplay "$pcm"
    echo "→ fini. Le PCM décodé reste là si tu veux le réécouter ($(du -h "$pcm" | cut -f1)) :"
    echo "   $pcm"
    ;;

  verifier)
    # La méthode de `src-tauri/tests/bit_perfect.rs`, mais sur un VRAI fichier de la
    # bibliothèque au lieu d'une fixture : on décode par la chaîne de la V2, on fait décoder
    # le même fichier par ffmpeg — un décodeur écrit par d'autres — et on compare octet par
    # octet. Comparer notre sortie à elle-même ne prouverait rien.
    command -v ffmpeg >/dev/null || { echo "ffmpeg absent : rien à comparer"; exit 2; }
    fichier="$(resoudre_piste "${1:-}")"
    [ -n "$fichier" ] || { echo "Lance d'abord : ./tools/passe-de-verite.sh bibliotheque"; exit 2; }
    [ -f "$fichier" ] || { echo "Fichier introuvable : $fichier"; exit 2; }
    batir
    echo "→ décodage par la V2"
    echo "   $fichier"
    "$binaire" jouer "$fichier" "$travail/ecoute.wav" > "$travail/jouer.txt" || exit 1
    cat "$travail/jouer.txt"
    bits="$(sed -n 's/^format : \([0-9]*\) bits.*/\1/p' "$travail/jouer.txt")"
    case "$bits" in 16) codec=pcm_s16le;; 32) codec=pcm_s32le;; *) codec=pcm_s24le;; esac
    echo "→ décodage du même fichier par ffmpeg ($codec), puis comparaison"
    # Les mêmes drapeaux que les goldens du dépôt (tools/make-fixtures-audio.sh) : sans eux
    # ffmpeg écrit un en-tête bavard, et la comparaison échouerait sur des octets qui ne
    # sont pas de l'audio.
    ffmpeg -nostdin -v error -y -i "$fichier" -c:a "$codec" \
      -fflags +bitexact -map_metadata -1 "$travail/golden.wav" || exit 1
    echo
    if cmp -s "$travail/ecoute.wav" "$travail/golden.wav"; then
      echo "IDENTIQUES, octet pour octet. La chaîne audio de la V2 ne modifie aucun bit"
      echo "entre ton fichier et sa sortie — sur CE fichier, pas sur une fixture."
    else
      echo "ILS DIFFÈRENT. C'est un défaut de bit-perfect, et voici où :"
      cmp -l "$travail/ecoute.wav" "$travail/golden.wav" | head -5
      echo "  (octets différents : $(cmp -l "$travail/ecoute.wav" "$travail/golden.wav" | wc -l))"
      exit 1
    fi
    echo
    # Cet outil ne supprime rien, jamais — pas même ses propres fichiers temporaires. Il dit
    # où ils sont et ce qu'ils pèsent ; l'effacement est ta décision.
    echo "Les deux WAV décodés restent dans $travail"
    echo "($(du -sh "$travail" | cut -f1) au total, à effacer quand tu veux)"
    ;;

  *)
    sed -n '3,9p' "${BASH_SOURCE[0]}"
    exit 2
    ;;
esac
