# Orchestration : un modèle qui décide, des modèles qui exécutent

> Question de Thomas, 2026-11-22 : *« je souhaite qu'il y ait un orchestrateur type Opus 5.5 capable
> de redistribuer les tâches à des modèles moins coûteux tout en garantissant un haut niveau de
> qualité et de maintenabilité. »*

## La réponse tient en une phrase

**On ne peut déléguer en sécurité que là où le harnais est fort.**

Ce n'est pas une formule : c'est la seule chose qui sépare une orchestration qui fait gagner du
temps d'une qui en fait perdre. Un modèle moins coûteux ne produit pas du travail *un peu moins
bon* — il produit du travail **qui a l'air bon**. La différence ne se voit ni à la lecture rapide,
ni en regardant l'application tourner. Elle se voit quand quelque chose casse trois semaines plus
tard.

Donc la question « quel modèle pour quelle tâche ? » n'a pas de réponse dans l'absolu. Elle en a une
seule :

> **Une tâche est délégable si, et seulement si, on peut écrire *avant* de la lancer le critère
> qu'une machine évaluera pour dire si c'est réussi.**

Pas « je relirai » — une machine, qui évalue, sans personne.

Si ce critère existe, le coût d'une erreur est nul : le test est rouge, on recommence. Si ce critère
n'existe pas, le coût d'une erreur est **le temps de Thomas**, et c'est la chose la plus chère du
projet. Entre les deux, il n'y a rien.

C'est pour ça que les deux questions de Thomas — l'entretien du harnais et l'orchestration — n'en
font qu'une. Chaque vérification ajoutée au harnais déplace la frontière du délégable. Le harnais
n'est pas un coût qu'on amortit : **c'est ce qui rend l'orchestration possible.**

---

## Qui fait quoi

Trois rôles, et ils ne se mélangent jamais. Surtout pas les deuxième et troisième.

### L'orchestrateur — Opus 5.5

Il ne code presque pas. Il fait les quatre choses qu'on ne peut pas sous-traiter :

1. **Tenir le contexte.** Il a lu le brief, les ADR, les invariants. Il sait pourquoi `--glass-lo`
   est dans `:root` et pourquoi CPAL est interdit.
2. **Découper.** Transformer « porter la page album » en tâches indépendantes, chacune avec son
   critère de réussite.
3. **Écrire le brief** de chaque tâche déléguée — c'est là que se joue tout le reste, voir plus bas.
4. **Juger le résultat.** Pas « est-ce que ça marche » (le harnais répond), mais **« est-ce que
   c'est la bonne solution »** : est-ce au bon endroit, est-ce que ça crée une dette, est-ce que ça
   aurait dû être trois lignes au lieu de trente.

Le point 4 est celui qui justifie le modèle cher. Un test vert ne dit rien de la maintenabilité, et
c'est précisément ce que Thomas demande de garantir.

### Les exécutants — Sonnet 5.5, Haiku 5.5

Ils reçoivent un brief autonome et un critère de réussite. Ils ne décident de rien. Ils ne lisent
pas le projet entier — ils lisent les fichiers que le brief nomme.

### Les vérificateurs — et ils ne corrigent jamais

Deux agents **en lecture seule**, qui existent déjà dans ce dépôt :

- **`test-runner`** (Haiku 5.5) — lance `./check`, rend un résumé court. Bon marché, mécanique.
- **`invariant-guardian`** (Opus 5.5) — confronte le diff aux règles « ne jamais casser » et rend
  un verdict BLOQUANT / AVERTISSEMENT / OK.

Qu'ils soient en lecture seule n'est pas un détail : un vérificateur qui peut corriger finit par
valider ses propres corrections.

---

## La table de routage

| Nature de la tâche | Modèle | Pourquoi |
|---|---|---|
| Arbitrage d'architecture, choix de modèle de données, ADR | **Opus 5.5** ou **Fable 5.2** | irréversible ou coûteux à défaire |
| Moteur audio, migration de données, pipeline de release, sécurité | **Opus 5.5**, jamais délégué | voir les zones interdites |
| Relecture de diff contre les invariants | **Opus 5.5** (`invariant-guardian`) | demande de juger, pas de vérifier |
| Implémenter une feature cadrée, avec ses tests écrits d'avance | **Sonnet 5.5** | périmètre clair, jugement limité |
| Porter un composant d'écran, avec sa capture de référence | **Sonnet 5.5** | le test de régression visuelle tranche |
| Écrire des tests à partir d'un comportement décrit | **Sonnet 5.5** | mécanique mais pas trivial |
| Renommage, bump de version, correction de typo, ajout de logs | **Haiku 5.5** | aucune ambiguïté |
| Lancer les vérifications et résumer | **Haiku 5.5** (`test-runner`) | aucune décision |
| Chercher où se trouve quelque chose dans le code | **Haiku 5.5** | lecture seule, résultat vérifiable |

