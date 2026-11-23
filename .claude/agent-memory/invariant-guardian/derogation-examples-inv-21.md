---
name: derogation-examples-inv-21
description: La dérogation src-tauri/examples/ à INV-21 (2026-11-23) a été écrite sans ADR alors que la règle en exige un, et elle fera rougir backend.test.ts au merge de la branche de migration — à trancher par Thomas
metadata:
  type: project
---

`tests/architecture/backend.test.ts` porte depuis le 2026-11-23 une liste `HORS_SCAN` des
fichiers Rust exemptés d'INV-21 : `src-tauri/build.rs` et **`src-tauri/examples/`**.

**Deux points ouverts, tous deux pour Thomas :**

1. Le texte d'INV-21 dans le skill dit « **Toute exception passera par un ADR** » et cette
   phrase est inchangée, deux lignes au-dessus de la nouvelle exception. L'entrée
   `docs/HISTORY.md` du même jour écrit « Aucun ADR : rien de structurant n'a été tranché
   ici ». Il faut donc soit un ADR, soit amender la phrase — pas les deux versions.
2. `src-tauri/examples/migrer.rs` existe sur la branche `claude/migration-noir-hean`
   (commit `8eaa6e4`) avec ~7 `println!`/`eprintln!`, et **pas** sur `main`. Au merge, le
   test « chaque dérogation est en vigueur ou dite muette » **rougira volontairement** pour
   faire retirer la note `muet`. C'est voulu, pas une régression : ne pas le diagnostiquer
   comme un défaut du merge.

**Why:** la dérogation est raisonnable sur le fond (un exemple lancé à la main n'est pas
dans le binaire livré, donc il ne peut pas rendre l'app aveugle chez un testeur, ce que la
règle protège) — mais c'est une exception à une règle 🔴, et le projet refuse qu'une règle
soit amendée en silence.
**How to apply:** à tout diff qui touche `HORS_SCAN`, vérifier que la dérogation reste
bornée au dossier (`startsWith` avec slash final) et que `examples/` ne contient que des
binaires lancés à la main — si l'app elle-même appelait cette logique, l'exemption
redeviendrait un angle mort. Voir [[verifier-les-assertions-de-matiere-reelle]].
