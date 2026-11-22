---
name: verifier-le-garde-fou-des-secrets
description: Comment prouver (et non supposer) que le compile_error!() de build.rs bloque un build release sans secrets — et le piège du garde-fou déjà généré dans target/
metadata:
  type: reference
---

Pour vérifier INV-17 sur le terrain, ne pas se contenter de lire
`src-tauri/target/release/build/hean-*/out/secrets_guard.rs` : ce fichier peut être **vide**
alors que le garde-fou fonctionne, parce que `build.rs` ne se relance que si une des variables
`rerun-if-env-changed` a changé. Un fichier vide ne prouve donc rien.

La vérification qui prouve quelque chose (profil release, cible déjà chaude, aucune source
modifiée) :

- `HEAN_ALLOW_MISSING_SECRETS=0 cargo build --release --locked --manifest-path src-tauri/Cargo.toml`
  → doit échouer sur `error: secrets compile-time manquants en profil release : …`
  (la valeur `0` sert à forcer la relance du script de build, et ne vaut pas dérogation).
- `HEAN_ALLOW_MISSING_SECRETS=1 …` → doit compiler avec
  `warning: build release SANS les secrets … Ne pas distribuer ce binaire.`

**Why:** vérifié le 2026-11-22 pendant la relecture de la phase 0 ; le `secrets_guard.rs`
vide dans `target/release` donnait l'impression d'un garde-fou muet, à tort.
**How to apply:** à refaire à chaque diff qui touche `src-tauri/build.rs` ou
`src-tauri/src/secrets.rs` (zones jamais déléguées). Un `cargo build` n'écrit que dans
`target/` : il reste compatible avec la lecture seule. Attention en revanche à
`cargo test --test export_bindings`, qui lui réécrit `src/ipc/bindings.ts`.
