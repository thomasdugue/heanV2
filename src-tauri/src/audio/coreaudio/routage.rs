//! Quel périphérique joue, et comment on y route le son — c'est-à-dire **PLAT-1 (🔴)**.
//!
//! Porté de `set_output_device`, `get_active_device_id`, `get_device_id` et
//! `check_device_change` de V1. Cette stratégie ne se déduit d'aucune documentation Apple ;
//! elle a coûté des semaines à V1, et la reprendre à l'identique est tout l'intérêt de ce
//! fichier.
//!
//! # La stratégie du défaut système, en trois cas
//!
//! | on va vers | ce qu'on fait du défaut système | pourquoi |
//! |---|---|---|
//! | un AirPlay | **on le met** sur l'AirPlay, puis on attend 800 ms | c'est le seul moyen d'exister : hors défaut système, macOS tue la session |
//! | autre chose, alors que **le défaut système EST un AirPlay** | **on n'y touche pas** | changer le défaut tuerait la session AirPlay, qui ne se rouvre pas depuis le HAL. Le son part sur le nouveau périphérique par assignement explicite sur l'AudioUnit |
//! | autre chose, et le défaut système n'est pas un AirPlay | on le met | pour que les touches de volume et l'indicateur de macOS suivent |
//!
//! ⚠️ **La deuxième ligne pose la bonne question, et ce n'était pas le cas avant le
//! 2026-11-26.** Le code demandait « est-ce que je *viens* d'un AirPlay ? », en lisant le
//! périphérique précédent. Les deux questions divergent au **deuxième** changement :
//! AirPlay → DAC → haut-parleurs. Au troisième saut le périphérique précédent est le DAC,
//! donc « je ne viens pas d'un AirPlay », donc on écrivait le défaut système — alors qu'il
//! pointait encore sur l'AirPlay, dont la session était ainsi tuée. V1 a exactement le même
//! défaut (`coreaudio_backend.rs:946-949`), et son commentaire énonce l'intention complète
//! (« *or if system default is currently AirPlay* ») que son code ne tient pas. Relevé par
//! `invariant-guardian` ; corrigé ici, parce qu'un tableau qui se lit comme une stratégie
//! complète ne doit pas être contredit par la ligne d'en dessous.
//!
//! **Limitation connue, et cosmétique** : dans le deuxième cas, macOS continue d'afficher
//! « AirPlay » dans sa notification de volume alors que le son sort ailleurs. Le volume agit
//! bien sur le bon périphérique. V1 vivait avec, et c'est le prix de la préservation.

use std::collections::HashMap;
use std::time::Duration;

use coreaudio_sys::AudioObjectID;

use super::cache::CachePeripheriques;
use super::exclusif;
use super::peripheriques::{self, InfoPeripherique};
use crate::audio::error::ErreurAudio;

/// Le temps laissé à un récepteur AirPlay pour ouvrir sa session réseau après avoir été
/// désigné défaut système. 800 ms, la valeur de V1 : sans cette attente, l'AudioUnit démarre
/// avant que l'AirPlay soit prêt et le son part sur l'ancien périphérique.
const ATTENTE_ACTIVATION_AIRPLAY: Duration = Duration::from_millis(800);

pub struct Routage {
    cache: CachePeripheriques,
    /// Périphérique choisi à la main. `None` : on suit le défaut système, donc brancher un
    /// casque bascule dessus comme le reste du Mac.
    choisi: Option<AudioObjectID>,
    /// Le dernier périphérique actif connu, pour voir qu'il a changé.
    dernier: AudioObjectID,
    mode_exclusif: bool,
    /// Vrai quand `choisi` l'a été par le mode exclusif plutôt que par l'utilisateur : le
    /// verrou doit alors être rendu quand on repasse en partagé.
    verrouille_par_exclusif: bool,
}

impl Routage {
    pub fn nouveau() -> Result<Self, ErreurAudio> {
        let defaut = peripheriques::peripherique_par_defaut()?;
        let mut routage = Self {
            cache: CachePeripheriques::default(),
            choisi: None,
            dernier: defaut,
            mode_exclusif: false,
            verrouille_par_exclusif: false,
        };
        routage.cache.rafraichir()?;
        let nom = routage
            .cache
            .info(defaut)
            .map(|i| i.nom.clone())
            .unwrap_or_else(|| defaut.to_string());
        tracing::info!(peripherique = %nom, id = defaut, "sortie audio initialisée");
        Ok(routage)
    }

    pub fn cache(&self) -> &CachePeripheriques {
        &self.cache
    }

