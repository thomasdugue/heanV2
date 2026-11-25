//! Le schéma de la base, et la règle qu'une version livrée ne se modifie plus : la table des
//! reprises avait été ajoutée dans la version 1 déjà sur `main`, donc elle n'apparaissait jamais
//! dans une base existante (INV-24, défaut du 2026-11-23).

use hean_lib::library::repository::MIGRATIONS;

use crate::bac::Bac;

/// La table `reprises` avait été ajoutée dans la migration **version 1**, déjà livrée. Le
/// mécanisme ne rejoue jamais une version appliquée, donc la table n'apparaissait jamais dans
/// une base existante : la reprise échouait à chaque démarrage sur « no such table », et les
/// exclusions de V1 n'étaient jamais reprises.
///
/// Ce test reconstitue une base au schéma de la version 1 — en lisant le SQL de la migration
/// elle-même, pour qu'il suive si elle change — et vérifie que l'ouverture la rattrape.
#[test]
fn une_base_au_schema_de_la_version_1_recoit_la_table_des_reprises() {
    let bac = Bac::neuf("schema-v1");
    bac.poser_un_dossier_v1("/Musique/supprimee.flac");
    let base = bac.nouveau().join("library.db");
    std::fs::create_dir_all(bac.nouveau()).expect("hean");

    let premiere = MIGRATIONS.first().expect("au moins une migration");
    assert_eq!(premiere.version, 1, "la version 1 est la base de ce scénario");
    {
        let conn = rusqlite::Connection::open(&base).expect("base");
        conn.execute_batch(premiere.sql).expect("schéma de la version 1");
        conn.execute_batch("PRAGMA user_version = 1").expect("version");
    }
    assert!(
        MIGRATIONS.len() >= 2,
        "la table des reprises doit vivre dans une migration qui lui est propre"
    );

    // La base est vierge : elle est écartée, la migration se fait, et la reprise fonctionne —
    // ce qui n'était possible qu'avec une table créée par une migration postérieure.
    let depot = bac.preparer().expect("le démarrage doit réussir");
    assert_eq!(
        depot.chemins_exclus().expect("exclus"),
        vec!["/Musique/supprimee.flac".to_string()],
        "la reprise a eu lieu, donc la table existait"
    );
}

/// Et la version 1 ne doit plus jamais changer : une base déjà livrée ne la rejouera pas.
#[test]
fn la_version_1_ne_contient_pas_la_table_des_reprises() {
    let premiere = MIGRATIONS.first().expect("une migration");
    assert!(
        !premiere.sql.contains("reprises"),
        "ajoutée à la version 1 après coup, cette table n'apparaîtrait jamais dans une base \
         existante — c'est le défaut du 2026-11-23"
    );
    assert!(
        MIGRATIONS.iter().any(|m| m.version == 2 && m.sql.contains("reprises")),
        "elle appartient à la version 2"
    );
}
