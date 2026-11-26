//! La vie de l'AudioUnit : l'ouvrir sur le bon périphérique, la démarrer, la purger, la
//! libérer. Le callback de rendu, lui, est dans [`super::rendu`].
//!
//! Porté de `coreaudio_stream.rs` de V1, avec **une seule différence de fond**, imposée par
//! l'ADR 0007 : V1 configurait l'AudioUnit en `kAudioFormatFlagsNativeFloatPacked` et
//! poussait des `f32`. Le format interne de V2 est `i32` mis à l'échelle sur 32 bits, donc
//! le flux est déclaré en **entier signé 32 bits**. Ce n'est pas une amélioration
//! optionnelle : donner des `i32` à un flux déclaré flottant ferait interpréter les mêmes
//! bits comme des nombres à virgule, c'est-à-dire du bruit blanc à pleine échelle.
//!
//! Conséquence heureuse, et c'est le chemin bit-perfect : quand le DAC travaille en 24 bits,
//! la conversion que fait l'AUHAL depuis notre 32 bits est un **décalage à droite**, donc
//! exacte pour toute source de 24 bits ou moins. Aucune fréquence n'est touchée ici : elle a
//! été négociée avant, et un écart éventuel est déjà parti vers l'écran (voir
//! [`super::negociation`]).

use std::ffi::c_void;
use std::mem;
use std::sync::Arc;

use coreaudio_sys::{
    kAudioFormatFlagIsPacked, kAudioFormatFlagIsSignedInteger, kAudioFormatLinearPCM,
    kAudioOutputUnitProperty_CurrentDevice, kAudioUnitManufacturer_Apple,
    kAudioUnitProperty_SetRenderCallback, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Global,
    kAudioUnitScope_Input, kAudioUnitSubType_HALOutput, kAudioUnitType_Output,
    AURenderCallbackStruct, AudioComponentDescription, AudioComponentFindNext,
    AudioComponentInstanceDispose, AudioComponentInstanceNew, AudioObjectID, AudioOutputUnitStart,
    AudioOutputUnitStop, AudioStreamBasicDescription, AudioUnit as AudioUnitSys,
    AudioUnitInitialize, AudioUnitReset, AudioUnitSetProperty, AudioUnitUninitialize,
};

use super::rendu::{
    rendre, DonneesDuCallback, EtatDuFlux, OCTETS_PAR_ECHANTILLON, TRAMES_MAX_PAR_CALLBACK,
};
use super::tampon::Consommateur;
use crate::audio::error::ErreurAudio;
use crate::audio::format::FormatAudio;

/// Un flux ouvert vers un périphérique.
pub struct Flux {
    unite: AudioUnitSys,
    format: FormatAudio,
    etat: Arc<EtatDuFlux>,
    demarre: bool,
    /// Gardé en vie : le callback détient un pointeur brut vers son contenu.
    _donnees: Box<DonneesDuCallback>,
}

// SÛRETÉ : `Flux` est déplacé entre le fil qui le crée et le fil de lecture, mais jamais
// partagé — tout le partage passe par `Arc<EtatDuFlux>`, dont les champs sont atomiques.
// L'`AudioUnit` est un pointeur opaque que CoreAudio protège lui-même.
unsafe impl Send for Flux {}

