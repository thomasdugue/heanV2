---
name: hean-smb
description: Sous-système NAS/SMB de Hean — pavao/libsmbclient singleton, URI smb://, téléchargement progressif, SmbProgressiveFile, gapless SMB, Keychain/password cache, discovery mDNS, scan différentiel. Charger avant de toucher src-tauri/src/network/, audio_play/audio_preload_next côté SMB, ou network.js.
---

> ⚠️ **Mi-V1, mi-V2 depuis le 2026-11-25.** La partie **lecture et inventaire** existe
> maintenant dans V2 et est décrite en tête de ce fichier. Le reste — téléchargement
> progressif, `SmbProgressiveFile`, gapless SMB, pochettes, découverte mDNS — est encore du
> savoir V1 : les chemins `noir-tauri/src/…` sont ceux de V1 et n'existent pas dans ce dépôt.
> Référence complète : [`bootstrap/04-invariants/README.md`](../../../bootstrap/04-invariants/README.md).

## Ce qui existe dans V2 (ADR 0016)

```
src-tauri/src/network/
├── mod.rs           scanner_une_source() — la forme en TROIS TEMPS (voir ci-dessous)
├── identifiants.rs  Keychain : com.tdugue.hean.smb SEUL — ADR 0015 tranché (option B)
├── client.rs        libsmbclient via pavao, UN mutex de process, reessayer_une_fois (INV-03)
├── uri.rs           smb://{id}/{partage}/{chemin} — format hérité de V1, épinglé par test
├── entete.rs        1 Mio d'en-tête pour FLAC/WAV/AIFF, fichier entier pour MP3/M4A/OGG
├── inventaire.rs    parcours différentiel, sans verrou de bibliothèque
└── reprise_v1.rs    les sources de network_sources.json, identifiants CONSERVÉS
```

**La forme en trois temps d'un scan, et pourquoi elle n'est pas négociable.** Un scan dure des
minutes et `libsmbclient` est un singleton de process. Donc : (1) verrou **court** sur le dépôt
pour lire la source et les empreintes connues ; (2) parcours SMB et lecture des tags **sans
aucun verrou de bibliothèque** ; (3) verrou **court** pour appliquer en une transaction. Tenir
le verrou pendant le parcours bloquerait `list_tracks` et gèlerait la fenêtre — W005 par la cause.

**INV-27 — un scan n'efface que dans son périmètre.** `library::scan::Perimetre` n'a pas de
variante « tout ». Sans ça, le premier scan de disque effaçait les 3 137 pistes du NAS. Et
`scan::Conclusion::Partielle` interdit toute disparition quand un dossier n'a pas pu être lu :
un dossier refusé n'est pas un dossier vide.

**Le scan différentiel d'une source distante.** `pavao::list_dirplus` (readdirplus SMB2) rend
taille et date **par dossier**, donc sans payer un `stat` par fichier ; quand le serveur ne
répond pas, le repli ne rend que des noms et une piste déjà connue n'est pas relue. V1 était
toujours dans ce second cas : ses 3 137 entrées de cache portent `size: 0, modified: 0`.

**Ce qui n'existe PAS encore dans V2** : lecture audio depuis le NAS, pochettes SMB, découverte
mDNS. Le backend de sortie CoreAudio n'existe pas, donc du streaming ne serait démontrable par
rien. Tout ce qui suit décrit V1.


# hean-smb — NAS, SMB & streaming réseau

## Network/NAS subsystem (`src-tauri/src/network/`)

```
network/
├── mod.rs          — NetworkSource, SmbCredentials types; load/save network_sources.json
├── smb.rs          — SMB client (pavao wrapper); CONNECTION mutex; browse/read_file/read_file_head
├── scanner.rs      — Differential scan; start_progressive_download; extract_smb_metadata_and_cover
├── credentials.rs  — macOS Keychain + PASSWORD_CACHE (session-level in-memory cache)
└── discovery.rs    — mDNS/Bonjour discovery (_smb._tcp.local., 5s timeout)
```

**SMB URI format:** `smb://{source_id}/{share}/{remote_path}`
- `source_id` = UUID of `NetworkSource` — used to look up host + credentials
- `remote_path` starts with `/`

**Progressive download (`scanner.rs` + `audio_decoder.rs`):**
- `start_progressive_download(source, share, remote_path, cancel_previous: bool)` — spawns OS thread that downloads in 64KB chunks, writes to `smb_buffer/{hash}.tmp`, updates `PROGRESSIVE_DOWNLOADS` registry
- `cancel_previous = true` for `audio_play` (cancel old download), `false` for `audio_preload_next` (don't interrupt current track)
- `audio_play` waits for 4MB threshold (≈111ms at 36 MB/s) before starting the engine, with 15s timeout
- `SmbProgressiveFile` (in `audio_decoder.rs`) implements `Read + Seek + MediaSource` with blocking wait loops
- **`byte_len()` must return `Some(bytes_written)` at all times** — returning `None` makes Symphonia treat the stream as non-seekable

**`add_network_source` returns a `NetworkSource` object** (not a string ID). Callers must use `result.id` to get the UUID for subsequent commands like `scan_network_source_cmd`. See `network.js` line 876 for the canonical pattern: `invoke('scan_network_source_cmd', { sourceId: result.id })`.

**Critical SMB constraint:** `libsmbclient` (via pavao) is a **process-level singleton**. Two concurrent `SmbClient` instances cause EINVAL (os error 22). All SMB operations share a single `CONNECTION: Lazy<Mutex<Option<ActiveConnection>>>` — concurrent access is serialized through this lock.

**Gapless with SMB:** `audio_preload_next` is `async` and SMB-aware — it parses `smb://`, calls `start_progressive_download(cancel_previous=false)`, waits 4MB, then calls `engine.preload_next(temp_path)`. Triggered at 60s remaining (vs. 10s for local files) to cover NAS latency.

**Gapless transition + seek:** `CallbackData` in `coreaudio_stream.rs` carries `current_path: Arc<Mutex<Option<String>>>` + `next_path: Arc<Mutex<Option<String>>>`. During gapless transition: `*data.current_path.lock() = data.next_path.lock().take()`. Without this, seek-restart after gapless opens the wrong file.

**Password cache:** `credentials.rs` keeps `PASSWORD_CACHE: Lazy<Mutex<HashMap<String,String>>>` — Keychain accessed at most once per source per session. `get_cover_smb()` checks this cache and skips the SMB connection if no session password is present (avoids Keychain dialogs at startup).

## DAP Sync — feature archivée

La feature DAP Sync (synchronisation vers Digital Audio Player via Mass Storage / MTP / SMB) a été retirée du build actif le 2026-04-22 à cause de bugs d'instabilité non résolus. Le code est conservé sur la branche `archive/dap-sync-v1` et le tag `dap-sync-archive-2026-04`. Voir [`docs/archive/dap-sync/`](../../../bootstrap/05-postmortem/archive-dap-sync/) pour le rapport complet (architecture, décisions, bugs connus, procédure de réintégration).

**Reste encore actif** : `network/smb_utils.rs` (résolution `smb://` → chemin local), utilisé par le streaming audio SMB.
