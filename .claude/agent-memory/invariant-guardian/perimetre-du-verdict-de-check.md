---
name: perimetre-du-verdict-de-check
description: Ce que ./check fait ROUGIR et ce qu'il se contente d'afficher — tests/ux est hors verdict, donc un comportement porté qui régresse laisse ./check vert ; à revérifier à chaque diff qui porte un comportement
metadata:
  type: reference
---

Dans `./check`, le bloc « comportements UX » capture la sortie de `npm run test:ux` dans une
variable **sans jamais toucher `fail`** : c'est un tableau de bord, pas un verdict. Le verdict,
c'est `npm run test`, dont le périmètre est `vitest.config.ts` — `tests/architecture/**`,
`tests/unit/**`, et depuis la phase 2 `tests/ux/contrat-complet.test.ts` **uniquement**.

Conséquence à vérifier à chaque diff qui **porte** un comportement (remplace un `aPorter('Vxxx')`
par de vraies assertions) : le test porté vit dans `tests/ux/`, donc hors verdict. S'il régresse,
`./check` reste VERT et le tableau de bord le recompte en « reste à porter » (`24 - rouges`) — la
régression est relabellisée en dette. Seul le protocole de session (test-runner au début et à la
fin) l'attrape, c'est-à-dire de la discipline, ce que l'ADR 0005 refuse.

Les deux questions à poser sur un diff de phase :
1. quels comportements sont désormais portés, et sont-ils dans le périmètre de `vitest.config.ts` ?
2. le backend du comportement est-il doublé en Rust (`src-tauri/tests/`, lui dans le verdict via
   `cargo test`) ? Si oui le trou est partiel ; sinon il est total.

Au 2026-11-22 (phase 2) : 6 portés — V004, V012, V013, V014, V015, W006 — tous hors verdict ;
V015 et W006 doublés côté Rust.

**Why:** relecture de la phase 2 — c'est le seul affaiblissement trouvé dans un diff par ailleurs
exemplaire, et il ne se voit pas en lisant les tests, seulement en lisant le périmètre.
**How to apply:** même méthode que [[verifier-inv-21-completement]] — une règle n'est tenue que là
où le garde-fou regarde, et ce périmètre se vérifie au lieu de se supposer.
