//! Décodage d'un fichier audio vers des trames `i32` mises à l'échelle 32 bits.
//!
//! Le décodeur rend des **blocs** (`suivant()`), parce que c'est ainsi qu'un moteur de
//! lecture s'en sert : on ne charge pas un album de 24/192 en mémoire. `decoder_entierement`
//! n'est qu'une commodité pour les tests et les vérifications.
//!
//! Un paquet indécodable fait **échouer** le décodage au lieu d'être sauté. Sauter
//! produirait du bruit en silence — exactement la dégradation invisible que ce projet
//! refuse. La fixture `corrupt.flac` existe pour que ce choix soit vérifié.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use symphonia::core::audio::SignalSpec;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as ErreurSymphonia;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::units::Time;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::convert::ajouter_entrelace;
use super::error::ErreurAudio;
use super::format::{FormatAudio, Profondeur};
use super::samples::Trames;

/// Un décodeur positionné sur un fichier, qui rend ses trames bloc par bloc.
pub struct Decodeur {
    lecteur: Box<dyn FormatReader>,
    codec: Box<dyn Decoder>,
    piste: u32,
    format: FormatAudio,
    chemin: PathBuf,
    /// Le premier bloc, quand il a fallu le décoder pour connaître le format (voir
    /// `ouvrir`). Il est rendu tel quel au premier `suivant()` : le décoder pour inspecter
    /// le flux ne doit pas faire perdre son audio.
    en_attente: Option<Trames>,
    /// La durée annoncée par le conteneur, quand il l'annonce.
    duree: Option<Duration>,
}

/// Le prochain bloc décodé de la piste `piste`, ou `None` à la fin du flux.
///
/// Fonction libre et non méthode : `ouvrir` en a besoin **avant** que le `Decodeur` existe,
/// parce que certains conteneurs ne livrent leur format qu'après un premier décodage.
fn prochain_bloc(
    lecteur: &mut dyn FormatReader,
    codec: &mut dyn Decoder,
    piste: u32,
    chemin: &Path,
) -> Result<Option<(SignalSpec, Vec<i32>)>, ErreurAudio> {
    loop {
        let paquet = match lecteur.next_packet() {
            Ok(p) => p,
            // Fin de flux. Symphonia la signale par une erreur d'E/S, pas par `None`.
            Err(ErreurSymphonia::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(None)
            }
            Err(e) => return Err(ErreurAudio::FluxCorrompu(e.to_string())),
        };
        if paquet.track_id() != piste {
            continue;
        }

        let tampon = match codec.decode(&paquet) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    chemin = %chemin.display(),
                    erreur = %e,
                    "paquet indécodable — le décodage s'arrête au lieu de jouer du bruit"
                );
                return Err(ErreurAudio::FluxCorrompu(e.to_string()));
            }
        };

        // Un paquet vide est licite (le premier paquet Ogg/Vorbis en est un) : on continue
        // au lieu de rendre un bloc vide, qu'un appelant prendrait pour une fin de flux.
        if tampon.frames() == 0 {
            continue;
        }
        let spec = *tampon.spec();
        let mut echantillons = Vec::new();
        ajouter_entrelace(&tampon, &mut echantillons);
        return Ok(Some((spec, echantillons)));
    }
}

impl Decodeur {
    /// Ouvre un fichier et lit son format sans décoder d'audio.
    pub fn ouvrir(chemin: &Path) -> Result<Self, ErreurAudio> {
        let fichier = File::open(chemin).map_err(|source| ErreurAudio::Lecture {
            chemin: chemin.to_path_buf(),
            source,
        })?;
        let flux = MediaSourceStream::new(Box::new(fichier), Default::default());

        // L'extension n'est qu'un indice : symphonia sonde le contenu. Un fichier mal
        // nommé se décode donc quand même, ce qui est le bon comportement pour une
        // bibliothèque musicale réelle.
        let mut indice = Hint::new();
        if let Some(ext) = chemin.extension().and_then(|e| e.to_str()) {
            indice.with_extension(ext);
        }

        let sonde = symphonia::default::get_probe()
            .format(
                &indice,
                flux,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|_| ErreurAudio::FormatInconnu {
                chemin: chemin.to_path_buf(),
            })?;
        let lecteur = sonde.format;

        // La PREMIÈRE PISTE AUDIO, et non `default_track()` : un M4A à pochette embarquée
        // expose aussi une piste image, et `default_track()` pourrait la désigner.
        let piste = lecteur
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| ErreurAudio::AucunePiste {
                chemin: chemin.to_path_buf(),
            })?
            .clone();

        let p = &piste.codec_params;

        // Pas de profondeur déclarée : les formats avec perte, et ALAC. Voir
        // `Profondeur::SANS_DECLARATION` — il n'y a pas d'échantillon d'origine à préserver.
        let profondeur = match p.bits_per_sample {
            Some(bits) => Profondeur::depuis_bits(bits)
                .ok_or(ErreurAudio::ProfondeurNonPriseEnCharge { bits })?,
            None => Profondeur::SANS_DECLARATION,
        };

        let mut codec = symphonia::default::get_codecs()
            .make(p, &DecoderOptions::default())
            .map_err(|e| ErreurAudio::FluxCorrompu(e.to_string()))?;

