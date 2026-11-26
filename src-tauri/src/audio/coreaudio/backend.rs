//! `BackendCoreAudio` — l'implémentation de [`BackendAudio`] qui parle au DAC.
//!
//! # Ce que `preparer` rend, et pourquoi c'est le format du *périphérique*
//!
//! L'AudioUnit est toujours alimentée **à la fréquence du fichier** : c'est ce que le
//! décodeur produit. Ce que `preparer` négocie, c'est la fréquence du *périphérique*, et
//! c'est elle qu'il rend. Quand les deux coïncident, les échantillons traversent l'AUHAL
//! sans qu'une seule valeur change — le chemin bit-perfect. Quand elles diffèrent, l'AUHAL
//! rééchantillonne, et c'est exactement ce que l'appelant doit pouvoir constater :
//!
//! - **le périphérique ne sait pas servir la fréquence** → rien n'est touché, aucun flux
//!   n'est ouvert, et le verdict dit quoi afficher. L'appelant refuse (ADR 0018). C'est la
//!   correction du défaut de V1 relevé dans `docs/MESURES-V1.md` §11, où V1 choisissait une
//!   autre fréquence et jouait quand même ;
//! - **AirPlay** → la fréquence n'est pas touchée non plus, mais pour une autre raison
//!   (PLAT-1 : y toucher tue la session réseau) et avec une autre conséquence : la lecture a
//!   lieu, macOS rééchantillonne, et ça se **dit**.
//!
//! Le verdict est donc une pièce du contrat, pas un détail : voir [`Verdict`].

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use coreaudio_sys::AudioObjectID;

use super::flux::Flux;
use super::frequence;
use super::peripheriques::InfoPeripherique;
use super::rendu::EtatDuFlux;
use super::routage::Routage;
use super::verdict::Verdict;
use super::tampon::{self, Jauge, Producteur};
use crate::audio::backend::BackendAudio;
use crate::audio::chain::Volume;
use crate::audio::error::ErreurAudio;
use crate::audio::format::FormatAudio;

/// Combien de secondes de son le tampon circulaire peut avancer sur la sortie. 5 s, la
/// valeur de V1 : c'est la marge qui absorbe une lecture disque lente ou un décodage en
/// retard sans faire de trou.
const SECONDES_DE_TAMPON: f64 = 5.0;

/// Ce qu'il faut avoir mis dans le tampon avant de lancer la sortie. Sans ce
/// pré-remplissage, les premiers callbacks tombent sur un tampon vide : un clic au
/// démarrage, et un compteur de famines faussé dès la première piste. 0,5 s se décode en
/// quelques millisecondes, donc le démarrage reste perçu comme immédiat.
const SECONDES_DE_PREROLL: f64 = 0.5;

/// Plafond d'attente de la vidange du tampon à l'arrêt. Au-delà, on coupe : mieux vaut
/// perdre la fin d'une piste que de bloquer l'application sur un périphérique qui ne
/// consomme plus (débranché pendant la lecture).
const ATTENTE_DE_VIDANGE_MAX: Duration = Duration::from_secs(10);

/// Plafond d'attente de la purge du tampon par le callback. Un callback arrive toutes les
/// quelques millisecondes ; 200 ms veut dire qu'il n'en vient plus, et la lecture ne doit pas
/// bloquer pour autant.
const ATTENTE_DE_PURGE_MAX: Duration = Duration::from_millis(200);

pub struct BackendCoreAudio {
    routage: Routage,
    /// Les fréquences à remettre à l'arrêt, par périphérique. Hean ne laisse pas le DAC dans
    /// un état que personne n'a demandé.
    frequences_dorigine: HashMap<AudioObjectID, u32>,
    flux: Option<Flux>,
    producteur: Option<Producteur>,
    jauge: Option<Jauge>,
    etat: Option<Arc<EtatDuFlux>>,
    verdict: Verdict,
    preroll: usize,
    demarre: bool,
}

