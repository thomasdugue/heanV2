//! Le cache des périphériques, et la survie des périphériques sans fil.
//!
//! Porté de `refresh_device_cache` et de la résolution d'identifiant périmé de
//! `set_output_device` (V1). C'est le module de **PLAT-1 (🔴)**, et son existence entière
//! tient à une propriété de macOS qui ne se déduit d'aucune documentation :
//!
//! > Un périphérique AirPlay n'existe dans CoreAudio que tant qu'il est le défaut système.
//! > Dès qu'il perd ce statut, macOS tue la session et il **disparaît de la liste du HAL**.
//!
//! Sans cache de session, un AirPlay disparaîtrait donc de la liste de Hean à la seconde où
//! l'utilisateur choisit autre chose — et il ne pourrait plus y revenir. Le cache le garde
//! visible, en le marquant **périmé**, et un retour sur lui passe par la résolution de nom
//! ci-dessous : macOS lui attribue un **nouvel** identifiant à chaque réactivation, et
//! écrire sur l'ancien est accepté sans aucun effet.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use coreaudio_sys::AudioObjectID;

use super::frequence;
use super::peripheriques::{self, InfoPeripherique};
use crate::audio::error::ErreurAudio;

#[derive(Default)]
pub struct CachePeripheriques {
    /// Ce que la dernière lecture du HAL a donné, plus les sans-fil réinjectés.
    connus: HashMap<AudioObjectID, InfoPeripherique>,
    /// Les périphériques sans fil (AirPlay et Bluetooth) vus depuis le lancement de Hean.
    /// Ils survivent à leur disparition du HAL pour toute la durée de la session.
    sans_fil_de_la_session: HashMap<AudioObjectID, InfoPeripherique>,
    /// Ceux qui sont dans `connus` uniquement parce qu'ils ont été réinjectés : CoreAudio ne
    /// les voit plus, donc leur identifiant est mort.
    perimes: HashSet<AudioObjectID>,
}

impl CachePeripheriques {
    /// Relit la liste des périphériques et leurs propriétés.
    ///
    /// Un périphérique dont une propriété est illisible n'est pas écarté : il vient peut-être
    /// d'être débranché pendant la lecture, et l'omettre silencieusement de la liste serait
    /// plus déroutant que de le montrer sans sa fréquence. Les valeurs de repli ici ne
    /// décident de rien — la décision de jouer interroge le matériel (`frequence::est_servie`).
    pub fn rafraichir(&mut self) -> Result<(), ErreurAudio> {
        let ids = peripheriques::tous_les_peripheriques_de_sortie()?;
        let defaut = peripheriques::peripherique_par_defaut().ok();

        self.connus.clear();
        self.perimes.clear();

        for id in ids {
            let nom = peripheriques::nom_du_peripherique(id)
                .unwrap_or_else(|_| format!("Périphérique {id}"));
            let info = InfoPeripherique {
                id,
                nom,
                par_defaut: Some(id) == defaut,
                frequences: frequence::frequences_supportees(id).unwrap_or_default(),
                frequence_actuelle: frequence::frequence_du_peripherique(id).unwrap_or(0),
                canaux_max: peripheriques::canaux_max(id),
                transport: peripheriques::type_de_transport(id),
            };

            if info.est_airplay() || info.est_bluetooth() {
                self.memoriser_sans_fil(&info);
            }
            self.connus.insert(id, info);
        }

        // Réinjection : les sans-fil de la session que CoreAudio ne voit plus. Après la
        // déduplication par nom faite juste au-dessus, il ne reste ici que des périphériques
        // réellement absents.
        let memorises: Vec<(AudioObjectID, InfoPeripherique)> = self
            .sans_fil_de_la_session
            .iter()
            .map(|(id, info)| (*id, info.clone()))
            .collect();
        for (id, mut info) in memorises {
            if let Entry::Vacant(place) = self.connus.entry(id) {
                info.par_defaut = false; // il ne peut plus l'être : il n'existe plus pour le HAL
                place.insert(info);
                self.perimes.insert(id);
            }
        }

        tracing::debug!(
            peripheriques = self.connus.len(),
            perimes = self.perimes.len(),
            "cache des périphériques rafraîchi"
        );
        Ok(())
    }

