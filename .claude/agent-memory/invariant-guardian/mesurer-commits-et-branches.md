---
name: mesurer-commits-et-branches
description: Comment re-mesurer les chiffres d'historique qu'un diff avance (commits de main, branches poussées, commits portant un fichier) — et les deux pièges qui font qu'un relevé honnête tombe à côté
metadata:
  type: reference
---

Les ADR du dépôt chiffrent de plus en plus souvent ce que l'historique expose. Deux pièges rendent
un relevé de bonne foi faux, et les deux se vérifient en une commande.

- **`git branch -r` compte les refs périmées.** Les branches fusionnées sont supprimées côté GitHub,
  mais les refs de suivi restent dans chaque worktree : 25 le 2026-11-27 contre **22** vraies, que
  seul `git ls-remote --heads origin | wc -l` donne. Un chiffre de branches mesuré localement est
  donc presque toujours trop haut.
- **Le `main` local n'est pas `main`.** Le 2026-11-27 il portait un commit jamais poussé
  (`eb7d32c`), donc 61 commits contre 60 sur `origin/main` — et toute mesure prise sur lui sortait
  décalée de un. **Ce qui se publie, c'est `origin/main`** : c'est la seule référence à interroger,
  comme `git ls-remote` et non `git branch -r` pour les branches. Même classe d'erreur que
  `git rev-list --all`, qui ajoutait des branches locales. Vérifier d'abord
  `git rev-list --count origin/main` contre `git rev-list --count main`.
- **Le dépôt fusionne en squash**, donc `main` gagne **un** commit par PR : un chiffre écrit pendant
  la PR et le même projeté après fusion diffèrent de un. Dire lequel on cite.

Commits portant un fichier donné (ici les captures) :
`for c in $(git rev-list origin/main); do git ls-tree -r --name-only $c -- <dossier> | grep -q '\.png$' && echo x; done | wc -l`.
⚠️ Mesuré trois fois le 2026-11-27 sur `origin/main` — par le dossier, par `01-home.png` seul, et en
exigeant les huit : **52 sur 60**, les trois fois. Les huit captures sont arrivées dans le même
commit, donc dossier et fichier donnent le même nombre ; la distinction ne mord que si un diff en
ajoute par lots. Le **53** relevé d'abord venait du `main` local, pas du dossier.

Auteurs : `git log origin/main --format='%ae' | sort | uniq -c` (37 commits à l'adresse personnelle
le 2026-11-27, 23 à l'adresse `noreply` — mesure exacte, elle a tenu).

Voir aussi [[verifier-les-comptes-dinvariants]] : même doctrine, un nombre écrit à la main que
personne ne recompte est un mensonge en attente (ADR 0005).
