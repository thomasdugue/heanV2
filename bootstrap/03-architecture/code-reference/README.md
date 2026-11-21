# Code de référence V1 — porter ou réécrire ?

Ce dossier contient des **copies** de fichiers V1 (commit `02508ea`). Ce n'est pas du code à
compiler : c'est de la matière première. Tout n'a pas la même valeur.

La distinction qui compte : certains fichiers encodent du **savoir sur la plateforme** (CoreAudio,
libsmbclient, WebKit) qui a coûté des semaines et qui ne se redécouvre pas en lisant la doc Apple.
D'autres encodent juste des **choix d'architecture V1** qu'on veut précisément changer.

## 🟩 À porter quasiment tel quel

| Fichier | Lignes | Pourquoi |
|---|---:|---|
| `rust/audio/coreaudio_backend.rs` | 1 326 | Le plus précieux du dépôt. Énumération des devices, transport types, négociation de sample rate, hog mode, et toute la **stratégie AirPlay** (préservation du défaut système, devices stale, session cache). Réécrire = re-découvrir que `set_system_default_device` accepte silencieusement sans effet sur un device AirPlay stale. |
| `rust/audio/coreaudio_stream.rs` | 667 | Callback temps réel, assignement AudioUnit explicite, transition gapless (`current_path` / `next_path`), seek par reset. |
| `rust/audio/types.rs` + `backend.rs` | 332 | `DeviceInfo` (avec `transport_type` + `is_airplay`), trait de backend prévu pour un futur WASAPI. Bonne abstraction, à garder. |
| `rust/resampler.rs` | 172 | Wrapper rubato, testé. |
| `rust/eq.rs` | 234 | 8 biquads + bypass. Simple et juste. |
| `rust/media_controls.rs` | 136 | souvlaki / `MPRemoteCommandCenter`. Petit mais c'est la seule façon de battre Apple Music sur les media keys. **Ne pas retirer `unsafe impl Send for MediaControlsWrapper`.** |
| `rust/network/smb.rs` + `credentials.rs` + `discovery.rs` | 791 | Singleton `CONNECTION`, retry sur connexion périmée, Keychain + cache session, mDNS. Trois invariants BLOQUANTS vivent là. |
| `rust/sentry_init.rs` | 210 | Anonymisation des paths + toggle `AtomicBool` dans `before_send`, avec 5 tests. Conforme RGPD, déjà validé. |

## 🟨 À porter en adaptant

| Fichier | Lignes | Ce qui doit changer |
|---|---:|---|
| `rust/audio_engine.rs` | 1 052 | Le pipeline (RingBuffer → EQ → volume → HAL) est bon. La spec V1 listait une « Phase 5 : architecture audio (RwLock + message passing) » jamais faite : l'engine est derrière un `Mutex<Option<AudioEngine>>` global. À reprendre avec du message passing dès le départ. |
| `rust/audio_decoder.rs` | 908 | `SmbProgressiveFile` (`Read + Seek + MediaSource` avec attente bloquante, `byte_len()` qui renvoie toujours `Some`) est à garder tel quel. Le reste du wrapper Symphonia peut être simplifié. |
| `rust/network/scanner.rs` | 868 | Le scan différentiel et `start_progressive_download` (chunks 64 Ko, seuil 4 Mo, registre `PROGRESSIVE_DOWNLOADS`) sont bons. Les accès aux caches globaux changent si V2 passe en SQLite. |
| `rust/logging.rs` | 170 | OK, juste corriger le commentaire d'en-tête qui annonce un chemin Linux. |
| `js/drag.js` | 209 | Le drag HTML5 **ne marche pas** dans la WebView : l'implémentation mousedown/mousemove 5 px/mouseup reste nécessaire quel que soit le framework. À reprendre comme hook/composant. |
| `js/error-tracking.js` | 85 | `window.error` + `unhandledrejection` → `report_js_error`. Décision à garder (pas de `@sentry/browser`, 1,5 Mo pour rien). |

## 🟥 À lire, puis réécrire — ne pas porter

| Fichier | Lignes | Pourquoi |
|---|---:|---|
| `js/state.js` | 196 | Document l'état complet de l'app : **très utile à lire** pour savoir ce que V2 doit modéliser. Mais le pattern « objets partagés par référence qu'on ne réassigne jamais » est précisément ce qui produit les régressions V013/V014. À remplacer par un store typé. |
| `js/app.js` | 119 | Les 95 slots du médiateur sont un **catalogue des interactions inter-modules** — lisez-le comme une carte des dépendances. Le pattern lui-même (service locator non typé) est à abandonner. |
| `js/utils.js` | 488 | Contient `escapeHtml` (attention : doit échapper les guillemets, cf. commit `bfb2bcf`), `formatTime`, `showToast`, `clearObject`, `SHUFFLE_ICON` (⤮ U+292E — **pas** ⤢ U+2922). À redispatcher proprement. |
| `Cargo.toml.v1` | — | Liste de dépendances validée en production ; à reprendre comme point de départ en mettant les versions à jour (lofty 0.18, reqwest 0.11, sentry 0.47 ont des versions plus récentes aujourd'hui). |
| `tauri.conf.json.v1` | — | **La CSP est à reprendre presque telle quelle** (elle est stricte et juste), sauf le trou DM Sans / Google Fonts décrit dans [`../02-design/ECARTS-DESIGN.md`](../../02-design/ECARTS-DESIGN.md). Noter `minimumSystemVersion: "11.0"`, `transparent: true`, `titleBarStyle: "Overlay"`, `hiddenTitle: true`, et la pubkey updater (qui doit matcher `~/.tauri/hean.key`). |

## Non copiés ici (et pourquoi)

- `lib.rs` (4 870 lignes) — c'est le fichier à ne pas reproduire : 86 commandes, 15 états globaux, la
  normalisation des genres et tout le reste dans un seul fichier. Consultable dans le dépôt V1 au
  besoin ; sa surface est inventoriée dans [`../API-IPC-V1.md`](../API-IPC-V1.md).
- `views.js` (3 512), `playback.js` (1 983), `playlists.js` (1 589), `panels.js` (1 567),
  `renderer.js` (1 090), `onboarding.js` (964), `library.js` (957), `network.js` (868) — mêmes
  raisons. Leur **comportement** est capturé dans
  [`../../01-produit/FEATURE-SET.md`](../../01-produit/FEATURE-SET.md) et
  [`../../01-produit/UX-ANTI-REGRESSION.md`](../../01-produit/UX-ANTI-REGRESSION.md), et leur
  **rendu** dans [`../../02-design/reference/`](../../02-design/reference/).
- Les tests V1 (`src/__tests__/`, 15 fichiers Jest) — à relire avant d'écrire ceux de V2 :
  `AlbumGrouping.test.js` et `ShuffleButton.test.js` sont les deux seuls qui gardent un invariant.
