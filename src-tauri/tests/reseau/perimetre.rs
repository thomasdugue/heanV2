//! **INV-27 — un scan ne conclut la disparition d'une piste que dans son périmètre.**
//!
//! La règle est née le 2026-11-25, en branchant la part réseau : `Depot::appliquer` efface
//! toute piste connue qu'il n'a pas revue, et un parcours de disque ne revoit évidemment
//! aucune piste SMB. Sans périmètre, le **premier scan de disque** de Thomas aurait effacé
//! ses 3 137 pistes réseau — 91 % de sa bibliothèque — et son premier scan réseau ses 305
//! pistes locales.
//!
//! Ces tests mélangent donc les deux provenances dans **une même base** et scannent chacune
//! à son tour. C'est la seule forme qui prouve quelque chose : un test qui ne mettrait qu'un
//! type de piste dans la base passerait au vert avec ou sans la règle.

use std::sync::Mutex;

use hean_lib::library::SourceReseau;
use hean_lib::network::uri;

use crate::commun::Bac;
use crate::faux_partage::FauxPartage;

fn source() -> SourceReseau {
    SourceReseau {
        id: "dcdd24af".into(),
        nom: "dugstation".into(),
        hote: "dugstation.local".into(),
        partage: "music".into(),
        chemin_distant: "/LOSSLESS".into(),
        utilisateur: "admin".into(),
        domaine: None,
        invite: false,
        active: true,
    }
}

fn flac() -> Vec<u8> {
    std::fs::read(crate::commun::fixture("flac-16-44.flac")).expect("la fixture doit exister")
}

/// Un faux NAS avec deux pistes, et rien d'autre.
fn nas() -> FauxPartage {
    FauxPartage::nouveau()
        .dossier("/LOSSLESS", &[("album", true)])
        .dossier("/LOSSLESS/album", &[("a.flac", false), ("b.flac", false)])
        .fichier("/LOSSLESS/album/a.flac", flac())
        .fichier("/LOSSLESS/album/b.flac", flac())
}

/// Scanne la source en appelant **la fonction de production**, `network::scanner_avec`.
///
/// ⚠️ **Cette fonction réassemblait les trois temps à la main jusqu'au 2026-11-25**, et
/// c'était le défaut que ces tests avaient en propre : les huit assertions ci-dessous
/// éprouvaient INV-27, mais pas son **câblage**. Quelqu'un qui aurait écrit
/// `Perimetre::Source(source_id.to_string())` — sans le slash final — ou `Conclusion::Complete`
/// en dur dans `network/mod.rs` aurait fait disparaître les pistes d'une source voisine, ou
/// celles d'un partage momentanément refusé, **et tous ces tests seraient restés verts**.
/// C'est mot pour mot la leçon d'INV-24 : un test qui saute une étape de la séquence réelle
/// n'éprouve pas le produit. Relevé par invariant-guardian.
///
/// Ce qui reste hors d'atteinte — et c'est la part irréductible — est `ClientSmb` lui-même :
/// le trousseau et `libsmbclient`. La fabrique injectée est exactement cette frontière.
fn scanner_le_nas(
    depot: &Mutex<hean_lib::library::Depot>,
    partage: &FauxPartage,
    source: &SourceReseau,
) -> hean_lib::library::RapportDeScan {
    hean_lib::network::scanner_avec(depot, &source.id, |_| Ok(Box::new(partage)))
        .expect("le scan doit aboutir")
}

/// Le dépôt derrière son verrou, comme l'application le tient (`Mutex<Depot>`).
fn verrouille(depot: hean_lib::library::Depot) -> Mutex<hean_lib::library::Depot> {
    Mutex::new(depot)
}

/// Lire la bibliothèque sans écrire : les assertions passent par là.
fn pistes(depot: &Mutex<hean_lib::library::Depot>) -> Vec<hean_lib::library::Track> {
    depot.lock().expect("verrou").lister_pistes().expect("liste")
}

/// Enregistre la source **dans la base**, parce que c'est ce que la production fait :
/// `scanner_avec` la relit par son identifiant et refuse de scanner ce qu'elle ne connaît
/// pas. Les tests passaient un `&SourceReseau` de côté jusqu'au 2026-11-25 et sautaient donc
/// cette étape.
fn enregistrer(depot: &Mutex<hean_lib::library::Depot>, source: &SourceReseau) {
    depot
        .lock()
        .expect("verrou")
        .enregistrer_source_reseau(source)
        .expect("source enregistrée");
}

fn scanner_le_disque(depot: &Mutex<hean_lib::library::Depot>) -> hean_lib::library::RapportDeScan {
    depot.lock().expect("verrou").scanner().expect("scan du disque")
}

