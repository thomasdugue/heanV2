---
name: perimetre-de-check-sizes
description: La limite de 400 lignes n'est appliquée que sur src, src-tauri/src, tests et tools — src-tauri/tests est hors scan, et bibliotheque.rs y dépasse déjà la limite
metadata:
  type: reference
---

`tools/check-sizes.sh` boucle sur quatre dossiers : `src`, `src-tauri/src`, `tests`, `tools`.
**`src-tauri/tests/` n'y est pas** — donc la règle « aucun fichier ne dépasse ~400 lignes »
n'y est pas tenue, et `./check` reste vert.

Au 2026-11-23 : `src-tauri/tests/bibliotheque.rs` = 487 lignes (416 avant cette branche),
`decodage.rs` 274, `bit_perfect.rs` 166. Le dépassement est donc **déjà réel** et il grossit à
chaque phase, puisque c'est le fichier où atterrissent les tests d'intégration de la bibliothèque.

Donc, à chaque diff qui ajoute des tests Rust d'intégration :
`wc -l src-tauri/tests/*.rs` — ne pas se contenter de `bash tools/check-sizes.sh`.

**Why:** même nature que [[verifier-inv-21-completement]] — `src-tauri/tests/` était aussi
l'angle mort d'INV-21 jusqu'au 2026-11-22. Un dossier oublié par un garde-fou le reste pour
tous les garde-fous écrits sur le même modèle.
**How to apply:** vérifier le périmètre du script, pas seulement son verdict. Et si le diff
ajoute `src-tauri/tests/` au scan, s'attendre à un rouge immédiat sur `bibliotheque.rs` : il
faudra découper avant, pas après.
