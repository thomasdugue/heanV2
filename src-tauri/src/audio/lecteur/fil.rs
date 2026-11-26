//! Le fil de lecture : une machine à quatre états, et rien de plus.
//!
//! ```text
//!   Repos ──Jouer──▶ Lecture ──format différent──▶ AttenteDeVidange ──▶ Lecture
//!     ▲                 │                                                  │
//!     └──── vidé ◀── FinDeFile ◀──── plus rien à décoder ◀─────────────────┘
//! ```
//!
//! Les deux états d'attente existent pour **une seule raison** : ne jamais bloquer. Un
//! `terminer()` qui attend la vidange du tampon prend jusqu'à cinq secondes, pendant
//! lesquelles ni une pause ni un changement de piste ne seraient entendus. En faisant de
//! l'attente un état plutôt qu'un appel bloquant, la boucle continue de tourner, de publier
//! la position et de recevoir les commandes.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use super::{Commande, EtatPartage};
use crate::audio::backend::BackendAudio;
use crate::audio::coreaudio::BackendCoreAudio;
use crate::audio::decode::Decodeur;
use crate::audio::format::FormatAudio;

/// Le repos de la boucle quand elle n'a rien à faire : le tampon est plein, ou on attend
/// qu'il se vide. 5 ms laisse la position se publier à 200 Hz sans occuper un cœur.
const REPOS: Duration = Duration::from_millis(5);

/// Une piste poussée dans le flux courant, et où elle s'y trouve.
///
/// Il en faut une file et non une seule : à 5 s de tampon, la piste qu'on **entend** n'est
/// pas celle que le décodeur lit. V1 avait ce défaut exact — son `current_path` pointait sur
/// la piste en cours de décodage, donc un déplacement juste après un enchaînement allait
/// chercher la durée de la piste précédente.
pub(super) struct EnVol {
    /// Position du premier échantillon, en total écrit dans le flux courant.
    pub(super) debut: usize,
    /// Position de fin, connue dès que la piste est entièrement poussée.
    pub(super) fin: Option<usize>,
    /// Où, dans la piste, se trouve son premier échantillon poussé. Non nul après un
    /// déplacement.
    pub(super) debut_ms: u64,
    pub(super) index: usize,
    pub(super) duree_ms: u64,
    pub(super) chemin: PathBuf,
}

pub(super) enum Etape {
    Repos,
    Lecture,
    /// Le tampon doit se vider avant d'ouvrir la piste d'index donné : son format diffère,
    /// donc il faut renégocier la fréquence du DAC.
    AttenteDeVidange(usize),
    /// Plus rien à décoder ; on laisse sortir ce qui reste.
    FinDeFile,
}

pub(super) struct Fil {
    pub(super) etat: Arc<EtatPartage>,
    pub(super) backend: Option<BackendCoreAudio>,
    pub(super) file: Vec<PathBuf>,
    /// Index de la piste que le **décodeur** lit, qui n'est pas celle qu'on entend.
    pub(super) index_decode: usize,
    pub(super) decodeur: Option<Decodeur>,
    pub(super) format_du_flux: Option<FormatAudio>,
    /// Échantillons décodés que le backend n'a pas encore acceptés.
    pub(super) reste: Vec<i32>,
    pub(super) deja_pris: usize,
    /// Total poussé dans le flux courant.
    pub(super) ecrit: usize,
    pub(super) en_vol: VecDeque<EnVol>,
    pub(super) etape: Etape,
}

