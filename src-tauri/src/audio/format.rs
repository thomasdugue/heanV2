//! Description d'un flux audio : fréquence, canaux, profondeur.
//!
//! La profondeur est le seul chiffre dont dépend l'exactitude du bit-perfect (elle commande
//! le décalage de remise à l'échelle, voir `samples.rs`). Elle ne se devine donc jamais :
//! elle vient du conteneur, ou le décodage échoue en le disant.

use std::fmt;

/// Profondeur d'un échantillon **telle qu'elle existe dans le fichier**.
///
/// Pas de variante « autre » : une profondeur inconnue doit produire une erreur, pas un
/// comportement par défaut. V1 a payé cher les valeurs de repli silencieuses (INV-04,
/// INV-17) — une profondeur fausse ne fait pas crasher l'app, elle dégrade le son, ce que
/// seul l'oreille de Thomas verrait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profondeur {
    Bits16,
    Bits24,
    Bits32,
}

impl Profondeur {
    /// La profondeur retenue quand le conteneur n'en déclare aucune.
    ///
    /// C'est le cas des formats avec perte (MP3, AAC, Vorbis), que symphonia décode en
    /// flottant : il n'y a pas de « profondeur d'origine » à préserver, parce que les
    /// échantillons d'origine n'existent plus. On sort en 24 bits — la profondeur usuelle
    /// des DAC — plutôt qu'en 16, pour ne pas tronquer un décodage flottant sans raison.
    /// Un fichier avec perte n'est jamais bit-perfect : la question ne se pose que pour la
    /// qualité de la conversion, pas pour la fidélité à la source.
    pub const SANS_DECLARATION: Self = Self::Bits24;

    /// Le nombre de bits. Utilisé par les en-têtes WAV et par la remise à l'échelle.
    pub const fn bits(self) -> u32 {
        match self {
            Self::Bits16 => 16,
            Self::Bits24 => 24,
            Self::Bits32 => 32,
        }
    }

    /// Le nombre d'octets qu'occupe un échantillon dans un fichier PCM.
    pub const fn octets(self) -> usize {
        self.bits() as usize / 8
    }

    /// De combien de bits un échantillon natif est décalé pour occuper toute la plage i32.
    /// C'est LA constante du bit-perfect : le décalage inverse doit être exactement celui-ci.
    pub const fn decalage(self) -> u32 {
        32 - self.bits()
    }

    /// Reconnaît une profondeur annoncée par un conteneur.
    ///
    /// Refuse tout ce qui n'est pas 16, 24 ou 32 — y compris 8 bits, que le scanner de V1
    /// ne rencontrait jamais et qu'il vaut mieux voir échouer que deviner.
    pub const fn depuis_bits(bits: u32) -> Option<Self> {
        match bits {
            16 => Some(Self::Bits16),
            24 => Some(Self::Bits24),
            32 => Some(Self::Bits32),
            _ => None,
        }
    }
}

/// Le format d'un flux. Ce que `BackendAudio::preparer` reçoit, et ce qu'il **rend**
/// réellement — voir `backend.rs` : le format rendu peut différer du format demandé, et
/// c'est précisément ce qu'il faut pouvoir constater pour refuser un resampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatAudio {
    /// Fréquence d'échantillonnage en Hz (44 100, 192 000…).
    pub frequence: u32,
    /// Nombre de canaux. 2 dans la quasi-totalité des cas.
    pub canaux: u16,
    pub profondeur: Profondeur,
}

impl FormatAudio {
    pub const fn nouveau(frequence: u32, canaux: u16, profondeur: Profondeur) -> Self {
        Self {
            frequence,
            canaux,
            profondeur,
        }
    }

    /// Octets d'une trame (un échantillon par canal).
    pub const fn octets_par_trame(&self) -> usize {
        self.canaux as usize * self.profondeur.octets()
    }

    /// Octets par seconde, pour les en-têtes WAV.
    pub const fn octets_par_seconde(&self) -> u32 {
        self.frequence * self.octets_par_trame() as u32
    }
}

impl fmt::Display for FormatAudio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} bits / {} Hz / {} canaux",
            self.profondeur.bits(),
            self.frequence,
            self.canaux
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_decalage_complete_toujours_32_bits() {
        // Le bit-perfect repose entièrement sur cette égalité : l'échantillon natif, décalé
        // de `decalage()`, occupe exactement les bits hauts d'un i32, et rien n'est perdu.
        for p in [Profondeur::Bits16, Profondeur::Bits24, Profondeur::Bits32] {
            assert_eq!(p.bits() + p.decalage(), 32, "profondeur {p:?}");
        }
    }

    #[test]
    fn une_profondeur_inhabituelle_est_refusee_et_non_devinee() {
        assert_eq!(Profondeur::depuis_bits(16), Some(Profondeur::Bits16));
        assert_eq!(Profondeur::depuis_bits(24), Some(Profondeur::Bits24));
        assert_eq!(Profondeur::depuis_bits(32), Some(Profondeur::Bits32));
        // 8 bits et 20 bits existent dans la nature. On préfère une erreur à un son dégradé.
        assert_eq!(Profondeur::depuis_bits(8), None);
        assert_eq!(Profondeur::depuis_bits(20), None);
        assert_eq!(Profondeur::depuis_bits(0), None);
    }

    #[test]
    fn les_tailles_servent_les_entetes_wav() {
        let f = FormatAudio::nouveau(44_100, 2, Profondeur::Bits16);
        assert_eq!(f.octets_par_trame(), 4);
        assert_eq!(f.octets_par_seconde(), 176_400);

        let f = FormatAudio::nouveau(192_000, 2, Profondeur::Bits24);
        assert_eq!(f.octets_par_trame(), 6);
        assert_eq!(f.octets_par_seconde(), 1_152_000);
    }
}
