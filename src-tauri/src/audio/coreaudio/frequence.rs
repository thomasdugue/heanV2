//! La négociation de fréquence : lire, écrire, vérifier, et choisir un repli.
//!
//! C'est **le** module du bit-perfect, et la règle du produit n'est pas « jamais de
//! rééchantillonnage » : c'est **« bit-perfect quand le DAC sait décoder, et le dire quand
//! il ne sait pas »**. La fréquence du périphérique suit donc celle du fichier quand elle
//! peut, et sinon on replie sur la plus proche au-dessus — ce qui fait rééchantillonner
//! l'AUHAL, ce qui est une altération, et ce qui **s'affiche** (fréquence entrante, fréquence
//! sortante, pas de badge bit-perfect).
//!
//! ⚠️ **Ne pas confondre avec « aucun rééchantillonnage », formule qu'on lit encore dans le
//! skill `hean-invariants`.** Prise au pied de la lettre, elle ferait refuser le repli, donc
//! supprimer une fonction que Thomas veut. Ce qui est interdit, c'est le rééchantillonnage
//! par confort, par optimisation ou par défaut de conception — pas celui qu'impose un
//! périphérique qui ne sait pas faire autrement.
//!
//! Porté de `get_device_sample_rate`, `set_device_sample_rate_internal`,
//! `get_supported_sample_rates` et `find_best_supported_rate` de V1.

use std::time::Duration;

use coreaudio_sys::{
    kAudioDevicePropertyAvailableNominalSampleRates, kAudioDevicePropertyNominalSampleRate,
    AudioObjectID, AudioValueRange,
};

use super::hal;
use crate::audio::error::ErreurAudio;

/// Les fréquences dites « audiophiles ». Reprise telle quelle de `SampleRate::STANDARD_RATES`
/// de V1. C'est elle que le repli consulte (`meilleure_frequence_supportee`), exactement
/// comme V1, et c'est aussi elle que l'écran montre.
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

/// La fréquence à demander au périphérique pour jouer une source à `source_hz`.
///
/// Porté **tel quel** de `find_best_supported_rate` (V1, `coreaudio_backend.rs:761`) :
/// correspondance exacte, sinon la plus petite fréquence supportée **supérieure ou égale**,
/// sinon la plus haute disponible. Quand la liste est vide, 44 100 — qui échouera au réglage,
/// et c'est préférable à un choix inventé.
///
/// Rendre autre chose que `source_hz` veut dire que l'AUHAL va rééchantillonner. Ce n'est
/// **pas** un défaut : c'est le repli que Thomas veut, et l'appelant doit l'**afficher**
/// (voir [`super::negociation::Negociation`]).
///
/// ⚠️ La liste reçue est celle des huit fréquences **standard** que le périphérique annonce,
/// pas ses plages brutes — exactement comme V1. Un périphérique qui annoncerait une plage
/// continue 8–96 kHz jouerait donc un MP3 en 32 kHz replié à 44,1 kHz, alors qu'il saurait
/// le servir nativement. C'est le comportement de V1, repris sans le changer : « porter
/// d'abord », et aucun défaut n'a été mesuré ici. Noté comme question ouverte.
pub fn meilleure_frequence_supportee(source_hz: u32, supportees: &[u32]) -> u32 {
    if supportees.is_empty() {
        return 44_100;
    }
    if supportees.contains(&source_hz) {
        return source_hz;
    }
    if let Some(&au_dessus) = supportees.iter().filter(|&&f| f >= source_hz).min() {
        return au_dessus;
    }
    supportees.iter().copied().max().unwrap_or(44_100)
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

    #[test]
    fn aucune_plage_annoncee_ne_sert_rien() {
        assert!(!dans_les_plages(44_100, &[]));
    }

    /// Le repli, porté tel quel de V1 : exact, sinon au-dessus, sinon le plus haut.
    #[test]
    fn le_repli_prefere_la_frequence_exacte_puis_celle_juste_au_dessus() {
        let dac = [44_100, 48_000, 88_200, 96_000];
        // Exact : aucun rééchantillonnage, c'est le cas bit-perfect.
        assert_eq!(meilleure_frequence_supportee(96_000, &dac), 96_000);
        assert_eq!(meilleure_frequence_supportee(44_100, &dac), 44_100);
        // Au-dessus : un MP3 en 32 kHz monte à 44,1 — la fréquence AUGMENTE.
        assert_eq!(meilleure_frequence_supportee(32_000, &dac), 44_100);
        assert_eq!(meilleure_frequence_supportee(22_050, &dac), 44_100);
        assert_eq!(meilleure_frequence_supportee(64_000, &dac), 88_200);
        // Rien au-dessus : le plus haut disponible, et là la fréquence DESCEND.
        assert_eq!(meilleure_frequence_supportee(192_000, &dac), 96_000);
        assert_eq!(meilleure_frequence_supportee(384_000, &dac), 96_000);
    }

    /// Le cas réel du Mac mini de Thomas, relevé le 2026-11-26 : ses haut-parleurs
    /// plafonnent à 96 kHz. Un 24/192 s'y joue donc replié, et l'app doit afficher
    /// « 192 kHz / 24 bit → 96 kHz » sans badge bit-perfect.
    #[test]
    fn un_24_192_sur_les_haut_parleurs_du_mac_mini_se_replie_a_96() {
        let haut_parleurs = [44_100, 48_000, 88_200, 96_000];
        assert_eq!(meilleure_frequence_supportee(192_000, &haut_parleurs), 96_000);
    }

    /// Liste vide : 44 100, qui échouera au réglage. Un choix inventé qui « marche » serait
    /// pire — il jouerait à une fréquence que personne n'a vérifiée.
    #[test]
    fn sans_aucune_frequence_annoncee_le_repli_est_44_100() {
        assert_eq!(meilleure_frequence_supportee(192_000, &[]), 44_100);
    }
}