    pub fn rafraichir(&mut self) -> Result<(), ErreurAudio> {
        self.cache.rafraichir()
    }

    pub fn liste(&self) -> Vec<InfoPeripherique> {
        self.cache.liste()
    }

    /// Le périphérique qui doit jouer : celui choisi à la main, sinon le défaut système.
    pub fn peripherique_actif(&self) -> Result<AudioObjectID, ErreurAudio> {
        match self.choisi {
            Some(id) => Ok(id),
            None => peripheriques::peripherique_par_defaut(),
        }
    }

    pub fn info_active(&self) -> Option<&InfoPeripherique> {
        self.peripherique_actif().ok().and_then(|id| self.cache.info(id))
    }

    pub fn mode_exclusif(&self) -> bool {
        self.mode_exclusif
    }

    /// L'identifiant à assigner à l'AudioUnit, ou `None` pour laisser le défaut système.
    ///
    /// `None` pour un AirPlay, et ce n'est pas un repli :
    /// `AudioUnitSetProperty(kAudioOutputUnitProperty_CurrentDevice)` **échoue
    /// systématiquement** sur un AirPlay. Le routage passe alors par le défaut système, que
    /// [`Self::definir_peripherique`] vient justement d'y poser.
    pub fn id_pour_audiounit(&self) -> Result<Option<AudioObjectID>, ErreurAudio> {
        let id = self.peripherique_actif()?;
        let airplay = self.cache.info(id).is_some_and(InfoPeripherique::est_airplay);
        Ok(if airplay { None } else { Some(id) })
    }

    /// Voit si le périphérique actif a changé sous nos pieds (casque branché, DAC débranché)
    /// et rend le nouveau le cas échéant.
    ///
    /// Le mode exclusif est relâché sur l'**ancien** périphérique avant tout : un
    /// périphérique débranché alors qu'on le tenait encore refuse ensuite de se rouvrir.
    pub fn verifier_changement(&mut self) -> Option<AudioObjectID> {
        let actuel = self.peripherique_actif().ok()?;
        if actuel == self.dernier {
            return None;
        }
        let ancien = self.dernier;
        if self.mode_exclusif {
            let _ = exclusif::desactiver_mode_exclusif(ancien);
        }
        self.dernier = actuel;
        let _ = self.cache.rafraichir();

        // Un périphérique choisi à la main qui a disparu : on revient au défaut système,
        // plutôt que de rester pointé sur un identifiant mort.
        if let Some(choisi) = self.choisi {
            if !self.cache.contient(choisi) {
                tracing::info!(choisi, "le périphérique choisi a disparu — retour au défaut système");
                self.choisi = None;
            }
        }
        let nom = |id: AudioObjectID| {
            self.cache.info(id).map(|i| i.nom.clone()).unwrap_or_else(|| id.to_string())
        };
        tracing::info!(avant = %nom(ancien), apres = %nom(actuel), "la sortie audio a changé");
        Some(actuel)
    }

    /// Choisit un périphérique de sortie. C'est ici que vit la stratégie du défaut système.
    pub fn definir_peripherique(&mut self, demande: AudioObjectID) -> Result<(), ErreurAudio> {
        if !self.cache.contient(demande) {
            self.cache.rafraichir()?;
            if !self.cache.contient(demande) {
                return Err(ErreurAudio::AucunPeripherique);
            }
        }
        // Un identifiant AirPlay périmé se résout vers l'identifiant vivant : écrire sur le
        // mort serait accepté sans aucun effet (PLAT-1).
        let id = self.cache.resoudre(demande)?;
        let vers_airplay = self.cache.info(id).is_some_and(InfoPeripherique::est_airplay);
        // La question est « le défaut système EST-IL un AirPlay ? », et non « est-ce que je
        // viens d'un AirPlay ? » — voir l'avertissement en tête de fichier.
        let defaut_systeme_est_airplay = peripheriques::peripherique_par_defaut()
            .ok()
            .and_then(|defaut| self.cache.info(defaut))
            .is_some_and(InfoPeripherique::est_airplay);

        if self.mode_exclusif {
            let _ = exclusif::desactiver_mode_exclusif(self.dernier);
            if vers_airplay {
                // Le mode exclusif est incompatible avec une diffusion réseau : il est
                // désactivé maintenant, sans quoi l'état resterait « exclusif » à tort.
                self.mode_exclusif = false;
                self.verrouille_par_exclusif = false;
                tracing::info!("mode exclusif désactivé : incompatible avec AirPlay");
            }
        }

        self.choisi = Some(id);
        self.dernier = id;
        self.verrouille_par_exclusif = false;

        if vers_airplay {
            peripheriques::definir_peripherique_par_defaut_systeme(id)?;
            std::thread::sleep(ATTENTE_ACTIVATION_AIRPLAY);
            tracing::info!(id, "sortie AirPlay activée (défaut système, attente d'activation)");
        } else if defaut_systeme_est_airplay {
            tracing::info!(
                id,
                "sortie changée sans toucher au défaut système : il pointe sur un AirPlay, \
                 dont la session est ainsi préservée (PLAT-1)"
            );
        } else {
            // ⚠️ Ici l'échec n'est PAS fatal, et c'est une correction de ma propre erreur de
            // portage : V1 ignorait le code de retour (`let _ = ...`), j'avais mis un `?`.
            // Entre deux périphériques filaires, le défaut système ne sert qu'à faire suivre
            // les touches de volume et l'indicateur de macOS — c'est **cosmétique**. Un `?`
            // aurait refusé un changement de sortie qui aurait parfaitement fonctionné.
            // Le cas AirPlay ci-dessus, lui, garde son `?` : là le défaut système est le
            // seul chemin de routage qui existe (PLAT-1), donc son échec est fatal.
            if let Err(e) = peripheriques::definir_peripherique_par_defaut_systeme(id) {
                tracing::warn!(
                    id,
                    erreur = %e,
                    "défaut système non synchronisé : la sortie change quand même, mais les \
                     touches de volume de macOS continueront d'agir sur l'ancienne"
                );
            } else {
                tracing::info!(id, "sortie changée, défaut système synchronisé");
            }
        }
        Ok(())
    }

