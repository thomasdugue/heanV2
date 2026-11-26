//! Le moteur audio.
//!
//! # L'ordre dans lequel ce module a été construit, et pourquoi
//!
//! Pas en commençant par CoreAudio, mais par ce qui rend CoreAudio vérifiable. En V1, le
//! bit-perfect était une **promesse** : aucun test ne portait sur la chaîne audio, et toute
//! vérification passait par un DAC branché et une oreille. Un défaut de bit-perfect ne fait
//! échouer aucun test — ça joue, un peu moins bien — donc il pouvait vivre indéfiniment.
//!
//! Ici la garantie est une assertion qui tourne en CI, sans matériel :
//!
//! | pièce | rôle |
//! |---|---|
//! | [`format`] | fréquence, canaux, profondeur — le seul chiffre dont dépend l'exactitude |
//! | [`samples`] | le format interne (`i32` à l'échelle 32 bits) et l'aller-retour exact |
//! | [`convert`] | la convention d'échelle de symphonia, mesurée et sous test |
//! | [`decode`] | fichier → trames, en blocs |
//! | [`chain`] | le volume, et surtout le chemin qui ne traite rien |
//! | [`wav`] | l'en-tête WAV, dans la forme exacte qu'écrit ffmpeg |
//! | [`backend`] | le trait : le point de substitution |
//! | [`wav_backend`] | un backend qui écrit un fichier au lieu de parler au HAL |
//!
//! Le test `src-tauri/tests/bit_perfect.rs` referme la boucle : il décode une fixture FLAC
//! vers WAV à travers le backend de test et compare le résultat **octet par octet** à un
//! fichier écrit par ffmpeg, donc par un décodeur indépendant du nôtre.
//!
//! # La sortie, portée le 2026-11-26
//!
//! | pièce | rôle |
//! |---|---|
//! | [`coreaudio`] | le backend qui parle au DAC — périphériques, fréquence, mode exclusif, AudioUnit |
//! | [`lecteur`] | ce qui transforme « appuie sur lecture » en son : file, pause, déplacement, enchaînement |
//!
//! **CoreAudio direct, jamais CPAL** (ADR 0002). Et la règle du bit-perfect n'est pas
//! « aucun rééchantillonnage », formule qu'on lit dans le skill `hean-invariants` et
//! qui est plus absolue que la décision réelle : c'est **« bit-perfect quand le DAC sait
//! décoder, et le dire quand il ne sait pas »**. La fréquence du périphérique suit celle du
//! fichier quand elle peut ; sinon on replie, et l'écran affiche la fréquence entrante, la
//! fréquence sortante, et pas de badge bit-perfect. Voir [`coreaudio::negociation`].
//!
//! # Ce que ce module ne contient toujours pas
//!
//! L'égaliseur (ses huit fréquences de bandes sont une donnée de parité que le bootstrap ne
//! porte pas), les touches media (PLAT-3), et **la lecture d'un fichier du NAS** — celle-là
//! demande de toucher à `src-tauri/src/network/`, hors du périmètre de la session qui a
//! porté la sortie (ADR 0013). Les pièges de plateforme sont dans
//! `bootstrap/04-invariants/SAVOIR-AUDIO.md`, et PLAT-1 est appliqué dans
//! [`coreaudio::routage`].

pub mod backend;
pub mod chain;
pub mod convert;
pub mod coreaudio;
pub mod decode;
pub mod error;
pub mod format;
pub mod lecteur;
pub mod samples;
pub mod wav;
pub mod wav_backend;

pub use backend::BackendAudio;
pub use coreaudio::{BackendCoreAudio, Negociation};
pub use lecteur::{Commande, EtatPartage, Lecteur};
pub use chain::{Chaine, Volume};
pub use decode::{decoder_entierement, Decodeur};
pub use error::ErreurAudio;
pub use format::{FormatAudio, Profondeur};
pub use samples::Trames;
pub use wav_backend::BackendWav;
