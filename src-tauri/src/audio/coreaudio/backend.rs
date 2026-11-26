//! `BackendCoreAudio` — l'implémentation de [`BackendAudio`] qui parle au DAC.
//!
//! # Ce que `preparer` rend, et pourquoi c'est le format du *périphérique*
//!
//! L'AudioUnit est toujours alimentée **à la fréquence du fichier** : c'est ce que le
//! décodeur produit. Ce que `preparer` négocie, c'est la fréquence du *périphérique*, et
//! c'est elle qu'il rend. Quand les deux coïncident, les échantillons traversent l'AUHAL
//! sans qu'une seule valeur change — le chemin bit-perfect.
//!
//! Quand elles diffèrent, l'AUHAL rééchantillonne. **Ce n'est pas un défaut, c'est le repli
//! que Thomas veut** — et la seule chose qui le rende honnête est que l'écart soit
//! *affiché* : fréquence entrante, fréquence sortante, pas de badge bit-perfect. D'où
//! [`Negociation`], qui porte les deux chiffres jusqu'à l'écran, et qui est l'équivalent de
//! l'`AudioSpecs` de V1 (`audio_engine.rs:540`, affiché par `playback.js:862`).
//!
//! Deux façons d'en arriver là, et elles ne se traitent pas pareil :
//!
//! - **le périphérique ne sait pas servir la fréquence** → on replie sur la plus proche
//!   au-dessus (`frequence::meilleure_frequence_supportee`, porté tel quel de V1) et on règle
//!   le périphérique dessus ;
//! - **AirPlay** → on ne touche **pas** à sa fréquence (PLAT-1 : y toucher tue la session
//!   réseau) et on joue à sa fréquence native, macOS rééchantillonnant en interne.
//!
//! Dans les deux cas la lecture a lieu. Refuser serait supprimer une fonction qui marche en
//! V1 ; se taire serait la tromperie.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use coreaudio_sys::AudioObjectID;

use super::flux::Flux;
use super::frequence;
use super::negociation::Negociation;
use super::peripheriques::InfoPeripherique;
use super::rendu::EtatDuFlux;
use super::routage::Routage;
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

pub struct BackendCoreAudio {
    routage: Routage,
    /// Les fréquences à remettre à l'arrêt, par périphérique. Hean ne laisse pas le DAC dans
    /// un état que personne n'a demandé.
    frequences_dorigine: HashMap<AudioObjectID, u32>,
    pub(super) flux: Option<Flux>,
    pub(super) producteur: Option<Producteur>,
    pub(super) jauge: Option<Jauge>,
    pub(super) etat: Option<Arc<EtatDuFlux>>,
    negociation: Option<Negociation>,
    preroll: usize,
    pub(super) demarre: bool,
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
            negociation: None,
            preroll: 0,
            demarre: false,
        })
    }

    /// Le résultat de la dernière négociation : ce qui entre, ce qui sort. À lire **après
    /// chaque `preparer`**, parce que c'est ce que l'écran doit afficher.
    pub fn negociation(&self) -> Option<Negociation> {
        self.negociation
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

    /// Coupe la lecture **tout de suite**, en jetant ce qui attend, et rend le matériel.
    ///
    /// C'est le pendant de [`BackendAudio::terminer`], qui lui *attend* la vidange du tampon
    /// pour ne pas couper la fin d'une piste. Les deux existent parce que les deux situations
    /// sont opposées : à la fin d'un album on veut entendre les dernières secondes ; quand
    /// Thomas clique une autre piste, les entendre serait un défaut — il attendrait jusqu'à
    /// cinq secondes avant que sa piste démarre, en écoutant la précédente.
    pub fn interrompre(&mut self) {
        self.declarer_fin_de_flux();
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
            // réseau, et macOS y rééchantillonne de toute façon en interne. Un AirPlay n'est
            // donc jamais bit-perfect, et ça ne se corrige pas : ça se dit.
            frequence::frequence_du_peripherique(id)?
        } else {
            // LE REPLI, PORTÉ TEL QUEL DE V1 (`find_best_supported_rate`). Quand le
            // périphérique sait servir la fréquence du fichier, c'est elle — et c'est le
            // chemin bit-perfect. Sinon on prend la plus proche au-dessus, ce qui fait
            // rééchantillonner l'AUHAL, ce qui est une altération, et ce que l'écran affiche.
            let cible = frequence::meilleure_frequence_supportee(format.frequence, &info.frequences);
            // La fréquence d'origine est mémorisée AVANT la première modification, pour
            // pouvoir la remettre : Hean ne laisse pas le DAC dans un état que personne n'a
            // demandé.
            let actuelle = frequence::frequence_du_peripherique(id)?;
            self.frequences_dorigine.entry(id).or_insert(actuelle);
            if actuelle != cible {
                frequence::definir_frequence_du_peripherique(id, cible)?;
            }
            cible
        };

        let negociation = Negociation {
            source: format,
            sortie_hz: hz_du_peripherique,
            airplay: info.est_airplay(),
        };
        self.negociation = Some(negociation);
        if !negociation.sans_reechantillonnage() {
            // `info!` et non `warn!` : le repli fonctionne, ce n'est pas une panne. V1 faisait
            // le même choix côté écran — la classe `resampled` est cyan, délibérément pas
            // rouge (`playback.js:890`).
            tracing::info!(
                peripherique = %info.nom,
                source_hz = format.frequence,
                sortie_hz = hz_du_peripherique,
                "repli de fréquence : l'AUHAL rééchantillonne, l'écran l'affiche"
            );
        }

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
            sortie_hz = hz_du_peripherique,
            bit_perfect = negociation.sans_reechantillonnage(),
            "sortie préparée"
        );
        Ok(obtenu)
    }

    /// Pousse ce qui rentre dans le tampon et rend combien a été pris. **Ne bloque jamais** :
    /// un appelant qui reçoit moins que ce qu'il a offert rappelle plus tard avec le reste.
    fn ecrire(&mut self, echantillons: &[i32]) -> Result<usize, ErreurAudio> {
        let producteur = self
            .producteur
            .as_mut()
            .ok_or_else(|| ErreurAudio::FormatRefuse {
                demande: "écriture avant préparation".to_owned(),
                obtenu: "aucun flux ouvert".to_owned(),
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
        // Avant d'attendre la vidange : les callbacks qui vont suivre tombent à vide par
        // construction, et ce ne sont pas des trous.
        self.declarer_fin_de_flux();
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
        let tampons_inattendus = self.tampons_inattendus();
        self.fermer_le_flux();
        self.routage
            .rendre_le_materiel(&mut self.frequences_dorigine);
        if famines > 0 {
            tracing::warn!(famines, "la lecture a eu des trous : le décodeur n'a pas suivi");
        }
        if tampons_inattendus > 0 {
            tracing::warn!(
                tampons_inattendus,
                "CoreAudio a donné des tampons désentrelacés : notre hypothèse de format est                  fausse, et ces callbacks ont été servis en silence"
            );
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

