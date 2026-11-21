# Architecture V1 — état réel au 2026-11-21

> Ce document décrit **ce qui existe**, pas ce qu'il faut refaire. Pour le jugement sur ce qui doit
> changer, voir [`05-postmortem/DETTE-TECHNIQUE-V1.md`](../05-postmortem/DETTE-TECHNIQUE-V1.md).
> Mesures prises sur le commit `02508ea`.

## Vue d'ensemble

```
┌──────────────────────────────────────────────────────────────┐
│ FRONTEND — 16 076 lignes JS (20 modules ES6, aucun bundler)  │
│ + index.html (938 l.) + styles.css (10 654 l., un seul fichier) │
│ Servi tel quel depuis src/ (frontendDist: "../src")          │
└───────────────────────────┬──────────────────────────────────┘
                            │ 86 commandes Tauri (invoke) + 17 events (listen)
┌───────────────────────────▼──────────────────────────────────┐
│ BACKEND RUST — 12 319 lignes                                  │
│ lib.rs (4 870 l.) = 86 commandes + setup + 15 caches globaux  │
│ ┌─ audio/ ──────────────────────────────────────────────────┐ │
│ │ coreaudio_backend.rs 1 326 · coreaudio_stream.rs 667      │ │
│ │ backend.rs 184 (trait) · types.rs 148 · stream.rs 118     │ │
│ └───────────────────────────────────────────────────────────┘ │
│ audio_engine.rs 1 052 · audio_decoder.rs 908 · resampler 172  │
│ eq.rs 234 · media_controls.rs 136                             │
│ ┌─ network/ ────────────────────────────────────────────────┐ │
│ │ scanner.rs 868 · smb.rs 611 · smb_utils.rs 200            │ │
│ │ mod.rs 156 · credentials.rs 95 · discovery.rs 85          │ │
│ └───────────────────────────────────────────────────────────┘ │
│ sentry_init.rs 210 · logging.rs 170                           │
└──────────────────────────────────────────────────────────────┘
       │                                           │
       ▼                                           ▼
  CoreAudio HAL → DAC                      libsmbclient (pavao)
                                           + ~92 dylibs Samba
```

**Total ≈ 29 900 lignes** (JS 16 076 + Rust 12 319 + CSS 10 654 + HTML 938 ≈ 39 987 avec le markup).
Tauri v2, macOS uniquement, `withGlobalTauri: true`, fenêtre transparente + titlebar overlay.

## Frontend — les 20 modules

| Module | Lignes | Rôle |
|---|---:|---|
| `views.js` | 3 512 | **toutes** les vues, virtual scroll, `transitionView`, carousels, tri |
| `playback.js` | 1 983 | contrôle audio via invoke, gapless, interpolation 60 fps, devices, shuffle/repeat |
| `playlists.js` | 1 589 | CRUD playlists, favoris, mosaïques de covers |
| `panels.js` | 1 567 | queue, track info, édition métadonnées, menus contextuels, bulk edit |
| `renderer.js` | 1 090 | orchestrateur, `init()`, settings, listeners globaux |
| `onboarding.js` | 964 | parcours 6 étapes |
| `library.js` | 957 | scan, covers, thumbnails, groupement albums/artistes |
| `network.js` | 868 | NAS/SMB (modals, discovery, sources) |
| `shortcuts.js` | 547 | raccourcis globaux + locaux, remapping |
| `fullscreen-player.js` | 537 | overlay immersif, particules, extraction de couleur, RMS |
| `utils.js` | 488 | `showToast`, `escapeHtml`, `formatTime`, `clearObject`, `SHUFFLE_ICON` |
| `eq.js` | 392 | courbe SVG, presets |
| `search.js` | 338 | index inversé, scoring |
| `lyrics.js` | 282 | lrclib.net + lyrics.ovh, synchronisation |
| `feedback.js` | 250 | modal de feedback |
| `drag.js` | 209 | drag & drop custom (HTML5 cassé dans la WebView) |
| `state.js` | 196 | **état centralisé**, partagé par référence |
| `app.js` | 119 | **médiateur**, 95 slots `null` remplis à l'init |
| `auto-update.js` | 103 | plugin updater |
| `error-tracking.js` | 85 | `window.error` + `unhandledrejection` → Sentry |

