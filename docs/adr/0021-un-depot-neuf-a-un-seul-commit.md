# 0021 — Le dépôt public est un dépôt neuf à un seul commit

- **Date** : 2026-11-27
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-27 — « Dépôt neuf ».
- **Complète** : [`0019`](0019-le-depot-passe-en-public.md)

## Contexte

L'ADR [0019](0019-le-depot-passe-en-public.md) a masqué les données sensibles dans l'arbre courant,
l'ADR [0020](0020-les-captures-sortent-du-depot.md) a retiré les 8 captures. Les deux restent
**cosmétiques** tant que l'historique les porte : le mot de passe app-specific Apple et le DSN
Sentry sont dans l'arbre de **tous** les commits de `origin/main` — le fichier qui les porte est là
depuis le commit initial, vérifié commit par commit — et les captures dans 53 sur 61.

Deux formes existaient pour fermer ça. **Mesuré le 2026-11-27, l'une des deux ne ferme rien.**
GitHub garde pour chaque pull request une référence `refs/pull/<n>/head` que le propriétaire du
dépôt ne peut pas supprimer : ni par `git push --delete`, ni par un `--force` sur `main`. Le dépôt
en porte **26**, et **25 d'entre elles donnent encore accès au mot de passe, au DSN et aux huit
captures**. Réécrire l'historique de `main` aurait donc nettoyé la vue par défaut en laissant les
objets atteignables par leur SHA, et les diffs des 24 PR fusionnées lisibles tels quels. (GitHub
purge ces objets sur demande au support, après coup, sans délai garanti — ce n'est pas une
opération qu'on tient par soi-même.)

## Décision

**Le dépôt public est un dépôt neuf, à un seul commit** : l'arbre de `main` masqué, sans aucun
passé. `thomasdugue/heanV2` reste privé et garde tout — historique, PR, revues, issues.

Pourquoi le coût est faible, et c'est mesuré le 2026-11-27, pas supposé :

- **une seule PR ouverte** (#26, celle du masquage) ; 24 fusionnées, 1 fermée. Les 14 branches
  poussées qui paraissent « en avance » sur `main` le sont parce que les fusions sont des *squash*,
  pas parce qu'elles portent du travail non intégré. Mon estimation précédente — « 22 branches
  partiraient à refaire » — comptait des branches mortes.
- **le travail en cours tient en une branche** : `claude/coreaudio-sortie-8e47a4`, 3 fichiers et
  +96 lignes hors de `main`, en trois commits qu'aucune PR ne suit. ⚠️ Les 6 worktrees n'ont aucun fichier
  modifié, mais ça ne le prouvait pas : `git status` ne voit pas ce qui est commité, et le dépôt
  neuf n'ayant **aucun ancêtre commun**, une branche laissée derrière ne se fusionne plus — elle se
  rejoue par `git format-patch`. D'où la seconde condition de l'étape 1 du protocole.
- **l'historique raconté ne disparaît pas** : il est dans l'**arbre**, pas dans git — `HISTORY.md`,
  les 21 ADR, [`MESURES-V1.md`](../MESURES-V1.md). C'est la raison d'être de ces fichiers, et elle
  devient ici littérale.

La marche à suivre, dans l'ordre : [`PROTOCOLE-DEPOT-NEUF.md`](../PROTOCOLE-DEPOT-NEUF.md). Deux de
ses étapes ne se rattrapent plus une fois le dépôt public — `git config user.email` avant le commit
initial, et le dernier balayage de l'arbre — donc elle est écrite, pas improvisée.

## Ce que ça n'enlève pas

1. **La révocation reste à faire.** Le mot de passe et le DSN resteront dans le dépôt privé et dans
   ses 26 refs de PR. Privé n'est pas publié : l'exposition se limite au compte. Mais un
   justificatif qui a existé en clair se révoque — [0019](0019-le-depot-passe-en-public.md) § « ce
   que le masquage ne règle pas ».
2. **Dans le dépôt public, un secret qui entre ne sort plus** : sa première PR y créera une
   `refs/pull/1/head` aussi indestructible que les 26 d'ici. C'est l'argument de **INV-29** — aucun
   justificatif en clair dans un fichier suivi — proposé et pas encore tranché.
3. **Le nom « Thomas Dugué » reste** : auteur du commit initial, propriétaire du compte, et il
   figurera sur l'application signée.

## Alternatives écartées

**Réécrire l'historique de `main`.** Écartée sur une mesure, pas sur un principe : les 25 refs de
PR ci-dessus y survivent, donc l'opération coûte 22 branches et 6 worktrees à refaire **pour ne
rien nettoyer**. C'est le genre de travail qui donne l'impression d'avoir réglé le problème, ce qui
est pire que de ne pas l'avoir fait.

**Passer le dépôt actuel en public tel quel**, ce que 0019 décidait avant cette mesure. Publie
l'historique entier et les 26 refs de PR, c'est-à-dire exactement tout ce que 0019 et 0020 viennent
de retirer de l'arbre.

**Garder les deux dépôts en miroir** — le privé pour le travail, le public pour la vitrine.
Écartée parce qu'elle manque le but : la CI gratuite est **la** raison de la bascule (0019), et des
runners gratuits ne servent qu'au dépôt qui **reçoit les PR**. Un miroir laisse la facture
intacte. Le dépôt neuf devient donc le dépôt de travail.

## Conséquences acceptées

- **L'historique des PR n'est pas public.** Les 24 PR fusionnées, leurs revues et les verdicts du
  guardian restent côté privé. Ce qu'elles ont **décidé** est dans les ADR et `HISTORY.md` ; ce
  qu'elles ont **discuté** se perd pour un lecteur extérieur. C'est le prix, et il est connu.
- **`git blame` ne remonte pas avant le commit initial** côté public. Pour une archéologie, c'est
  le dépôt privé qu'on ouvre.
- **Le dépôt privé ne se supprime pas** : il devient la seule copie de l'historique, et personne
  d'autre n'en a de clone (ADR [0017](0017-ce-que-thomas-a-tranche-le-25-novembre.md) : aucun
  utilisateur, aucun tiers).
- **Le dossier de travail change** : on clone le dépôt neuf à côté, et
  `$HOME/Documents/Thomas/heanV2` devient l'archive. On ne rebranche pas l'ancien — son `main` ne
  correspondrait à rien côté serveur, avec 25 références de suivi périmées. Les six worktrees sont
  abandonnés, et c'est gratuit : aucun ne porte de fichier modifié.
- **La bascule reste réversible dans le même sens que 0019** : le dépôt neuf repasse en privé en un
  clic. Ce qui ne l'est pas, c'est ce qui aura été lu, cloné ou indexé entre-temps.
