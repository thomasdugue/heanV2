//! Un `BackendAudio` qui écrit dans un fichier WAV au lieu de parler au HAL.
//!
//! C'est la pièce qui rend l'audio vérifiable en CI, sans DAC et sans oreille. Ce n'est
//! **pas un mock** : il consomme les mêmes trames, dans le même format interne, en suivant
//! le même contrat que le backend CoreAudio. Un mock encoderait une hypothèse sur du code
//! qu'on ne contrôle pas ; ici le fichier produit est un vrai résultat, comparable octet
//! par octet à la sortie d'un décodeur indépendant (HARNAIS.md §1, et « ce que je
//! n'ajouterais pas » : pas de mocking systématique).

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::backend::BackendAudio;
use super::chain::Chaine;
use super::error::ErreurAudio;
use super::format::FormatAudio;
use super::samples::octets_pcm;
use super::wav;

pub struct BackendWav {
    chemin: PathBuf,
    sortie: Option<BufWriter<File>>,
    format: Option<FormatAudio>,
    octets_de_donnees: u32,
    chaine: Chaine,
}

impl BackendWav {
    /// Un backend qui écrira dans `chemin`, chaîne **transparente** : c'est la
    /// configuration du bit-perfect, et c'est celle qu'on veut par défaut pour qu'un test
    /// écrit sans y penser mesure la bonne chose.
    pub fn nouveau(chemin: impl Into<PathBuf>) -> Self {
        Self {
            chemin: chemin.into(),
            sortie: None,
            format: None,
            octets_de_donnees: 0,
            chaine: Chaine::TRANSPARENTE,
        }
    }

    /// Même backend, avec une chaîne donnée (volume, et plus tard l'égaliseur).
    pub fn avec_chaine(chemin: impl Into<PathBuf>, chaine: Chaine) -> Self {
        Self {
            chaine,
            ..Self::nouveau(chemin)
        }
    }

    pub fn chemin(&self) -> &Path {
        &self.chemin
    }

    /// Nombre d'octets de PCM écrits jusqu'ici.
    pub fn octets_de_donnees(&self) -> u32 {
        self.octets_de_donnees
    }
}

impl BackendAudio for BackendWav {
    /// Un fichier sert n'importe quel format : il rend donc exactement celui demandé.
    ///
    /// C'est ce qui en fait un étalon utile. Quand le backend CoreAudio rendra un format
    /// différent de celui demandé — un DAC qui ne monte pas à 192 kHz, un périphérique
    /// AirPlay dont on ne doit surtout pas changer la fréquence (PLAT-1) — l'écart sera
    /// imputable au matériel et non à la chaîne, puisque la chaîne est déjà prouvée ici.
    fn preparer(&mut self, format: FormatAudio) -> Result<FormatAudio, ErreurAudio> {
        let fichier = File::create(&self.chemin).map_err(ErreurAudio::Ecriture)?;
        let mut sortie = BufWriter::new(fichier);
        // En-tête provisoire : les tailles ne sont connues qu'à la fin. Il est réécrit par
        // `terminer()`, et c'est pour cela que `terminer()` ne peut pas être un `Drop` :
        // sans lui le fichier reste annoncé vide, ce qui est une perte en silence.
        sortie
            .write_all(&wav::entete(&format, 0))
            .map_err(ErreurAudio::Ecriture)?;
        self.sortie = Some(sortie);
        self.format = Some(format);
        self.octets_de_donnees = 0;
        tracing::debug!(chemin = %self.chemin.display(), format = %format, "backend WAV prêt");
        Ok(format)
    }

    fn ecrire(&mut self, echantillons: &[i32]) -> Result<usize, ErreurAudio> {
        let format = self.format.ok_or_else(|| ErreurAudio::FormatRefuse {
            demande: "écriture avant préparation".to_owned(),
            obtenu: "aucun format".to_owned(),
        })?;
        let sortie = self.sortie.as_mut().ok_or_else(|| {
            ErreurAudio::Ecriture(std::io::Error::other("fichier non ouvert"))
        })?;

        // LE CHEMIN BIT-PERFECT, visible en une ligne : quand la chaîne est transparente,
        // la tranche d'entrée part telle quelle, sans copie ni modification.
        let mut tampon = Vec::new();
        let a_ecrire: &[i32] = if self.chaine.est_transparente() {
            echantillons
        } else {
            tampon.extend_from_slice(echantillons);
            self.chaine.appliquer(&mut tampon);
            &tampon
        };

        let octets = octets_pcm(a_ecrire, format.profondeur);
        sortie.write_all(&octets).map_err(ErreurAudio::Ecriture)?;
        self.octets_de_donnees += octets.len() as u32;
        Ok(a_ecrire.len())
    }

