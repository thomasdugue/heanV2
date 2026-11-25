//! La reprise du `config.json` de V1 vue depuis la séquence de démarrage : un fichier qu'on n'a
//! pas su lire ne prouve rien, et une reprise réussie ne se rejoue pas (INV-06 à travers la
//! migration).

use hean_lib::library::reprise_v1::config_de_v1;

use crate::bac::Bac;

/// Un `config.json` tronqué ne doit pas être pris pour « cet utilisateur n'avait aucune
/// exclusion ». V1 écrivait ses six fichiers JSON **sans écriture atomique** : un fichier
/// coupé en deux est le cas réel.
///
/// Avant la correction, le marqueur de reprise était posé quand même et la liste d'exclusion
/// était perdue **définitivement**, même en réparant le fichier.
#[test]
fn un_config_illisible_ne_marque_pas_la_reprise_comme_faite() {
    let bac = Bac::neuf("config-tronque");
    bac.poser("playlists.json", b"[]");
    // Un fichier coupé en plein milieu d'un tableau, comme V1 en laissait quand il était
    // tué pendant une sauvegarde. Composé avec la constante du dépôt, pas en clair.
    let config = bac.poser(
        "config.json",
        // Un vrai `config.json` coupé en son milieu, et non une forme inventée : c'est
        // `reprise_v1` qui sait ce que V1 écrivait, et le test se contente de le tronquer.
        {
            let entier = config_de_v1(&["/Musique"], &["/Musique/supprimee.flac"]);
            entier[..entier.len() / 2].to_string()
        }
        .as_bytes(),
    );

    // Le démarrage réussit — un config abîmé n'empêche pas d'utiliser l'application — mais
    // rien n'a été repris, et l'erreur est journalisée.
    let depot = bac.preparer().expect("l'application démarre");
    assert_eq!(depot.chemins_exclus().expect("exclus").len(), 0);
    assert_eq!(depot.racines().expect("racines").len(), 0);
    drop(depot);

    // Le fichier est réparé — restauré d'une sauvegarde, par exemple. La reprise doit
    // encore être possible : c'est ce que le marqueur posé à tort interdisait pour toujours.
    // Il est réparé **à son nouvel emplacement**, et c'est celui-là qui compte : la reprise lit
    // le `config.json` du dossier de données, pas celui de V1. L'ancien existe toujours — il
    // n'est plus supprimé (ADR 0014) — et porte encore sa version tronquée, ce qui ne gêne rien.
    assert!(config.exists(), "le dossier de V1 est conservé avec son config abîmé (INV-26)");
    std::fs::write(
        bac.nouveau().join("config.json"),
        config_de_v1(&["/Musique"], &["/Musique/supprimee.flac"]),
    )
    .expect("réparation");
    let depot = bac.preparer().expect("second démarrage");
    assert_eq!(
        depot.chemins_exclus().expect("exclus"),
        vec!["/Musique/supprimee.flac".to_string()],
        "la reprise a bien été retentée"
    );
    assert_eq!(depot.racines().expect("racines").len(), 1);
}

/// Et l'inverse, qui doit rester vrai : une reprise réussie ne se rejoue pas. La rejouer
/// ressusciterait les réglages changés depuis dans V2 — typiquement une piste que
/// l'utilisateur a cessé d'exclure.
///
/// La preuve se lit à l'envers, et sans une ligne de SQL : on change le `config.json` entre
/// les deux démarrages, et ce changement ne doit **pas** se retrouver en base. (Écrire ici
/// un `DELETE` sur la liste d'exclusion pour « dé-exclure » serait plus direct, et c'est
/// exactement ce qu'INV-06 interdit : un seul fichier du projet nomme cette table.)
#[test]
fn une_reprise_reussie_ne_se_rejoue_pas_au_demarrage_suivant() {
    let bac = Bac::neuf("pas-deux-fois");
    bac.poser_un_dossier_v1("/Musique/supprimee.flac");

    let depot = bac.preparer().expect("premier démarrage");
    assert_eq!(depot.chemins_exclus().expect("exclus").len(), 1);
    assert_eq!(depot.racines().expect("racines").len(), 1);
    drop(depot);

    // Le `config.json` migré est modifié après coup : une exclusion et une racine de plus.
    std::fs::write(
        bac.nouveau().join("config.json"),
        config_de_v1(
            &["/Musique", "/Ajoute/apres"],
            &["/Musique/supprimee.flac", "/Ajoute/apres.flac"],
        ),
    )
    .expect("config modifié");

    let depot = bac.preparer().expect("second démarrage");
    assert_eq!(
        depot.chemins_exclus().expect("exclus").len(),
        1,
        "la reprise ne doit pas rejouer : sinon elle écraserait ce que l'utilisateur a changé"
    );
    assert_eq!(depot.racines().expect("racines").len(), 1);
}
