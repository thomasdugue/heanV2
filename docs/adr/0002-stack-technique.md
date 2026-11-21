# 0002 — Stack technique de heanV2

- **Date** : 2026-11-21
- **État** : Acceptée
- **Décideur** : Thomas
- **Suit** : [0001](0001-reecrire-hean-de-zero.md)

## Contexte

L'[ADR 0001](0001-reecrire-hean-de-zero.md) acte la réécriture. Quatre décisions de stack en
découlaient, chacune visant une faiblesse mesurée de V1.

Trois choix n'étaient pas en discussion : **Tauri v2**, **Rust**, **CoreAudio direct** via
`coreaudio-rs` + `coreaudio-sys` (jamais CPAL — il ne donnait pas accès au buffer interne : seek
cassé, pas d'`AudioUnitReset`, pas de contrôle de la taille de buffer).

## Décisions

### TypeScript strict + Vite

V1 : 16 076 lignes de JavaScript, aucune analyse statique à part `node --check` (syntaxe). Les bugs
les plus coûteux étaient des erreurs de forme : `data.mp3_count` au lieu de `data.stats.mp3_count`,
`device.name` au lieu de `device.display_name`, `.size` sur un Array (V014), un `add_network_source`
qui renvoie un objet là où le code lisait une string. Une seule session (2026-03-09) a été consommée
par neuf bugs de ce type.

**Écarté** : rester en JS vanilla. Zéro build, mais cela rend `tauri-specta` inutile (des types
générés que rien ne vérifie), laisse les 3 suites de tests skippées, et rend le refactoring risqué —
donc on ne refactore pas, donc la dette revient.

**Conséquence acceptée** : il y a désormais une étape de build. En pratique le HMR de Vite rend la
boucle de développement **plus rapide** que celle de V1, qui rechargeait toute la page.

### Svelte 5 (runes)

Cinq régressions de V1 appartiennent à une seule famille : la manipulation manuelle du DOM. Covers
chargées en async dans des cellules déconnectées par un rebuild concurrent, contrainte `isConnected`
sur chaque `<img>`, rebuild planifié via `requestAnimationFrame` avec un garde
`_thumbsRefreshScheduled` contre la boucle infinie. Un modèle de composants réactif supprime la
famille entière.

Svelte a été retenu pour quatre raisons : compilé (pas de VDOM, bon pour 10 000 lignes de liste à
60 fps), les runes remplacent proprement l'état partagé par référence de V1, la syntaxe est assez
proche du HTML/CSS existant pour que le portage d'`index.html` et de `styles.css` soit direct, et le
CSS scopé par composant attaque le fichier de 10 654 lignes.

**Écartés** : **Solid** (mêmes bénéfices, réactivité plus fine, mais écosystème nettement plus petit
donc moins de matière connue d'un modèle) · **Lit** (la gestion d'état reste à la charge du
développeur — on réintroduit le problème de V1 — et le Shadow DOM frotte avec un design system global
à tokens, glassmorphism et watermarks traversants) · **vanilla + Vite + TS** (on réécrit un modèle de
composants à la main, précisément là où V1 a saigné).

**Conséquence acceptée** : Svelte 5 est récent et une grande partie des exemples en ligne est encore
en Svelte 4. Le `CLAUDE.md` impose explicitement les runes et interdit de mélanger les deux syntaxes.

### tauri-specta — IPC typée générée

V1 : 86 commandes et 17 events, tous appelés par des chaînes de caractères. Le nom de la commande,
celui du paramètre et la forme du payload n'étaient vérifiés nulle part.

Avec des types générés depuis le Rust, renommer une commande ou changer un payload casse la
compilation du frontend. Effet de bord décisif : le client IPC devient **substituable**, donc
`invoke` est mockable — les 3 suites de tests que V1 avait dû skipper (`Navigation`,
`PlayerControls`, `AlbumView`) redeviennent testables.

**Écarté** : des chaînes à la main comme en V1.

### SQLite (`rusqlite`, feature `bundled`)

V1 maintenait sept caches globaux (`METADATA_CACHE`, `TRACKS_CACHE`, `COVER_CACHE`,
`INTERNET_NOT_FOUND_CACHE`, `ADDED_DATES_CACHE`, `LISTENING_HISTORY`, `CACHE_DIRTY`), chacun persisté
dans son propre JSON, modifiés concurremment par le scan de fond, les écritures lofty et les
commandes IPC — sans transaction.

Quatre invariants classés BLOQUANTS décrivent le même problème. Le plus grave (invariant 8) :
supprimer une entrée de `METADATA_CACHE` au lieu de la mettre à jour créait une race fatale — le scan
concurrent relisait le fichier audio pas encore écrit par lofty, réinsérait les anciennes métadonnées,
et **cette corruption persistait indéfiniment**. L'invariant 9 ajoute que le snapshot doit être pris
avant le lock de `TRACKS_CACHE` « pour éviter le deadlock ».

Un seul écrivain et des transactions font disparaître les invariants 7, 8, 9 et V015 par
construction. Bénéfice secondaire : tri, filtre, recherche et statistiques passent du JavaScript à
des requêtes indexées, et le démarrage instantané est plus rapide qu'en parsant plusieurs Mo de JSON.

**Écartés** : un seul fichier JSON (inspectable à la main, mais sans transaction : un fichier à
moitié cohérent reste possible) · les 6 JSON de V1 (statu quo, les 4 invariants bloquants restent).

**Conséquences acceptées** : il faut concevoir un schéma et des migrations versionnées ; les données
sont moins inspectables à la main (`sqlite3` au lieu de `cat`) ; `bundled` ajoute ~1 Mo au binaire,
négligeable face aux ~92 dylibs Samba déjà embarquées.

## Décision reportée

**La typographie.** `index.html` de V1 charge DM Sans depuis Google Fonts, mais la CSP est
`style-src 'self' 'unsafe-inline'` sans `font-src` : la feuille devrait être bloquée, et l'app
tournerait donc déjà en Geist Mono + San Francisco. **Non vérifié visuellement.** La décision — Geist
Mono partout (parité garantie) ou Geist Mono + DM Sans embarquée pour les grands titres (l'intention
du design system, jamais réellement appliquée) — attend cette vérification. Rien en phase 0 n'en
dépend.
