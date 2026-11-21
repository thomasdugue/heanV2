# Feature set Hean — contrat de parité pour V2

> **Statut : source de vérité fonctionnelle.** Reconstruit le 2026-11-21 **à partir du code réel**
> (branche `claude/refonte-fab-5-2-3901f8`, commit `02508ea`), pas à partir de la spec de mars.
> La spec ([`03-architecture/SPEC-V1-COMPLETE.md`](../03-architecture/SPEC-V1-COMPLETE.md)) décrit
> l'état du 8 mars 2026 (v0.1.0) : elle est plus détaillée sur certains mécanismes, mais **périmée**
> sur les compteurs et sur le design. Ce fichier-ci l'emporte.
>
> Chaque ligne est une feature que V2 doit avoir pour être « à l'identique ».
> Cocher au fur et à mesure du portage.
>
> ⚠️ **Révisé le 2026-11-21 (soir).** Les 124 features sont justes — elles viennent du code. Mais
> **la cible de parité visuelle a changé** : ce n'est plus `02508ea` ni la beta distribuée, c'est le
> commit `e12a200` (redesign v3 porté sur main). Les colonnes « Réf. V1 » restent valables ; le
> design de chaque écran se lit désormais dans
> [`../02-design/reference/views.js`](../02-design/reference/views.js) et
> [`../02-design/reference/styles.css`](../02-design/reference/styles.css). Voir
> [`../02-design/ECARTS-DESIGN.md`](../02-design/ECARTS-DESIGN.md) §0.

Légende colonne **Risque** : 🔴 difficile / plateforme hostile · 🟠 piégeux · 🟢 direct.

---

## 1. Lecture audio (le cœur)

