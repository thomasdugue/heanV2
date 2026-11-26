---
name: derogation-examples-inv-21
description: État de la dérogation src-tauri/examples/ à INV-21 — réglée par l'ADR 0011 et en vigueur depuis le 2026-11-25 ; ce qu'il reste à vérifier à chaque diff qui touche HORS_SCAN
metadata:
  type: project
---

`tests/architecture/backend.test.ts` porte une liste `HORS_SCAN` des fichiers Rust exemptés
d'INV-21 : `src-tauri/build.rs` et `src-tauri/examples/`.

**Les deux points que je signalais sont réglés** (vérifié le 2026-11-25) :

1. L'exception à une règle 🔴 a son ADR : `docs/adr/0011-derogation-inv-21-pour-les-exemples.md`,
   tranché par Thomas le 2026-11-23. La phrase « toute exception passera par un ADR » du skill
   est donc tenue, pas contournée.
2. La note `muet` de la dérogation `examples/` a été retirée à l'arrivée de
   `examples/migrer.rs` : le test « en vigueur ou dite muette » a rougi au bon moment et a
   demandé le retrait. Le mécanisme a servi pour de vrai.

**Why:** la dérogation tient sur une propriété de Cargo (un exemple n'est pas dans le binaire
livré, donc il ne peut pas rendre l'app aveugle chez un testeur), pas sur la bonne conduite de
qui l'écrit — c'est ce qui la rend acceptable sur une règle 🔴.
**How to apply:** à tout diff qui touche `HORS_SCAN`, vérifier (a) qu'un motif de dossier finit
par `/` et un motif de fichier est exact (`couvrePar` — un `startsWith` généralisé ferait
couvrir `build.rs2.rs`), (b) que le nouveau dossier n'est pas appelé par l'app elle-même, (c)
que la liste `RUST()` de `backend.test.ts` (src + tests) et celle de
`une-seule-source-de-pistes.test.ts` (src + tests + **examples**) restent cohérentes : elles
diffèrent volontairement, et c'est un endroit où « simplifier » rouvrirait un angle mort.
