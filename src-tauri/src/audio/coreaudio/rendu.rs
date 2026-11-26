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
    /// Nombre de callbacks où rien n'était disponible **alors que le décodeur devait
    /// suivre**.
    ///
    /// **C'est la mesure de « ça ne coupe pas ».** Chaque famine est un trou audible. V1
    /// n'avait aucun compteur : une coupure ne laissait aucune trace, et « j'ai cru entendre
    /// un craquement » n'était pas vérifiable. Ici le chiffre remonte jusqu'à l'écran.
    ///
    /// ⚠️ **Un compteur d'anomalie ne compte que des anomalies**, et c'est tout l'objet de
    /// `fin_de_flux` et de l'amortissement après purge ci-dessous. Sans eux, chaque fin
    /// d'album et chaque déplacement dans la piste en ajoutaient — dans le chemin normal.
    /// Un indicateur qui crie au loup est désarmé par celui qui le lit, et l'aveuglement
    /// revient : c'est le raisonnement d'INV-21 appliqué à une mesure. Relevé par
    /// `invariant-guardian` le 2026-11-26.
    pub famines: AtomicU64,
    /// Posé quand il n'y a plus rien à décoder : les callbacks qui restent à servir d'ici
    /// l'arrêt de l'AudioUnit tombent légitimement à vide, et n'ont rien d'une famine.
    pub fin_de_flux: AtomicBool,
    /// Callbacks où CoreAudio a donné autre chose qu'un tampon entrelacé unique. Doit rester
    /// à zéro : le format déclaré par `flux.rs` est entrelacé, donc l'AUHAL n'a aucune raison
    /// d'en donner plusieurs. S'il le fait, notre hypothèse est fausse et il vaut mieux un
    /// silence compté qu'un son dont les canaux seraient intervertis.
    pub tampons_inattendus: AtomicU64,
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
            fin_de_flux: AtomicBool::new(false),
            tampons_inattendus: AtomicU64::new(0),
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
    /// Vrai depuis une purge et jusqu'au premier callback entièrement servi : le tampon a
    /// été vidé exprès, donc les callbacks à vide qui suivent sont attendus et non comptés.
    /// Un seul callback d'amortissement ne suffirait pas — après un déplacement, le décodeur
    /// met plusieurs dizaines de millisecondes à refaire de l'avance.
    pub(super) amorti: bool,
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
        donnees.amorti = true;
        faire_silence(tampons);
        return 0;
    }

    if donnees.etat.en_pause.load(Ordering::Relaxed) {
        // En pause, on ne consomme rien : le tampon reste tel quel et la reprise est
        // instantanée.
        faire_silence(tampons);
        return 0;
    }

    if tampons.len() != 1 {
        // Notre hypothèse de format est fausse. Le chemin désentrelacé de V1 existait pour
        // ce cas ; le reprendre à moitié juste serait pire que de ne pas l'avoir — une trame
        // partielle y décalait l'offset de canal, et L/R restaient intervertis pour tout le
        // reste du flux. Silence **compté**, donc visible, plutôt qu'un son faux.
        donnees
            .etat
            .tampons_inattendus
            .fetch_add(1, Ordering::Relaxed);
        faire_silence(tampons);
        return 0;
    }

    let lus = donnees.consommateur.tirer(&mut donnees.travail[..attendus]);
    if lus >= attendus {
        // Le tampon a de l'avance : ce qui précédait est rattrapé.
        donnees.amorti = false;
    } else if !donnees.amorti && !donnees.etat.fin_de_flux.load(Ordering::Relaxed) {
        // Un vrai trou : le décodeur n'a pas suivi, et ça s'entend.
        donnees.etat.famines.fetch_add(1, Ordering::Relaxed);
    }

    // LE CHEMIN BIT-PERFECT : à volume exactement 100 %, `appliquer` sort immédiatement et
    // les échantillons partent tels quels (voir `chain.rs`).
    Chaine::avec_volume(donnees.etat.volume()).appliquer(&mut donnees.travail[..lus]);

    ecrire_vers_coreaudio(tampons, &donnees.travail[..lus]);
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

/// Recopie `echantillons` (entrelacés) dans le tampon unique de CoreAudio, et met le reste
/// à **zéro**.
///
/// Un seul cas, et c'est voulu : le format déclaré par `flux.rs` est entrelacé, donc l'AUHAL
/// donne un tampon unique. Le cas désentrelacé est écarté par l'appelant, qui le compte.
///
/// SÛRETÉ : mêmes garanties que `faire_silence`.
unsafe fn ecrire_vers_coreaudio(
    tampons: &mut [coreaudio_sys::AudioBuffer],
    echantillons: &[i32],
) {
    let Some(t) = tampons.first_mut() else {
        return;
    };
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
}
