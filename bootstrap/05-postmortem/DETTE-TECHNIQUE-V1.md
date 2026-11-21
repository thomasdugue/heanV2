# Postmortem V1 — ce qui a mal tourné, et ce que ça implique pour V2

> Analyse écrite le 2026-11-21 à partir du code, de l'historique des sessions, des 19 invariants et
> des 24 comportements anti-régression. C'est **mon jugement**, pas un relevé factuel : à discuter,
> pas à appliquer tel quel. Les faits sur lesquels il s'appuie sont chiffrés et sourcés.
>
> Précision importante : V1 **fonctionne**. Une beta signée et notarisée est partie à ~50 testeurs,
> l'audio est bit-perfect, le gapless marche sur NAS, l'AirPlay est géré. Le problème n'est pas la
> qualité du résultat, c'est le **coût marginal de chaque changement**, qui monte.

---

## 1. Le symptôme : 19 invariants, dont 11 bloquants

Un invariant, dans V1, c'est la trace d'une régression déjà payée. Le fichier
[`04-invariants/INVARIANTS.md`](../04-invariants/INVARIANTS.md) dit explicitement : « Chaque règle ici
a été payée par une régression réelle. »

C'est une excellente pratique — et en même temps le meilleur indicateur du problème. Classons-les :

| Catégorie | Nombre | Ce que ça dit |
|---|---:|---|
| **Pièges de plateforme** (CoreAudio, libsmbclient, WebKit, dyld, notarisation) | 7 | ✅ Savoir légitime et irremplaçable. À transporter en V2 tel quel. |
| **Cohérence de caches** (invariants 7, 8, 9 + V015) | 4 | ⚠️ Un seul problème de conception, décrit quatre fois. |
| **« N'oublie pas de répéter ce filtre partout »** (invariant 6) | 1 | ⚠️ Défense en profondeur = aveu qu'il n'y a pas de point d'entrée unique. |
| **Un seul composant UI** (mosaïques playlist : invariants 10, 11, 12, 13) | 4 | 🔴 Quatre invariants pour une vignette. Le composant est à refaire, pas à protéger. |
| **Duplication de logique** (invariant 19, shuffle) | 1 | 🔴 La logique était copiée dans `shortcuts.js` **avec un autre glyphe**. |
| **Build silencieux** (invariants 16, 17) | 2 | ⚠️ `option_env!()` renvoie `None` sans bruit ; corrigé par un script wrapper au lieu d'un échec de compilation. |

Autrement dit : **8 des 19 invariants (42 %) protègent des faiblesses structurelles**, pas des
spécificités de la plateforme. En V2, ils ne devraient pas exister — ils devraient être impossibles.

Le cas le plus parlant : la mosaïque 28×28. Le bug WebKit (une cellule vide dans un petit CSS Grid
résout `height: 100%` contre le conteneur) a été **corrigé quatre fois** avant qu'on abandonne Grid
pour du positionnement absolu, et le problème d'async des covers a demandé un troisième correctif
(rebuild via `requestAnimationFrame` + garde `_thumbsRefreshScheduled` contre la boucle infinie). Un
composant de 28 pixels de côté a produit 4 invariants, 2 sessions de debug et une garde anti-boucle.

---

## 2. Les causes racines

### 2.1 Zéro vérification statique sur 16 000 lignes de JavaScript

C'est, de loin, la cause la plus coûteuse.

> « Pas de bundler, pas de linter JS : `src/` est servi tel quel. »
> « `node --check` — seule analyse statique JS » — `noir-tauri/CLAUDE.md`

Conséquences visibles dans l'historique :

- **V014** : `artist.albums` est passé de `Set` à `Array` ; les appelants utilisaient encore `.size`.
  Un type l'aurait attrapé à l'écriture.
- **Invariant 19** : `shortcuts.js` dupliquait `cycleShuffleMode()` avec le glyphe ⤢ (U+2922) au lieu
  de ⤮ (U+292E). Deux implémentations divergentes de la même fonction, découvertes par l'œil.
- **Le médiateur `app.js`** : 95 propriétés initialisées à `null`, remplies à l'exécution. Un slot mal
  orthographié ou jamais rempli ne se manifeste que quand l'utilisateur emprunte ce chemin de code.
  C'est un service locator sans contrat.
