---
name: un-compteur-sans-lecteur
description: Méthode de relecture — un compteur d'anomalie n'est une trace que s'il a un lecteur sur le chemin RÉEL ; grep ses lecteurs, pas sa définition
metadata:
  type: feedback
---

Quand un diff répond à « ce cas sort du silence **et il est compté** », vérifier qui **lit** le
compteur, et sur quel chemin d'exécution. Un `fetch_add` dans un callback plus un getter ne
prouvent rien : il faut un lecteur sur le chemin que l'application prend vraiment.

**Why:** relevé le 2026-11-26 sur `audio/coreaudio` — `tampons_inattendus` n'était lu que dans
`BackendAudio::terminer`, et le lecteur réel appelle `interrompre`, jamais `terminer`
(`terminer` ne sert qu'au backend WAV et aux tests). `demandes_trop_grandes` n'avait **aucun**
lecteur. Résultat : silence à l'écoute, zéro ligne de journal, rien à l'écran — la famille
d'INV-21 (🔴), un défaut qui ne laisse aucune trace, avec en plus l'illusion de couverture
donnée par un `tracing::warn!` posé sur un chemin mort.

**How to apply:** pour chaque compteur ou drapeau d'anomalie du diff,
`grep -rn <nom> src-tauri/src src/` et classer les occurrences : définition, incrément,
**lecteur**. Si tous les lecteurs sont dans une fonction que la production n'appelle pas, la
correction est incomplète — le dire, même si le compteur existe. Même logique que
[[perimetre-du-verdict-de-check]] : exister n'est pas être lu.