    fn terminer(&mut self) -> Result<(), ErreurAudio> {
        let (Some(mut sortie), Some(format)) = (self.sortie.take(), self.format) else {
            return Ok(()); // déjà terminé, ou jamais préparé : rien à écrire
        };
        sortie.flush().map_err(ErreurAudio::Ecriture)?;
        let mut fichier = sortie.into_inner().map_err(|e| {
            ErreurAudio::Ecriture(std::io::Error::other(e.to_string()))
        })?;
        // Réécriture de l'en-tête, maintenant que les tailles sont connues.
        fichier
            .seek(SeekFrom::Start(0))
            .map_err(ErreurAudio::Ecriture)?;
        fichier
            .write_all(&wav::entete(&format, self.octets_de_donnees))
            .map_err(ErreurAudio::Ecriture)?;
        fichier.flush().map_err(ErreurAudio::Ecriture)?;
        tracing::debug!(
            chemin = %self.chemin.display(),
            octets = self.octets_de_donnees,
            "fichier WAV terminé"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::format::Profondeur;
    use super::*;

    fn chemin(nom: &str) -> PathBuf {
        std::env::temp_dir().join(format!("hean-bw-{}-{nom}", std::process::id()))
    }

    fn stereo_16() -> FormatAudio {
        FormatAudio::nouveau(44_100, 2, Profondeur::Bits16)
    }

    #[test]
    fn un_cycle_complet_produit_un_fichier_de_la_bonne_taille() {
        let p = chemin("complet.wav");
        let mut b = BackendWav::nouveau(&p);
        assert_eq!(b.preparer(stereo_16()).expect("préparer"), stereo_16());
        assert_eq!(b.ecrire(&[0, 0, 1 << 16, -1 << 16]).expect("écrire"), 4);
        b.terminer().expect("terminer");

        let octets = std::fs::read(&p).expect("relecture");
        // 44 octets d'en-tête canonique + 4 échantillons de 2 octets.
        assert_eq!(octets.len(), wav::TAILLE_ENTETE_CANONIQUE + 8);
        assert_eq!(&octets[0..4], b"RIFF");
        // La taille annoncée dans l'en-tête a bien été RÉÉCRITE à la fin.
        assert_eq!(&octets[40..44], &8_u32.to_le_bytes());
        let _ = std::fs::remove_file(&p);
    }

    /// Le piège que `terminer()` existe pour éviter, et pourquoi ce n'est pas un `Drop`.
    ///
    /// Sans appel explicite, l'en-tête reste celui écrit à la préparation : il annonce
    /// **zéro** octet de données, alors que le fichier en contient. Un lecteur le voit donc
    /// vide. Un `Drop` ne pourrait rien signaler si la réécriture échouait — ce serait une
    /// perte de données en silence, exactement ce que ce projet refuse.
    #[test]
    fn sans_terminer_lentete_annonce_un_fichier_vide() {
        let p = chemin("sans-terminer.wav");
        {
            let mut b = BackendWav::nouveau(&p);
            b.preparer(stereo_16()).expect("préparer");
            b.ecrire(&[1 << 16; 64]).expect("écrire");
            b.terminer().expect("vidage des tampons");
        }
        // Témoin : ici terminer() A été appelé, donc l'en-tête est juste.
        let bon = std::fs::read(&p).expect("relecture");
        assert_eq!(&bon[40..44], &128_u32.to_le_bytes());

        // Et maintenant le même scénario SANS terminer() : l'en-tête reste à zéro.
        let p2 = chemin("sans-terminer-2.wav");
        let mut b = BackendWav::nouveau(&p2);
        b.preparer(stereo_16()).expect("préparer");
        b.ecrire(&[1 << 16; 64]).expect("écrire");
        assert_eq!(b.octets_de_donnees(), 128, "les octets ont bien été comptés");
        drop(b);
        let incomplet = std::fs::read(&p2).expect("relecture");
        assert_eq!(
            &incomplet[40..44],
            &0_u32.to_le_bytes(),
            "sans terminer(), l'en-tête annonce un fichier vide"
        );
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(&p2);
    }

    #[test]
    fn ecrire_avant_de_preparer_est_une_erreur_et_non_un_panic() {
        let mut b = BackendWav::nouveau(chemin("jamais-prepare.wav"));
        assert!(b.ecrire(&[0, 0]).is_err());
        // Et terminer sans avoir préparé ne doit pas échouer : il n'y a rien à écrire.
        assert!(b.terminer().is_ok());
    }

    #[test]
    fn terminer_deux_fois_ne_corrompt_pas_le_fichier() {
        let p = chemin("deux-fois.wav");
        let mut b = BackendWav::nouveau(&p);
        b.preparer(stereo_16()).expect("préparer");
        b.ecrire(&[0, 0]).expect("écrire");
        b.terminer().expect("premier terminer");
        b.terminer().expect("second terminer : sans effet");
        let octets = std::fs::read(&p).expect("relecture");
        assert_eq!(octets.len(), wav::TAILLE_ENTETE_CANONIQUE + 4);
        let _ = std::fs::remove_file(&p);
    }
}