pub fn tourner(reception: Receiver<Commande>, etat: Arc<EtatPartage>) {
    let mut fil = Fil {
        etat,
        backend: None,
        file: Vec::new(),
        index_decode: 0,
        decodeur: None,
        format_du_flux: None,
        reste: Vec::new(),
        deja_pris: 0,
        ecrit: 0,
        en_vol: VecDeque::new(),
        etape: Etape::Repos,
    };
    loop {
        if matches!(fil.etape, Etape::Repos) {
            // Au repos, on bloque : pas de boucle d'attente active quand rien ne joue.
            match reception.recv() {
                Ok(commande) => {
                    if !fil.traiter(commande) {
                        break;
                    }
                }
                Err(_) => break,
            }
            continue;
        }
        loop {
            match reception.try_recv() {
                Ok(commande) => {
                    if !fil.traiter(commande) {
                        return fil.rendre_le_materiel();
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return fil.rendre_le_materiel(),
            }
        }
        let a_travaille = fil.avancer();
        fil.publier();
        if !a_travaille {
            std::thread::sleep(REPOS);
        }
    }
    fil.rendre_le_materiel();
}

impl Fil {
    /// Un tour de travail. Rend `true` si quelque chose a avancé.
    fn avancer(&mut self) -> bool {
        match self.etape {
            Etape::Repos => false,
            Etape::Lecture => self.decoder_et_pousser(),
            Etape::AttenteDeVidange(index) => {
                if self.backend.as_ref().is_some_and(|b| b.en_attente() > 0) {
                    return false;
                }
                // Le tampon est vide : on peut refermer le flux et renégocier sans couper
                // du son déjà décodé.
                self.fermer_le_flux();
                match self.ouvrir_la_piste(index, Duration::ZERO) {
                    Ok(()) => self.etape = Etape::Lecture,
                    Err(e) => {
                        self.etat.dire(e.to_string());
                        self.tout_fermer();
                    }
                }
                true
            }
            Etape::FinDeFile => {
                if self.backend.as_ref().is_some_and(|b| b.en_attente() > 0) {
                    return false;
                }
                self.tout_fermer();
                true
            }
        }
    }

    fn decoder_et_pousser(&mut self) -> bool {
        if self.deja_pris < self.reste.len() {
            let Some(backend) = self.backend.as_mut() else {
                return false;
            };
            let pris = match backend.ecrire(&self.reste[self.deja_pris..]) {
                Ok(n) => n,
                Err(e) => {
                    self.etat.dire(e.to_string());
                    self.tout_fermer();
                    return true;
                }
            };
            self.deja_pris += pris;
            self.ecrit += pris;
            return pris > 0;
        }

        // Le reste est écoulé : au bloc suivant.
        let Some(decodeur) = self.decodeur.as_mut() else {
            return false;
        };
        match decodeur.suivant() {
            Ok(Some(bloc)) => {
                self.reste = bloc.echantillons;
                self.deja_pris = 0;
                true
            }
            Ok(None) => {
                self.piste_decodee();
                true
            }
            Err(e) => {
                self.etat.dire(e.to_string());
                self.tout_fermer();
                true
            }
        }
    }

    /// La piste courante est entièrement décodée : on enchaîne.
    ///
    /// **Une boucle, et non un appel récursif**, et ce n'est pas un détail de style : la
    /// branche « piste illisible » s'appelait elle-même, une trame de pile par piste. Un NAS
    /// qui tombe ou un disque éjecté en cours d'album fait échouer l'ouverture de *toutes*
    /// les suivantes — jusqu'à 3 421 chez Thomas — sur un fil de 2 Mio de pile. Un
    /// débordement de pile est un `SIGSEGV` sans une ligne de journal : la famille d'INV-21,
    /// un défaut qui ne laisse aucune trace. Relevé par `invariant-guardian`.
    fn piste_decodee(&mut self) {
        if let Some(derniere) = self.en_vol.back_mut() {
            derniere.fin = Some(self.ecrit);
        }
        self.decodeur = None;

        let mut suivante = self.index_decode + 1;
        loop {
            if suivante >= self.file.len() {
                self.declarer_fin_de_flux();
                self.etape = Etape::FinDeFile;
                return;
            }
            // L'ENCHAÎNEMENT SANS TROU : si le format est identique, on continue de pousser
            // dans le MÊME tampon. Il ne se passe rien, donc il n'y a rien à entendre. Sinon
            // il faut laisser le tampon se vider avant de renégocier la fréquence du DAC.
            //
            // Le fichier n'est ouvert qu'UNE fois, et le décodeur est réutilisé : connaître
            // le format demande de l'ouvrir, et le rouvrir ensuite doublerait
            // l'entrée/sortie à chaque passage de piste — invisible en local, pas sur le NAS.
            match Decodeur::ouvrir(&self.file[suivante]) {
                Ok(decodeur) if Some(decodeur.format()) == self.format_du_flux => {
                    self.inscrire_la_piste(suivante, decodeur, 0);
                    return;
                }
                Ok(_) => {
                    self.declarer_fin_de_flux();
                    self.etape = Etape::AttenteDeVidange(suivante);
                    return;
                }
                Err(e) => {
                    // Une piste illisible ne doit pas arrêter l'album : on la dit et on
                    // passe à la suivante.
                    self.etat.dire(format!("piste ignorée — {e}"));
                    self.index_decode = suivante;
                    suivante += 1;
                }
            }
        }
    }

    /// Dit au flux qu'il n'y aura plus de trames : les callbacks qui restent tombent à vide
    /// légitimement, et ne doivent pas être comptés comme des trous dans le son.
    fn declarer_fin_de_flux(&self) {
        if let Some(backend) = self.backend.as_ref() {
            backend.declarer_fin_de_flux();
        }
    }

    pub(super) fn position_ms(&self) -> u64 {
        let (Some(audible), Some(format), Some(backend)) =
            (self.en_vol.front(), self.format_du_flux, self.backend.as_ref())
        else {
            return 0;
        };
        let par_seconde = format.frequence as usize * format.canaux as usize;
        if par_seconde == 0 {
            return 0;
        }
        let ecoule = backend.total_lu().saturating_sub(audible.debut);
        audible.debut_ms + (ecoule as u64 * 1000 / par_seconde as u64)
    }

    fn publier(&mut self) {
        // Retirer de la file les pistes entièrement sorties du tampon : la première qui
        // reste est celle qu'on entend.
        loop {
            let sortie = match (self.en_vol.front(), self.backend.as_ref()) {
                (Some(p), Some(b)) => p.fin.is_some_and(|fin| b.total_lu() >= fin),
                _ => false,
            };
            if !sortie || self.en_vol.len() <= 1 {
                break;
            }
            self.en_vol.pop_front();
        }

        let position = self.position_ms();
        if let Some(audible) = self.en_vol.front() {
            self.etat.index.store(audible.index, Ordering::Relaxed);
            self.etat.duree_ms.store(audible.duree_ms, Ordering::Relaxed);
            let plafond = if audible.duree_ms > 0 {
                position.min(audible.duree_ms)
            } else {
                position
            };
            self.etat.position_ms.store(plafond, Ordering::Relaxed);
            if let Ok(mut p) = self.etat.piste.lock() {
                let chemin = audible.chemin.display().to_string();
                if p.as_deref() != Some(chemin.as_str()) {
                    *p = Some(chemin);
                }
            }
        }
        if let Some(backend) = self.backend.as_ref() {
            self.etat.famines.store(backend.famines(), Ordering::Relaxed);
        }
        // Les quatre conditions du bit-perfect, et elles sont toutes nécessaires : pas de
        // rééchantillonnage, volume exactement à 100 %, mode exclusif, et l'égaliseur plat
        // — ce dernier est vrai par construction tant qu'il n'existe pas (ADR 0007).
        let bit_perfect = self.etat.sans_reechantillonnage.load(Ordering::Relaxed)
            && self.etat.volume().est_unite()
            && self.etat.mode_exclusif.load(Ordering::Relaxed);
        self.etat.bit_perfect.store(bit_perfect, Ordering::Relaxed);
    }

    /// Referme le flux **immédiatement**, en jetant ce qui attendait.
    ///
    /// `interrompre` et non `terminer` : `terminer` attend la vidange du tampon, donc jusqu'à
    /// cinq secondes. C'est ce qu'on veut à la fin d'un album — entendre les dernières
    /// secondes — et c'est un défaut partout ailleurs : quand Thomas clique une autre piste,
    /// il attendrait la fin de la précédente avant d'entendre la sienne. Les deux endroits
    /// qui veulent la vidange l'obtiennent autrement, et sans bloquer : les étapes
    /// `AttenteDeVidange` et `FinDeFile` attendent dans la boucle, où les commandes
    /// continuent d'arriver, et appellent ceci une fois le tampon déjà vide.
    pub(super) fn fermer_le_flux(&mut self) {
        if let Some(backend) = self.backend.as_mut() {
            if backend.flux_ouvert() {
                backend.interrompre();
            }
        }
        self.decodeur = None;
        self.format_du_flux = None;
        self.reste.clear();
        self.deja_pris = 0;
        self.ecrit = 0;
        self.en_vol.clear();
    }

    pub(super) fn tout_fermer(&mut self) {
        let derniere = self.en_vol.front().map(|p| (p.index, p.duree_ms));
        self.fermer_le_flux();
        self.etape = Etape::Repos;
        self.etat.en_lecture.store(false, Ordering::Relaxed);
        self.etat.en_pause.store(false, Ordering::Relaxed);
        if let Some((index, duree)) = derniere {
            // La barre reste en fin de piste plutôt que de retomber à zéro : c'est ce que
            // montre un lecteur qui vient de finir un album.
            self.etat.index.store(index, Ordering::Relaxed);
            self.etat.position_ms.store(duree, Ordering::Relaxed);
        }
    }

    fn rendre_le_materiel(&mut self) {
        self.fermer_le_flux();
        // Le `Drop` du backend remet la fréquence d'origine et relâche la réservation.
        self.backend = None;
        self.etat.en_lecture.store(false, Ordering::Relaxed);
    }

}