    /// Active ou coupe le mode exclusif sur le périphérique actif.
    ///
    /// **Refusé sur un AirPlay.** Le garde-fou est ici, dans le Rust, et pas seulement dans
    /// l'interface : V1 avait les deux, parce qu'un appel IPC direct contournerait l'écran.
    pub fn definir_mode_exclusif(&mut self, actif: bool) -> Result<(), ErreurAudio> {
        let id = self.peripherique_actif()?;
        let info = self.cache.info(id);
        let nom = info.map(|i| i.nom.clone()).unwrap_or_else(|| id.to_string());
        if actif && info.is_some_and(InfoPeripherique::est_airplay) {
            return Err(ErreurAudio::CoreAudio {
                operation: format!("réserver « {nom} » : le mode exclusif est impossible en AirPlay"),
                code: 0,
            });
        }
        if actif == self.mode_exclusif {
            return Ok(());
        }
        if actif {
            exclusif::activer_mode_exclusif(id, &nom)?;
            // On se verrouille sur ce périphérique : sinon un changement de défaut système
            // ferait jouer ailleurs un son dont le périphérique réservé est encore tenu.
            self.choisi = Some(id);
            self.verrouille_par_exclusif = true;
        } else {
            exclusif::desactiver_mode_exclusif(id)?;
            if self.verrouille_par_exclusif {
                self.choisi = None;
                self.verrouille_par_exclusif = false;
            }
        }
        self.mode_exclusif = actif;
        Ok(())
    }

    /// Réapplique le mode exclusif sur le périphérique actif. Appelé juste avant d'ouvrir un
    /// flux : la réservation peut avoir été perdue en changeant de périphérique.
    pub fn reappliquer_mode_exclusif(&self) -> Result<(), ErreurAudio> {
        if !self.mode_exclusif {
            return Ok(());
        }
        let id = self.peripherique_actif()?;
        let nom = self
            .cache
            .info(id)
            .map(|i| i.nom.clone())
            .unwrap_or_else(|| id.to_string());
        exclusif::activer_mode_exclusif(id, &nom)
    }

    /// Relâche tout ce qui a été pris sur le matériel : la réservation, et les fréquences.
    pub fn rendre_le_materiel(&mut self, frequences_dorigine: &mut HashMap<AudioObjectID, u32>) {
        if self.mode_exclusif {
            if let Ok(id) = self.peripherique_actif() {
                let _ = exclusif::desactiver_mode_exclusif(id);
            }
            self.mode_exclusif = false;
            self.verrouille_par_exclusif = false;
        }
        for (id, hz) in frequences_dorigine.drain() {
            // Remettre la fréquence d'origine : Hean ne laisse pas le DAC dans un état que
            // personne n'a demandé.
            if let Err(e) = super::frequence::definir_frequence_du_peripherique(id, hz) {
                tracing::warn!(peripherique = id, hz, erreur = %e, "fréquence d'origine non restaurée");
            }
        }
    }
}
