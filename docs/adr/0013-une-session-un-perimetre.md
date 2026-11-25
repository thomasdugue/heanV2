# 0013 — Une session, un périmètre ; et aucune session n'en démarre une autre

- **Date** : 2026-11-23
- **État** : Acceptée
- **Décideur** : Thomas
- **Suit** : [0005](0005-entretien-du-harnais-et-orchestration.md)

> **Pour Thomas, en clair.** Ce soir, deux sessions se sont mises à écrire dans le même
> fichier sans le savoir, et trois sessions sur cinq n'avaient été demandées par personne :
> une session en avait démarré une autre, qui en avait démarré une troisième. Rien n'a été
> perdu, parce que quelqu'un regardait. Cette décision pose la frontière qui manquait :
> **une session ne démarre pas de session, et deux sessions n'écrivent jamais dans le même
> fichier.** Elle décrit aussi le détecteur qui rendra la règle mécanique — il reste à
> écrire, et c'est dit ici explicitement plutôt que promis ailleurs.


> **Renuméroté de 0010 en 0013 le 2026-11-24.** Cet ADR est né sous le numéro 0010, qui
> était déjà pris par la PR #8, ouverte et non fusionnée. `tools/check-adr.mjs` ne pouvait
> pas le voir : il ne lit que la branche courante. C'est le trou que l'[ADR 0011](0011-derogation-inv-21-pour-les-exemples.md)
> décrit et la raison pour laquelle la règle de numérotation dit désormais « ce tableau **et**
> les PR ouvertes ». Une autre session l'a relevé. L'ironie mérite d'être écrite plutôt que
> tue : **la décision contre le chevauchement entre sessions a été rédigée en chevauchant une
> PR ouverte.** Seul le rangement change.

## Contexte

Le 2026-11-23 au soir, **cinq sessions** ont travaillé en parallèle sur ce dépôt. Deux
avaient été demandées par Thomas : la relecture de la PR #8 et la passe de vérité. Les
trois autres étaient nées d'elles-mêmes :

- `passe-verite` a démarré `champ-format-mpeg` pour corriger un mot d'affichage ;
- `champ-format-mpeg` a démarré `check-sizes-ne-voit-pas` pour un trou de détecteur ;
- cette troisième génération s'est mise à modifier `tools/check-sizes.sh` **pendant que**
  `harnais-detecteurs` y écrivait sa correction. Aucune des deux ne savait que l'autre
  existait.

L'écrasement a été évité parce qu'un humain lisait les rapports et a interrompu la plus
jeune avant son premier `git push`. C'est le mode de récupération le plus fragile qui
soit : il dépend d'une personne éveillée au bon moment.

**Le même mode de défaillance avait frappé la veille.** Deux sessions avaient attribué le
numéro `0007` à deux décisions différentes, chacune prenant « le suivant après 0006 » sans
voir l'autre ([ADR 0008](0008-epingler-la-chaine-doutils.md)). La réponse fut
`tools/check-adr.mjs`, et c'était la bonne réponse — mais pour **un seul fichier**. Le
fichier suivant que deux sessions se sont disputé n'était pas un ADR, et rien ne l'a vu.

Ce n'est donc pas un incident, c'est une **forme**. Et elle se reproduira : deux branches
vivantes en même temps est le mode de travail normal de ce projet
([ADR 0005](0005-entretien-du-harnais-et-orchestration.md)). Ce n'est pas le parallélisme
qui est en cause — c'est l'absence de frontière entre les parallèles.

Un dernier fait, qui pèse dans la décision : les trois sessions nées d'elles-mêmes
travaillaient toutes sur un sujet **réel**. Aucune n'avait tort sur le fond. Elles avaient
tort sur le droit de se lancer.

## Décision

**Trois règles, et un mécanisme qui les mesure.**

### 1. Une session ne démarre pas de session

Une session qui trouve un travail hors de son périmètre **l'écrit et le remonte** — dans sa
réponse, et dans le dépôt si c'est durable (« Bugs connus » de `CLAUDE.md`, ou les décisions
ouvertes de [`README.md`](README.md)). Elle ne le lance pas, et elle ne le fait pas non plus
elle-même.

**C'est Thomas qui décide de ce qui se lance.** Pas d'exception : une session n'a pas la vue
d'ensemble qui permet de savoir si un travail en double est déjà en cours ailleurs — c'est
précisément ce qui a manqué ce soir.

### 2. Une session = un périmètre, nommé dans son brief

Le brief qui démarre une session **nomme les fichiers ou dossiers qu'elle tient**. Ce qu'il
ne nomme pas, la session ne l'écrit pas : elle le remonte.