Fichiers copiés pour référence dans [`code-reference/js/`](code-reference/js/) :
`state.js`, `app.js`, `utils.js`, `drag.js`, `error-tracking.js`.

### Le pattern médiateur (`app.js`)

Un objet exporté avec ~95 propriétés initialisées à `null`. Chaque module remplit ses slots à
l'initialisation ; les autres appellent `app.showToast(...)` au lieu d'importer.
**Raison d'être** : éviter les dépendances circulaires sans bundler.
**Coût** : aucune vérification statique — une faute de frappe sur un slot est un
`undefined is not a function` à l'exécution, et uniquement sur le chemin de code concerné.

### L'état partagé (`state.js`)

9 objets exportés — `playback`, `library`, `search`, `ui`, `queue`, `sort`, `caches`, `favorites`,
`contextMenu`, `dom`, plus `PERF` et `HOME_CACHE_TTL`.

**Règle absolue** : on mute les propriétés, on ne réassigne **jamais** l'objet, sinon les autres
modules gardent l'ancienne référence :

```js
library.tracks.length = 0; library.tracks.push(...newTracks)   // ✅
library.tracks = newTracks                                      // ❌
clearObject(library.albums)                                     // ✅ vide sans casser la ref
```

Champs les plus sensibles :
- `playback.playbackContext` — `'album'` / `'library'` / `'playlist'` / `null` : détermine ce qui se
  passe en fin de piste.
- `ui.tracksViewOrder` — paths dans l'**ordre visuel** de la vue Pistes. La navigation séquentielle
  en contexte `'library'` doit passer par là, jamais par `library.tracks[i ± 1]` (l'ordre de
  `library.tracks` est l'ordre de scan).
- `library.tracksByPath` — Map pour les lookups O(1), reconstruite après chaque scan.
- `caches.coverCache` / `thumbnailCache` — stockent des **URLs `noir://`** (~60 octets), pas du
  base64 (~700 Ko). Gain mesuré : 700 Mo → 60 Ko de heap pour 1 000 albums.

### Patterns frontend imposés

```js
try { await invoke('cmd', { arg }) } catch (e) { showToast('Message utilisateur') }  // invoke toujours wrappé
invoke('save_all_caches').catch(console.error)                                       // fire-and-forget assumé

grid.addEventListener('click', (e) => {            // event delegation, jamais un listener par item
  const card = e.target.closest('.album-card')
  if (card?.dataset.albumKey) showAlbumDetail(card.dataset.albumKey)
})

if (myAbort) myAbort.abort()                       // listeners document transitoires → AbortController
myAbort = new AbortController()
document.addEventListener('click', closeHandler, { signal: myAbort.signal })

await transitionView(async () => { await displayHomeView() })   // renderVersion annule les renders obsolètes
```

## Backend — chaîne audio

```
File | SmbProgressiveFile
  → [Symphonia decoder]            → f32 interleaved
  → [Resampler FFT (rubato)]        uniquement si le DAC ne gère pas le rate source
  → [RingBuffer lock-free]          HeapRb<f32>, 5 s
  → (thread callback temps réel)
  → [EQ 8 bandes biquad]            bypassé si tous les gains ≈ 0 dB
  → [Volume]                        sample × volume (f32)
  → [CoreAudio HAL]                 kAudioUnitSubType_HALOutput, device explicite
  → DAC
```

**Bit-perfect** = pas de resampling **et** EQ off **et** volume 100 % **et** hog mode.
Détails et pièges : [`04-invariants/SAVOIR-AUDIO.md`](../04-invariants/SAVOIR-AUDIO.md).

## Backend — NAS/SMB

```
network/
├── mod.rs          NetworkSource, SmbCredentials ; load/save network_sources.json
├── smb.rs          client pavao ; mutex CONNECTION ; browse / read_file / read_file_head
├── scanner.rs      scan différentiel ; start_progressive_download ; metadata+cover distantes
├── credentials.rs  Keychain macOS + PASSWORD_CACHE session
├── discovery.rs    mDNS/Bonjour (_smb._tcp.local., 5 s)
└── smb_utils.rs    résolution smb:// → chemin local
```

URI : `smb://{source_id}/{share}/{remote_path}` où `source_id` est l'UUID de la `NetworkSource`.
**Contrainte dure** : `libsmbclient` est un singleton au niveau du process — deux `SmbClient`
concurrents donnent EINVAL (os error 22). Tout passe par `CONNECTION: Lazy<Mutex<...>>`.
Détails : [`04-invariants/SAVOIR-SMB.md`](../04-invariants/SAVOIR-SMB.md).

## Les 15 états globaux Rust

| Static | Type | Rôle |
|---|---|---|
| `METADATA_CACHE` | `Mutex<MetadataCache>` | métadonnées par path — **source des invariants 7-9** |
| `TRACKS_CACHE` | `Mutex<TracksCache>` | liste complète des pistes (démarrage instantané) |
| `COVER_CACHE` | `Mutex<CoverCache>` | chemins des covers extraites |
| `INTERNET_NOT_FOUND_CACHE` | `Mutex<…>` | covers introuvables en ligne (TTL 30 j) |
| `ADDED_DATES_CACHE` | `Mutex<…>` | dates d'import |
| `LISTENING_HISTORY` | `Mutex<…>` | 1 000 dernières écoutes |
| `CACHE_DIRTY` | `Mutex<bool>` | flag de sauvegarde différée |
| `WRITE_METADATA_LOCK` | `Mutex<()>` | sérialise les écritures lofty |
| `AUDIO_ENGINE` | `Mutex<Option<AudioEngine>>` | moteur |
| `APP_HANDLE` | `Mutex<Option<AppHandle>>` | pour émettre des events hors commande |
| `HTTP_CLIENT` | `Client` | reqwest partagé |
| `NETWORK_SOURCES` | `Mutex<Vec<NetworkSource>>` | sources NAS |
| `CONNECTION` | `Mutex<Option<ActiveConnection>>` | **le** client SMB (singleton) |
| `LAST_CREDENTIALS` | `Mutex<Option<StoredCredentials>>` | pour la reconnexion |
| `PASSWORD_CACHE` | `Mutex<HashMap<String,String>>` | évite les dialogues Keychain |
| `CURRENT_DOWNLOAD_CANCEL` | `Mutex<Option<Arc<AtomicBool>>>` | annulation du download SMB courant |
| `LAST_DOWNLOAD_ERROR` | `Mutex<Option<String>>` | propagation d'erreur du thread de download |
| `SENTRY_ENABLED` | `AtomicBool` | toggle RGPD lu dans `before_send` |
| `GENRE_MAP` / `DEEZER_GENRE_MAP` / `ID3V1_GENRES` | tables statiques | normalisation des genres |

**C'est beaucoup.** 7 de ces états sont des caches qui doivent rester cohérents entre eux pendant
qu'un scan tourne en arrière-plan ; c'est l'origine directe de 4 invariants BLOQUANTS (voir le
postmortem).

