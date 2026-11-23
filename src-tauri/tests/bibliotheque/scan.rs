//! Le scan : exclusion permanente (INV-06), scan différentiel (V004), fichiers illisibles
//! ou abîmés (INV-22), et la persistance de la base.

use std::path::Path;

use crate::commun::Bac;

// =====================================================================================
// INV-06 — la suppression est permanente
// =====================================================================================

/// Une piste exclue ne revient pas, **même après un nouveau scan** alors que le fichier est
/// toujours là. C'est le cœur d'INV-06 : en V1 il fallait répéter le filtre à chaque point
/// d'entrée, et le quatrième point d'entrée l'oubliait.
#[test]
fn une_piste_exclue_ne_revient_pas_apres_un_scan() {
    let bac = Bac::avec("inv06", &["tags-nfd.flac", "artist-feat.flac"]);
    let chemin = bac.chemin("tags-nfd.flac");
    let mut depot = bac.scanne();
    assert_eq!(depot.lister_pistes().expect("liste").len(), 2);

    depot.exclure(&chemin).expect("exclusion");
    assert_eq!(depot.lister_pistes().expect("liste").len(), 1, "exclusion immédiate");

    let rapport = depot.scanner().expect("rescan");
    assert_eq!(rapport.ajoutees, 0, "le scan ne doit pas réintroduire la piste exclue");
    assert_eq!(depot.lister_pistes().expect("liste").len(), 1);
    assert!(
        Path::new(&chemin).exists(),
        "le fichier reste sur le disque : c'est la bibliothèque qui l'ignore"
    );

    // Et la piste reste invisible même à la lecture unitaire : le filtre n'a pas deux
    // comportements selon le chemin d'appel.
    assert!(depot.piste(&chemin).expect("lecture").is_none());
}

// =====================================================================================
// V004 — pas de rechargement si le scan n'a rien trouvé
// =====================================================================================

/// Le scan différentiel : deux passages de suite, le second ne trouve rien.
///
/// C'est la condition côté backend de V004. Si le second scan annonçait des modifications,
/// l'interface rechargerait sa liste pour rien — et c'est exactement la course que V1
/// déclenchait avec son `scan_complete` inconditionnel.
#[test]
fn v004_un_second_scan_ne_trouve_rien_de_nouveau() {
    let bac = Bac::avec("v004", &["tags-nfd.flac", "empty-tags.flac"]);
    let mut depot = bac.depot();
    depot.ajouter_racine(&bac.dossier).expect("racine");

    let premier = depot.scanner().expect("premier scan");
    assert_eq!(premier.ajoutees, 2, "{premier:?}");

    let second = depot.scanner().expect("second scan");
    assert!(second.sans_changement(), "{second:?}");

    // Un fichier qui disparaît, en revanche, EST un changement.
    std::fs::remove_file(bac.dossier.join("empty-tags.flac")).expect("suppression");
    let troisieme = depot.scanner().expect("troisième scan");
    assert_eq!(troisieme.disparues, 1, "{troisieme:?}");
    assert!(!troisieme.sans_changement());
}

/// Un fichier dont les tags sont illisibles est **compté et sauté**, pas fatal.
///
/// La faute est INJECTÉE — un fichier d'extension `.flac` qui ne contient pas du FLAC —
/// parce qu'aucune des fixtures abîmées ne la produit (voir le test suivant) et qu'un
/// détecteur qu'on n'a pas vu attraper quelque chose ne garantit rien. Une bibliothèque de
/// 10 000 pistes ne doit pas se perdre entière à cause d'un fichier.
#[test]
fn un_fichier_illisible_ninterrompt_pas_le_scan() {
    let bac = Bac::avec("illisible", &["flac-16-44.flac"]);
    std::fs::write(bac.dossier.join("faux.flac"), b"ceci n'est pas du FLAC").expect("faux fichier");

    let mut depot = bac.depot();
    depot.ajouter_racine(&bac.dossier).expect("racine");
    let rapport = depot.scanner().expect("le scan ne doit pas échouer");

    assert_eq!(rapport.ajoutees, 1, "la piste saine doit entrer : {rapport:?}");
    assert_eq!(rapport.illisibles, 1, "le fichier illisible doit être compté : {rapport:?}");
    assert_eq!(depot.lister_pistes().expect("liste").len(), 1);
}

/// MESURE, et elle est contre l'attente — INV-22, vu depuis le lecteur de tags.
///
/// La phase 1 avait mesuré le comportement du **décodeur** sur les trois façons dont un
/// fichier s'abîme : seul un en-tête STREAMINFO incohérent fait échouer symphonia, les deux
/// autres passent en silence. Mesuré ici sur **lofty** : les trois passent, l'en-tête abîmé
/// compris. Les deux bibliothèques ne lisent pas les mêmes octets et ne s'accordent donc pas
/// sur ce qui est un fichier valide.
///
/// Et la conséquence visible : pour `corrupt-entete.flac`, qui dure une seconde, le
/// conteneur annonce **plus de soixante-dix minutes**, et lofty le rapporte sans broncher.
/// Une telle piste entrerait dans la bibliothèque avec une durée absurde affichée à
/// l'utilisateur. Le contrôle d'intégrité — durée annoncée contre trames réellement rendues
/// — est INV-22, daté de la phase 5 ; ce test fixe la prémisse en attendant, exactement
/// comme les trois tests de `decodage.rs` fixent celle du décodeur. **Si une mise à jour de
/// lofty se mettait à refuser ces fichiers, ce test échouerait : cet échec serait la bonne
/// nouvelle.**
#[test]
fn les_fichiers_abimes_passent_le_lecteur_de_tags_sans_erreur() {
    let bac = Bac::avec("abimes", &["corrupt-entete.flac", "corrupt-trames.flac", "corrupt-tronque.flac"]);
    let mut depot = bac.depot();
    depot.ajouter_racine(&bac.dossier).expect("racine");
    let rapport = depot.scanner().expect("scan");

    assert_eq!(rapport.illisibles, 0, "aucun n'est refusé par lofty : {rapport:?}");
    assert_eq!(rapport.ajoutees, 3, "{rapport:?}");

    let entete = depot
        .piste(&bac.chemin("corrupt-entete.flac"))
        .expect("lecture")
        .expect("piste");
    assert!(
        entete.duree_ms > 3_600_000,
        "un en-tête abîmé annonce une durée absurde, mesurée à 4 275 789 ms : {} ms",
        entete.duree_ms
    );
}

/// Un fichier sans aucun tag entre dans la bibliothèque, avec son nom de fichier pour titre.
#[test]
fn un_fichier_sans_tag_a_son_nom_pour_titre() {
    let bac = Bac::avec("sans-tag", &["empty-tags.flac"]);
    let depot = bac.scanne();
    let pistes = depot.lister_pistes().expect("liste");
    assert_eq!(pistes.len(), 1);
    assert_eq!(pistes[0].titre, "empty-tags");
    assert_eq!(pistes[0].album, "", "pas d'album inventé");
    assert!(pistes[0].duree_ms > 100, "la durée vient du conteneur");
}

/// La base survit à sa fermeture : les migrations ne se rejouent pas et rien n'est perdu.
#[test]
fn la_base_se_reouvre_sans_perdre_la_bibliotheque() {
    let bac = Bac::avec("reouverture", &["tags-nfd.flac", "artist-feat.flac"]);
    {
        let _ = bac.scanne();
    }
    let depot = bac.depot();
    assert_eq!(depot.lister_pistes().expect("liste").len(), 2);
    assert_eq!(depot.racines().expect("racines").len(), 1, "la racine aussi est persistée");
}
