//! [`Negociation`] — ce que la sortie a réellement obtenu, et donc ce qu'il faut afficher.
//!
//! # La règle du produit, dans les mots de Thomas
//!
//! > Hean n'est pas bit-perfect en permanence ; il l'est quand le DAC sait décoder, et il le
//! > dit. Quand il y a repli, il affiche la fréquence entrante et la fréquence sortante pour
//! > que l'utilisateur sache qu'il y a altération.
//!
//! Ce n'est donc **pas** « jamais de rééchantillonnage ». Ce qui est interdit, c'est le
//! rééchantillonnage par confort, par optimisation ou par défaut de conception. Celui
//! qu'impose un périphérique qui ne sait pas faire autrement est permis, **et il s'affiche**.
//!
//! V1 tenait déjà exactement ça, et c'est le point qu'on perd si on ne lit que le Rust :
//! `audio_engine.rs:540-549` calcule `is_mismatch = source_sr != output_sr` et l'émet dans
//! `playback_audio_specs` ; `src/playback.js:862-903` affiche deux champs — source
//! (`192kHz/24bit`) et sortie — et pose la classe `bit-perfect` ou `resampled` (cyan,
//! délibérément pas rouge : le repli fonctionne, ce n'est pas une panne) ;
//! `src/fullscreen-player.js:338-350` recopie l'état dans le lecteur plein écran.
//!
//! Ce type-ci est donc l'équivalent de `AudioSpecs` de V1 : les deux chiffres, et de quoi
//! savoir si le badge s'allume.

use crate::audio::format::FormatAudio;

/// Le résultat d'une négociation de sortie : ce qui entre, ce qui sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Negociation {
    /// Le format du fichier, tel que le décodeur le rend.
    pub source: FormatAudio,
    /// La fréquence à laquelle le périphérique tourne **réellement**, après négociation.
    /// Différente de `source.frequence` quand il y a repli.
    pub sortie_hz: u32,
    /// Vrai quand la sortie est un AirPlay. Sa fréquence n'a alors **pas** été touchée
    /// (PLAT-1 : y toucher tue la session réseau), et macOS rééchantillonne en interne.
    /// Un AirPlay n'est donc jamais bit-perfect, et ça ne se corrige pas : ça se dit.
    pub airplay: bool,
}

impl Negociation {
    /// Vrai quand le périphérique tourne exactement à la fréquence du fichier.
    ///
    /// C'est l'`is_mismatch` de V1, dans l'autre sens. **C'est la seule des quatre
    /// conditions du bit-perfect que ce type connaisse** : le volume et le mode exclusif
    /// sont ailleurs, et l'égaliseur n'existe pas encore (ADR 0007).
    pub fn sans_reechantillonnage(&self) -> bool {
        self.source.frequence == self.sortie_hz
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::format::Profondeur;

    fn source(hz: u32) -> FormatAudio {
        FormatAudio::nouveau(hz, 2, Profondeur::Bits24)
    }

    #[test]
    fn meme_frequence_des_deux_cotes_veut_dire_aucun_reechantillonnage() {
        let n = Negociation {
            source: source(96_000),
            sortie_hz: 96_000,
            airplay: false,
        };
        assert!(n.sans_reechantillonnage());
    }

    /// Le cas du Mac mini de Thomas : un 24/192 sur des haut-parleurs qui plafonnent à
    /// 96 kHz. Ça joue — c'est le repli que Thomas veut — mais ce n'est pas bit-perfect.
    #[test]
    fn un_repli_vers_le_bas_nest_pas_bit_perfect() {
        let n = Negociation {
            source: source(192_000),
            sortie_hz: 96_000,
            airplay: false,
        };
        assert!(!n.sans_reechantillonnage());
    }

    /// Et un repli vers le HAUT non plus. C'est le cas qu'on oublie : un MP3 en 32 kHz monte
    /// à 44,1 kHz, la fréquence augmente, et le signal est tout de même rééchantillonné.
    #[test]
    fn un_repli_vers_le_haut_nest_pas_bit_perfect_non_plus() {
        let n = Negociation {
            source: source(32_000),
            sortie_hz: 44_100,
            airplay: false,
        };
        assert!(!n.sans_reechantillonnage());
    }

    /// Un AirPlay dont la fréquence native coïncide par hasard avec celle du fichier. Le
    /// calcul de fréquence dit « pas de rééchantillonnage », et il a raison **de son point
    /// de vue** : nous n'en faisons pas. macOS, lui, en fait — d'où `airplay`, que
    /// l'appelant doit regarder en plus (PLAT-1).
    #[test]
    fn un_airplay_a_la_bonne_frequence_reste_marque_airplay() {
        let n = Negociation {
            source: source(44_100),
            sortie_hz: 44_100,
            airplay: true,
        };
        assert!(n.sans_reechantillonnage(), "notre chaîne ne rééchantillonne pas");
        assert!(n.airplay, "mais macOS le fait, et l'appelant doit le savoir");
    }
}