**Le déclencheur, pas la taille.** Une tâche n'est pas déléguée parce qu'elle est petite, mais parce
qu'elle est **spécifiée et vérifiable**. Une correction d'une ligne dans le callback CoreAudio reste
chez Opus. Une migration de 400 lignes avec un test de non-régression part chez Sonnet.

---

## Le brief : là où tout se joue

Un brief incomplet ne produit pas un résultat incomplet — il produit un résultat **inventé**. Un
modèle à qui il manque une information ne s'arrête pas : il comble.

Un brief délégable contient **les six**, sans exception :

1. **Le but**, en une phrase, en termes de comportement observable.
2. **Les fichiers** à toucher, nommés. Et ceux à ne pas toucher.
3. **Le critère de réussite**, exécutable : *« `./check` vert, plus le test `X` qui est rouge
   aujourd'hui et doit passer au vert »*.
4. **Ce qui ne doit pas bouger** : les invariants concernés, cités.
5. **Le contexte minimal** pour ne pas avoir à chercher — la convention à suivre, l'exemple à imiter.
6. **La limite** : à partir de quoi il faut s'arrêter et rendre la main au lieu d'élargir.

**Si l'orchestrateur ne peut pas écrire le point 3, il n'y a pas de délégation.** Il écrit d'abord
le test — ou il fait la tâche lui-même.

Le point 6 a un corollaire que l'orchestrateur doit respecter : **une tâche déléguée ne se négocie
pas en cours de route.** Si l'exécutant découvre que le brief est faux, il s'arrête et rend la main.
Il ne « fait au mieux ».

---

## 🔴 Les zones qui ne se délèguent jamais

Pas parce que c'est difficile — parce qu'**un test vert n'y prouve rien**.

| Zone | Pourquoi aucun test ne suffit |
|---|---|
| **Le moteur audio** (CoreAudio, RingBuffer, callback temps réel) | Un défaut de bit-perfect ne fait échouer aucun test : ça joue, et un peu moins bien. L'oreille de Thomas est le seul juge, et la boucle de retour est trop lente pour un exécutant |
| **La migration des données** | Une migration ratée, c'est la bibliothèque de Thomas — la seule qui existe. Le test passe sur des fixtures ; la réalité a des chemins NFD et des caches à demi écrits |
| **Le pipeline de release** | Signature, notarisation, bundling des dylibs. Ça ne casse pas en CI : ça casse chez l'utilisateur, à l'installation |
| **Tout ce qui touche à un secret ou au Keychain** | Une erreur ne se voit pas. Elle se découvre |
| **Le choix de ce qu'on construit** | Ce n'est pas une tâche technique. C'est celle de Thomas |

Cette liste est en dur dans le fichier de routage. Un exécutant qui touche à un fichier de ces zones
fait échouer la vérification, même si le code est bon.

---

## Ce que ça fait gagner, honnêtement

Pas ce qu'on croit.

L'économie en argent est réelle mais secondaire : en pratique, 15 à 25 % des tours demandent un
modèle cher — le découpage, le brief, le jugement — et le reste est de l'exécution. Le gain se
mesure, il n'est pas spectaculaire.

**Le vrai gain est ailleurs : l'attention du modèle cher va là où elle compte.** Un modèle qui passe
sa session à renommer des variables et à lancer des tests n'a plus de contexte disponible quand
arrive la question qui engage six mois. C'est la ressource rare du projet — pas les jetons.

**Et le vrai coût, si c'est mal fait** : du travail plausible mais faux, détecté tard, qu'il faut
comprendre avant de le défaire. Une seule occurrence efface l'économie de plusieurs semaines. D'où
la règle du début, qui n'est pas négociable.

---

## Comment ça se met en place

Presque rien à construire : les pièces existent, il leur manque un contrat.

**En phase 0**, avec le reste du harnais :

1. Un fichier de politique de routage — la table ci-dessus, lisible par un modèle au début de chaque
   session, et un `.claude/agents/` par rôle avec son modèle écrit dans sa définition. Les deux
   vérificateurs (`test-runner`, `invariant-guardian`) sont déjà là.
2. Un **gabarit de brief** à six points, qui sert de checklist : pas de brief complet, pas de
   délégation.
3. La **liste des zones interdites** en dur, vérifiée mécaniquement.

**Ensuite, à chaque fois que le harnais gagne une vérification**, une catégorie de tâches devient
délégable. C'est le lien entre les deux documents, et c'est pour ça que
[`ENTRETIEN-DU-HARNAIS.md`](ENTRETIEN-DU-HARNAIS.md) vient en premier : sans lui, l'orchestration
n'a nulle part où s'appuyer.

> **Le signe que ça marche** : la part de tâches déléguées **augmente** mois après mois, sans que
> le nombre de défauts qui atteignent Thomas augmente. Si l'un monte avec l'autre, c'est que la
> frontière a été franchie trop vite — on resserre, et on ajoute la vérification qui manquait.
