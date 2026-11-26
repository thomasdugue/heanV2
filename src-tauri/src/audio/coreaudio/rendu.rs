//! Le callback de rendu, et l'état que le fil de lecture partage avec lui.
//!
//! ⚠️ **Ce fichier est le seul du projet dont le code tourne sur un fil temps réel.** Le
//! callback de CoreAudio n'alloue pas, ne verrouille pas, ne journalise pas : un blocage de
//! quelques millisecondes se traduit par un trou dans le son. Tout ce qu'il a à dire passe
//! par des compteurs atomiques, que `EtatDuFlux` expose au reste du moteur.
//!
//! Séparé de [`super::flux`], qui ne porte que la vie de l'AudioUnit (créer, démarrer,
//! arrêter, purger, libérer) : ce sont deux régimes d'exécution différents, et les mélanger
//! dans un fichier ferait perdre de vue lequel des deux on est en train de modifier.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use coreaudio_sys::{AudioBufferList, AudioTimeStamp, AudioUnitRenderActionFlags};

use super::tampon::Consommateur;
use crate::audio::chain::{Chaine, Volume};

/// Octets d'un échantillon sur le flux : on sort en entier 32 bits (ADR 0007).
pub(super) const OCTETS_PAR_ECHANTILLON: u32 = 4;

/// Trames que le tampon de travail du callback peut traiter d'un coup. CoreAudio demande
/// typiquement 512 trames, 4 096 au pire ; 16 384 laisse quatre fois la marge et coûte
/// 128 Ko. Au-delà, le callback sort du silence et compte à part plutôt que d'allouer sur le
/// fil temps réel.
pub(super) const TRAMES_MAX_PAR_CALLBACK: usize = 16_384;

/// Ce que le fil de lecture observe du flux pendant qu'il joue.
pub struct EtatDuFlux {
    /// Vrai quand la lecture est en pause : le callback sort du silence sans consommer.
    pub en_pause: AtomicBool,
    /// Le volume, en bits d'un `f32`. Lu à chaque callback, donc un changement est immédiat
    /// — et non après les secondes déjà dans le tampon.
    volume: AtomicU32,
    /// Nombre de callbacks où rien n'était disponible alors que la lecture tournait.
    ///
    /// **C'est la mesure de « ça ne coupe pas ».** Chaque famine est un trou audible. V1
    /// n'avait aucun compteur : une coupure ne laissait aucune trace, et « j'ai cru entendre
    /// un craquement » n'était pas vérifiable. Ici le chiffre remonte jusqu'à l'écran.
    pub famines: AtomicU64,
    /// Demande de purge du tampon, posée par le fil de lecture et honorée **par le
    /// callback** — parce que le consommateur du tampon vit dans le callback et nulle part
    /// ailleurs. C'est la moitié « notre tampon » du déplacement dans la piste ; l'autre
    /// moitié est `AudioUnitReset`, qui vide les ~50 ms déjà parties dans le HAL.
    pub purge_demandee: AtomicBool,
    /// Trames que le callback a demandées en trop d'un coup (voir
    /// [`TRAMES_MAX_PAR_CALLBACK`]). Doit rester à zéro ; s'il ne l'est pas, la constante est
    /// à relever, et ce n'est pas une famine ordinaire.
    pub demandes_trop_grandes: AtomicU64,
}

impl EtatDuFlux {
    pub(super) fn nouveau() -> Self {
        Self {
            en_pause: AtomicBool::new(false),
            volume: AtomicU32::new(1.0f32.to_bits()),
            purge_demandee: AtomicBool::new(false),
            famines: AtomicU64::new(0),
            demandes_trop_grandes: AtomicU64::new(0),
        }
    }

    pub fn definir_volume(&self, volume: Volume) {
        self.volume.store(volume.valeur().to_bits(), Ordering::Relaxed);
    }

    pub fn volume(&self) -> Volume {
        Volume::nouveau(f32::from_bits(self.volume.load(Ordering::Relaxed)))
    }
}

/// Ce que le callback reçoit par son `inputProcRefCon`. Possédé par une `Box` que
/// [`super::flux::Flux`] garde en vie aussi longtemps que l'AudioUnit.
pub(super) struct DonneesDuCallback {
    pub(super) consommateur: Consommateur,
    pub(super) canaux: usize,
    pub(super) etat: Arc<EtatDuFlux>,
    /// Pré-alloué une fois pour toutes : le callback ne doit jamais allouer.
    pub(super) travail: Vec<i32>,
}

