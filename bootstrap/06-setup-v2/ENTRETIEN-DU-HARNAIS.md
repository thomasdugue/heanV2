# Comment le harnais reste à jour tout seul

> Question de Thomas, 2026-11-22 : *« je veux m'assurer qu'on a un système qui met à jour le harnais
> automatiquement à mesure que le projet avance — specs, CI, tests du design system, et toutes les
> choses que j'oublie. »*
>
> Réponse courte : **on ne peut pas garantir qu'un document soit à jour. On peut garantir qu'il soit
> impossible de le laisser périmer sans que ça sonne.** Ce document décrit le mécanisme, par ordre
> de force décroissante.

C'est le complément de [`HARNAIS.md`](HARNAIS.md), qui dit *quoi construire*. Celui-ci dit *comment
ça ne pourrit pas*.

---

## Le principe : trois niveaux de force

Un document maintenu par discipline finit périmé. Toujours. V1 l'a prouvé : 11 affirmations fausses
dans la documentation, et le meilleur document design du projet vivait **hors du dépôt**.

La seule question qui vaille pour chaque artefact du harnais est : **« à quel niveau peut-il
monter ? »** Et la réponse est toujours : le plus haut possible.

| Niveau | Mécanisme | Dérive possible ? |
|---|---|---|
| **1. Généré** | l'artefact est produit par un script depuis le code ; la CI régénère et échoue si ça diffère | **Impossible** |
| **2. Apparié** | deux sources décrivent la même chose, un test les compare | Impossible sans un test rouge |
| **3. Réclamé** | rien ne peut le déduire, donc un garde-fou **bloque** et pose la question | Possible, mais jamais en silence |

Tout ce qui est au niveau 3 est une dette. On y met le strict nécessaire, et on fait remonter d'un
niveau chaque fois qu'on trouve comment.

---

## Niveau 1 — ce qui se génère

Rien de ce qui suit ne doit jamais être écrit à la main. Script de génération, fichier commité,
CI qui régénère et compare.

| Artefact | Dérivé de |
|---|---|
| Catalogue des commandes IPC et de leur signature | les annotations `tauri-specta` du Rust |
| Catalogue des events et de leur payload | idem |
| **Types TypeScript du front** | idem — c'est le cœur de la décision stack |
| `tokens.css` et la page de tokens | le fichier de tokens source |
| Page de composants du design system | les composants eux-mêmes |
| Inventaire des fixtures audio | le dossier `fixtures/` |
| Tableau « invariant → test qui le protège » | les annotations dans les tests |
| Tableau « feature du contrat de parité → tests » | idem |
| Surface de l'état applicatif | le schéma de l'état |
| Schéma de la base | les migrations |

**Le critère pour savoir si c'est générable** : si deux personnes peuvent écrire deux versions
différentes du document en regardant le même code, ce n'est pas un document — c'est un bug en
attente. Génère-le.

## Niveau 2 — ce qui s'apparie

Deux sources disent la même chose sous deux formes. Un test les confronte.

| Source A | Source B | Le test échoue si… |
|---|---|---|
| `tokens.css` (la référence) | le CSS réellement servi | une valeur diverge |
| Captures de référence V1 | le rendu de V2 | l'écart dépasse le seuil |
| Contrat de parité (124 lignes) | les tests de comportement | une feature n'a aucun test qui la cite |
| Liste des invariants | les tests | un invariant n'a pas de test, ou un test cite un invariant qui n'existe plus |
| Budgets de performance | la mesure réelle en CI | un budget est dépassé |
| Liens des documents | l'arborescence | un lien ne résout pas *(déjà en place : `check-links`)* |
| Taille de `CLAUDE.md` | la limite de 150 lignes | dépassée *(déjà en place)* |

Deux de ces sept tournent déjà. Les cinq autres arrivent avec la phase qui les rend pertinents —
le tableau de `HARNAIS.md` dit laquelle.

## Niveau 3 — ce qui se réclame

Pour le reste — les décisions, le *pourquoi*, les règles nées d'une douleur — aucune machine ne peut
les déduire. Le seul mécanisme qui marche : **un garde-fou qui bloque et pose une question fermée.**

V1 avait déjà la bonne idée, appliquée à un seul artefact : le hook `invariant-reminder.sh` bloque la
fin de session quand du code a changé sans que le fichier d'invariants bouge, et demande « oui /
non ». **C'est le modèle à généraliser.**

### Le contrat de fin de tâche

Un seul garde-fou, qui regarde ce qui a changé et réclame ce qui manque :

