# Mesures de référence V1 — 2026-11-21

Les chiffres contre lesquels V2 sera comparée. Relevés sur `/Applications/Hean.app`
(v0.2.0-beta.1), machine de Thomas, à froid.

**Méthode** : [`../tools/measure-app.sh`](../tools/measure-app.sh). V2 devra être mesurée avec le
**même script** — sinon les chiffres ne sont pas comparables. C'est pour ça que l'outil est dans le
dépôt et pas dans un presse-papier.

---

## 1. Échelle de la bibliothèque de référence

Ce n'est pas une bibliothèque de 10 000 pistes : les budgets doivent être calés sur **3 443**.

| | |
|---|---|
| Pistes | **3 443** |
| Albums (clé = nom d'album) | **336** |
| Artistes | **508** |
| Local / NAS | **306 / 3 137** — soit **91 % sur SMB** |
| Formats | 3 341 FLAC (97 %), 82 MP3, 19 M4A, 1 WAV |
| Playlists | 6 |
| Historique d'écoute | 188 entrées |
| Chemins de bibliothèque | 1 |
| `excluded_paths` | 0 |

**Conséquence pour V2** : la bibliothèque est à 91 % en réseau. Tout ce qui touche au scan, aux
covers et à la lecture passe par SMB. Les budgets de démarrage et de covers ne veulent rien dire
mesurés sur une bibliothèque locale.

## 2. Dossier de données — 889 Mo

| Élément | Taille | Fichiers |
|---|---|---|
| `smb_buffer/` | **606 Mo** | 19 | ⚠️ voir §6 |
| `covers/` | 275 Mo | 2 173 | ~127 Ko par cover (pleine résolution) |
| `network_scan_cache.json` | 2,2 Mo | | |
| `thumbnails/` | **2,1 Mo** | **247** | ⚠️ voir §5 |
| `tracks_cache.json` | 1,7 Mo | | 3 443 pistes |
| `added_dates_cache.json` | 511 Ko | | |
| `cover_cache.json` | 398 Ko | | 1 828 entrées |
| `metadata_cache.json` | 117 Ko | | 307 entrées seulement |
| `listening_history.json` | 73 Ko | | |
| `playlists.json` | 6,3 Ko | | |
| `logs/` | 32 Ko | 8 | 72 lignes en tout |

## 3. Démarrage à froid

| Mesure | Valeur |
|---|---|
| Processus visible | **90 ms** |
| RSS natif stabilisé | **127 Mo à t ≈ 2,5 s** |
| CPU retombé à ~0 % | **t ≈ 2,8 s** |
| Première ligne de log Rust | t + 0,2 s environ |

**Démarrage « prêt » ≈ 2,8 s.** C'est un **proxy** (plateau de RSS + CPU retombé), pas le premier
pixel : sans instrumentation dans l'app, « UI utilisable » n'est pas observable de l'extérieur.
V2 devra poser un marqueur explicite (un `tracing::info!` au premier rendu terminé) pour que la
mesure devienne exacte — et ce marqueur doit être ajouté **en phase 0**, pas après.

## 4. Mémoire au repos, vue d'accueil

| Processus | RSS | phys_footprint |
|---|---|---|
| `noir-tauri` (Rust + natif) | 128 Mo | **53 Mo** |
| `WebKit.WebContent` (JS + DOM) | 201 Mo | **155 Mo** |
| `WebKit.GPU` | 85 Mo | 20 Mo |
| `WebKit.Networking` | 19 Mo | 7 Mo |
| **Total** | **433 Mo** | **235 Mo** |

Deux choses à retenir :

1. **La WebView coûte 3× le natif** (155 Mo contre 53 Mo de footprint). Pour une app qui se veut
   légère, le gros de la mémoire est dans le frontend — alors que le frontend ne pèse que 960 Ko sur
   disque. C'est le DOM, le heap JS et les bitmaps décodés des covers.
2. **235 Mo de footprint au repos**, sur la vue d'accueil, avant d'avoir fait défiler quoi que ce
   soit. C'est le chiffre à ne pas dégrader. Et c'est plus élevé que ce que « très light » suggère :
   si la légèreté est une promesse produit, **il y a du travail à faire en V2, pas seulement à ne pas
   régresser**.

## 5. Observation : le cache de thumbnails est quasi vide

**247 thumbnails pour 2 173 covers extraites** — 11 % de couverture.

Le pipeline de covers est censé servir des miniatures 80×80 dans les listes et les carrousels
(`get_cover_thumbnail`), et ne tomber sur la pleine résolution (`get_cover`, ~127 Ko) qu'en secours.
Dans les faits, `get_cover_thumbnail` renvoie `null` dans ~89 % des cas et c'est la **pleine
résolution qui est décodée** pour afficher une vignette.

C'est très probablement une part importante des 155 Mo de la WebView, et une piste de gain
immédiate pour V2 : générer les thumbnails de façon fiable au scan, et ne jamais décoder une cover
pleine résolution pour une vignette.

> À confirmer : il faudrait vérifier si le scan échoue à générer les thumbnails ou s'il ne les
> génère que pour les pistes locales (306 sur 3 443 — l'ordre de grandeur correspond à peu près aux
> 247 fichiers).

## 6. Deux bugs trouvés en mesurant

### 🔴 L'auto-updater n'a jamais fonctionné

L'endpoint configuré dans `tauri.conf.json` :

```
https://github.com/thomasdugue/noirdesktop/releases/latest/download/latest.json
```

→ **HTTP 404**, vérifié. Cause : la seule release publiée est une **prerelease**, et
`/releases/latest` de GitHub **ignore les prereleases** (l'API `repos/.../releases/latest` renvoie
elle aussi 404). Les logs comptent 23 erreurs `update endpoint did not respond with a successful
status code`.

**Conséquence** : aucun beta-testeur n'a jamais pu recevoir de mise à jour automatique depuis le
lancement du 2026-05-01.

**Leçon pour V2** — à ajouter aux invariants de release : *ne jamais pointer l'updater sur
`/releases/latest` tant qu'on publie des prereleases.* Soit on publie en release normale, soit on
pointe sur un tag fixe (`releases/download/latest/latest.json` avec un tag `latest` qu'on déplace),
soit on héberge `latest.json` ailleurs. Et dans tous les cas : **un smoke test qui `curl` l'endpoint
et vérifie le code 200 fait partie de la checklist de release.**

### 🟠 Le nettoyage de `smb_buffer` ne nettoie pas

`smb_buffer/` contenait **607 Mo** en 19 fichiers datés du 29 octobre. L'app a été lancée pour cette
mesure — le commit `eaee676` (« cleanup stale smb_buffer temp files on app startup ») devait les
supprimer. Après lancement : **toujours 607 Mo**.

Soit la condition de péremption ne correspond pas, soit le nettoyage ne tourne pas dans ce chemin de
démarrage. À reproduire et à comprendre **avant** de porter ce code en V2 — et à couvrir par un test.

## 6bis. Dérive mémoire au repos — à élucider

Deux relevés du **même processus** (PID 10612, lancé à 09:45), à ~70 minutes d'intervalle, sans
aucune interaction entre les deux :

| Processus | juste après le lancement | ~70 min plus tard, au repos | Δ RSS |
|---|---|---|---|
| `noir-tauri` | 128 Mo | 142 Mo | +14 Mo |
| `WebKit.WebContent` | 201 Mo | 270 Mo | **+69 Mo** |
| `WebKit.GPU` | 85 Mo | 117 Mo | +32 Mo |
| `WebKit.Networking` | 19 Mo | 19 Mo | 0 |
| **Total** | **433 Mo RSS / 235 Mo footprint** | **547 Mo RSS / 255 Mo footprint** | **+114 Mo / +20 Mo** |

Le journal ne contient que deux lignes sur la période (init du logging, puis l'erreur d'updater) :
rien n'indique un scan.

**Cause indéterminée.** Trois hypothèses, non tranchées : un scan de fond non journalisé (3 137
pistes en SMB), la croissance normale des caches internes de WebKit au repos, ou une fuite. Une
croissance de +20 Mo de footprint en une heure d'inactivité n'est pas alarmante en soi, mais elle
n'est pas expliquée — et une app censée rester ouverte toute une session d'écoute doit savoir ce
qu'elle fait de sa mémoire.

**Pour V2** : c'est exactement ce qu'un marqueur d'instrumentation réglerait en une minute. À
trancher avec une mesure longue (plusieurs heures, échantillonnage à la minute) une fois V2 capable
de journaliser ses phases.

## 7. Ce qui reste à mesurer

Trois chiffres manquent, parce qu'ils demandent d'interagir avec l'app :

| Mesure | Comment |
|---|---|
| Mémoire après défilement de toute la grille d'albums | protocole §3 de [`../tools/measure-app.sh`](../tools/measure-app.sh) |
| Images par seconde du virtual scroll sur 3 443 pistes | demande un compteur de FPS dans l'app |
| Temps de réponse de la recherche | demande une instrumentation dans l'app |

### Tentative du 2026-11-21 à 10:56 — invalide

Un échantillonnage de 90 s a été lancé, mais la courbe est **plate** (545 → 547 Mo de RSS,
254 → 255 Mo de footprint, soit 2 Mo de variation sur 90 s) : **aucune navigation n'a eu lieu dans
la fenêtre de mesure.**

Cause : l'échantillonnage a démarré **en même temps** que l'envoi des consignes, donc la fenêtre de
90 s a commencé avant qu'elles puissent être lues. Erreur de conception du protocole, pas de la
mesure.

**Correction du protocole** : celui qui navigue doit **démarrer lui-même** le script, pour contrôler
le top départ. D'où le bloc de lancement ci-dessous.

**Protocole** (à reproduire à l'identique sur V2) :

```bash
cd /Users/tsunami25/Documents/Thomas/heanV2 && ./tools/measure-app.sh noir-tauri 90
```

Puis, pendant que le script échantillonne : 10 s au repos sur l'accueil → aller sur **Albums** et
défiler toute la grille sans pause → taper trois requêtes dans la recherche. Les trois paliers se
lisent sur la courbe imprimée.

Pour les FPS et la recherche, il n'y a pas de raccourci : il faut un compteur dans l'app. **En V2,
l'instrumentation fait partie de la phase 0** — un marqueur de premier rendu, un compteur de FPS
activable, et un chronomètre sur la recherche. Sans ça, on en est réduit à des proxys, comme ici.
