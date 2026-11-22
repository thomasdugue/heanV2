//! Les échantillons décodés, et la seule conversion dont dépend le bit-perfect.
//!
//! # Pourquoi `i32` et non `f32`
//!
//! Le pipeline de V1 décodait en `f32` entrelacé. `f32` représente exactement tout entier
//! jusqu'à 2^24, donc 16 et 24 bits y survivent intacts — mais il faut **le démontrer**, en
//! raisonnant sur la mantisse, et la démonstration tombe au premier fichier 32 bits entier.
//!
//! Ici le format interne est l'échantillon natif **décalé à gauche** pour occuper toute la
//! plage de `i32`. Le retour est le décalage arithmétique inverse : exact par construction,
//! sans arrondi, et vérifiable d'un coup d'œil. « Bit-perfect » cesse d'être une propriété
//! numérique à prouver pour devenir une propriété structurelle — quand l'EQ est désactivé
//! et le volume à 100 %, aucune conversion n'a lieu du tout (voir `chain.rs`).
//!
//! C'est aussi la convention de symphonia pour ses tampons `S32`, ce qui évite une
//! conversion à l'entrée du décodeur. Cette convention est **vérifiée** par un test et non
//! supposée : voir `decode.rs`.

use super::format::{FormatAudio, Profondeur};

/// Un bloc de trames décodées, **entrelacées** (L, R, L, R…), en `i32` mis à l'échelle sur
/// 32 bits. L'entrelacement est le format attendu par CoreAudio comme par un fichier WAV :
/// désentrelacer pour réentrelacer ensuite ne servirait personne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trames {
    pub format: FormatAudio,
    pub echantillons: Vec<i32>,
}

/// Porte un échantillon natif de `profondeur` bits dans la plage complète de `i32`.
///
/// `natif` doit tenir dans la profondeur annoncée ; les bits hauts en excès seraient
/// silencieusement perdus, ce qui est exactement le défaut qu'on refuse ailleurs. C'est au
/// décodeur de ne passer que des valeurs valides, et `ramener_au_natif` en est l'inverse
/// exact — propriété testée ci-dessous.
#[inline]
pub const fn mettre_a_echelle(natif: i32, profondeur: Profondeur) -> i32 {
    natif << profondeur.decalage()
}

/// L'inverse exact de `mettre_a_echelle`. Décalage **arithmétique** : il propage le bit de
/// signe, donc les valeurs négatives reviennent intactes.
#[inline]
pub const fn ramener_au_natif(echelle: i32, profondeur: Profondeur) -> i32 {
    echelle >> profondeur.decalage()
}

/// Convertit un échantillon flottant (plage \[-1, 1\]) vers l'échelle 32 bits.
///
/// Réservé aux formats avec perte, que symphonia décode en flottant. Le calcul passe par
/// `f64` : en `f32`, `1.0 * 2^31` n'est pas représentable exactement et l'arrondi ferait
/// déborder. La valeur est bornée, car un décodeur avec perte peut dépasser 1,0 — un
/// dépassement non borné reviendrait en boucle par l'autre bout de la plage, c'est-à-dire
/// en craquement à pleine échelle.
#[inline]
pub fn depuis_flottant(v: f32) -> i32 {
    let mis_a_echelle = f64::from(v) * 2_147_483_648.0_f64;
    if mis_a_echelle >= f64::from(i32::MAX) {
        i32::MAX
    } else if mis_a_echelle <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        mis_a_echelle as i32
    }
}

impl Trames {
    pub fn nouveau(format: FormatAudio, echantillons: Vec<i32>) -> Self {
        Self {
            format,
            echantillons,
        }
    }

    /// Nombre de trames (un échantillon par canal). Zéro canal est impossible : le format
    /// vient d'un conteneur qui en déclare toujours au moins un.
    pub fn nombre_de_trames(&self) -> usize {
        let canaux = self.format.canaux as usize;
        if canaux == 0 {
            0
        } else {
            self.echantillons.len() / canaux
        }
    }

    /// Les octets PCM **little-endian**, dans la profondeur du format.
    ///
    /// C'est ici que la garantie devient observable : chaque échantillon est ramené au
    /// natif par décalage, puis ses octets de poids faible sont écrits tels quels.
    pub fn vers_octets_pcm(&self) -> Vec<u8> {
        octets_pcm(&self.echantillons, self.format.profondeur)
    }
}

