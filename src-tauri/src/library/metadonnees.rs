//! Lecture et écriture des tags d'un fichier audio, via lofty.
//!
//! **INV-07 — la source de vérité d'une métadonnée est le fichier audio, pas la base.**
//! C'est la seule des quatre règles de cohérence de cache de V1 qui n'est pas rayée par
//! SQLite : une transaction règle la cohérence entre plusieurs copies internes, mais elle ne
//! dit rien du fichier. Si on n'écrit que dans la base, l'édition de l'utilisateur disparaît
//! dès qu'il ouvre le fichier dans un autre logiciel — et il n'a aucun moyen de comprendre
//! pourquoi.
//!
//! D'où l'ordre, qui est **la** décision de ce module : écrire le fichier, **puis le
//! relire**, et n'alimenter la base qu'avec ce que la relecture a rendu. Si l'écriture
//! échoue, la base n'a pas bougé. Si lofty normalise une valeur à sa façon, c'est sa version
//! qui entre dans la base — donc la base ne peut pas annoncer autre chose que le fichier.
//!
//! Vérifié par `src-tauri/tests/bibliotheque/edition.rs`.

use std::path::Path;

use lofty::config::WriteOptions;
use lofty::file::FileType;
use lofty::prelude::{Accessor, AudioFile, ItemKey, TaggedFileExt};
use lofty::tag::Tag;

use super::cle::AlbumKey;
use super::types::{Edition, Track};

/// Les métadonnées brutes lues dans un fichier, plus ce que le conteneur annonce.
#[derive(Clone)]
pub struct Lues {
    pub track: Track,
    /// La clé typée. Elle sort d'ici déjà construite pour que personne n'ait à se demander
    /// s'il faut normaliser (INV-01).
    pub cle: AlbumKey,
}

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error("lecture des tags de {chemin} : {source}")]
    Lecture {
        chemin: String,
        #[source]
        source: lofty::error::LoftyError,
    },
    #[error("écriture des tags de {chemin} : {source}")]
    Ecriture {
        chemin: String,
        #[source]
        source: lofty::error::LoftyError,
    },
}

/// Ce qui s'affiche quand lofty rend un type de fichier hors des sept formats supportés.
///
/// Un nom qui se lit comme un défaut, pas comme un format : « Unknown » se signale, là où
/// `Mpeg` ou `Mp4` passent pour des noms plausibles et ne se signalent jamais.
pub const FORMAT_INCONNU: &str = "Unknown";

/// Le nom affichable d'un format audio, ou `None` si lofty rend un type hors des sept
/// formats que l'app déclare supporter (features de symphonia dans `Cargo.toml`, et
/// `scan::EXTENSIONS`).
///
/// **Jamais `format!("{:?}", fichier.file_type())`.** C'est ce que ce module faisait
/// jusqu'au 2026-11-23 : la représentation de **débogage** d'un type de lofty, envoyée
/// telle quelle vers l'affichage. Elle annonçait `Mpeg` là où le fichier, ffprobe et la V1
/// écrivent tous `MP3`. La passe de vérité du 2026-11-23 a comparé 306 fichiers réels champ
/// par champ : sur 182 écarts avec la V1, **c'est le seul où la V1 avait raison** — et la
/// parité avec elle est la cible du projet (ADR 0004).
///
/// Les noms sont ceux que la V1 affichait, relevés dans son code et confirmés par le cache
/// de la V1 installée (`FLAC`, `MP3`, `WAV`). `Vorbis` est le seul choix neuf : la V1
/// repliait le `.ogg` sur « Other », ce qui n'est pas un nom de format. On nomme le codec,
/// comme pour ALAC et AAC, puisque OGG n'est que le conteneur.
///
/// Et surtout : une table explicite est liée par un contrat, un `Debug` ne l'est pas. Une
/// mise à jour de lofty peut renommer un variant sans que rien ne bronche — ici, elle ne
/// compile plus ou fait rougir `src-tauri/tests/bibliotheque/format.rs`.
pub fn nom_de_format(type_fichier: FileType, bits: Option<u8>) -> Option<&'static str> {
    Some(match type_fichier {
        FileType::Flac => "FLAC",
        FileType::Mpeg => "MP3",
        FileType::Wav => "WAV",
        FileType::Aiff => "AIFF",
        FileType::Vorbis => "Vorbis",
        // Un `.m4a` est un conteneur MP4 : il peut porter de l'ALAC (sans perte) ou de
        // l'AAC (avec perte). C'est la distinction qui compte pour un auditeur, et c'est
        // elle que le `Debug` écrasait — les deux s'affichaient `Mp4`. lofty ne renseigne
        // la profondeur de bits que pour le premier : c'est le test qu'utilisait la V1.
        FileType::Mp4 if bits.is_some() => "ALAC",
        FileType::Mp4 => "AAC",
        // AAC brut (ADTS), sans conteneur MP4. L'extension `.aac` n'est pas scannée, mais
        // lofty reconnaît le contenu, pas le nom du fichier.
        FileType::Aac => "AAC",
        _ => return None,
    })
}

