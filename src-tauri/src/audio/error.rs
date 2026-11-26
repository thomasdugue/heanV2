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

    /// Le HAL de macOS a refusé une opération. Le code est l'`OSStatus` brut : il ne se
    /// traduit pas en français, et le donner tel quel est ce qui permet de le chercher.
    #[error("CoreAudio a refusé {operation} (code {code})")]
    CoreAudio { operation: String, code: i32 },

    #[error("aucun périphérique de sortie audio")]
    AucunPeripherique,

    /// Le mode exclusif est tenu par un autre processus. Message destiné à Thomas : il doit
    /// pouvoir savoir quoi fermer, donc le PID en fait partie.
    #[error("« {nom} » est réservé par un autre programme (PID {pid}) — le fermer, ou jouer en mode partagé")]
    PeripheriqueReserve { nom: String, pid: i32 },

    /// Le périphérique ne sait pas servir la fréquence du fichier, et Hean ne rééchantillonne
    /// pas (ADR 0002, et ADR 0018 pour ce que ça donne à l’écran). C'est la correction du
    /// défaut de V1 relevé dans `docs/MESURES-V1.md` §11 : V1 jouait quand même, à une autre
    /// fréquence, sans rien dire. Ce texte-ci est lu par Thomas : il nomme le périphérique,
    /// la fréquence qui manque et celles qui existent.
    #[error("« {peripherique} » ne sait pas jouer ce fichier à sa fréquence native ({demandee} Hz) ; il ne propose que {proposees}. Hean ne rééchantillonne pas : la lecture est refusée plutôt que dégradée en silence.")]
    FrequenceNonServie {
        peripherique: String,
        demandee: u32,
        proposees: String,
    },

    #[error("écriture impossible : {0}")]
    Ecriture(#[source] std::io::Error),
}
