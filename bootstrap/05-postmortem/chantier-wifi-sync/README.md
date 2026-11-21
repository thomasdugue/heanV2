# Chantier repris : WiFi Sync v3

> **Ce n'est pas une feature de parité.** WiFi Sync n'a jamais été distribué : il ne figure donc pas
> dans les 124 lignes de [`../../01-produit/FEATURE-SET.md`](../../01-produit/FEATURE-SET.md).
> C'est un **chantier à reprendre**, au même titre que
> [`../archive-dap-sync/`](../archive-dap-sync/README.md) — à la différence près que celui-ci
> **fonctionnait**.

## Pourquoi ce dossier existe

Le code a vécu **sept mois dans un répertoire de travail**, sans jamais être commité : pas de
branche, pas de stash, absent du clone de sauvegarde. Mis à l'abri le 2026-11-21 sur
[`archive/wifi-sync-v3`](https://github.com/thomasdugue/noirdesktop/tree/archive/wifi-sync-v3)
(commit `b8c1019`), dans `thomasdugue/noirdesktop`.

## Ce que c'est

Le successeur de la sync MTP / Mass Storage, retirée de la beta le 2026-04-22. Le DAP n'est plus
branché en USB : il se connecte au Mac **avec son navigateur**, sur le même WiFi.

Le Mac lance un serveur HTTPS, affiche un QR code, et le DAP télécharge les albums via la File
System Access API de son navigateur Chromium.

| Fichier | Lignes | Rôle |
|---|---:|---|
| [`code/rust/wifi_sync/webui.rs`](code/rust/wifi_sync/webui.rs) | 2 664 | La WebUI servie au navigateur du DAP |
| [`code/rust/wifi_sync/mod.rs`](code/rust/wifi_sync/mod.rs) | 1 255 | État, sélection d'albums, débit, token, clés d'album |
| [`code/rust/wifi_sync/server.rs`](code/rust/wifi_sync/server.rs) | 1 121 | Serveur axum, routes, auth, mDNS |
| [`code/rust/wifi_sync/zip_stream.rs`](code/rust/wifi_sync/zip_stream.rs) | 354 | Un ZIP par album, `transfer-encoding: chunked` |
| [`code/rust/wifi_sync/db.rs`](code/rust/wifi_sync/db.rs) | 197 | Ce qui est déjà synchronisé, par appareil |
| [`code/rust/wifi_sync/tls.rs`](code/rust/wifi_sync/tls.rs) | 86 | Certificat auto-signé généré au premier lancement |

**5 677 lignes de Rust**, plus le câblage dans [`integration/`](integration/).

### Surface HTTP

`/` (la WebUI) · `/ping` · `/status` · `/album/:album_id` · `/file` · `/sync-state` · `/sync-mark` ·
`/throughput` · `/font/geist-mono.woff2` · `/debug/log`

Port **8443** en HTTPS (443 demanderait root). `/ping`, `/debug/log` et `/font/…` sont publics ; tout
le reste exige le token.

### Les 7 commandes Tauri

`start_wifi_sync_server` · `stop_wifi_sync_server` · `get_wifi_sync_status` ·
`set_wifi_sync_root_prefix` · `wifi_sync_rotate_token` · `wifi_sync_reset_access_code` ·
`wifi_sync_reset_device`

## Ce qui est déjà résolu — et vaut d'être repris tel quel

C'est là l'intérêt du dossier : ces problèmes ont coûté du temps et sont réglés.

- **Token persistant.** Stocké sur disque, donc un onglet ouvert sur le DAP survit à un
  stop/start du serveur et à un redémarrage de l'app. C'était le bug principal de la v2.1.
- **HTTP Range / 206 Partial Content** sur `/file` — reprise d'un téléchargement interrompu, décrit
  dans le `Cargo.toml` comme un correctif *« self-healing contre l'InvalidStateError de la File
  System Access API de Chromium en milieu d'album »*.
- **Déduplication exFAT.** Un test couvre la collision de casse sur les cartes SD en exFAT.
- **Artiste canonique.** `featuring`, `ft.`, `with` sont retirés, et la majorité l'emporte en cas de
  désaccord entre pistes d'un même album (test sur *GNX* de Kendrick Lamar).
- **Préfixe de dossier configurable** (`HEAN` par défaut), embarqué dans chaque ZIP.
- **mDNS**, pour éviter de taper l'IP — avec repli sur l'IP affichée, parce que `hean.local` ne
  marchait pas de façon fiable.

## 30 tests

Répartis : `mod.rs` 13 · `server.rs` 12 · `db.rs` 3 · `zip_stream.rs` 2. Ils couvrent la frontière
d'authentification (`protected_route_without_token_is_forbidden`,
`ping_token_hash_differs_per_token`), les codes 400 sur paramètres manquants, et les cas tordus de
noms d'albums. `tls.rs` et `webui.rs` n'en ont aucun.

## État réel, et ce qui reste

Le protocole de test [`TEST-PROTOCOL-V2.2.md`](TEST-PROTOCOL-V2.2.md) porte **les réponses écrites à
la main par Thomas** — c'est la seule preuve directe que ça tournait. Deux frictions y sont notées :

1. **L'avertissement de certificat** sur le DAP : *« je dois cliquer sur accéder à ce site
   (dangereux) »*. Inhérent à un certificat auto-signé sur un réseau local. À traiter comme un point
   d'UX (l'expliquer dans le tutoriel), pas comme un bug.
2. **La vérification du token persistant** : *« je ne sais pas comment faire »* — le protocole
   demandait un `cat` dans un terminal. Le statut doit être lisible **dans l'app**.

Et une question de Thomas restée sans réponse : *« à quoi sert rotate access token ? »* — si
l'utilisateur ne comprend pas le bouton, il ne devrait pas être là, ou pas là.

[`DESIGN-PLAN-V3.md`](DESIGN-PLAN-V3.md) décrit l'étape suivante, **jamais implémentée** : faire de
WiFi Sync le citoyen de première classe de l'onglet App Sync, avec deux écrans à ajouter
(`wifi-welcome` quand le serveur est éteint, `wifi-tutorial` à la première connexion) et la
suppression de MTP / Mass Storage. C'est une spécification complète, wireframes inclus : à lire comme
le cahier des charges de la feature en V2.

## Dépendances que ça ajoute

`axum` 0.7 · `axum-extra` · `axum-range` · `axum-server` + `rustls` · `rcgen` · `async_zip` ·
`qrcode` · `rand` · `async-stream` · `tokio-util` · `tower` + `http-body-util` (tests).

Détail et commentaires d'origine : [`integration/Cargo.toml.diff`](integration/Cargo.toml.diff).

> ⚠️ **Rien de tout ça n'a été compilé depuis avril 2026.** Les versions sont à revérifier, et le
> `cargo check` est le premier geste de toute reprise.

## Également sauvé au passage

[`integration/smb-retry.diff`](integration/smb-retry.diff) — une boucle de retry (2 tentatives)
contre les connexions SMB zombies : l'hôte a oublié la connexion, mais libsmbclient garde encore le
handle. **Indépendant de WiFi Sync**, et utile en soi. À comparer avec l'invariant SMB de
[`../../04-invariants/INVARIANTS.md`](../../04-invariants/INVARIANTS.md), qui exige déjà ce retry :
ce diff en est une version plus robuste.
