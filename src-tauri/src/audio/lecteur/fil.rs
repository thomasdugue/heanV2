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
use std::time::{Duration, Instant};

use super::{Commande, EtatPartage};
use crate::audio::backend::BackendAudio;
use crate::audio::coreaudio::BackendCoreAudio;
use crate::audio::decode::Decodeur;
use crate::audio::format::FormatAudio;

/// Le repos de la boucle quand elle n'a rien à faire : le tampon est plein, ou on attend
/// qu'il se vide. 5 ms laisse la position se publier à 200 Hz sans occuper un cœur.
const REPOS: Duration = Duration::from_millis(5);

/// Échéance des deux états d'attente. Le tampon fait 5 s, donc une vidange normale prend au
/// plus 5 s ; 15 s veut dire que le périphérique ne consomme plus du tout — DAC débranché,
/// session AirPlay tombée.
///
/// ⚠️ **Il n'y en avait aucune, et c'était un figement silencieux.** `AttenteDeVidange` et
/// `FinDeFile` attendaient `en_attente() == 0` sans borne : la lecture restait bloquée entre
/// deux pistes, barre de progression immobile, sans message ni ligne de journal. Le *fil*,
/// lui, ne bloquait pas — d'où la formule trompeuse que portait ce fichier : vraie du fil,
/// fausse du produit. Relevé par `invariant-guardian`.
const ECHEANCE_DATTENTE: Duration = Duration::from_secs(15);

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
    /// Les trous comptés par les flux **déjà fermés** de cette lecture.
    ///
    /// Un changement de fréquence en cours d'album rouvre le flux, et le compteur du nouveau
    /// repart de zéro. Sans ce cumul, l'écran annoncerait « 0 trou » après une coupure
    /// survenue avant le changement de piste — et la bibliothèque de Thomas est à 97 % en
    /// FLAC de fréquences variées, donc le cas est la règle, pas l'exception.
    pub(super) famines_des_flux_fermes: u64,
    /// Depuis quand on attend que le tampon se vide. `None` hors des états d'attente.
    pub(super) attente_depuis: Option<Instant>,
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
        famines_des_flux_fermes: 0,
        attente_depuis: None,
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
                if self.backend.as_ref().is_some_and(|b| b.en_attente() > 0)
                    && !self.attente_expiree()
                {
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
                if self.backend.as_ref().is_some_and(|b| b.en_attente() > 0)
                    && !self.attente_expiree()
                {
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
                // LES DEUX ISSUES S'ENTENDENT PAREIL, ET NE SE CORRIGENT PAS AU MÊME
                // ENDROIT : un silence au passage de piste est un défaut dans la première
                // branche et le comportement normal dans la seconde. Sans ces deux lignes,
                // le seul chemin que l'étape 8 du protocole éprouve ne laisse aucune trace,
                // et un enchaînement réussi ne se distingue pas d'une vidange légitime.
                Ok(decodeur) if Some(decodeur.format()) == self.format_du_flux => {
                    tracing::info!(
                        fichier = %self.file[suivante]
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy(),
                        "piste enchaînée dans le même flux"
                    );
                    self.inscrire_la_piste(suivante, decodeur, 0);
                    return;
                }
                Ok(autre) => {
                    tracing::info!(
                        fichier = %self.file[suivante]
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy(),
                        format = %autre.format(),
                        "format différent : le tampon se vide avant de renégocier la sortie"
                    );
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

    /// Vrai quand l'attente de vidange a dépassé son échéance. Le dit une fois, en nommant
    /// ce qui est abandonné : une attente qui renonce en silence est un figement déguisé.
    fn attente_expiree(&mut self) -> bool {
        let depuis = *self.attente_depuis.get_or_insert_with(Instant::now);
        if depuis.elapsed() < ECHEANCE_DATTENTE {
            return false;
        }
        let restant = self.backend.as_ref().map(|b| b.en_attente()).unwrap_or(0);
        tracing::warn!(
            restant,
            secondes = ECHEANCE_DATTENTE.as_secs(),
            "le périphérique ne consomme plus : la fin de ce qui était décodé est abandonnée"
        );
        self.etat.dire(
            "La sortie audio ne répond plus — la fin de la piste est perdue. Vérifie que le périphérique est toujours branché.",
        );
        self.attente_depuis = None;
        true
    }

    /// Dit au flux qu'il n'y aura plus de trames : les callbacks qui restent tombent à vide
    /// légitimement, et ne doivent pas être comptés comme des trous dans le son.
    fn declarer_fin_de_flux(&self) {
        if let Some(backend) = self.backend.as_ref() {
            backend.declarer_fin_de_flux();
        }
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
                // Les trous du flux qui se ferme sont retenus : le compteur du prochain flux
                // repart de zéro, et l'écran doit montrer le total de la lecture.
                self.famines_des_flux_fermes += backend.famines();
                // Les compteurs se lisent ICI, sur le chemin que la production emprunte
                // vraiment — et non dans `BackendAudio::terminer`, que rien n'appelle sur ce
                // backend.
                backend.journaliser_les_anomalies();
                backend.interrompre();
            }
        }
        self.decodeur = None;
        self.format_du_flux = None;
        self.reste.clear();
        self.deja_pris = 0;
        self.ecrit = 0;
        self.en_vol.clear();
        self.attente_depuis = None;
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
