# 0011 — Les exemples en ligne de commande écrivent sur stdout : dérogation à INV-21

- **Date** : 2026-11-23
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-23 — il a décidé que cette dérogation soit tracée par un
  ADR plutôt que par un commentaire dans un test. La **forme** de la dérogation (même modèle que
  `build.rs`, et la liste des exceptions assertée) avait été arrêtée par la session de phase 0,
  à l'intérieur d'un périmètre qu'il avait approuvé ; ce qu'il tranche ici est le point que le
  texte d'INV-21 exigeait de lui, et lui seul : **une exception à une règle 🔴 se classe comme la
  règle**. Ce que le champ « Décideur » doit dire en général quand une session décide à
  l'intérieur d'un brief approuvé reste une question ouverte devant lui
  ([ADR 0009](0009-bibliotheque-cle-dalbum-et-source-de-verite.md)) ; la formulation d'ici suivra
  sa réponse.
- **Suit** : [0005](0005-entretien-du-harnais-et-orchestration.md) (le harnais s'entretient par
  mécanisme, jamais par discipline)

## Contexte

**INV-21 — « aucun `println!` / `eprintln!` dans le backend » — est classée 🔴 BLOQUANT par Thomas
le 2026-11-22**, et son texte se termine par une phrase sans ambiguïté : « Toute exception passera
par un ADR. »

Ce que la règle protège n'est pas un défaut d'affichage. V1 comptait **252 `println!` contre 3
appels `tracing::`**, et son `logging.rs` ne capturait pas stdout : l'app installée a produit **63
lignes de journal dans toute sa vie**. Les trois bugs trouvés le 2026-11-21 — dont la mise à jour
automatique qui renvoyait 404 depuis le lancement, et qui n'a donc **jamais** fonctionné pour aucun
testeur — étaient tous annoncés en clair par le build de dev et **invisibles en production**. Le
raisonnement de Thomas : un défaut qui se voit se répare, un défaut invisible se propage.

Le détecteur de cette règle scannait `src-tauri/src` et `src-tauri/tests`. **Tout ce qui est
ailleurs sous `src-tauri/` échappait donc à une règle 🔴 — et par omission, non par décision.**

Deux fichiers sont concernés, et leur statut était asymétrique :

- `src-tauri/build.rs` était hors scan **et expliqué** : ses `println!("cargo::…")` sont le
  protocole obligatoire de Cargo, lu par `cargo` lui-même, et aucun `tracing` ne peut les
  remplacer. C'était la bonne forme.
- `src-tauri/examples/migrer.rs` était hors scan et **expliqué nulle part**. C'est l'utilitaire de
  vérification de la migration `noir/` → `hean/`, lancé à la main par
  `tools/verifier-migration-reelle.sh`. Il porte des `println!` **volontairement**.

Un angle mort non écrit est celui qu'on redécouvre. Et il était d'autant plus coûteux ici qu'il
touchait la règle dont la gravité venait d'être arbitrée.

## Décision

**Les exemples Cargo (`src-tauri/examples/`) sont hors du scan d'INV-21, et la liste des exceptions
est elle-même assertée.**

La dérogation tient sur une raison et une seule : **un exemple n'est pas dans le binaire livré.**
Cargo ne compile un `examples/*.rs` que sur demande explicite (`cargo run --example`) ; il n'entre
ni dans la bibliothèque ni dans le binaire de l'app. Un `println!` qui y vit **ne peut donc pas
rendre l'app aveugle chez un testeur**, qui est exactement ce qu'INV-21 protège. S'y ajoute que
l'outil se lance dans un terminal, sous les yeux de Thomas, pendant qu'il vérifie une copie réelle
d'un dossier `noir/` : **un outil en ligne de commande sans sortie n'a aucun intérêt.**

La dérogation ne repose donc pas sur la bonne conduite de qui écrira dans `examples/`, mais sur une
**propriété de Cargo**. C'est ce qui la rend tenable, et c'est la différence avec une exception de
confort.

Trois propriétés de forme, qui sont la moitié de la décision :

1. **Les exceptions sont écrites dans le test**, dans la constante `HORS_SCAN` de
   [`tests/architecture/backend.test.ts`](../../tests/architecture/backend.test.ts), chacune avec
   sa justification — dont la présence est vérifiée, pas seulement attendue.
2. **La liste est assertée.** Tout `.rs` posé hors des dossiers scannés fait rougir `./check` tant
   que sa dérogation n'est pas écrite. C'est la différence entre une dérogation et un oubli : en
   ajouter une demande de **toucher au test**, au lieu de se produire par omission.
