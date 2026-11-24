//! Le bac à sable partagé par les tests d'intégration de la bibliothèque.
//!
//! Ce dossier n'est PAS une cible de test : Cargo ne compile en cible que `tests/*.rs` et
//! `tests/*/main.rs`, jamais `tests/*/mod.rs`. Chaque cible qui en a besoin le déclare avec
//! `#[path = "../commun/mod.rs"] mod commun;`.
//!
//! Les fixtures sont **copiées dans un dossier temporaire** avant toute écriture. Écrire
//! dans `fixtures/audio/` abîmerait un actif commité et versionné — et ferait échouer les
//! tests de bit-perfect de la phase 1 pour une raison fausse.

use std::path::PathBuf;

use hean_lib::library::Depot;

pub fn fixture(nom: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/audio")).join(nom)
}

/// Un dossier temporaire à soi, avec les fixtures demandées recopiées dedans.
///
/// Sans `tempfile` : la dépendance existe (elle vient de proptest) mais une fonction de
/// huit lignes évite d'en faire une dépendance directe, et le nettoyage explicite dit ce
/// qu'il nettoie.
pub struct Bac {
    pub dossier: PathBuf,
}

impl Bac {
    pub fn avec(nom_du_test: &str, fixtures: &[&str]) -> Bac {
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

    pub fn chemin(&self, nom: &str) -> String {
        self.dossier.join(nom).display().to_string()
    }

    /// Un dépôt dont la base est un FICHIER du bac, pas une base en mémoire : c'est la
    /// seule façon de prouver qu'une donnée survit à la fermeture de l'application (V015).
    pub fn depot(&self) -> Depot {
        Depot::ouvrir(&self.dossier.join("library.db")).expect("ouverture du dépôt")
    }

    pub fn scanne(&self) -> Depot {
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
