---
name: hean-orchestration
description: Politique de routage des modèles pour heanV2 — qui fait quoi, le gabarit de brief à six points, et les cinq zones qui ne se délèguent jamais. Charger au début d'une session où l'on envisage de déléguer une tâche à un sous-agent, ou pour savoir si une tâche est délégable.
---

# hean-orchestration — déléguer sans perdre en qualité

## La règle, en une phrase

> **Une tâche n'est déléguée que si l'on peut écrire, AVANT de la lancer, le critère qu'une
> machine évaluera pour dire si c'est réussi.**

Pas « je relirai » — une machine, qui évalue, sans personne. Si ce critère existe, une erreur
coûte un test rouge et on recommence. S'il n'existe pas, une erreur coûte **le temps de
Thomas**, qui est la chose la plus chère du projet. Entre les deux, il n'y a rien.

La raison est que le travail d'un modèle moins coûteux n'est pas *un peu moins bon* : il
**a l'air bon**. L'écart ne se voit ni à la lecture rapide, ni en regardant l'app tourner. Il
se voit quand quelque chose casse trois semaines plus tard.

Corollaire : **on ne délègue en sécurité que là où le harnais est fort.** Chaque vérification
ajoutée à `./check` déplace la frontière du délégable. Le harnais se paie deux fois.

## Qui fait quoi

| Rôle | Modèle | Ce qu'il fait |
|---|---|---|
| **Orchestrateur** | Opus 5.5 | Tient le contexte, découpe, écrit les briefs, **juge la solution** (pas sa correction : ça, c'est le harnais). Ne code presque pas. |
| **Exécutants** | Sonnet 5.5, Haiku 5.5 | Reçoivent un brief autonome et un critère. Ne décident de rien. Ne lisent que les fichiers que le brief nomme. |
| **Vérificateurs** | `test-runner` (Haiku), `invariant-guardian` (Opus) | **Lecture seule.** Un vérificateur qui peut corriger finit par valider ses propres corrections. |

Le jugement de l'orchestrateur est ce qui justifie le modèle cher : un test vert ne dit rien
de la maintenabilité — est-ce au bon endroit, est-ce que ça crée une dette, est-ce que ça
aurait dû être trois lignes au lieu de trente.

## Table de routage

| Nature de la tâche | Modèle | Pourquoi |
|---|---|---|
| Arbitrage d'architecture, modèle de données, ADR | Opus 5.5 / Fable 5.2 | irréversible ou coûteux à défaire |
| Une des cinq zones ci-dessous | Opus 5.5, **jamais délégué** | un test vert n'y prouve rien |
| Relecture d'un diff contre les invariants | Opus 5.5 (`invariant-guardian`) | demande de juger, pas de vérifier |
| Feature cadrée, dont les tests sont écrits d'avance | Sonnet 5.5 | périmètre clair, jugement limité |
| Porter un composant, avec sa capture de référence | Sonnet 5.5 | la régression visuelle tranche |
| Écrire des tests à partir d'un comportement décrit | Sonnet 5.5 | mécanique, mais pas trivial |
| Renommage, bump de version, typo, ajout de logs | Haiku 5.5 | aucune ambiguïté |
| Lancer les vérifications et résumer | Haiku 5.5 (`test-runner`) | aucune décision |
| Chercher où se trouve quelque chose | Haiku 5.5 | lecture seule, résultat vérifiable |

**Le déclencheur est la vérifiabilité, pas la taille.** Une correction d'une ligne dans le
callback CoreAudio reste chez Opus. Une migration de 400 lignes avec un test de
non-régression part chez Sonnet.

## Le gabarit de brief — les six points, sans exception

Un brief incomplet ne produit pas un résultat incomplet : il produit un résultat **inventé**.
Un modèle à qui il manque une information ne s'arrête pas, il comble.

1. **Le but**, en une phrase, en termes de comportement observable.
2. **Les fichiers** à toucher, nommés. Et ceux à ne pas toucher.
3. **Le critère de réussite, exécutable** : « `./check` vert, plus le test `X` aujourd'hui
   rouge qui doit passer au vert ».
4. **Ce qui ne doit pas bouger** : les invariants concernés, cités par leur `INV-NN`.
5. **Le contexte minimal** : la convention à suivre, l'exemple à imiter.
6. **La limite** : à partir de quoi s'arrêter et rendre la main au lieu d'élargir.

**Si l'orchestrateur ne peut pas écrire le point 3, il n'y a pas de délégation.** Il écrit
d'abord le test, ou il fait la tâche lui-même.

Le point 6 a un corollaire : **une tâche déléguée ne se négocie pas en cours de route.** Si
l'exécutant découvre que le brief est faux, il s'arrête et rend la main. Il ne « fait au
mieux ».

## 🔴 Les cinq zones qui ne se délèguent jamais

Source de vérité : [`tools/zones-interdites.txt`](../../../tools/zones-interdites.txt).
`tools/check-zones.sh` vérifie que cette liste-ci dit la même chose — sinon un modèle lirait
une liste périmée sans que personne le sache.

- **zone:moteur-audio** — `src-tauri/src/audio/**`, `src-tauri/src/coreaudio*`. Un défaut de
  bit-perfect ne fait échouer aucun test : ça joue, un peu moins bien.
- **zone:migration-donnees** — `src-tauri/src/migration/**`. Une migration ratée, c'est la
  bibliothèque de 50 testeurs.
- **zone:pipeline-release** — `scripts/release*`, `.github/workflows/release*`,
  `src-tauri/build.rs`. Ça ne casse pas en CI, ça casse chez l'utilisateur.
- **zone:secrets-et-keychain** — `src-tauri/src/secrets.rs`, `src-tauri/src/keychain*`. Une
  erreur ne se voit pas, elle se découvre.
- **zone:choix-produit** — `bootstrap/01-produit/**`, `docs/adr/**`. Ce n'est pas une tâche
  technique, c'est celle de Thomas.

**Ce que la machine peut et ne peut pas faire, honnêtement.** Aucun script ne sait quel
modèle a écrit une ligne. Ce qui est mécanique : la liste ne peut pas diverger entre ses deux
copies (`check-zones.sh`), et le hook de fin de tâche **pose la question** dès qu'un diff
touche une de ces zones. Le reste tient à cette règle-ci, lue en début de session.

## Le signe que ça marche

La part de tâches déléguées **augmente** mois après mois, sans que le nombre de défauts qui
atteignent Thomas augmente. Si les deux montent ensemble, la frontière a été franchie trop
vite : on resserre, et on ajoute la vérification qui manquait.
