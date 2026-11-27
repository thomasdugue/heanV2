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
novembre, soit 2,0 min par run en moyenne. ⚠️ Ces 232 min sont du **temps de mur, pas du
facturé** : GitHub arrondit chaque job à la minute supérieure, ce qui fait environ 294 min, et
compte une minute macOS pour dix. Le facturé exact n'est pas mesurable ici — l'API `/timing`
renvoie `billable.MACOS.total_ms: 0` sur tous les runs, probablement à cause du blocage
lui-même. Sur **un seul** run réussi relevé pas à pas, `./check` prend 69 s et `brew install
samba` 14 s : le job est maigre, le cache Rust fait son travail. Le coût vient donc du
**nombre** de runs, pas de leur durée.

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
| Chemins `/Users/<compte>` | 11 fichiers, 23 fois | `$HOME` — et non `~`, qui ne s'étend pas entre guillemets doubles ; le dépôt en a un cas, `docs/PROTOCOLE-SON.md` |
| Mot de passe app-specific Apple | `GUIDE-LAUNCH-BETA-V1.md`, 1 fois | `<MOT_DE_PASSE_APP_SPECIFIQUE>` |

**Vérifié le 2026-11-27** : un balayage de tous les fichiers suivis, motifs tolérants aux
guillemets — c'est un guillemet ouvrant qui avait caché le mot de passe Apple — ne trouve plus
aucune valeur : ni jeton à préfixe connu (`ghp_`, `github_pat_`, `sk-`, `xox…`, `AKIA`,
`BEGIN … PRIVATE KEY`), ni forme `xxxx-xxxx-xxxx-xxxx`, ni 32 hexadécimaux suivis d'un `@`. Les
neuf lignes restantes sont des noms de variables, des fixtures de test et un en-tête HTTP.
`secrets.env.template` n'a que des valeurs vides, et aucun `scripts/.env.local` n'existe.

## Ce que le masquage ne règle pas, et qui reste à la charge de Thomas

⚠️ **Masquer un fichier ne touche pas l'historique.** Publier expose ce qui est sur GitHub :
**60 commits sur `main`**, et 139 au total sur les 26 branches poussées — relevé le 2026-11-26, et
ce dernier compte baisse, les branches fusionnées étant supprimées : 22 le 2026-11-27. (Le compte
`git rev-list --all` est un faux ami : il inclut des branches locales qui ne seront jamais
publiées, et il dérive à chaque commit de n'importe quelle session.)

1. **Le mot de passe app-specific Apple et le DSN Sentry sont dans l'arbre des 60 commits de
   `main`** — c'est-à-dire de *tous* : le fichier qui les porte est là depuis le commit initial.
   Les masquer dans `HEAD` est donc, à soi seul, cosmétique. Un justificatif publié ne se nettoie
   pas, il **se révoque**, des deux côtés :
   - **Le mot de passe app-specific, sur appleid.apple.com.** C'est le seul des trois qui
     *authentifie* : il soumet des notarisations au nom de l'Apple ID. À révoquer en premier.
   - **La clé Sentry, côté Sentry.** ⚠️ Ce n'est **pas** celle du projet courant : relevé le
     2026-11-27, le compte ne porte qu'une organisation (`hean-app`, région EU), un projet
     (`rust`) et une clé, d'organisation `o4511310012088322` — quand le DSN du dépôt porte
     `o4511016013463552`. La clé exposée est celle de V1, dans une organisation que ce compte
     ne voit plus : déjà morte, ou sous un autre identifiant Sentry — à vérifier en se
     connectant. La révoquer ne casse donc rien ici. Et un DSN n'ouvre aucune donnée : publié,
     il permet seulement de noyer le quota d'événements.
2. **37 commits de `main` portent l'adresse e-mail personnelle en champ auteur** (116 sur
   l'ensemble des branches poussées). Masquer les 4 fichiers n'y change rien : seule une
   réécriture d'historique l'enlèverait, et elle casserait tous les clones existants. Pour la
   suite : `git config user.email` vers l'adresse `noreply`.
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
  — mesure : les 142 fichiers suivis de `bootstrap/`, filtrés sur `.rs` (36), `.js` (12),
  `.html` (7), `.css` (3) et `.py` (1) ; le reste est de la prose. Ce chiffre porte un argument
  juridique, il doit dire ce qu'il compte
  de V1.
- **Les documents de stratégie commerciale deviennent publics** : `pricing-gtm.md` (209 lignes),
  `market-viability.md` (150) et `gtm-playbook-solo-dev.md` (112), dans
  `bootstrap/01-produit/recherche/`.
- **La bascule est réversible** : repasser en privé se fait en un clic. Ce qui ne l'est pas, c'est
  ce qui aura été lu, cloné ou indexé entre-temps.
- En phase 6, si la release signée passe dans la CI : sur un dépôt public, les PR venant de forks
  n'ont pas accès aux secrets. C'est une protection, pas une gêne — elle ne devient dangereuse que
  contournée par `pull_request_target`.
