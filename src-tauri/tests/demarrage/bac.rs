//! Le bac à sable de ces tests : un dossier temporaire avec `noir/` et `hean/` côte à côte,
//! comme dans `~/Library/Application Support`.
//!
//! Extrait de `demarrage.rs` le 2026-11-25, quand celui-ci a dépassé 400 lignes en recevant le
//! test d'INV-26. Le dépassement était **annoncé** : l'encadré « qui va rougir le premier » de
//! `tools/check-sizes.sh` nommait ce fichier comme le deuxième à franchir la limite, et disait
//! « c'est là qu'atterrira chaque test futur de la séquence de démarrage ». Donc découper, pas
//! désactiver — et découper selon le thème, comme `tests/bibliotheque/`.

use std::path::{Path, PathBuf};

use hean_lib::demarrage;
use hean_lib::library::reprise_v1::config_de_v1;
use hean_lib::library::Depot;

/// Un bac avec `noir/` et `hean/` côte à côte, comme dans `Application Support`.
pub(crate) struct Bac {
    pub(crate) base: PathBuf,
}

impl Bac {
    pub(crate) fn neuf(nom: &str) -> Bac {
        let base = std::env::temp_dir().join(format!("hean-dem-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("bac");
        Bac { base }
    }

    pub(crate) fn ancien(&self) -> PathBuf {
        self.base.join("noir")
    }

    pub(crate) fn nouveau(&self) -> PathBuf {
        self.base.join("hean")
    }

    pub(crate) fn poser(&self, relatif: &str, contenu: &[u8]) -> PathBuf {
        let p = self.ancien().join(relatif);
        std::fs::create_dir_all(p.parent().expect("parent")).expect("dossiers");
        std::fs::write(&p, contenu).expect("écriture");
        p
    }

    /// Un `noir/` de testeur : des données, un dossier de musique et une piste supprimée.
    pub(crate) fn poser_un_dossier_v1(&self, exclusion: &str) {
        self.poser("playlists.json", br#"[{"nom":"Jazz"}]"#);
        self.poser("covers/pochette.jpg", b"des octets de pochette");
        self.poser(
            "config.json",
            config_de_v1(&["/Musique"], &[exclusion]).as_bytes(),
        );
    }

    pub(crate) fn preparer(&self) -> Result<Depot, demarrage::Erreur> {
        demarrage::preparer(&self.ancien(), &self.nouveau())
    }
}

impl Drop for Bac {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

pub(crate) fn interdire_la_lecture(fichier: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut p = std::fs::metadata(fichier).expect("meta").permissions();
    p.set_mode(0o000);
    std::fs::set_permissions(fichier, p).expect("permissions");
}

pub(crate) fn autoriser_la_lecture(fichier: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut p = std::fs::metadata(fichier).expect("meta").permissions();
    p.set_mode(0o644);
    std::fs::set_permissions(fichier, p).expect("permissions");
}
