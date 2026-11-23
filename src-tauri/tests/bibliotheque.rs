//! Ce que la bibliothèque garantit, prouvé sur de vrais fichiers.
//!
//! Ces tests ne vérifient pas « le code fait ce qu'il dit » : ils rejouent les régressions
//! qui ont coûté le plus cher en V1, sur les fixtures faites pour elles.
//!
//! Les fixtures sont **copiées dans un dossier temporaire** avant toute écriture. Écrire
//! dans `fixtures/audio/` abîmerait un actif commité et versionné — et ferait échouer les
//! tests de bit-perfect de la phase 1 pour une raison fausse.

use std::path::{Path, PathBuf};

use hean_lib::library::metadonnees::{lire, nom_de_format};
use hean_lib::library::{Depot, Edition};
use lofty::file::FileType;

fn fixture(nom: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/audio")).join(nom)
}

/// Un dossier temporaire à soi, avec les fixtures demandées recopiées dedans.
///
/// Sans `tempfile` : la dépendance existe (elle vient de proptest) mais une fonction de
/// huit lignes évite d'en faire une dépendance directe, et le nettoyage explicite dit ce
/// qu'il nettoie.
struct Bac {
    dossier: PathBuf,
}

impl Bac {
    fn avec(nom_du_test: &str, fixtures: &[&str]) -> Bac {
        let dossier = std::env::temp_dir().join(format!("hean-test-{nom_du_test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).expect("dossier temporaire");
        for nom in fixtures {
            // Le nom de fichier est conservé : `accents-éàü.flac` doit continuer à
            // s'ouvrir, et c'est précisément ce que cette fixture existe pour éprouver.
            std::fs::copy(fixture(nom), dossier.join(nom)).unwrap_or_else(|e| panic!("copie de {nom} : {e}"));
        }
        Bac { dossier }
    }

    fn chemin(&self, nom: &str) -> String {
        self.dossier.join(nom).display().to_string()
    }

    /// Un dépôt dont la base est un FICHIER du bac, pas une base en mémoire : c'est la
    /// seule façon de prouver qu'une donnée survit à la fermeture de l'application (V015).
    fn depot(&self) -> Depot {
        Depot::ouvrir(&self.dossier.join("library.db")).expect("ouverture du dépôt")
    }

    fn scanne(&self) -> Depot {
        let mut d = self.depot();
        d.ajouter_racine(&self.dossier).expect("racine");
        d.scanner().expect("scan");
        d
    }
}

impl Drop for Bac {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dossier);
    }
}

// =====================================================================================
// INV-01 — un album = une seule entrée
// =====================================================================================

/// La régression la plus coûteuse de V1, sur les deux fixtures faites pour elle.
///
/// `tags-nfd.flac` porte l'album « Café Noir » en **NFD** et `artist-feat.flac` le même
/// album en NFC, avec un artiste « Zoé Léger feat. Hean ». V1 produisait deux albums : une
/// fois par la normalisation manquante, une fois par la déduplication sur `artiste + album`
/// (régression W003). Ici la clé vient du type `AlbumKey` et ne prend que l'album.
#[test]
fn deux_ecritures_du_meme_album_ne_font_quune_entree() {
    let bac = Bac::avec("inv01", &["tags-nfd.flac", "artist-feat.flac"]);
    let depot = bac.scanne();
    let pistes = depot.lister_pistes().expect("liste");

    assert_eq!(pistes.len(), 2, "les deux pistes doivent être dans la bibliothèque");
    let cles: std::collections::BTreeSet<_> = pistes.iter().map(|p| p.cle_album.as_str()).collect();
    assert_eq!(cles.len(), 1, "un seul album attendu, trouvé : {cles:?}");

    // Et les artistes DIFFÈRENT bien : sans quoi le test passerait pour la mauvaise raison.
    let artistes: std::collections::BTreeSet<_> = pistes.iter().map(|p| p.artiste.as_str()).collect();
    assert_eq!(artistes.len(), 2, "les deux pistes doivent avoir des artistes distincts");
}

