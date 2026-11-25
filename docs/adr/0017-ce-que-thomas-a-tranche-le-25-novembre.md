# 0017 — Ce que Thomas a tranché le 25 novembre : pas d'utilisateurs, et le harnais se gèle

- **Date** : 2026-11-25
- **État** : Acceptée
- **Décideur** : **Thomas**, en réponse à un audit de trajectoire mené par un modèle extérieur
  au projet. Ses mots sont cités là où ils tranchent, parce qu'une attribution qui ne se
  revérifie pas finit par être prêtée.

## 1. Le fait qui invalide des prémisses écrites partout

**« la beta V1 n'a pas été lancée, aucun user pour le moment »** — Thomas, 2026-11-25.

Le dépôt affirmait le contraire : `bootstrap/`, `README.md`, `tools/zones-interdites.txt`, le
skill `hean-invariants` et **quatre ADR** parlaient de « ~50 testeurs ». Le chiffre n'a jamais
correspondu à rien. Il a déjà faussé deux raisonnements le jour même : l'audit de trajectoire a
recommandé un correctif d'urgence sur V1 « pour ses cinquante testeurs », et l'ADR 0015 chiffrait
le coût d'un nouveau nom de service Keychain comme « Thomas **plus chaque testeur** ».

**Les ADR 0001, 0003, 0004 et 0010 s'appuient sur cette prémisse fausse.** Ils ne sont pas
modifiés — un ADR est immuable — mais aucune décision à venir ne doit s'y référer sur ce point.

**Conséquence, qui vaut pour tout le reste du projet : aucune compatibilité descendante à
tenir.** Changer un format de données, un identifiant de bundle, un nom de service ou un
protocole ne coûte qu'à Thomas, une fois. Là où un arbitrage hésitait entre « propre » et
« compatible », le coût de « propre » est désormais presque nul.

## 2. Périmètre produit

- **L'enrichissement Deezer est obligatoire** (genres B11, images d'artistes C05). L'audit
  proposait de le reporter après la première bêta V2 : refusé.
- **WiFi Sync sort du périmètre.** Pas un report à une phase tardive : une sortie. Le chantier
  reste décrit pour mémoire.
- **La cible de parité — 124 features ou ~85-90 — est reportée** jusqu'à ce que le son sorte des
  enceintes. Question posée, réponse : « elle peut attendre que le son sorte : OUI ».

## 3. L'ordre : le son, puis tout le reste

**« On va faire le son lorsque l'agent aura fini sur le NAS. »**

Le plan de reconstruction disait déjà « l'audio d'abord — c'est là qu'une erreur de conception
est irrattrapable », et listait `coreaudio_backend.rs`, `coreaudio_stream.rs` et `backend.rs`
« portés tels quels » en phase 1. Aucun ne l'a été : la phase 1 a été redéfinie en « décoder
vers un fichier » puis déclarée faite. **Le projet s'est écrit un garde-fou le 21 novembre et
l'a contourné dans la semaine** — c'est le constat le plus utile de l'audit.

Jalon posé avec la décision : **dans 14 jours, V2 joue un FLAC 24/192 sur le DAC de Thomas et un
album du NAS sans coupure.** Jalon manqué = la question « corriger V1 plutôt que réécrire » se
rouvre.

## 4. Porter d'abord, améliorer ensuite

**Redessiner un sous-système en le portant depuis V1 exige un défaut *mesuré*** — une ligne de
`docs/MESURES-V1.md` — **pas une intuition.** Sans mesure, on porte à l'identique et on note
l'amélioration comme décision ouverte.

Le cas qui fonde la règle : l'ADR 0007 a choisi un format interne en entiers alors que la sortie
CoreAudio de V1 travaille en flottants, **avant** que cette sortie soit portée. Le « portage tel
quel » promis par le plan n'est donc plus possible, et ce coût n'a été ni mesuré ni arbitré.

## 5. Le harnais est gelé

Mesuré le 2026-11-25 : **26 000 lignes d'outils, de tests et de documents pour 5 530 lignes
d'application**, dont 40 % de commentaires dans le Rust. Sur 31 sessions, environ 11 au harnais
contre 8 au produit. Le journal des sessions fait 1 390 lignes en cinq jours — précisément le
défaut de V1 que ce projet cite pour justifier sa réécriture, reparu dans sa documentation.

Ce qui est gelé : `tools/`, les invariants, les skills, les agents. **Une vérification nouvelle
n'entre que si le défaut qu'elle attrape peut perdre des données ou du son, ou s'il est arrivé
deux fois.** Quatre chantiers planifiés en tête de `check` passent en « gelés, à rouvrir quand
le son sort » : mode parité, régression visuelle, conteneur Samba avec injection de pannes,
liste unique des zones Rust.

Et quatre règles entrent dans `CLAUDE.md`, lu en entier à chaque session :

- **Ce qu'un test applique ne s'écrit pas pour être lu.** Les 27 fiches d'invariants quittent
  `SKILL.md` (617 lignes, lues avant chaque modification de code) pour `CATALOGUE.md`, qu'on
  n'ouvre que pour ajouter ou modifier une règle. Restent sous les yeux les pièges de plateforme,
  que jamais aucun test ne prouvera.
- **Un ADR n'existe que pour une décision de Thomas**, ou une qu'il pourrait vouloir annuler. Le
  reste est un message de commit. Une page maximum, vérifiée.
- **Les commentaires disent pourquoi, jamais quoi.** Sans repasse rétroactive.
- **Jamais deux sessions qui écrivent ; autant qu'on veut qui lisent.** L'ADR 0013 interdisait
  trop largement : cinq répertoires de travail en parallèle ont coûté deux collisions de numéros
  et un quasi-écrasement, mais une session qui ne fait que lire ne coûte rien.

**Deux formats de session** remplacent le protocole unique, parce qu'une correction de vingt
lignes payait le même que le portage d'un moteur audio : une **passe** (`--fast`, commit direct,
pas d'ADR) et un **chantier** (protocole complet, PR).

## Ce qui n'a pas été retenu de l'audit

- **Réparer V1 en 2-3 jours** pour rendre un canal de livraison à ses testeurs : sans objet, il
  n'y en a pas. C'est le seul endroit où le fait du §1 annule une recommandation entière.
- **« Je n'aurais pas recommandé la réécriture »** : la critique du procédé est fondée — décision
  prise sans estimation de coût, diagnostic et décision du même auteur le même jour, alternative
  écartée en un paragraphe. Elle ne répond pas à l'argument qui tient : V1 n'est pas
  refactorisable **par Thomas**, qui ne peut pas superviser la transformation. Conclusion commune
  des deux avis : ne pas revenir en arrière.
