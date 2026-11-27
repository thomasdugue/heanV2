//! Lire un fichier distant **en direct**, à n'importe quelle position (ADR 0022).
//!
//! Ce module ne connaît ni symphonia ni l'audio : il rend un `Read + Seek` et sa taille,
//! c'est tout. Le moteur audio en fait un `MediaSource` de son côté (`audio::source`). C'est ce
//! qui garde la règle de zone tenable — le réseau n'importe rien de l'audio, l'audio n'importe
//! rien de SMB, et le câblage des deux vit là où il a déjà le droit de connaître les deux.
//!
//! ## Le cache de blocs n'est pas une optimisation
//!
//! `pavao::SmbFile` **emprunte** le client, qui vit sous le verrou de process de PLAT-2 : un
//! handle ne survit donc pas à la sortie du verrou, et chaque lecture est un cycle complet
//! « ouvrir → se positionner → lire → fermer ». Or un décodeur demande de petites tranches.
//! Une ouverture SMB par petite tranche serait inutilisable : c'est pour ça qu'on lit par
//! blocs larges et qu'on sert la mémoire. Deux tests comptent les ouvertures, parce qu'une
//! phrase ne peut pas échouer.

use std::io::{self, Read, Seek, SeekFrom};

use super::client::Smb;

/// La taille d'un bloc lu d'un coup : **1 Mio**.
///
/// La même que l'en-tête de tags, et pour la raison déjà mesurée : 1 Mio coûte **~29 ms** de
/// transfert sur le NAS de Thomas (ADR 0016 §3, mesuré). C'est donc un chiffre repris d'une
/// mesure et non choisi — et la profondeur juste reste à mesurer sur la lecture elle-même,
/// ce que l'ADR 0022 laisse explicitement ouvert.
pub const TAILLE_BLOC: usize = 1024 * 1024;

/// Un fichier d'un partage SMB, lu à la demande et jamais copié sur le disque.
pub struct FichierDistant<S: Smb> {
    partage: S,
    chemin: String,
    /// La taille réelle, donnée par le partage à la première tranche. Constante ensuite :
    /// c'est elle qui rend le déplacement juste, là où une longueur qui grandit ne le rend pas.
    taille: u64,
    position: u64,
    bloc: Vec<u8>,
    debut_du_bloc: u64,
}

impl<S: Smb> FichierDistant<S> {
    /// Ouvre le fichier et lit son premier bloc.
    ///
    /// La première tranche est lue **tout de suite**, et pas paresseusement : elle rend la
    /// taille, dont `byte_len` a besoin avant la moindre lecture, et elle réchauffe le cache
    /// pour le sondage de format qui suit immédiatement. Repousser cette lecture obligerait à
    /// annoncer une taille inconnue, ce qui ferait traiter le flux comme non déplaçable.
    pub fn ouvrir(partage: S, chemin: &str) -> Result<Self, super::client::Erreur> {
        let premier = partage.tranche(chemin, 0, TAILLE_BLOC)?;
        Ok(Self {
            partage,
            chemin: chemin.to_string(),
            taille: premier.taille,
            position: 0,
            bloc: premier.octets,
            debut_du_bloc: 0,
        })
    }

    /// La taille du fichier, telle que le partage l'a donnée.
    pub fn taille(&self) -> u64 {
        self.taille
    }

    fn dans_le_bloc(&self, position: u64) -> bool {
        position >= self.debut_du_bloc && position < self.debut_du_bloc + self.bloc.len() as u64
    }

    /// Remplace le bloc en mémoire par celui qui contient `position`.
    ///
    /// Le bloc commence À `position` et non à un multiple de `TAILLE_BLOC` : après un
    /// déplacement, ce qu'on va lire est ce qui suit le point visé, et aligner ferait relire
    /// jusqu'à un mégaoctet déjà derrière nous.
    fn charger(&mut self, position: u64) -> io::Result<()> {
        let tranche = self
            .partage
            .tranche(&self.chemin, position, TAILLE_BLOC)
            // L'erreur typée devient un `io::Error`, parce que `Read` n'en porte pas d'autre.
            // Son texte est conservé : c'est lui qui nomme l'hôte et le chemin, et c'est ce
            // texte que l'écran affichera (MESURES-V1 §8 — un refus de lecture doit se lire).
            .map_err(|e| io::Error::other(e.to_string()))?;
        self.bloc = tranche.octets;
        self.debut_du_bloc = position;
        // LA TAILLE N'EST PAS RELUE, et c'est un choix qui se corrige mal autrement : elle a
        // été annoncée à symphonia à l'ouverture (`SourceMesuree`), et une longueur qui change
        // après son annonce fait viser à côté — le décodeur calcule sa cible sur l'ancienne, on
        // l'écrase en silence. C'est exactement le défaut que l'ADR 0022 reproche au progressif
        // de V1, et il avait réussi à se glisser ici par la petite porte du « mieux vaut la
        // vérité courante ». Un fichier qui change sous nous se comporte donc comme un fichier
        // local tronqué pendant la lecture : la piste s'arrête plus court, et le dit.
        Ok(())
    }
}

