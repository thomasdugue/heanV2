# Contrat anti-régression — avancement

<!-- FICHIER GÉNÉRÉ — NE PAS ÉDITER. Source : tests/ux/_contrat.ts. Régénérer : npm run gen -->

Les 24 comportements cassés au moins une fois en V1. `./check` affiche le compte vert/rouge ; il est **normal qu'ils soient tous rouges tant que la phase correspondante n'est pas faite**.
9 sur 24 sont des pièges de plateforme : ils reviendront quelle que soit la techno.

| # | Attendu | Phase | Piège de plateforme |
|---|---|---|---|
| V001 | La home s’affiche complète au démarrage, sans flash ni état partiel | 3 | — |
| V002 | Recently Played : 3 colonnes ≥ 768 px, 2 entre 480 et 768, 1 en dessous, max 6 items | 3 | — |
| V003 | Les carousels s’étendent jusqu’au bord droit, la grille ne gonfle pas | 3 | oui |
| V004 | Pas de rechargement de la bibliothèque si le scan n’a rien trouvé de nouveau | 2 | — |
| V005 | On peut glisser une piste vers une playlist | 3 | oui |
| V006 | La vue Pistes reste à 60 fps avec 10 000+ pistes | 3 | — |
| V007 | Les touches media contrôlent Hean, même si Apple Music tourne | 1 | oui |
| V008 | Une session AirPlay survit à un changement de device | 1 | oui |
| V009 | Aucun dialogue Keychain au lancement | 5 | oui |
| V010 | La section « loose tracks » d’une page artiste fait toute la largeur | 3 | oui |
| V011 | La scrollbar des carousels est invisible au repos, visible au survol | 3 | — |
| V012 | Après un bulk edit, toutes les vues se rafraîchissent | 2 | — |
| V013 | Les objets d’état partagés ne sont jamais réassignés | 2 | oui |
| V014 | artist.albums se compte avec .length | 2 | oui |
| V015 | Une métadonnée éditée survit au prochain démarrage | 2 | — |
| V016 | Sous 900 px la sidebar se réduit à 48 px | 3 | — |
| W001 | Les vignettes de playlist de la sidebar s’affichent au premier démarrage | 3 | — |
| W002 | La mosaïque 28×28 n’a pas de cellule noire fantôme | 3 | oui |
| W003 | Un album avec des artistes « feat. » différents n’apparaît qu’une fois dans une mosaïque | 3 | — |
| W004 | L’icône shuffle ne bouge pas quand on change de mode | 3 | — |
| W005 | L’UI ne gèle pas quand le NAS est injoignable | 5 | — |
| W006 | Un tag contenant un guillemet n’est pas tronqué à la sauvegarde | 2 | — |
| W007 | Une bibliothèque uniquement NAS s’affiche sur la home | 5 | — |
| W008 | Les fichiers temporaires SMB ne s’accumulent pas | 5 | — |