/// **LE test d'INV-27.** Les deux provenances cohabitent, et chaque scan laisse l'autre
/// intacte — dans les deux ordres, parce qu'une règle qui ne tiendrait que dans un sens
/// n'en est pas une.
#[test]
fn un_scan_de_disque_nefface_pas_les_pistes_du_nas_et_reciproquement() {
    let bac = Bac::avec("inv27", &["tags-nfd.flac", "artist-feat.flac"]);
    let depot = verrouille(bac.scanne());
    enregistrer(&depot, &source());
    let locales = pistes(&depot).len();
    assert_eq!(locales, 2, "les deux fixtures locales sont en base");

    // Le NAS arrive.
    let rapport = scanner_le_nas(&depot, &nas(), &source());
    assert_eq!(rapport.ajoutees, 2, "deux pistes distantes ajoutées");
    assert_eq!(
        rapport.disparues, 0,
        "le scan réseau n'a pas à conclure la disparition d'une piste locale"
    );
    assert_eq!(pistes(&depot).len(), 4);

    // Puis un scan de disque, qui ne voit aucune piste SMB.
    let rapport = scanner_le_disque(&depot);
    assert_eq!(
        rapport.disparues, 0,
        "le scan de disque ne voit aucune piste du NAS : il ne doit pas les effacer"
    );
    assert_eq!(
        pistes(&depot).len(),
        4,
        "les 4 pistes sont toujours là — c'est INV-27"
    );

    // Et dans l'autre sens : un second scan réseau ne touche pas aux locales.
    let rapport = scanner_le_nas(&depot, &nas(), &source());
    assert!(rapport.sans_changement(), "{rapport:?}");
    assert_eq!(pistes(&depot).len(), 4);
}

/// Deux sources réseau ne se marchent pas dessus non plus. Le périmètre est le **préfixe**
/// de l'URI, slash final compris : sans ce slash, une source dont l'identifiant commence
/// comme celui d'une autre effacerait les pistes de sa voisine.
#[test]
fn deux_sources_reseau_ne_seffacent_pas_lune_lautre() {
    let bac = Bac::avec("inv27-deux-sources", &[]);
    let depot = verrouille(bac.depot());
    enregistrer(&depot, &source());

    let mut deuxieme = source();
    // Un identifiant qui COMMENCE comme le premier : c'est le cas qui casse si le préfixe
    // oublie son slash final.
    deuxieme.id = "dcdd24af-bis".into();
    deuxieme.nom = "autre-nas".into();
    enregistrer(&depot, &deuxieme);

    scanner_le_nas(&depot, &nas(), &source());
    scanner_le_nas(&depot, &nas(), &deuxieme);
    assert_eq!(pistes(&depot).len(), 4, "2 pistes × 2 sources");

    // Rescanner la première ne doit rien retirer à la seconde.
    let rapport = scanner_le_nas(&depot, &nas(), &source());
    assert_eq!(rapport.disparues, 0, "{rapport:?}");
    assert_eq!(pistes(&depot).len(), 4);

    // Et un NAS devenu vide n'efface QUE ses pistes.
    let vide = FauxPartage::nouveau().dossier("/LOSSLESS", &[]);
    let rapport = scanner_le_nas(&depot, &vide, &source());
    assert_eq!(rapport.disparues, 2, "les deux pistes de cette source, et pas plus");
    let restantes = pistes(&depot);
    assert_eq!(restantes.len(), 2);
    assert!(
        restantes.iter().all(|p| p.path.starts_with("smb://dcdd24af-bis/")),
        "seules les pistes de l'autre source restent : {:?}",
        restantes.iter().map(|p| &p.path).collect::<Vec<_>>()
    );
}

/// Une piste distante **exclue** ne revient pas au scan suivant, comme une locale (INV-06).
/// Le filtre d'exclusion ne connaît pas la provenance, et c'est le point : il n'y a qu'un
/// filtre, à un seul endroit — en V1 il était recopié, et `scan_network_source_cmd` était
/// justement l'un des trois endroits où il fallait penser à le remettre.
#[test]
fn une_piste_distante_exclue_ne_revient_pas() {
    let bac = Bac::avec("inv06-distant", &[]);
    let depot = verrouille(bac.depot());
    enregistrer(&depot, &source());
    scanner_le_nas(&depot, &nas(), &source());

    let a = uri::de_fichier("dcdd24af", "music", "/LOSSLESS/album/a.flac");
    depot.lock().expect("verrou").exclure(&a).expect("exclusion");
    assert_eq!(pistes(&depot).len(), 1);

    let rapport = scanner_le_nas(&depot, &nas(), &source());
    assert_eq!(rapport.ajoutees, 0, "le scan réseau ne réintroduit pas une piste exclue");
    assert_eq!(pistes(&depot).len(), 1);
    assert!(depot.lock().expect("verrou").piste(&a).expect("lecture").is_none());
}

