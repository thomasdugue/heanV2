//! Écriture d'un en-tête WAV.
//!
//! # Pourquoi deux formes d'en-tête
//!
//! Au-delà de 16 bits, un WAV ne s'écrit pas en PCM « canonique » (`wFormatTag = 1`, bloc
//! `fmt ` de 16 octets) mais en `WAVE_FORMAT_EXTENSIBLE` (`0xFFFE`, bloc de 40 octets, avec
//! le nombre de bits réellement valides, un masque de canaux et un GUID de sous-format).
//! C'est ce que recommande la spécification Microsoft, et c'est ce que produit ffmpeg.
//!
//! Ce détail n'est pas cosmétique : c'est lui qui rend le test bit-perfect comparable
//! **octet par octet** à un fichier écrit par ffmpeg. Si on écrivait un en-tête canonique
//! en 24 bits, il faudrait comparer les flux PCM en sautant les en-têtes — donc écrire soi-
//! même la logique de lecture dont dépend le test, et ne plus rien prouver d'indépendant.

use super::format::FormatAudio;

/// `KSDATAFORMAT_SUBTYPE_PCM` : `00000001-0000-0010-8000-00AA00389B71`, sérialisé comme le
/// veut la spécification (les trois premiers champs en little-endian, le dernier tel quel).
const GUID_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

/// Taille d'un en-tête canonique : 12 (RIFF/WAVE) + 24 (`fmt `) + 8 (`data`).
pub const TAILLE_ENTETE_CANONIQUE: usize = 44;
/// Taille d'un en-tête extensible : le bloc `fmt ` porte 24 octets de plus.
pub const TAILLE_ENTETE_EXTENSIBLE: usize = 68;

/// Le masque de canaux de `WAVE_FORMAT_EXTENSIBLE`.
///
/// Les valeurs sont celles qu'écrit ffmpeg, relevées et non devinées : mono = `FRONT_CENTER`,
/// stéréo = `FRONT_LEFT | FRONT_RIGHT`. Au-delà, l'ordre des canaux de l'app n'est pas
/// établi, et un masque faux désignerait des enceintes qui ne sont pas les bonnes : on écrit
/// alors 0, qui veut dire « ordre non spécifié » plutôt qu'un ordre inventé.
const fn masque_de_canaux(canaux: u16) -> u32 {
    match canaux {
        1 => 0x4,
        2 => 0x3,
        _ => 0,
    }
}

/// L'en-tête d'un fichier WAV contenant `octets_de_donnees` octets de PCM.
///
/// Le choix de la forme est fonction de la seule profondeur : jusqu'à 16 bits inclus la
/// forme canonique, au-delà la forme extensible.
pub fn entete(format: &FormatAudio, octets_de_donnees: u32) -> Vec<u8> {
    let extensible = format.profondeur.bits() > 16;
    let taille_fmt: u32 = if extensible { 40 } else { 16 };
    let taille_entete = if extensible {
        TAILLE_ENTETE_EXTENSIBLE
    } else {
        TAILLE_ENTETE_CANONIQUE
    };

    let mut o = Vec::with_capacity(taille_entete);
    o.extend_from_slice(b"RIFF");
    // Tout ce qui suit ce champ : la taille du fichier moins les 8 octets « RIFF » + taille.
    o.extend_from_slice(&(octets_de_donnees + taille_entete as u32 - 8).to_le_bytes());
    o.extend_from_slice(b"WAVE");

    o.extend_from_slice(b"fmt ");
    o.extend_from_slice(&taille_fmt.to_le_bytes());
    o.extend_from_slice(&if extensible { 0xFFFE_u16 } else { 1_u16 }.to_le_bytes());
    o.extend_from_slice(&format.canaux.to_le_bytes());
    o.extend_from_slice(&format.frequence.to_le_bytes());
    o.extend_from_slice(&format.octets_par_seconde().to_le_bytes());
    o.extend_from_slice(&(format.octets_par_trame() as u16).to_le_bytes());
    o.extend_from_slice(&(format.profondeur.bits() as u16).to_le_bytes());
    if extensible {
        o.extend_from_slice(&22_u16.to_le_bytes()); // cbSize : les 22 octets qui suivent
        o.extend_from_slice(&(format.profondeur.bits() as u16).to_le_bytes()); // bits valides
        o.extend_from_slice(&masque_de_canaux(format.canaux).to_le_bytes());
        o.extend_from_slice(&GUID_PCM);
    }

    o.extend_from_slice(b"data");
    o.extend_from_slice(&octets_de_donnees.to_le_bytes());
    debug_assert_eq!(o.len(), taille_entete);
    o
}

