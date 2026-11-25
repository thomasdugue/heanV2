//! **INV-26** — le dossier de V1 n'est jamais supprimé, et **INV-24** sur le cas qui autorise
//! une suppression : le verdict « cette base est vierge ».
//!
//! Les deux vivent ensemble parce que ce sont les deux seuls endroits de la séquence de démarrage
//! où quelque chose peut disparaître : l'ancien dossier (qui ne doit plus jamais disparaître) et
//! une base vide écartée pour laisser la migration se faire (qui doit disparaître, mais seulement
//! si elle ne contient réellement rien).

use hean_lib::library::Depot;

use crate::bac::Bac;

/// **La décision de Thomas, le 2026-11-25 : « Oui, garde-le »** (ADR 0014, qui remplace sur ce
/// point l'ADR 0010). Le dossier de V1 reste en place comme retour arrière vers la V1.
///
/// Pourquoi un test et pas une ligne de documentation : la suppression était là, elle rendait la
/// main plus propre, et un « nettoyage » de phase 3 la remettrait sans que personne ne la relise
/// comme une décision. C'est exactement ce qui est arrivé à `appelleInserer` la même nuit — un
/// détecteur sans assertion de matière réelle. Ici l'assertion est la matière même.
///
/// Les deux propriétés sont éprouvées **sur la séquence réelle de démarrage**, et deux fois de
/// suite, parce que la crainte légitime est qu'un original qui survit fasse remigrer 283 Mo à
/// chaque lancement : la source survit, et le second démarrage ne retouche à rien.
#[test]
fn le_dossier_de_v1_survit_au_demarrage_et_ne_fait_pas_remigrer() {
    let bac = Bac::neuf("inv26");
    bac.poser_un_dossier_v1("/Musique/supprimee.flac");

    let depot = bac.preparer().expect("premier démarrage");
    assert_eq!(depot.chemins_exclus().expect("exclus").len(), 1);
    drop(depot);

    // 1. Il est là, et il est intact — pas vidé, pas à moitié.
    assert!(bac.ancien().exists(), "le dossier de V1 n'est jamais supprimé");
    assert_eq!(
        std::fs::read(bac.ancien().join("playlists.json")).expect("playlists de V1"),
        br#"[{"nom":"Jazz"}]"#.to_vec(),
        "et son contenu est intact : c'est tout l'objet du retour arrière"
    );
    assert!(bac.ancien().join("covers/pochette.jpg").exists());

    // 2. Une marque posée dans le dossier de données après la migration, pour voir si le second
    //    démarrage la balaye. Un échange qui se rejouerait emporterait ce fichier avec lui.
    let marque = bac.nouveau().join("marque-du-premier-demarrage.txt");
    std::fs::write(&marque, b"ne doit pas disparaitre").expect("marque");

    let depot = bac.preparer().expect("second démarrage");
    assert!(marque.exists(), "le second démarrage n'a pas rejoué l'échange");
    assert!(bac.ancien().exists(), "et la source est toujours là");
    assert_eq!(
        depot.chemins_exclus().expect("exclus").len(),
        1,
        "la bibliothèque est inchangée (INV-06 : la piste supprimée dans V1 reste exclue)"
    );
}

/// **Le test qui empêche la prochaine table d'être effacée en silence.**
///
/// Le verdict « cette base est vierge » autorise sa suppression. Il était rendu, dans la
/// première version du correctif, en consultant trois listes nommées à la main : les pistes,
/// les racines, les exclusions. C'était juste à la date d'écriture — et faux le jour où la
/// phase 3 ajoute `playlists`, `favoris` ou `historique`, précisément les données que la
/// migration annonce protéger : une base contenant trois playlists et aucune piste aurait
/// été déclarée vide, donc **effacée**, sans qu'aucun test ne rougisse.
///
/// La table créée ici n'est dans aucune migration : elle tient le rôle de celle que la phase
/// suivante ajoutera. Si la vacuité redevenait une liste écrite à la main, ce test tombe.
#[test]
fn une_table_que_le_schema_ne_connait_pas_encore_empeche_le_verdict_vierge() {
    let bac = Bac::neuf("vacuite-derivee");
    bac.poser_un_dossier_v1("/Musique/supprimee.flac");
    let base = bac.nouveau().join("library.db");

    // Une base de V2 vierge, plus une table de la phase à venir, avec une ligne dedans.
    {
        let depot = Depot::ouvrir(&base).expect("base");
        assert!(depot.est_vierge().expect("vacuité"), "une base neuve est vierge");
    }
    {
        let conn = rusqlite::Connection::open(&base).expect("base");
        conn.execute_batch(
            "CREATE TABLE donnees_de_la_phase_suivante (x TEXT);
             INSERT INTO donnees_de_la_phase_suivante VALUES ('une playlist de Thomas');",
        )
        .expect("table de la phase suivante");
    }
    {
        let depot = Depot::ouvrir(&base).expect("base");
        assert!(
            !depot.est_vierge().expect("vacuité"),
            "une base qui contient quelque chose, où que ce soit, n'est pas vierge"
        );
    }

    // Donc la migration ne l'efface pas, et ne fusionne pas par-dessus.
    let depot = bac.preparer().expect("l'application démarre");
    drop(depot);
    assert!(base.exists(), "la base n'a pas été effacée");
    assert!(bac.ancien().join("playlists.json").exists(), "le dossier de V1 est intact");
    assert!(!bac.nouveau().join("playlists.json").exists(), "et rien n'a été migré");
}
