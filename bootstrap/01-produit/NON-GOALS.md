# Hors périmètre — décisions produit à ne pas rejouer

Chaque ligne est une décision déjà prise en V1, avec sa raison. Les reprendre telles quelles évite de
re-débattre. Si l'une doit changer pour V2, c'est un arbitrage explicite à poser au départ, pas une
dérive en cours de route.

## Plateformes

| Hors périmètre | Pourquoi |
|---|---|
| Windows | Le backend audio est CoreAudio natif (hog mode, sample rate HAL). Le trait `audio/backend.rs` existe pour un futur WASAPI, mais rien n'est implémenté. |
| Linux | Idem, plus un marché négligeable sur ce segment. |
| macOS < 11 (Big Sur) | `minimumSystemVersion: "11.0"`. Les versions antérieures sont refusées au lancement. |
| Mac App Store | **Impossible techniquement** : il n'existe aucun entitlement App Sandbox pour l'accès exclusif à un device de sortie. `kAudioDevicePropertyHogMode` exige la propriété exclusive au niveau PID. Audirvana, Roon, HQPlayer et Swinsian ont tous fait le même constat. → distribution directe uniquement. Détail dans [`recherche/gtm-playbook-solo-dev.md`](recherche/gtm-playbook-solo-dev.md). |
| iOS / iPadOS | Jamais envisagé. |

## Formats audio

| Hors périmètre | Pourquoi |
|---|---|
| DSD | Pipeline PCM f32 de bout en bout ; DSD demande un chemin DoP ou natif séparé. |
| MQA | Propriétaire, licence nécessaire. |
| Opus | Non prioritaire ; extension retirée du scanner pour ne pas lister des fichiers illisibles. |
| WMA | Idem. |

Supportés : FLAC, ALAC, WAV, AIFF, MP3, AAC/M4A, OGG Vorbis.

## Features

| Hors périmètre | Pourquoi |
|---|---|
| **DAP Sync** (sync vers baladeur : Mass Storage / MTP / SMB) | Retiré du build le 2026-04-22 : instabilité MTP non résolue (Transaction ID mismatch, timeouts, `InvalidObjectHandle`). ~1 mois de travail archivé. Rapport complet : [`05-postmortem/archive-dap-sync/`](../05-postmortem/archive-dap-sync/). **Ne pas remettre ça au programme de V2 sans relire `KNOWN_ISSUES.md` de l'archive.** Le code complet (~12 400 lignes) est conservé — voir [`../05-postmortem/archive-dap-sync/README-CODE.md`](../05-postmortem/archive-dap-sync/README-CODE.md) : à traiter comme une feature à porter après la phase 8, pas comme un patch à réappliquer. |
| Scrobbling (Last.fm, ListenBrainz) | Listé comme « envisagé » en V1, jamais commencé. |
| Intégration Roon / Audirvana / TIDAL / Qobuz | Pas de streaming, l'app est un lecteur de fichiers locaux + NAS. |
| Sync iCloud / multi-machines | Toute la persistance est locale (JSON + Keychain). |
| Crossfade | « Envisagé » en V1. Incompatible avec la promesse bit-perfect telle qu'implémentée. |
| ReplayGain | « Envisagé ». Implique une modification du gain → sort du bit-perfect strict. |
| Visualiseur spectral FFT | « Envisagé ». Le RMS temps réel existe (fullscreen player), pas la FFT. |
| AutoEQ (profils par casque) | « Prévu » en V1, la structure biquad est compatible, rien n'est fait. |
| Mode clair | Thème sombre intégral assumé, c'est l'identité du produit. |

## Nom du produit

Le produit s'est appelé **Noir** jusqu'au printemps 2026, puis **Hean**. Raison : collision de marque
et de SEO avec « Noir — Dark Mode for Safari » (getnoir.app, une des extensions Safari les plus
populaires), Noir Labs (plugins audio Ableton) et Native Instruments NOIRE. Détail :
[`recherche/market-viability.md`](recherche/market-viability.md).

**Conséquence pour V2** : beaucoup d'identifiants techniques V1 portent encore « noir » —
`com.tdugue.noir` (identifiant historique, devenu `com.tdugue.hean`), protocole `noir://`, dossier de
données `~/.local/share/noir/`, dépôt `noirdesktop`, worker `noir-feedback`, variables
`NOIR_WORKER_URL` / `NOIR_WORKER_SECRET`. **Arbitrage à poser au départ de V2** : on renomme tout en
`hean` ou on garde les chemins historiques. **Tranché en V2 : `hean/`, avec migration one-shot**
(ADR 0003, 0010, 0014). La « compatibilité des données des beta-testeurs » invoquée ici n'a jamais
existé — la bêta V1 n'a pas été distribuée. Voir [`03-architecture/PERSISTANCE.md`](../03-architecture/PERSISTANCE.md).

## Modèle économique (contexte, pas une décision figée)

Licence perpétuelle, pas d'abonnement. Prix étudié : **49 € en early-bird → 69 € standard**,
distribution directe (LemonSqueezy). Revenu Year 1 estimé ~22 400 € pour < 1 200 € de coûts fixes.
Détail : [`recherche/pricing-gtm.md`](recherche/pricing-gtm.md).

**Avant d'arbitrer une décision produit avec le marché en tête, lire [`MARCHE.md`](MARCHE.md)** :
ce qui est utilisable dans les trois études, et ce qui ne doit pas être cité.
