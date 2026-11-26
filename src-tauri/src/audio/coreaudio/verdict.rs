//! [`Verdict`] — l'état du bit-perfect après une négociation, et ce qu'il faut en dire.
//!
//! Un type à part, et pas un booléen, parce qu'il y a **trois** situations et non deux. Les
//! deux premières laissent jouer, la troisième non, et c'est la distinction que V1 n'avait
//! pas : elle ne connaissait que « ça joue », quelle que soit la fréquence réellement servie
//! (`docs/MESURES-V1.md` §11).
//!
//! | verdict | ça joue ? | bit-perfect ? |
//! |---|---|---|
//! | [`Verdict::Exact`] | oui | oui, si le volume est à 100 % et le mode exclusif actif |
//! | [`Verdict::ReechantillonneParMacos`] | oui | **non**, et l'app le dit (AirPlay, PLAT-1) |
//! | [`Verdict::FrequenceNonServie`] | **non** | sans objet — rien n'a été touché (ADR 0018) |
//!
//! Séparé de [`super::backend`] pour une raison plus utile que la taille : c'est la seule
//! pièce de la décision d'ADR 0018 que la CI peut éprouver sans matériel, et ses tests vivent
//! donc à côté d'elle.

use crate::audio::error::ErreurAudio;

/// L'état du bit-perfect après une négociation, et ce qu'il faut en dire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Le périphérique tourne exactement à la fréquence du fichier. Rien n'est
    /// rééchantillonné, rien n'est converti si le volume est à 100 %.
    Exact,
    /// AirPlay : macOS rééchantillonne de son côté, et on ne peut pas l'en empêcher sans
    /// tuer la session (PLAT-1). La lecture a lieu ; elle n'est **pas** bit-perfect, et
    /// l'interface doit le dire.
    ReechantillonneParMacos { peripherique: String, hz: u32 },
    /// Le périphérique ne sait pas servir cette fréquence. **Rien n'a été touché et rien ne
    /// joue** : l'appelant doit refuser et afficher le message.
    FrequenceNonServie {
        peripherique: String,
        demandee: u32,
        proposees: String,
    },
}

impl Verdict {
    /// L'erreur à remonter, quand le verdict en est une.
    pub fn en_erreur(&self) -> Option<ErreurAudio> {
        match self {
            Self::FrequenceNonServie {
                peripherique,
                demandee,
                proposees,
            } => Some(ErreurAudio::FrequenceNonServie {
                peripherique: peripherique.clone(),
                demandee: *demandee,
                proposees: proposees.clone(),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le verdict n'est pas décoratif : c'est lui qui transforme un écart de fréquence en
    /// **phrase lisible** plutôt qu'en rééchantillonnage muet. Ce test est la seule partie de
    /// la décision d'ADR 0018 que la CI peut éprouver sans DAC, et il porte sur ce qui compte
    /// vraiment : que le message dise au lecteur quoi faire.
    #[test]
    fn un_refus_de_frequence_nomme_le_peripherique_la_frequence_et_les_alternatives() {
        let verdict = Verdict::FrequenceNonServie {
            peripherique: "Haut-parleurs Mac mini".to_owned(),
            demandee: 192_000,
            proposees: "44100, 48000, 88200, 96000 Hz".to_owned(),
        };
        let erreur = verdict.en_erreur().expect("un refus doit rendre une erreur");
        let texte = erreur.to_string();
        for attendu in [
            "Haut-parleurs Mac mini",
            "192000",
            "96000",
            "ne rééchantillonne pas",
        ] {
            assert!(
                texte.contains(attendu),
                "le message de refus ne contient pas « {attendu} » : {texte}"
            );
        }
    }

    /// Les deux verdicts qui laissent jouer ne doivent **pas** produire d'erreur — sans quoi
    /// AirPlay deviendrait injouable, ce qui serait perdre une fonction qui marche en V1.
    #[test]
    fn les_verdicts_qui_laissent_jouer_ne_sont_pas_des_erreurs() {
        assert!(Verdict::Exact.en_erreur().is_none());
        assert!(Verdict::ReechantillonneParMacos {
            peripherique: "Salon".to_owned(),
            hz: 44_100,
        }
        .en_erreur()
        .is_none());
    }
}
