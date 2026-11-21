# Le savoir dur — ce qui ne se redécouvre pas

**C'est le dossier le plus important du bootstrap.** Si V2 ne reprend qu'une chose de V1, c'est ça.

Le reste (le design, le feature set, l'architecture) se reconstruit en lisant le code. Ce qui est ici
a été payé en sessions de debug sur des APIs qui mentent, des bugs WebKit, et des outils Apple qui
échouent en silence. Ça ne se déduit d'aucune documentation officielle.

## Comment lire ces fichiers

| Fichier | Contenu | À lire |
|---|---|---|
| [`INVARIANTS.md`](INVARIANTS.md) | Les 19 règles « ne jamais casser », classées par gravité (🔴 BLOQUANT / 🟡 AVERTISSEMENT), + les décisions techniques actées | **en entier, avant d'écrire une ligne de V2** |
| [`SAVOIR-AUDIO.md`](SAVOIR-AUDIO.md) | Pipeline CoreAudio, bit-perfect, hog mode, sample rate switching, **et toute la stratégie AirPlay** | avant de toucher à l'audio |
| [`SAVOIR-SMB.md`](SAVOIR-SMB.md) | Singleton libsmbclient, progressive download, `SmbProgressiveFile`, gapless SMB, Keychain | avant de toucher au NAS |
| [`SAVOIR-FRONTEND.md`](SAVOIR-FRONTEND.md) | Pipeline covers 4 niveaux, contrainte `isConnected`, `min-width: 0`, mosaïques playlist | avant de toucher aux vues. ⚠️ Ses chemins de thumbnails (`~/.noir_desktop/`) et sa palette sont faux : voir [`../02-design/ECARTS-DESIGN.md`](../02-design/ECARTS-DESIGN.md) et [`../03-architecture/PERSISTANCE.md`](../03-architecture/PERSISTANCE.md) |
| [`SAVOIR-OPS.md`](SAVOIR-OPS.md) | Feedback via worker Cloudflare, Sentry + anonymisation + toggle RGPD, logs, dashboard dev | avant de toucher à l'observabilité |
| [`SAVOIR-RELEASE.md`](SAVOIR-RELEASE.md) | Pipeline signature + notarisation + dylibbundler + updater, en 8 étapes | avant de builder une release |

> ⚠️ `SAVOIR-RELEASE.md` contient **deux versions** du pipeline. La section du haut (« Release
> pipeline — Apple signing + notarisation + updater ») est la bonne. Les « Étapes 0 à 5 » plus bas
> datent d'avant le worker Cloudflare et mentionnent `NOIR_GITHUB_FEEDBACK_TOKEN`, une variable qui
> n'existe plus. L'encadré en tête du fichier le signale ; j'ai laissé le tout intact pour ne rien
> perdre, mais en V2 il faut n'en garder qu'une.

## Les 7 pièges de plateforme — le noyau irremplaçable

Si vous ne retenez que sept choses :

1. **AirPlay n'existe dans CoreAudio que tant qu'il est le défaut système macOS.** Dès qu'il perd ce
   statut, macOS tue la session et le device disparaît du HAL. `set_system_default_device` sur un
   device AirPlay stale **accepte silencieusement sans rien faire**. D'où la stratégie de
   préservation : quand on quitte AirPlay vers un autre device, **on ne change pas le défaut
   système** ; l'audio est routé par assignement AudioUnit explicite. Et
   `AudioUnitSetProperty(kAudioOutputUnitProperty_CurrentDevice)` échoue **toujours** pour AirPlay —
   donc `get_device_id()` renvoie `None` pour ces devices.
2. **`libsmbclient` est un singleton au niveau du process.** Deux `SmbClient` concurrents →
   EINVAL (os error 22). Tout passe par un mutex unique. Ce n'est pas un choix d'architecture, c'est
   une contrainte de la bibliothèque C.
3. **Les media keys macOS ne s'attrapent pas avec des global shortcuts.** Apple Music intercepte
   F7/F8/F9 en priorité. Seule solution : `MPRemoteCommandCenter` via souvlaki.
4. **Le drag & drop HTML5 ne fonctionne pas dans la WebView Tauri.** Implémentation manuelle
   mousedown/mousemove/mouseup obligatoire.
5. **Bug WebKit sur les petits CSS Grid** : dans une grille de 28×28 px, une cellule vide résout
   `height: 100%` contre le conteneur et non contre son grid area. Corrigé 4 fois avant d'abandonner
   Grid pour du positionnement absolu (`calc(50% - 0.5px)`). La même grille en 96×96 ne pose aucun
   problème.
6. **`dylibbundler` ajoute un `LC_RPATH` à chaque dylib**, mais ~66 des 92 dylibs Samba de Homebrew
   en avaient déjà un → doublons → **dyld refuse de charger depuis Big Sur**. Il faut une passe
   `install_name_tool -delete_rpath` puis re-signer chaque dylib touchée. Sans ça, l'app crash au
   lancement sur tout Mac sans Homebrew.
7. **Tauri notarise l'app mais pas le DMG.** Sans `xcrun notarytool submit` explicite sur le DMG,
   Gatekeeper le refuse même si l'app à l'intérieur est notarisée. Il faut notariser **et stapler les
   deux**.

À quoi s'ajoutent deux pièges de l'écosystème Tauri :
`option_env!()` renvoie `None` **en silence** si la variable manque (binaire livré sans secrets), et
`min-width: 0` manquant sur un conteneur flex fait gonfler le flex item à ~4 500 px dès qu'un enfant
utilise `width: calc(100% + x)`.

## Faire vivre les invariants en V2

Le mécanisme V1 vaut la peine d'être repris tel quel :

1. Une **source de vérité unique** (ce fichier d'invariants) + un résumé d'une ligne par règle dans
   le `CLAUDE.md` du projet.
2. Chaque règle porte **une gravité** (BLOQUANT = merge refusé / AVERTISSEMENT = signalé). Une règle
   ajoutée sans gravité est BLOQUANTE par défaut jusqu'à classement.
3. Chaque règle dit **pourquoi elle existe** (la régression qu'elle a coûtée). Sans le pourquoi, la
   règle sera supprimée par quelqu'un qui la trouve arbitraire.
4. Un **agent de relecture** (`invariant-guardian`, copié dans
   [`../06-setup-v2/agents/`](../06-setup-v2/agents/)) confronte le diff aux règles avant chaque PR.
5. Un **hook `Stop`** (`invariant-reminder.sh`, copié dans
   [`../06-setup-v2/hooks/`](../06-setup-v2/hooks/)) qui propose d'ajouter une règle quand du code a
   changé sans que le fichier d'invariants bouge.

**Ce qui doit changer en V2** : un invariant devrait, autant que possible, être **un test** plutôt
qu'une phrase. En V1, seuls 3 des 19 invariants sont couverts par un test
(`AlbumGrouping.test.js`, `ShuffleButton.test.js`, les 5 tests `sentry_init`). Les 16 autres reposent
sur la vigilance humaine ou celle d'un agent. Voir
[`../06-setup-v2/PLAN-DE-RECONSTRUCTION.md`](../06-setup-v2/PLAN-DE-RECONSTRUCTION.md), phase 0.

## Mémoire de l'agent de relecture

V1 a aussi une mémoire d'agent (un faux positif rencontré, puis corrigé) :

> La règle de zone « audio **et** UI dans le même diff → AVERTISSEMENT » ne se déclenche que sur les
> chemins **Rust** (`src-tauri/src/audio/`, `audio_engine.rs`, `audio_decoder.rs`, `coreaudio_*`).
> Les fichiers JS `playback.js`, `shortcuts.js`, `renderer.js` sont du frontend malgré leur nom.
> *(Appris lors de la relecture du correctif shuffle, 2026-10-29.)*
