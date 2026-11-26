//! Ce que le fil publie pour l'écran : la position, et le badge bit-perfect.
//!
//! Séparé de [`super::fil`], qui porte la boucle, et de [`super::pilotage`], qui porte les
//! commandes. La ligne de partage est celle des régimes : là-bas on décide et on avance, ici
//! on **constate**. Rien dans ce fichier ne change l'état de la lecture.
//!
//! La subtilité est que la piste qu'on **entend** n'est pas celle que le décodeur lit : à
//! cinq secondes de tampon, ils peuvent être deux pistes différentes. C'est la file `en_vol`
//! qui tranche, et sa tête est l'audible — V1 avait ce défaut exact, son `current_path`
//! désignant la piste en cours de décodage.

use std::sync::atomic::Ordering;

use super::fil::Fil;

impl Fil {
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

    pub(super) fn publier(&mut self) {
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
            let total = self.famines_des_flux_fermes + backend.famines();
            self.etat.famines.store(total, Ordering::Relaxed);
        }
        // Les quatre conditions du bit-perfect, et elles sont toutes nécessaires : pas de
        // rééchantillonnage, volume exactement à 100 %, mode exclusif, et l'égaliseur plat
        // — ce dernier est vrai par construction tant qu'il n'existe pas (ADR 0007).
        let bit_perfect = self.etat.sans_reechantillonnage.load(Ordering::Relaxed)
            && self.etat.volume().est_unite()
            && self.etat.mode_exclusif.load(Ordering::Relaxed);
        self.etat.bit_perfect.store(bit_perfect, Ordering::Relaxed);
    }
}
