//! La chaîne de traitement, et le chemin qui ne traite rien.
//!
//! # Le bypass n'est pas une optimisation
//!
//! En V1, l'EQ était « bypassé si tous les gains ≈ 0 dB » — présenté comme un gain de
//! performance. C'est en réalité **le** chemin bit-perfect, et c'est la seule chose qui
//! fasse de « bit-perfect » autre chose qu'un espoir : quand la chaîne est transparente,
//! les échantillons ne sont ni copiés, ni convertis, ni multipliés. Il n'y a aucune
//! occasion d'arrondi parce qu'il n'y a aucune opération.
//!
//! D'où le choix des **comparaisons exactes** ci-dessous. Une tolérance (« volume à
//! 99,9999 % ≈ 100 % ») ferait prendre le chemin transparent pour un réglage que
//! l'utilisateur a réellement modifié : l'app annoncerait bit-perfect en ignorant le
//! curseur. Exact veut dire exact, et c'est aussi ce qui s'explique le plus simplement :
//! bit-perfect si, et seulement si, le volume est à 100 %.

/// Le volume de lecture, de 0 (silence) à 1 (100 %).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volume(f32);

impl Volume {
    /// 100 % : le seul réglage compatible avec le bit-perfect.
    pub const UNITE: Self = Self(1.0);

    /// Borne la valeur dans \[0, 1\]. Un volume négatif inverserait la phase et un volume
    /// supérieur à 1 écrêterait : ni l'un ni l'autre n'est ce que demande un curseur.
    pub fn nouveau(valeur: f32) -> Self {
        Self(valeur.clamp(0.0, 1.0))
    }

    pub fn valeur(self) -> f32 {
        self.0
    }

    /// Exactement 100 %. Voir l'en-tête du module pour le refus de toute tolérance.
    pub fn est_unite(self) -> bool {
        self.0 == 1.0
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self::UNITE
    }
}

/// La chaîne appliquée aux échantillons avant la sortie.
///
/// Elle ne porte que le volume pour l'instant. L'égaliseur s'y ajoutera avec le moteur :
/// ses fréquences de bandes sont une donnée de parité avec V1, et les inventer créerait
/// exactement l'écart silencieux que la reconstruction cherche à éviter. Quand il arrivera,
/// `est_transparente` devra exiger **aussi** qu'il soit plat.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Chaine {
    pub volume: Volume,
}

impl Chaine {
    /// La chaîne qui ne fait rien : le chemin bit-perfect.
    pub const TRANSPARENTE: Self = Self {
        volume: Volume::UNITE,
    };

    pub fn avec_volume(volume: Volume) -> Self {
        Self { volume }
    }

    /// Vrai si la chaîne ne modifie aucun échantillon. Un appelant qui lit `true` a le
    /// droit de transmettre la tranche d'entrée **sans la copier**.
    pub fn est_transparente(&self) -> bool {
        self.volume.est_unite()
    }

    /// Applique la chaîne en place.
    ///
    /// Sort immédiatement si la chaîne est transparente : la multiplication par 1,0 est
    /// certes neutre en virgule flottante, mais s'en remettre à cette propriété pour
    /// garantir le bit-perfect serait reposer une promesse sur un raisonnement numérique
    /// plutôt que sur une absence d'opération.
    pub fn appliquer(&self, echantillons: &mut [i32]) {
        if self.est_transparente() {
            return;
        }
        let v = f64::from(self.volume.valeur());
        for e in echantillons.iter_mut() {
            // `f64` : un `i32` à pleine échelle ne tient pas exactement dans un `f32`, et
            // l'atténuation introduirait une erreur bien plus grande que l'arrondi final.
            *e = (f64::from(*e) * v).round() as i32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_chaine_par_defaut_est_transparente() {
        assert!(Chaine::default().est_transparente());
        assert!(Chaine::TRANSPARENTE.est_transparente());
    }

    #[test]
    fn un_volume_presque_a_cent_pour_cent_nest_pas_transparent() {
        // Le point de la règle : pas de tolérance. Le curseur a bougé, donc on traite.
        assert!(!Chaine::avec_volume(Volume::nouveau(0.999_999)).est_transparente());
        assert!(!Chaine::avec_volume(Volume::nouveau(0.0)).est_transparente());
        assert!(Chaine::avec_volume(Volume::nouveau(1.0)).est_transparente());
    }

    #[test]
    fn une_chaine_transparente_ne_modifie_aucun_echantillon() {
        let depart = vec![i32::MIN, -1, 0, 1, 1234, i32::MAX];
        let mut e = depart.clone();
        Chaine::TRANSPARENTE.appliquer(&mut e);
        assert_eq!(e, depart, "le chemin bit-perfect ne touche à rien");
    }

    #[test]
    fn le_volume_a_zero_fait_silence() {
        let mut e = vec![i32::MIN, -1000, 1000, i32::MAX];
        Chaine::avec_volume(Volume::nouveau(0.0)).appliquer(&mut e);
        assert_eq!(e, vec![0, 0, 0, 0]);
    }

    #[test]
    fn le_volume_a_moitie_divise_par_deux() {
        let mut e = vec![1000, -1000, 0];
        Chaine::avec_volume(Volume::nouveau(0.5)).appliquer(&mut e);
        assert_eq!(e, vec![500, -500, 0]);
    }

    #[test]
    fn un_volume_hors_plage_est_borne() {
        // Négatif = phase inversée, au-dessus de 1 = écrêtage. Ni l'un ni l'autre.
        assert_eq!(Volume::nouveau(-3.0).valeur(), 0.0);
        assert_eq!(Volume::nouveau(7.0).valeur(), 1.0);
        assert!(Volume::nouveau(7.0).est_unite());
    }

    #[test]
    fn lattenuation_ne_deborde_pas_a_pleine_echelle() {
        // `i32::MIN` × 1,0 en `f32` arrondirait au-dessus de `i32::MAX` : d'où le `f64`.
        let mut e = vec![i32::MIN, i32::MAX];
        Chaine::avec_volume(Volume::nouveau(0.5)).appliquer(&mut e);
        assert_eq!(e, vec![i32::MIN / 2, (i32::MAX as f64 * 0.5).round() as i32]);
    }
}
