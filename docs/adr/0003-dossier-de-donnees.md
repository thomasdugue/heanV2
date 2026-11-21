# 0003 — Dossier de données `hean/` avec migration depuis `noir/`

- **Date** : 2026-11-21
- **État** : Acceptée
- **Décideur** : Thomas

## Contexte

Le produit s'appelle **Hean** depuis le printemps 2026 — il s'appelait Noir, renommé à cause d'une
collision de marque et de SEO avec « Noir — Dark Mode for Safari », Noir Labs et Native Instruments
NOIRE. Mais la quasi-totalité des identifiants techniques porte encore l'ancien nom :

- dossier de données : `dirs::data_dir().join("noir")`, soit
  `~/Library/Application Support/noir/` sur macOS ;
- protocole custom : `noir://` (sert l'audio et les covers depuis le disque) ;
- variables compile-time : `NOIR_WORKER_URL`, `NOIR_WORKER_SECRET` ;
- dépôt : `noirdesktop` · worker Cloudflare : `noir-feedback`.

Ce dossier contient tout ce qui compte pour un utilisateur : bibliothèque scannée, playlists,
favoris, historique d'écoute, sources NAS, caches de covers et de thumbnails. Environ 50 beta-testeurs
en ont un.

## Décision

**`~/Library/Application Support/hean/`**, avec une **migration one-shot au premier lancement** de V2.

La migration déplace le dossier `noir/` vers `hean/` (copie puis vérification puis suppression de
l'ancien, pas un `mv` sec), et consigne dans un fichier d'état qu'elle a eu lieu, pour être
idempotente.

Portée de la migration — tout ce qui précède, **plus** :

- `dap_sync.db`, la base SQLite de la feature archivée, laissée en place en V1 *exprès* pour préserver
  les configurations des beta-testeurs en vue d'une réintégration ;
- les entrées **Keychain** des mots de passe SMB. Renommer le dossier ne les déplace pas : il faut
  décider si V2 lit les mêmes entrées (même nom de service) ou les recrée. **Point ouvert à traiter
  en phase 3** — si V2 change le nom de service, chaque testeur doit ressaisir ses mots de passe NAS,
  ce qui est exactement le symptôme listé comme « bug sérieux à reporter » dans la doc beta.

## Alternatives écartées

| Alternative | Pourquoi écartée |
|---|---|
| **Garder `noir/`** | Compatibilité gratuite, zéro risque de perte, et V1 et V2 pourraient coexister sur la même machine (utile pour le mode parité du harnais). Écartée parce que l'incohérence deviendrait définitive : dossier, protocole et variables garderaient un nom abandonné pour toujours. Dette cosmétique, mais permanente, et payée par chaque personne — ou chaque modèle — qui découvrira le projet ensuite. |
| **`hean/` sans migration** | Propre immédiatement, rien à écrire. Écartée : les ~50 testeurs perdraient bibliothèque, playlists, favoris, historique et sources NAS, avec un re-scan complet au premier lancement. Mauvais signal pour des gens qui ont pris le temps de configurer. |

## Conséquences acceptées

- **Une migration à écrire, et surtout à tester.** Une migration non testée est une perte de données
  chez 50 personnes. Elle se teste sur une **copie réelle** d'un dossier `noir/` complet, pas sur
  des fixtures : c'est la seule façon de rencontrer les cas tordus (chemins NFD, fichiers de cache
  corrompus, dossier partiellement écrit).
- **Le renommage à décider pour le reste** : `noir://` → `hean://` et `NOIR_WORKER_*` →
  `HEAN_WORKER_*`. Le protocole est interne (aucune donnée persistée ne le référence durablement), les
  variables impliquent de mettre à jour `.env.local` et les secrets côté Cloudflare.
- **Coexistence V1/V2 compromise** : une fois la migration passée, V1 ne retrouvera plus ses données.
  Si le mode parité du harnais a besoin des deux côte à côte, il faut travailler sur une **copie** du
  dossier et non sur l'original — à prévoir avant d'exécuter la migration pour de vrai.
- **Point lié non tranché** : V2 est-elle une mise à jour de V1 (même bundle identifier
  `com.tdugue.hean`, les testeurs la reçoivent par l'updater Tauri) ou une application distincte
  installée à côté ? Le premier cas rend la migration irréversible pour le testeur. **À trancher
  avant la phase 7** (distribution).
