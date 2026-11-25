//! **Le format d'URI, de bout en bout.** Pas l'assemblage pris isolément — il a ses tests
//! unitaires dans `network::uri` — mais la chaîne entière : le `network_sources.json` réel de
//! V1, repris en base, scanné, et les chemins qui finissent dans la table des pistes.
//!
//! Pourquoi cette chaîne mérite son propre test : un maillon juste et un maillon faux
//! donnent une bibliothèque vide, et il n'y a aucun moyen de le voir dans un diff. Les
//! 3 137 pistes réseau de Thomas portent le préfixe
//! `smb://dcdd24af-ca15-48a4-adde-18b5fcfa92f0/music/LOSSLESS/complete/` dans le cache de
//! V1 ; ce que ce test vérifie, c'est que V2 écrit **le même**.
//!
//! Si ce test échoue un jour, la conséquence est à savoir avant de « corriger » quoi que ce
//! soit : sa bibliothèque réseau redeviendrait invisible, et un retour à V1 ne reconnaîtrait
//! pas non plus les chemins de V2.

use hean_lib::library::scan::{Conclusion, Perimetre};
use hean_lib::network::{inventaire, reprise_v1, uri};

use crate::commun::Bac;
use crate::faux_partage::FauxPartage;

/// Le `network_sources.json` **réel** de Thomas, relevé le 2026-11-25 dans
/// `~/Library/Application Support/hean/` après la migration depuis `noir/`.
const SOURCES_V1: &str = r#"[
  {
    "id": "dcdd24af-ca15-48a4-adde-18b5fcfa92f0",
    "name": "dugstation",
    "host": "dugstation.local",
    "share": "music",
    "remote_path": "/LOSSLESS/complete",
    "credentials": { "username": "admin", "domain": null, "is_guest": false },
    "enabled": true,
    "last_connected": null,
    "last_scan": null
  }
]"#;

/// Un préfixe relevé dans son `network_scan_cache.json` : **3 137 entrées sur 3 137** le
/// portent. C'est la cible de compatibilité, et elle est copiée, pas déduite.
const PREFIXE_REEL: &str = "smb://dcdd24af-ca15-48a4-adde-18b5fcfa92f0/music/LOSSLESS/complete/";

/// Un chemin de piste complet, copié d'une entrée réelle du cache de V1.
const PISTE_REELLE: &str = "smb://dcdd24af-ca15-48a4-adde-18b5fcfa92f0/music/LOSSLESS/complete/\
To Pimp a Butterfly (2015)/Kendrick Lamar - To Pimp a Butterfly - 16 - Mortal Man.mp3";

fn mp3() -> Vec<u8> {
    std::fs::read(crate::commun::fixture("mp3-320.mp3")).expect("la fixture doit exister")
}

/// Le faux NAS de Thomas : l'arborescence exacte qui produit `PISTE_REELLE`.
fn dugstation() -> FauxPartage {
    FauxPartage::nouveau()
        .dossier("/LOSSLESS/complete", &[("To Pimp a Butterfly (2015)", true)])
        .dossier(
            "/LOSSLESS/complete/To Pimp a Butterfly (2015)",
            &[("Kendrick Lamar - To Pimp a Butterfly - 16 - Mortal Man.mp3", false)],
        )
        .fichier(
            "/LOSSLESS/complete/To Pimp a Butterfly (2015)/\
             Kendrick Lamar - To Pimp a Butterfly - 16 - Mortal Man.mp3",
            mp3(),
        )
}