#[cfg(test)]
mod tests {
    use super::super::format::Profondeur;
    use super::*;

    /// Comparaison aux octets relevés dans `fixtures/audio/golden/flac-16-44.wav`, écrit par
    /// ffmpeg. Un en-tête « presque bon » ferait échouer le test bit-perfect sur un détail
    /// d'en-tête au lieu du son : autant le prouver ici, en isolé.
    #[test]
    fn lentete_16_bits_est_celui_decrit_par_ffmpeg() {
        let f = FormatAudio::nouveau(44_100, 2, Profondeur::Bits16);
        let e = entete(&f, 176_400);
        assert_eq!(e.len(), TAILLE_ENTETE_CANONIQUE);
        assert_eq!(
            e,
            vec![
                b'R', b'I', b'F', b'F', 0x34, 0xb1, 0x02, 0x00, // RIFF, 176 436
                b'W', b'A', b'V', b'E', b'f', b'm', b't', b' ', //
                0x10, 0x00, 0x00, 0x00, // taille du bloc fmt : 16
                0x01, 0x00, // PCM canonique
                0x02, 0x00, // 2 canaux
                0x44, 0xac, 0x00, 0x00, // 44 100 Hz
                0x10, 0xb1, 0x02, 0x00, // 176 400 octets/s
                0x04, 0x00, // 4 octets par trame
                0x10, 0x00, // 16 bits
                b'd', b'a', b't', b'a', 0x10, 0xb1, 0x02, 0x00,
            ]
        );
    }

    /// Idem pour `golden/flac-24-192.wav` : forme extensible, 68 octets, GUID PCM.
    #[test]
    fn lentete_24_bits_est_extensible_comme_celui_decrit_par_ffmpeg() {
        let f = FormatAudio::nouveau(192_000, 2, Profondeur::Bits24);
        let e = entete(&f, 576_000);
        assert_eq!(e.len(), TAILLE_ENTETE_EXTENSIBLE);
        assert_eq!(&e[0..4], b"RIFF");
        assert_eq!(&e[4..8], &(576_000_u32 + 60).to_le_bytes());
        assert_eq!(&e[12..16], b"fmt ");
        assert_eq!(&e[16..20], &40_u32.to_le_bytes());
        assert_eq!(&e[20..22], &0xFFFE_u16.to_le_bytes(), "WAVE_FORMAT_EXTENSIBLE");
        assert_eq!(&e[22..24], &2_u16.to_le_bytes());
        assert_eq!(&e[24..28], &192_000_u32.to_le_bytes());
        assert_eq!(&e[28..32], &1_152_000_u32.to_le_bytes());
        assert_eq!(&e[32..34], &6_u16.to_le_bytes(), "6 octets par trame");
        assert_eq!(&e[34..36], &24_u16.to_le_bytes());
        assert_eq!(&e[36..38], &22_u16.to_le_bytes(), "cbSize");
        assert_eq!(&e[38..40], &24_u16.to_le_bytes(), "bits valides");
        assert_eq!(&e[40..44], &3_u32.to_le_bytes(), "FRONT_LEFT | FRONT_RIGHT");
        assert_eq!(&e[44..60], &GUID_PCM);
        assert_eq!(&e[60..64], b"data");
        assert_eq!(&e[64..68], &576_000_u32.to_le_bytes());
    }

    #[test]
    fn le_masque_de_canaux_nest_jamais_invente() {
        assert_eq!(masque_de_canaux(1), 0x4);
        assert_eq!(masque_de_canaux(2), 0x3);
        // Au-delà, « non spécifié » plutôt qu'un ordre d'enceintes faux.
        assert_eq!(masque_de_canaux(6), 0);
    }

    #[test]
    fn la_frontiere_des_formes_est_a_16_bits() {
        let c = FormatAudio::nouveau(44_100, 2, Profondeur::Bits16);
        let e1 = FormatAudio::nouveau(44_100, 2, Profondeur::Bits24);
        let e2 = FormatAudio::nouveau(44_100, 2, Profondeur::Bits32);
        assert_eq!(entete(&c, 0).len(), TAILLE_ENTETE_CANONIQUE);
        assert_eq!(entete(&e1, 0).len(), TAILLE_ENTETE_EXTENSIBLE);
        assert_eq!(entete(&e2, 0).len(), TAILLE_ENTETE_EXTENSIBLE);
    }
}