        // ALAC et AAC ne déclarent PAS leur nombre de canaux dans le conteneur : mesuré sur
        // `fixtures/audio/alac.m4a` et `aac.m4a`, où `codec_params.channels` vaut `None`.
        // L'information n'existe qu'après le décodage d'un premier paquet. Refuser le
        // fichier serait perdre deux des sept formats annoncés ; deviner « stéréo » serait
        // jouer un mono sur un seul canal sans le dire. On décode donc, et on garde le bloc.
        let mut lecteur = lecteur;
        let (format, en_attente) = match (p.sample_rate, p.channels) {
            (Some(frequence), Some(canaux)) => (
                FormatAudio::nouveau(frequence, canaux.count() as u16, profondeur),
                None,
            ),
            _ => {
                let (spec, echantillons) =
                    prochain_bloc(&mut *lecteur, &mut *codec, piste.id, chemin)?.ok_or_else(
                        || ErreurAudio::AucunePiste {
                            chemin: chemin.to_path_buf(),
                        },
                    )?;
                let format =
                    FormatAudio::nouveau(spec.rate, spec.channels.count() as u16, profondeur);
                (format, Some(Trames::nouveau(format, echantillons)))
            }
        };

        if format.frequence == 0 || format.canaux == 0 {
            return Err(ErreurAudio::FormatIndeterminable {
                chemin: chemin.to_path_buf(),
            });
        }

        // La durée vient du conteneur : nombre de trames / fréquence. `n_frames` manque sur
        // un MP3 sans en-tête Xing, et on ne la devine pas à partir de la taille du fichier
        // — un débit variable rendrait l'estimation fausse de plusieurs dizaines de secondes.
        let duree = p
            .n_frames
            .filter(|_| format.frequence > 0)
            .map(|trames| Duration::from_secs_f64(trames as f64 / f64::from(format.frequence)));

        tracing::debug!(
            chemin = %chemin.display(),
            format = %format,
            format_declare = en_attente.is_none(),
            duree_s = duree.map(|d| d.as_secs()),
            "fichier audio ouvert"
        );

        Ok(Self {
            lecteur,
            codec,
            piste: piste.id,
            format,
            chemin: chemin.to_path_buf(),
            en_attente,
            duree,
        })
    }

    /// Le format du flux, tel que le conteneur le déclare.
    pub fn format(&self) -> FormatAudio {
        self.format
    }

    /// La durée annoncée par le conteneur, quand il l'annonce.
    ///
    /// `None` est un cas réel et non une négligence : un MP3 sans en-tête Xing, un flux
    /// Ogg tronqué. L'interface doit savoir afficher « durée inconnue » plutôt que zéro —
    /// une barre de progression calée sur zéro est pire qu'absente.
    pub fn duree(&self) -> Option<Duration> {
        self.duree
    }

    /// Déplace la lecture à `position`, et rend la position **réellement atteinte**.
    ///
    /// Le retour n'est pas décoratif : symphonia se déplace à la trame de départ du paquet
    /// qui contient `position`, donc l'arrivée est en général un peu avant la demande. Un
    /// appelant qui afficherait la position demandée plutôt que l'atteinte verrait sa barre
    /// de progression reculer d'elle-même à la première mise à jour.
    ///
    /// `SeekMode::Accurate` et non `Coarse` : le déplacement grossier se contente du point
    /// d'index le plus proche, ce qui sur un FLAC sans table de recherche peut rater de
    /// plusieurs secondes. Le coût est un décodage partiel du paquet d'arrivée, soit
    /// quelques millisecondes.
    pub fn aller_a(&mut self, position: Duration) -> Result<Duration, ErreurAudio> {
        // Un bloc gardé de l'ouverture n'a plus rien à voir avec la nouvelle position.
        self.en_attente = None;
        let atteint = self
            .lecteur
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::from(position.as_secs_f64()),
                    track_id: Some(self.piste),
                },
            )
            .map_err(|e| ErreurAudio::FluxCorrompu(e.to_string()))?;
        // Obligatoire après un déplacement : le décodeur garde l'état de la trame
        // précédente, et décoder la suivante sans réinitialiser produit un craquement.
        self.codec.reset();
        if self.format.frequence == 0 {
            return Ok(position);
        }
        Ok(Duration::from_secs_f64(
            atteint.actual_ts as f64 / f64::from(self.format.frequence),
        ))
    }

    /// Le bloc de trames suivant, ou `None` à la fin du fichier.
    pub fn suivant(&mut self) -> Result<Option<Trames>, ErreurAudio> {
        // Le bloc décodé à l'ouverture pour connaître le format, s'il y en a un : l'avoir
        // décodé pour inspecter le flux ne doit pas faire perdre son audio.
        if let Some(bloc) = self.en_attente.take() {
            return Ok(Some(bloc));
        }
        let bloc = prochain_bloc(&mut *self.lecteur, &mut *self.codec, self.piste, &self.chemin)?;
        Ok(bloc.map(|(_, echantillons)| Trames::nouveau(self.format, echantillons)))
    }
}

/// Décode un fichier en entier. Commodité pour les tests et les vérifications ; le moteur
/// de lecture, lui, consomme `Decodeur::suivant()`.
pub fn decoder_entierement(chemin: &Path) -> Result<Trames, ErreurAudio> {
    let mut d = Decodeur::ouvrir(chemin)?;
    let format = d.format();
    let mut tout = Vec::new();
    while let Some(bloc) = d.suivant()? {
        tout.extend_from_slice(&bloc.echantillons);
    }
    Ok(Trames::nouveau(format, tout))
}
