//! Le mode exclusif — *hog mode*. L'une des quatre conditions du bit-perfect.
//!
//! Porté de `get_hog_mode_pid`, `enable_hog_mode_internal` et `disable_hog_mode_internal`
//! de V1. `kAudioDevicePropertyHogMode` porte le PID du processus qui s'est réservé le
//! périphérique, ou `-1` si personne ne l'a fait.
//!
//! ⚠️ **Jamais sur un AirPlay** : le mode exclusif est incompatible avec une diffusion
//! réseau. Le refus est dans `backend.rs`, là où le périphérique actif est connu.

use coreaudio_sys::{kAudioDevicePropertyHogMode, AudioObjectID};

use super::hal;
use crate::audio::error::ErreurAudio;

/// Personne ne tient le périphérique.
pub const LIBRE: i32 = -1;

fn notre_pid() -> i32 {
    std::process::id() as i32
}

/// Le PID du processus qui s'est réservé le périphérique, ou [`LIBRE`].
pub fn pid_du_mode_exclusif(id: AudioObjectID) -> Result<i32, ErreurAudio> {
    let adr = hal::globale(kAudioDevicePropertyHogMode);
    hal::lire(id, &adr, LIBRE, "lire le détenteur du mode exclusif")
}

/// Réserve le périphérique pour Hean, **puis relit pour vérifier**.
///
/// La relecture est ce qui distingue cette fonction d'une promesse : V1 l'avait déjà, et
/// c'était juste — écrire le PID peut rendre `0` sans que la réservation ait eu lieu.
pub fn activer_mode_exclusif(id: AudioObjectID, nom: &str) -> Result<(), ErreurAudio> {
    let nous = notre_pid();
    let detenteur = pid_du_mode_exclusif(id).unwrap_or(LIBRE);
    if detenteur == nous {
        return Ok(());
    }
    if detenteur != LIBRE {
        // On tente quand même : macOS accorde la réservation si l'autre processus l'a
        // abandonnée sans la relâcher proprement, ce qui arrive après un plantage.
        tracing::warn!(
            peripherique = %nom,
            detenteur,
            "le mode exclusif est déjà tenu — tentative de reprise"
        );
    }

    let adr = hal::globale(kAudioDevicePropertyHogMode);
    if let Err(e) = hal::ecrire(id, &adr, &nous, "réserver le périphérique") {
        if detenteur != LIBRE {
            return Err(ErreurAudio::PeripheriqueReserve {
                nom: nom.to_owned(),
                pid: detenteur,
            });
        }
        return Err(e);
    }

    let verifie = pid_du_mode_exclusif(id).unwrap_or(LIBRE);
    if verifie != nous {
        return Err(ErreurAudio::PeripheriqueReserve {
            nom: nom.to_owned(),
            pid: verifie,
        });
    }
    tracing::info!(peripherique = %nom, pid = nous, "mode exclusif actif");
    Ok(())
}

/// Relâche la réservation. Écrire [`LIBRE`] est la façon dont le HAL l'exprime.
///
/// Appelé aussi sur l'**ancien** périphérique avant tout changement : un périphérique
/// débranché alors qu'on le tenait encore refuse ensuite de se rouvrir.
pub fn desactiver_mode_exclusif(id: AudioObjectID) -> Result<(), ErreurAudio> {
    let adr = hal::globale(kAudioDevicePropertyHogMode);
    hal::ecrire(id, &adr, &LIBRE, "relâcher le périphérique")?;
    tracing::debug!(peripherique = id, "mode exclusif relâché");
    Ok(())
}
