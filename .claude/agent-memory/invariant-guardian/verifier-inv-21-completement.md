---
name: verifier-inv-21-completement
description: Angle mort d'INV-21 refermé le 2026-11-22 — le test d'architecture scanne désormais src-tauri/src ET src-tauri/tests ; garder le grep manuel pour les dossiers hors scope
metadata:
  type: reference
---

> **Mise à jour du 2026-11-22 (commit `4769020`)** : le trou décrit ci-dessous est
> **refermé**. `backend.test.ts` scanne maintenant `src-tauri/src` **et** `src-tauri/tests`
> (constante `RUST`), et la mutation le confirme — un `println!` ajouté dans `decodage.rs`
> fait rougir le test. Le reste de la fiche garde sa valeur : il dit *pourquoi* le trou
> existait, et la méthode reste bonne pour tout dossier non scanné.

`tests/architecture/backend.test.ts` applique INV-21 (aucun `println!` / `eprintln!` /
`dbg!`). À l'origine il ne lisait que `sources('src-tauri/src', ['.rs'])` :

- Les modules `#[cfg(test)]` **dans** `src-tauri/src/**` étaient bien couverts.
- Les **tests d'intégration** de `src-tauri/tests/*.rs` ne l'étaient **pas** — un `println!`
  y passait `./check` au vert. Tant que ce dossier n'avait qu'un fichier, le trou ne se
  voyait pas. Couvert depuis.
- `src-tauri/build.rs` reste volontairement hors scope (ses `println!("cargo::…")` sont le
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
prouver INV-21 « y compris dans les tests du crate », et le mécanisme ne le couvrait pas.
**How to apply:** le mécanisme couvre désormais les deux dossiers, donc le test
d'architecture suffit pour eux. La leçon qui reste : une règle n'est tenue que là où le
garde-fou regarde, et ce périmètre se vérifie au lieu de se supposer.
