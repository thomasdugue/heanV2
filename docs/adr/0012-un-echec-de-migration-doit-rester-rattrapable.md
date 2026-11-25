# 0012 — Un échec de migration doit rester rattrapable

- **Date** : 2026-11-23
- **État** : Acceptée
- **Décideur** : **Thomas pour la décision de corriger**, le 2026-11-23. Ses mots, puisque
  c'est ce qui rend l'attribution revérifiable : la relecture d'invariants de la PR #8 lui a
  rendu un verdict de quatre défauts 🔴, et il a répondu **« corrige les 4 défauts
  bloquants »**. Le découpage de `library/repository.rs`, proposé ensuite dans la même
  session, a reçu **« Non, plus tard »** — il n'est donc pas fait.
  **Les choix de méthode ci-dessous sont ceux de la session.** Même distinction que pour les
  [ADR 0009](0009-bibliotheque-cle-dalbum-et-source-de-verite.md) et
  [0010](0010-methode-de-migration-des-donnees-v1.md), et pour la même raison : « Thomas a
  décidé ceci » est ce qui fait qu'une règle ne se rediscute pas, donc ça ne s'écrit que
  quand c'est vrai.
  **Et ça s'écrit avec ses mots, pas avec les nôtres** — leçon du 2026-11-23, née d'un relais
  entre deux sessions : une attribution paraphrasée ne peut pas être revérifiée, donc elle se
  reformule à chaque passage. « Thomas a décidé d'étendre la limite de 400 lignes » et
  « Thomas a décidé d'étendre le périmètre de la règle des 400 lignes » sonnent pareil et
  disent le contraire : l'un affaiblit la règle, l'autre l'applique à plus d'endroits. Une
  citation entre guillemets se vérifie ; un résumé se déforme.
- **Remplace** : la conséquence acceptée de l'[ADR 0010](0010-methode-de-migration-des-donnees-v1.md)
  « la migration peut échouer sans empêcher l'application de démarrer ». Le reste de
  l'ADR 0010 — les quatre temps, l'inventaire explicite, la reprise de la configuration, la
  vérification à deux niveaux, le témoin — reste en vigueur.

## Contexte

L'ADR 0010 a écrit, en conséquence acceptée : « La migration peut échouer sans empêcher
l'application de démarrer. L'échec est journalisé en `error!` et la tentative reprend au
lancement suivant. Démarrer sur une bibliothèque vide serait pire que démarrer sur l'ancien
dossier intact. »

Le raisonnement était juste et **la propriété était fausse**. La relecture d'invariants du
2026-11-23 l'a mesurée en rejouant la séquence réelle de `run()` :

```
1er lancement, migration : Err(Copie { fichier: "covers/interdite.jpg", … PermissionDenied })
après Depot::ouvrir : …/hean existe = true, contenu : ["library.db"]
2e lancement (la cause de l'échec a disparu) : Rapport { etat: CibleDejaPeuplee, fichiers: 0 }
```

`run()` ouvrait la bibliothèque **quoi qu'il arrive**, y compris juste après l'échec. Or
ouvrir la bibliothèque crée le dossier de données et le fichier de base. Le lancement suivant
trouvait donc un dossier « déjà peuplé » — par la base vide que V2 venait de créer lui-même —
et renonçait **définitivement**. Un seul fichier de pochette illisible, un disque
momentanément plein, et les 572 Mo du dossier de V1 restaient en place, invisibles : la
reprise n'ayant jamais lieu, les pistes supprimées revenaient au premier scan. **INV-06
(🔴 BLOQUANT) violé à travers la migration, et de façon permanente.**

Deux choses expliquent que ce défaut ait survécu à une PR entière, et elles comptent plus que
le défaut :

1. **le test censé prouver la propriété ne la testait pas.**
   `un_fichier_illisible_fait_tout_annuler_sans_rien_supprimer` appelait
   `migrer_si_necessaire` deux fois de suite — ce que `run()` ne fait jamais, puisqu'il ouvre
   la base entre les deux. Le test concluait « le lancement suivant peut retenter » au sujet
   d'un programme qui n'existe pas ;
2. **la propriété vivait dans `run()`**, qui lance Tauri et n'est donc pas testable. Une
   propriété qu'aucun test ne peut atteindre est une phrase dans un document, et une phrase
   dans un document ne peut pas échouer — c'est la leçon fondatrice de ce projet, retrouvée
   ici dans le code le plus sensible qu'il contienne.