impl<S: Smb> Read for FichierDistant<S> {
    fn read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        if destination.is_empty() || self.position >= self.taille {
            return Ok(0);
        }
        if !self.dans_le_bloc(self.position) {
            self.charger(self.position)?;
        }
        // Un bloc vide alors qu'on est avant la fin annoncée : le fichier a rétréci sous nous.
        // On le dit par une fin de flux, pas par une boucle qui redemanderait le même bloc sans
        // fin — mais **on le journalise**, parce qu'une piste qui s'arrête plus court sans
        // laisser de trace est de la musique perdue à l'insu de tous (famille d'INV-22), et
        // qu'un défaut qui ne se signale pas coûte plus que lui-même (INV-21).
        if self.bloc.is_empty() {
            tracing::warn!(
                chemin = %self.chemin,
                position = self.position,
                taille_annoncee = self.taille,
                "le partage rend moins d'octets qu'annoncé : la piste s'arrête plus court"
            );
            return Ok(0);
        }
        let decalage = (self.position - self.debut_du_bloc) as usize;
        let combien = destination.len().min(self.bloc.len() - decalage);
        destination[..combien].copy_from_slice(&self.bloc[decalage..decalage + combien]);
        self.position += combien as u64;
        Ok(combien)
    }
}

impl<S: Smb> Seek for FichierDistant<S> {
    /// Se déplacer **ne touche pas le réseau** : seule la prochaine lecture le fera, et
    /// seulement si elle sort du bloc en mémoire. Le sondage de format de symphonia saute
    /// beaucoup, et la plupart de ses sauts retombent dans le premier mégaoctet.
    fn seek(&mut self, de: SeekFrom) -> io::Result<u64> {
        let cible = match de {
            SeekFrom::Start(p) => i128::from(p),
            SeekFrom::End(d) => i128::from(self.taille) + i128::from(d),
            SeekFrom::Current(d) => i128::from(self.position) + i128::from(d),
        };
        if cible < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "déplacement avant le début du fichier",
            ));
        }
        // Borné à la taille : au-delà, la lecture rendra zéro octet, ce qu'un décodeur lit
        // comme une fin de flux. C'est le comportement d'un fichier local.
        self.position = u64::try_from(cible).unwrap_or(u64::MAX).min(self.taille);
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::network::client::{Entree, Erreur, Tranche};

    /// Un partage en mémoire **qui compte ses ouvertures**. C'est la couture du trait `Smb`
    /// employée pour ce qu'elle vaut : le NAS de Thomas n'est pas toujours allumé et la CI n'y
    /// a pas accès, donc tout ce qui suit s'éprouve sans réseau.
    struct FauxPartage {
        contenu: Vec<u8>,
        ouvertures: Cell<usize>,
        echoue_apres: Option<usize>,
    }

    impl FauxPartage {
        fn de(contenu: Vec<u8>) -> Self {
            Self { contenu, ouvertures: Cell::new(0), echoue_apres: None }
        }
    }

    impl Smb for FauxPartage {
        fn lister(&self, _chemin: &str) -> Result<Vec<Entree>, Erreur> {
            Ok(Vec::new())
        }

        fn lire(&self, _chemin: &str, max: Option<usize>) -> Result<Vec<u8>, Erreur> {
            let fin = max.unwrap_or(self.contenu.len()).min(self.contenu.len());
            Ok(self.contenu[..fin].to_vec())
        }

        fn tranche(&self, chemin: &str, debut: u64, max: usize) -> Result<Tranche, Erreur> {
            self.ouvertures.set(self.ouvertures.get() + 1);
            if self.echoue_apres.is_some_and(|n| self.ouvertures.get() > n) {
                return Err(Erreur::Fichier {
                    hote: "nas.local".to_string(),
                    chemin: chemin.to_string(),
                    detail: "le partage a fermé la session".to_string(),
                });
            }
            let taille = self.contenu.len() as u64;
            let debut = usize::try_from(debut.min(taille)).unwrap();
            let fin = debut.saturating_add(max).min(self.contenu.len());
            Ok(Tranche { octets: self.contenu[debut..fin].to_vec(), taille })
        }
    }

    /// Deux blocs et demi, pour que la lecture traverse vraiment des frontières de bloc.
    fn contenu(octets: usize) -> Vec<u8> {
        (0..octets).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn tout_le_fichier_se_lit_a_lidentique() {
        let attendu = contenu(TAILLE_BLOC * 2 + 7777);
        let mut f = FichierDistant::ouvrir(FauxPartage::de(attendu.clone()), "/a.flac").unwrap();
        let mut lu = Vec::new();
        f.read_to_end(&mut lu).unwrap();
        assert_eq!(lu, attendu);
    }

    #[test]
    fn la_taille_annoncee_est_celle_du_fichier_et_non_ce_qui_est_deja_lu() {
        let f = FichierDistant::ouvrir(FauxPartage::de(contenu(5_000_000)), "/a.flac").unwrap();
        // 5 Mo annoncés alors qu'un seul mégaoctet est en mémoire : c'est toute la différence
        // avec une longueur qui grandit au fil du téléchargement (ADR 0022).
        assert_eq!(f.taille(), 5_000_000);
    }

    #[test]
    fn un_deplacement_puis_une_lecture_rend_les_bons_octets() {
        let attendu = contenu(TAILLE_BLOC * 2);
        let mut f = FichierDistant::ouvrir(FauxPartage::de(attendu.clone()), "/a.flac").unwrap();
        let cible = TAILLE_BLOC as u64 + 1234;
        assert_eq!(f.seek(SeekFrom::Start(cible)).unwrap(), cible);
        let mut tampon = [0u8; 64];
        f.read_exact(&mut tampon).unwrap();
        let d = usize::try_from(cible).unwrap();
        assert_eq!(&tampon[..], &attendu[d..d + 64]);
    }

    #[test]
    fn se_deplacer_depuis_la_fin_vise_juste() {
        let attendu = contenu(100_000);
        let mut f = FichierDistant::ouvrir(FauxPartage::de(attendu.clone()), "/a.flac").unwrap();
        assert_eq!(f.seek(SeekFrom::End(-10)).unwrap(), 99_990);
        let mut lu = Vec::new();
        f.read_to_end(&mut lu).unwrap();
        assert_eq!(lu, attendu[99_990..]);
    }

    #[test]
    fn lire_apres_la_fin_rend_zero_octet_et_non_une_erreur() {
        let mut f = FichierDistant::ouvrir(FauxPartage::de(contenu(1000)), "/a.flac").unwrap();
        f.seek(SeekFrom::Start(5000)).unwrap();
        let mut tampon = [0u8; 16];
        // Un décodeur sonde légitimement la fin d'un flux : une erreur ici ferait passer un
        // sondage normal pour un fichier illisible.
        assert_eq!(f.read(&mut tampon).unwrap(), 0);
    }

    #[test]
    fn les_petites_lectures_ne_paient_pas_une_ouverture_chacune() {
        // L'affirmation de l'ADR 0022 — « le cache de blocs est une condition de
        // fonctionnement » — tenue par un compte, pas par une phrase. Sans cache, ce test
        // compterait une ouverture SMB par appel à `read`, soit 640 au lieu de 3.
        let mut f = FichierDistant::ouvrir(
            FauxPartage::de(contenu(TAILLE_BLOC * 2 + TAILLE_BLOC / 2)),
            "/a.flac",
        )
        .unwrap();
        let mut tampon = [0u8; 4096];
        let mut lectures = 0;
        while f.read(&mut tampon).unwrap() > 0 {
            lectures += 1;
        }
        assert!(lectures > 600, "lectures = {lectures}");
        assert_eq!(f.partage.ouvertures.get(), 3, "une ouverture par bloc, pas par lecture");
    }

    #[test]
    fn un_deplacement_dans_le_bloc_courant_ne_touche_pas_le_reseau() {
        let mut f = FichierDistant::ouvrir(FauxPartage::de(contenu(TAILLE_BLOC * 2)), "/a.flac")
            .unwrap();
        let apres_ouverture = f.partage.ouvertures.get();
        let mut tampon = [0u8; 8];
        for position in [0u64, 100, 50_000, 999_999] {
            f.seek(SeekFrom::Start(position)).unwrap();
            f.read_exact(&mut tampon).unwrap();
        }
        // Le sondage de format de symphonia saute beaucoup, et ses sauts retombent dans le
        // premier mégaoctet : s'ils coûtaient chacun un aller-retour, l'ouverture d'une piste
        // distante paierait des dizaines d'ouvertures avant la première note.
        assert_eq!(f.partage.ouvertures.get(), apres_ouverture);
    }

    #[test]
    fn un_deplacement_avant_le_debut_est_refuse_et_ne_panique_pas() {
        let mut f = FichierDistant::ouvrir(FauxPartage::de(contenu(1000)), "/a.flac").unwrap();
        assert!(f.seek(SeekFrom::Current(-1)).is_err());
        assert!(f.seek(SeekFrom::End(-5000)).is_err());
    }

    #[test]
    fn un_echec_du_partage_garde_le_texte_qui_nomme_lhote_et_le_chemin() {
        let mut partage = FauxPartage::de(contenu(TAILLE_BLOC * 2));
        partage.echoue_apres = Some(1);
        let mut f = FichierDistant::ouvrir(partage, "/musique/a.flac").unwrap();
        f.seek(SeekFrom::Start(TAILLE_BLOC as u64 + 1)).unwrap();
        let mut tampon = [0u8; 8];
        let message = f.read(&mut tampon).unwrap_err().to_string();
        // C'est ce texte que l'écran affichera : le perdre, c'est renvoyer Thomas à un journal
        // que personne n'ouvre (MESURES-V1 §8).
        assert!(message.contains("nas.local"), "{message}");
        assert!(message.contains("/musique/a.flac"), "{message}");
    }
}