/// Les octets PCM little-endian d'un bloc d'échantillons.
///
/// Fonction libre et non méthode : le backend écrit des tranches qu'il ne possède pas, et
/// construire un `Trames` juste pour les convertir imposerait une copie sur le chemin du
/// bit-perfect — celui qui doit justement ne rien recopier ni rien modifier.
pub fn octets_pcm(echantillons: &[i32], profondeur: Profondeur) -> Vec<u8> {
    let octets = profondeur.octets();
    let mut sortie = Vec::with_capacity(echantillons.len() * octets);
    for &e in echantillons {
        let natif = ramener_au_natif(e, profondeur);
        // Complément à deux : les octets de poids faible du `i32` SONT ceux du `i24` ou du
        // `i16`, y compris pour une valeur négative. Rien à corriger.
        sortie.extend_from_slice(&natif.to_le_bytes()[..octets]);
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La propriété centrale, vérifiée de façon **exhaustive** en 16 bits : les 65 536
    /// valeurs possibles font l'aller-retour sans perte. Pas un échantillonnage — toutes.
    #[test]
    fn laller_retour_est_exact_sur_toutes_les_valeurs_16_bits() {
        for natif in i16::MIN..=i16::MAX {
            let n = i32::from(natif);
            let e = mettre_a_echelle(n, Profondeur::Bits16);
            assert_eq!(
                ramener_au_natif(e, Profondeur::Bits16),
                n,
                "aller-retour perdu pour {n}"
            );
        }
    }

    /// 24 bits : 16,7 millions de valeurs, trop pour un test de routine. On couvre les
    /// bornes (là où le signe et le débordement se jouent) et un pas premier, qui balaie
    /// tous les motifs de bits de poids faible sans se synchroniser sur une puissance de 2.
    #[test]
    fn laller_retour_est_exact_sur_les_bornes_et_un_pas_premier_en_24_bits() {
        const MIN: i32 = -(1 << 23);
        const MAX: i32 = (1 << 23) - 1;
        let mut cas = vec![MIN, MIN + 1, -1, 0, 1, MAX - 1, MAX];
        let mut v = MIN;
        while v < MAX {
            cas.push(v);
            v += 7919;
        }
        for n in cas {
            let e = mettre_a_echelle(n, Profondeur::Bits24);
            assert_eq!(ramener_au_natif(e, Profondeur::Bits24), n, "24 bits : {n}");
        }
    }

    #[test]
    fn en_32_bits_la_conversion_est_lidentite() {
        // Décalage nul : le chemin bit-perfect ne touche littéralement à rien.
        for n in [i32::MIN, -1, 0, 1, 42, i32::MAX] {
            assert_eq!(mettre_a_echelle(n, Profondeur::Bits32), n);
            assert_eq!(ramener_au_natif(n, Profondeur::Bits32), n);
        }
    }

    /// Les octets produits sont comparés à ceux relevés dans le golden `flac-24-192.wav`,
    /// qu'ffmpeg a écrits — donc à un encodeur indépendant du nôtre.
    #[test]
    fn les_octets_pcm_correspondent_a_ceux_dun_encodeur_independant() {
        // Golden 24 bits, trois premiers échantillons : 00 00 00 | 8d c4 fe | 19 5d 00,
        // soit les natifs 0, -80 755 et 23 833.
        let f = FormatAudio::nouveau(192_000, 2, Profondeur::Bits24);
        let t = Trames::nouveau(
            f,
            vec![
                mettre_a_echelle(0, Profondeur::Bits24),
                mettre_a_echelle(-80_755, Profondeur::Bits24),
                mettre_a_echelle(23_833, Profondeur::Bits24),
            ],
        );
        assert_eq!(
            t.vers_octets_pcm(),
            vec![0x00, 0x00, 0x00, 0x8d, 0xc4, 0xfe, 0x19, 0x5d, 0x00]
        );

        // Golden 16 bits, deux premiers échantillons : 00 00 | c4 fe, natifs 0 et -316.
        let f = FormatAudio::nouveau(44_100, 2, Profondeur::Bits16);
        let t = Trames::nouveau(
            f,
            vec![
                mettre_a_echelle(0, Profondeur::Bits16),
                mettre_a_echelle(-316, Profondeur::Bits16),
            ],
        );
        assert_eq!(t.vers_octets_pcm(), vec![0x00, 0x00, 0xc4, 0xfe]);
    }

    #[test]
    fn un_flottant_hors_plage_est_borne_et_ne_reboucle_pas() {
        // Sans bornage, 2,0 deviendrait un grand négatif : un craquement à pleine échelle.
        assert_eq!(depuis_flottant(2.0), i32::MAX);
        assert_eq!(depuis_flottant(-2.0), i32::MIN);
        assert_eq!(depuis_flottant(1.0), i32::MAX);
        assert_eq!(depuis_flottant(0.0), 0);
        assert!(depuis_flottant(0.5) > 0);
    }

    #[test]
    fn le_nombre_de_trames_tient_compte_des_canaux() {
        let f = FormatAudio::nouveau(44_100, 2, Profondeur::Bits16);
        assert_eq!(Trames::nouveau(f, vec![1, 2, 3, 4]).nombre_de_trames(), 2);
        let mono = FormatAudio::nouveau(44_100, 1, Profondeur::Bits16);
        assert_eq!(Trames::nouveau(mono, vec![1, 2, 3]).nombre_de_trames(), 3);
    }
}