/// Lit les tags et les propriétés audio d'un fichier.
///
/// Le repli quand un tag manque : le **nom de fichier** pour le titre, et la chaîne vide
/// pour l'artiste et l'album. La fixture `empty-tags.flac` est exactement ce cas — une
/// piste sans tags doit rester lisible et affichable, pas disparaître de la bibliothèque.
pub fn lire(chemin: &Path) -> Result<Lues, Erreur> {
    let fichier = lofty::read_from_path(chemin).map_err(|source| Erreur::Lecture {
        chemin: chemin.display().to_string(),
        source,
    })?;
    depuis_le_fichier_lu(fichier, chemin)
}

/// Lit les tags et les propriétés audio depuis des **octets déjà en mémoire**, et les
/// attribue au chemin donné — qui n'a pas à être un fichier de ce Mac.
///
/// C'est ce que la part réseau utilise (ADR 0016) : une piste SMB n'a pas de chemin local,
/// et `lofty::read_from_path` ne sait lire qu'un fichier du disque — c'est la raison exacte
/// pour laquelle V1 n'avait **aucune pochette** pour 91 % de sa bibliothèque
/// (`docs/MESURES-V1.md` §5). La même fonction de lecture sert donc les deux provenances :
/// une seule interprétation des tags, donc aucune chance que le local et le réseau
/// affichent deux choses différentes du même fichier.
///
/// ⚠️ **Les octets doivent couvrir ce dont le format a besoin pour sa durée.** Un en-tête
/// tronqué donne un titre juste et une durée fausse pour MP3, M4A et OGG — voir
/// `network::entete`, qui décide quels formats ont droit à une lecture partielle, et le test
/// `src-tauri/tests/reseau/entete.rs`, qui le vérifie en tronquant les fixtures.
pub fn lire_depuis(octets: &[u8], chemin_affiche: &str) -> Result<Lues, Erreur> {
    let echec = |source| Erreur::Lecture {
        chemin: chemin_affiche.to_string(),
        source,
    };
    // `Probe::new` et non `lofty::read_from`, qui n'accepte qu'un `&mut File` : c'est le même
    // chemin de code, un cran plus bas. `guess_file_type` renifle le CONTENU, donc le nom
    // du fichier distant n'a pas à être crédible.
    let fichier = lofty::probe::Probe::new(std::io::Cursor::new(octets))
        .guess_file_type()
        .map_err(|e| echec(lofty::error::LoftyError::from(e)))?
        .read()
        .map_err(echec)?;
    depuis_le_fichier_lu(fichier, Path::new(chemin_affiche))
}

/// Le corps commun aux deux lectures. Il prend le fichier lofty déjà ouvert et le chemin à
/// afficher : tout ce qui suit — le repli de titre, la table de formats, la clé d'album —
/// est identique, et doit le rester.
fn depuis_le_fichier_lu(fichier: lofty::file::TaggedFile, chemin: &Path) -> Result<Lues, Erreur> {
    let proprietes = fichier.properties();
    let tag = fichier.primary_tag().or_else(|| fichier.first_tag());
    let bits = proprietes.bit_depth();

    // Un type hors liste ne retombe PAS sur le `Debug` de lofty : c'est le défaut qu'on
    // corrige, et il ne se voyait pas — `Mpeg` ressemble à un nom de format. Le repli se
    // voit donc deux fois : « Unknown » dans l'interface, et une ligne dans le journal
    // (INV-21 — ce qui n'est pas journalisé est invisible chez les testeurs). Le scan
    // n'accepte que sept extensions : ce cas veut dire que le contenu d'un fichier ne
    // correspond pas à son nom, ce qui se signale au lieu de se maquiller.
    let format = nom_de_format(fichier.file_type(), bits).unwrap_or_else(|| {
        tracing::warn!(
            chemin = %chemin.display(),
            type_lofty = ?fichier.file_type(),
            "format audio hors des sept formats supportés"
        );
        FORMAT_INCONNU
    });

    let texte = |lu: Option<std::borrow::Cow<'_, str>>| lu.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let titre = tag.and_then(|t| texte(t.title())).unwrap_or_else(|| {
        chemin
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    });
    let artiste = tag.and_then(|t| texte(t.artist())).unwrap_or_default();
    let album = tag.and_then(|t| texte(t.album())).unwrap_or_default();
    let cle = AlbumKey::new(&album);

    Ok(Lues {
        track: Track {
            path: chemin.display().to_string(),
            titre,
            artiste,
            album,
            album_artiste: tag
                .and_then(|t| t.get_string(&ItemKey::AlbumArtist))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            cle_album: cle.as_str().to_string(),
            annee: tag.and_then(|t| t.year()),
            piste: tag.and_then(|t| t.track()),
            disque: tag.and_then(|t| t.disk()),
            genre: tag.and_then(|t| texte(t.genre())),
            // `as` tronquerait une durée absurde en une durée plausible ; `try_into` la
            // rend simplement nulle, ce qui se voit.
            duree_ms: u32::try_from(proprietes.duration().as_millis()).unwrap_or(0),
            format: format.to_string(),
            echantillonnage: proprietes.sample_rate(),
            bits: bits.map(u32::from),
            debit: proprietes.audio_bitrate(),
        },
        cle,
    })
}

