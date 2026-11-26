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
use crate::audio::coreaudio::BackendCoreAudio;
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
                // C'est ICI qu'une nouvelle lecture commence, et nulle part ailleurs : le
                // compte de trous repart donc de zéro ici. Le mettre dans `demarrer_a`
                // — comme je l'avais fait — le remettait aussi à zéro sur « piste suivante »,
                // alors qu'un enchaînement automatique, lui, conservait le total. L'écran dit
                // « depuis le début » : une asymétrie entre le manuel et l'automatique est le
                // genre de détail qui fait douter du chiffre.
                self.famines_des_flux_fermes = 0;
                self.etat.famines.store(0, Ordering::Relaxed);
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
            Commande::Changer(pas) => self.changer_de_piste(pas),
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
            // LA PIÈCE QUI REND LE REPLI HONNÊTE. `preparer` a peut-être réglé le
            // périphérique sur une autre fréquence que celle du fichier — c'est le repli que
            // Thomas veut — et la seule chose qui en fasse de l'information plutôt qu'une
            // altération cachée est que les deux chiffres arrivent à l'écran.
            let n = backend.negociation().ok_or(ErreurAudio::AucunPeripherique)?;
            etat.source_hz.store(n.source.frequence, Ordering::Relaxed);
            etat.source_bits
                .store(n.source.profondeur.bits(), Ordering::Relaxed);
            etat.sortie_hz.store(n.sortie_hz, Ordering::Relaxed);
            etat.airplay.store(n.airplay, Ordering::Relaxed);
            // Un AirPlay n'est jamais bit-perfect, même quand les deux fréquences
            // coïncident : macOS rééchantillonne de son côté et on ne peut pas l'en empêcher
            // (PLAT-1). V1 l'étiquette de la même façon dans sa liste de périphériques.
            etat.sans_reechantillonnage
                .store(n.sans_reechantillonnage() && !n.airplay, Ordering::Relaxed);
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

        // UNE LECTURE QUI NE LAISSE AUCUNE TRACE NE SE DIAGNOSTIQUE PAS. Le 2026-11-26,
        // Thomas a dit « je n'entends pas de son » et le journal n'avait rien à montrer : ni
        // succès, ni échec, pas même la preuve que le moteur avait été appelé. Cette ligne
        // sépare les deux questions — « est-ce que ça a été demandé ? » et « est-ce que ça a
        // été joué, et à quelle fréquence ? » — et c'est la première qu'on se pose.
        tracing::info!(
            fichier = %self.file[index]
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            source_hz = format.frequence,
            bits = format.profondeur.bits(),
            sortie_hz = self.etat.sortie_hz.load(Ordering::Relaxed),
            sans_reechantillonnage = self.etat.sans_reechantillonnage.load(Ordering::Relaxed),
            depart_ms = debut_ms,
            "piste ouverte"
        );
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

        // Le décodeur peut être absent : en fin de file, il a été relâché dès la dernière
        // trame décodée, et il reste jusqu'à cinq secondes de son à jouer. Un déplacement à
        // ce moment-là réouvre la piste plutôt que de remonter « aucun périphérique de
        // sortie audio » — un message faux, qui arrêtait la lecture par-dessus le marché.
        if self.decodeur.is_none() {
            self.fermer_le_flux();
            self.ouvrir_la_piste(index, depart)?;
            self.etape = Etape::Lecture;
            return Ok(());
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
        // ⚠️ Le nom publié ne change PAS tant qu'un flux est ouvert, et c'est le point.
        // `info_active()` lit le défaut système **vivant** : si Thomas change de sortie dans
        // les Réglages Son de macOS pendant une piste, l'AudioUnit continue de jouer sur
        // l'ancien périphérique — elle a été ouverte avec son identifiant — alors que
        // l'écran afficherait déjà le nouveau nom, avec les fréquences de l'ancien et
        // peut-être le badge bit-perfect. Un indicateur qui nomme une sortie qui ne joue pas
        // est un indicateur qui ment. Tant qu'un flux joue, c'est `ouvrir_la_piste` qui a
        // publié le bon nom, et lui seul.
        //
        // Suivre un changement de défaut système en cours de piste demanderait de rouvrir le
        // flux ; V1 le faisait par sondage (`check_device_change`). Ce n'est pas fait ici, et
        // c'est écrit plutôt que supposé.
        let flux_ouvert = backend.flux_ouvert();
        if let Ok(mut p) = etat.peripheriques.lock() {
            *p = liste;
        }
        if let (false, Some(active)) = (flux_ouvert, active) {
            etat.peripherique_id.store(active.id, Ordering::Relaxed);
            if let Ok(mut p) = etat.peripherique.lock() {
                *p = active.nom;
            }
        }
        Ok(())
    }

}
