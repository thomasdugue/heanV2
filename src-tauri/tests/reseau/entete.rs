//! **La prémisse de l'ADR 0016 §3, mise à l'épreuve** : lire l'en-tête d'un fichier distant
//! au lieu du fichier entier donne-t-il les mêmes métadonnées ?
//!
//! La réponse dépend du format, et c'est tout l'enjeu : un en-tête tronqué donne un titre
//! juste et une **durée fausse** pour MP3, M4A et OGG, parce que leur durée se déduit de la
//! taille du fichier ou de sa dernière page. Une durée plausible et fausse est le pire mode
//! de défaut — elle ne se signale pas.
//!
//! Ces tests tronquent donc **chaque fixture du dépôt** et comparent champ par champ. Ils
//! rougiraient si `LISIBLES_EN_ENTETE` accueillait un format à tort, et aussi si une version
//! de lofty changeait de comportement — ce qui est exactement ce qu'on veut savoir, puisque
//! V1 avait dû écrire son propre parseur de blocs FLAC pour cette raison.

use hean_lib::library::metadonnees;
use hean_lib::network::entete;

use crate::commun::fixture;

/// Tous les fichiers audio du dossier de fixtures, sauf ceux qui sont abîmés exprès : un
/// fichier corrompu ne se lit pas plus entier que tronqué, donc il ne dit rien ici.
///
/// La liste est **dérivée du dossier** et non écrite à la main : une fixture ajoutée demain
/// entre dans ce test le jour où elle arrive.
fn fixtures_lisibles() -> Vec<String> {
    let dossier = fixture("");
    let mut out: Vec<String> = std::fs::read_dir(&dossier)
        .expect("le dossier de fixtures doit exister")
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter(|n| !n.starts_with("corrupt-") && !n.ends_with(".md"))
        .filter(|n| {
            hean_lib::library::scan::est_audio(std::path::Path::new(n))
        })
        .collect();
    out.sort();
    assert!(out.len() >= 10, "corpus trop petit : {out:?}");
    out
}

/// Une troncature **plus courte que le fichier**, pour que la question se pose vraiment. Les
/// fixtures font quelques kilo-octets, là où `entete::TAILLE_ENTETE` fait 1 Mio : les lire
/// avec la vraie taille ne tronquerait rien et ce test serait un décor.
fn tronque(octets: &[u8]) -> Vec<u8> {
    octets[..octets.len() / 2].to_vec()
}

/// **L'assertion centrale.** Pour chaque format autorisé à la lecture partielle, la moitié
/// du fichier rend exactement les mêmes champs que le fichier entier.
#[test]
fn les_formats_autorises_rendent_les_memes_metadonnees_tronques_ou_entiers() {
    let mut eprouves = 0;
    for nom in fixtures_lisibles() {
        if entete::a_lire(&nom).is_none() {
            continue; // ce format est lu en entier : rien à comparer
        }
        let chemin = fixture(&nom);
        let octets = std::fs::read(&chemin).expect("lecture de la fixture");
        let entier = metadonnees::lire_depuis(&octets, &nom).expect("le fichier entier se lit");
        let moitie = metadonnees::lire_depuis(&tronque(&octets), &nom);
        let Ok(moitie) = moitie else {
            // Un en-tête qui ne se parse pas n'est pas une faute : c'est le cas que le
            // repli sur le fichier entier couvre (ADR 0016). Ce qui serait une faute est un
            // en-tête qui se parse et rend autre chose.
            continue;
        };
        assert_eq!(moitie.track.titre, entier.track.titre, "{nom} — titre");
        assert_eq!(moitie.track.artiste, entier.track.artiste, "{nom} — artiste");
        assert_eq!(moitie.track.album, entier.track.album, "{nom} — album");
        assert_eq!(moitie.track.format, entier.track.format, "{nom} — format");
        assert_eq!(
            moitie.track.echantillonnage, entier.track.echantillonnage,
            "{nom} — échantillonnage"
        );
        assert_eq!(moitie.track.bits, entier.track.bits, "{nom} — bits");
        // La durée, le champ qui trahit. S'il diffère, ce format n'a rien à faire dans
        // `LISIBLES_EN_ENTETE` : voir le tableau en tête de `network::entete`.
        assert_eq!(
            moitie.track.duree_ms, entier.track.duree_ms,
            "{nom} — DURÉE : ce format ne peut pas être lu partiellement"
        );
        eprouves += 1;
    }
    assert!(
        eprouves >= 3,
        "seules {eprouves} fixtures ont éprouvé la lecture partielle — le corpus ne couvre \
         plus les formats autorisés, et ce test ne garantit plus rien"
    );
}

/// **La contre-épreuve, et c'est elle qui donne du mordant à la précédente.** Au moins un
/// format exclu de la liste rend bien une durée **différente** quand on le tronque. Sans
/// cette assertion, `LISIBLES_EN_ENTETE` pourrait contenir tous les formats sans qu'aucun
/// test ne rougisse : la liste serait une décoration.
#[test]
fn au_moins_un_format_exclu_rend_vraiment_une_duree_fausse_tronque()
{
    let mut trahisons = Vec::new();
    for nom in fixtures_lisibles() {
        if entete::a_lire(&nom).is_some() {
            continue; // ce format est autorisé : il est éprouvé par le test précédent
        }
        let octets = std::fs::read(fixture(&nom)).expect("lecture");
        let entier = metadonnees::lire_depuis(&octets, &nom).expect("entier");
        if let Ok(moitie) = metadonnees::lire_depuis(&tronque(&octets), &nom) {
            if moitie.track.duree_ms != entier.track.duree_ms {
                trahisons.push(format!(
                    "{nom} : {} ms tronqué contre {} ms entier",
                    moitie.track.duree_ms, entier.track.duree_ms
                ));
            }
        }
    }
    assert!(
        !trahisons.is_empty(),
        "aucun format exclu ne trahit sa durée à la troncature : soit lofty a changé, soit \
         la liste des formats lus en entier n'a plus de raison d'être. Les deux demandent \
         de relire l'ADR 0016 §3."
    );
}

/// Et la stratégie annoncée est bien celle appliquée : un FLAC demande un en-tête, un MP3
/// demande tout le fichier. C'est la couture entre `entete::a_lire` et le client SMB.
#[test]
fn la_taille_demandee_depend_du_format_et_de_rien_dautre() {
    assert_eq!(entete::a_lire("/x/a.flac"), Some(entete::TAILLE_ENTETE));
    assert_eq!(entete::a_lire("/x/a.mp3"), None);
    // Le chemin n'intervient pas : seul le format décide. Un dossier nommé `.mp3` ne doit
    // pas changer la stratégie de lecture d'un FLAC qu'il contient.
    assert_eq!(entete::a_lire("/x/album.mp3/a.flac"), Some(entete::TAILLE_ENTETE));
}
