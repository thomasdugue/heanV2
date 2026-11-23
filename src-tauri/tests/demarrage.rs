//! La séquence de démarrage : migrer, ouvrir la bibliothèque, reprendre la configuration.
//!
//! **Ces tests existent parce que les précédents ne voyaient pas ce qu'ils affirmaient.** La
//! migration était éprouvée en appelant `migrer_si_necessaire` deux fois de suite — ce que
//! `run()` ne fait jamais : entre les deux, il ouvre la bibliothèque, et c'est cette
//! ouverture qui créait le dossier de données et rendait l'échec définitif. Le test
//! concluait « le lancement suivant peut retenter » sur une séquence qui n'était pas celle du
//! produit.
//!
//! D'où la règle que tiennent les tests ci-dessous : **on rejoue la séquence entière**, celle
//! de `demarrage::preparer`, et jamais ses morceaux pris à part. Un test qui saute une étape
//! prouve quelque chose au sujet d'un programme qui n'existe pas.
//!
//! Règles couvertes : **INV-24** (rien n'écrit dans le dossier de données avant que la
//! migration ait rendu son verdict) et **INV-06** à travers la migration — une piste
//! supprimée dans V1 ne revient pas dans V2. Décisions : ADR 0012, qui remplace sur ce point
//! la conséquence acceptée de l'ADR 0010.

use std::path::{Path, PathBuf};

use hean_lib::demarrage;
use hean_lib::library::repository::{CLE_V1_EXCLUSIONS, CLE_V1_RACINES, MIGRATIONS};
use hean_lib::library::Depot;

/// Un bac avec `noir/` et `hean/` côte à côte, comme dans `Application Support`.
struct Bac {
    base: PathBuf,
}

impl Bac {
    fn neuf(nom: &str) -> Bac {
        let base = std::env::temp_dir().join(format!("hean-dem-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("bac");
        Bac { base }
    }

    fn ancien(&self) -> PathBuf {
        self.base.join("noir")
    }

    fn nouveau(&self) -> PathBuf {
        self.base.join("hean")
    }

    fn poser(&self, relatif: &str, contenu: &[u8]) -> PathBuf {
        let p = self.ancien().join(relatif);
        std::fs::create_dir_all(p.parent().expect("parent")).expect("dossiers");
        std::fs::write(&p, contenu).expect("écriture");
        p
    }

    /// Un `noir/` de testeur : des données, un dossier de musique et une piste supprimée.
    fn poser_un_dossier_v1(&self, exclusion: &str) {
        self.poser("playlists.json", br#"[{"nom":"Jazz"}]"#);
        self.poser("covers/pochette.jpg", b"des octets de pochette");
        self.poser(
            "config.json",
            config_v1("[\"/Musique\"]", &format!("[\"{exclusion}\"]")).as_bytes(),
        );
    }

    fn preparer(&self) -> Result<Depot, demarrage::Erreur> {
        demarrage::preparer(&self.ancien(), &self.nouveau())
    }
}

impl Drop for Bac {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

/// Un `config.json` de V1, composé sans écrire les noms de clés en clair (INV-06).
fn config_v1(racines: &str, exclusions: &str) -> String {
    format!(
        "{{\"{CLE_V1_RACINES}\":{racines},\"{CLE_V1_EXCLUSIONS}\":{exclusions},\"sentry_enabled\":null}}"
    )
}

fn interdire_la_lecture(fichier: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut p = std::fs::metadata(fichier).expect("meta").permissions();
    p.set_mode(0o000);
    std::fs::set_permissions(fichier, p).expect("permissions");
}

fn autoriser_la_lecture(fichier: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut p = std::fs::metadata(fichier).expect("meta").permissions();
    p.set_mode(0o644);
    std::fs::set_permissions(fichier, p).expect("permissions");
}

// =====================================================================================
// Le défaut qui a motivé ce fichier : un échec de migration devenait définitif
// =====================================================================================

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
    assert!(!bac.ancien().exists(), "l'ancien dossier a été supprimé");
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

// =====================================================================================
// Le schéma : une version livrée ne se modifie plus
// =====================================================================================

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

// =====================================================================================
// La reprise : un fichier qu'on n'a pas su lire ne prouve rien
// =====================================================================================

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
        format!("{{\"{CLE_V1_RACINES}\":[\"/Mus").as_bytes(),
    );

    // Le démarrage réussit — un config abîmé n'empêche pas d'utiliser l'application — mais
    // rien n'a été repris, et l'erreur est journalisée.
    let depot = bac.preparer().expect("l'application démarre");
    assert_eq!(depot.chemins_exclus().expect("exclus").len(), 0);
    assert_eq!(depot.racines().expect("racines").len(), 0);
    drop(depot);

    // Le fichier est réparé — restauré d'une sauvegarde, par exemple. La reprise doit
    // encore être possible : c'est ce que le marqueur posé à tort interdisait pour toujours.
    // Il est réparé à son nouvel emplacement : la migration a réussi, elle, et l'ancien
    // dossier n'existe plus.
    assert!(!config.exists(), "le dossier de V1 a été migré puis supprimé");
    std::fs::write(
        bac.nouveau().join("config.json"),
        config_v1("[\"/Musique\"]", "[\"/Musique/supprimee.flac\"]"),
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
        config_v1("[\"/Musique\",\"/Ajoute/apres\"]", "[\"/Musique/supprimee.flac\",\"/Ajoute/apres.flac\"]"),
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
