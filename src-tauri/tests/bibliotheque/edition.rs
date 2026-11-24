//! La source de vérité est le fichier audio (INV-07), et l'édition qui y écrit : survie au
//! redémarrage (V015), guillemets (W006), bulk edit (B08).

use crate::commun::Bac;
use hean_lib::library::Edition;

// =====================================================================================
// INV-07 — la source de vérité est le fichier audio
// =====================================================================================

/// Une métadonnée éditée est **dans le fichier**, pas seulement dans la base.
///
/// C'est le test qui fait quitter la dette à INV-07 : on édite par le dépôt, puis on relit
/// le fichier avec un dépôt tout neuf **dont la base a été supprimée**. Si l'écriture
/// n'était allée qu'en base, la relecture rendrait l'ancienne valeur — exactement ce que
/// l'utilisateur de V1 constatait en ouvrant son fichier dans un autre logiciel.
#[test]
fn une_metadonnee_editee_est_ecrite_dans_le_fichier_audio() {
    let bac = Bac::avec("inv07", &["tags-nfd.flac"]);
    let chemin = bac.chemin("tags-nfd.flac");
    {
        let mut depot = bac.scanne();
        let piste = depot
            .editer(&chemin, &Edition { titre: Some("Deuxième Première".into()), ..Default::default() })
            .expect("édition");
        assert_eq!(piste.titre, "Deuxième Première");
    }

    // La base est jetée : seul le fichier reste. C'est lui la source de vérité.
    std::fs::remove_file(bac.dossier.join("library.db")).expect("suppression de la base");
    let depot = bac.scanne();
    let pistes = depot.lister_pistes().expect("liste");
    assert_eq!(pistes.len(), 1);
    assert_eq!(
        pistes[0].titre, "Deuxième Première",
        "le titre doit venir du fichier, pas d'un cache"
    );
}

/// V015 — une métadonnée éditée survit au prochain démarrage.
///
/// Même scénario, vu depuis l'utilisateur : on édite, on « quitte » (le dépôt est détruit,
/// la base fermée), on « redémarre » en réouvrant la même base, **sans rescanner**. En V1,
/// `METADATA_CACHE` et `TRACKS_CACHE` pouvaient être désynchronisés et l'édition se perdait.
#[test]
fn v015_une_metadonnee_editee_survit_au_redemarrage() {
    let bac = Bac::avec("v015", &["accents-éàü.flac"]);
    let chemin = bac.chemin("accents-éàü.flac");
    {
        let mut depot = bac.scanne();
        depot
            .editer(&chemin, &Edition { genre: Some("Jazz Manouche".into()), annee: Some(1954), ..Default::default() })
            .expect("édition");
    }

    let depot = bac.depot(); // réouverture : aucun scan, on relit la base telle quelle
    let piste = depot.piste(&chemin).expect("lecture").expect("la piste doit être là");
    assert_eq!(piste.genre.as_deref(), Some("Jazz Manouche"));
    assert_eq!(piste.annee, Some(1954));
}

/// W006 — un tag contenant un guillemet n'est pas tronqué à la sauvegarde.
///
/// En V1, `escapeHtml()` n'échappait pas les guillemets et le tag était tronqué **à
/// l'enregistrement**, donc la donnée était perdue, pas seulement mal affichée. Le tour
/// complet est éprouvé ici : fichier → lofty → SQLite → relecture, avec les quatre
/// caractères qui cassent du HTML et celui qui casse du SQL.
#[test]
fn w006_un_tag_avec_guillemets_traverse_tout_sans_etre_tronque() {
    let perilleux = r#"Tiré d'un "live" <b>&</b> ailleurs — 100% ' ok"#;
    let bac = Bac::avec("w006", &["no-cover.flac"]);
    let chemin = bac.chemin("no-cover.flac");

    {
        let mut depot = bac.scanne();
        let piste = depot
            .editer(
                &chemin,
                &Edition {
                    titre: Some(perilleux.to_string()),
                    album: Some(format!("Album \"{perilleux}\"")),
                    ..Default::default()
                },
            )
            .expect("édition");
        assert_eq!(piste.titre, perilleux, "la valeur rendue vient de la relecture du fichier");
    }

    // Après redémarrage, et après un nouveau scan qui relit les tags du fichier.
    std::fs::remove_file(bac.dossier.join("library.db")).expect("suppression de la base");
    let depot = bac.scanne();
    let piste = depot.piste(&chemin).expect("lecture").expect("piste");
    assert_eq!(piste.titre, perilleux, "titre tronqué ou transformé");
    assert!(piste.album.contains('"'), "album : {}", piste.album);
    // La clé d'album porte les mêmes guillemets : rien n'a été échappé en route.
    assert!(piste.cle_album.contains('"'), "clé : {}", piste.cle_album);
}

