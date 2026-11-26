//! La négociation de fréquence : lire, écrire, vérifier, et savoir ce que le DAC sait faire.
//!
//! C'est **le** module du bit-perfect. Rééchantillonner est interdit (ADR 0002), donc la
//! fréquence du périphérique suit celle du fichier — et quand elle ne peut pas, personne ne
//! joue à la place (ADR 0018).
//!
//! Porté de `get_device_sample_rate`, `set_device_sample_rate_internal` et
//! `get_supported_sample_rates` de V1.

use std::time::Duration;

use coreaudio_sys::{
    kAudioDevicePropertyAvailableNominalSampleRates, kAudioDevicePropertyNominalSampleRate,
    AudioObjectID, AudioValueRange,
};

use super::hal;
use crate::audio::error::ErreurAudio;

/// Les fréquences dites « audiophiles ». Reprise telle quelle de `SampleRate::STANDARD_RATES`
/// de V1, et **elle ne sert qu'à l'affichage** — voir `est_servie` pour pourquoi la décision
/// de jouer ou non ne passe pas par cette liste.
pub const FREQUENCES_STANDARD: [u32; 8] = [
    44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000,
];

/// Combien de temps on laisse au matériel pour confirmer un changement de fréquence.
/// 100 ms, la valeur de V1 : un DAC USB reverrouille sa PLL, ce n'est pas instantané.
const DELAI_DE_CONFIRMATION: Duration = Duration::from_millis(100);

/// La fréquence à laquelle le périphérique tourne en ce moment.
pub fn frequence_du_peripherique(id: AudioObjectID) -> Result<u32, ErreurAudio> {
    let adr = hal::globale(kAudioDevicePropertyNominalSampleRate);
    let hz: f64 = hal::lire(id, &adr, 0.0f64, "lire la fréquence du périphérique")?;
    Ok(hz.round() as u32)
}

/// Les plages de fréquences que le périphérique annonce servir, **brutes**.
///
/// Une plage dont le minimum égale le maximum est une fréquence discrète (le cas usuel) ;
/// une vraie plage existe sur certains périphériques agrégés, qui acceptent tout l'intervalle.
pub fn plages_de_frequences(id: AudioObjectID) -> Result<Vec<AudioValueRange>, ErreurAudio> {
    let adr = hal::globale(kAudioDevicePropertyAvailableNominalSampleRates);
    hal::lire_tableau(id, &adr, "lire les fréquences disponibles")
}

/// Les fréquences **standard** que le périphérique sait servir, triées. C'est la liste que
/// V1 appelait `get_supported_sample_rates`, et c'est celle qu'on montre à l'écran.
pub fn frequences_supportees(id: AudioObjectID) -> Result<Vec<u32>, ErreurAudio> {
    let plages = plages_de_frequences(id)?;
    let mut hz: Vec<u32> = FREQUENCES_STANDARD
        .iter()
        .copied()
        .filter(|&f| dans_les_plages(f, &plages))
        .collect();
    hz.sort_unstable();
    hz.dedup();
    Ok(hz)
}

/// Vrai si `hz` tombe dans une des plages annoncées.
pub fn dans_les_plages(hz: u32, plages: &[AudioValueRange]) -> bool {
    let cible = f64::from(hz);
    plages
        .iter()
        .any(|p| cible >= p.mMinimum && cible <= p.mMaximum)
}

/// Vrai si le périphérique sait servir **exactement** cette fréquence.
///
/// ⚠️ **Et c'est pour cette fonction que `plages_de_frequences` existe au lieu de réutiliser
/// `frequences_supportees`.** Décider ici sur la liste des huit fréquences standard
/// introduirait un défaut que V1 n'avait pas : V1, ne trouvant pas la fréquence dans sa
/// liste, en choisissait une autre et jouait quand même (`MESURES-V1.md` §11). V2 refuse —
/// donc une fréquence hors liste mais **réellement servie par le matériel** ferait refuser un
/// fichier que V1 jouait. Le cas est concret : un MP3 en 32 kHz ou 22,05 kHz, et Thomas en a
/// 82. On interroge donc le matériel, jamais la liste d'affichage.
pub fn est_servie(id: AudioObjectID, hz: u32) -> Result<bool, ErreurAudio> {
    Ok(dans_les_plages(hz, &plages_de_frequences(id)?))
}

