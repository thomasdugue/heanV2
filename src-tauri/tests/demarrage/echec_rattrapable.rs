//! **INV-24** — rien n'est créé dans le dossier de données avant que la migration ait rendu
//! son verdict, et un échec reste rattrapable au lancement suivant (ADR 0012).
//!
//! Et les trois cas qui décident de ce verdict : une bibliothèque vierge est écartée, un fichier
//! caché n'est pas une bibliothèque, une vraie bibliothèque arrête la migration sans empêcher
//! l'application de démarrer.

use std::path::{Path, PathBuf};

use hean_lib::demarrage;
use hean_lib::library::Depot;

use crate::bac::{autoriser_la_lecture, interdire_la_lecture, Bac};

/// **LE test.** Un échec de migration doit rester rattrapable au lancement suivant — ce que
/// l'ADR 0010 annonçait et que le code ne faisait pas, parce que `run()` ouvrait la
/// bibliothèque juste après l'échec et créait ainsi le dossier que le lancement suivant
/// prendrait pour une installation en place.
///
/// Ce qui était perdu sans ça : les 572 Mo du dossier de V1, et surtout la liste d'exclusion
/// — donc les pistes supprimées revenaient au premier scan (INV-06 🔴), définitivement.
#[test]
fn un_echec_de_migration_ne_cree_rien_et_reste_rattrapable() {
    let bac = Bac::neuf("echec-rattrapable");
    bac.poser_un_dossier_v1("/Musique/supprimee.flac");
    let interdite = bac.poser("covers/interdite.jpg", b"illisible");
    interdire_la_lecture(&interdite);

    // Premier lancement : la migration échoue.
    // `expect_err` demanderait `Debug` sur le dépôt, qu'il n'a pas — et qu'il n'a pas à
    // avoir : une bibliothèque entière n'est pas une chose qu'on imprime.
    let e = match bac.preparer() {
        Err(e) => e,
        Ok(_) => panic!("la migration doit échouer sur un fichier illisible"),
    };
    assert!(
        matches!(e, demarrage::Erreur::Migration { .. }),
        "l'échec doit être celui de la migration : {e}"
    );
    assert!(
        e.to_string().contains("vos données sont intactes"),
        "le message est lu par quelqu'un qui ne lit pas de code : {e}"
    );

    // Et c'est tout l'enjeu : RIEN n'a été créé dans le dossier de données.
    assert!(
        !bac.nouveau().exists(),
        "aucun dossier de données ne doit être créé après un échec de migration"
    );
    assert!(bac.ancien().join("playlists.json").exists(), "les données de V1 sont intactes");

    // Deuxième lancement, la cause de l'échec ayant disparu.
    autoriser_la_lecture(&interdite);
    let depot = bac.preparer().expect("le second lancement doit réussir");

    assert!(bac.nouveau().join("playlists.json").exists(), "les données sont arrivées");
    assert!(bac.ancien().exists(), "et l'ancien dossier est conservé (INV-26)");
    assert_eq!(depot.racines().expect("racines").len(), 1, "le dossier de musique est repris");
    assert_eq!(
        depot.chemins_exclus().expect("exclus"),
        vec!["/Musique/supprimee.flac".to_string()],
        "et la piste supprimée dans V1 reste exclue (INV-06)"
    );
}

/// Le même mécanisme, sans aucun échec : une version précédente de V2 a déjà démarré une
/// fois, donc le dossier de données contient une base — vide. Ce n'est pas une bibliothèque,
/// c'est le reliquat d'un démarrage, et ça ne doit pas éteindre la migration.
///
/// C'est le cas réel de toute machine ayant lancé une build des phases 0 à 2.
#[test]
fn une_bibliotheque_vierge_nempeche_pas_la_migration() {
    let bac = Bac::neuf("vierge");
    bac.poser_un_dossier_v1("/Musique/supprimee.flac");

    // Ce qu'une build antérieure laissait derrière elle : le dossier et une base vide.
    drop(Depot::ouvrir(&bac.nouveau().join("library.db")).expect("base d'une build antérieure"));
    assert!(bac.nouveau().join("library.db").exists());

    let depot = bac.preparer().expect("la migration doit se faire quand même");
    assert!(bac.nouveau().join("playlists.json").exists(), "les données sont arrivées");
    assert_eq!(depot.chemins_exclus().expect("exclus").len(), 1, "et l'exclusion est reprise");
}

/// Un fichier caché ne vaut pas une bibliothèque. Un `.DS_Store` déposé par un passage dans
/// le Finder suffisait à éteindre la migration, alors que l'inventaire, lui, ignore déjà les
/// fichiers cachés : les deux doivent s'accorder.
#[test]
fn un_fichier_cache_dans_la_cible_nempeche_pas_la_migration() {
    let bac = Bac::neuf("ds-store");
    bac.poser_un_dossier_v1("/Musique/supprimee.flac");
    std::fs::create_dir_all(bac.nouveau()).expect("hean");
    std::fs::write(bac.nouveau().join(".DS_Store"), b"du Finder").expect("ds_store");

    bac.preparer().expect("la migration doit se faire");
    assert!(bac.nouveau().join("playlists.json").exists());
    // Et le fichier du Finder n'a pas été détruit au passage : il a suivi, parce que
    // « ce n'est pas une donnée » n'autorise pas à supprimer le fichier de quelqu'un.
    assert!(
        bac.nouveau().join(".DS_Store").exists(),
        "le fichier caché doit être déplacé, pas supprimé"
    );
}

/// La limite, et elle ne bouge pas : **deux bibliothèques ne se fusionnent jamais**
/// (ADR 0010 §6). Une base qui contient quelque chose de réel arrête la migration — mais
/// l'application démarre, parce que ses données à elle sont intactes.
#[test]
fn une_vraie_bibliotheque_bloque_la_migration_sans_empecher_le_demarrage() {
    let bac = Bac::neuf("deja-peuplee");
    bac.poser_un_dossier_v1("/Musique/supprimee.flac");
    {
        let mut depot =
            Depot::ouvrir(&bac.nouveau().join("library.db")).expect("bibliothèque de V2");
        depot.ajouter_racine(Path::new("/Musique/choisi/dans/V2")).expect("racine");
    }

    let depot = bac.preparer().expect("l'application doit démarrer");
    assert_eq!(
        depot.racines().expect("racines"),
        vec![PathBuf::from("/Musique/choisi/dans/V2")],
        "la bibliothèque de V2 est intacte, rien n'a été fusionné"
    );
    assert!(bac.ancien().join("playlists.json").exists(), "et le dossier de V1 non plus");
    assert!(!bac.nouveau().join("playlists.json").exists(), "rien n'a été migré");
}
