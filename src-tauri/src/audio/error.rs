//! Les erreurs du moteur audio.
//!
//! Aucune variante n'a de valeur de repli. V1 a montré deux fois ce que coûte une
//! dégradation silencieuse : `option_env!()` qui rend `None` sans rien dire (INV-17) et
//! `unwrap_or_default()` sur un mot de passe (INV-04). En audio, la dégradation silencieuse
//! est pire encore, parce qu'elle ne se voit pas du tout : ça joue, un peu moins bien.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ErreurAudio {
    #[error("fichier illisible : {chemin} ({source})")]
    Lecture {
        chemin: PathBuf,
        source: std::io::Error,
    },

    #[error("format de fichier non reconnu : {chemin}")]
    FormatInconnu { chemin: PathBuf },

    #[error("aucune piste audio dans {chemin}")]
    AucunePiste { chemin: PathBuf },

    #[error("flux audio corrompu : {0}")]
    FluxCorrompu(String),

    /// Le conteneur annonce une profondeur qu'on refuse de deviner (voir
    /// `Profondeur::depuis_bits`). Échouer vaut mieux que jouer à la mauvaise résolution.
    #[error("profondeur de {bits} bits non prise en charge")]
    ProfondeurNonPriseEnCharge { bits: u32 },

    /// Ni le conteneur ni le premier paquet décodé ne donnent un format exploitable.
    /// ALAC et AAC ne déclarent pas leurs canaux (voir `decode.rs`) ; si même le paquet
    /// n'en porte pas, il n'y a rien à deviner.
    #[error("format indéterminable : {chemin}")]
    FormatIndeterminable { chemin: PathBuf },

    /// Le backend n'a pas pu servir le format demandé. Le bit-perfect interdit de
    /// rééchantillonner (ADR 0002), donc ce cas est une erreur et non un repli.
    #[error("le backend ne peut pas servir {demande} (il propose {obtenu})")]
    FormatRefuse { demande: String, obtenu: String },

    #[error("écriture impossible : {0}")]
    Ecriture(#[source] std::io::Error),
}
