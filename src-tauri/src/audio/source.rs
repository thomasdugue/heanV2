//! Un flux d'octets dont la longueur est connue, présenté à symphonia.
//!
//! Dix lignes utiles, et une raison de zone : symphonia veut un `MediaSource`, c'est-à-dire
//! un `Read + Seek` qui sait dire s'il est déplaçable et combien il pèse. Un fichier local
//! l'est déjà. Une piste du NAS, non — et c'est ici qu'elle le devient, **sans que ce module
//! sache ce qu'est SMB** : il ne voit qu'un flux et un nombre. Le réseau, de son côté,
//! n'importe rien de l'audio. Les deux zones restent donc séparées (ADR 0022).

use std::io::{Read, Seek};

use symphonia::core::io::MediaSource;

/// Un flux déplaçable dont on connaît la taille.
pub struct SourceMesuree<R> {
    flux: R,
    taille: u64,
}

impl<R> SourceMesuree<R> {
    /// `taille` est la longueur **réelle** du média, pas ce qui en a déjà été lu.
    ///
    /// La distinction n'est pas théorique : annoncer une longueur qui grandit ferait viser à
    /// côté tout déplacement fait avant la fin du transfert, et annoncer `None` ferait traiter
    /// le flux comme non déplaçable — donc une barre de position morte sur 91 % de la
    /// bibliothèque.
    pub fn nouvelle(flux: R, taille: u64) -> Self {
        Self { flux, taille }
    }
}

impl<R: Read> Read for SourceMesuree<R> {
    fn read(&mut self, destination: &mut [u8]) -> std::io::Result<usize> {
        self.flux.read(destination)
    }
}

impl<R: Seek> Seek for SourceMesuree<R> {
    fn seek(&mut self, de: std::io::SeekFrom) -> std::io::Result<u64> {
        self.flux.seek(de)
    }
}

impl<R: Read + Seek + Send + Sync> MediaSource for SourceMesuree<R> {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.taille)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, SeekFrom};

    use super::*;

    #[test]
    fn la_longueur_annoncee_est_celle_quon_a_donnee_et_le_flux_est_deplacable() {
        let source = SourceMesuree::nouvelle(Cursor::new(vec![7u8; 300]), 300);
        assert!(source.is_seekable());
        assert_eq!(source.byte_len(), Some(300));
    }

    #[test]
    fn lire_et_se_deplacer_traversent_jusquau_flux() {
        let mut source = SourceMesuree::nouvelle(Cursor::new((0..50u8).collect::<Vec<_>>()), 50);
        assert_eq!(source.seek(SeekFrom::Start(10)).unwrap(), 10);
        let mut tampon = [0u8; 4];
        source.read_exact(&mut tampon).unwrap();
        assert_eq!(tampon, [10, 11, 12, 13]);
    }
}