- **L'IPC n'est typée nulle part** : `invoke('scan_network_source_cmd', { sourceId: result.id })` — le
  nom de la commande, le nom du paramètre et sa forme sont trois strings. Le bug « scan NAS à 0
  piste » du 2026-03-09 vient exactement de là (`add_network_source` renvoie un objet, le code lisait
  une string). Même chose pour les 9 bugs d'intégration de l'onboarding le même jour : tous des noms
  de champs de payload faux (`data.mp3_count` au lieu de `data.stats.mp3_count`,
  `device.name` au lieu de `device.display_name`…).

**Pour V2** : TypeScript strict + un bundler, et surtout une couche IPC **générée** depuis les types
Rust (`tauri-specta` ou équivalent) de façon qu'une commande renommée, un paramètre ajouté ou un
payload d'event modifié casse la compilation du frontend. À mon sens, c'est la décision qui aura le
meilleur rapport effort/bénéfice de tout le projet : elle élimine une famille entière de bugs qui a
occupé plusieurs sessions complètes.

### 2.2 Des fichiers trop gros pour être raisonnés

| Fichier | Lignes |
|---|---:|
| `lib.rs` | 4 870 |
| `views.js` | 3 512 |
| `playback.js` | 1 983 |
| `playlists.js` | 1 589 |
| `panels.js` | 1 567 |
| `styles.css` | **10 654 (un seul fichier)** |

`lib.rs` contient 86 commandes, 15 états globaux, la table de normalisation des genres ID3v1, le
mapping Deezer, le protocole `noir://` et le setup. `views.js` contient toutes les vues, le virtual
scroll, les carousels et les transitions.

Effet de bord documenté : `docs/HISTORY.md` note que l'historique des sessions a dû être **déplacé
hors de `CLAUDE.md` parce que le fichier était tronqué à la lecture**. Quand la documentation du
projet devient trop grosse pour être lue en entier, le code l'est depuis longtemps.

**Pour V2** : une limite dure (par exemple 400 lignes par fichier, 1 responsabilité), et
`styles.css` découpé par composant avec un fichier de tokens isolé.

### 2.3 Sept caches, plusieurs écrivains, aucune transaction

`METADATA_CACHE`, `TRACKS_CACHE`, `COVER_CACHE`, `INTERNET_NOT_FOUND_CACHE`, `ADDED_DATES_CACHE`,
`LISTENING_HISTORY`, `CACHE_DIRTY` — tous des `Lazy<Mutex<...>>` globaux, chacun persisté dans son
propre JSON, modifiés concurremment par le scan de fond, les écritures lofty et les commandes IPC.

L'invariant 8 raconte le scénario complet :

> « La suppression créait une race condition fatale : le background scan (concurrent) re-lisait le
> fichier audio (pas encore écrit par lofty), réinsérait les anciennes métadonnées dans
> `METADATA_CACHE`, et **cette corruption persistait indéfiniment**. »

Et l'invariant 9 ajoute que le snapshot doit être pris **avant** le lock de `TRACKS_CACHE` « pour
éviter le deadlock ». On est à deux doigts d'écrire un moteur transactionnel à la main.

**Pour V2** : SQLite (`rusqlite`), un seul écrivain, des transactions. Les invariants 7, 8, 9 et V015
disparaissent par construction. Le démarrage instantané reste possible (une requête indexée sur une
table de pistes est plus rapide que parser plusieurs Mo de JSON). Coût : perdre l'inspection « à la
main » des caches, et une migration pour les beta-testeurs.

### 2.4 Des secrets qui manquent en silence

```rust
option_env!("HEAN_SENTRY_DSN")   // → None, sans erreur, sans warning
```

Six secrets compile-time. Si l'un manque, le binaire est livré **sans Sentry, sans feedback, sans
signature** — et rien ne le dit. La parade V1 est un script bash wrapper (`release.sh`) qui valide
les variables avant le build, plus deux invariants pour rappeler de ne jamais le contourner.

**Pour V2** : un `build.rs` avec `compile_error!()` quand une variable requise est absente en profil
release. La compilation échoue, il n'y a plus rien à se rappeler. Le script wrapper devient un confort,
pas un garde-fou.

### 2.5 La documentation dérive du code, et personne ne le voit

Relevé du jour :