/// N pistes d'un même album produisent exactement 1 entrée, quels que soient leurs
/// artistes et quel que soit l'ordre dans lequel on les scanne (HARNAIS.md §4).
#[test]
fn le_nombre_dalbums_ne_depend_pas_de_lordre_du_scan() {
    for ordre in [["tags-nfd.flac", "artist-feat.flac"], ["artist-feat.flac", "tags-nfd.flac"]] {
        let bac = Bac::avec("inv01-ordre", &ordre);
        let depot = bac.scanne();
        let cles: std::collections::BTreeSet<_> = depot
            .lister_pistes()
            .expect("liste")
            .iter()
            .map(|p| p.cle_album.clone())
            .collect();
        assert_eq!(cles.len(), 1, "ordre {ordre:?}");
    }
}

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

// =====================================================================================
// INV-23 — un libellé destiné à l'affichage ne vient jamais d'un `{:?}`
// =====================================================================================

/// Chaque fixture rend le nom que la V1 affichait, sur les sept formats supportés.
///
/// Le défaut que ce test rejoue : `lire` remplissait `format` avec
/// `format!("{:?}", fichier.file_type())`, c'est-à-dire le `Debug` d'un type de lofty
/// envoyé vers l'affichage. Les 40 MP3 de la bibliothèque de Thomas annonçaient `Mpeg`, et
/// les deux `.m4a` annonçaient tous les deux `Mp4` — ALAC et AAC confondus. C'était le seul
/// des 182 écarts de la passe de vérité du 2026-11-23 où la V1 avait raison.
///
/// Remettre le `{:?}` fait rougir six des sept lignes ci-dessous. Et si une mise à jour de
/// lofty renomme un variant, c'est ici que ça se voit, pas chez un testeur.
#[test]
fn chaque_format_supporte_rend_son_nom_daffichage() {
    let attendu = [
        ("flac-16-44.flac", "FLAC"),
        ("mp3-320.mp3", "MP3"),
        ("wav.wav", "WAV"),
        ("aiff.aiff", "AIFF"),
        // Même conteneur MP4, deux codecs : c'est la distinction que le `Debug` écrasait.
        ("alac.m4a", "ALAC"),
        ("aac.m4a", "AAC"),
        ("vorbis.ogg", "Vorbis"),
    ];

    for (nom, format) in attendu {
        // Lecture seule : aucune raison de recopier la fixture, rien n'écrit ici.
        let lues = lire(&fixture(nom)).unwrap_or_else(|e| panic!("{nom} : {e}"));
        assert_eq!(lues.track.format, format, "{nom}");
    }
}

/// Les sept formats supportés rendent sept noms distincts — et le test interroge la
/// fonction, il ne recopie pas sa table.
///
/// Première écriture de ce test : il dédupliquait un **tableau littéral** et comparait sa
/// longueur à elle-même. Vert quoi que fasse le code de production — « de la confiance sans
/// en mériter », la famille exacte du détecteur d'INV-06 qui cherchait `\bTrack\b` là où
/// specta émet `Track_Serialize[]` : zéro trouvaille sur huit commandes, auto-test vert
/// pendant tout ce temps. Relevé par `invariant-guardian` sur ce diff même.
///
/// Les sept formats, c'est bien sept **appels** : le conteneur MP4 compte pour deux, selon
/// qu'il porte de l'ALAC ou de l'AAC. Un nom en double y rougit maintenant, et un huitième
/// format qui se replierait sur un nom existant aussi.
#[test]
fn les_sept_formats_rendent_sept_noms_distincts() {
    let sept: [(FileType, Option<u8>); 7] = [
        (FileType::Flac, None),
        (FileType::Mpeg, None),
        (FileType::Wav, None),
        (FileType::Aiff, None),
        (FileType::Vorbis, None),
        (FileType::Mp4, Some(16)),
        (FileType::Mp4, None),
    ];

    let noms: Vec<&str> = sept
        .iter()
        .map(|(t, bits)| nom_de_format(*t, *bits).unwrap_or_else(|| panic!("{t:?} n'a pas de nom")))
        .collect();

    let mut distincts = noms.clone();
    distincts.sort_unstable();
    distincts.dedup();
    assert_eq!(distincts.len(), 7, "deux formats affichent le même nom : {noms:?}");
}