impl BackendCoreAudio {
    pub fn nouveau() -> Result<Self, ErreurAudio> {
        Ok(Self {
            routage: Routage::nouveau()?,
            frequences_dorigine: HashMap::new(),
            flux: None,
            producteur: None,
            jauge: None,
            etat: None,
            verdict: Verdict::Exact,
            preroll: 0,
            demarre: false,
        })
    }

    /// Le verdict de la dernière négociation. À lire **après chaque `preparer`**.
    pub fn verdict(&self) -> &Verdict {
        &self.verdict
    }

    /// L'état partagé avec le callback : pause, volume, compteur de famines.
    pub fn etat_du_flux(&self) -> Option<Arc<EtatDuFlux>> {
        self.etat.clone()
    }

    /// Les compteurs du tampon, pour savoir où en est la lecture.
    pub fn jauge(&self) -> Option<Jauge> {
        self.jauge.clone()
    }

    pub fn liste_peripheriques(&self) -> Vec<InfoPeripherique> {
        self.routage.liste()
    }

    pub fn info_active(&self) -> Option<InfoPeripherique> {
        self.routage.info_active().cloned()
    }

    pub fn mode_exclusif(&self) -> bool {
        self.routage.mode_exclusif()
    }

    pub fn rafraichir_peripheriques(&mut self) -> Result<(), ErreurAudio> {
        self.routage.rafraichir()
    }

    pub fn definir_peripherique(&mut self, id: AudioObjectID) -> Result<(), ErreurAudio> {
        self.routage.definir_peripherique(id)
    }

    pub fn definir_mode_exclusif(&mut self, actif: bool) -> Result<(), ErreurAudio> {
        self.routage.definir_mode_exclusif(actif)
    }

    pub fn definir_volume(&self, volume: Volume) {
        if let Some(etat) = &self.etat {
            etat.definir_volume(volume);
        }
    }

