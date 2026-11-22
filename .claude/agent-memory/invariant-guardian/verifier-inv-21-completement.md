---
name: verifier-inv-21-completement
description: Le test d'architecture qui protège INV-21 ne scanne que src-tauri/src — les tests d'intégration de src-tauri/tests/ y échappent, il faut les grepper à la main en relecture
metadata:
  type: reference
---

`tests/architecture/backend.test.ts` applique INV-21 (aucun `println!` / `eprintln!` /
`dbg!`) via `sources('src-tauri/src', ['.rs'])`. Deux conséquences pour une relecture :

- Les modules `#[cfg(test)]` **dans** `src-tauri/src/**` sont bien couverts.
- Les **tests d'intégration** de `src-tauri/tests/*.rs` ne le sont **pas**. Un `println!`
  y passerait `./check` au vert.
- `src-tauri/build.rs` est volontairement hors scope (ses `println!("cargo::…")` sont le
  protocole de Cargo).

Donc, à chaque diff qui ajoute des tests Rust :
`grep -rn "println!\|eprintln!\|dbg!\|print!(" src-tauri/src src-tauri/tests`

Les vérifications du harnais qui tournent vite et sans effet de bord (utiles pour prouver
qu'un diff n'a rien affaibli) : `bash tools/check-sizes.sh`, `check-zones.sh`,
`check-skills.sh`, `check-fixtures-audio.sh`, `node tools/check-invariants.mjs`, et
`npx vitest run tests/architecture` (~0,3 s, 32 tests). Éviter `cargo test` sur tout le
crate : la cible `export_bindings` réécrit `src/ipc/bindings.ts` (voir
[[verifier-le-garde-fou-des-secrets]]).

**Why:** relecture de la phase 1 (moteur audio vérifiable) le 2026-11-22 — il fallait
prouver INV-21 « y compris dans les tests du crate », et le mécanisme ne le couvre pas.
**How to apply:** ne jamais conclure « INV-21 OK » sur la seule foi du test
d'architecture quand le diff touche `src-tauri/tests/`.
