//! Combien d'octets lire d'un fichier distant pour en extraire les tags — et **pour quels
//! formats c'est légitime** (ADR 0016 §3).
//!
//! Lire un fichier entier sur un NAS coûte des secondes ; lire son en-tête coûte des
//! dizaines de millisecondes. Sur 3 137 pistes, la différence est entre deux minutes et une
//! heure. Mais l'en-tête ne porte pas la même chose selon le format, et **la durée est le
//! champ qui trahit** :
//!
//! | Format | Où est la durée | En-tête suffisant ? |
//! |---|---|---|
//! | FLAC | bloc `STREAMINFO`, au début | **oui** |
//! | WAV | taille du chunk `data`, dans l'en-tête | **oui** |
//! | AIFF | `numSampleFrames` du chunk `COMM`, au début | **oui** |
//! | MP3 | en-tête Xing s'il existe, **sinon déduite de la taille du fichier** | non |
//! | M4A / AAC | atome `moov`, qui peut être **à la fin** | non |
//! | OGG Vorbis | granule de la **dernière** page | non |
//!
//! Les trois du bas, lus partiellement, donneraient une durée **plausible et fausse** : le
//! pire mode de défaut, celui qui ne se signale pas. Ils sont donc lus en entier. Chez
//! Thomas cela fait 82 MP3 + 19 M4A sur 3 443 pistes, soit **3 %**
//! ([`docs/MESURES-V1.md`](../../../docs/MESURES-V1.md) §1).
//!
//! Et ce n'est pas une affirmation : `src-tauri/tests/reseau/entete.rs` tronque **chaque
//! fixture audio** du dépôt et compare champ par champ ce que la lecture partielle rend avec
//! ce que le fichier entier rend. Un format qui apparaîtrait dans la colonne de gauche à
//! tort ferait rougir ce test — et il rougirait aussi si une version de lofty changeait de
//! comportement.

/// L'en-tête lu d'un fichier distant : **1 Mio**.
///
/// V1 lisait 512 Kio et devait, pour cette raison, embarquer son propre parseur de blocs
/// FLAC (~200 lignes) : au-delà de son curseur, lofty suivait les offsets du `SEEKTABLE` et
/// échouait. Un Mio laisse place à une pochette intégrée de taille courante, et le repli sur
/// le fichier entier couvre le reste — ce qui évite une seconde implémentation de FLAC à
/// maintenir (ADR 0016, alternatives écartées).
pub const TAILLE_ENTETE: usize = 1024 * 1024;

/// Les extensions dont la durée se lit dans un bloc d'en-tête, donc les seules qu'on se
/// permet de lire partiellement.
///
/// La liste dit ce qu'elle **autorise**, pas ce qu'elle interdit, et c'est volontaire : un
/// format qui arrive demain dans `scan::EXTENSIONS` sera lu en entier par défaut — lent et
/// juste — au lieu d'être lu partiellement par omission, c'est-à-dire vite et faux.
pub const LISIBLES_EN_ENTETE: &[&str] = &["flac", "wav", "aiff", "aif"];

/// Combien d'octets demander pour ce fichier : `Some(n)` pour un en-tête, `None` pour tout.
pub fn a_lire(chemin: &str) -> Option<usize> {
    let ext = std::path::Path::new(chemin)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if LISIBLES_EN_ENTETE.contains(&ext.as_str()) {
        Some(TAILLE_ENTETE)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{a_lire, LISIBLES_EN_ENTETE, TAILLE_ENTETE};
    use crate::library::scan;

    #[test]
    fn seuls_les_formats_a_entete_sont_lus_partiellement() {
        for ext in LISIBLES_EN_ENTETE {
            assert_eq!(a_lire(&format!("/a/b.{ext}")), Some(TAILLE_ENTETE), ".{ext}");
            // La casse d'une extension ne doit pas décider d'une stratégie de lecture :
            // `.FLAC` sort de certains logiciels de rip.
            assert_eq!(a_lire(&format!("/a/b.{}", ext.to_uppercase())), Some(TAILLE_ENTETE));
        }
        for ext in ["mp3", "m4a", "ogg", "aac"] {
            assert_eq!(a_lire(&format!("/a/b.{ext}")), None, ".{ext} se lit en entier");
        }
        // Sans extension : en entier. Le repli est lent et juste, pas vite et faux.
        assert_eq!(a_lire("/a/b"), None);
    }

    /// **La liste autorise, elle n'interdit pas.** Donc tout format supporté qui n'est pas
    /// dans `LISIBLES_EN_ENTETE` doit être lu en entier — et ce test le vérifie en dérivant
    /// sa matière de `scan::EXTENSIONS`, pas d'une seconde liste écrite à la main. Un format
    /// ajouté demain au scan est couvert le jour où il est ajouté.
    #[test]
    fn tout_format_supporte_hors_liste_est_lu_en_entier() {
        let partiels: Vec<&&str> = scan::EXTENSIONS
            .iter()
            .filter(|e| a_lire(&format!("x.{e}")).is_some())
            .collect();
        let attendus: Vec<&&str> = scan::EXTENSIONS
            .iter()
            .filter(|e| LISIBLES_EN_ENTETE.contains(e))
            .collect();
        assert_eq!(partiels, attendus);
        assert!(!partiels.is_empty(), "sinon ce test ne regarde rien");
        // Et la liste des autorisés est bien un sous-ensemble strict de ce que l'app lit :
        // si elle les couvrait tous, la question de la durée ne se poserait plus, et ce
        // module n'aurait plus de raison d'être.
        assert!(partiels.len() < scan::EXTENSIONS.len());
    }
}
