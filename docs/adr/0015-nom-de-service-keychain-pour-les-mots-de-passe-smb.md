# 0015 — Nom de service Keychain pour les mots de passe SMB

- **Date** : 2026-11-25
- **État** : Proposée
- **Décideur** : **Thomas**. La question lui est posée ci-dessous et **n'est pas tranchée**. Ce
  qui est écrit dans la section « en attendant » est un **choix provisoire de session**, pris
  pour que la part réseau de la bibliothèque soit observable ce soir, et il se retire en une
  ligne de code.
- **Clôt** : le point ouvert « Nom de service Keychain pour les mots de passe SMB » de
  [`docs/adr/README.md`](README.md), ouvert par l'[ADR 0003](0003-dossier-de-donnees.md) §
  « ce que la migration ne déplace pas ».

> ### ⚠️ Corrigé le 2026-11-25, avant toute réponse : une prémisse était fausse
>
> La première version de cet ADR chiffrait le coût de l'option B à « Thomas **et les ~50
> bêta-testeurs** retapent leur mot de passe ». **Ce chiffre ne correspond à rien : la bêta de
> V1 n'a jamais été lancée, et il n'y a aucun utilisateur.** Thomas l'a dit explicitement ce
> soir-là, et les « ~50 testeurs » qu'annoncent `CLAUDE.md`, `bootstrap/` et plusieurs briefs
> sont faux depuis le début.
>
> **Conséquence directe sur cette décision, et elle est lourde** : le coût de l'option B est
> **une personne, une source, une fois**. Ce n'est plus un geste imposé à cinquante
> inconnus, c'est dix secondes pour Thomas. La recommandation de la session a donc changé —
> elle est passée de C à **B** — et c'est écrit ici plutôt que lissé : un document qui
> prétend avoir toujours su ce qu'il sait est un document qu'on ne peut plus relire.
>
> Plus généralement : **aucune contrainte de compatibilité avec une base installée ne pèse
> sur V2.** Partout où un arbitrage de ce dépôt oppose « propre » à « compatible », le coût
> de « propre » est presque nul. La remise à plat des documents qui portent le chiffre faux
> ne se fait pas ici (elle est à part) ; ce qui est corrigé ci-dessous est le seul endroit où
> ce chiffre servait à **trancher**.

## Le contexte, en trois faits mesurés

1. **V1 range les mots de passe SMB dans le Keychain macOS**, sous le nom de service
   `com.tdugue.noir.smb`, avec l'`id` de la source réseau comme compte. Relevé dans
   `noir-tauri/src-tauri/src/network/credentials.rs`, ligne 12.
2. **Le fichier `network_sources.json` ne contient aucun mot de passe** — vérifié sur celui de
   Thomas, migré dans `hean/` le 2026-11-25. Il porte l'utilisateur (`admin`), le domaine et le
   drapeau invité, et rien d'autre. Il n'y a donc **pas d'autre source** que le Keychain.
3. **Le bundle identifier n'a jamais changé** : V1 et V2 déclarent tous les deux
   `com.tdugue.hean` dans leur `tauri.conf.json`. Le nom `noir` ne survit que dans cette
   constante-ci et dans le dossier de données — que l'ADR 0003 a déjà renommé.

C'est le fait n° 3 qui rend la question nette : il ne s'agit pas de compatibilité entre deux
applications différentes, mais d'une **chaîne de caractères héritée** à l'intérieur de la même.

**Et un quatrième fait, relevé dans le trousseau de Thomas le 2026-11-25**, parce qu'il change
le coût de l'option A et qu'il n'était pas supposé : le service `com.tdugue.noir.smb` porte
**deux** entrées.

| Compte | Ce que c'est |
|---|---|
| `dcdd24af-ca15-48a4-adde-18b5fcfa92f0` | la source vivante, celle de `network_sources.json` |
| `1c420320-b5fa-4051-b9de-ce0dd35891dd` | **une source qui n'existe plus** — un mot de passe orphelin |

