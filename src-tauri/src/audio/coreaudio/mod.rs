//! Le backend CoreAudio : la sortie vers le DAC.
//!
//! # Le découpage, et pourquoi il suit les coutures de V1
//!
//! V1 tenait tout en deux fichiers — `coreaudio_backend.rs` (1 326 lignes) et
//! `coreaudio_stream.rs` (667). La limite de 400 lignes par fichier interdit le portage
//! fichier pour fichier, mais le découpage ne suit pas une règle arithmétique : il suit les
//! coutures que V1 avait déjà, pour que chaque fonction reste reconnaissable depuis V1.
//!
//! | pièce | ce qu'elle porte | venait de |
//! |---|---|---|
//! | [`hal`] | les six accès au HAL, les seuls du projet | répété 12 fois dans V1 |
//! | [`peripheriques`] | les trouver, lire leurs propriétés | `get_all_output_devices`, `get_device_name`… |
//! | [`frequence`] | la négociation de fréquence | `get/set_device_sample_rate`, `get_supported_sample_rates` |
//! | [`exclusif`] | le mode exclusif (*hog mode*) | `*_hog_mode_internal` |
//! | [`cache`] | le cache, et la survie des périphériques sans fil | `refresh_device_cache` |
//! | [`tampon`] | le tampon circulaire décodeur → callback | `ringbuf` (dépendance en V1) |
//! | [`flux`] | la vie de l'AudioUnit | `coreaudio_stream.rs` |
//! | [`rendu`] | le callback temps réel | idem |
//! | [`routage`] | quel périphérique joue, et PLAT-1 | `set_output_device`, `check_device_change` |
//! | [`mesures`] | ce que le backend laisse observer et régler en jouant | **neuf** : V1 ne mesurait rien |
//! | [`negociation`] | ce qui entre, ce qui sort, et le badge bit-perfect | `AudioSpecs` + `playback.js` |
//! | [`backend`] | l'implémentation de `BackendAudio` | `impl AudioBackend for CoreAudioBackend` |
//!
//! # Ce qui a changé par rapport à V1, et seulement ça
//!
//! 1. **Le flux est déclaré en entier 32 bits** et non en flottant : conséquence directe du
//!    format interne acté par l'ADR 0007. Obligatoire, pas optionnel.
//! 2. **Les famines sont comptées.** V1 n'avait aucun compteur de trou dans le son ; ici
//!    chaque callback servi à vide alors que le décodeur devait suivre est compté et remonte
//!    jusqu'à l'écran.
//!
//! Le **repli de fréquence**, lui, est porté tel quel : quand le périphérique ne sait pas
//! servir la fréquence du fichier, on replie sur la plus proche et **on l'affiche** —
//! fréquence entrante, fréquence sortante, pas de badge bit-perfect. C'est la règle du
//! produit (voir [`negociation`]), pas un défaut à corriger.
//!
//! Tout le reste est porté à l'identique, PLAT-1 compris.
//!
//! # ⚠️ Ce qu'aucun test ne prouvera
//!
//! Tout ce fichier-ci parle à CoreAudio, donc à du matériel. Seul [`tampon`] s'éprouve en
//! CI, et c'est pour ça qu'il est écrit à la main plutôt que pris dans une dépendance. Le
//! reste se vérifie avec un DAC branché : le protocole est dans `docs/PROTOCOLE-SON.md`,
//! et les trois vérifications de machine dans `src-tauri/tests/sortie_coreaudio.rs`.

pub mod backend;
pub mod cache;
pub mod exclusif;
pub mod flux;
pub mod frequence;
pub mod hal;
pub mod mesures;
pub mod negociation;
pub mod peripheriques;
pub mod rendu;
pub mod routage;
pub mod tampon;

pub use backend::BackendCoreAudio;
pub use negociation::Negociation;
pub use peripheriques::InfoPeripherique;