/// Règle la fréquence du périphérique, **puis la relit pour vérifier**.
///
/// La relecture n'est pas de la prudence décorative : `AudioObjectSetPropertyData` peut
/// rendre `0` sans que le matériel ait suivi, et la différence entre « réglé » et « demandé »
/// est exactement celle entre bit-perfect et rééchantillonnage silencieux.
pub fn definir_frequence_du_peripherique(
    id: AudioObjectID,
    hz: u32,
) -> Result<(), ErreurAudio> {
    let adr = hal::globale(kAudioDevicePropertyNominalSampleRate);
    let demande = f64::from(hz);
    hal::ecrire(id, &adr, &demande, "régler la fréquence du périphérique")?;

    std::thread::sleep(DELAI_DE_CONFIRMATION);

    let obtenue = frequence_du_peripherique(id)?;
    if obtenue != hz {
        tracing::warn!(
            peripherique = id,
            demandee = hz,
            obtenue,
            "le périphérique n'a pas pris la fréquence demandée"
        );
        return Err(ErreurAudio::FormatRefuse {
            demande: format!("{hz} Hz"),
            obtenu: format!("{obtenue} Hz"),
        });
    }
    tracing::info!(peripherique = id, hz, "fréquence du périphérique réglée");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plage(min: u32, max: u32) -> AudioValueRange {
        AudioValueRange {
            mMinimum: f64::from(min),
            mMaximum: f64::from(max),
        }
    }

    /// Le cas usuel : un DAC annonce des fréquences discrètes, une plage par fréquence.
    #[test]
    fn des_frequences_discretes_se_reconnaissent_une_par_une() {
        let plages = [plage(44_100, 44_100), plage(48_000, 48_000), plage(96_000, 96_000)];
        for servie in [44_100, 48_000, 96_000] {
            assert!(dans_les_plages(servie, &plages), "{servie} Hz");
        }
        for absente in [22_050, 88_200, 192_000, 384_000] {
            assert!(!dans_les_plages(absente, &plages), "{absente} Hz");
        }
    }

    /// Un périphérique agrégé peut annoncer un intervalle continu. Tout ce qui est dedans
    /// est servi — y compris des fréquences qui ne sont dans aucune liste « standard ».
    #[test]
    fn une_plage_continue_sert_tout_ce_quelle_contient() {
        let plages = [plage(8_000, 96_000)];
        for servie in [8_000, 22_050, 32_000, 44_100, 96_000] {
            assert!(dans_les_plages(servie, &plages), "{servie} Hz");
        }
        assert!(!dans_les_plages(192_000, &plages));
    }

    /// LE DÉFAUT QUE CE MODULE EXISTE POUR NE PAS INTRODUIRE. Un MP3 en 32 kHz n'est dans
    /// aucune des huit fréquences standard ; si la décision de jouer passait par cette
    /// liste, Hean refuserait un fichier que le matériel sait parfaitement servir — et V1,
    /// lui, le jouait. Refuser plus que nécessaire serait une régression, pas une rigueur.
    #[test]
    fn une_frequence_hors_liste_standard_mais_servie_ne_doit_pas_etre_refusee() {
        let plages = [plage(32_000, 32_000), plage(44_100, 44_100)];
        assert!(!FREQUENCES_STANDARD.contains(&32_000), "prémisse du test");
        assert!(
            dans_les_plages(32_000, &plages),
            "le matériel la sert : la décision doit suivre le matériel, pas la liste d'affichage"
        );
    }

    #[test]
    fn aucune_plage_annoncee_ne_sert_rien() {
        assert!(!dans_les_plages(44_100, &[]));
    }
}
