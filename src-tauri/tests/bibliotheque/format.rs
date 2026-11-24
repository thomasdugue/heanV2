//! INV-23 — un libellé destiné à l'affichage ne vient jamais d'un `{:?}`.
//!
//! Déplacé ici le 2026-11-23 : ces tests étaient écrits à la fin de `bibliotheque.rs`, que
//! la PR #13 a découpé pour repasser sous la limite de 400 lignes. Le thème a son module,
//! comme `main.rs` le demande.

use hean_lib::library::metadonnees::{lire, nom_de_format};
use lofty::file::FileType;

use crate::commun::fixture;

/// Chaque fixture rend le nom que la V1 affichait, sur les sept formats supportés.
///
/// Le défaut que ce test rejoue : `lire` remplissait `format` avec
/// `format!("{:?}", fichier.file_type())`, c'est-à-dire le `Debug` d'un type de lofty
/// envoyé vers l'affichage. Les 40 MP3 de la bibliothèque de Thomas annonçaient `Mpeg`, et
/// les deux `.m4a` annonçaient tous les deux `Mp4` — ALAC et AAC confondus. C'était le seul
/// des 182 écarts de la passe de vérité du 2026-11-23 où la V1 avait raison.
///
/// Remettre le `{:?}` fait rougir six des sept lignes ci-dessous. Et si une mise à jour de
/// lofty renomme un variant, c'est ici que ça se voit, pas chez un testeur.
#[test]
fn chaque_format_supporte_rend_son_nom_daffichage() {
    let attendu = [
        ("flac-16-44.flac", "FLAC"),
        ("mp3-320.mp3", "MP3"),
        ("wav.wav", "WAV"),
        ("aiff.aiff", "AIFF"),
        // Même conteneur MP4, deux codecs : c'est la distinction que le `Debug` écrasait.
        ("alac.m4a", "ALAC"),
        ("aac.m4a", "AAC"),
        ("vorbis.ogg", "Vorbis"),
    ];

    for (nom, format) in attendu {
        // Lecture seule : aucune raison de recopier la fixture, rien n'écrit ici.
        let lues = lire(&fixture(nom)).unwrap_or_else(|e| panic!("{nom} : {e}"));
        assert_eq!(lues.track.format, format, "{nom}");
    }
}

/// Les sept formats supportés rendent sept noms distincts — et le test interroge la
/// fonction, il ne recopie pas sa table.
///
/// Première écriture de ce test : il dédupliquait un **tableau littéral** et comparait sa
/// longueur à elle-même. Vert quoi que fasse le code de production — « de la confiance sans
/// en mériter », la famille exacte du détecteur d'INV-06 qui cherchait `\bTrack\b` là où
/// specta émet `Track_Serialize[]` : zéro trouvaille sur huit commandes, auto-test vert
/// pendant tout ce temps. Relevé par `invariant-guardian` sur ce diff même.
///
/// Les sept formats, c'est bien sept **appels** : le conteneur MP4 compte pour deux, selon
/// qu'il porte de l'ALAC ou de l'AAC. Un nom en double y rougit maintenant, et un huitième
/// format qui se replierait sur un nom existant aussi.
#[test]
fn les_sept_formats_rendent_sept_noms_distincts() {
    let sept: [(FileType, Option<u8>); 7] = [
        (FileType::Flac, None),
        (FileType::Mpeg, None),
        (FileType::Wav, None),
        (FileType::Aiff, None),
        (FileType::Vorbis, None),
        (FileType::Mp4, Some(16)),
        (FileType::Mp4, None),
    ];

    let noms: Vec<&str> = sept
        .iter()
        .map(|(t, bits)| nom_de_format(*t, *bits).unwrap_or_else(|| panic!("{t:?} n'a pas de nom")))
        .collect();

    let mut distincts = noms.clone();
    distincts.sort_unstable();
    distincts.dedup();
    assert_eq!(distincts.len(), 7, "deux formats affichent le même nom : {noms:?}");
}
