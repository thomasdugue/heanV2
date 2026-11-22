#!/usr/bin/env bash
# APPARIEMENT (niveau 2) — la version épinglée ↔ la version qui tourne vraiment.
#
# Sans ce test, `rust-toolchain.toml` serait une affirmation de plus dans un fichier, et on
# sait ce que ça a coûté à V1. Un fichier dit « 1.93.1 » ; ce script vérifie que c'est bien
# 1.93.1 qui compile — ce qui n'est pas acquis si rustc vient d'ailleurs que de rustup.
set -uo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fichier="$root/rust-toolchain.toml"
fail=0

epinglee=$(grep -E '^\s*channel\s*=' "$fichier" 2>/dev/null | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "${epinglee:-}" ]; then
  echo "  rust-toolchain.toml : aucun 'channel' lisible"
  echo "check-toolchain : ÉCHEC"; exit 1
fi

qui_tourne=$(cd "$root" && rustc --version 2>/dev/null | awk '{print $2}')
if [ -z "${qui_tourne:-}" ]; then
  echo "  rustc introuvable — installer rustup : https://rustup.rs"
  echo "check-toolchain : ÉCHEC"; exit 1
fi

if [ "$qui_tourne" != "$epinglee" ]; then
  echo "  épinglée : $epinglee   en train de tourner : $qui_tourne"
  echo "  → ./check ne vérifie donc PAS la même chose ici qu'en CI (ADR 0008)."
  echo "    Si rustup gère ta chaîne, il installe la bonne version tout seul : relance."
  echo "    Si rustc vient d'ailleurs (brew…), c'est lui qui prend le dessus — passer par rustup."
  echo "    Pour monter de version volontairement : changer 'channel' dans rust-toolchain.toml."
  fail=1
fi

# clippy est déclaré dans `components` : s'il manque, `./check` saute une vérification en
# silence, ce qui est pire qu'un échec.
if ! (cd "$root" && cargo clippy --version >/dev/null 2>&1); then
  echo "  clippy absent de la chaîne épinglée — 'rustup component add clippy'"
  fail=1
fi

if [ "$fail" -eq 0 ]; then echo "check-toolchain : OK (Rust $epinglee, clippy présent)"; else echo "check-toolchain : ÉCHEC"; fi
exit $fail
