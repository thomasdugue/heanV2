//! Ce que le backend laisse **observer et commander** pendant qu'il joue.
//!
//! Séparé de [`super::backend`], qui porte la négociation et le cycle de vie du flux. La
//! ligne de partage a un sens : là-bas on ouvre et on ferme, ici on regarde et on règle — et
//! tout ce qui est ici passe par des atomiques, donc n'attend jamais.
//!
//! Les deux compteurs sont le cœur du fichier. V1 n'en avait **aucun** : une coupure dans le
//! son ne laissait aucune trace, et « j'ai cru entendre un craquement » n'était pas
//! vérifiable. Mais un compteur qui compte le chemin normal se fait retirer par celui qui le
//! lit, d'où `declarer_fin_de_flux` : voir [`super::rendu::EtatDuFlux::famines`].

use std::sync::Arc;
use std::time::{Duration, Instant};

use std::sync::atomic::Ordering;

use super::backend::BackendCoreAudio;
use super::rendu::EtatDuFlux;
use super::tampon::Jauge;
use crate::audio::error::ErreurAudio;

/// Plafond d'attente de la purge du tampon par le callback. Un callback arrive toutes les
/// quelques millisecondes ; 200 ms veut dire qu'il n'en vient plus, et la lecture ne doit pas
/// bloquer pour autant.
const ATTENTE_DE_PURGE_MAX: Duration = Duration::from_millis(200);

impl BackendCoreAudio {
    /// L'état partagé avec le callback : pause, volume, compteur de famines.
    pub fn etat_du_flux(&self) -> Option<Arc<EtatDuFlux>> {
        self.etat.clone()
    }

    /// Les compteurs du tampon, pour savoir où en est la lecture.
    pub fn jauge(&self) -> Option<Jauge> {
        self.jauge.clone()
    }

    /// Met la lecture en pause sans rien consommer : la reprise est instantanée, et le
    /// tampon n'est pas jeté.
    pub fn definir_pause(&self, en_pause: bool) {
        if let Some(etat) = &self.etat {
            etat.en_pause
                .store(en_pause, Ordering::Relaxed);
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
                .store(true, Ordering::Release);
            let limite = Instant::now() + ATTENTE_DE_PURGE_MAX;
            while etat
                .purge_demandee
                .load(Ordering::Acquire)
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

    /// Dit au callback qu'il n'y a plus rien à décoder.
    ///
    /// Sans ça, tous les callbacks entre la dernière trame et l'arrêt de l'AudioUnit sont
    /// comptés comme des trous — donc **chaque fin d'album** en produisait. Un compteur
    /// d'anomalie qui compte le chemin normal se fait retirer, et c'est l'aveuglement qui
    /// revient. Relevé par `invariant-guardian`.
    pub fn declarer_fin_de_flux(&self) {
        if let Some(etat) = &self.etat {
            etat.fin_de_flux
                .store(true, Ordering::Relaxed);
        }
    }

    /// Journalise les trois compteurs d'anomalie qui ne sont pas nuls.
    ///
    /// ⚠️ **À appeler sur un chemin que la production emprunte vraiment.** Les deux premiers
    /// compteurs n'étaient lus que dans `BackendAudio::terminer`, que rien n'appelle sur ce
    /// backend : un `fetch_add` et un getter ne font pas une trace, et des lecteurs tous
    /// situés dans une fonction morte sont pires que pas de compteur — le défaut reste muet
    /// **et** on croit le contraire. Même famille qu'INV-21 (🔴). Relevé par
    /// `invariant-guardian`.
    pub fn journaliser_les_anomalies(&self) {
        let famines = self.famines();
        if famines > 0 {
            tracing::warn!(famines, "la lecture a eu des trous : le décodeur n'a pas suivi");
        }
        let desentrelaces = self.tampons_inattendus();
        if desentrelaces > 0 {
            tracing::warn!(
                tampons_inattendus = desentrelaces,
                "CoreAudio a donné des tampons désentrelacés : notre hypothèse de format est fausse, et ces callbacks ont été servis en silence"
            );
        }
        let trop_grandes = self
            .etat
            .as_ref()
            .map(|e| e.demandes_trop_grandes.load(Ordering::Relaxed))
            .unwrap_or(0);
        if trop_grandes > 0 {
            tracing::warn!(
                demandes_trop_grandes = trop_grandes,
                "CoreAudio a demandé plus de trames que le tampon de travail n'en tient : ces callbacks ont été servis en silence, et TRAMES_MAX_PAR_CALLBACK est à relever"
            );
        }
    }

    /// Callbacks où CoreAudio n'a pas donné un tampon entrelacé unique. Doit rester à zéro.
    pub fn tampons_inattendus(&self) -> u64 {
        self.etat
            .as_ref()
            .map(|e| e.tampons_inattendus.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Le nombre de callbacks servis à vide depuis l'ouverture du flux : la mesure de « ça
    /// ne coupe pas ». Chaque unité est un trou audible.
    pub fn famines(&self) -> u64 {
        self.etat
            .as_ref()
            .map(|e| e.famines.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

}
