//! Où vit la base. Un seul chemin, écrit une fois (ADR 0003).
//!
//! V1 documentait **trois** chemins de dossier de données, et deux étaient faux : son
//! `CLAUDE.md` et son `logging.rs` annonçaient `~/.local/share/noir/`, qui est le chemin
//! **Linux** de `dirs::data_dir()`. Personne ne s'en est aperçu parce qu'une phrase dans un
//! document ne peut pas échouer. Ici la fonction est unique, et un test dit ce qu'elle rend.
//!
//! La migration one-shot depuis `~/Library/Application Support/noir/` est hors de ce
//! module et de cette phase : c'est l'une des cinq zones jamais déléguées
//! (`tools/zones-interdites.txt`), elle demande des fixtures tirées de vraies données V1,
//! et elle aura sa session.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// `~/Library/Application Support/hean/`, ou `None` si `HOME` est absent ou relatif.
///
/// **`None` plutôt qu'un chemin de repli**, et c'est la correction d'un défaut relevé en
/// relecture : rendre un chemin relatif ferait créer la base **dans le répertoire courant**,
/// donc une bibliothèque vide d'apparence parfaitement normale. L'utilisateur croirait avoir
/// tout perdu, et rien ne le signalerait. C'est exactement le mode de défaut qu'INV-21
/// décrit : un défaut qui se voit se répare, un défaut invisible se propage.
pub fn dossier_de_donnees() -> Option<PathBuf> {
    depuis_home(std::env::var_os("HOME"))
}

/// La même chose, mais la variable d'environnement passée en paramètre — c'est ce qui rend
/// le cas « `HOME` absent » testable sans toucher à l'environnement du processus de test.
pub fn depuis_home(home: Option<OsString>) -> Option<PathBuf> {
    let home = PathBuf::from(home?);
    if !home.is_absolute() || home.as_os_str().is_empty() {
        return None;
    }
    Some(home.join("Library").join("Application Support").join("hean"))
}

/// Le fichier de la bibliothèque.
pub fn base_de_bibliotheque() -> Option<PathBuf> {
    dossier_de_donnees().map(|d| d.join("library.db"))
}

/// Le même chemin, à partir d'un dossier de données donné. Sert au démarrage, qui a déjà
/// résolu le dossier et veut le nommer dans son message d'erreur.
pub fn base_dans(dossier: &Path) -> PathBuf {
    dossier.join("library.db")
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    #[test]
    fn le_chemin_est_celui_de_ladr_0003() {
        let base = super::base_de_bibliotheque().expect("HOME est défini dans un test");
        let texte = base.display().to_string();
        assert!(
            texte.ends_with("Library/Application Support/hean/library.db"),
            "chemin inattendu : {texte}"
        );
        // Et surtout PAS le chemin Linux que la doc de V1 annonçait à tort.
        assert!(!texte.contains(".local/share"), "{texte}");
        assert!(!texte.contains("/noir/"), "le dossier s'appelle hean (ADR 0003)");
    }

    #[test]
    fn sans_home_il_ny_a_pas_de_chemin_de_repli() {
        // Les trois façons dont ça peut mal tourner, et aucune ne doit rendre un chemin
        // relatif : la base se créerait dans le répertoire courant, et la bibliothèque
        // paraîtrait simplement vide.
        assert_eq!(super::depuis_home(None), None);
        assert_eq!(super::depuis_home(Some(OsString::new())), None);
        assert_eq!(super::depuis_home(Some(OsString::from("relatif/maison"))), None);
    }

    #[test]
    fn avec_un_home_absolu_le_chemin_est_complet() {
        // Le `HOME` d'essai n'est volontairement PAS de la forme `/Users/quelquun` :
        // `tests/architecture/aucun-chemin-machine.test.ts` refuse un chemin de machine
        // dans le code, et il a raison même pour une valeur de test.
        let p = super::depuis_home(Some(OsString::from("/maison-dessai"))).expect("chemin");
        assert_eq!(
            p.display().to_string(),
            "/maison-dessai/Library/Application Support/hean"
        );
        assert_eq!(super::base_dans(&p).file_name().unwrap(), "library.db");
    }
}
