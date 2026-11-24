//! INV-01 — un album = une seule entrée. Voir `main.rs` pour le principe commun.

use crate::commun::Bac;

/// La régression la plus coûteuse de V1, sur les deux fixtures faites pour elle.
///
/// `tags-nfd.flac` porte l'album « Café Noir » en **NFD** et `artist-feat.flac` le même
/// album en NFC, avec un artiste « Zoé Léger feat. Hean ». V1 produisait deux albums : une
/// fois par la normalisation manquante, une fois par la déduplication sur `artiste + album`
/// (régression W003). Ici la clé vient du type `AlbumKey` et ne prend que l'album.
#[test]
fn deux_ecritures_du_meme_album_ne_font_quune_entree() {
    let bac = Bac::avec("inv01", &["tags-nfd.flac", "artist-feat.flac"]);
    let depot = bac.scanne();
    let pistes = depot.lister_pistes().expect("liste");

    assert_eq!(pistes.len(), 2, "les deux pistes doivent être dans la bibliothèque");
    let cles: std::collections::BTreeSet<_> = pistes.iter().map(|p| p.cle_album.as_str()).collect();
    assert_eq!(cles.len(), 1, "un seul album attendu, trouvé : {cles:?}");

    // Et les artistes DIFFÈRENT bien : sans quoi le test passerait pour la mauvaise raison.
    let artistes: std::collections::BTreeSet<_> = pistes.iter().map(|p| p.artiste.as_str()).collect();
    assert_eq!(artistes.len(), 2, "les deux pistes doivent avoir des artistes distincts");
}

/// N pistes d'un même album produisent exactement 1 entrée, quels que soient leurs
/// artistes et quel que soit l'ordre dans lequel on les scanne (HARNAIS.md §4).
#[test]
fn le_nombre_dalbums_ne_depend_pas_de_lordre_du_scan() {
    for ordre in [["tags-nfd.flac", "artist-feat.flac"], ["artist-feat.flac", "tags-nfd.flac"]] {
        let bac = Bac::avec("inv01-ordre", &ordre);
        let depot = bac.scanne();
        let cles: std::collections::BTreeSet<_> = depot
            .lister_pistes()
            .expect("liste")
            .iter()
            .map(|p| p.cle_album.clone())
            .collect();
        assert_eq!(cles.len(), 1, "ordre {ordre:?}");
    }
}