impl Flux {
    /// Ouvre un AudioUnit `HALOutput` sur `peripherique` (ou le défaut système si `None`) et
    /// le configure pour `format`.
    ///
    /// `peripherique` vaut `None` pour un AirPlay, et c'est obligatoire :
    /// `AudioUnitSetProperty(kAudioOutputUnitProperty_CurrentDevice)` échoue systématiquement
    /// sur un AirPlay, qui ne s'atteint que par le défaut système (PLAT-1).
    pub fn ouvrir(
        peripherique: Option<AudioObjectID>,
        format: FormatAudio,
        consommateur: Consommateur,
    ) -> Result<Self, ErreurAudio> {
        let canaux = u32::from(format.canaux);
        let description = AudioComponentDescription {
            componentType: kAudioUnitType_Output,
            // `HALOutput` et non `DefaultOutput` : c'est le seul sous-type qui accepte qu'on
            // lui désigne un périphérique.
            componentSubType: kAudioUnitSubType_HALOutput,
            componentManufacturer: kAudioUnitManufacturer_Apple,
            componentFlags: 0,
            componentFlagsMask: 0,
        };

        // SÛRETÉ : `description` est valide et `AudioComponentFindNext` ne la conserve pas.
        let composant = unsafe { AudioComponentFindNext(std::ptr::null_mut(), &description) };
        if composant.is_null() {
            return Err(ErreurAudio::CoreAudio {
                operation: "trouver le composant de sortie HAL".to_owned(),
                code: 0,
            });
        }

        let mut unite: AudioUnitSys = std::ptr::null_mut();
        // SÛRETÉ : `unite` est un pointeur que l'appel initialise.
        let code = unsafe { AudioComponentInstanceNew(composant, &mut unite) };
        if code != 0 || unite.is_null() {
            return Err(ErreurAudio::CoreAudio {
                operation: "instancier l'unité de sortie".to_owned(),
                code,
            });
        }

        // À partir d'ici l'unité existe : toute sortie en erreur doit la libérer. `Partielle`
        // s'en charge par son `Drop`, y compris si un `?` part au milieu.
        let mut partielle = Partielle(unite);

        if let Some(id) = peripherique {
            definir_propriete(
                unite,
                kAudioOutputUnitProperty_CurrentDevice,
                kAudioUnitScope_Global,
                &id,
                "désigner le périphérique de sortie de l'unité",
            )?;
        }

        let asbd = AudioStreamBasicDescription {
            mSampleRate: f64::from(format.frequence),
            mFormatID: kAudioFormatLinearPCM,
            // Entier signé, empaqueté, boutisme natif. **Pas** `NativeFloatPacked` comme en
            // V1 : le format interne de V2 est `i32` (ADR 0007).
            mFormatFlags: kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked,
            mBytesPerPacket: OCTETS_PAR_ECHANTILLON * canaux,
            mFramesPerPacket: 1,
            mBytesPerFrame: OCTETS_PAR_ECHANTILLON * canaux,
            mChannelsPerFrame: canaux,
            mBitsPerChannel: OCTETS_PAR_ECHANTILLON * 8,
            mReserved: 0,
        };
        definir_propriete(
            unite,
            kAudioUnitProperty_StreamFormat,
            kAudioUnitScope_Input,
            &asbd,
            "déclarer le format du flux",
        )?;

        let etat = Arc::new(EtatDuFlux::nouveau());
        let donnees = Box::new(DonneesDuCallback {
            consommateur,
            canaux: format.canaux as usize,
            etat: Arc::clone(&etat),
            travail: vec![0i32; TRAMES_MAX_PAR_CALLBACK * format.canaux as usize],
            amorti: 0,
        });

        let rappel = AURenderCallbackStruct {
            inputProc: Some(rendre),
            // Le pointeur vise le contenu de la `Box`, qui ne bouge pas quand la `Box` est
            // déplacée dans `Flux`. Elle est gardée en vie par le champ `_donnees`.
            inputProcRefCon: &*donnees as *const DonneesDuCallback as *mut c_void,
        };
        definir_propriete(
            unite,
            kAudioUnitProperty_SetRenderCallback,
            kAudioUnitScope_Input,
            &rappel,
            "installer le callback de rendu",
        )?;

        // SÛRETÉ : l'unité est configurée ; `AudioUnitInitialize` ne prend rien d'autre.
        let code = unsafe { AudioUnitInitialize(unite) };
        if code != 0 {
            return Err(ErreurAudio::CoreAudio {
                operation: "initialiser l'unité de sortie".to_owned(),
                code,
            });
        }

        partielle.0 = std::ptr::null_mut(); // l'unité est désormais possédée par `Flux`
        tracing::info!(
            peripherique = peripherique.unwrap_or(0),
            format = %format,
            "flux CoreAudio ouvert"
        );
        Ok(Self {
            unite,
            format,
            etat,
            demarre: false,
            _donnees: donnees,
        })
    }

