# 0005 — Le harnais s'entretient par mécanisme, et l'orchestration suit le harnais

- **Date** : 2026-11-22
- **État** : Acceptée
- **Décideur** : Thomas

## Contexte

Avant de commencer la phase 0, Thomas pose deux questions : comment s'assurer que le harnais reste à
jour à mesure que le projet avance (specs, CI, tests du design system, *« et toutes les choses que
j'oublie »*), et comment faire travailler un orchestrateur coûteux avec des modèles moins chers sans
perdre en qualité ni en maintenabilité.

Ce sont deux faces d'un même problème. La V1 en a fourni la démonstration la veille : son
observabilité était un décor (252 `println!` contre 3 appels `tracing::`), et trois bugs tournaient
depuis des mois sans que rien ne les signale. Ce qui n'est pas vérifié mécaniquement n'est pas
vérifié — ni par un humain, ni par un modèle, cher ou non.

## Décision

**Deux mécanismes, actés, et liés par une seule règle.**

### 1. L'entretien du harnais se fait par niveau de force, jamais par discipline

Chaque artefact monte au niveau le plus haut possible : **généré** (produit depuis le code, la CI
échoue sur la dérive), **apparié** (deux sources, un test les compare), **réclamé** (un garde-fou
bloque et pose une question fermée, avec une porte de sortie explicite).

Le niveau 3 est une dette : **son nombre d'artefacts doit décroître avec le temps.** S'il croît, le
harnais devient une corvée et sera contourné.

Et une règle qui couvre ce qu'aucune liste écrite d'avance ne couvrira :

> **Tout défaut qui atteint Thomas est un trou dans le harnais.** Son correctif n'est accepté
> qu'accompagné de la vérification qui aurait dû le voir.

Détail, tableaux et contrat de fin de tâche :
[`../../bootstrap/06-setup-v2/ENTRETIEN-DU-HARNAIS.md`](../../bootstrap/06-setup-v2/ENTRETIEN-DU-HARNAIS.md).

### 2. La délégation à un modèle moins coûteux a un critère unique

> **Une tâche n'est déléguée que si l'on peut écrire, avant de la lancer, le critère qu'une machine
> évaluera pour dire si c'est réussi.**

Pas « je relirai ». Si le critère existe, une erreur coûte un test rouge ; sinon elle coûte le temps
de Thomas. Entre les deux il n'y a rien : un modèle moins coûteux ne produit pas du travail un peu
moins bon, il produit du travail **qui a l'air bon**.

Opus 5.5 tient le contexte, découpe, écrit les briefs et **juge la solution** — un test vert ne dit
rien de la maintenabilité. Sonnet 5.5 et Haiku 5.5 exécutent contre un critère. Les vérificateurs
(`test-runner`, `invariant-guardian`) restent **en lecture seule**.

Cinq zones ne se délèguent jamais, parce qu'un test vert n'y prouve rien : moteur audio, migration
de données, pipeline de release, secrets et Keychain, choix de produit.

Détail, table de routage et gabarit de brief :
[`../../bootstrap/06-setup-v2/ORCHESTRATION.md`](../../bootstrap/06-setup-v2/ORCHESTRATION.md).

### La règle qui les lie

**On ne délègue en sécurité que là où le harnais est fort.** Donc l'entretien du harnais passe
avant : sans lui, l'orchestration n'a nulle part où s'appuyer. Et chaque vérification ajoutée
déplace la frontière du délégable — le harnais se paie deux fois.

## Alternatives écartées

| Alternative | Pourquoi écartée |
|---|---|
| **Compter sur la discipline** — une checklist, un rappel dans `CLAUDE.md` | C'est ce que faisait V1, avec 19 règles en prose. Résultat : 11 affirmations fausses dans la documentation, le meilleur document design hors du dépôt, et une règle CSS violée 45 fois sans que personne le voie. Une phrase ne peut pas échouer. |
| **Énumérer d'avance tout ce qu'il faudra maintenir** | Produit la liste de ce dont on se souvient déjà, et donne l'illusion d'être couvert. L'inconnu ne se couvre que par une boucle qui apprend des défauts réels. |
| **Déléguer selon la taille de la tâche** (les petites aux modèles bon marché) | Critère séduisant et faux. Une correction d'une ligne dans le callback CoreAudio est indélégable ; une migration de 400 lignes avec test de non-régression ne l'est pas. Ce qui compte est la vérifiabilité, pas le volume. |
| **Un orchestrateur qui relit lui-même le travail des exécutants** | Déplace le goulot sans le supprimer : le modèle cher relit tout, donc on paie l'exécution deux fois et on perd le bénéfice. La relecture doit porter sur la **solution**, le harnais sur la **correction**. |
| **Construire un cadre d'orchestration sur mesure** | Les pièces existent (`.claude/agents/`, les hooks, `./check`). Il leur manquait un contrat, pas du code. |

## Conséquences acceptées

- **La phase 0 grossit** de deux éléments : les générateurs de niveau 1 avec la CI qui échoue sur la
  dérive, et le contrat de fin de tâche. Plus la politique de routage et le gabarit de brief. C'est
  du temps avant la première feature, et c'est assumé — c'est la raison d'être de la réécriture.
- **Un garde-fou qui bloque sera parfois pénible.** Il doit toujours offrir une sortie explicite,
  enregistrée. Sans porte de sortie, il est contourné en une semaine et c'est pire que rien.
- **La règle « un défaut = une vérification » coûte à chaque bug.** Le surcoût est faible, parce que
  le diagnostic vient d'être fait — mais il est systématique, et il faut s'y tenir quand ça presse.
- **L'économie financière de l'orchestration est modeste** (de l'ordre de 15 à 25 % des tours
  nécessitent le modèle cher). Le gain réel est l'attention du modèle cher, pas le budget. Décider
  sur la promesse d'économie mènerait à déléguer trop, donc à payer plus cher.
- **Une part du harnais ne sera construite que plus tard.** L'appariement arrive avec ce qu'il
  apparie : régression visuelle avec l'UI, budgets de performance avec la bibliothèque. Seuls les
  deux éléments ci-dessus sont en phase 0.
