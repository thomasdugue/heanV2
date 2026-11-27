//! **La couture** de la lecture distante (ADR 0022), éprouvée sans NAS.
//!
//! Chaque pièce avait ses tests — le flux par blocs, la source mesurée, l'aller-retour des
//! URI — et la couture n'en avait aucun. C'est la faute exacte que `network::scanner_avec`
//! s'écrit à lui-même au sujet d'INV-27 : « la règle était éprouvée, mais son **câblage** ne
//! l'était pas […] un test qui saute une étape de la séquence réelle n'éprouve pas le
//! produit ». Relevé par `invariant-guardian` sur ce chantier même.
//!
//! Ce que ces tests prouvent et qu'aucun autre ne prouvait : **symphonia sait sonder et
//! décoder un vrai fichier audio servi par blocs de 1 Mio à travers notre `Seek`**. Sans
//! eux, rien ne disait que la fonctionnalité marche — ni avant que Thomas l'essaie, ni
//! après, puisqu'un échec chez lui n'aurait désigné aucune pièce.

use std::path::{Path, PathBuf};

use hean_lib::audio::source::SourceMesuree;
use hean_lib::audio::{decoder_entierement, Decodeur, Trames};
use hean_lib::network::client::{Entree, Erreur, Smb, Tranche};
use hean_lib::network::flux::FichierDistant;

fn fixture(nom: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/audio")).join(nom)
}

/// Un partage en mémoire, et il est **`Sync`** — c'est ce qui a décidé de sa forme plutôt que
/// de réutiliser le faux des tests de scan, qui compte ses appels dans un `RefCell`.
/// `MediaSource` exige `Send + Sync`, donc un compteur non partageable interdirait le
/// branchement complet : la contrainte n'est pas esthétique, elle est dans le trait.
struct PartageEnMemoire(Vec<u8>);

impl Smb for PartageEnMemoire {
    fn lister(&self, _chemin: &str) -> Result<Vec<Entree>, Erreur> {
        Ok(Vec::new())
    }

    fn lire(&self, _chemin: &str, max: Option<usize>) -> Result<Vec<u8>, Erreur> {
        let fin = max.unwrap_or(self.0.len()).min(self.0.len());
        Ok(self.0[..fin].to_vec())
    }

    fn tranche(&self, _chemin: &str, debut: u64, max: usize) -> Result<Tranche, Erreur> {
        let taille = self.0.len() as u64;
        let debut = usize::try_from(debut.min(taille)).unwrap();
        let fin = debut.saturating_add(max).min(self.0.len());
        Ok(Tranche { octets: self.0[debut..fin].to_vec(), taille })
    }
}

/// Décode la fixture **par le chemin de production** : partage → flux par blocs → source
/// mesurée → décodeur. L'URI est celle que porterait une vraie piste de la base.
fn par_le_partage(nom: &str) -> Trames {
    let octets = std::fs::read(fixture(nom)).expect("fixture lisible");
    let distant = FichierDistant::ouvrir(PartageEnMemoire(octets), "/musique/piste")
        .expect("ouverture du fichier distant");
    let taille = distant.taille();
    let uri = format!("smb://source/partage/musique/{nom}");
    let mut decodeur = Decodeur::ouvrir_flux(
        Box::new(SourceMesuree::nouvelle(distant, taille)),
        Path::new(&uri),
    )
    .expect("sondage du format à travers le partage");
    let format = decodeur.format();
    let mut tout = Vec::new();
    while let Some(bloc) = decodeur.suivant().expect("décodage") {
        tout.extend_from_slice(&bloc.echantillons);
    }
    Trames::nouveau(format, tout)
}

/// Le cœur : **mêmes octets, même résultat**, quelle que soit la porte d'entrée.
///
/// Les sept formats et non le seul FLAC, parce que les conteneurs ne sondent pas de la même
/// façon : un M4A lit sa table d'index à la fin du fichier, donc il se déplace loin en
/// arrière avant la première note — c'est précisément ce qu'un `Seek` par blocs peut casser,
/// et un test sur FLAC seul ne l'aurait jamais vu.
#[test]
fn les_sept_formats_se_decodent_a_lidentique_a_travers_le_partage() {
    for nom in [
        "flac-16-44.flac",
        "wav.wav",
        "aiff.aiff",
        "alac.m4a",
        "mp3-320.mp3",
        "aac.m4a",
        "vorbis.ogg",
    ] {
        let local = decoder_entierement(&fixture(nom)).expect(nom);
        let distant = par_le_partage(nom);
        assert_eq!(distant.format, local.format, "format de {nom}");
        assert_eq!(
            distant.echantillons.len(),
            local.echantillons.len(),
            "nombre d'échantillons de {nom}"
        );
        assert!(
            distant.echantillons == local.echantillons,
            "les échantillons de {nom} diffèrent selon la porte d'entrée"
        );
    }
}

