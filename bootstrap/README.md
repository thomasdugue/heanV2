# heanv2-bootstrap

Tout ce qu'il faut pour recommencer Hean de zéro sans rien perdre.

Dossier autonome : il peut être copié tel quel dans un nouveau dépôt `heanV2` et suffit à démarrer
sans jamais rouvrir le dépôt V1. Constitué le **2026-11-21** à partir du dépôt `noirdesktop`,
branche `claude/refonte-fab-5-2-3901f8`, commit `02508ea` (Hean v0.2.0-beta.1).

> ⚠️ **Révisé le soir du 2026-11-21.** Trois chantiers vivaient hors de `main`, et deux n'étaient
> commités nulle part. La **cible de parité visuelle a changé** : c'est le redesign v3, commit
> `e12a200` ([`../docs/adr/0004`](../docs/adr/0004-cible-de-parite.md)), et non la version
> distribuée. Et un chantier **WiFi Sync** complet a été récupéré :
> [`05-postmortem/chantier-wifi-sync/`](05-postmortem/chantier-wifi-sync/README.md).

---

## Lire dans cet ordre

| | Fichier | Pourquoi |
|---|---|---|
| 1 | [`00-BRIEF-V2.md`](00-BRIEF-V2.md) | L'objectif, ce qui est tranché, ce qui ne l'est pas, le vrai risque. 2 minutes. |
| 2 | [`05-postmortem/DETTE-TECHNIQUE-V1.md`](05-postmortem/DETTE-TECHNIQUE-V1.md) | **Pourquoi on refait.** Les 5 causes racines, chiffrées, et 12 propositions classées par rendement. C'est le document qui demande tes arbitrages. |
| 3 | [`04-invariants/README.md`](04-invariants/README.md) | **Le savoir qui ne se redécouvre pas.** Les 7 pièges de plateforme. À lire avant d'écrire une ligne. |
| 4 | [`01-produit/FEATURE-SET.md`](01-produit/FEATURE-SET.md) | **Le contrat de parité.** 124 features à cocher. |
| 5 | [`02-design/ECARTS-DESIGN.md`](02-design/ECARTS-DESIGN.md) | ⭐ Avant de reproduire le design : il existe **trois designs** de Hean et **six documents** qui les décrivent. Dit lequel est la cible, et pourquoi la PR #37 est cassée. |
| 6 | [`06-setup-v2/PLAN-DE-RECONSTRUCTION.md`](06-setup-v2/PLAN-DE-RECONSTRUCTION.md) | Les 6 arbitrages de la phase 0, puis 8 phases avec leurs critères de sortie. |
| 7 | [`06-setup-v2/HARNAIS.md`](06-setup-v2/HARNAIS.md) | **Le harnais.** Les 12 ajouts qui font qu'un projet ne régresse pas, par rendement, et ce que chaque faiblesse de V1 devient. |

Le reste se consulte au besoin.

---

## Contenu