3. **Un motif qui ne finit pas par `/` désigne un fichier à l'exact.** `build.rs` ne couvre donc
   pas un hypothétique `build.rs2.rs`. Élargir une dérogation est le début de la fin d'une règle,
   même par accident.

## Alternatives écartées

**Scanner `examples/` et y imposer `tracing`.** C'est l'alternative sérieuse, et elle échoue sur un
point concret : `tracing` écrit dans le **fichier de journal de l'app**, pas dans le terminal. Un
utilitaire de vérification dont la sortie part dans un fichier que Thomas devrait ensuite aller
chercher est inutilisable pour ce qu'on lui demande — regarder défiler, en direct, ce que la
migration s'apprête à faire sur une copie réelle de sa bibliothèque. On paierait le coût de la
règle sans acheter ce qu'elle protège, puisque rien de tout cela n'est livré.

**Laisser l'omission telle quelle.** C'était le statu quo, et il était invisible. Une règle 🔴 dont
le périmètre réel est plus petit que son énoncé est désarmée là où personne ne regarde : c'est le
modèle de défaut que V1 a payé le plus cher, et celui que le détecteur d'INV-06 vient de reproduire
en phase 1 (il cherchait `\bTrack\b` là où specta émet `Track_Serialize[]` : zéro commande attrapée
sur huit, auto-test vert pendant tout ce temps).

**Réécrire l'utilitaire en shell dans `tools/`,** où INV-21 ne s'applique pas. Il a besoin des types
Rust du projet (`library/`, `migration/`) pour dire quelque chose de vrai sur l'inventaire et la
vérification. Les réimplémenter en shell serait une **deuxième implémentation de la logique de
migration**, donc une deuxième vérité — la famille de défaut d'INV-01, et le prix en serait
autrement plus lourd que celui d'une exception écrite.

**Une exception large, du type « tout `src-tauri/` hors `src/` ».** Elle couvrirait d'avance des
dossiers dont personne n'a examiné le cas, par exemple un futur `benches/` dont le code, lui, peut
très bien finir appelé depuis la bibliothèque. Une dérogation doit nommer ce qu'elle couvre.

## Conséquences acceptées

- **Poser un `.rs` hors de `src-tauri/src` et `src-tauri/tests` fait rougir `./check`** jusqu'à ce
  que sa dérogation soit écrite. C'est une friction, elle est voulue, et elle est de la taille de
  ce qu'elle protège : trois lignes à écrire pour amender une règle 🔴.
- **La note « muet » de l'entrée `examples/` fera rougir le test une fois**, le jour où la branche
  de migration fusionnera et que le dossier existera pour de bon. **Ce n'est pas une régression du
  merge** : le message dit quoi faire (retirer la note). Une dérogation se confirme quand elle entre
  en vigueur, elle ne s'hérite pas en silence.
- **Rien n'empêche techniquement d'écrire de la vraie logique d'app dans `examples/`** pour
  contourner la règle. Ce serait détourner un dossier de son usage, et la propriété de Cargo citée
  plus haut cesserait de protéger quoi que ce soit. Aucun test ne l'attrape aujourd'hui, et c'est
  assumé : écrit ici, c'est une chose qu'une relecture peut voir.
- **Le texte d'INV-21 renvoie désormais à cet ADR.** La règle, sa gravité et son pourquoi ne
  changent pas — seule son exception est tracée là où elle ne se modifie pas.

## Ce que cet ADR ne tranche pas

- **Le numéro 0010**, pris par une PR encore ouverte au moment d'écrire ces lignes (la migration
  des données de V1). `tools/check-adr.mjs` ne voit que les fichiers suivis par git **sur la
  branche courante** : il ne peut pas détecter une collision avec une branche vivante, et c'est
  pour cette raison que le numéro suivant se prend en regardant [l'index](README.md) *et* les PR
  ouvertes. D'où 0011.
- **La convention générale du champ « Décideur »** quand une session décide à l'intérieur d'un
  brief approuvé. Question ouverte devant Thomas depuis le 2026-11-22
  ([ADR 0009](0009-bibliotheque-cle-dalbum-et-source-de-verite.md)).
- **Si `tools/check-invariants.mjs` doit exiger mécaniquement un numéro d'ADR** pour chaque
  exception d'une règle dont le texte annonce « passera par un ADR ». Proposé par
  `invariant-guardian` le 2026-11-23, et c'est la forme que ce projet préfère — mais c'est une
  vérification nouvelle, donc sa propre tâche. En attendant, la règle tient par ce fichier.
