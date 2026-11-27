---
name: mesurer-commits-et-branches
description: Comment re-mesurer les chiffres d'historique qu'un diff avance (commits de main, branches poussées, travail hors de main, commits portant un fichier) — et les pièges qui font qu'un relevé honnête tombe à côté
metadata:
  type: reference
---

Les ADR du dépôt chiffrent de plus en plus souvent ce que l'historique expose. Deux pièges rendent
un relevé de bonne foi faux, et les deux se vérifient en une commande.

- **`git branch -r` compte les refs périmées.** Les branches fusionnées sont supprimées côté GitHub,
  mais les refs de suivi restent dans chaque worktree : 25 le 2026-11-27 contre **22** vraies, que
  seul `git ls-remote --heads origin | wc -l` donne. Un chiffre de branches mesuré localement est
  donc presque toujours trop haut.
- **Le `main` local n'est pas `main`** — et il le redevient. Le 2026-11-27 au matin il portait un
  commit jamais poussé (`eb7d32c`), donc 61 contre 60 : une mesure prise sur lui sortait décalée de
  un, et les documents ont été corrigés 61→60 et 53→52. **Le même commit a été poussé à 14:07**, et
  les corrections sont redevenues fausses dans l'autre sens, à recorriger 60→61 et 52→53. ⚠️ **La
  leçon n'est pas « mesurer sur `origin/main` », c'est que le nombre lui-même est le défaut** : il
  se périme au prochain push de n'importe qui. Quand l'argument tient sans lui — « dans *tous* les
  commits », vérifié commit par commit — l'écrire sans nombre. **Ce qui se publie, c'est
  `origin/main`** : la seule référence à interroger,
  comme `git ls-remote` et non `git branch -r` pour les branches. Même classe d'erreur que
  `git rev-list --all`, qui ajoutait des branches locales. Vérifier d'abord
  `git rev-list --count origin/main` contre `git rev-list --count main`.
- **Le dépôt fusionne en squash**, donc `main` gagne **un** commit par PR : un chiffre écrit pendant
  la PR et le même projeté après fusion diffèrent de un. Dire lequel on cite.
- **Trois points recomptent ce qui est déjà fusionné.** `git diff origin/main...branche` part de la
  **base de fusion**, qui est d'avant le *squash* : sur une branche dont la PR a été fusionnée puis
  qui a continué, il réaffiche tout ce que la PR avait apporté. Mesuré le 2026-11-27 sur
  `coreaudio-sortie` : **40 fichiers et +5 207 lignes** à trois points, **3 fichiers et +96** à deux
  points — le vrai. Pour « ce que cette branche a et que `main` n'a pas », c'est `git diff` à **deux
  points** (arbre contre arbre). ⚠️ Et il garde un fond de 400 à 1 100 lignes sur les branches
  mortes, qui est la dérive des fichiers qui grossissent (`HISTORY.md`, les ADR) : il **classe**, il
  ne désigne pas. Ce qui désigne le travail vivant, c'est la **date** — une branche fusionnée n'a
  pas de commit après sa fusion (`gh pr list --head "$b" --state merged --jq '.[0].mergedAt'`
  contre `git log -1 --format=%ct`). Les deux ensemble réduisent vingt-et-une branches à quatre.
- **`git status` ne voit pas le travail commité**, donc « zéro fichier modifié dans les worktrees »
  ne prouve pas « rien à sauver ». Écrit tel quel dans le protocole du 2026-11-27, corrigé le même
  jour : c'est la mesure facile prise pour la mesure demandée.
- **Vérifier « cette ligne est-elle dans `main` ? » ligne par ligne sur-rapporte.** Une entrée de
  `HISTORY.md` reformulée ou réindentée à la fusion est absente au `grep -F` et présente en fait.
  Lire ce que la ligne dit, puis le chercher par mots-clés.

Commits portant un fichier donné (ici les captures) :
`for c in $(git rev-list origin/main); do git ls-tree -r --name-only $c -- <dossier> | grep -q '\.png$' && echo x; done | wc -l`.
⚠️ Mesuré trois fois le 2026-11-27 sur `origin/main` — par le dossier, par `01-home.png` seul, et en
exigeant les huit : **52 sur 60** le matin, **53 sur 61** après le push de 14:07, les trois fois le
même nombre. Les huit captures sont arrivées dans le même commit, donc dossier et fichier donnent le
même résultat ; la distinction ne mord que si un diff en ajoute par lots. Toujours redater le
relevé : à six heures d'écart, le même calcul a donné deux réponses justes.

**Ce que GitHub garde et que `git` local ne montre pas** : une `refs/pull/<n>/head` par pull
request, que le propriétaire du dépôt **ne peut pas supprimer** — ni par `git push --delete`, ni par
un `--force`. Elles survivent donc à une réécriture d'historique, ce qui a décidé l'ADR 0021. Pour
les mesurer : `git ls-remote origin 'refs/pull/*/head' | wc -l`, puis
`git fetch origin 'refs/pull/*/head:refs/remotes/verif-pr/*'` pour inspecter leurs arbres — et
**supprimer ces refs ensuite** (`git update-ref -d`), sinon elles faussent `git rev-list --all` et
rapatrient en local les secrets qu'on cherchait à mesurer.

Auteurs : `git log origin/main --format='%ae' | sort | uniq -c` (37 commits à l'adresse personnelle
le 2026-11-27, 23 à l'adresse `noreply` — mesure exacte, elle a tenu).

Voir aussi [[verifier-les-comptes-dinvariants]] : même doctrine, un nombre écrit à la main que
personne ne recompte est un mensonge en attente (ADR 0005).