/// Le scan différentiel d'une source distante : le deuxième scan ne **relit** rien. C'est ce
/// qui fait qu'un rescan de 3 137 pistes coûte le parcours des dossiers et rien de plus — et
/// une absence de lecture ne se prouve qu'en comptant les lectures.
#[test]
fn un_second_scan_reseau_ne_relit_aucun_fichier() {
    let bac = Bac::avec("differentiel-reseau", &[]);
    let depot = verrouille(bac.depot());
    enregistrer(&depot, &source());

    let premier = nas();
    scanner_le_nas(&depot, &premier, &source());
    assert_eq!(premier.lectures().len(), 2, "le premier scan lit les deux fichiers");

    let second = nas();
    let rapport = scanner_le_nas(&depot, &second, &source());
    assert!(rapport.sans_changement(), "{rapport:?}");
    assert!(
        second.lectures().is_empty(),
        "le second scan ne doit lire aucun fichier : {:?}",
        second.lectures()
    );
}

/// Et quand le serveur **donne** taille et date (`readdirplus`), le différentiel les utilise :
/// un fichier dont la date a changé est relu, les autres non. C'est le cas que V1 n'avait
/// jamais — ses 3 137 entrées de cache portent toutes `size: 0, modified: 0`.
#[test]
fn un_fichier_distant_modifie_est_relu_quand_le_serveur_donne_ses_stats() {
    let bac = Bac::avec("differentiel-stats", &[]);
    let depot = verrouille(bac.depot());
    enregistrer(&depot, &source());

    let avec_stats = |mtime: i64| {
        FauxPartage::nouveau()
            .dossier("/LOSSLESS", &[("album", true)])
            .dossier_avec_stats(
                "/LOSSLESS/album",
                &[("a.flac", false, 1_000, mtime), ("b.flac", false, 2_000, 42)],
            )
            .fichier("/LOSSLESS/album/a.flac", flac())
            .fichier("/LOSSLESS/album/b.flac", flac())
    };

    let premier = avec_stats(1);
    scanner_le_nas(&depot, &premier, &source());
    assert_eq!(premier.lectures().len(), 2);

    // `a.flac` a changé de date, `b.flac` non.
    let second = avec_stats(999);
    let rapport = scanner_le_nas(&depot, &second, &source());
    assert_eq!(rapport.modifiees, 1, "{rapport:?}");
    assert_eq!(rapport.ajoutees, 0);
    assert_eq!(
        second.lectures(),
        vec!["/LOSSLESS/album/a.flac"],
        "seul le fichier modifié est relu"
    );
}

/// La stratégie de lecture annoncée est bien celle qui arrive au partage : un FLAC demande
/// un en-tête de 1 Mio, un MP3 demande tout le fichier (ADR 0016 §3). Ce qui est éprouvé ici
/// n'est pas la table des formats — `src-tauri/tests/reseau/entete.rs` s'en charge — mais le fait que
/// l'inventaire la **consulte** au lieu de lire tout, toujours.
#[test]
fn linventaire_demande_un_entete_pour_un_flac_et_tout_pour_un_mp3() {
    let bac = Bac::avec("entete-demandee", &[]);
    let depot = verrouille(bac.depot());
    enregistrer(&depot, &source());
    let mp3 = std::fs::read(crate::commun::fixture("mp3-320.mp3")).expect("fixture");
    let partage = FauxPartage::nouveau()
        .dossier("/LOSSLESS", &[("album", true)])
        .dossier("/LOSSLESS/album", &[("a.flac", false), ("b.mp3", false)])
        .fichier("/LOSSLESS/album/a.flac", flac())
        .fichier("/LOSSLESS/album/b.mp3", mp3);

    scanner_le_nas(&depot, &partage, &source());
    assert_eq!(
        partage.tailles_demandees(),
        vec![Some(hean_lib::network::entete::TAILLE_ENTETE), None],
        "le FLAC se lit par son en-tête, le MP3 en entier"
    );
    assert_eq!(pistes(&depot).len(), 2);
}