Un brief sans périmètre est un brief incomplet, au même titre qu'un brief sans critère de
réussite ([ADR 0005](0005-entretien-du-harnais-et-orchestration.md) : on ne délègue que ce
dont le succès s'écrit avant le lancement).

### 3. Deux sessions n'écrivent jamais dans le même fichier

Celle qui découvre le chevauchement **s'arrête et remonte**. Elle ne corrige pas, ne pousse
pas, n'écrase pas, et ne « fusionne » pas le travail de l'autre à sa place.

**Une exception, et elle est nommée** : les journaux en **ajout seul** — `docs/HISTORY.md`
et l'index [`docs/adr/README.md`](README.md). Chaque session y ajoute sa ligne ; le
chevauchement y est normal et se résout à la fusion. Tout autre fichier partagé est un
défaut.

### 4. Le mécanisme — `./check` mesure le chevauchement

Les trois règles ci-dessus sont, au sens de l'[ADR 0005](0005-entretien-du-harnais-et-orchestration.md),
du niveau **réclamé** : elles demandent qu'on y pense. C'est le niveau le plus faible, et
c'est exactement celui qui a échoué ce soir. Elles doivent donc devenir **appariées** : une
mesure, pas une phrase.

**Forme retenue**, à implémenter par la session qui tient `tools/` :

Pour chaque branche locale **dont les commits ne sont pas déjà en amont**, `./check` calcule
les fichiers qu'elle modifie par rapport à `main`, et **refuse** si un même fichier apparaît
dans deux branches, hors fichiers partagés par nature.

**La première rédaction de cette spécification était fausse, et la mesure l'a montrée.** Elle
disait « pour chaque branche locale vivante » et `git diff --name-only main...<branche>`.
Appliquée aux 8 branches vivantes du 2026-11-23, elle aurait crié sur **12 fichiers**, dont
deux où une branche se disputait avec **sa propre branche déjà fusionnée** : `main...<branche>`
montre ce qu'une branche a changé depuis son point de départ, pas ce qui en diffère encore,
donc une branche fusionnée en rebase ou en squash continue de déclarer ses fichiers pour
toujours. Ce dépôt fusionne en rebase et en squash : le détecteur aurait accumulé des faux
positifs **définitifs**.

**Les deux correctifs retenus :**

- **`git cherry main <branche>`** pour établir la liste des branches, et non l'énumération
  des branches locales. Il compare les *patch-id*, donc il sait qu'un commit rebasé est déjà
  en amont. Ça ferme le défaut sans rien connaître des PR ni appeler d'API.
- **L'exception ne couvre pas que les journaux en ajout seul.** Mesurés comme partagés par
  nature : `docs/HISTORY.md` (8 branches), le skill des invariants (6), l'index des ADR (5),
  les artefacts générés (4), `CLAUDE.md` (4). Chacun est nommé **un par un, avec sa raison
  écrite** — jamais un motif, qui exempterait par accident ce que personne n'a examiné.

**Ce qui tient de la forme de départ**, et c'est l'essentiel :

- Elle **n'entretient aucune liste de propriétaires**. C'est la même inversion que celle
  appliquée aux détecteurs de taille le 2026-11-23 : on mesure le réel au lieu d'énumérer
  les cas connus. Une liste de propriétaires pourrirait exactement comme la liste
  d'extensions qui a laissé passer un fichier de 569 lignes.
- Elle **ne lit aucun worktree**, seulement des branches. `tools/check-adr.mjs` porte déjà
  l'avertissement inverse en commentaire : lire les fichiers d'un worktree de session
  produit des faux positifs.
- Elle **ne dépend de rien hors de la machine** : pas d'API GitHub, pas de jeton, donc elle
  tourne à l'identique en local et en CI, comme l'exige
  l'[ADR 0006](0006-socle-et-harnais-de-phase-0.md). C'est la raison pour laquelle on
  préfère `git cherry` au pilotage par les PR ouvertes, qui réglerait le même défaut.

**Le piège, à ne pas perdre de vue** : un détecteur qui crie toujours est un détecteur qu'on
éteint — cause racine n°3 du postmortem de V1. La liste des fichiers partagés par nature doit
rester **courte et mesurée** ; si elle s'allonge à chaque session, c'est que la forme est
mauvaise et il faut revenir ici plutôt que l'étendre.

**Tant que ce détecteur n'existe pas, les trois règles restent réclamées.** C'est dit ici
pour que personne ne croie la frontière tenue par une machine alors qu'elle l'est par de
l'attention.

## Conséquences

- Les briefs de session gagnent une ligne obligatoire : le périmètre.
- Un travail trouvé en chemin ne se perd pas — il devient une ligne dans « Bugs connus » ou
  une décision ouverte, visible par la session suivante, au lieu d'une session de plus.
- Le nombre de sessions vivantes redevient une décision, donc une dépense, qui appartient à
  Thomas.
- `./check` gagnera une vérification. Elle est **planifiée**, pas active : elle se compte
  comme dette visible, selon la convention de l'[ADR 0006](0006-socle-et-harnais-de-phase-0.md).

## Ce qui n'est pas décidé ici

- **Le nombre de sessions simultanées.** Thomas a énoncé le 2026-11-23 une règle de travail
  — deux au plus, aucune session pour moins d'une heure de travail, fermeture dès la PR
  fusionnée — mais elle relève de sa conduite du projet, pas de l'architecture du dépôt.
  Elle n'est pas gravée ici et peut changer sans ADR.
- **Le sort d'un travail remonté.** Rien n'oblige Thomas à le lancer, ni à le lancer tout de
  suite. Une ligne dans « Bugs connus » qui dort un mois est un résultat acceptable ; une
  session qu'on n'a pas demandée ne l'est pas.
