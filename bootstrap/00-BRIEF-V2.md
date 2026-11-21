# Brief heanV2

## L'objectif

Refaire **l'intégralité** de Hean, avec **le même design** et **le même feature set** que la beta
v0.2.0-beta.1, sur une base technique saine — parce que V1 a accumulé des choix qui rendent chaque
changement de plus en plus coûteux.

Trois contraintes, dans cet ordre de priorité :

1. **Parité fonctionnelle.** Rien ne disparaît. La référence est
   [`01-produit/FEATURE-SET.md`](01-produit/FEATURE-SET.md) : 124 features cochables.
2. **Parité visuelle.** Même design — mais **lequel** n'était pas évident : il existe trois
   designs de Hean, et deux sont hors de `main`. La cible est le **redesign v3**, commit `e12a200`
   ([`../docs/adr/0004`](../docs/adr/0004-cible-de-parite.md)), **pas** la version distribuée aux
   testeurs. Les références sont [`02-design/tokens.css`](02-design/tokens.css) pour les **valeurs**,
   [`02-design/reference/views.js`](02-design/reference/views.js) pour le **markup** des écrans
   refaits, et
   [`02-design/reference/noir-design-system.html`](02-design/reference/noir-design-system.html) pour
   les **composants** (108 Ko, 28 sections, aperçus live — il était hors du dépôt, et il précède le
   redesign de deux mois). **Pas** `DESIGN-SYSTEM.md`, qui diverge du code sur 5 couleurs, une ombre
   et la typographie. Tout est dans [`02-design/ECARTS-DESIGN.md`](02-design/ECARTS-DESIGN.md) §0.
   ⚠️ **Ce design n'a jamais été vu** : personne ne l'a lancé. Les captures d'écran sont donc à la
   fois la référence de V2 et la validation de la cible.
3. **Base saine.** Les propositions sont dans
   [`05-postmortem/DETTE-TECHNIQUE-V1.md`](05-postmortem/DETTE-TECHNIQUE-V1.md), section 4, classées
   par rendement. Elles demandent des arbitrages — ce sont les tiens.

## Ce qui est déjà tranché et ne se rouvre pas

- **Tauri v2 + Rust.** Démontré en production, DMG signé et notarisé distribué.
- **CoreAudio direct** (`coreaudio-rs` / `coreaudio-sys`), **jamais CPAL** — CPAL ne donnait pas accès
  au buffer interne (seek cassé, pas d'`AudioUnitReset`, pas de contrôle de la taille de buffer).
- **Bit-perfect = pas de resampling**, hog mode pour l'accès exclusif au DAC.
- **macOS uniquement**, macOS 11+ minimum, **hors Mac App Store** (aucun entitlement sandbox
  n'existe pour l'accès exclusif à un device de sortie — mur technique dur).
- **Distribution directe**, licence perpétuelle.

## Ce qui reste à trancher (phase 0)

Six décisions, listées avec leurs options et mon avis dans
[`06-setup-v2/PLAN-DE-RECONSTRUCTION.md`](06-setup-v2/PLAN-DE-RECONSTRUCTION.md) : langage frontend,
framework UI, IPC typée, store de bibliothèque, nom du dossier de données, typographie.

## Le vrai risque

Ce n'est pas de rater l'architecture. C'est de **perdre des features sans s'en apercevoir**.

V1 contient énormément de détails invisibles jusqu'à ce qu'ils manquent : le hint de permission
« Réseau local » de macOS sur l'étape NAS de l'onboarding, le fallback hostname `.local` quand le NAS
répond en IPv6, la scrollbar des carousels invisible au repos, le `grid-column: 1 / -1` sur les loose
tracks d'une page artiste, les labels du menu contextuel qui s'adaptent à la multi-sélection, le
`RunEvent::Reopen` pour le clic sur l'icône du Dock, les 4 niveaux de fallback du pipeline de covers.

C'est pour ça que ce dossier existe.

## Ce que V1 lègue de plus précieux

Pas le code : le **savoir sur une plateforme hostile**, dans
[`04-invariants/`](04-invariants/). Sept pièges qui ne se déduisent d'aucune documentation Apple, et
qui coûteraient les mêmes semaines à redécouvrir :

- AirPlay n'existe dans CoreAudio que tant qu'il est le défaut système ;
  `set_system_default_device` échoue en silence sur un device stale.
- `libsmbclient` est un singleton au niveau du process (EINVAL si deux clients).
- Les media keys macOS ne s'attrapent qu'avec `MPRemoteCommandCenter` (Apple Music gagne sur les
  global shortcuts).
- Le drag & drop HTML5 ne fonctionne pas dans la WebView.
- Bug WebKit sur les petits CSS Grid (corrigé 4 fois avant d'abandonner Grid).
- `dylibbundler` crée des `LC_RPATH` en doublon que dyld refuse depuis Big Sur.
- Tauri notarise l'app mais pas le DMG.

Plus **une pratique** : écrire la règle, sa gravité et *pourquoi elle existe* le jour où la
régression est corrigée. C'est la meilleure chose du dépôt V1, et elle se transporte telle quelle.

## Note de méthode

Une préférence déjà enregistrée, à respecter en V2 : **tout protocole de test commence par le bloc de
lancement** — `cd <chemin absolu> && <commande>` — parce qu'il est exécuté à la main et qu'on ne veut
pas chercher la commande à chaque fois.

Et la règle de travail de V1, qui a fait ses preuves : **ne jamais modifier l'audio quand on travaille
sur l'UI, et inversement.**