/// Le callback de rendu. **Fil temps réel : aucune allocation, aucun verrou, aucun log.**
///
/// SÛRETÉ : `ref_con` est le pointeur installé par `Flux::ouvrir`, qui vise le contenu d'une
/// `Box` vivante aussi longtemps que l'AudioUnit. CoreAudio garantit que ce callback n'est
/// pas réentrant, donc l'accès mutable exclusif aux données est bien exclusif.
pub(super) unsafe extern "C" fn rendre(
    ref_con: *mut c_void,
    _drapeaux: *mut AudioUnitRenderActionFlags,
    _horodatage: *const AudioTimeStamp,
    _bus: u32,
    trames: u32,
    sortie: *mut AudioBufferList,
) -> i32 {
    let donnees = &mut *(ref_con as *mut DonneesDuCallback);
    let liste = &mut *sortie;
    let tampons = std::slice::from_raw_parts_mut(
        liste.mBuffers.as_mut_ptr(),
        liste.mNumberBuffers as usize,
    );

    let attendus = trames as usize * donnees.canaux;
    if attendus > donnees.travail.len() {
        // Plus de trames que le tampon de travail n'en tient. Allouer ici serait une faute
        // temps réel bien pire qu'un silence, donc on sort du silence et on le compte à part
        // des famines : la cause est une constante à relever, pas un décodeur en retard.
        donnees
            .etat
            .demandes_trop_grandes
            .fetch_add(1, Ordering::Relaxed);
        faire_silence(tampons);
        return 0;
    }

    if donnees.etat.purge_demandee.swap(false, Ordering::AcqRel) {
        // Jeter ce qui attend, et ne rien jouer ce tour-ci : les échantillons en attente
        // sont ceux d'avant le saut. `vider` ne fait qu'avancer deux compteurs — rien qui
        // puisse bloquer le fil temps réel.
        donnees.consommateur.vider();
        faire_silence(tampons);
        return 0;
    }

    if donnees.etat.en_pause.load(Ordering::Relaxed) {
        // En pause, on ne consomme rien : le tampon reste tel quel et la reprise est
        // instantanée.
        faire_silence(tampons);
        return 0;
    }

    let lus = donnees.consommateur.tirer(&mut donnees.travail[..attendus]);
    if lus < attendus {
        donnees.etat.famines.fetch_add(1, Ordering::Relaxed);
    }

    // LE CHEMIN BIT-PERFECT : à volume exactement 100 %, `appliquer` sort immédiatement et
    // les échantillons partent tels quels (voir `chain.rs`).
    Chaine::avec_volume(donnees.etat.volume()).appliquer(&mut donnees.travail[..lus]);

    ecrire_vers_coreaudio(tampons, &donnees.travail[..lus], donnees.canaux);
    0
}

/// SÛRETÉ : chaque `mData` est un tampon valide de `mDataByteSize` octets, aligné pour des
/// entiers 32 bits, fourni par CoreAudio pour ce callback.
unsafe fn faire_silence(tampons: &mut [coreaudio_sys::AudioBuffer]) {
    for t in tampons.iter_mut() {
        if t.mData.is_null() {
            continue;
        }
        std::ptr::write_bytes(t.mData as *mut u8, 0, t.mDataByteSize as usize);
    }
}

/// Recopie `echantillons` (entrelacés) vers les tampons de CoreAudio, et met le reste de
/// chaque tampon à **zéro**.
///
/// Les deux formes sont gérées comme en V1 : un tampon unique entrelacé — ce que l'AUHAL
/// donne pour le format déclaré ici — et le cas désentrelacé, un tampon par canal.
///
/// SÛRETÉ : mêmes garanties que `faire_silence`.
unsafe fn ecrire_vers_coreaudio(
    tampons: &mut [coreaudio_sys::AudioBuffer],
    echantillons: &[i32],
    canaux: usize,
) {
    if tampons.len() == 1 {
        let t = &mut tampons[0];
        if t.mData.is_null() {
            return;
        }
        let cases = t.mDataByteSize as usize / OCTETS_PAR_ECHANTILLON as usize;
        let destination = std::slice::from_raw_parts_mut(t.mData as *mut i32, cases);
        let combien = echantillons.len().min(cases);
        destination[..combien].copy_from_slice(&echantillons[..combien]);
        for case in destination[combien..].iter_mut() {
            *case = 0;
        }
        return;
    }

    let trames_lues = if canaux == 0 { 0 } else { echantillons.len() / canaux };
    for (canal, t) in tampons.iter_mut().enumerate().take(canaux) {
        if t.mData.is_null() {
            continue;
        }
        let cases = t.mDataByteSize as usize / OCTETS_PAR_ECHANTILLON as usize;
        let destination = std::slice::from_raw_parts_mut(t.mData as *mut i32, cases);
        // Tout le tampon est écrit, et ce qui dépasse les trames lues est mis à zéro.
        // Laisser la queue telle quelle rejouerait le contenu du callback précédent — un
        // bourdonnement, pas un silence.
        for (trame, case) in destination.iter_mut().enumerate() {
            *case = if trame < trames_lues {
                echantillons[trame * canaux + canal]
            } else {
                0
            };
        }
    }
}
