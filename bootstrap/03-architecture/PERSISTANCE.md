# Persistance V1 — où vivent les données, et pourquoi ça compte pour V2

Vérifié dans le code (`get_data_dir()`, `lib.rs:317`) le 2026-11-21.

## Le chemin réel

```rust
pub(crate) fn get_data_dir() -> PathBuf {
    let home = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("noir")
}
```

Sur macOS, `dirs::data_dir()` = `~/Library/Application Support`. Donc **le dossier réel est** :

```
~/Library/Application Support/noir/
```

> ⚠️ **Trois chemins différents sont documentés dans V1, deux sont faux.**
> - `noir-tauri/CLAUDE.md` et le commentaire d'en-tête de `logging.rs` disent
>   `~/.local/share/noir/` → c'est le chemin **Linux** de `dirs::data_dir()`. Faux sur macOS.
> - Le skill `hean-frontend` dit `~/.noir_desktop/thumbnails/` et
>   `~/.noir_desktop/covers/internet_*.jpg` → ancien chemin, plus utilisé.
> - `hean-ops` dit `~/Library/Application Support/noir/feedback/` → **celui-là est juste**.
>
> À corriger dans la doc de V2 : un seul chemin, écrit une fois.

## Contenu

| Fichier / dossier | Contenu | Remarque pour V2 |
|---|---|---|
| `config.json` | `library_paths`, **`excluded_paths`**, device audio, `sentry_enabled` | `excluded_paths` porte l'invariant 6 (suppression permanente) |
| `tracks_cache.json` | liste complète des pistes | lu au boot → démarrage instantané. Peut peser plusieurs Mo |
| `metadata_cache.json` | métadonnées par path | **source des invariants 7-9** (race avec le scan de fond) |
| `cover_cache.json` | chemins des covers extraites | |
| `internet_not_found_cache.json` | couples artiste/album sans cover en ligne, TTL 30 jours | évite de re-taper MusicBrainz |
| `added_dates_cache.json` | dates d'import | alimenté aussi par le scan NAS (fix du 2026-03-15) |
| `playlists.json` | playlists utilisateur + favoris | |
| `listening_history.json` | 1 000 dernières écoutes | alimente Recently Played / Top Artists |
| `eq_settings.json` | état et bandes de l'EQ | la spec parlait d'un `settings.json` : il n'existe pas |
| `network_sources.json` | sources NAS — **sans mot de passe** | les mots de passe vont au Keychain macOS |
| `network_scan_cache.json` | état du scan différentiel NAS | |
| `covers/` | pochettes extraites (JPEG/PNG) + `internet_{md5(artist\|\|\|album)}.jpg` + `artist_{hash}.jpg` | servi via `noir://` |
| `thumbnails/` | miniatures 80×80 `{hash}_thumb.jpg` | générées au scan |
| `smb_buffer/` | `.tmp` du progressive download SMB | nettoyé au lancement (commit `eaee676`) |
| `feedback/` | feedbacks sauvés en local (toujours, même si le POST réussit) | |
| `logs/` | `noir-YYYY-MM-DD.log`, rotation journalière, 7 jours | lu par « Attach recent logs » |
| `dap_sync.db` | SQLite de la feature archivée | laissé en place volontairement pour préserver les configs des beta-testeurs |

**Hors de ce dossier** : les mots de passe SMB (Keychain macOS, via `security-framework`), la largeur
de sidebar et les raccourcis personnalisés (`localStorage` de la WebView), ainsi que les caches de
sélection de carousels (`localStorage`).

## Les trois questions à trancher avant de coder V2

### 1. On garde `noir/` ou on passe à `hean/` ?

Le produit s'appelle Hean depuis le printemps 2026 mais le dossier de données s'appelle toujours
`noir`. Si V2 écrit dans `~/Library/Application Support/hean/`, **la bibliothèque, les playlists,
l'historique et les sources NAS restent derrière** — sauf migration explicite. Écrit à l'époque pour
« les beta-testeurs » : il n'y en a jamais eu, et la seule bibliothèque en jeu est celle de Thomas.

Options : (a) garder `noir/` (compat gratuite, nom incohérent), (b) passer à `hean/` avec une
migration one-shot au premier lancement, (c) passer à `hean/` sans migration et l'assumer.
**Mon avis : (b)**, la migration est une copie de fichiers et un rename, et
c'est le bon moment pour le faire.

Le même arbitrage vaut pour le protocole `noir://`, l'identifiant de bundle (`com.tdugue.hean` est
déjà à jour dans `tauri.conf.json`) et les variables `NOIR_WORKER_*`.

### 2. JSON ou SQLite ?

V1 maintient **6 fichiers JSON de cache** qui doivent rester cohérents entre eux pendant qu'un scan
tourne en arrière-plan et qu'une écriture lofty se produit. C'est l'origine directe de quatre
invariants classés BLOQUANTS :

- invariant 7 — les métadonnées éditées doivent aller dans le fichier audio ;
- invariant 8 — `METADATA_CACHE` doit être **mis à jour**, jamais `remove()` (le `remove` créait une
  corruption permanente : le scan concurrent relisait le fichier pas encore écrit par lofty et
  réinsérait les anciennes valeurs) ;
- invariant 9 — `start_background_scan` doit ré-appliquer un snapshot de `METADATA_CACHE` pris
  **avant** le lock de `TRACKS_CACHE` (sinon deadlock) ;
- V015 — `write_metadata` doit toucher `METADATA_CACHE` **et** `TRACKS_CACHE`.

Ces quatre règles décrivent le même problème : *plusieurs copies de la même donnée, plusieurs
écrivains, pas de transaction*. Une base SQLite avec un seul écrivain et des transactions les fait
disparaître par construction. C'est, à mon sens, le gain structurel le plus net de V2 — voir
[`05-postmortem/DETTE-TECHNIQUE-V1.md`](../05-postmortem/DETTE-TECHNIQUE-V1.md).

Contre-argument honnête : le JSON est trivial à inspecter et à réparer à la main, et
`load_tracks_from_cache` au boot est déjà rapide. Si V2 reste en JSON, alors il faut au minimum un
**seul** fichier d'état et un **seul** écrivain.

### 3. Qui possède `excluded_paths` ?

En V1, la règle « une piste supprimée ne revient jamais » est appliquée par **défense en
profondeur** : le filtre `excluded_paths` est répété dans `init_cache()`,
`start_background_scan()` et `scan_network_source_cmd()`, et l'invariant dit « et dans tout nouveau
point d'entrée ». Un invariant formulé comme « n'oubliez pas de répéter ce filtre partout » est un
invariant qui sera violé un jour.

En V2 : un seul point d'entrée pour charger des pistes (un repository / gateway), le filtre dedans,
et un test qui vérifie qu'aucune autre fonction ne construit la liste de pistes.
