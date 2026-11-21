# DAP Sync — le code complet, conservé

> Ajouté le 2026-11-21 à la demande de Thomas : **le travail ne doit pas être perdu**, même si la
> feature n'a jamais fonctionné de façon fiable.
>
> Les 7 markdown à côté de ce fichier (`OVERVIEW`, `DECISIONS`, `KNOWN_ISSUES`, `REINTEGRATION`,
> `TEST_PROTOCOL`, `TEST_EXECUTION_LOG`, `README`) étaient déjà dans le dépôt. **Le code, non** : il
> ne vivait que sur la branche `archive/dap-sync-v1` et le tag `dap-sync-archive-2026-04`. Il est
> maintenant ici, intégralement.

## Ce que ça représente

**~13 500 lignes retirées** par le commit `47acae0` (2026-04-22), soit près de la moitié de la taille
de l'app actuelle.

| Fichier | Lignes | Rôle |
|---|---:|---|
| `code/rust/dap_sync/sync_engine.rs` | 2 368 | moteur de synchronisation, vagues, reprise, replug |
| `code/rust/dap_sync/mtp.rs` | 1 834 | protocole MTP (le cœur du problème) |
| `code/rust/dap_sync/sync_plan.rs` | 1 733 | calcul du plan de sync (diff local ↔ destination) |
| `code/rust/dap_sync/db.rs` | 273 | SQLite : destinations + sélections |
| `code/rust/dap_sync/volumes.rs` | 215 | volumes externes macOS |
| `code/rust/dap_sync/smb_utils.rs` | 200 | résolution `smb://` (**la seule partie encore active** — déplacée dans `network/`) |
| `code/rust/dap_sync/manifest.rs` | 198 | manifeste de sync (reprise incrémentale) |
| `code/rust/dap_sync/watcher.rs` | 138 | détection montage/démontage (notify + fsevent) |
| `code/rust/dap_sync/mod.rs` | 14 | |
| **Rust total** | **6 973** | |
| `code/js/dap-sync.js` | 2 957 | toute l'UI : sélection d'albums, progression, écran de fin |
| `code/css/dap-sync.css` | 2 482 | 328 sélecteurs, extraits des lignes supprimées de `styles.css` |
| `code/docs/TEST-MTP-SYNC.md` + `TEST_MTP_SYNC-racine.md` | 331 | protocoles de test MTP |

Plus le câblage, dans `integration/extraits/` : **596 lignes de `lib.rs`** (22 commandes Tauri),
35 lignes de markup, 105 lignes de `panels.js`, 26 de `renderer.js`, 12 de `views.js`, 11 slots de
médiateur.

### Les 22 commandes Tauri

`dap_list_external_volumes` · `dap_get_volume_info` · `dap_eject_volume` ·
`dap_start_volume_watcher` · `dap_save_destination` · `dap_get_destination` ·
`dap_get_destinations` · `dap_delete_destination` · `dap_save_selection` · `dap_get_selections` ·
`dap_save_selections_batch` · `dap_compute_sync_plan` · `dap_execute_sync` · `dap_cancel_sync` ·
`dap_read_manifest` · `dap_detect_mtp_devices` · `dap_mtp_test` · `dap_mtp_list_files` ·
`dap_mtp_send_file` · `dap_mtp_purge_albums` · `dap_mtp_purge_ghosts` · `dap_execute_mtp_sync`

### Les 9 dépendances Rust retirées

`integration/Cargo.toml.avec-dap` contient le fichier complet de l'époque. Les deps spécifiques :

```toml
mtp-rs = { git = "https://github.com/vdavid/mtp-rs.git", branch = "main" }  # ⚠️ git, pas crates.io
futures = "0.3"
bytes = "1.5"
rusqlite = { version = "0.31", features = ["bundled"] }
notify = { version = "6", features = ["macos_fsevent"] }
sha2 = "0.10"                  # quick hash de détection de modification
unicode-normalization = "0.1"  # exFAT exige NFC, macOS fournit souvent NFD
deunicode = "1"                # le driver exFAT de macOS corrompt les dossiers accentués (ô, ã, é)
tempfile = "3"                 # dev-dependency, tests du sync engine
```

> ⚠️ `mtp-rs` est tiré d'un **dépôt git tiers sur la branche `main`**, pas d'une version figée sur
> crates.io. Si ce dépôt disparaît ou change, le code ne compile plus. **Si tu veux vraiment
> sécuriser ce travail, forke `vdavid/mtp-rs` sur ton compte** — c'est la dépendance la plus fragile
> de tout l'édifice.

## Pourquoi ça n'a pas marché

Résumé ; le détail est dans [`KNOWN_ISSUES.md`](KNOWN_ISSUES.md) et
[`DECISIONS.md`](DECISIONS.md), qui sont à lire avant toute reprise.