| Affirmation documentée | Réalité |
|---|---|
| Spec : « 82 commandes Tauri » | 86 |
| Spec : « 29 variables CSS », « 7 813 lignes de CSS » | 60 variables, 10 654 lignes |
| Spec : « Version 0.1.0 » | 0.2.0-beta.1 |
| Spec : radius 2/6/8 px | 4/8/12 px |
| `DESIGN.md` : `--color-bg-elevated: #1e1e1e` | `#333` |
| `DESIGN.md` : `--color-error: #f87171` | `#ff4444` |
| `DESIGN.md` : « DM Sans — corps » | `body { font-family: var(--font-body) }` = **Geist Mono** ; DM Sans n'est utilisée que 7 fois et son chargement via Google Fonts est vraisemblablement bloqué par la CSP |
| `CLAUDE.md` : données dans `~/.local/share/noir/` | `~/Library/Application Support/noir/` (chemin Linux documenté pour une app macOS) |
| `hean-frontend` : thumbnails dans `~/.noir_desktop/` | ancien chemin, abandonné |
| `hean-release` : « étape 0 — valider `NOIR_GITHUB_FEEDBACK_TOKEN` » | cette variable **n'existe plus** (le skill le signale lui-même en encadré) |
| `network.js` écoute `network_source_connected` | aucun `emit` correspondant côté Rust |

Et le pire : **la meilleure documentation design du projet — 108 Ko, 28 sections, des aperçus live de
chaque composant, une référence écran par écran, une section anti-patterns et la seule section
accessibilité du projet — vivait hors du dépôt git**, dans
`~/Documents/Thomas/Noir Design System/noir-design-system.html`, référencée par une unique ligne du
skill `hean-frontend`. Elle n'était ni versionnée, ni sauvegardée, ni mentionnée dans les specs. Un
`rm -rf` malheureux, un changement de machine, et elle disparaissait. (Elle est désormais dans
[`../02-design/reference/`](../02-design/reference/).)

**Pour V2** : tout ce qui est référence vit dans le dépôt. Sans exception.

Aucune de ces dérives n'est grave isolément. Ensemble, elles signifient que **la documentation n'est
plus fiable sans vérification** — et c'est exactement ce qui pousse à relire 4 000 lignes de `lib.rs`
à chaque tâche. Détail complet des écarts design : [`../02-design/ECARTS-DESIGN.md`](../02-design/ECARTS-DESIGN.md).

**Pour V2** : générer ce qui peut l'être (liste de commandes, tokens CSS, inventaire des events)
depuis le code, en CI, et documenter à la main seulement ce qui ne se déduit pas du code — c'est-à-dire
les invariants et les décisions.

---

### 2.6 L'observabilité est un décor — ajoutée le 2026-11-21 (soir)

**252 `println!` contre 3 appels `tracing::`** dans le backend, et `logging.rs` ne capture pas
stdout. L'app installée a produit **63 lignes de journal** en tout. Donc : l'échec systématique de la
génération de thumbnails, l'activité du scan réseau de fond, les échecs de résolution de chemin SMB
— tout cela est annoncé en clair par le build de dev et **n'existe pas** chez les ~50 testeurs.

Les trois bugs trouvés en mesurant V1 (auto-updater en 404 depuis le lancement, nettoyage de
`smb_buffer` inopérant, thumbnails à 11 % de couverture) sont tous des bugs **silencieux**. Pas
subtils : silencieux. Avec un Sentry qui ne remonte que les panics et les erreurs JS, un bug qui ne
plante pas ne remonte jamais.

C'est la cause racine la plus coûteuse des cinq précédentes, parce qu'elle les rend **indétectables**.
Elle a été trouvée par accident, en regardant trente secondes de sortie d'un build de dev.

Détail et correction proposée : [`../../docs/MESURES-V1.md`](../../docs/MESURES-V1.md) §8.

## 3. Ce que V1 a fait bien, et qu'il faut absolument garder

Il serait absurde de tout jeter. Ces choix-là sont bons et validés en production :

1. **Tauri v2 + Rust + CoreAudio direct.** Le choix de parler au HAL plutôt que de passer par CPAL ou
   AVAudioEngine est ce qui rend le bit-perfect et le hog mode possibles. `coreaudio_backend.rs` est
   l'actif le plus précieux du dépôt.
2. **Le protocole `noir://`** plutôt que du base64 sur l'IPC : 700 Mo → 60 Ko de heap pour 1 000
   albums. Décision mesurée, à reprendre.
3. **Le virtual scroll à pool fixe** (60 nœuds, item 48 px) : 10 000+ pistes à 60 fps.
4. **La pratique de l'invariant.** Écrire la règle, sa gravité et *pourquoi elle existe*, le jour où
   la régression est corrigée — avec un hook `Stop` qui le rappelle quand du code change sans que le
   fichier d'invariants bouge. À transporter tel quel, c'est la meilleure chose dans ce dépôt.
5. **Les deux agents** `test-runner` et `invariant-guardian`, avec un protocole de session
   explicite (tests au début, tests + relecture des invariants à la fin).