    /// Mémorise un périphérique sans fil, **en évacuant ses identités mortes**.
    ///
    /// macOS attribue un nouvel `AudioObjectID` à chaque réactivation d'un AirPlay ou d'un
    /// Bluetooth. Sans ce nettoyage, le même casque apparaîtrait deux, trois, quatre fois
    /// dans la liste au fil d'une session — une seule de ces entrées étant vivante. V1 avait
    /// ce défaut puis l'a corrigé ; la déduplication se fait **par nom**, qui est la seule
    /// chose stable d'un périphérique sans fil.
    fn memoriser_sans_fil(&mut self, info: &InfoPeripherique) {
        let morts: Vec<AudioObjectID> = self
            .sans_fil_de_la_session
            .iter()
            .filter(|(id, memorise)| memorise.nom == info.nom && **id != info.id)
            .map(|(id, _)| *id)
            .collect();
        for mort in morts {
            tracing::debug!(
                peripherique = %info.nom,
                ancien = mort,
                nouveau = info.id,
                "le périphérique sans fil a changé d'identifiant — l'ancien est retiré"
            );
            self.sans_fil_de_la_session.remove(&mort);
        }
        self.sans_fil_de_la_session.insert(info.id, info.clone());
    }

    /// Tous les périphériques connus, triés par nom pour que la liste ne danse pas d'un
    /// rafraîchissement à l'autre (un `HashMap` n'a pas d'ordre).
    pub fn liste(&self) -> Vec<InfoPeripherique> {
        let mut tous: Vec<InfoPeripherique> = self.connus.values().cloned().collect();
        tous.sort_by(|a, b| a.nom.cmp(&b.nom).then(a.id.cmp(&b.id)));
        tous
    }

    pub fn info(&self, id: AudioObjectID) -> Option<&InfoPeripherique> {
        self.connus.get(&id)
    }

    pub fn est_perime(&self, id: AudioObjectID) -> bool {
        self.perimes.contains(&id)
    }

    pub fn contient(&self, id: AudioObjectID) -> bool {
        self.connus.contains_key(&id)
    }

    /// Rend l'identifiant **vivant** d'un périphérique, en résolvant un identifiant AirPlay
    /// périmé vers celui que macOS lui a réattribué.
    ///
    /// C'est le pas que PLAT-1 rend obligatoire : écrire le défaut système sur un identifiant
    /// AirPlay mort **réussit sans rien faire**, donc sans cette résolution Hean croirait
    /// avoir changé de sortie et le son continuerait ailleurs. Quand aucun remplaçant vivant
    /// n'existe, c'est une erreur nommée — le périphérique est vraiment parti, et Thomas doit
    /// le resélectionner dans les Réglages Son de macOS.
    pub fn resoudre(&self, id: AudioObjectID) -> Result<AudioObjectID, ErreurAudio> {
        let Some(info) = self.connus.get(&id) else {
            return Err(ErreurAudio::AucunPeripherique);
        };
        if !self.est_perime(id) {
            return Ok(id);
        }
        let vivant = self
            .connus
            .iter()
            .find(|(autre, candidat)| {
                candidat.nom == info.nom && candidat.est_airplay() && !self.est_perime(**autre)
            })
            .map(|(autre, _)| *autre);
        match vivant {
            Some(vivant) => {
                tracing::info!(
                    peripherique = %info.nom,
                    perime = id,
                    vivant,
                    "identifiant AirPlay périmé résolu vers l'identifiant actif"
                );
                Ok(vivant)
            }
            None => Err(ErreurAudio::CoreAudio {
                operation: format!(
                    "atteindre « {} » : le périphérique AirPlay n'est plus disponible, \
                     le resélectionner dans les Réglages Son de macOS",
                    info.nom
                ),
                code: 0,
            }),
        }
    }
}
