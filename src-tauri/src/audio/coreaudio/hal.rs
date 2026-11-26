//! Les six accès au HAL de macOS, et rien d'autre dans tout le projet.
//!
//! V1 répétait une trentaine de lignes d'`unsafe` par propriété lue — douze fois dans
//! `coreaudio_backend.rs`, pour un fichier de 1 326 lignes. Ce n'est pas tenable sous la
//! limite de 400 lignes, et ce n'était de toute façon pas souhaitable : chaque copie était
//! une occasion d'oublier un test du code de retour, et V1 en oubliait (le `status` de
//! `get_device_transport_type` n'était jamais lu).
//!
//! Les fonctions ci-dessous sont **les seules** à appeler `AudioObject*`, et toutes
//! vérifient le code de retour. Le découpage ne change aucun comportement : c'est la même
//! séquence d'appels, écrite une fois.

use std::ffi::c_void;
use std::mem;

use coreaudio_sys::{
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject,
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID,
    AudioObjectPropertyAddress, AudioObjectSetPropertyData,
};

use crate::audio::error::ErreurAudio;

/// L'objet qui représente le système lui-même : c'est lui qui porte la liste des
/// périphériques et le périphérique de sortie par défaut.
pub const SYSTEME: AudioObjectID = kAudioObjectSystemObject as AudioObjectID;

/// `kAudioObjectPropertyElementMain` partout : aucune propriété lue ici n'est par canal.
pub fn adresse(selecteur: u32, portee: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selecteur,
        mScope: portee,
        mElement: kAudioObjectPropertyElementMain,
    }
}

pub fn globale(selecteur: u32) -> AudioObjectPropertyAddress {
    adresse(selecteur, kAudioObjectPropertyScopeGlobal)
}

fn verifier(operation: &str, code: i32) -> Result<(), ErreurAudio> {
    if code == 0 {
        Ok(())
    } else {
        Err(ErreurAudio::CoreAudio {
            operation: operation.to_owned(),
            code,
        })
    }
}

/// Lit une propriété de taille fixe : un nombre, un identifiant de périphérique, un
/// pointeur `CFStringRef`.
///
/// `defaut` sert à initialiser la variable que le HAL va remplir — il n'est **pas** une
/// valeur de repli : si l'appel échoue, la fonction rend une erreur et jamais `defaut`.
pub fn lire<T: Copy>(
    objet: AudioObjectID,
    adr: &AudioObjectPropertyAddress,
    defaut: T,
    operation: &str,
) -> Result<T, ErreurAudio> {
    let mut valeur = defaut;
    let mut taille = mem::size_of::<T>() as u32;
    // SÛRETÉ : `valeur` est un `T` vivant et `taille` dit exactement sa taille ; le HAL
    // n'écrit pas au-delà de ce qu'on lui annonce.
    let code = unsafe {
        AudioObjectGetPropertyData(
            objet,
            adr,
            0,
            std::ptr::null(),
            &mut taille,
            &mut valeur as *mut T as *mut c_void,
        )
    };
    verifier(operation, code)?;
    Ok(valeur)
}

/// La taille en octets d'une propriété de longueur variable. Toujours demandée **avant** de
/// lire : c'est elle qui dit combien d'éléments allouer.
pub fn taille(
    objet: AudioObjectID,
    adr: &AudioObjectPropertyAddress,
    operation: &str,
) -> Result<u32, ErreurAudio> {
    let mut octets: u32 = 0;
    // SÛRETÉ : `octets` est un `u32` vivant ; cet appel n'écrit rien d'autre.
    let code =
        unsafe { AudioObjectGetPropertyDataSize(objet, adr, 0, std::ptr::null(), &mut octets) };
    verifier(operation, code)?;
    Ok(octets)
}

/// Lit une propriété de longueur variable comme un tableau de `T` : la liste des
/// périphériques (`AudioObjectID`), les plages de fréquences (`AudioValueRange`).
pub fn lire_tableau<T: Copy + Default>(
    objet: AudioObjectID,
    adr: &AudioObjectPropertyAddress,
    operation: &str,
) -> Result<Vec<T>, ErreurAudio> {
    let mut octets = taille(objet, adr, operation)?;
    let combien = octets as usize / mem::size_of::<T>();
    let mut valeurs: Vec<T> = vec![T::default(); combien];
    if combien == 0 {
        return Ok(valeurs);
    }
    // SÛRETÉ : le tampon fait `combien * size_of::<T>()` octets, soit exactement ce
    // qu'annonce `octets`, que le HAL vient lui-même de calculer.
    let code = unsafe {
        AudioObjectGetPropertyData(
            objet,
            adr,
            0,
            std::ptr::null(),
            &mut octets,
            valeurs.as_mut_ptr() as *mut c_void,
        )
    };
    verifier(operation, code)?;
    // Le HAL peut rendre MOINS que la taille annoncée entre les deux appels (un
    // périphérique débranché pendant ce temps). Tronquer évite de lire des `T::default()`
    // pris pour des données réelles — un périphérique d'identifiant 0, par exemple.
    valeurs.truncate(octets as usize / mem::size_of::<T>());
    Ok(valeurs)
}

/// Lit une propriété de longueur variable dans un tampon d'octets brut.
///
/// Sert à `AudioBufferList`, dont la taille dépend du nombre de flux du périphérique et
/// qu'on ne peut donc pas lire comme un tableau d'un type de taille connue.
pub fn lire_octets(
    objet: AudioObjectID,
    adr: &AudioObjectPropertyAddress,
    operation: &str,
) -> Result<Vec<u8>, ErreurAudio> {
    let mut octets = taille(objet, adr, operation)?;
    if octets == 0 {
        return Ok(Vec::new());
    }
    let mut tampon = vec![0u8; octets as usize];
    // SÛRETÉ : `tampon` fait exactement `octets` octets.
    let code = unsafe {
        AudioObjectGetPropertyData(
            objet,
            adr,
            0,
            std::ptr::null(),
            &mut octets,
            tampon.as_mut_ptr() as *mut c_void,
        )
    };
    verifier(operation, code)?;
    tampon.truncate(octets as usize);
    Ok(tampon)
}

/// Écrit une propriété.
///
/// ⚠️ Un `Ok(())` ici ne veut **pas** dire que le réglage a pris effet : c'est tout le sujet
/// de PLAT-1 (🔴), où `kAudioHardwarePropertyDefaultOutputDevice` sur un périphérique AirPlay
/// périmé est accepté sans rien faire. Toute écriture dont l'effet compte se relit — voir
/// `frequence::definir` et `exclusif::activer`, qui le font tous les deux.
pub fn ecrire<T>(
    objet: AudioObjectID,
    adr: &AudioObjectPropertyAddress,
    valeur: &T,
    operation: &str,
) -> Result<(), ErreurAudio> {
    // SÛRETÉ : on annonce la taille exacte de `*valeur`, que le HAL lit sans la modifier.
    let code = unsafe {
        AudioObjectSetPropertyData(
            objet,
            adr,
            0,
            std::ptr::null(),
            mem::size_of::<T>() as u32,
            valeur as *const T as *const c_void,
        )
    };
    verifier(operation, code)
}