| # | Feature | Détail | Risque | Réf. V1 |
|---|---|---|---|---|
| A01 | Décodage multi-format | FLAC, ALAC, WAV, AIFF, MP3, AAC/M4A, OGG Vorbis — via Symphonia | 🟢 | `audio_decoder.rs` |
| A02 | Sortie CoreAudio directe | `kAudioUnitSubType_HALOutput`, pas de CPAL, pas d'AVAudioEngine | 🔴 | `audio/coreaudio_stream.rs` |
| A03 | Bit-perfect | Pas de resampling quand le DAC accepte le rate source ; EQ off ; volume 100 % | 🔴 | `audio/coreaudio_backend.rs` |
| A04 | Sample rate switching dynamique | Négociation `kAudioDevicePropertyNominalSampleRate` entre deux morceaux de rates différents | 🔴 | idem |
| A05 | Hog Mode (accès exclusif) | `kAudioDevicePropertyHogMode` + toggle UI + tooltip ; refusé sur AirPlay | 🔴 | idem |
| A06 | RingBuffer lock-free | `HeapRb<f32>`, 5 s de capacité, callback temps réel | 🟠 | `audio_engine.rs` |
| A07 | Resampler FFT | Rubato, uniquement si le DAC ne gère pas le rate source | 🟠 | `resampler.rs` |
| A08 | EQ paramétrique 8 bandes | Biquad IIR, bypass total si tous les gains ≈ 0 dB, courbe SVG interactive + presets | 🟠 | `eq.rs`, `src/eq.js` |
| A09 | Volume | Multiplication `sample × volume` f32 dans le callback (pas le volume système) | 🟢 | `audio_engine.rs` |
| A10 | Gapless playback | Double-buffering ; `audio_preload_next` à ~10 s restantes en local, ~60 s en SMB | 🔴 | `audio_engine.rs` |
| A11 | Seek instantané | Reset AudioUnit + repositionnement décodeur, pas de recréation de stream | 🟠 | `audio/coreaudio_stream.rs` |
| A12 | Device following | Détection hot-plug (polling), reprise sur le nouveau device, position préservée (`_lastGoodPosition`) | 🟠 | `coreaudio_backend.rs`, `playback.js` |
| A13 | AirPlay | Détection transport type, badge UI, stratégie de préservation du défaut système, retry 1,5 s, pas de changement de rate | 🔴 | voir [`SAVOIR-AUDIO.md`](../04-invariants/SAVOIR-AUDIO.md) |
| A14 | Bluetooth | Même traitement que AirPlay côté transport type (`0x626C7565`), hog mode désactivé | 🟠 | idem |
| A15 | Media keys macOS | souvlaki / `MPRemoteCommandCenter` — prend le contrôle même si Apple Music tourne | 🔴 | `media_controls.rs` |
| A16 | Now Playing système | `update_media_metadata` / `update_media_playback_state` (titre, artiste, artwork, état) | 🟢 | idem |
| A17 | Affichage specs audio | Source (rate/bit) → Output (rate/bit) dans la player bar + badge bit-perfect animé | 🟢 | `index.html`, `playback.js` |
| A18 | Progression 60 fps | Interpolation JS entre les events `playback_progress` du Rust, RAF stoppé à l'idle | 🟠 | `playback.js` |
| A19 | RMS temps réel | Niveau audio calculé dans le callback CoreAudio, consommé par le fullscreen player | 🟠 | `coreaudio_stream.rs` |
| A20 | Shuffle 3 modes | off → album → library ; la track courante est exclue du tirage ; `shufflePlayedTracks` | 🟠 | `playback.js` |
| A21 | Repeat 3 modes | off → all → one | 🟢 | idem |
| A22 | Playback context | `'album'` (stop en fin d'album) / `'library'` (séquentiel via `ui.tracksViewOrder`) / `'playlist'` / `null` | 🟠 | idem |
| A23 | Auto-resume au démarrage | Option dans Settings | 🟢 | `renderer.js` |

**Formats refusés volontairement** : DSD, MQA, Opus, WMA (extensions retirées du scanner).

---

## 2. Bibliothèque

| # | Feature | Détail | Risque | Réf. V1 |
|---|---|---|---|---|
| B01 | Démarrage instantané | `load_tracks_from_cache` lit `tracks_cache.json` avant tout scan ; UI utilisable immédiatement | 🟠 | `lib.rs` |
| B02 | Scan en arrière-plan | `start_background_scan`, parallélisé (rayon), events `scan_progress` / `scan_complete` | 🟠 | `lib.rs` |
| B03 | Scan différentiel | Ne relit que ce qui a changé (mtime / taille) ; NAS inclus | 🟠 | `network/scanner.rs` |
| B04 | Plusieurs dossiers de bibliothèque | Ajout / suppression, `library_paths` dans `config.json` | 🟢 | `lib.rs` |
| B05 | Extraction métadonnées | lofty : titre, artiste, album, album artist, année, n° piste, n° disque, genre, durée, format, sample rate, bit depth, bitrate | 🟢 | `lib.rs` |
| B06 | Écriture métadonnées | `write_metadata` écrit **dans le fichier audio** (lofty) + met à jour `METADATA_CACHE` et `TRACKS_CACHE` | 🔴 | invariants 7-9 |
| B07 | Édition inline (1 track) | Panel Track Info en mode édition : titre, artiste, album, année, n° piste, genre | 🟢 | `panels.js` |
| B08 | Bulk edit (N tracks) | Modal Artist / Album / Year / Genre, placeholder « mixed values », refresh inconditionnel de la vue | 🟠 | `panels.js` |
| B09 | Suppression permanente | `excluded_paths` dans `config.json`, filtré à **chaque** point d'entrée de chargement/scan | 🔴 | invariant 6 |
| B10 | Normalisation des genres | Mapping / nettoyage des genres bruts | 🟢 | `lib.rs` |
| B11 | Enrichissement genres | `trigger_genre_enrichment` via Deezer, events de progression, réversible (`reset_genre_enrichment`) | 🟠 | `lib.rs` |
| B12 | Groupement albums / artistes | 1 album = 1 entrée, clé `normalizeKey()` (trim + NFC) ; `artist.albums` est un **Array** | 🔴 | invariant 1, V014 |
| B13 | Lookup O(1) | `library.tracksByPath` Map, reconstruite après chaque scan/suppression | 🟢 | `library.js` |
| B14 | Stats de bibliothèque | `get_library_stats` : total tracks, répartition par format, taille | 🟢 | `lib.rs` |
| B15 | Dates d'ajout | `added_dates_cache.json`, alimenté aussi par le scan NAS | 🟠 | `lib.rs` |
| B16 | Détection de doublons | Alerte dans le panel Track Info si le même fichier existe à plusieurs endroits | 🟢 | `panels.js` |
| B17 | Dossiers inaccessibles | Event `library_paths_inaccessible` → avertissement UI (NAS débranché, disque éjecté) | 🟢 | `lib.rs` |
| B18 | Ouvrir dans le Finder | Via menu contextuel | 🟢 | `panels.js` |

---

## 3. Pochettes & images

| # | Feature | Détail | Risque | Réf. V1 |
|---|---|---|---|---|
| C01 | Pipeline cover 4 niveaux | `thumbnailCache` mémoire → `get_cover_thumbnail` (disque) → `get_cover` (tags) → `fetch_internet_cover` | 🔴 | `library.js` |
| C02 | Protocole `noir://` | Sert covers + audio depuis le disque ; path canonicalisé + boundary-check ; range requests ; `Cache-Control: immutable` | 🟠 | `lib.rs` |
| C03 | Thumbnails générés au scan | 80×80 JPEG dans `thumbnails/`, `generate_thumbnails_batch` | 🟢 | `lib.rs` |
| C04 | Covers internet | MusicBrainz + CoverArtArchive, rate-limited, cache `internet_not_found_cache.json` (TTL 30 j) | 🟠 | `lib.rs` |
| C05 | Images d'artistes | Deezer (+ WikiMedia), `fetch_artist_image` | 🟠 | `lib.rs` |
| C06 | Lazy loading | IntersectionObserver + file d'attente ; **l'`<img>` doit être dans le DOM avant l'appel** (`isConnected`) | 🔴 | invariant covers |
| C07 | Révélation progressive | Les covers apparaissent au fur et à mesure du chargement, sans faire sauter la grille | 🟢 | `views.js` |
| C08 | Extraction de couleur | Canvas 64×64 → k-means 5 couleurs → `pickAmbientColor()` (score saturation × brightness) | 🟠 | `fullscreen-player.js` |

---

## 4. NAS / SMB

| # | Feature | Détail | Risque | Réf. V1 |
|---|---|---|---|---|
| D01 | Découverte mDNS | `_smb._tcp.local.`, timeout 5 s, event `nas_device_found`, fallback hostname `.local` si IPv6 | 🟠 | `network/discovery.rs` |
| D02 | Connexion SMB2/3 | pavao → libsmbclient ; **singleton process-level**, tout passe par le mutex `CONNECTION` | 🔴 | `network/smb.rs` |
| D03 | Browse shares & dossiers | `smb_list_shares`, `smb_browse` | 🟢 | idem |
| D04 | Credentials Keychain | macOS Keychain + `PASSWORD_CACHE` session ; jamais `unwrap_or_default()` | 🔴 | invariants 4, V009 |
| D05 | Guest / domaine | `isGuest`, `domain` supportés à la connexion et à l'ajout de source | 🟢 | `network.js` |
| D06 | Sources réseau persistées | `network_sources.json` **sans mot de passe** (le mdp va au Keychain) | 🟠 | `network/mod.rs` |
| D07 | Scan NAS différentiel | `scan_network_source_cmd`, métadonnées + cover extraites à distance, `network_scan_cache.json` | 🟠 | `network/scanner.rs` |
| D08 | Progressive download | Thread OS, chunks 64 KB → `smb_buffer/{hash}.tmp` ; lecture démarre à 4 Mo (timeout 15 s) | 🔴 | `network/scanner.rs` |
| D09 | `SmbProgressiveFile` | `Read + Seek + MediaSource` avec attente bloquante ; `byte_len()` renvoie **toujours** `Some(...)` | 🔴 | `audio_decoder.rs` |
| D10 | Gapless sur SMB | `audio_preload_next` async SMB-aware, `cancel_previous=false`, déclenché à 60 s | 🔴 | idem |
| D11 | Retry connexion périmée | Reconnexion fraîche si `open_with()` échoue | 🔴 | invariant 3 |
| D12 | Propagation d'erreur download | `take_last_download_error()` quand `bytes_written == 0` | 🟠 | invariant 5 |
| D13 | Auto-reconnexion au démarrage | `autoReconnectNetworkSources()` sans dialogue Keychain au cold start | 🟠 | `network.js` |
| D14 | Toggle / sync / remove source | Dans Settings → Réseau | 🟢 | `network.js` |
| D15 | Nettoyage `smb_buffer` | Fichiers temp périmés supprimés au lancement | 🟢 | `lib.rs` |
| D16 | UI non bloquante NAS offline | `get_cover` et `write_metadata` ne gèlent plus l'UI quand le NAS est injoignable | 🟠 | commit `701f091` |

---

## 5. Playlists, favoris, queue

| # | Feature | Détail | Risque | Réf. V1 |
|---|---|---|---|---|
| E01 | CRUD playlists | Créer, renommer, supprimer, ajouter/retirer une track, réordonner | 🟢 | `playlists.js` |
| E02 | Import / export M3U | `import_playlist_m3u`, `export_playlist_m3u` | 🟢 | `lib.rs` |
| E03 | Créer une playlist depuis un album | Menu contextuel album | 🟢 | `playlists.js` |
| E04 | Mosaïque de covers | 0 → icône ♪ · 1 → image pleine · 2 → côte à côte · 3-4 → grille 2×2 (le 4e répète le 1er si 3) | 🔴 | invariants 10-13 |
| E05 | Mosaïque sidebar 28×28 | Positionnement **absolu** (`calc(50% - 0.5px)`), jamais CSS Grid (bug WebKit) ; rebuild auto via rAF | 🔴 | invariants 12-13 |
| E06 | Favoris | Set de paths, toggle, `Cmd+Shift+D`, vue dédiée | 🟢 | `playlists.js` |
| E07 | Queue | Ajouter / jouer après / retirer / vider ; indicateurs sur les tracks ; panel latéral 320 px | 🟢 | `panels.js` |
| E08 | Auto-queue | `autoQueueSource` { type, id, tracks } alimente la queue depuis album/library/playlist/mix | 🟠 | `state.js` |
| E09 | Boutons au survol d'une track | « Add to Queue » / « Add to Playlist » : opacity 0 → 1 au hover | 🟠 | invariant 14 |

---

## 6. Interface

| # | Feature | Détail | Risque | Réf. V1 |
|---|---|---|---|---|
| F01 | Fenêtre | 1200×800 (min 320×400), transparente, titlebar overlay, titre masqué, zone de drag 38 px | 🟠 | `tauri.conf.json` |
| F02 | Sidebar redimensionnable | 280 px par défaut, 180-400 px, handle 8 px, persistée en localStorage | 🟢 | `renderer.js` |
| F03 | Collapse sidebar < 900 px | 48 px, actions masquées, inline width effacée via `matchMedia` | 🟠 | V016 |
| F04 | 4 vues de nav | Home · Albums · Artistes · Pistes | 🟢 | `views.js` |
| F05 | Vues de détail | Album · Artiste (discographie + loose tracks) · Playlist · Discovery mix | 🟢 | idem |
| F06 | Home — Resume Playback | Dernière piste jouée | 🟢 | idem |
| F07 | Home — Recently Added | En **première** position, albums NAS inclus | 🟠 | idem |
| F08 | Home — Recently Played | Grille responsive 3/2/1 colonnes, max 6, dédup par album | 🟠 | invariant 2, V002 |
| F09 | Home — Top Artists | Carousel, 8 artistes les plus écoutés | 🟢 | idem |
| F10 | Home — Discovery Mixes | 5 mixes thématiques générés (Discover, Audiophile, Long Albums, Random Mix…), stables sur la session | 🟠 | idem |
| F11 | Home — carousels | Largeur `calc(100% + clamp(16px,4vw,30px) + 24px)`, scroll horizontal, scrollbar hover-only | 🔴 | V003, V011 |
| F12 | Home — cache hybride | `homeDataCache`, TTL 5 min, invalidé par `scan_complete` conditionnel | 🟠 | V004 |
| F13 | Animation sine-wave | SVG sur la home, `animation-play-state: paused` hors vue | 🟢 | `styles.css` |
| F14 | Virtual scroll (Pistes) | Pool de 60 nœuds, item 48 px, buffer 10, event delegation, spacer = n × 48 | 🔴 | invariant virtual scroll, V006 |
| F15 | Multi-sélection | Cmd/Ctrl+clic, Shift+clic (plage), `selectedTrackPaths` Set, classe `.track-item-selected` | 🟠 | `views.js` |
| F16 | Tri | Pistes : par colonne (titre/artiste/album/qualité/durée) · Albums : artiste/album asc/desc · Artistes : nom asc/desc | 🟢 | `views.js` |
| F17 | Scrollbar alphabétique | Sur les grilles Albums et Artistes | 🟢 | idem |
| F18 | Recherche live | Index inversé `Map<mot, Set<index>>`, prefix + substring, scoring multi-mots, debounce 200 ms, < 16 ms sur 10 k pistes | 🟠 | `search.js` |
| F19 | Filtre de la vue Pistes | `updateTracksFilter()` en temps réel sur le virtual scroll | 🟢 | idem |
| F20 | Menu contextuel track | Lire / Lire après / Queue / Playlist (sous-menu) / Éditer (1 ou N) / Aller à l'album / Aller à l'artiste / Finder / Supprimer — labels adaptés à la multi-sélection, position viewport-aware | 🟠 | `panels.js` |
| F21 | Menu contextuel album | Lire, queue, créer playlist, supprimer (HTML séparé) | 🟢 | idem |
| F22 | Panel Track Info | Cover, liens artiste/album, badge qualité, specs (format/rate/bitrate/bit depth), métadonnées, chemin sélectionnable, refresh, edit — `Cmd+Shift+I` | 🟢 | idem |
| F23 | Panel Queue | 320 px, `Cmd+Shift+L` | 🟢 | idem |
| F24 | Panel Settings | Audio · Bibliothèque · Réseau · Lecture · Raccourcis · Privacy · Mises à jour | 🟢 | `renderer.js` |
| F25 | Panel EQ | Courbe SVG, presets, toggle, `Cmd+Shift+E` | 🟢 | `eq.js` |
| F26 | Panel Lyrics | Paroles synchronisées, scrollable | 🟢 | `lyrics.js` |
| F27 | Un seul panel à la fois | `closeAllPanels()` | 🟢 | `panels.js` |
| F28 | Fullscreen player | Particle system 3 phases, couleurs extraites de la cover, RMS, next track, z-index 10000 | 🟠 | `fullscreen-player.js` |
| F29 | Fullscreen lyrics | Overlay superposé au fullscreen player | 🟢 | `lyrics.js` |
| F30 | Drag & drop custom | HTML5 drag cassé dans la WebView → mousedown/mousemove 5 px/mouseup, ghost suivant le curseur (+15 px), Escape annule | 🔴 | V005 |
| F31 | Zones de drop | Playlist (ajoute) · sidebar playlists (crée une playlist) · surbrillance `.drag-over` | 🟢 | `drag.js` |
| F32 | Toasts | Bas droite, slide-up, auto-dismiss 3 s, success / error | 🟢 | `utils.js` |
| F33 | Modal de confirmation | Suppressions | 🟢 | `index.html` |
| F34 | Module d'indexation | Dans la sidebar : progression, dossier courant, %, repliable | 🟢 | `library.js` |
| F35 | Transitions de vue | `transitionView` async : fade-out 130 ms → `await render()` → fade-in 200 ms, `renderVersion` annule les renders obsolètes | 🔴 | V001 |
| F36 | Historique de navigation | `ui.navigationHistory`, retour arrière | 🟢 | `views.js` |
| F37 | Clic sur le Dock | `RunEvent::Reopen` → réaffiche la fenêtre masquée | 🟠 | `lib.rs` |

---

## 7. Raccourcis clavier

Locaux (in-app) : Espace (play/pause hors input) · `Cmd+Shift+←/→` (piste préc./suiv.) · `Cmd+Shift+Espace` (play/pause) · `Cmd+Shift+↑/↓` (volume) · `Cmd+Shift+D` (favori) · `Cmd+Shift+P` (focus recherche) · `Cmd+Shift+L` (queue) · `Cmd+Shift+I` (track info) · `Cmd+Shift+E` (EQ) · `Cmd+I` (éditer la track courante) · `Escape` (fermer).

Globaux (OS, via `tauri-plugin-global-shortcut`) : `MediaPlayPause`, `MediaTrackNext`, `MediaTrackPrevious`, `MediaVolumeUp/Down`, avec fallback `Cmd+Shift+…` si l'enregistrement échoue.
**Jamais F7/F8/F9** (conflit Apple Music) — les media keys passent par souvlaki.

Personnalisables : remapping avec capture de touche + reset, persisté en localStorage.

---

## 8. Onboarding (6 étapes)

Affiché si aucune source de bibliothèque n'est configurée. Étapes : bienvenue → choix de la source → dossier local → découverte NAS (avec hint permission « Réseau local » macOS) → scan avec progression → fin.
Compteurs par format affichés (`data.stats.mp3_count`…). ~964 lignes dans `onboarding.js` — prototype HTML conservé dans [`02-design/reference/prototype-onboarding.html`](../02-design/reference/prototype-onboarding.html).

---

## 9. Observabilité, feedback, distribution

| # | Feature | Détail | Risque | Réf. V1 |
|---|---|---|---|---|
| G01 | Sentry (Rust) | Panics via panic hook ; DSN compile-time `option_env!("HEAN_SENTRY_DSN")` | 🟠 | `sentry_init.rs` |
| G02 | Erreurs JS → Sentry | `window.error` + `unhandledrejection` → `report_js_error` → `capture_event` Rust. **Pas** de `@sentry/browser` | 🟠 | `error-tracking.js` |
| G03 | Anonymisation | `scrub_event` : `/Users/<name>/` → `<HOME>/` dans message, breadcrumbs, exceptions ; `send_default_pii: false` | 🔴 | `sentry_init.rs` (5 tests) |
| G04 | Opt-out RGPD | Settings → Privacy ; `AtomicBool` runtime + `config.json` ; `before_send` drop si false | 🔴 | invariant 15 |
| G05 | Logs persistés | tracing-appender, rotation journalière, 7 jours, `~/Library/Application Support/noir/logs/` | 🟢 | `logging.rs` |
| G06 | Modal feedback | Type (Bug/UX/Feature/Other) · sévérité conditionnelle · titre 80 car. avec compteur · description ≥ 10 car. · email optionnel · contexte auto-capturé · `Cmd+Enter` · shake sur erreur | 🟢 | `feedback.js` |
| G07 | « Attach recent logs » | `get_recent_logs(maxKb: 200)` préfixé à la description | 🟢 | `lib.rs` |
| G08 | Feedback → GitHub Issues | Via **Cloudflare Worker** qui détient le `GITHUB_TOKEN` ; l'app ne porte qu'URL + secret partagé | 🔴 | `cloudflare-worker.js` |
| G09 | Fallback local | Sauvegarde JSON dans `feedback/` **toujours**, même si le POST réussit | 🟢 | `lib.rs` |
| G10 | Auto-update | `tauri-plugin-updater`, `latest.json` sur GitHub Releases, check au démarrage (configurable) + manuel | 🟠 | `auto-update.js` |
| G11 | Signature + notarisation | App **et** DMG notarisés séparément ; `minimumSystemVersion: 11.0` | 🔴 | [`SAVOIR-RELEASE.md`](../04-invariants/SAVOIR-RELEASE.md) |
| G12 | Dylib bundling | ~92 dylibs Samba embarquées + fix LC_RPATH dupliqués + re-sign | 🔴 | invariant 17 |
| G13 | DMG brandé | `generate-dmg.py` + `dmg-window.html` (700×390) | 🟢 | `scripts/` |

---

## 10. Ce qui n'existe pas en V1

Voir [`NON-GOALS.md`](NON-GOALS.md). En résumé : Windows/Linux, DSD/MQA/Opus/WMA, scrobbling, Roon/Audirvana/TIDAL/Qobuz, sync iCloud, crossfade, ReplayGain, visualiseur FFT, AutoEQ, DAP Sync (archivé).

## 11. Hors parité, mais à reprendre : WiFi Sync

**WiFi Sync n'est pas dans les 124 lignes ci-dessus** — il n'a jamais été distribué, donc il ne fait
pas partie de « l'identique ». Mais ce n'est pas non plus un non-goal : c'est 5 677 lignes de Rust
qui **fonctionnaient**, avec 30 tests, et c'est le remplaçant prévu de la sync MTP.

Le DAP se connecte avec son navigateur sur le WiFi, le Mac sert un HTTPS + QR code, les albums
partent en ZIP streamés. Le cahier des charges de l'étape suivante est déjà écrit.

→ [`../05-postmortem/chantier-wifi-sync/README.md`](../05-postmortem/chantier-wifi-sync/README.md)

**Décision à prendre** (elle n'est pas prise) : WiFi Sync entre-t-il dans le périmètre de V2, et à
quelle phase ? Il est indépendant du reste — il n'a besoin que de la bibliothèque et des covers —
donc il peut arriver tard sans bloquer quoi que ce soit.
