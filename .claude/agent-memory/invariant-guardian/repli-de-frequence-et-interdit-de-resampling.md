---
name: repli-de-frequence-et-interdit-de-resampling
description: Le repli de fréquence est voulu par Thomas (porté de V1, affiché) alors que le skill et l'ADR 0007 §5 disent « le bit-perfect interdit de rééchantillonner » — vérifier l'état du conflit avant de juger
metadata:
  type: project
---

Depuis le 2026-11-26, `audio/coreaudio` **joue en repliant** quand le périphérique ne sait pas
servir la fréquence du fichier (`frequence::meilleure_frequence_supportee`, porté de
`find_best_supported_rate` de V1) au lieu de refuser la lecture. L'écart s'affiche : deux
fréquences et pas de badge bit-perfect (`negociation.rs` → `EtatLecture` → `Lecture.svelte`).

**Why:** c'est ce que faisait V1 (`audio_engine.rs:540` → `playback.js:862-903`), la parité est
la cible (ADR 0004), et refuser supprimerait une fonction qui marche. Mais trois endroits
écrivent encore l'inverse : `hean-invariants/SKILL.md` (« bit-perfect = pas de resampling »,
dans « les interdits qui ne se rouvrent pas »), l'ADR 0007 §5 (« donc de le refuser — puisque
le bit-perfect interdit de rééchantillonner ») et le Contexte du même ADR, qui prête la formule
à l'ADR 0002 — où elle **ne figure pas** (vérifié le 2026-11-26). L'ADR qui acte le
renversement n'est **pas sur cette branche** : il vit dans une PR ouverte (#24), pas encore
fusionnée, et son numéro était déjà pris quand j'en avais écrit un concurrent — retiré depuis.
Le numéro est volontairement absent d'ici : `tools/check-adr.mjs` refuse tout renvoi vers un
ADR qui n'existe pas encore dans l'arbre, et il a raison.

**How to apply:** vérifier l'état réel avant de juger un diff qui rééchantillonne. Si un ADR
acte désormais le repli et que la liste d'interdits du skill est corrigée, cette mémoire est
périmée : la supprimer. Sinon le conflit tient, et c'est un conflit de **traçabilité de
décision**, pas de comportement : la règle réelle est « bit-perfect quand le DAC sait décoder,
et le dire quand il ne sait pas ».