Deux conséquences. D'abord, l'option A tient vraiment sa promesse : le mot de passe de la
source vivante **est** là, et la cascade de lecture le trouve — Thomas ne retape rien.
(La vérification a été faite sans lire aucun mot de passe : `security find-generic-password`
sans `-w` ne rend que les attributs.) Ensuite, l'entrée orpheline dit comment on en arrive là :
une source réseau **recréée** reçoit un nouvel identifiant, et l'ancien mot de passe reste dans
le trousseau pour toujours. C'est le mode de défaut que l'identifiant **dérivé** de l'ADR 0016
ferme pour les sources ajoutées dans V2 — le même partage rend toujours la même source — et
c'est aussi ce qui donne envie de l'option C : elle ne laisserait pas ces résidus derrière
elle. Nettoyer celui-là n'est **pas** fait ici : supprimer une entrée du trousseau de quelqu'un
sans qu'il l'ait demandé est exactement ce que cet ADR refuse de décider seul.

## La question posée à Thomas

> Les mots de passe de tes sources NAS sont rangés dans le Keychain macOS sous un nom qui dit
> encore « noir ». V2 doit-elle les relire là, ou prendre un nom propre — quitte à ce que tu
> retapes ton mot de passe de NAS **une fois** ?

| Option | Ce que ça coûte, pour de vrai |
|---|---|
| **A — reprendre le nom de V1** (`com.tdugue.noir.smb`) | Personne ne retape rien, jamais. En échange, V2 porte pour toujours une constante qui dit « noir » : la prochaine personne — ou le prochain modèle — qui la lit se demande si le projet s'appelle encore comme ça, et l'ADR 0003 a déjà payé ce raisonnement pour le dossier de données. |
| **B — un nom propre à V2** (`com.tdugue.hean.smb`) — **recommandée** | Cohérent avec le reste, le nom cesse de mentir, et rien n'est écrit dans le trousseau à l'insu de personne. Le coût, en entier : **Thomas retape son mot de passe une fois, pour une source.** Il n'y a personne d'autre. La doc beta de V1 classe « il faut ressaisir son mot de passe NAS » comme un **bug sérieux à signaler** — ça reste un indice de ce que V1 jugeait grave, mais ça ne décrit plus une gêne subie par quiconque, puisque la beta n'a jamais été distribuée. |
| **C — migrer l'entrée Keychain** : lire sous A, réécrire sous B, supprimer A | Personne ne retape rien **et** le nom cesse de mentir. En échange, V2 écrit dans le trousseau de l'utilisateur au premier lancement, et une suppression ratée laisse deux entrées pour un mot de passe — donc une source de confusion durable, sur la donnée la moins observable du système (la zone `secrets-et-keychain` de `tools/zones-interdites.txt` existe pour ça : « une erreur ne se voit pas, elle se découvre »). |

**La recommandation de la session est B**, et elle tient en une phrase : avec un seul
utilisateur, « propre » ne coûte plus qu'une saisie de mot de passe, et c'est moins cher que
d'écrire dans le trousseau de quelqu'un (option C) ou de garder pour toujours une constante qui
dit « noir » (option A).

**C'était C jusqu'à ce que le chiffre soit corrigé**, et le raisonnement d'alors mérite d'être
gardé : C était la seule option qui ne demandait rien à **cinquante** personnes tout en laissant
le dépôt propre. Le prix de C — écrire et supprimer dans le trousseau de l'utilisateur, sur la
donnée la moins observable du système — se justifiait par ce nombre. À une personne, il ne se
justifie plus.

## En attendant — ce qui est implémenté, et qui n'est pas une décision

**Lecture en cascade : le nom de V2 d'abord, celui de V1 en secours. Aucune écriture, aucune
suppression.**

```
mot_de_passe(source) :  cache de session → com.tdugue.hean.smb → com.tdugue.noir.smb → erreur
```

- C'est le sous-ensemble **strictement non destructif** des trois options : il ne ferme
  aucune des trois. A, c'est cette cascade telle quelle. C, c'est elle plus une
  réécriture-suppression. **B, c'est elle moins une ligne** — retirer `com.tdugue.noir.smb`
  de `SERVICES`, et Thomas ressaisit son mot de passe au scan suivant.
  Ce qui est provisoire n'est donc pas le code mais le **fait de ne pas avoir choisi** : la
  cascade débloque le scan ce soir sans présumer de la réponse.