/// Un dossier que le NAS refuse est compté et sauté, et **ses pistes ne disparaissent pas**.
///
/// C'est le test qui a fait naître `Conclusion` : écrit d'abord comme ci-dessous, il
/// échouait — les deux pistes du dossier refusé étaient effacées, parce que `appliquer`
/// conclut la disparition de tout ce qu'il n'a pas revu. « Je n'ai pas pu lire » et « il n'y
/// a plus rien » se confondaient, et c'est la perte que B17 décrit depuis V1.
#[test]
fn un_dossier_refuse_ne_fait_pas_disparaitre_ses_pistes() {
    let bac = Bac::avec("dossier-refuse", &[]);
    let depot = verrouille(bac.depot());
    enregistrer(&depot, &source());
    scanner_le_nas(&depot, &nas(), &source());
    assert_eq!(pistes(&depot).len(), 2);

    let refusant = FauxPartage::nouveau()
        .dossier("/LOSSLESS", &[("album", true)])
        .refuser("/LOSSLESS/album");
    let rapport = scanner_le_nas(&depot, &refusant, &source());
    assert_eq!(
        rapport.disparues, 0,
        "un dossier refusé n'est pas un dossier vide : {rapport:?}"
    );
    assert_eq!(
        pistes(&depot).len(),
        2,
        "les deux pistes sont toujours là"
    );

    // Et quand le dossier répond à nouveau en étant vraiment vide, la disparition se
    // conclut — sinon la protection empêcherait de voir une suppression pour toujours.
    let vide = FauxPartage::nouveau()
        .dossier("/LOSSLESS", &[("album", true)])
        .dossier("/LOSSLESS/album", &[]);
    let rapport = scanner_le_nas(&depot, &vide, &source());
    assert_eq!(rapport.disparues, 2, "{rapport:?}");
    assert!(pistes(&depot).is_empty());
}

/// Le même raisonnement sur le **disque**, parce que c'est le même code et le même défaut :
/// une racine de bibliothèque devenue illisible — disque externe éjecté, volume SMB démonté
/// — ne doit pas effacer ses pistes. Thomas a une seule racine de bibliothèque
/// (`docs/MESURES-V1.md` §1) : si elle devenait illisible une fois, l'ancien comportement
/// vidait la bibliothèque entière.
#[test]
fn une_racine_de_disque_devenue_illisible_nefface_pas_ses_pistes() {
    let bac = Bac::avec("racine-illisible", &["tags-nfd.flac", "artist-feat.flac"]);
    let depot = verrouille(bac.scanne());
    assert_eq!(pistes(&depot).len(), 2);

    // Une seconde racine qui n'existe pas : le parcours la saute et le dit.
    depot
        .lock()
        .expect("verrou")
        .ajouter_racine(std::path::Path::new("/dossier/qui/nexiste/pas"))
        .expect("racine");
    let rapport = scanner_le_disque(&depot);
    assert_eq!(rapport.disparues, 0, "{rapport:?}");
    assert_eq!(pistes(&depot).len(), 2);
}

/// **Le message qu'on lit quand le scan ne peut pas démarrer.** Il doit nommer les trois
/// causes possibles, parce que SMB ne dit pas laquelle.
///
/// C'est le test d'un message d'erreur, et il a sa raison : le 2026-11-25, la racine refusée
/// rendait « server returned a bad file descriptor » — ni « accès refusé », ni « chemin
/// absent ». Thomas a saisi son mot de passe **quatre fois** devant cette phrase. Un message
/// d'erreur est une fonctionnalité ; celle-ci s'éprouve comme les autres.
#[test]
fn une_racine_refusee_explique_quoi_verifier_et_dans_quel_ordre() {
    let bac = Bac::avec("message-racine", &[]);
    let depot = verrouille(bac.depot());
    enregistrer(&depot, &source());

    let refusant = FauxPartage::nouveau().refuser("/LOSSLESS");
    let e = hean_lib::network::scanner_avec(&depot, &source().id, |_| Ok(Box::new(&refusant)))
        .expect_err("une racine refusée est une erreur");
    let message = e.to_string();

    // Le NAS a répondu : il ne faut pas envoyer chercher du côté du réseau.
    assert!(message.contains("a répondu"), "{message}");
    // Les trois causes, et l'ordre dans lequel les vérifier.
    assert!(message.contains("mot de passe"), "{message}");
    assert!(message.contains("droit"), "{message}");
    assert!(message.contains("n'existe plus"), "{message}");
    // Et l'aveu : SMB ne distingue pas. Sans lui, on croit que l'app sait et ne dit pas.
    assert!(message.contains("SMB ne dit pas laquelle"), "{message}");
    // Le dossier et le partage sont nommés : « ça ne marche pas » ne se diagnostique pas.
    assert!(message.contains("/LOSSLESS"), "{message}");
    assert!(message.contains("music"), "{message}");
    assert!(message.contains("dugstation.local"), "{message}");
}
