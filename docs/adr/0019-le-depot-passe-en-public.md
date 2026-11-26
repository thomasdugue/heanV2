# 0019 — Le dépôt passe en public le temps de la construction de V2

- **Date** : 2026-11-26
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-26 — « on va passer en public le temps de la construction
  de la V2 ».

## Contexte

La CI ne rend plus aucun verdict depuis le 2026-11-26 : le job `check` échoue en 3 à 4 secondes
sans exécuter une seule vérification, sur l'annotation GitHub « recent account payments have failed
or your spending limit needs to be increased ». Ce n'est pas un défaut du code — un rouge de PR ne
dit donc plus rien, ce qui est l'état exact où un garde-fou devient un bruit qu'on apprend à
ignorer.

**Mesuré le 2026-11-26** : 114 runs et **232 minutes de runner macOS** entre le 21 et le 26
novembre, soit 2,0 min par run en moyenne (dont 69 s de `./check` : le job est déjà maigre, le
cache Rust fait son travail, `brew install samba` ne coûte que 14 s). Sur un dépôt **privé**, une
minute macOS vaut environ dix minutes Linux. Le coût vient donc du **nombre** de runs, pas de leur
durée.

Les runners standard — `macos-latest` en est un — sont gratuits et sans plafond de minutes sur un
dépôt **public**.

## Décision

**Le dépôt `thomasdugue/heanV2` passe en public pendant la construction de V2**, et les données
sensibles sont masquées avant la bascule.

Ce qui a été masqué dans l'arbre courant, par cette décision :

| Donnée | Où | Remplacée par |
|---|---|---|
| DSN Sentry réel (projet EU) | `bootstrap/06-setup-v2/GUIDE-LAUNCH-BETA-V1.md`, 3 fois | `https://<CLE_PUBLIQUE>@<ORG>.ingest.de.sentry.io/<PROJET>` |
| Apple Team ID | même fichier, 3 fois | `<TEAM_ID>` |
| Adresse e-mail personnelle | 4 fichiers | l'adresse `users.noreply.github.com`, déjà publique |
| Numéro de série du FiiO JM21 | 2 fichiers, 16 fois | `<SERIAL_APPAREIL>` |
| Chemins `\/Users\/<compte>` | 11 fichiers, 23 fois | `$HOME` (correct y compris entre guillemets, contrairement à `~`) |

## Ce que le masquage ne règle pas, et qui reste à la charge de Thomas

⚠️ **Masquer un fichier ne touche pas l'historique.** Publier le dépôt publie les 156 commits.

1. **Le DSN Sentry est dans l'arbre des 156 commits** — c'est-à-dire de *tous* : le fichier qui le
   porte est là depuis le commit initial. Le masquer dans `HEAD` est donc, à lui seul,
   cosmétique. Un secret publié ne se nettoie pas, il **se révoque** : la seule correction est
   de supprimer la clé côté Sentry et d'en créer une neuve. Un DSN ne donne accès à aucune
   donnée — il part de toute façon dans chaque binaire distribué, donc il est déjà extractible
   avec `strings` — mais publié, il permet de noyer le quota d'événements.
2. **133 commits portent l'adresse e-mail personnelle en champ auteur.** Masquer les 4 fichiers
   ne change rien : seule une réécriture d'historique l'enlèverait, et elle casserait tous les
   clones existants. Pour la suite : `git config user.email` vers l'adresse `noreply`.
3. **Le nom « Thomas Dugué » n'est pas masqué**, volontairement. Il est l'auteur de chaque commit,
   le propriétaire du compte GitHub, et il figurera sur l'application signée. Le masquer dans
   trois fichiers serait du théâtre.

## Alternatives écartées

**Rester privé et supprimer le run redondant sur `main`.** Mesuré : 34 des 114 runs sont sur
`main`, et au moins l'un d'eux — la fusion de la PR #23 — a retesté un arbre identique bit pour
bit, le sommet de `main` étant déjà dans la branche. C'est un gros tiers de la dépense pour une
information déjà acquise. Écartée parce qu'elle ne supprime pas la dépense, elle la divise ; et
parce qu'elle a un coût réel : si `main` bouge entre le dernier run d'une PR et sa fusion, le
squash produit un arbre que personne n'a testé. **Reste disponible si la bascule est annulée.**

**Déplacer les vérifications non-Rust sur Ubuntu.** Diviserait la facture par dix, mais casserait
« un seul job, une seule commande » (ADR [0006](0006-socle-et-harnais-de-phase-0.md),
[0008](0008-epingler-la-chaine-doutils.md)) — précisément ce qui empêche la CI de diverger du
local, et qui est la leçon de V1 où la CI avait ses propres étapes.

**Un runner auto-hébergé sur le Mac de Thomas.** Compute gratuit, mais tue la propriété « la CI
part d'un disque nu », notée en tête de [`check`](../../check) : c'est elle qui a attrapé le
`node_modules` périmé du 2026-11-25.

## Conséquences acceptées

- **Aucun fichier `LICENSE`.** Publier n'est pas ouvrir : par défaut, tous droits réservés — le
  code est lisible et copiable en fait, pas réutilisable en droit. Hean reste un produit payant
  (« distribution directe, licence perpétuelle »), et `bootstrap/` embarque 59 fichiers source
  de V1.
- **Les documents de stratégie commerciale deviennent publics** : `pricing-gtm.md` (209 lignes),
  `market-viability.md` (150) et `gtm-playbook-solo-dev.md` (112), dans
  `bootstrap/01-produit/recherche/`.
- **La bascule est réversible** : repasser en privé se fait en un clic. Ce qui ne l'est pas, c'est
  ce qui aura été lu, cloné ou indexé entre-temps.
- En phase 6, si la release signée passe dans la CI : sur un dépôt public, les PR venant de forks
  n'ont pas accès aux secrets. C'est une protection, pas une gêne — elle ne devient dangereuse que
  contournée par `pull_request_target`.
