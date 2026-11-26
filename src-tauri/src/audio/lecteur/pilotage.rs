//! Les commandes : ce que le fil de lecture fait quand on lui demande quelque chose.
//!
//! Séparé de [`super::fil`], qui porte la boucle et la machine à états. La ligne de partage
//! est nette et elle a un sens : **ici on réagit à une demande de Thomas**, là-bas on fait
//! avancer le son tout seul. Les deux moitiés tournent sur le même fil et possèdent le même
//! état, ce qui est exactement ce qui rend la lecture sûre — il n'y a jamais deux écrivains.
//!
//! Le point le plus délicat du fichier est [`Fil::aller_a`] : un déplacement porte sur la
//! piste **qu'on entend**, qui n'est pas celle que le décodeur lit. V1 a payé ce défaut.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use super::fil::{EnVol, Etape, Fil};
use super::Commande;
use crate::audio::backend::BackendAudio;
use crate::audio::coreaudio::{BackendCoreAudio, Verdict};
use crate::audio::decode::Decodeur;
use crate::audio::error::ErreurAudio;

impl Fil {
    /// Rend `false` quand le fil doit s'arrêter.
    pub(super) fn traiter(&mut self, commande: Commande) -> bool {
        let resultat = match commande {
            Commande::Fin => return false,
            Commande::Jouer { file, depart } => {
                self.file = file;
                self.etat.total.store(self.file.len(), Ordering::Relaxed);
                self.demarrer_a(depart)
            }
            Commande::Pause => {
                self.etat.en_pause.store(true, Ordering::Relaxed);
                if let Some(b) = &self.backend {
                    b.definir_pause(true);
                }
                Ok(())
            }
            Commande::Reprendre => {
                self.etat.en_pause.store(false, Ordering::Relaxed);
                if let Some(b) = &self.backend {
                    b.definir_pause(false);
                }
                Ok(())
            }
            Commande::Arreter => {
                self.tout_fermer();
                Ok(())
            }
            Commande::Aller(ms) => self.aller_a(ms),
            Commande::Suivante => self.changer_de_piste(1),
            Commande::Precedente => self.changer_de_piste(-1),
            Commande::Volume(v) => {
                self.etat.definir_volume(v);
                if let Some(b) = &self.backend {
                    b.definir_volume(v);
                }
                Ok(())
            }
            Commande::Peripherique(id) => self.changer_de_peripherique(id),
            Commande::ModeExclusif(actif) => self.changer_le_mode_exclusif(actif),
            Commande::RafraichirPeripheriques => self.rafraichir_les_peripheriques(),
        };
        if let Err(e) = resultat {
            // Une erreur de lecture va à l'écran, pas seulement au journal : c'est le canal
            // qui manquait entièrement à V1 (MESURES-V1.md §8).
            self.etat.dire(e.to_string());
            self.tout_fermer();
        }
        true
    }

    /// Le backend, créé au premier besoin. Hean ne touche pas au matériel au lancement.
    fn backend(&mut self) -> Result<&mut BackendCoreAudio, ErreurAudio> {
        if self.backend.is_none() {
            self.backend = Some(BackendCoreAudio::nouveau()?);
        }
        Ok(self.backend.as_mut().expect("créé juste au-dessus"))
    }