/// Écrit une édition **dans le fichier**, puis relit le fichier et rend ce qu'il contient.
///
/// L'ordre est l'invariant : rien ne remonte d'ici qui ne soit déjà sur le disque.
pub fn ecrire(chemin: &Path, edition: &Edition) -> Result<Lues, Erreur> {
    let echec = |source| Erreur::Ecriture {
        chemin: chemin.display().to_string(),
        source,
    };
    let mut fichier = lofty::read_from_path(chemin).map_err(echec)?;

    // Un fichier sans aucun tag (fixture `empty-tags.flac`) n'a pas de tag à modifier : il
    // faut en créer un du type que le conteneur accepte, sinon l'édition serait perdue
    // sans erreur — le mode de défaut exact que INV-21 et INV-07 visent tous les deux.
    let type_principal = fichier.primary_tag_type();
    if fichier.primary_tag().is_none() {
        // Le tag existant, s'il y en a un d'un autre type, est repris tel quel : le
        // remplacer perdrait les champs qu'on n'édite pas.
        let reprise = fichier.first_tag().cloned();
        fichier.insert_tag(reprise.unwrap_or_else(|| Tag::new(type_principal)));
    }
    let tag = fichier.primary_tag_mut().expect("un tag principal vient d'être garanti");

    // Les guillemets ne subissent AUCUN traitement : W006 — en V1, `escapeHtml()`
    // n'échappait pas les guillemets, et un tag qui en contenait était tronqué à la
    // sauvegarde. Un tag est une chaîne de caractères, pas du HTML ; l'échappement est
    // l'affaire de l'affichage, et Svelte s'en charge au rendu.
    if let Some(v) = &edition.titre {
        tag.set_title(v.clone());
    }
    if let Some(v) = &edition.artiste {
        tag.set_artist(v.clone());
    }
    if let Some(v) = &edition.album {
        tag.set_album(v.clone());
    }
    if let Some(v) = edition.annee {
        tag.set_year(v);
    }
    if let Some(v) = edition.piste {
        tag.set_track(v);
    }
    if let Some(v) = &edition.genre {
        tag.set_genre(v.clone());
    }

    fichier.save_to_path(chemin, WriteOptions::default()).map_err(echec)?;

    // LA relecture. Ce qui part vers la base vient du fichier, pas de ce qu'on croyait y
    // avoir écrit.
    lire(chemin)
}

#[cfg(test)]
mod tests {
    use super::{nom_de_format, FORMAT_INCONNU};
    use lofty::file::FileType;

    /// La prémisse, épinglée : le `Debug` de lofty n'est pas un nom de format.
    ///
    /// Si une version de lofty se mettait à écrire `MP3` dans son `Debug`, ce test
    /// échouerait — et cet échec serait la bonne nouvelle. Il dit à la relecture suivante
    /// pourquoi la table existe, sans avoir à relire un journal de session.
    #[test]
    fn le_debug_de_lofty_nest_pas_un_nom_de_format() {
        assert_eq!(format!("{:?}", FileType::Mpeg), "Mpeg");
        assert_eq!(nom_de_format(FileType::Mpeg, None), Some("MP3"));
        assert_eq!(format!("{:?}", FileType::Mp4), "Mp4");
    }

    /// Le même conteneur MP4 rend deux noms, selon qu'il porte de l'ALAC ou de l'AAC.
    #[test]
    fn un_m4a_se_distingue_par_sa_profondeur_de_bits() {
        assert_eq!(nom_de_format(FileType::Mp4, Some(16)), Some("ALAC"));
        assert_eq!(nom_de_format(FileType::Mp4, None), Some("AAC"));
    }

    /// Le détecteur éprouvé contre la faute qu'il est censé voir : un type hors liste.
    ///
    /// `Opus` est un vrai variant de lofty que l'app ne supporte pas (il n'est ni dans les
    /// features de symphonia ni dans `scan::EXTENSIONS`). La fonction doit rendre `None`,
    /// et c'est `lire` qui journalise et affiche `FORMAT_INCONNU` — jamais un `Debug`.
    #[test]
    fn un_type_hors_liste_ne_rend_aucun_nom() {
        for hors_liste in [FileType::Opus, FileType::Ape, FileType::Speex, FileType::WavPack] {
            assert_eq!(nom_de_format(hors_liste, None), None, "{hors_liste:?}");
            assert_eq!(nom_de_format(hors_liste, Some(24)), None, "{hors_liste:?}");
        }
        assert_ne!(FORMAT_INCONNU, format!("{:?}", FileType::Opus));
    }
}
