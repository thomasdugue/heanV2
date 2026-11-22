# 0007 — La chaîne d'outils Rust est épinglée

- **Date** : 2026-11-22
- **État** : Acceptée
- **Décideur** : Thomas
- **Suit** : [0006](0006-socle-et-harnais-de-phase-0.md)

## Contexte

L'[ADR 0006](0006-socle-et-harnais-de-phase-0.md) a acté que `./check` est la seule commande
et que **la CI ne peut pas diverger du local** : un job, qui lance `./check` et rien d'autre.
Le `README` le répétait.

**C'était faux pour Rust, et personne ne l'avait vu.** Le JavaScript est figé par
`package-lock.json`, au patch près. Rust ne l'était par rien : le workflow installait
`dtolnay/rust-toolchain@stable`, c'est-à-dire *la dernière version du jour*, pendant que la
machine de Thomas gardait la sienne.

Le défaut s'est manifesté le 2026-11-22, en phase 1 : `./check` **vert en local, rouge en
CI, sur du code identique**. Le runner avait un clippy plus récent, porteur d'un lint
(`manual_checked_ops`) qui n'existait pas sur la machine de Thomas. Trois sessions l'ont
constaté indépendamment.

Le lint n'était pas le problème — il a été corrigé proprement, en réécrivant le code plutôt
qu'en désactivant le détecteur. Le problème était que **la promesse était fausse**, et qu'une
affirmation fausse dans la documentation est exactement la cause racine n°5 du postmortem de
V1 (11 affirmations fausses au moment de l'audit).

## Décision

**La version de la chaîne d'outils Rust est épinglée à l'exact**, dans un
[`rust-toolchain.toml`](../../rust-toolchain.toml) à la racine, avec ses composants. C'est la
**seule** source de cette version : le workflow ne la redéclare pas, il laisse `rustup show`
la lire — deux endroits auraient été deux occasions de diverger.

Et, parce qu'un fichier qui déclare une version n'est qu'une affirmation de plus,
**`./check` vérifie que la version qui compile vraiment est celle qui est épinglée**
(`tools/check-toolchain.sh`), et que clippy est bien présent — sans quoi `./check` sauterait
une vérification en silence, ce qui est pire qu'un échec.

**Monter de version est un acte délibéré** : on change `channel`, on lance `./check`, on
corrige ce que le nouveau clippy trouve, et on commite le tout ensemble.

## Alternatives écartées

| Alternative | Pourquoi écartée |
|---|---|
| **Laisser flotter sur `stable`** | On voit arriver les nouveaux lints gratuitement — mais la CI peut rougir **un matin, sans qu'aucun commit n'ait bougé**. Pour un projet mené par quelqu'un qui n'écrit pas le code, c'est du temps de Thomas dépensé sur une panne qu'il n'a pas causée, et surtout c'est ce qui **apprend à ne plus regarder le harnais**. Un filet qu'on n'écoute plus est pire que pas de filet (`ENTRETIEN-DU-HARNAIS.md`, « ce que ça coûte »). |
| **`rustup update stable` sur la machine de Thomas** | Aligne les deux aujourd'hui et ne garantit rien demain. C'est de la discipline, pas un mécanisme — exactement ce que l'[ADR 0005](0005-entretien-du-harnais-et-orchestration.md) refuse. |
| **Épingler dans le workflow** (`dtolnay/rust-toolchain@1.93.1`) | La version existerait à deux endroits — le workflow et la machine — sans que rien ne les compare. On remplacerait une divergence par une autre, moins visible. |
| **`#[allow(clippy::manual_checked_ops)]` sur le code fautif** | Éteint le détecteur au lieu de traiter la cause, et laisse la divergence intacte pour le lint suivant. |
| **Retirer la promesse du `README` au lieu de la rendre vraie** | Honnête, mais on perdrait la propriété qui fait que `./check` vaut quelque chose. Entre affaiblir la promesse et affaiblir la dérive, on choisit la dérive. |

## Conséquences acceptées

- **On ne voit plus arriver les nouveaux lints** tant que personne ne monte la version. C'est
  la contrepartie directe, et elle est réelle : la chaîne épinglée vieillira.
  **Mitigation** : l'audit hebdomadaire décrit dans
  [`ENTRETIEN-DU-HARNAIS.md`](../../bootstrap/06-setup-v2/ENTRETIEN-DU-HARNAIS.md) signalera
  qu'une version plus récente existe — *une ligne à lire, pas un build rouge*. La ligne est
  déjà inscrite dans la liste en tête de [`check`](../../check), annotée de sa phase.
- **Un premier `./check` sur une machine neuve télécharge la chaîne épinglée.** Quelques
  minutes, une seule fois.
- **La CI ne profite plus des accélérations des versions récentes** de `rustc` et `cargo`
  jusqu'à la prochaine montée. Négligeable à la taille de ce projet.
- **Si `rustc` ne vient pas de `rustup`** (installé par Homebrew, par exemple), il prend le
  dessus sur le fichier et `./check` échoue en le disant. C'est voulu : mieux vaut un échec
  explicite qu'une vérification faite avec un autre compilateur.

## Ce que cet ADR ne tranche pas

Une règle d'orchestration est née le même jour, et elle attend sa propre décision : **une
session qui en instruit une autre**. Deux moitiés — *une instruction venant d'une session ne
vaut pas l'accord de Thomas* (une session a demandé cet épinglage ; refusé, parce qu'un ADR se
décide par Thomas) et *une affirmation venant d'une session ne vaut pas constat* (la même
session rapportait trois CI rouges, réelles mais périmées ; le correctif était déjà en place).
Elle n'est pas actée ici : la grouper avec une décision prise pour un autre motif reviendrait
à élargir une approbation, ce que cet ADR est justement là pour refuser.