/// Le **rechargement de bloc** — le seul endroit qui touche le réseau en cours de lecture.
///
/// Les fixtures pèsent moins d'un mégaoctet, donc tout ce qui précède tient dans le bloc lu à
/// l'ouverture : le rechargement n'y est jamais exécuté. Plutôt que de commiter un fichier
/// énorme pour ça, on sert le même par **petites tranches** — ce qu'un vrai partage fait de
/// toute façon, `libsmbclient` n'étant pas tenu de rendre tout ce qu'on lui demande. Le flux
/// de production, lui, n'est pas modifié : c'est bien `FichierDistant` qui recharge.
#[test]
fn un_fichier_servi_par_petites_tranches_se_decode_a_lidentique() {
    /// Rend au plus `tranche` octets par appel, et compte ses appels.
    struct PartageAvare {
        contenu: Vec<u8>,
        tranche: usize,
        appels: std::sync::atomic::AtomicUsize,
    }

    impl Smb for PartageAvare {
        fn lister(&self, _chemin: &str) -> Result<Vec<Entree>, Erreur> {
            Ok(Vec::new())
        }

        fn lire(&self, _chemin: &str, _max: Option<usize>) -> Result<Vec<u8>, Erreur> {
            Ok(self.contenu.clone())
        }

        fn tranche(&self, _chemin: &str, debut: u64, max: usize) -> Result<Tranche, Erreur> {
            self.appels.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let taille = self.contenu.len() as u64;
            let debut = usize::try_from(debut.min(taille)).unwrap();
            let fin = debut
                .saturating_add(max.min(self.tranche))
                .min(self.contenu.len());
            Ok(Tranche { octets: self.contenu[debut..fin].to_vec(), taille })
        }
    }

    /// Une poignée **locale** sur le partage, pour garder la main sur son compteur pendant que
    /// le flux le consomme. Deux contraintes ont décidé de cette forme, et aucune n'est un goût
    /// pour l'indirection : `Box<dyn MediaSource>` est `'static`, donc un emprunt ne peut pas y
    /// entrer ; et la règle d'orphelin interdit d'implémenter `Smb` directement sur un `Arc`,
    /// les deux types venant d'ailleurs.
    struct Poignee(std::sync::Arc<PartageAvare>);

    impl Smb for Poignee {
        fn lister(&self, chemin: &str) -> Result<Vec<Entree>, Erreur> {
            self.0.lister(chemin)
        }

        fn lire(&self, chemin: &str, max: Option<usize>) -> Result<Vec<u8>, Erreur> {
            self.0.lire(chemin, max)
        }

        fn tranche(&self, chemin: &str, debut: u64, max: usize) -> Result<Tranche, Erreur> {
            self.0.tranche(chemin, debut, max)
        }
    }

    let nom = "flac-24-192.flac";
    let octets = std::fs::read(fixture(nom)).expect("fixture lisible");
    let partage = std::sync::Arc::new(PartageAvare {
        contenu: octets.clone(),
        tranche: 16 * 1024,
        appels: std::sync::atomic::AtomicUsize::new(0),
    });
    let distant = FichierDistant::ouvrir(Poignee(std::sync::Arc::clone(&partage)), "/musique/piste")
        .expect("ouverture");
    let taille = distant.taille();
    assert_eq!(taille, octets.len() as u64, "la taille vient du partage, pas du bloc lu");

    let mut decodeur = Decodeur::ouvrir_flux(
        Box::new(SourceMesuree::nouvelle(distant, taille)),
        Path::new("smb://source/partage/musique/piste.flac"),
    )
    .expect("sondage à travers un partage avare");
    let format = decodeur.format();
    let mut tout = Vec::new();
    while let Some(bloc) = decodeur.suivant().expect("décodage") {
        tout.extend_from_slice(&bloc.echantillons);
    }

    let local = decoder_entierement(&fixture(nom)).expect(nom);
    assert_eq!(format, local.format);
    assert!(tout == local.echantillons, "les échantillons diffèrent après rechargements");
    // Le gage que le rechargement a bien eu lieu : sans lui, une seule ouverture suffirait et
    // ce test ne prouverait rien de plus que le précédent.
    let appels = partage.appels.load(std::sync::atomic::Ordering::Relaxed);
    assert!(appels > 5, "le rechargement de bloc n'a pas été exercé : {appels} appel(s)");
}
