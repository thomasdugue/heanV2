#!/usr/bin/env bash
#
# release.sh — fabrique un Hean.app installable, signé, et son DMG.
#
#   ./scripts/release.sh --test    build local : secrets absents tolérés, pas de notarisation
#   ./scripts/release.sh           build distribuable : secrets et notarisation exigés
#
# Pourquoi ce script existe, et pourquoi il ne se contourne pas :
#
# `libsmbclient` (pavao, ADR 0016) vient de Homebrew et traîne 92 bibliothèques Samba.
# `npm run tauri build` seul produit une app qui ne démarre QUE sur une machine qui a
# Homebrew et samba installés — c'est-à-dire la machine de développement, et aucune autre.
# Le défaut ne se voit donc jamais ici : il se voit chez celui qui installe. C'est arrivé
# en V1. Les trois étapes qui le corrigent (embarquer, dédoublonner les rpath, re-signer)
# ne sont pas des optimisations, ce sont des conditions de démarrage.
#
# Le dédoublonnage des rpath en particulier : `dylibbundler` ajoute
# `@executable_path/../Frameworks/` à chaque dylib, mais 66 des 92 en avaient déjà un.
# dyld refuse un LC_RPATH en double depuis Big Sur — l'app se ferme au lancement sans
# message exploitable.
set -euo pipefail

racine=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$racine"

mode_test=0
[[ "${1:-}" == "--test" ]] && mode_test=1

note() { printf '\033[0;32m→\033[0m %s\n' "$1"; }
halte() { printf '\033[0;31m✗ %s\033[0m\n' "$1" >&2; exit 1; }

# Écrit les dépendances à chemin absolu hors du bundle, pour tout Mach-O qu'il contient.
# Sort 2 si elle n'a inspecté aucun binaire : sans ça, une inspection qui ne lit rien rend
# le même résultat qu'un bundle propre. Deux versions s'y sont déjà cassées —
# `xargs -I{}` renonçait sur les chemins longs en écrivant son erreur ailleurs, puis le
# filtre en fin de boucle faisait sortir la fonction en 1 selon l'ordre de `find`, donc un
# échec un jour sur deux. D'où la forme actuelle : on collecte tout, on filtre une fois, et
# le résultat est la SORTIE, jamais le code de retour.
fuites_du_bundle() {
  local racine_bundle="$1" f brut=""
  while IFS= read -r f; do
    file "$f" 2>/dev/null | grep -q Mach-O || continue
    brut+=$(otool -L "$f" 2>/dev/null | tail -n +2 | sed "s|^|  $(basename "$f") : |")$'\n'
  done < <(find "$racine_bundle" -type f \( -perm -u+x -o -name '*.dylib' \))
  [[ -n "$brut" ]] || return 2
  grep -vE '/usr/lib|/System/|@executable_path|@rpath|@loader_path' <<<"$brut" || true
}

# --- 1. Secrets et identité -------------------------------------------------------------
# `scripts/.env.local` est gitignoré. Il porte les valeurs réelles ; ce script ne les lit
# que pour les passer au compilateur, et n'en affiche jamais une seule.
if [[ -f scripts/.env.local ]]; then
  set -a; # shellcheck disable=SC1091
  source scripts/.env.local; set +a
  note "scripts/.env.local chargé"
fi

identite="${APPLE_SIGNING_IDENTITY:-}"
if [[ -z "$identite" ]]; then
  identite=$(security find-identity -v -p codesigning \
    | grep "Developer ID Application" | head -1 | sed -E 's/.*"(.*)".*/\1/')
fi
[[ -n "$identite" ]] || halte "aucun certificat « Developer ID Application » dans le porte-clés."
note "certificat de signature trouvé"

if (( mode_test )); then
  export HEAN_ALLOW_MISSING_SECRETS=1
  note "mode test : les secrets compile-time manquants sont tolérés"
  printf '  l'\''app produite n'\''aura ni remontée de crash ni bouton de retour.\n'
