# Plan de reconstruction heanV2

> **Proposition, pas une décision.** Les arbitrages de la phase 0 t'appartiennent ; le reste découle
> de ce que tu auras tranché. L'ordre des phases, lui, est important : il est fait pour que la
> parité fonctionnelle soit **vérifiable** à chaque étape plutôt que constatée à la fin.
>
> Principe directeur : **le risque de V2 n'est pas technique, il est de perdre des features sans le
> voir.** D'où l'ordre : d'abord le filet (tests + invariants), puis le plus risqué (audio), puis le
> volume (UI).

---

## Phase 0 — Arbitrages et socle (avant toute ligne de code)

### Les 6 décisions à prendre

| # | Décision | Options | Mon avis |
|---|---|---|---|
| 1 | **Langage frontend** | TypeScript strict / JS vanilla comme V1 | **TypeScript.** C'est la cause racine n°1 du postmortem. |
| 2 | **Framework UI** | Svelte / Solid / Lit / React / vanilla + bundler | À arbitrer. 5 régressions V1 viennent de la manipulation DOM manuelle (cellules déconnectées, `isConnected`, rebuild rAF). Un framework les élimine ; il ajoute une dépendance et un build. Si tu veux rester minimal : vanilla + Vite + TypeScript, mais avec un vrai modèle de composant. |
| 3 | **IPC typée** | `tauri-specta` (types générés depuis Rust) / strings comme V1 | **tauri-specta.** Élimine la famille de bugs la plus coûteuse de V1. |
| 4 | **Store de bibliothèque** | SQLite (`rusqlite`) / un seul JSON / 6 JSON comme V1 | **SQLite.** Fait disparaître 4 invariants bloquants. |
| 5 | **Dossier de données** | garder `~/Library/Application Support/noir/` / passer à `hean/` + migration | **`hean/` + migration one-shot.** C'est le bon moment. Voir [`../03-architecture/PERSISTANCE.md`](../03-architecture/PERSISTANCE.md). |
| 6 | **Typographie** | Geist Mono partout (= l'app actuelle) / DM Sans + Geist Mono technique, embarquées | À arbitrer après avoir regardé l'app. Voir [`../02-design/ECARTS-DESIGN.md`](../02-design/ECARTS-DESIGN.md). |

Trois décisions sont déjà prises et ne se rouvrent pas : **Tauri v2 + Rust**, **CoreAudio direct
(jamais CPAL)**, **bit-perfect = pas de resampling**.

### Le socle à poser

- [ ] Dépôt `heanV2` (ou dossier `heanv2/` dans le dépôt actuel — à toi de voir).
- [ ] Copier ce dossier bootstrap dedans, et y poser `CLAUDE.md` (template dans
      [`CLAUDE.md`](CLAUDE.md)) + `.claude/skills/` (depuis [`../04-invariants/`](../04-invariants/),
      ce sont déjà des `SKILL.md` avec frontmatter) + `.claude/agents/` (depuis
      [`agents/`](agents/)) + `.claude/hooks/` (depuis [`hooks/`](hooks/)).
- [ ] CI dès le premier commit : [`ci/ci.yml`](ci/ci.yml) adapté — `tsc --noEmit`, lint, tests
      front, `cargo test --locked`, et **branche `main` protégée**.
- [ ] `build.rs` avec `compile_error!()` sur les secrets manquants en profil release (remplace la
      moitié de [`ci/release.sh.v1`](ci/release.sh.v1)).
- [ ] Limite dure de taille de fichier vérifiée en CI (~400 lignes).
- [ ] **Captures d'écran de V1** : chaque vue, chaque panel, chaque modal → dans
      `02-design/reference/captures/`. Sans ça, « le design à l'identique » n'est pas vérifiable.

### Le filet anti-régression, écrit AVANT les features

- [ ] Les **24 comportements** de [`../01-produit/UX-ANTI-REGRESSION.md`](../01-produit/UX-ANTI-REGRESSION.md)
      traduits en tests (certains unitaires, certains e2e). Ils échouent tous : c'est normal.
- [ ] Les **19 invariants** de [`../04-invariants/INVARIANTS.md`](../04-invariants/INVARIANTS.md)
      triés en trois piles : (a) devenus impossibles par construction — on les raye et on note
      pourquoi, (b) traduisibles en test — on écrit le test, (c) irréductibles (pièges plateforme) —
      on les garde comme règles écrites avec leur gravité.
- [ ] **Critère de sortie de phase 0** : la CI tourne, elle est rouge sur les tests de comportement,
      et le fichier d'invariants de V2 existe avec ses trois piles.

---

## Phase 1 — Moteur audio (le plus risqué d'abord)

Portage, pas réécriture. Voir
[`../03-architecture/code-reference/README.md`](../03-architecture/code-reference/README.md).

- [ ] `audio/coreaudio_backend.rs`, `coreaudio_stream.rs`, `types.rs`, `backend.rs` portés tels quels.
- [ ] `audio_engine.rs` porté **avec message passing** au lieu du `Mutex<Option<AudioEngine>>` global
      (la « Phase 5 » jamais faite de V1).
- [ ] `audio_decoder.rs` : garder `SmbProgressiveFile` intact, simplifier le reste.
- [ ] `resampler.rs`, `eq.rs`, `media_controls.rs` portés.
- [ ] Lecture d'un fichier local → bit-perfect vérifié (rate source = rate device, EQ off, volume 100 %).
- [ ] **Critère de sortie** : on lit un FLAC 24/192 en hog mode, on seek, on change de device, on
      passe en AirPlay et on en revient — et les tests `cargo test` de l'audio passent.

Ne pas avancer tant que cette phase n'est pas verte : tout le reste en dépend, et c'est la partie où
une erreur coûte le plus cher.

---

## Phase 2 — Bibliothèque et persistance

- [ ] Schéma du store (SQLite ou autre selon l'arbitrage 4), **un seul écrivain**.
- [ ] Scan : local + parallélisé (rayon), différentiel, events de progression.
- [ ] Un **unique point d'entrée** pour charger des pistes, avec le filtre `excluded_paths` dedans
      (remplace l'invariant 6).
- [ ] Extraction métadonnées (lofty) + **écriture** dans le fichier audio, sous transaction.
- [ ] Groupement albums/artistes avec une clé d'album **typée** (un constructeur unique qui fait
      trim + NFC — remplace l'invariant 1).
- [ ] Pipeline covers : thumbnails au scan, protocole custom (ex-`noir://`), 4 niveaux de fallback.
- [ ] Migration des données V1 si l'arbitrage 5 le demande.
- [ ] **Critère de sortie** : les tests de comportement B\* et C\* de
      [`../01-produit/FEATURE-SET.md`](../01-produit/FEATURE-SET.md) passent. Les invariants 7, 8, 9
      et V015 sont rayés comme « impossibles par construction », avec la justification écrite.

---

## Phase 3 — NAS / SMB

- [ ] Porter `network/` tel quel (singleton `CONNECTION`, retry, Keychain + cache session, mDNS).
- [ ] Progressive download (chunks 64 Ko, seuil 4 Mo, timeout 15 s) + `SmbProgressiveFile`.
- [ ] Gapless SMB (preload à 60 s restantes, `cancel_previous = false`).
- [ ] Aucune I/O SMB bloquante sur un thread de commande (régression W005).
- [ ] **Critère de sortie** : lecture gapless d'un album entier depuis le NAS, débranchement du NAS
      en cours de scan sans gel de l'UI, zéro dialogue Keychain au démarrage.

---

## Phase 4 — Design system et coquille UI

- [ ] `tokens.css` posé tel quel depuis [`../02-design/tokens.css`](../02-design/tokens.css),
      en un seul fichier généré/isolé.
- [ ] Layout : fenêtre transparente + titlebar overlay, sidebar redimensionnable (280 px, 180-400,
      collapse < 900 px), player bar, zones de panels.
- [ ] Glassmorphism, ambient glow, watermarks dynamiques (pipeline d'extraction de couleur).
- [ ] CSS découpé par composant.
- [ ] **Critère de sortie** : comparaison côte à côte avec les captures de V1 sur la coquille vide.

---

## Phase 5 — Vues

Dans cet ordre (du plus structurant au plus accessoire) : Pistes (virtual scroll) → Albums →
Artistes → détail album → détail artiste → Home (carousels, mixes) → Playlist.

- [ ] Virtual scroll : pool fixe, item 48 px, event delegation, multi-sélection.
- [ ] Recherche : index inversé, debounce 200 ms, < 16 ms sur 10 k pistes.
- [ ] Menus contextuels (labels adaptés à la multi-sélection), queue, track info, bulk edit.
- [ ] Playlists + favoris + **mosaïques** (le composant à refaire proprement, pas à protéger par 4
      invariants).
- [ ] Drag & drop custom (HTML5 inopérant dans la WebView).
- [ ] Transitions de vue avec annulation des renders obsolètes.
- [ ] **Critère de sortie** : tous les tests F\* et E\* passent, captures comparées vue par vue.

---

## Phase 6 — Finitions produit

Fullscreen player (particules, RMS, couleurs) · lyrics (lrclib + lyrics.ovh) · EQ (courbe SVG,
presets) · onboarding 6 étapes · raccourcis + remapping · settings complet · auto-update.

---

## Phase 7 — Observabilité et distribution

- [ ] Sentry Rust + forwarder JS + anonymisation (porter `sentry_init.rs` et ses 5 tests) + toggle RGPD.
- [ ] Logs tracing avec rotation.
- [ ] Feedback → worker Cloudflare ([`ci/cloudflare-worker.js`](ci/cloudflare-worker.js) réutilisable
      tel quel) + fallback local.
- [ ] Pipeline release : signature, dylibbundler + fix LC_RPATH, notarisation app **et** DMG,
      tarball updater, `latest.json`. Runbook : [`../04-invariants/SAVOIR-RELEASE.md`](../04-invariants/SAVOIR-RELEASE.md).
- [ ] **Critère de sortie** : un DMG signé+notarisé qui se lance sur un Mac sans Homebrew.

---

## Phase 8 — Recette de parité

- [ ] Parcourir [`../01-produit/FEATURE-SET.md`](../01-produit/FEATURE-SET.md) ligne par ligne, en
      cochant, **avec V1 et V2 ouverts côte à côte**.
- [ ] Rejouer la suite de tests manuels V1 ([`SPEC-SUITE-DE-TESTS-V1.md`](SPEC-SUITE-DE-TESTS-V1.md)).
- [ ] Comparer les captures d'écran vue par vue.
- [ ] Écrire la liste des écarts **assumés** (ce qu'on a volontairement changé) — c'est ce qui ira
      dans les notes de version.

---

## Combien de temps ?

Je ne donne pas d'estimation en jours : V1 représente ~30 000 lignes construites sur plusieurs mois,
et une réécriture assistée n'obéit pas aux mêmes ratios. Ce que je peux dire :

- les phases 1 et 3 sont du **portage** (rapide, mais à faire avec soin) ;
- la phase 5 est le **gros volume** (les vues, c'est ~9 000 lignes de JS en V1) ;
- la phase 0 paraîtra lente et c'est elle qui décide du résultat.

**Le piège à éviter** : commencer par l'UI parce que c'est gratifiant. L'audio d'abord — c'est là
qu'une erreur de conception est irrattrapable.
