//! La migration des données de V1, éprouvée sur des arbres de fichiers fabriqués.
//!
//! Ces tests ne remplacent PAS la vérification sur une copie réelle — l'ADR 0003 est
//! explicite : « elle se teste sur une copie réelle d'un dossier `noir/` complet, pas sur
//! des fixtures : c'est la seule façon de rencontrer les cas tordus ». Celle-là se lance
//! avec `tools/verifier-migration-reelle.sh`, et son résultat est consigné.
//!
//! Ce que ces tests-ci apportent, et que la copie réelle n'apporte pas : les cas de défaut.
//! Un dossier réel se migre une fois, correctement, et ne dit rien de ce qui se passe quand
//! la copie échoue au milieu — c'est-à-dire exactement l'instant où les données des
//! cinquante testeurs sont en jeu.

use std::path::{Path, PathBuf};

use hean_lib::library::reprise_v1::config_de_v1;
use hean_lib::migration::{self, Etat};

/// Un bac avec `noir/` dedans, prêt à migrer vers `hean/`.
struct Bac {
    base: PathBuf,
}

impl Bac {
    fn neuf(nom: &str) -> Bac {
        let base = std::env::temp_dir().join(format!("hean-migr-{nom}-{}", std::process::id()));
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

    /// Écrit un fichier sous `noir/`, en créant les dossiers au besoin.
    fn poser(&self, relatif: &str, contenu: &[u8]) -> PathBuf {
        let p = self.ancien().join(relatif);
        std::fs::create_dir_all(p.parent().expect("parent")).expect("dossiers");
        std::fs::write(&p, contenu).expect("écriture");
        p
    }

    /// Un `noir/` ressemblant à celui mesuré chez Thomas le 2026-11-22.
    fn poser_un_dossier_realiste(&self) {
        // Le `config.json` est composé avec les constantes du dépôt plutôt qu'avec les noms
        // de clés en clair : INV-06 veut qu'un seul fichier du backend nomme la liste
        // d'exclusion, et un détecteur qui se mettrait à signaler des données de test
        // finirait désarmé (voir `library/repository.rs`).
        self.poser("config.json", config_de_v1(&["/Musique"], &[]).as_bytes());
        self.poser("tracks_cache.json", b"[les pistes]");
        self.poser("metadata_cache.json", b"{les metadonnees}");
        self.poser("playlists.json", br#"[{"nom":"Jazz"}]"#);
        self.poser("listening_history.json", b"[mille ecoutes]");
        self.poser("network_sources.json", b"[le NAS]");
        self.poser("covers/pochette-1.jpg", b"des octets de pochette");
        self.poser("covers/pochette-2.jpg", b"d'autres octets");
        self.poser("thumbnails/abc_thumb.jpg", b"une vignette");
        self.poser("logs/noir.log.2026-11-21", b"des lignes de journal");
        self.poser("feedback/feedback_1777844906.json", b"{un retour}");
        // Le cache jetable, qui pesait 289 Mo sur 572 chez Thomas.
        self.poser("smb_buffer/2db60c33d8867478.flac", b"un gros fichier temporaire");
        self.poser("smb_buffer/44e020b003020598.flac", b"un autre");
    }
}

impl Drop for Bac {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn migrer(bac: &Bac) -> migration::Rapport {
    migration::migrer_si_necessaire(&bac.ancien(), &bac.nouveau()).expect("migration")
}

#[test]
fn un_dossier_v1_complet_se_migre_et_lancien_est_conserve() {
    let bac = Bac::neuf("complet");
    bac.poser_un_dossier_realiste();

    let rapport = migrer(&bac);
    assert_eq!(rapport.etat, Etat::Faite);
    assert_eq!(rapport.fichiers, 11, "{rapport:?}");
    assert_eq!(rapport.ignores, 2, "les deux fichiers de smb_buffer : {rapport:?}");

    // Ce qui compte pour un testeur : playlists, historique, sources NAS, pochettes.
    for attendu in [
        "playlists.json",
        "listening_history.json",
        "network_sources.json",
        "covers/pochette-1.jpg",
        "thumbnails/abc_thumb.jpg",
        "logs/noir.log.2026-11-21",
        "feedback/feedback_1777844906.json",
    ] {
        assert!(bac.nouveau().join(attendu).exists(), "{attendu} manque à l'arrivée");
    }
    assert_eq!(
        std::fs::read(bac.nouveau().join("playlists.json")).expect("lecture"),
        br#"[{"nom":"Jazz"}]"#.to_vec()
    );

    // Le cache jetable n'a PAS suivi — **et il n'est plus là non plus dans l'original.** C'est
    // la seconde décision de Thomas du 2026-11-25 : « Oui, jette le cache » (ADR 0014). Lui
    // seul : tout le reste de `noir/` est conservé, assertions juste en dessous.
    assert!(!bac.nouveau().join("smb_buffer").exists(), "smb_buffer ne se migre pas");
    assert!(!bac.ancien().join("smb_buffer").exists(), "et il est jeté de l'original");

    // **L'ancien dossier est TOUJOURS LÀ, et c'est la décision de Thomas** (ADR 0014,
    // INV-26 : « Oui, garde-le »). Cette assertion disait l'inverse jusqu'au 2026-11-25.
    // Le provisoire, lui, a bien disparu : c'est l'échange qui l'a consommé.
    assert!(bac.ancien().exists(), "le dossier de V1 n'est jamais supprimé (INV-26)");
    assert!(
        bac.ancien().join("playlists.json").exists(),
        "et il est intact, pas vidé : c'est tout l'objet du retour arrière"
    );
    // Les trois fichiers qui portent le mot « cache » sans en être : ils sont l'état de la
    // bibliothèque V1, donc ce qui rend le retour arrière utile. Thomas n'a pas arbitré ceux-là.
    assert!(bac.ancien().join("tracks_cache.json").exists(), "tracks_cache.json n'est pas un cache jetable");
    assert!(bac.ancien().join("metadata_cache.json").exists());
    assert!(!bac.base.join("hean.en-cours-de-migration").exists());
}

#[test]
fn le_temoin_dit_ce_qui_a_ete_migre() {
    let bac = Bac::neuf("temoin");
    bac.poser_un_dossier_realiste();
    migrer(&bac);

    let temoin = std::fs::read_to_string(bac.nouveau().join("migration-depuis-noir.json")).expect("témoin");
    assert!(temoin.contains("\"fichiers\": 11"), "{temoin}");
    assert!(temoin.contains("\"ignores\": 2"), "{temoin}");
    assert!(temoin.contains("noir"), "le témoin doit dire d'où ça vient : {temoin}");
}

#[test]
fn la_migration_est_idempotente() {
    let bac = Bac::neuf("idempotence");
    bac.poser_un_dossier_realiste();
    assert_eq!(migrer(&bac).etat, Etat::Faite);

    // **Deuxième lancement, avec le dossier de V1 TOUJOURS LÀ** — et c'est la question que
    // posait l'ADR 0014 : garder l'original ne doit pas faire remigrer 283 Mo à chaque
    // démarrage. Ce test l'établit au lieu de le relire : le seul juge est le témoin écrit dans
    // la cible, donc une source qui survit ne redéclenche rien. Avant le 2026-11-25 ce test
    // devait recréer `noir/` à la main pour poser la question ; maintenant elle se pose d'elle-même.
    assert!(bac.ancien().exists(), "la source a survécu au premier passage (INV-26)");
    // Et son contenu change entre les deux : si la migration repassait, elle écraserait.
    bac.poser("playlists.json", b"[une autre playlist]");
    assert_eq!(
        migrer(&bac).etat,
        Etat::DejaFaite,
        "ni Faite (remigration) ni CibleDejaPeuplee (témoin ignoré) : le témoin seul décide"
    );
    assert_eq!(
        std::fs::read(bac.nouveau().join("playlists.json")).expect("lecture"),
        br#"[{"nom":"Jazz"}]"#.to_vec(),
        "les données migrées ne doivent pas être écrasées"
    );
}

#[test]
fn sans_dossier_v1_il_ny_a_rien_a_migrer() {
    let bac = Bac::neuf("rien");
    // Une installation neuve : personne n'a jamais lancé V1.
    assert_eq!(migrer(&bac).etat, Etat::RienAMigrer);
    assert!(!bac.nouveau().exists(), "aucun dossier ne doit être créé pour rien");
}

#[test]
fn un_dossier_darrivee_deja_peuple_est_laisse_tranquille() {
    // Le cas qui ferait le plus de dégâts : deux bibliothèques, et une fusion sauvage.
    let bac = Bac::neuf("peuple");
    bac.poser_un_dossier_realiste();
    std::fs::create_dir_all(bac.nouveau()).expect("hean");
    std::fs::write(bac.nouveau().join("library.db"), b"une base deja la").expect("base");

    let rapport = migrer(&bac);
    assert_eq!(rapport.etat, Etat::CibleDejaPeuplee);
    assert_eq!(rapport.fichiers, 0);
    assert!(bac.ancien().exists(), "l'ancien dossier ne doit PAS être supprimé");
    assert_eq!(
        std::fs::read(bac.nouveau().join("library.db")).expect("lecture"),
        b"une base deja la".to_vec()
    );
}

#[test]
fn un_nom_de_fichier_en_nfd_survit_et_se_relit_sous_les_deux_ecritures() {
    // MESURE, et elle corrige ce que je croyais en écrivant ce test : sur **APFS, un nom en
    // NFD et le même nom en NFC désignent UN SEUL fichier**. Ma première version de ce test
    // posait les deux écritures de « Café Noir » en attendant deux fichiers ; la seconde
    // écriture avait simplement écrasé la première. Le système de fichiers compare les noms
    // après normalisation, tout en conservant les octets qu'on lui a donnés.
    //
    // Ce que ça change pour la migration : rien à faire de spécial, et c'est la bonne
    // nouvelle — le nom est recopié tel quel et se relit sous les deux écritures. Ce que ça
    // change pour le reste du projet : la normalisation NFC d'INV-01 porte sur les **clés
    // d'album** (comparées par nous, dans SQLite) et non sur les chemins (comparés par le
    // système). Deux problèmes qui se ressemblent, un seul des deux est le nôtre.
    //
    // L'assertion est donc une affirmation **sur APFS**, assumée : l'app est macOS seulement
    // (ADR 0002) et la CI est un seul job macOS. Sur un volume sensible à la normalisation —
    // un partage SMB, par exemple — les deux écritures seraient deux fichiers, et ce test
    // échouerait en le disant.
    let bac = Bac::neuf("nfd");
    let nfd = "covers/Cafe\u{0301} Noir.jpg"; // « Café » en e + accent combinant
    let nfc = "covers/Caf\u{00e9} Noir.jpg"; // « Café » en é précomposé
    bac.poser(nfd, b"la pochette de Cafe Noir");

    let rapport = migrer(&bac);
    assert_eq!(rapport.etat, Etat::Faite);
    assert_eq!(rapport.fichiers, 1, "{rapport:?}");
    for ecriture in [nfd, nfc] {
        assert_eq!(
            std::fs::read(bac.nouveau().join(ecriture)).unwrap_or_else(|e| panic!("{ecriture} : {e}")),
            b"la pochette de Cafe Noir".to_vec(),
            "le fichier doit se relire sous l'écriture {ecriture:?}"
        );
    }
}

#[test]
fn un_fichier_illisible_fait_tout_annuler_sans_rien_supprimer() {
    // LE test qui compte. Une copie qui échoue au milieu ne doit rien laisser à l'arrivée
    // et surtout rien supprimer au départ. La faute est injectée par les permissions :
    // c'est la façon la plus proche du réel de faire échouer une lecture.
    let bac = Bac::neuf("illisible");
    bac.poser_un_dossier_realiste();
    let interdit = bac.poser("covers/interdite.jpg", b"illisible");
    interdire_la_lecture(&interdit);

    let e = migration::migrer_si_necessaire(&bac.ancien(), &bac.nouveau())
        .expect_err("la migration doit échouer");
    assert!(format!("{e}").contains("interdite.jpg"), "{e}");

    assert!(bac.ancien().exists(), "l'ancien dossier doit être intact");
    assert!(
        bac.ancien().join("playlists.json").exists(),
        "les données de l'utilisateur doivent être intactes"
    );
    assert!(!bac.nouveau().exists(), "aucun dossier d'arrivée à moitié rempli");
    assert!(
        !bac.base.join("hean.en-cours-de-migration").exists(),
        "le provisoire doit être nettoyé"
    );

    // Et le lancement suivant peut retenter : rien n'est dans un état bloqué.
    autoriser_la_lecture(&interdit);
    assert_eq!(migrer(&bac).etat, Etat::Faite);
}

#[test]
fn un_provisoire_qui_traine_est_ecrase_et_non_repris() {
    // Le reste d'une tentative interrompue. Le reprendre serait prendre pour vérifiées des
    // données copiées on ne sait quand.
    let bac = Bac::neuf("provisoire");
    bac.poser_un_dossier_realiste();
    let provisoire = bac.base.join("hean.en-cours-de-migration");
    std::fs::create_dir_all(provisoire.join("covers")).expect("provisoire");
    std::fs::write(provisoire.join("playlists.json"), b"[une copie partielle]").expect("partiel");

    assert_eq!(migrer(&bac).etat, Etat::Faite);
    assert_eq!(
        std::fs::read(bac.nouveau().join("playlists.json")).expect("lecture"),
        br#"[{"nom":"Jazz"}]"#.to_vec(),
        "c'est la copie fraîche qui doit arriver, pas le reste de la précédente"
    );
}

#[test]
fn un_lien_symbolique_nest_pas_suivi() {
    // Un lien pourrait pointer hors du dossier de données : une migration ne copie que ce
    // qu'elle a vu.
    let bac = Bac::neuf("lien");
    bac.poser("playlists.json", b"[]");
    let dehors = bac.base.join("dehors.txt");
    std::fs::write(&dehors, b"je ne dois pas etre copie").expect("dehors");
    std::os::unix::fs::symlink(&dehors, bac.ancien().join("lien.txt")).expect("lien");

    let rapport = migrer(&bac);
    assert_eq!(rapport.fichiers, 1, "{rapport:?}");
    assert!(!bac.nouveau().join("lien.txt").exists());
    assert!(dehors.exists(), "le fichier visé par le lien ne doit pas être touché");
}

#[test]
fn un_dossier_v1_qui_ne_contient_que_du_cache_se_migre_quand_meme() {
    // Rien à garder, mais le témoin doit être écrit : sinon la migration serait retentée à
    // chaque lancement, pour rien. Et depuis l'ADR 0014, c'est le témoin SEUL qui l'empêche,
    // puisque l'ancien dossier reste là — d'où l'assertion ci-dessous, qui ne vérifie plus une
    // disparition mais la survie.
    let bac = Bac::neuf("que-du-cache");
    bac.poser("smb_buffer/un.flac", b"temporaire");
    let rapport = migrer(&bac);
    assert_eq!(rapport.etat, Etat::Faite);
    assert_eq!(rapport.fichiers, 0);
    assert_eq!(rapport.ignores, 1);
    assert!(bac.ancien().exists(), "le dossier de V1 reste, même vide de données (INV-26)");
    assert!(!bac.ancien().join("smb_buffer").exists(), "et son cache est jeté (ADR 0014)");
    assert!(bac.nouveau().join("migration-depuis-noir.json").exists());
}

/// **Un cache qui résiste ne doit pas faire échouer la migration.** Il coûte 289 Mio, pas une
/// donnée — et faire échouer la migration pour de la place disque serait exactement l'inverse de
/// ce que l'ADR 0012 a établi : ce qui n'est pas une perte ne doit pas empêcher de démarrer.
///
/// La panne est provoquée en retirant le droit d'écrire sur `noir/` : on peut encore lire son
/// contenu — donc le copier — mais plus y supprimer quoi que ce soit.
#[test]
fn un_cache_qui_ne_part_pas_ne_fait_pas_echouer_la_migration() {
    use std::os::unix::fs::PermissionsExt;
    let bac = Bac::neuf("cache-coince");
    bac.poser_un_dossier_realiste();

    let mut p = std::fs::metadata(bac.ancien()).expect("meta").permissions();
    p.set_mode(0o500);
    std::fs::set_permissions(bac.ancien(), p).expect("lecture seule");

    let rapport = migrer(&bac);

    // Rendre le droit d'écrire AVANT les assertions : sans ça, l'effacement du bac à la fin du
    // test échoue en silence et laisse des dossiers temporaires derrière lui.
    let mut p = std::fs::metadata(bac.ancien()).expect("meta").permissions();
    p.set_mode(0o700);
    std::fs::set_permissions(bac.ancien(), p).expect("droits rendus");

    assert_eq!(rapport.etat, Etat::Faite, "la migration réussit quand même : {rapport:?}");
    assert!(bac.nouveau().join("playlists.json").exists(), "et les données sont arrivées");
    assert!(bac.ancien().join("smb_buffer").exists(), "le cache est resté, c'est le scénario");
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
