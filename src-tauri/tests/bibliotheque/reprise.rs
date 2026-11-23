//! La reprise de la configuration de V1 — INV-06 à travers la migration.
//!
//! Ce module existe parce que la bibliothèque a une histoire plus vieille qu'elle : les
//! dossiers de musique et la liste des pistes supprimées vivaient dans le `config.json` de
//! V1, et V2 ne lit aucun JSON de V1. Ce que ces tests gardent et que
//! `src-tauri/tests/demarrage.rs` ne couvre pas : la reprise confrontée à un **vrai scan de fichiers
//! audio**, donc la preuve de bout en bout qu'une piste exclue dans V1 ne réapparaît pas.
//!
//! Un module du thème, comme l'en-tête de `main.rs` le demande, et le bac à sable est celui
//! de `../commun/` : mes tests avaient d'abord recopié ces huit lignes de bac, ce qui aurait
//! fait deux dossiers temporaires à maintenir pour la même chose.

use hean_lib::library::repository::{CLE_V1_EXCLUSIONS, CLE_V1_RACINES};
use hean_lib::library::reprise_v1;

use crate::commun::Bac;

/// Un `config.json` de V1, composé sans écrire les noms de clés en clair (INV-06).
fn config_v1(racines: &[&str], exclusions: &[&str]) -> String {
    let tableau =
        |v: &[&str]| format!("[{}]", v.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(","));
    format!(
        "{{\"{CLE_V1_RACINES}\":{},\"{CLE_V1_EXCLUSIONS}\":{},\"sentry_enabled\":null}}",
        tableau(racines),
        tableau(exclusions)
    )
}

/// **Une piste supprimée dans V1 ne revient pas dans V2.**
///
/// C'est INV-06 (🔴) à travers la migration, et c'est le défaut que déplacer les fichiers
/// ne corrige pas : V2 ne lit aucun JSON de V1, donc sans reprise la liste d'exclusion
/// serait perdue et la piste réapparaîtrait au premier scan. Le testeur verrait revenir ce
/// qu'il avait supprimé, sans comprendre pourquoi.
#[test]
fn une_piste_exclue_dans_v1_reste_exclue_apres_la_reprise() {
    let bac = Bac::avec("reprise", &["tags-nfd.flac", "artist-feat.flac"]);
    let exclue = bac.chemin("tags-nfd.flac");
    let mut depot = bac.depot();

    let reprise = reprise_v1::reprendre(
        &mut depot,
        &config_v1(&[&bac.dossier.display().to_string()], &[&exclue]),
    )
    .expect("reprise");
    assert_eq!(reprise.racines, 1, "{reprise:?}");
    assert_eq!(reprise.exclusions, 1, "{reprise:?}");

    // Le dossier de musique de V1 est devenu une racine : la bibliothèque se rescanne
    // toute seule, le testeur n'a rien à reconfigurer.
    assert_eq!(depot.racines().expect("racines").len(), 1);
    let rapport = depot.scanner().expect("scan");
    assert_eq!(rapport.ajoutees, 1, "la piste exclue ne doit pas entrer : {rapport:?}");

    let pistes = depot.lister_pistes().expect("liste");
    assert_eq!(pistes.len(), 1);
    assert!(!pistes.iter().any(|p| p.path == exclue), "la piste exclue est revenue");
}

/// Une reprise déjà faite ne se rejoue pas : elle ressusciterait les réglages que
/// l'utilisateur a changés depuis dans V2.
#[test]
fn la_reprise_ne_se_rejoue_pas() {
    let bac = Bac::avec("reprise-idempotence", &["tags-nfd.flac"]);
    let chemin = bac.chemin("tags-nfd.flac");
    let config = config_v1(&[&bac.dossier.display().to_string()], &[&chemin]);
    let mut depot = bac.depot();

    assert!(!reprise_v1::reprendre(&mut depot, &config).expect("1re").deja_faite);
    assert_eq!(depot.chemins_exclus().expect("exclus").len(), 1);

    let seconde = reprise_v1::reprendre(&mut depot, &config).expect("2e");
    assert!(seconde.deja_faite, "{seconde:?}");
    assert_eq!(seconde.exclusions, 0, "rien n'est réinséré : {seconde:?}");
}

/// **Un `config.json` illisible est une erreur, et le marqueur n'est PAS posé.**
///
/// Ce test disait l'inverse jusqu'au 2026-11-23 — il affirmait, avec sa justification, que
/// « le marqueur est posé quand même : on ne retentera pas éternellement de lire un fichier
/// abîmé ». L'échange était mauvais : il troquait un journal bruyant contre une violation
/// silencieuse et **définitive** d'INV-06. V1 écrivait ses six fichiers JSON sans écriture
/// atomique, donc un `config.json` tronqué est le cas réel ; en conclure « cet utilisateur
/// n'avait aucune exclusion » fait revenir ses pistes supprimées, pour toujours.
///
/// Un test porté verrouille autant un bug qu'une règle : celui-ci aurait fait respecter le
/// défaut par le prochain lecteur. Le parcours complet est dans `src-tauri/tests/demarrage.rs`.
#[test]
fn un_config_v1_illisible_est_une_erreur_et_ne_marque_rien() {
    let bac = Bac::avec("reprise-illisible", &["tags-nfd.flac"]);
    let mut depot = bac.depot();

    let e = reprise_v1::reprendre(&mut depot, "{ ceci n'est pas du json")
        .expect_err("un JSON illisible doit remonter une erreur");
    assert!(matches!(e, reprise_v1::Erreur::ConfigIllisible { .. }), "{e}");

    // Rien n'a été repris, et surtout : la reprise reste à faire. Un fichier réparé plus
    // tard — restauré d'une sauvegarde — sera donc bien pris en compte.
    assert_eq!(depot.chemins_exclus().expect("exclus").len(), 0);
    let chemin = bac.chemin("tags-nfd.flac");
    let reprise = reprise_v1::reprendre(
        &mut depot,
        &config_v1(&[&bac.dossier.display().to_string()], &[&chemin]),
    )
    .expect("après réparation");
    assert!(!reprise.deja_faite, "la reprise ne doit pas avoir été marquée : {reprise:?}");
    assert_eq!(reprise.exclusions, 1);
}