## Décisions

### 1. Tant qu'une migration reste à faire, rien n'est créé dans le dossier de données

Donc **un échec de migration empêche le démarrage**, et c'est l'inverse de ce que l'ADR 0010
avait accepté. La raison du renversement tient en une phrase : **ne pas démarrer est
rattrapable, démarrer ne l'est pas.**

- ne pas démarrer : les données sont intactes dans leur dossier d'origine, le message le
  **nomme** et dit que relancer réessaiera, et le lancement suivant réessaie vraiment ;
- démarrer : la bibliothèque est vide, le dossier de données est créé, et la migration est
  éteinte pour toujours — sans que l'utilisateur ait de quoi s'en apercevoir.

L'argument de l'ADR 0010 (« démarrer sur une bibliothèque vide serait pire que démarrer sur
l'ancien dossier intact ») est **retourné par la mesure** : démarrer ne laissait pas l'ancien
dossier intact et utilisable, il le condamnait. Le choix n'était pas entre « vide » et
« intact », il était entre « temporairement indisponible » et « définitivement perdu ».

Le message d'erreur est écrit pour être lu par quelqu'un qui ne lit pas de code : « la
migration de vos données depuis l'ancienne version a échoué, et Hean n'a donc rien créé : vos
données sont intactes dans <le dossier, nommé>. Relancer Hean réessaiera. »

**Réserve, relevée en relecture et assumée** : ce message n'a pas encore de canal pour
atteindre quelqu'un. L'échec se produit avant que Tauri ne démarre — donc ni fenêtre ni
dialogue — et `observability` n'écrit aujourd'hui que sur stdout, la journalisation sur
fichier étant datée de la phase 6. Pour un testeur qui double-clique l'application, elle
rebondit dans le Dock et s'arrête sans un mot visible. C'est le mode de défaut d'INV-21
appliqué au message censé le réparer. Le choix de ne pas démarrer reste le bon — les données
sont intactes, relancer réessaie — mais **la lisibilité de ce message dépend de la phase 6**,
et un dialogue natif avant de paniquer reste à trancher.

**Écarté** : démarrer en mode dégradé, sur une bibliothèque en mémoire. Ça demandait un
second chemin d'exécution pour toute l'application — donc un second ensemble de défauts
possibles — afin de rendre service dans un cas qui se résout en relançant.

### 2. Une bibliothèque vierge n'est pas une bibliothèque

Un dossier de données qui ne contient qu'une base **sans aucune piste, sans racine et sans
exclusion** est le reliquat d'un démarrage, pas le travail de quelqu'un. Il est écarté — la
base est effacée — au lieu de bloquer la migration.

C'est ce qui rattrape les machines où V2 a tourné une fois avant d'avoir cette migration :
celle de Thomas, et celles des testeurs qui recevraient une build intermédiaire. Sans cette
règle, la correction n° 1 ne les aurait pas sauvées : leur dossier de données existe déjà.

« Vierge » se vérifie par l'API publique du dépôt (`lister_pistes`, `racines`,
`chemins_exclus`) et non par une requête écrite dans le module de démarrage : un seul fichier
du projet nomme la table des chemins exclus (INV-06), et ce n'est pas celui-là.

**Ce qui ne change pas** : deux bibliothèques ne se fusionnent **jamais** (ADR 0010 §6). Une
base qui contient quoi que ce soit de réel arrête la migration, l'application démarre sur sa
bibliothèque, et l'anomalie est journalisée en `error!` — pas en `warn!` : quelqu'un a deux
dossiers de données et il faut que ça se retrouve dans un journal envoyé.

**Et un fichier caché n'est pas une donnée non plus.** Un seul `.DS_Store`, déposé par un
passage dans le Finder, suffisait à éteindre la migration, alors que l'inventaire ignore déjà
les fichiers cachés : les deux doivent s'accorder. Ils sont **déplacés** dans le dossier
provisoire, jamais supprimés — l'échange a besoin d'une cible vide, ce n'est pas une raison
de détruire le fichier de quelqu'un, et ils retrouvent leur place après l'échange.

### 3. La séquence de démarrage est une fonction, pas le corps de `run()`