## Protocole `noir://`

Sert les fichiers audio **et** les covers depuis le disque. Path canonicalisé + boundary-check contre
`library_paths`, range requests supportées, `Cache-Control: max-age=31536000, immutable`.
Remplace le base64 sur l'IPC (33 % d'overhead + double sérialisation JSON évités).

## APIs externes (reqwest, côté Rust)

MusicBrainz + CoverArtArchive (covers), Deezer (images d'artistes + genres), WikiMedia,
lrclib.net + lyrics.ovh (paroles, appelées côté JS). Tous ces domaines sont dans la CSP
(`connect-src`), ainsi que `github.com`.

## Tests V1

| Suite | État |
|---|---|
| Jest (`src/__tests__/`, 15 fichiers) | **149 pass / 14 skipped / 0 fail** |
| Skipped | `Navigation`, `PlayerControls`, `AlbumView` — nécessitent de mocker `invoke` Tauri |
| `cargo test` | ~100 pass / 0 fail / 11 ignored (`network_source`, `tauri_commands` ignorés) |
| Analyse statique JS | `node --check` uniquement (syntaxe). **Pas de linter, pas de types.** |
| CI | `.github/workflows/ci.yml` — `node --check` + Jest (ubuntu), `cargo test --locked` (macOS), sur chaque PR vers `main` |

Fichiers de test couvrant un invariant : `AlbumGrouping.test.js` (invariant 1),
`ShuffleButton.test.js` (invariant 19), `sentry_init` (5 tests d'anonymisation).
