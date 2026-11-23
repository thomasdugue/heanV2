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
use lofty::prelude::{Accessor, AudioFile, ItemKey, TaggedFileExt};
use lofty::tag::Tag;

use super::cle::AlbumKey;
use super::types::{Edition, Track};

/// Les métadonnées brutes lues dans un fichier, plus ce que le conteneur annonce.
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

    let proprietes = fichier.properties();
    let tag = fichier.primary_tag().or_else(|| fichier.first_tag());

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
            format: format!("{:?}", fichier.file_type()),
            echantillonnage: proprietes.sample_rate(),
            bits: proprietes.bit_depth().map(u32::from),
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