6. **Le worker Cloudflare pour le feedback** : le binaire ne porte aucun token GitHub, seulement une
   URL et un secret partagé. Si le binaire fuit, on révoque un secret, pas un PAT.
7. **Sentry côté Rust uniquement**, erreurs JS forwardées : un seul DSN, pas 1,5 Mo de SDK, pas de
   double comptage. Avec anonymisation des paths testée.
8. **La CSP stricte** et la capability Tauri minimale.
9. **Le découplage audio/UI comme règle de travail** (« ne jamais modifier l'audio quand tu travailles
   sur l'UI, et inversement »).

---

## 4. Propositions pour V2 — à arbitrer, par ordre de rendement

| # | Proposition | Supprime | Coût | Avis |
|---|---|---|---|---|
| 1 | **TypeScript strict + bundler + IPC typée générée** (tauri-specta) | V014, les 9 bugs d'onboarding, le bug `result.id`, le médiateur non vérifié | moyen | 🟢 à faire |
| 2 | **SQLite comme store unique** de la bibliothèque | invariants 7, 8, 9, V015 | moyen | 🟢 à faire |
| 3 | **`build.rs` + `compile_error!` pour les secrets** | invariant 16, la moitié de `release.sh` | faible | 🟢 à faire |
| 4 | **Les 24 comportements anti-régression écrits comme tests AVANT de porter les features** | la dette de test de V1 (3 suites skippées car `invoke` non mockable) | moyen | 🟢 à faire |
| 5 | **Un seul point d'entrée pour charger des pistes** (`excluded_paths` dedans) | invariant 6 | faible | 🟢 à faire |
| 6 | **Store immuable** au lieu des objets partagés par référence | V013, invariant « ne jamais réassigner » | faible | 🟢 à faire |
| 7 | **CSS découpé par composant + tokens isolés** et générés | la dérive design, un fichier de 10 654 lignes | moyen | 🟢 à faire |
| 8 | **Limite dure de taille de fichier** (~400 lignes) | `lib.rs`, `views.js` | gratuit | 🟢 à faire |
| 9 | **Un framework de composants** (Svelte / Solid / Lit / React) au lieu du vanilla + innerHTML | la famille « cellule déconnectée du DOM », le rebuild manuel via rAF, `isConnected` | moyen-élevé | 🟡 à arbitrer — plus de dépendances, mais c'est là que sont 5 régressions V1 |
| 10 | **Message passing pour le moteur audio** (la « Phase 5 » jamais faite de V1) au lieu du `Mutex<Option<AudioEngine>>` global | une classe de deadlocks potentiels | moyen | 🟡 à arbitrer |
| 11 | **Remplacer libsmbclient** (client SMB pur Rust, ou build statique vendoré) | invariant 17, les ~92 dylibs, le fix LC_RPATH, l'étape la plus fragile de la release | **élevé et risqué** | 🔴 ne pas faire en V2 — garder pavao + le pipeline dylibbundler qui marche |
| 12 | Repartir sur un autre framework applicatif (Electron, SwiftUI, natif) | — | très élevé | 🔴 non — Tauri + CoreAudio est le bon choix et c'est démontré |

**Ce qu'il ne faut pas faire** : réécrire `coreaudio_backend.rs`, `coreaudio_stream.rs`,
`media_controls.rs` et le sous-système SMB. Ces fichiers sont du savoir condensé sur des APIs
hostiles ; les réécrire, c'est racheter les mêmes bugs. Porter, lire, comprendre, adapter — pas
réécrire. Voir [`../03-architecture/code-reference/README.md`](../03-architecture/code-reference/README.md).

---

## 5. Le vrai risque de V2

Ce n'est pas de rater l'architecture. C'est de **perdre la parité fonctionnelle sans s'en apercevoir**.

V1 contient 124 features indexées dans [`../01-produit/FEATURE-SET.md`](../01-produit/FEATURE-SET.md),
dont beaucoup sont des détails invisibles jusqu'à ce qu'ils manquent : le hint de permission « Réseau
local » sur l'étape NAS de l'onboarding, le fallback hostname `.local` quand le NAS répond en IPv6,
le fait que la scrollbar des carousels soit invisible au repos, le `grid-column: 1 / -1` sur les
loose tracks d'une page artiste, les labels du menu contextuel qui s'adaptent à la multi-sélection,
le `RunEvent::Reopen` pour le clic sur l'icône du Dock.

C'est pour ça que ce dossier existe, et c'est pour ça que `FEATURE-SET.md` est une **liste à cocher**
et pas un récit.
