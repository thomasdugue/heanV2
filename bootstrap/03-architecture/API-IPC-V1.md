# Surface IPC V1 — 86 commandes Tauri + 17 events

Inventaire exhaustif extrait de `src-tauri/src/lib.rs` (commit `02508ea`). Les catégories sont celles
des commentaires du `generate_handler!`. La spec de mars en annonçait 82 : 4 ont été ajoutées depuis
(`report_js_error`, `get_recent_logs`, `get_sentry_enabled`, `set_sentry_enabled` — la session
observabilité du 2026-05-01).

C'est la surface fonctionnelle à couvrir en V2. Elle n'a pas à être identique commande par commande —
mais chaque ligne correspond à un besoin réel du frontend.

## Cache & métadonnées (20)

`init_cache` · `save_all_caches` · `scan_folder` · `scan_folder_with_metadata` · `get_metadata` ·
`refresh_metadata` · `load_all_metadata_cache` · `get_added_dates` · `get_cover` ·
`get_cover_base64` · `get_cover_thumbnail` · `generate_thumbnails_batch` · `fetch_internet_cover` ·
`fetch_artist_image` · `clear_cache` · `add_library_path` · `remove_library_path` ·
`exclude_tracks_from_library` · `get_library_paths` · `select_folder`

## M3U (2)

`export_playlist_m3u` · `import_playlist_m3u`

## Playlists (7)

`get_playlists` · `create_playlist` · `rename_playlist` · `delete_playlist` ·
`add_track_to_playlist` · `remove_track_from_playlist` · `reorder_playlist_tracks`

## Favoris (3)

`toggle_favorite` · `is_favorite` · `get_favorites`

## Audio engine (9)

`audio_play` · `audio_pause` · `audio_resume` · `audio_stop` · `audio_seek` · `audio_set_volume` ·
`audio_get_state` · `audio_preload_next` · `set_gapless_enabled`

## Audio backend & devices (9)

`get_audio_devices` · `refresh_audio_devices` · `get_current_audio_device` · `set_audio_device` ·
`get_system_default_device_id` · `get_audio_sample_rate` · `set_exclusive_mode` ·
`is_exclusive_mode` · `hog_mode_status`

## Égaliseur (3)

`set_eq_enabled` · `set_eq_bands` · `get_eq_state`

## Historique d'écoute (7)

`record_play` · `get_listening_history` · `get_last_played` · `get_recent_albums` ·
`get_all_played_albums` · `get_all_played_paths` · `get_top_artists`

## Démarrage instantané & scan (3)

`load_tracks_from_cache` · `start_background_scan` · `get_library_stats`

## Genres (2)

`trigger_genre_enrichment` · `reset_genre_enrichment`

## Écriture de métadonnées (1)

`write_metadata`

## Feedback & observabilité (5)

`submit_feedback` · `report_js_error` · `get_recent_logs` · `get_sentry_enabled` ·
`set_sentry_enabled`

## Réseau / NAS (12)

`discover_nas_devices` · `smb_connect` · `smb_list_shares` · `smb_browse` · `add_network_source` ·
`remove_network_source` · `get_network_sources` · `toggle_network_source` ·
`update_network_source_credentials` · `get_network_status` · `reconnect_network_source` ·
`scan_network_source_cmd`

> ⚠️ `add_network_source` renvoie un **objet `NetworkSource`**, pas une string. Les appelants doivent
> utiliser `result.id` (piège qui a coûté un bug de scan NAS à 0 piste, cf. session du 2026-03-09).

## Media controls (2)

`update_media_metadata` · `update_media_playback_state`

## Application (1)

`quit_app`

---

## Events backend → frontend (15 émis)

| Event | Émis quand | Payload / remarque |
|---|---|---|
| `scan_progress` | pendant un scan | `phase`, `current`, `total`, `folder` |
| `scan_complete` | fin de scan | `new_tracks`, `removed_tracks`, `stats` — **le frontend ne recharge que si `new_tracks > 0 \|\| removed_tracks > 0`** (sinon race au démarrage) |
| `genre_enrichment_progress` | enrichissement genres | progression |
| `genre_enrichment_complete` | fin d'enrichissement | invalide les caches de carousels |
| `library_paths_inaccessible` | dossier/NAS injoignable | avertissement UI |
| `nas_device_found` | découverte mDNS | lu par `currentModalState` dans `network.js` |
| `playback_loading` | ouverture d'un fichier | |
| `playback_progress` | position de lecture | base de l'interpolation 60 fps côté JS |
| `playback_paused` / `playback_resumed` | état | arrête / relance la boucle RAF |
| `playback_ended` | fin de piste | |
| `playback_error` | erreur de lecture | |
| `playback_seeking` | seek en cours | |
| `playback_audio_specs` | specs de la source | rate + bit depth → affichage player bar |
| `playback_gapless_transition` | transition gapless | met à jour `currentTrackIndex` |

## Events écoutés côté JS mais non émis par le Rust de l'app (3)

`media-control` (émis par le plugin / souvlaki) · `network_source_connected` /
`network_source_disconnected`. **À vérifier en V2** : ces deux derniers sont écoutés dans
`network.js` mais aucun `emit("network_source_connected")` n'existe dans le Rust — soit du code mort,
soit une régression silencieuse de V1.

## Conventions de sécurité IPC

- `write_metadata` canonicalise le path et vérifie `starts_with()` sur `library_paths` (pas d'écriture
  hors bibliothèque).
- `escapeHtml()` sur **toute** donnée affichée (y compris les guillemets — corrigé en commit `bfb2bcf`).
- `save_file_secure()` → permissions `0600` sur les fichiers de config.
- Les `println!` Rust sont sous `#[cfg(debug_assertions)]`.
- CSP stricte dans `tauri.conf.json`, `assetProtocol` désactivé.
- Capability unique `default` : `core:default`, `core:window:allow-start-dragging`,
  `global-shortcut:*`, `updater:default`. Rien d'autre n'est autorisé.