else
  manquants=()
  for v in HEAN_SENTRY_DSN NOIR_WORKER_URL NOIR_WORKER_SECRET \
           APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
    [[ -z "${!v:-}" ]] && manquants+=("$v")
  done
  (( ${#manquants[@]} == 0 )) || halte "variables manquantes : ${manquants[*]}
Les renseigner dans scripts/.env.local (modèle : bootstrap/06-setup-v2/ci/secrets.env.template),
ou lancer ./scripts/release.sh --test pour un build local sans elles."
fi

# --- 2. Vérifications avant de compiler --------------------------------------------------
note "./check"
./check >/dev/null || halte "./check échoue — on ne fabrique pas une release là-dessus."

# --- 3. Compiler l'app SEULE -------------------------------------------------------------
# `--bundles app` et pas `dmg` : Tauri supprime Hean.app juste après avoir fait le DMG,
# or les trois étapes suivantes travaillent sur l'app. Le DMG est fabriqué à la fin, ici.
note "compilation (compter 2 à 15 min)"
npm run tauri build -- --bundles app

app="src-tauri/target/release/bundle/macos/Hean.app"
[[ -d "$app" ]] || halte "$app introuvable après compilation."

# --- 4. Embarquer les bibliothèques Samba ------------------------------------------------
note "embarquement des dépendances externes"
dylibbundler -b -x "$app/Contents/MacOS/Hean" -d "$app/Contents/Frameworks/" \
  -p @executable_path/../Frameworks/ -cd -of >/dev/null
note "$(ls "$app/Contents/Frameworks" | wc -l | tr -d ' ') bibliothèques embarquées"

# --- 5. Dédoublonner les LC_RPATH --------------------------------------------------------
supprimes=0
for f in "$app"/Contents/Frameworks/*.dylib; do
  while (( $(otool -l "$f" | grep -c 'cmd LC_RPATH') > 1 )); do
    install_name_tool -delete_rpath @executable_path/../Frameworks/ "$f" 2>/dev/null || break
    supprimes=$((supprimes + 1))
  done
done
note "$supprimes LC_RPATH en double supprimés"

# --- 6. Re-signer ------------------------------------------------------------------------
# install_name_tool invalide les signatures : tout est re-signé, dylibs d'abord.
note "signature"
for f in "$app"/Contents/Frameworks/*.dylib; do
  codesign --force --sign "$identite" --options runtime --timestamp "$f" >/dev/null 2>&1 \
    || halte "signature échouée sur $(basename "$f")"
done
codesign --force --sign "$identite" --options runtime --timestamp --deep "$app" >/dev/null 2>&1 \
  || halte "signature de l'app échouée"
codesign -vvv --strict --deep "$app" >/dev/null 2>&1 || halte "la signature produite est invalide"
note "signature valide"

# --- 7. Le garde-fou qui aurait vu le défaut de V1 ---------------------------------------
# Un chemin absolu hors du bundle dans N'IMPORTE quel binaire = l'app ne démarre pas chez
# quelqu'un d'autre. Cette vérification est ici et pas dans ./check parce qu'elle porte sur
# un artefact de build, pas sur le dépôt.
note "vérification : aucune dépendance hors du bundle"
fuites=$(fuites_du_bundle "$app") \
  || halte "l'inspection du bundle n'a lu aucun binaire — vérification non concluante."
[[ -z "$fuites" ]] || halte "dépendances hors du bundle, l'app ne démarrera pas ailleurs :
$fuites"
note "aucune fuite"

# --- 8. Notarisation (seulement en mode distribuable) ------------------------------------
if (( ! mode_test )); then
  note "notarisation de l'app (2 à 5 min chez Apple)"
  ditto -c -k --keepParent "$app" /tmp/hean-notarize.zip
  xcrun notarytool submit /tmp/hean-notarize.zip --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
  xcrun stapler staple "$app"
fi

# --- 9. Le DMG ---------------------------------------------------------------------------
version=$(node -p "require('./package.json').version")
dmg="src-tauri/target/release/bundle/dmg/Hean_${version}_aarch64.dmg"
mkdir -p "$(dirname "$dmg")"
rm -f "$dmg"
note "fabrication du DMG"
mise_en_scene=$(mktemp -d)
cp -R "$app" "$mise_en_scene/"
ln -s /Applications "$mise_en_scene/Applications"
hdiutil create -volname "Hean" -srcfolder "$mise_en_scene" -ov -format UDZO "$dmg" >/dev/null
rm -rf "$mise_en_scene"
codesign --force --sign "$identite" --timestamp "$dmg" >/dev/null 2>&1 \
  || halte "signature du DMG échouée"

# Le DMG se notarise AUSSI : Gatekeeper refuse un DMG non notarisé même quand l'app
# qu'il contient l'est.
if (( ! mode_test )); then
  note "notarisation du DMG"
  xcrun notarytool submit "$dmg" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" --wait
  xcrun stapler staple "$dmg"
fi

# --- 10. Verdict ------------------------------------------------------------------------
echo
note "DMG : $dmg ($(du -h "$dmg" | cut -f1))"
if (( mode_test )); then
  cat <<'TXT'

  App non notarisée : au premier lancement, macOS dira que le développeur n'a pas pu être
  vérifié. Réglages système → Confidentialité et sécurité → « Ouvrir quand même ».
  Une seule fois, puis plus jamais.
TXT
else
  spctl -a -vvv "$app" 2>&1 | tail -2
fi