`src-tauri/src/demarrage.rs` tient l'ordre migration → ouverture → reprise, et
`src-tauri/tests/demarrage.rs` le rejoue **en entier**. C'est la correction du défaut de
méthode, et c'est la plus importante des trois : tant que la séquence vivait dans `run()`,
aucun test ne pouvait l'atteindre, et les tests qui croyaient le faire en testaient une
imitation.

Règle qui en sort, et qui dépasse la migration : **un test qui saute une étape de la séquence
réelle prouve quelque chose au sujet d'un programme qui n'existe pas.** Quand une propriété
porte sur un enchaînement, c'est l'enchaînement qu'il faut appeler.

### 4. Une version de schéma livrée ne se modifie plus

La table `reprises` avait été ajoutée **dans la migration version 1**, déjà présente sur
`main`. Le mécanisme ne rejoue jamais une version appliquée — c'est sa raison d'être, et un
test le prouve — donc la table n'apparaissait jamais dans une base existante : la reprise
échouait à chaque démarrage sur « no such table: reprises », et les exclusions de V1
n'étaient jamais reprises. Elle est maintenant la version 2, et deux tests tiennent la
règle : l'un reconstitue une base au schéma de la version 1 et vérifie que l'ouverture la
rattrape, l'autre refuse que le mot `reprises` réapparaisse dans la version 1.

### 5. Un fichier qu'on n'a pas su lire ne prouve rien

Un `config.json` illisible ne pose plus le marqueur de reprise. V1 écrivait ses six fichiers
JSON **sans écriture atomique** : un fichier tronqué est le cas réel, pas l'exception. En
conclure « cet utilisateur n'avait aucune exclusion » faisait revenir ses pistes supprimées,
et **pour toujours**, puisque le marqueur interdit toute nouvelle tentative.

La distinction qui manquait est écrite dans le code : **« lu, rien dedans » n'est pas « pas
lisible »**. La première conclusion est légitime, la seconde ne doit jamais être tirée.

Le test qui affirmait l'inverse portait sa justification (« on ne retentera pas éternellement
de lire un fichier abîmé ») : l'échange troquait un journal bruyant contre une violation
silencieuse d'INV-06. **Un test porté verrouille autant un bug qu'une règle**, et le prochain
lecteur aurait respecté celui-là.

### 6. La reprise pose les exclusions avant les racines

Chaque insertion est sa propre transaction, et un échec de reprise n'empêche pas le
démarrage. Avec les racines en premier, l'état atteint était « des dossiers de musique et
aucune exclusion » : le premier scan ramenait les pistes supprimées. Dans l'autre ordre, le
pire état possible est « des exclusions et aucune racine » — rien à scanner, donc rien à
ressusciter — et le démarrage suivant reprend où on s'est arrêté.

**Écarté** : une transaction unique pour les deux listes. Elle serait plus satisfaisante,
mais elle demande une méthode de plus sur le dépôt alors que l'ordre ferme déjà complètement
le mode de défaut. La propriété est tenue par un test de forme avec son auto-test, éprouvé
par injection dans le vrai fichier : elle n'est pas observable à l'exécution, puisqu'elle ne
se manifeste que si une insertion échoue au milieu.

## Conséquences acceptées

- **Un échec de migration empêche Hean de démarrer**, jusqu'à ce que sa cause disparaisse.
  C'est le prix de la règle 1, et c'est le bon prix : la cause la plus probable est un disque
  plein ou une permission, toutes deux réparables, et relancer suffit.
- **Une base vierge est effacée** avant la migration. Une base sans piste, sans racine et
  sans exclusion ne contient rien à perdre ; si sa suppression échoue, la migration retombe
  simplement sur l'ancien comportement — « cible déjà peuplée » — jamais sur une perte.
- **Un `config.json` définitivement abîmé fait échouer la reprise à chaque démarrage.**
  L'erreur est journalisée, l'application démarre, et l'utilisateur peut ajouter ses dossiers
  à la main. L'information perdue l'est de toute façon : ce qui change, c'est qu'on ne le
  cache plus.
- **Un nouvel invariant** : [INV-24](../../.claude/skills/hean-invariants/SKILL.md) — rien
  n'est créé dans le dossier de données avant que la migration ait rendu son verdict. Classé
  🔴 BLOQUANT par défaut, en attente du classement de Thomas, et couvert par
  `src-tauri/tests/demarrage.rs`.