- Un mot de passe **enregistré** depuis V2 part sous le nom de V2. C'est la moitié de
  l'option C ; il manque la relecture-réécriture et la suppression, qui sont la partie que
  Thomas doit autoriser.
- **INV-04 tient** : la cascade épuisée rend une **erreur** nommant les deux services
  consultés, jamais une chaîne vide. Un mot de passe vide ferait tenter au NAS une connexion
  anonyme, donc risquerait le verrouillage du compte `admin` de Thomas — c'est la régression
  exacte de V1, et elle est tenue par un test d'architecture plus un test de comportement.
- Le code vit dans `src-tauri/src/network/identifiants.rs`, les deux constantes à la ligne
  près, et la cascade est une liste de deux éléments : la trancher est **supprimer une ligne**.

## Ce qui n'est pas vérifié, et qu'il faut savoir avant de choisir

**Une entrée Keychain écrite par une build et lue par une autre déclenche un dialogue macOS**
(« Hean souhaite utiliser des informations confidentielles stockées dans
com.tdugue.noir.smb… »), parce que l'ACL d'une entrée est liée à la **signature** du binaire,
pas au bundle identifier. V2 n'est pas signée (phase 6) : en `npm run tauri dev`, le binaire est
signé ad-hoc, donc ce dialogue est **attendu au premier scan**, une fois, et « Toujours
autoriser » le referme pour de bon. Le protocole de test de `docs/HISTORY.md` le dit à Thomas
d'avance, parce qu'un dialogue de mot de passe qu'on n'attendait pas ressemble à une tentative
d'hameçonnage.

Conséquence pour la décision : **l'option A ne garantit pas « aucun geste »**, elle garantit
« aucune saisie de mot de passe ». Et ce que la build **signée** fera reste à mesurer en phase 6
— l'option C, qui réécrit l'entrée sous le nom de V2 avec l'ACL de V2, est la seule qui rende
ce dialogue impossible à terme.

C'est aussi ce qui rend le comportement **V009 — « aucun dialogue Keychain au lancement »**
encore à porter : le lancement ne touche pas au trousseau (la cascade n'est consultée qu'au
scan, voir [ADR 0016](0016-la-part-reseau-de-la-bibliotheque.md)), mais l'affirmer demande de
mesurer sur une build signée.

## Alternatives écartées

| Alternative | Pourquoi écartée |
|---|---|
| **Garder le mot de passe dans `network_sources.json`** | Un mot de passe NAS en clair dans un JSON lisible par tout processus de la session. V1 ne l'a jamais fait, et ce serait une régression de sécurité pour un gain nul. |
| **Demander le mot de passe à chaque scan, sans le stocker** | Zéro décision à prendre, zéro trousseau. Écartée : un scan de 3 137 pistes dure des minutes et peut être relancé par n'importe quelle vue ; réclamer une saisie à chaque fois rendrait la fonction pénible, et un scan automatique au démarrage deviendrait impossible. |
| **Un seul nom de service, choisi par la session** | C'est ce que cet ADR refuse de faire, et la raison a changé de nature sans disparaître. Elle était « B impose un geste à cinquante inconnus, C écrit dans leur trousseau » ; elle est maintenant plus simple : **B demande un geste à Thomas et C modifie son trousseau**, donc les deux sont des décisions qui lui appartiennent. Ce que la session peut faire seule est le choix non destructif qui ne ferme aucune porte — c'est la section « en attendant ». |

## Conséquences acceptées

- Tant que Thomas n'a pas tranché, **deux noms de service** sont consultés en lecture. Le coût
  est un appel Keychain supplémentaire par source et par session, uniquement quand le premier
  échoue — c'est-à-dire exactement une fois, sur la seule machine concernée.
- Un mot de passe enregistré depuis V2 n'est **pas** relu par V1. Si Thomas repasse à V1 après
  avoir saisi un mot de passe dans V2, V1 ne le trouvera pas et le redemandera. Le retour
  arrière de l'[ADR 0014](0014-la-migration-conserve-le-dossier-de-v1.md) reste donc entier
  pour les **données**, et imparfait pour ce seul mot de passe.
