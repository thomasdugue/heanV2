#!/usr/bin/env bash
# Régénère TOUS les artefacts de niveau 1 (ENTRETIEN-DU-HARNAIS.md).
# Rien de ce qui est produit ici ne s'écrit à la main. La CI relance ce script et échoue si
# le résultat diffère du commit : la dérive devient impossible, pas seulement improbable.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

# 1. Types TypeScript de l'IPC, depuis les annotations Rust (tauri-specta).
cargo test --quiet --locked --manifest-path src-tauri/Cargo.toml --test export_bindings

# 2. Catalogue IPC, tokens CSS, tableaux de couverture.
node tools/gen/ipc.mjs
node tools/gen/tokens.mjs
node tools/gen/couverture.mjs