    pub(super) fn demarrer_a(&mut self, index: usize) -> Result<(), ErreurAudio> {
        if index >= self.file.len() {
            self.tout_fermer();
            return Ok(());
        }
        // Un démarrage explicite referme le flux : la fréquence du DAC se renégocie, et la
        // piste demandée doit commencer tout de suite, pas après ce qui reste dans le tampon.
        self.fermer_le_flux();
        self.etat.en_pause.store(false, Ordering::Relaxed);
        self.ouvrir_la_piste(index, Duration::ZERO)?;
        self.etape = Etape::Lecture;
        self.etat.en_lecture.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Ouvre une piste, prépare la sortie si le format change, et l'inscrit comme en vol.
    pub(super) fn ouvrir_la_piste(&mut self, index: usize, depart: Duration) -> Result<(), ErreurAudio> {
        let mut decodeur = Decodeur::ouvrir(&self.file[index])?;
        let format = decodeur.format();
        let mut debut_ms = 0;
        if !depart.is_zero() {
            debut_ms = decodeur.aller_a(depart)?.as_millis() as u64;
        }

        if self.format_du_flux != Some(format) {
            // Une poignée propre sur l'état partagé : `self.backend()` emprunte `self`, et
            // `Arc` est fait pour ça.
            let etat = Arc::clone(&self.etat);
            let backend = self.backend()?;
            let obtenu = backend.preparer(format)?;
            match backend.verdict().clone() {
                Verdict::Exact => {
                    etat.sans_reechantillonnage.store(true, Ordering::Relaxed);
                }
                Verdict::ReechantillonneParMacos { peripherique, hz } => {
                    // La lecture a lieu — refuser AirPlay serait perdre une fonction qui
                    // marche — mais elle n'est pas bit-perfect, et ça se dit (ADR 0018).
                    etat.sans_reechantillonnage.store(false, Ordering::Relaxed);
                    etat.dire(format!(
                        "« {peripherique} » est en AirPlay : macOS rééchantillonne vers {hz} Hz, \
                         la lecture n’est pas bit-perfect."
                    ));
                }
                verdict => {
                    // Fréquence non servie : rien n'a été touché, rien ne joue.
                    etat.sans_reechantillonnage.store(false, Ordering::Relaxed);
                    return Err(verdict.en_erreur().unwrap_or(ErreurAudio::AucunPeripherique));
                }
            }
            let exclusif = backend.mode_exclusif();
            let active = backend.info_active();
            let id_actif = active.as_ref().map(|i| i.id).unwrap_or(0);
            let nom = active
                .map(|i| i.nom)
                .unwrap_or_else(|| "sortie inconnue".to_owned());
            let volume = etat.volume();
            let en_pause = etat.en_pause.load(Ordering::Relaxed);
            backend.definir_volume(volume);
            backend.definir_pause(en_pause);
            etat.mode_exclusif.store(exclusif, Ordering::Relaxed);
            etat.peripherique_id.store(id_actif, Ordering::Relaxed);
            if let Ok(mut p) = etat.peripherique.lock() {
                *p = nom;
            }
            // Le flux est neuf : la comptabilité des positions repart de zéro.
            self.format_du_flux = Some(format);
            self.ecrit = 0;
            self.en_vol.clear();
            tracing::debug!(obtenu = %obtenu, "flux renégocié");
        }

        self.inscrire_la_piste(index, decodeur, debut_ms);
        Ok(())
    }

    /// Inscrit une piste déjà ouverte comme celle que le décodeur lit, et la met « en vol ».
    ///
    /// Séparé d'[`Self::ouvrir_la_piste`] pour l'enchaînement sans trou : là, le décodeur a
    /// déjà été ouvert pour comparer son format à celui du flux, et le rouvrir doublerait
    /// l'entrée/sortie à chaque passage de piste.
    pub(super) fn inscrire_la_piste(&mut self, index: usize, decodeur: Decodeur, debut_ms: u64) {
        let duree_ms = decodeur.duree().map(|d| d.as_millis() as u64).unwrap_or(0);
        self.en_vol.push_back(EnVol {
            debut: self.ecrit,
            fin: None,
            debut_ms,
            index,
            duree_ms,
            chemin: decodeur.chemin().to_path_buf(),
        });
        self.index_decode = index;
        self.decodeur = Some(decodeur);
        self.reste.clear();
        self.deja_pris = 0;
    }

    /// Déplacement dans la piste **qu'on entend**, et non celle que le décodeur lit.
    fn aller_a(&mut self, ms: u64) -> Result<(), ErreurAudio> {
        let Some(audible) = self.en_vol.front() else {
            return Ok(());
        };
        let index = audible.index;
        let depart = Duration::from_millis(ms);

        if self.index_decode != index {
            // Un enchaînement a déjà fait passer le décodeur à la piste suivante : il faut
            // réouvrir celle qu'on entend. C'est le défaut que V1 a payé sur `current_path`.
            self.fermer_le_flux();
            return self.ouvrir_la_piste(index, depart).map(|()| {
                self.etape = Etape::Lecture;
            });
        }

        let atteint = self
            .decodeur
            .as_mut()
            .ok_or(ErreurAudio::AucunPeripherique)?
            .aller_a(depart)?;
        self.reste.clear();
        self.deja_pris = 0;

        let (duree_ms, chemin) = {
            let a = self.en_vol.front().expect("vérifié juste au-dessus");
            (a.duree_ms, a.chemin.clone())
        };
        if let Some(backend) = self.backend.as_mut() {
            backend.purger()?;
            let lu = backend.total_lu();
            self.ecrit = lu;
            self.en_vol.clear();
            self.en_vol.push_back(EnVol {
                debut: lu,
                fin: None,
                debut_ms: atteint.as_millis() as u64,
                index,
                duree_ms,
                chemin,
            });
        }
        self.etape = Etape::Lecture;
        Ok(())
    }

    fn changer_de_piste(&mut self, pas: i64) -> Result<(), ErreurAudio> {
        let courant = self
            .en_vol
            .front()
            .map(|p| p.index as i64)
            .unwrap_or(self.index_decode as i64);
        let vise = courant + pas;
        if vise < 0 || vise as usize >= self.file.len() {
            self.tout_fermer();
            return Ok(());
        }
        self.demarrer_a(vise as usize)
    }

    fn changer_de_peripherique(&mut self, id: coreaudio_sys::AudioObjectID) -> Result<(), ErreurAudio> {
        let reprendre = self.en_vol.front().map(|p| (p.index, self.position_ms()));
        self.fermer_le_flux();
        self.backend()?.definir_peripherique(id)?;
        self.rafraichir_les_peripheriques()?;
        match reprendre {
            Some((index, ms)) => {
                self.ouvrir_la_piste(index, Duration::from_millis(ms))?;
                self.etape = Etape::Lecture;
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn changer_le_mode_exclusif(&mut self, actif: bool) -> Result<(), ErreurAudio> {
        let reprendre = self.en_vol.front().map(|p| (p.index, self.position_ms()));
        self.fermer_le_flux();
        let etat = Arc::clone(&self.etat);
        self.backend()?.definir_mode_exclusif(actif)?;
        etat.mode_exclusif.store(actif, Ordering::Relaxed);
        match reprendre {
            Some((index, ms)) => {
                self.ouvrir_la_piste(index, Duration::from_millis(ms))?;
                self.etape = Etape::Lecture;
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn rafraichir_les_peripheriques(&mut self) -> Result<(), ErreurAudio> {
        let etat = Arc::clone(&self.etat);
        let backend = self.backend()?;
        backend.rafraichir_peripheriques()?;
        let liste = backend.liste_peripheriques();
        let active = backend.info_active();
        if let Ok(mut p) = etat.peripheriques.lock() {
            *p = liste;
        }
        if let Some(active) = active {
            etat.peripherique_id.store(active.id, Ordering::Relaxed);
            if let Ok(mut p) = etat.peripherique.lock() {
                *p = active.nom;
            }
        }
        Ok(())
    }

}
