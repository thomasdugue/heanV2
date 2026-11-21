# Comportements UX validés — catalogue anti-régression

Chacun de ces comportements a été **cassé au moins une fois** en V1 puis corrigé. Même si V2 repart de
zéro, le comportement attendu (colonne « Attendu ») reste le contrat. La colonne « Cause en V1 » dit
ce qui l'avait cassé : utile pour reconnaître le piège dans une autre implémentation.

> Les ID `V0xx` viennent de la spec V1 (section 13), je les garde pour la traçabilité.
> Les ID `W0xx` sont ajoutés ici (comportements validés après la spec de mars).

| ID | Attendu | Cause en V1 | Dépend de la techno ? |
|---|---|---|---|
| **V001** | La home s'affiche complète au démarrage, sans flash ni état partiel | `transitionView` synchrone qui n'attendait pas le render async ; renders obsolètes non annulés | non — tout framework a ce piège |
| **V002** | Recently Played : 3 colonnes ≥ 768 px, 2 entre 480-768, 1 en dessous, max 6 items | `width` sans `box-sizing: border-box` | non |
| **V003** | Les carousels de la home s'étendent jusqu'au bord droit, scroll horizontal OK, la grille ne gonfle pas | `min-width: 0` manquant sur le conteneur flex → colonnes à 1492 px, grille à ~4500 px | **oui** (flexbox) |
| **V004** | Pas de rechargement de la bibliothèque si le scan n'a rien trouvé de nouveau | `scan_complete` rechargeait toujours → race avec le premier render de la home | non |
| **V005** | On peut glisser une track vers une playlist | HTML5 drag & drop **ne fonctionne pas** dans la WebView Tauri/WKWebView | **oui** (WebView) |
| **V006** | La vue Pistes reste à 60 fps avec 10 000+ pistes | création/destruction de nœuds DOM à chaque frame | non |
| **V007** | Les touches media du clavier contrôlent Hean, même si Apple Music tourne | global shortcuts Tauri F7/F8/F9 interceptés en priorité par Apple Music | **oui** (macOS) |
| **V008** | Une session AirPlay survit à un changement de device | `set_system_default_device` ne peut pas réactiver un device AirPlay devenu stale (l'API accepte sans effet) | **oui** (CoreAudio) |
| **V009** | Aucun dialogue Keychain au lancement | `retrieve_password()` appelé sans vérifier d'abord le cache session | **oui** (macOS) |
| **V010** | La section « loose tracks » d'une page artiste fait toute la largeur | contrainte à 180 px par `repeat(auto-fill, 180px)` du parent | **oui** (CSS Grid) |
| **V011** | La scrollbar des carousels est invisible au repos, visible au survol | — (comportement voulu, facile à perdre) | non |
| **V012** | Après un bulk edit, **toutes** les vues se rafraîchissent | condition `if (includes(currentView))` qui excluait certaines vues | non |
| **V013** | Les objets d'état partagés ne sont jamais réassignés | `library = {}` cassait la référence vue par les autres modules | **oui** (architecture V1 — disparaît avec un store) |
| **V014** | `artist.albums` se compte avec `.length` | passé de Set à Array sans mettre à jour les appelants | **oui** (JS non typé — disparaît avec TypeScript) |
| **V015** | Une métadonnée éditée survit au prochain démarrage | `METADATA_CACHE` et `TRACKS_CACHE` désynchronisés | non |
| **V016** | Sous 900 px la sidebar se réduit à 48 px | `style.width` inline (du resize) écrasait la media query | non |
| **W001** | Les vignettes de playlist de la sidebar s'affichent au premier démarrage | covers chargées en async dans des cellules déconnectées du DOM par un rebuild concurrent | non |
| **W002** | La mosaïque 28×28 n'a pas de cellule noire fantôme | bug WebKit : dans un petit CSS Grid, une cellule vide résout `height: 100%` contre le conteneur. **Corrigé 4 fois** avant d'abandonner Grid pour le positionnement absolu | **oui** (WebKit) |
| **W003** | Un album avec des artistes « feat. » différents n'apparaît qu'une fois dans une mosaïque | déduplication par `artist+album` au lieu de `album` seul | non |
| **W004** | L'icône shuffle ne bouge pas quand on change de mode | statut collé au texte du bouton (`'⤮ᴬ'`) + logique dupliquée dans `shortcuts.js` avec un **autre glyphe** (⤢ au lieu de ⤮) | non — symptôme de duplication |
| **W005** | L'UI ne gèle pas quand le NAS est injoignable | `get_cover` / `write_metadata` faisaient de l'I/O SMB bloquante sur le thread de commande | non |
| **W006** | Un tag contenant `"` n'est pas tronqué à la sauvegarde | `escapeHtml()` n'échappait pas les guillemets | non |
| **W007** | Une bibliothèque uniquement NAS s'affiche sur la home | la home supposait au moins une source locale | non |
| **W008** | Les fichiers temporaires SMB ne s'accumulent pas | `smb_buffer/` jamais nettoyé | non |

## Lecture pour V2

- **9 comportements sur 24 sont des pièges de plateforme** (CoreAudio, WebKit, macOS, WebView, flexbox).
  Ceux-là reviendront quelle que soit la techno choisie : ils sont documentés dans
  [`04-invariants/`](../04-invariants/) et c'est le vrai actif de V1.
- **2 comportements (V013, V014) disparaissent** si V2 utilise un store immuable et TypeScript.
- Le reste relève de tests. Voir [`06-setup-v2/PLAN-DE-RECONSTRUCTION.md`](../06-setup-v2/PLAN-DE-RECONSTRUCTION.md),
  phase 0 : écrire ces comportements comme tests **avant** de porter les features.
