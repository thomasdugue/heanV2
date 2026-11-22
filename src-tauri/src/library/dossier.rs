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

use std::path::PathBuf;

/// `~/Library/Application Support/hean/`. macOS uniquement, comme l'app (ADR 0002).
pub fn dossier_de_donnees() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Library").join("Application Support").join("hean")
}

/// Le fichier de la bibliothèque.
pub fn base_de_bibliotheque() -> PathBuf {
    dossier_de_donnees().join("library.db")
}

#[cfg(test)]
mod tests {
    #[test]
    fn le_chemin_est_celui_de_ladr_0003() {
        let base = super::base_de_bibliotheque();
        let texte = base.display().to_string();
        assert!(
            texte.ends_with("Library/Application Support/hean/library.db"),
            "chemin inattendu : {texte}"
        );
        // Et surtout PAS le chemin Linux que la doc de V1 annonçait à tort.
        assert!(!texte.contains(".local/share"), "{texte}");
        assert!(!texte.contains("/noir/"), "le dossier s'appelle hean (ADR 0003)");
    }
}