    pub fn etat(&self) -> Arc<EtatDuFlux> {
        Arc::clone(&self.etat)
    }

    pub fn format(&self) -> FormatAudio {
        self.format
    }

    pub fn demarrer(&mut self) -> Result<(), ErreurAudio> {
        if self.demarre {
            return Ok(());
        }
        // SÛRETÉ : l'unité est initialisée.
        let code = unsafe { AudioOutputUnitStart(self.unite) };
        if code != 0 {
            return Err(ErreurAudio::CoreAudio {
                operation: "démarrer la sortie".to_owned(),
                code,
            });
        }
        self.demarre = true;
        Ok(())
    }

    pub fn arreter(&mut self) -> Result<(), ErreurAudio> {
        if !self.demarre {
            return Ok(());
        }
        // SÛRETÉ : l'unité est démarrée.
        let code = unsafe { AudioOutputUnitStop(self.unite) };
        self.demarre = false;
        if code != 0 {
            return Err(ErreurAudio::CoreAudio {
                operation: "arrêter la sortie".to_owned(),
                code,
            });
        }
        Ok(())
    }

    /// Vide les tampons internes de CoreAudio — environ 50 ms de son déjà parti.
    ///
    /// C'est **la** pièce du déplacement instantané dans la piste, et la raison pour laquelle
    /// V1 a quitté CPAL : sans accès direct à l'AudioUnit, ces 50 ms d'ancien son se jouent
    /// après le saut et s'entendent comme un hoquet.
    pub fn purger(&mut self) -> Result<(), ErreurAudio> {
        // SÛRETÉ : l'unité est initialisée ; `AudioUnitReset` ne touche qu'à ses tampons.
        let code = unsafe { AudioUnitReset(self.unite, kAudioUnitScope_Global, 0) };
        if code != 0 {
            return Err(ErreurAudio::CoreAudio {
                operation: "purger les tampons de l'unité".to_owned(),
                code,
            });
        }
        Ok(())
    }
}

impl Drop for Flux {
    fn drop(&mut self) {
        // SÛRETÉ : séquence de libération exigée par CoreAudio, dans cet ordre. Les codes de
        // retour sont ignorés : on est dans un `Drop`, il n'y a plus personne à prévenir.
        unsafe {
            AudioOutputUnitStop(self.unite);
            AudioUnitUninitialize(self.unite);
            AudioComponentInstanceDispose(self.unite);
        }
        tracing::debug!("flux CoreAudio fermé");
    }
}

/// Une unité créée mais pas encore possédée par un [`Flux`]. Son `Drop` la libère, ce qui
/// évite une fuite sur chacun des cinq chemins d'erreur de `Flux::ouvrir` — V1 appelait
/// `AudioComponentInstanceDispose` à la main à chaque sortie, et en oubliait deux.
struct Partielle(AudioUnitSys);

impl Drop for Partielle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SÛRETÉ : l'unité a été créée et n'a pas été initialisée avec succès.
            unsafe { AudioComponentInstanceDispose(self.0) };
        }
    }
}

fn definir_propriete<T>(
    unite: AudioUnitSys,
    propriete: u32,
    portee: u32,
    valeur: &T,
    operation: &str,
) -> Result<(), ErreurAudio> {
    // SÛRETÉ : on annonce la taille exacte de `*valeur`, que l'appel lit sans la garder.
    let code = unsafe {
        AudioUnitSetProperty(
            unite,
            propriete,
            portee,
            0,
            valeur as *const T as *const c_void,
            mem::size_of::<T>() as u32,
        )
    };
    if code != 0 {
        // Pas de repli silencieux : V1 avait appris qu'un périphérique refusé puis ignoré
        // fait sortir le son sur le mauvais matériel sans que personne le sache.
        return Err(ErreurAudio::CoreAudio {
            operation: operation.to_owned(),
            code,
        });
    }
    Ok(())
}