/// **Le test de bout en bout.** Reprise du JSON de V1, scan du faux partage, et le chemin
/// qui atterrit dans la base est celui que V1 avait écrit.
#[test]
fn un_scan_ecrit_exactement_les_chemins_que_v1_avait_ecrits() {
    let bac = Bac::avec("uri-bout-en-bout", &[]);
    let mut depot = bac.depot();

    let reprise = reprise_v1::reprendre(&mut depot, SOURCES_V1).expect("reprise");
    assert_eq!(reprise.sources, 1);
    assert!(!reprise.deja_faite);

    let source = depot
        .source_reseau("dcdd24af-ca15-48a4-adde-18b5fcfa92f0")
        .expect("lecture")
        .expect("la source de V1 est en base, avec SON identifiant");
    assert_eq!(source.nom, "dugstation");

    let perimetre = Perimetre::Source(uri::prefixe(&source.id));
    let partage = dugstation();
    let deja = inventaire::DejaConnu::depuis_le_depot(&depot, &perimetre).expect("état connu");
    let inv = inventaire::inventorier(&partage, &source, &deja).expect("inventaire");
    let rapport = depot
        .appliquer(&inv.entrees, &perimetre, Conclusion::depuis(inv.dossiers_illisibles))
        .expect("application");
    assert_eq!(rapport.ajoutees, 1, "{rapport:?}");

    let pistes = depot.lister_pistes().expect("liste");
    assert_eq!(pistes.len(), 1);
    assert_eq!(
        pistes[0].path, PISTE_REELLE,
        "le chemin écrit par V2 doit être celui que V1 avait écrit"
    );
    assert!(pistes[0].path.starts_with(PREFIXE_REEL));
    // Et les métadonnées sont celles que V2 lit dans le fichier, pas celles du cache de V1
    // (ADR 0016 §3) : le format vient de la table explicite, jamais d'un `{:?}` (INV-23).
    assert_eq!(pistes[0].format, "MP3");
}

/// La reprise ne se rejoue pas. Une seconde reprise ressusciterait une source que
/// l'utilisateur aurait supprimée depuis dans V2 — le même raisonnement que pour la liste
/// d'exclusion (INV-06).
#[test]
fn la_reprise_des_sources_ne_se_rejoue_pas() {
    let bac = Bac::avec("uri-reprise-une-fois", &[]);
    let mut depot = bac.depot();
    reprise_v1::reprendre(&mut depot, SOURCES_V1).expect("première reprise");

    let seconde = reprise_v1::reprendre(&mut depot, SOURCES_V1).expect("seconde reprise");
    assert!(seconde.deja_faite);
    assert_eq!(seconde.sources, 0);
    assert_eq!(depot.sources_reseau().expect("liste").len(), 1);
}

/// Un `network_sources.json` illisible **ne pose pas le marqueur** : la reprise se retentera
/// au démarrage suivant. V1 écrivait ses JSON sans écriture atomique, donc un fichier coupé
/// en deux est le cas réel — et conclure « aucune source réseau » cacherait 91 % de la
/// bibliothèque de Thomas pour toujours.
#[test]
fn un_json_tronque_ne_marque_pas_la_reprise_comme_faite() {
    let bac = Bac::avec("uri-reprise-illisible", &[]);
    let mut depot = bac.depot();

    let tronque = &SOURCES_V1[..SOURCES_V1.len() / 2];
    reprise_v1::reprendre(&mut depot, tronque).expect_err("un JSON tronqué est une erreur");
    assert!(depot.sources_reseau().expect("liste").is_empty());

    // Le démarrage suivant, avec le fichier entier cette fois : la reprise a bien lieu.
    let reprise = reprise_v1::reprendre(&mut depot, SOURCES_V1).expect("reprise");
    assert!(!reprise.deja_faite, "la reprise doit avoir été retentée");
    assert_eq!(reprise.sources, 1);
}

/// Une source ajoutée à la main — pas reprise de V1 — reçoit un identifiant **dérivé**, donc
/// l'ajouter deux fois ne crée pas deux sources. C'est ce qui évite une bibliothèque en
/// double après un geste qu'on fait par erreur plus souvent qu'à son tour.
#[test]
fn ajouter_deux_fois_le_meme_partage_ne_fait_quune_source() {
    let bac = Bac::avec("uri-ajout-deux-fois", &[]);
    let mut depot = bac.depot();

    let id = uri::identifiant_derive("mon-nas.local", "music", "/Albums");
    for _ in 0..2 {
        depot
            .enregistrer_source_reseau(&hean_lib::library::SourceReseau {
                id: id.clone(),
                nom: "mon-nas".into(),
                hote: "mon-nas.local".into(),
                partage: "music".into(),
                chemin_distant: "/Albums".into(),
                utilisateur: "moi".into(),
                domaine: None,
                invite: false,
                active: true,
            })
            .expect("enregistrement");
    }
    assert_eq!(depot.sources_reseau().expect("liste").len(), 1);
}