    /// Met la lecture en pause sans rien consommer : la reprise est instantanée, et le
    /// tampon n'est pas jeté.
    pub fn definir_pause(&self, en_pause: bool) {
        if let Some(etat) = &self.etat {
            etat.en_pause
                .store(en_pause, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Jette ce qui attend et purge les tampons internes de CoreAudio.
    ///
    /// Les **deux** sont nécessaires au déplacement dans la piste : sans le premier on
    /// rejoue les secondes d'avant le saut, sans le second on entend les ~50 ms déjà parties
    /// dans le HAL. C'est précisément ce second point qui a fait quitter CPAL à V1 — sans
    /// accès direct à l'AudioUnit, ces 50 ms s'entendent comme un hoquet après chaque saut.
    ///
    /// La purge du tampon est **demandée** et non faite ici : son consommateur vit dans le
    /// callback. L'attente est bornée, et un callback qui ne vient pas (périphérique
    /// débranché) ne doit pas bloquer la lecture.
    pub fn purger(&mut self) -> Result<(), ErreurAudio> {
        if let Some(etat) = &self.etat {
            etat.purge_demandee
                .store(true, std::sync::atomic::Ordering::Release);
            let limite = Instant::now() + ATTENTE_DE_PURGE_MAX;
            while etat
                .purge_demandee
                .load(std::sync::atomic::Ordering::Acquire)
                && Instant::now() < limite
            {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        if let Some(flux) = self.flux.as_mut() {
            flux.purger()?;
        }
        Ok(())
    }

    /// Le total d'échantillons déjà sortis du tampon depuis l'ouverture du flux.
    pub fn total_lu(&self) -> usize {
        self.jauge.as_ref().map(Jauge::total_lu).unwrap_or(0)
    }

    /// Ce qui attend encore d'être joué.
    pub fn en_attente(&self) -> usize {
        self.jauge.as_ref().map(Jauge::en_attente).unwrap_or(0)
    }

    pub fn flux_ouvert(&self) -> bool {
        self.flux.is_some()
    }

    /// Le nombre de callbacks servis à vide depuis l'ouverture du flux : la mesure de « ça
    /// ne coupe pas ». Chaque unité est un trou audible.
    pub fn famines(&self) -> u64 {
        self.etat
            .as_ref()
            .map(|e| e.famines.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Coupe la lecture **tout de suite**, en jetant ce qui attend, et rend le matériel.
    ///
    /// C'est le pendant de [`BackendAudio::terminer`], qui lui *attend* la vidange du tampon
    /// pour ne pas couper la fin d'une piste. Les deux existent parce que les deux situations
    /// sont opposées : à la fin d'un album on veut entendre les dernières secondes ; quand
    /// Thomas clique une autre piste, les entendre serait un défaut — il attendrait jusqu'à
    /// cinq secondes avant que sa piste démarre, en écoutant la précédente.
    pub fn interrompre(&mut self) {
        let _ = self.purger();
        self.fermer_le_flux();
        self.routage
            .rendre_le_materiel(&mut self.frequences_dorigine);
    }

    /// Ferme le flux **sans** rendre le matériel : entre deux pistes de fréquences
    /// différentes, le périphérique reste à nous.
    fn fermer_le_flux(&mut self) {
        if let Some(mut flux) = self.flux.take() {
            let _ = flux.arreter();
        }
        self.producteur = None;
        self.jauge = None;
        self.etat = None;
        self.demarre = false;
    }
}

impl BackendAudio for BackendCoreAudio {
    /// Négocie la fréquence du périphérique et ouvre le flux. Rend le format **du
    /// périphérique** — voir l'en-tête du module.
    fn preparer(&mut self, format: FormatAudio) -> Result<FormatAudio, ErreurAudio> {
        self.fermer_le_flux();
        self.routage.verifier_changement();
        self.routage.rafraichir()?;

        let id = self.routage.peripherique_actif()?;
        let info = self
            .routage
            .cache()
            .info(id)
            .cloned()
            .ok_or(ErreurAudio::AucunPeripherique)?;

        let hz_du_peripherique = if info.est_airplay() {
            // PLAT-1 (🔴) : on ne touche PAS à la fréquence d'un AirPlay — ça tue la session
            // réseau, et macOS y rééchantillonne de toute façon en interne.
            let hz = frequence::frequence_du_peripherique(id)?;
            self.verdict = Verdict::ReechantillonneParMacos {
                peripherique: info.nom.clone(),
                hz,
            };
            hz
        } else if frequence::est_servie(id, format.frequence)? {
            self.frequences_dorigine
                .entry(id)
                .or_insert(frequence::frequence_du_peripherique(id)?);
            if frequence::frequence_du_peripherique(id)? != format.frequence {
                frequence::definir_frequence_du_peripherique(id, format.frequence)?;
            }
            self.verdict = Verdict::Exact;
            format.frequence
        } else {
            // LE DÉFAUT DE V1, CORRIGÉ. V1 appelait ici `find_best_supported_rate` et jouait
            // à une autre fréquence sans rien dire. On ne change **rien** — pas même la
            // fréquence du périphérique, pour ne pas laisser derrière un réglage que
            // personne n'a demandé — et on rend ce que le périphérique fait réellement.
            let hz = frequence::frequence_du_peripherique(id)?;
            self.verdict = Verdict::FrequenceNonServie {
                peripherique: info.nom.clone(),
                demandee: format.frequence,
                proposees: info.frequences_en_clair(),
            };
            tracing::warn!(
                peripherique = %info.nom,
                demandee = format.frequence,
                proposees = %info.frequences_en_clair(),
                "fréquence non servie — la lecture est refusée, pas rééchantillonnée (ADR 0018)"
            );
            return Ok(FormatAudio::nouveau(hz, format.canaux, format.profondeur));
        };

        self.routage.reappliquer_mode_exclusif()?;

        // Le tampon est dimensionné sur la fréquence du FICHIER : c'est à ce rythme que le
        // décodeur le remplit et que l'AudioUnit le consomme.
        let par_seconde = format.frequence as f64 * f64::from(format.canaux);
        let cases = (par_seconde * SECONDES_DE_TAMPON).ceil() as usize;
        self.preroll = (par_seconde * SECONDES_DE_PREROLL).ceil() as usize;
        let (producteur, consommateur) = tampon::nouveau(cases.max(1));
        self.jauge = Some(producteur.jauge());

        // L'AudioUnit est alimentée à la fréquence du fichier. Quand le périphérique tourne
        // à la même, rien n'est rééchantillonné.
        let flux = Flux::ouvrir(self.routage.id_pour_audiounit()?, format, consommateur)?;
        self.etat = Some(flux.etat());
        self.flux = Some(flux);
        self.producteur = Some(producteur);

        // Les canaux rendus sont ceux dont le flux a été configuré : c'est la fréquence,
        // et elle seule, qui peut différer de la demande.
        let obtenu = FormatAudio::nouveau(hz_du_peripherique, format.canaux, format.profondeur);
        tracing::info!(
            peripherique = %info.nom,
            fichier = %format,
            peripherique_hz = hz_du_peripherique,
            verdict = ?self.verdict,
            "sortie préparée"
        );
        Ok(obtenu)
    }

    /// Pousse ce qui rentre dans le tampon et rend combien a été pris. **Ne bloque jamais** :
    /// un appelant qui reçoit moins que ce qu'il a offert rappelle plus tard avec le reste.
    fn ecrire(&mut self, echantillons: &[i32]) -> Result<usize, ErreurAudio> {
        let producteur = self.producteur.as_mut().ok_or_else(|| {
            if let Some(e) = self.verdict.en_erreur() {
                return e;
            }
            ErreurAudio::FormatRefuse {
                demande: "écriture avant préparation".to_owned(),
                obtenu: "aucun flux ouvert".to_owned(),
            }
        })?;
        let pris = producteur.pousser(echantillons);

        if !self.demarre && producteur.total_ecrit() >= self.preroll {
            if let Some(flux) = self.flux.as_mut() {
                flux.demarrer()?;
            }
            self.demarre = true;
        }
        Ok(pris)
    }

    /// Laisse sortir ce qui reste dans le tampon, puis arrête tout et rend le matériel.
    fn terminer(&mut self) -> Result<(), ErreurAudio> {
        // Une piste plus courte que le pré-remplissage n'a jamais démarré la sortie : il
        // faut la lancer ici, sinon son son ne sort pas du tout.
        if !self.demarre && self.jauge.as_ref().is_some_and(|j| j.en_attente() > 0) {
            if let Some(flux) = self.flux.as_mut() {
                flux.demarrer()?;
            }
            self.demarre = true;
        }
        if let Some(jauge) = self.jauge.clone() {
            let limite = Instant::now() + ATTENTE_DE_VIDANGE_MAX;
            while jauge.en_attente() > 0 && Instant::now() < limite {
                std::thread::sleep(Duration::from_millis(5));
            }
            if jauge.en_attente() > 0 {
                tracing::warn!(
                    restant = jauge.en_attente(),
                    "le périphérique ne consomme plus : la fin de la piste est coupée"
                );
            }
        }
        let famines = self.famines();
        self.fermer_le_flux();
        self.routage
            .rendre_le_materiel(&mut self.frequences_dorigine);
        if famines > 0 {
            tracing::warn!(famines, "la lecture a eu des trous : le décodeur n'a pas suivi");
        }
        Ok(())
    }
}

impl Drop for BackendCoreAudio {
    fn drop(&mut self) {
        // Le matériel se rend même sur un chemin anormal : un DAC laissé réservé refuse de
        // s'ouvrir pour l'application suivante, et une fréquence laissée à 192 kHz surprend
        // la suivante aussi.
        self.fermer_le_flux();
        self.routage
            .rendre_le_materiel(&mut self.frequences_dorigine);
    }
}

