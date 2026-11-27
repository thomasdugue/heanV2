---
name: perimetre-de-check-sizes
description: Périmètre réel de la limite de 400 lignes depuis l'inversion du 2026-11-23 — six zones, toutes extensions sauf exceptions écrites, corpus git ; les deux fichiers qui rougiront les premiers ; et l'angle mort sur les fichiers non commités
metadata:
  type: reference
---

`tools/check-sizes.sh`, **vérifié le 2026-11-25** (136 fichiers mesurés, verdict OK) :

- `ZONES=(src src-tauri/src tests src-tauri/tests src-tauri/examples tools check)` — les deux
  trous que je signalais (`src-tauri/tests` depuis le 2026-11-23, `src-tauri/examples` depuis
  le 2026-11-25) sont **fermés**.
- La liste d'extensions a été **inversée** le 2026-11-23 : on mesure tout, sauf une liste
  d'exceptions écrite (binaires, tables de données). Donc `.py` et `.sh` sont couverts — le
  `wc -l` manuel que je recommandais n'est plus nécessaire, le verdict du script suffit.
- Corpus = `git ls-files`, donc identique en local et en CI.

**Ce qui reste à savoir, et que le script ne dit pas tout seul** : il écrit d'avance qui
rougira le premier. Au 2026-11-25 : `src-tauri/src/migration/mod.rs` = **400/400, zéro
marge** (zone jamais déléguée), et `src-tauri/src/library/repository.rs` était à 398 avant
d'être redescendu à 368 par l'extraction de `reglages.rs` et `lignes.rs`. Un diff qui ajoute
dix lignes à `migration/mod.rs` est donc rouge par construction : ce n'est pas un faux
positif, c'est la règle qui réclame un découpage.

**Relire un diff non commité : le `wc -l` manuel redevient obligatoire.** Corpus
`git ls-files` veut dire que **tout fichier non suivi est invisible** à check-sizes. Mesuré le
2026-11-28 sur `claude/lecture-nas` : 162 fichiers mesurés, verdict OK, alors que trois
fichiers neufs (`network/flux.rs` 292, `network/client/octets.rs` 103, `audio/source.rs` 74)
n'étaient dans aucun commit — donc dans aucun compte. Les **tests d'architecture**, eux, les
voyaient : leur lecteur (`tests/architecture/_lecture.ts`) parcourt le **disque**, pas l'index
git (c'est ce que prouve `lecture-depots-imbriques.test.ts`, qui fabrique ses fichiers dans un
dossier temporaire). Donc sur un arbre de travail : INV-21 et les autres règles de forme sont
déjà jugées, la limite de 400 lignes ne l'est pas encore.

**Why:** la leçon de V1 porte sur la taille, pas sur l'extension ni sur le dossier — et
`./check` ne dit « OK » que sur ce qu'il regarde. Le périmètre a changé deux fois en trois
jours : il se relit, il ne se suppose pas.
**How to apply:** faire confiance au verdict du script pour la limite elle-même **quand le
travail est commité** ; sinon `wc -l` sur les fichiers de `git status --porcelain` en `??`.
Garder l'œil sur les deux fichiers tendus ci-dessus et sur
`src-tauri/src/library/repository.rs`, dont le découpage est contraint par INV-06 (lui seul
peut nommer la table des exclusions). Voir [[verifier-inv-21-completement]] pour la même
méthode appliquée aux autres garde-fous.