// =====================================================================================
// Bulk edit (B08)
// =====================================================================================

/// Le bulk edit n'écrit QUE les champs remplis. V1 affichait « mixed values » pour les
/// champs qui diffèrent entre les pistes sélectionnées, et ne devait pas les écraser.
#[test]
fn un_bulk_edit_ne_touche_pas_aux_champs_laisses_vides() {
    let bac = Bac::avec("bulk", &["tags-nfd.flac", "artist-feat.flac"]);
    let chemins = vec![bac.chemin("tags-nfd.flac"), bac.chemin("artist-feat.flac")];
    let mut depot = bac.scanne();
    let titres_avant: Vec<_> = depot
        .lister_pistes()
        .expect("liste")
        .iter()
        .map(|p| p.titre.clone())
        .collect();

    let apres = depot
        .editer_en_lot(&chemins, &Edition { genre: Some("Chanson".into()), ..Default::default() })
        .expect("bulk edit");
    assert_eq!(apres.len(), 2);
    assert!(apres.iter().all(|p| p.genre.as_deref() == Some("Chanson")));

    let titres_apres: Vec<_> = depot
        .lister_pistes()
        .expect("liste")
        .iter()
        .map(|p| p.titre.clone())
        .collect();
    assert_eq!(titres_avant, titres_apres, "les titres ne devaient pas bouger");
    assert_ne!(titres_avant[0], titres_avant[1], "les deux titres diffèrent bien");
}

/// Une édition vide ne réécrit aucun fichier. Réécrire un FLAC pour rien déplace des
/// mégaoctets sur le disque de l'utilisateur — et sur un NAS, c'est bien pire.
#[test]
fn une_edition_vide_ne_reecrit_rien() {
    let bac = Bac::avec("vide", &["tags-nfd.flac"]);
    let chemin = bac.chemin("tags-nfd.flac");
    let mut depot = bac.scanne();
    let avant = std::fs::metadata(&chemin).expect("meta").modified().expect("mtime");

    let rendu = depot
        .editer_en_lot(std::slice::from_ref(&chemin), &Edition::default())
        .expect("édition vide");
    assert!(rendu.is_empty());

    let apres = std::fs::metadata(&chemin).expect("meta").modified().expect("mtime");
    assert_eq!(avant, apres, "le fichier ne devait pas être touché");
}

/// Éditer une piste que la bibliothèque ne connaît pas est une erreur, pas un silence.
#[test]
fn editer_une_piste_inconnue_remonte_une_erreur() {
    let bac = Bac::avec("inconnue", &["tags-nfd.flac"]);
    let mut depot = bac.scanne();
    let e = depot
        .editer("/nulle/part/absent.flac", &Edition { titre: Some("X".into()), ..Default::default() })
        .expect_err("doit échouer");
    assert!(format!("{e}").contains("inconnue"), "{e}");
}

/// Une édition vide rend la piste telle quelle, et ne prétend pas qu'elle est inconnue.
#[test]
fn une_edition_vide_ne_remonte_pas_une_fausse_erreur() {
    let bac = Bac::avec("vide-message", &["tags-nfd.flac"]);
    let chemin = bac.chemin("tags-nfd.flac");
    let mut depot = bac.scanne();

    let piste = depot.editer(&chemin, &Edition::default()).expect("une édition vide est un non-événement");
    assert_eq!(piste.path, chemin);

    // Et une piste réellement inconnue, elle, remonte bien l'erreur — sinon la correction
    // du message aurait fait disparaître le message utile avec le faux.
    let e = depot
        .editer("/nulle/part/absent.flac", &Edition::default())
        .expect_err("une piste inconnue reste une erreur");
    assert!(format!("{e}").contains("inconnue"), "{e}");
}