Le MTP, tel qu'implémenté par les firmwares de baladeurs, se dégrade après un certain nombre
d'opérations dans une même session : *Transaction ID mismatch*, timeouts, `InvalidObjectHandle`. La
parade était un découpage en « vagues » (`WAVE_SIZE`, réduit de 250 à 200 par marge de sécurité) avec
une détection de débranchement/rebranchement **à l'intérieur** de la synchronisation — ~200 lignes de
Rust, un `MTP_LOCK` tenu jusqu'à 5 minutes, et un chemin de replug d'urgence. Ça marchait, mais c'était
complexe et jamais stabilisé sur du vrai matériel.

Les 5 pistes d'amélioration identifiées lors de la revue d'ingénierie du 2026-04-03 étaient dans le
`TODOS.md` du dépôt (que je n'avais pas repris : il ne parlait que de ça). Les voici, conservées :

1. **Sync piloté par le frontend** — remplacer le découpage interne par N syncs de 250 fichiers
   déclenchés depuis le JS, avec une modale de rebranchement entre chaque. Supprime ~200 lignes de
   Rust, la détention longue du `MTP_LOCK` et le chemin d'urgence. Le manifeste gère déjà la reprise
   incrémentale. **C'est la piste la plus prometteuse.**
2. **DRY sur la détection de replug** — extraire `wait_for_device_unplug()` /
   `wait_for_device_replug()`, dupliqués (~80 lignes).
3. **Compteur d'opérations MTP réel** — compter toutes les opérations (ouverture/fermeture de session,
   uploads échoués, suppressions) et pas seulement les copies réussies. Le cyclage de session ajoute
   ~2 opérations par cycle, ~50 par vague : la marge de sécurité est plus mince que `WAVE_SIZE` ne le
   suggère.
4. **Cohérence des délais de retry** — le retry inline est à 500 ms contre 1 500 ms
   (`INTER_FILE_DELAY_MS`) pour les uploads normaux, soit 3× plus agressif alors que tout le design
   suppose que le firmware a besoin de respirer.
5. **`ptpcamerad`** — vérifier si le tuer pendant l'attente de rebranchement n'empêche pas macOS
   d'initialiser le device MTP (c'est `ptpcamerad` qui s'en charge). L'attente de 5 s après replug
   couvre peut-être déjà le problème.

Et deux pièges exFAT, qui eux sont **résolus** et valent d'être retenus : le driver exFAT de macOS
corrompt les noms de dossiers accentués (d'où `deunicode`), et exFAT exige du NFC alors que les
métadonnées macOS fournissent souvent du NFD (d'où `unicode-normalization`).

## Comment ressusciter ce code

`integration/retrait-47acae0.patch` est le diff de retrait complet sur les 10 fichiers de câblage
(3 701 lignes). Il est **réversible en théorie** (`git apply --reverse`) mais **ne s'applique plus
sur le code actuel** — vérifié : `lib.rs` et `Cargo.toml` ont trop changé en 7 mois. Considère-le
comme la **carte des points d'intégration**, pas comme un bouton de restauration.

Pour V2, la bonne approche est de traiter DAP Sync comme une **feature à part entière, portée après
la phase 8**, et non comme un patch à réappliquer :

1. Lire `OVERVIEW.md` (architecture), `DECISIONS.md` (ce qui a été tranché et pourquoi),
   `KNOWN_ISSUES.md` (ce qui casse) — dans cet ordre.
2. Reprendre d'abord les parties **saines et indépendantes du MTP** : `volumes.rs`, `watcher.rs`,
   `db.rs`, `manifest.rs`, `sync_plan.rs` — soit 2 557 lignes qui marchent. La synchronisation vers
   une carte SD en Mass Storage n'avait pas les problèmes du MTP.
3. Ne reprendre `mtp.rs` + la partie vagues de `sync_engine.rs` qu'ensuite, en appliquant la piste 1
   ci-dessus (sync piloté par le frontend) **avant** de retoucher quoi que ce soit d'autre.
4. Conserver `REINTEGRATION.md` comme référence de procédure, en sachant qu'elle décrit la
   réintégration dans V1.

Et une précaution déjà prise en V1, à maintenir : les tables SQLite `dap_destinations` et
`dap_sync_selection` ont été **laissées en place** dans la base des utilisateurs pour préserver les
configurations des beta-testeurs. Si V2 change de dossier de données, il faut les migrer aussi.

## Les branches git à ne pas supprimer

Même avec ce dossier, garde ces références — elles contiennent l'historique des commits, que ce
dossier n'a pas :

- `archive/dap-sync-v1` (locale **et** sur `origin`) → `c901ba2`
- tag `dap-sync-archive-2026-04` → `c901ba2`
- `feature/dap-sync-mtp`, `claude/dap-sync-investigation-fiAUf`,
  `claude/dap-sync-coherence-ui-polish` → les branches de travail, avec le détail des tentatives