| Si le diff touche… | …il doit aussi contenir |
|---|---|
| une commande IPC | les types régénérés |
| un composant d'interface | sa capture de référence mise à jour |
| les tokens ou le CSS | `tokens.css` régénéré |
| une feature du contrat de parité | la ligne cochée + au moins un test qui la cite |
| un correctif de bug | **un test qui échouait avant le correctif** |
| un choix structurant | un ADR |
| le pipeline de release | la checklist de release |
| n'importe quel code applicatif | la réponse « oui / non » à : « est-ce que ça crée une règle à ne jamais casser ? » |

La dernière ligne est la plus importante, et c'est exactement ce que fait déjà le hook de V1. Les
autres sont le même mécanisme appliqué aux autres artefacts.

**La règle d'or de ce garde-fou** : il doit toujours offrir une sortie explicite. « Non, pas de
règle ici » est une réponse valable — mais il faut la donner, et elle est enregistrée. Un garde-fou
sans porte de sortie est contourné en une semaine, et c'est alors pire que rien.

---

## L'audit périodique : pour ce qui échappe au diff

Certaines dérives ne se voient dans aucun diff, parce que rien n'a changé — c'est le monde autour
qui a bougé. L'endpoint de mise à jour de V1 renvoyait 404 **depuis le lancement**, et aucun diff ne
l'aurait jamais signalé.

Un job hebdomadaire, qui n'a le droit d'ouvrir qu'**une** issue, listant ce qui a dérivé :

- régénérer tous les artefacts de niveau 1 et signaler les écarts ;
- rejouer les mesures de performance et comparer aux budgets ;
- **faire un appel réel sur l'endpoint de mise à jour et vérifier le code 200** ;
- vérifier que chaque invariant a toujours son test, et chaque test son invariant ;
- lister les décisions ouvertes qui n'ont pas bougé depuis 30 jours ;
- lister les `TODO` plus vieux que 60 jours.

Une issue, une fois par semaine, lisible en deux minutes. Si elle est vide, elle ne s'ouvre pas.

---

## 🔴 Et « toutes les choses que j'oublie » ?

C'est la vraie question, et elle mérite une réponse honnête : **on ne peut pas les lister à
l'avance.** Toute tentative d'énumérer ce qu'on va oublier produit une liste de ce dont on se
souvient déjà.

Le seul système qui couvre l'inconnu est un système qui **apprend de la réalité** :

> **Tout défaut qui t'atteint est un trou dans le harnais.**
> Le corriger, ce n'est pas réparer le code — c'est ajouter la vérification qui aurait dû le voir.

Concrètement, quand un bug remonte : le correctif n'est accepté qu'accompagné du test qui échouait
avant lui. Pas d'exception, et le coût est faible parce que le diagnostic vient d'être fait.

C'est **exactement ainsi que les 19 invariants de V1 sont nés** — chacun a été payé par une
régression réelle. L'erreur de V1 n'était pas de les découvrir après coup : c'était de les écrire en
prose. Une phrase dans un document ne peut pas échouer. Un test, si.

Avec ce mécanisme, le harnais a une propriété qu'aucune liste écrite d'avance n'aura jamais :
**il grossit exactement là où le projet se casse**, et nulle part ailleurs. Au bout d'un an, il
protège ce qui est réellement fragile dans *ce* projet — pas ce qu'un modèle imaginait fragile au
départ.

Le corollaire, moins agréable : **le même bug ne doit jamais atteindre Thomas deux fois.** Si ça
arrive, ce n'est pas le bug qui est en cause, c'est le mécanisme.

---

## Ce que ça coûte, et ce qu'il ne faut pas faire

Le vrai danger n'est pas l'absence de harnais — c'est un harnais si lourd qu'on finit par le
contourner. Trois garde-fous sur le garde-fou :

| À éviter | Pourquoi |
|---|---|
| Un garde-fou qui bloque sans porte de sortie | Contourné en une semaine, et personne ne le dira |
| Plus d'une issue d'audit par semaine | Le bruit tue la lecture ; deux issues ignorées et le mécanisme est mort |
| Mettre les trois niveaux en place en phase 0 | Seuls le contrat de fin de tâche et les générateurs de niveau 1 sont en phase 0. L'appariement arrive avec ce qu'il apparie |
| Faire réclamer par le garde-fou ce qui est générable | Chaque question posée à un humain est un aveu d'échec du niveau 1 |

**La mesure du succès** : le nombre d'artefacts au niveau 3 **diminue** avec le temps. S'il
augmente, le harnais devient une corvée au lieu d'un filet.