```
00-BRIEF-V2.md              objectif, contraintes, ce qui est tranché
MANIFEST.md                 d'où vient chaque fichier, et ce qui a été volontairement exclu

01-produit/
  FEATURE-SET.md            ⭐ 124 features cochables, reconstruites depuis le CODE (pas la spec)
                            + §11 WiFi Sync, hors parité mais à reprendre
  UX-ANTI-REGRESSION.md     24 comportements validés, avec ce qui les avait cassés
  NON-GOALS.md              décisions produit déjà prises (plateformes, formats, features, nom)
  GUIDE-TESTEURS-V1.md      ce qu'on a promis aux ~50 beta-testeurs
  RELEASE-NOTES-V1.md       notes de la v0.2.0-beta.1
  recherche/                marché, pricing, GTM, + l'ancien CLAUDE.md de février

02-design/
  tokens.css                ⭐ SOURCE DE VÉRITÉ des valeurs : 64 variables, extraites des DEUX :root de la cible
  ECARTS-DESIGN.md          ⭐ les 3 designs, les 6 sources, laquelle croire, et pourquoi la PR #37 est cassée
  DESIGN-SYSTEM.md          l'intention 2026 (glass, ambient glow, watermarks) — fiable sur le concept
  reference/
    README.md               quoi ouvrir, dans quel ordre
    styles.css, views.js    🎯 LA CIBLE DE PARITÉ (commit e12a200) : valeurs + markup des écrans refaits
    noir-design-system.html ⭐ design system complet 108 Ko / 28 sections — était HORS du dépôt
    index.html              la coquille (ids des panels/modals, le <link> Google Fonts problématique)
    *.beta-v0.2.0           le CSS/markup DISTRIBUÉ, pour mesurer l'écart avec la cible
    GeistMono-Variable.woff2, app_logo.svg
    prototype-onboarding.html, mockup-home.html, splash-preview.html, dmg-window.html

03-architecture/
  ARCHITECTURE-V1.md        état réel mesuré : modules, LOC, état global, pipeline audio, tests
  API-IPC-V1.md             86 commandes Tauri + 17 events, inventaire exhaustif
  PERSISTANCE.md            les 16 fichiers de données + les 3 questions à trancher
  SPEC-V1-COMPLETE.md       la spec de mars (1 324 l.) — détaillée mais périmée, voir MANIFEST
  SPEC-V1-ancienne-fevrier.md   version antérieure, pour archive
  code-reference/
    README.md               ⭐ porter / adapter / réécrire, fichier par fichier
    rust/                   audio/ + network/ + engine, decoder, resampler, eq, media_controls, sentry, logging
    js/                     state.js, app.js, utils.js, drag.js, error-tracking.js
    Cargo.toml.v1, tauri.conf.json.v1

04-invariants/              ⭐ LE DOSSIER LE PLUS IMPORTANT
  README.md                 comment lire, les 7 pièges de plateforme, comment faire vivre les règles
  INVARIANTS.md             les 19 règles « ne jamais casser », avec gravité et raison
  SAVOIR-AUDIO.md           CoreAudio, bit-perfect, hog mode, stratégie AirPlay complète
  SAVOIR-SMB.md             singleton libsmbclient, progressive download, gapless SMB, Keychain
  SAVOIR-FRONTEND.md        pipeline covers, isConnected, min-width:0, mosaïques
  SAVOIR-OPS.md             worker Cloudflare, Sentry + anonymisation + RGPD, logs, dashboard
  SAVOIR-RELEASE.md         pipeline signature/notarisation/dylibbundler/updater (⚠️ 2 versions, la 1re est la bonne)

05-postmortem/
  DETTE-TECHNIQUE-V1.md     ⭐ pourquoi on refait + 12 propositions
  HISTORIQUE-SESSIONS.md    le journal des sessions V1 (mars → octobre 2026)
  BUGS-CONNUS-V1.md         bugs et limitations connus + addendum
  CHANGELOG-V1.md           historique cumulatif, dont les 4 phases de perf
  archive-dap-sync/         ⭐ la feature retirée, CODE COMPLET CONSERVÉ (~12 400 l.) + docs + diff de retrait
    README-CODE.md          inventaire, pourquoi ça cassait, les 5 pistes, comment ressusciter
  chantier-wifi-sync/       ⭐ le SUCCESSEUR du MTP, et il marchait : 5 677 l. de Rust + 30 tests
    README.md               ce qui est déjà résolu, l'état réel, ce qui reste
    DESIGN-PLAN-V3.md       le cahier des charges de l'étape suivante, jamais implémentée
    TEST-PROTOCOL-V2.2.md   le protocole AVEC les réponses de test écrites par Thomas

06-setup-v2/
  README.md                 ce qui s'installe tel quel, et où
  PLAN-DE-RECONSTRUCTION.md ⭐ phase 0 (6 arbitrages) + 8 phases
  HARNAIS.md                ⭐ les 12 ajouts au harnais, ce qu'il ne faut PAS ajouter, table de correspondance V1
  CLAUDE.md                 template prêt à poser à la racine de heanV2
  agents/                   test-runner, invariant-guardian
  hooks/                    invariant-reminder.sh
  ci/                       ci.yml, cloudflare-worker.js, generate-dmg.py, release.sh.v1, dev.sh.v1, secrets.env.template
  SPEC-SUITE-DE-TESTS-V1.md protocole de tests manuels, à rejouer en recette
  GUIDE-LAUNCH-BETA-V1.md   guide narratif du lancement beta
```

---

## Démarrer une session heanV2

```bash
mkdir -p ~/Documents/Thomas/heanV2 && cd ~/Documents/Thomas/heanV2 && git init
cp -R /Users/tsunami25/Documents/Thomas/noirdesktop/heanv2-bootstrap .
```

Puis, dans la session heanV2 :

> Lis `heanv2-bootstrap/00-BRIEF-V2.md`, `heanv2-bootstrap/05-postmortem/DETTE-TECHNIQUE-V1.md` et
> `heanv2-bootstrap/06-setup-v2/PLAN-DE-RECONSTRUCTION.md`. On est en phase 0 : pose-moi les
> 6 arbitrages un par un, avec ta recommandation, et n'écris aucun code avant qu'ils soient tranchés.

---

## Deux choses à faire avant de commencer

1. **Les captures d'écran de V1.** Aucune n'existe dans le dépôt. Le CSS seul ne dit pas à quoi
   ressemble le résultat composé — et sans captures, « le design à l'identique » n'est pas
   vérifiable. Il faut lancer V1 et capturer chaque vue, chaque panel, chaque modal, vers
   `02-design/reference/captures/`.
   ```bash
   cd /Users/tsunami25/Documents/Thomas/noirdesktop/noir-tauri && ./scripts/dev.sh
   ```
2. **Vérifier la typographie.** DM Sans est chargée depuis Google Fonts alors que la CSP ne
   l'autorise pas : l'app tourne probablement en Geist Mono + San Francisco, pas en DM Sans. À
   confirmer visuellement avant de figer le choix typographique de V2. Détail dans
   [`02-design/ECARTS-DESIGN.md`](02-design/ECARTS-DESIGN.md), section 2.
