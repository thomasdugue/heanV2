//! Le parcours du disque. Ce module **trouve des fichiers** et rien d'autre : il ne décide
//! pas ce qui entre dans la bibliothèque.
//!
//! C'est volontaire. La décision d'inclure ou d'exclure une piste appartient au dépôt, et à
//! lui seul (INV-06) : en V1, le filtre d'exclusion était recopié dans `init_cache()`,
//! `start_background_scan()` et `scan_network_source_cmd()`, et la règle disait « et dans
//! tout nouveau point d'entrée ». Un scan qui filtrerait lui aussi serait un deuxième
//! endroit où la règle vit, donc un endroit où elle peut se mettre à diverger.
//!
//! Le nom exact de la table d'exclusion n'est pas écrit ici, pas même en commentaire : le
//! test d'architecture d'INV-06 le cherche à la lettre dans tout le backend, et un
//! commentaire qui le citerait affaiblirait un détecteur qui ne se trompe aujourd'hui
//! jamais. Le détail est dans `repository.rs`, à côté du filtre.
//!
//! Le scan est séquentiel et mono-thread. V1 le parallélisait avec rayon (B02), et ce sera à
//! faire — mais pas avant qu'une mesure le réclame : la parallélisation de V1 arrivait avec
//! sept caches globaux modifiés concurremment, et c'est elle qui a rendu les invariants 8, 9
//! et 10 nécessaires. Un seul écrivain, d'abord (ADR 0002).

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Les extensions que la bibliothèque reconnaît. La liste suit **exactement** les formats
/// annoncés comme supportés, c'est-à-dire les features de symphonia dans `Cargo.toml` :
/// WMA, DSD, Opus et MQA n'y sont pas, et le rester rend l'absence de support vérifiable au
/// lieu d'accidentelle. Un fichier d'une autre extension n'est pas une erreur : il n'est
/// simplement pas de la musique que cette app sait jouer.
pub const EXTENSIONS: &[&str] = &["flac", "wav", "aiff", "aif", "m4a", "mp3", "ogg"];

/// Un fichier trouvé sur le disque, avec ce qu'il faut pour savoir s'il a changé.
///
/// Taille **et** date de modification : V1 utilisait les deux (B03), et c'est le bon choix.
/// La date seule rate un fichier réécrit dans la même seconde ; la taille seule rate une
/// réécriture de tags qui garde la même longueur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trouvee {
    pub chemin: PathBuf,
    pub taille: u64,
    pub mtime_ns: i64,
}

/// Parcourt les racines et rend tous les fichiers audio, triés par chemin.
///
/// Rien n'interrompt le parcours : un dossier illisible (NAS débranché, disque éjecté,
/// permissions) est journalisé et sauté. En V1, l'event `library_paths_inaccessible`
/// existait justement parce qu'un dossier absent ne doit pas faire perdre la bibliothèque
/// entière (B17).
pub fn parcourir(racines: &[PathBuf]) -> Vec<Trouvee> {
    let mut out = Vec::new();
    for racine in racines {
        descendre(racine, &mut out);
    }
    out.sort_by(|a, b| a.chemin.cmp(&b.chemin));
    out.dedup_by(|a, b| a.chemin == b.chemin);
    out
}

fn descendre(dossier: &Path, out: &mut Vec<Trouvee>) {
    let entrees = match std::fs::read_dir(dossier) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(dossier = %dossier.display(), erreur = %e, "dossier illisible, ignoré");
            return;
        }
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        // Les fichiers cachés sont sautés : `.DS_Store` n'est pas de la musique, et les
        // `._truc` des volumes SMB non plus.
        if chemin
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }
        match entree.file_type() {
            Ok(t) if t.is_dir() => descendre(&chemin, out),
            Ok(t) if t.is_file() => {
                if let Some(trouvee) = examiner(&chemin) {
                    out.push(trouvee);
                }
            }
            // Un lien symbolique n'est pas suivi : une boucle de liens ferait tourner le
            // scan indéfiniment, et `read_dir` ne protège de rien.
            _ => {}
        }
    }
}

fn examiner(chemin: &Path) -> Option<Trouvee> {
    if !est_audio(chemin) {
        return None;
    }
    let meta = std::fs::metadata(chemin).ok()?;
    let mtime_ns = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_nanos()).ok())
        .unwrap_or(0);
    Some(Trouvee {
        chemin: chemin.to_path_buf(),
        taille: meta.len(),
        mtime_ns,
    })
}

/// Vrai si l'extension est l'une de celles que l'app sait jouer. La comparaison est
/// insensible à la casse : `.FLAC` sort de certains logiciels de rip.
pub fn est_audio(chemin: &Path) -> bool {
    chemin
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| EXTENSIONS.contains(&e.as_str()))
}

#[cfg(test)]
mod tests {
    use super::{est_audio, parcourir, EXTENSIONS};
    use std::path::{Path, PathBuf};

    fn fixtures() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/audio"))
    }

    #[test]
    fn le_scan_trouve_les_fixtures_et_rien_dautre() {
        let trouvees = parcourir(&[fixtures()]);
        assert!(trouvees.len() >= 20, "{} fichiers trouvés", trouvees.len());

        // Le README du dossier ne doit PAS entrer dans la bibliothèque.
        assert!(
            !trouvees.iter().any(|t| t.chemin.ends_with("README.md")),
            "un .md n'est pas de la musique"
        );
        // Les goldens sont dans un sous-dossier : la descente est bien récursive.
        assert!(
            trouvees.iter().any(|t| t.chemin.ends_with("golden/flac-16-44.wav")),
            "le parcours doit être récursif"
        );
        // Un nom de fichier accentué s'ouvre (sur macOS il est stocké en NFD).
        let accents = trouvees
            .iter()
            .find(|t| t.chemin.to_string_lossy().contains("accents-"))
            .expect("accents-éàü.flac doit être trouvé");
        assert!(accents.taille > 1_000, "la taille doit être lue");
        assert!(accents.mtime_ns > 0, "la date de modification doit être lue");
    }

    #[test]
    fn un_dossier_illisible_ne_fait_pas_echouer_le_scan() {
        // Le cas B17 : une racine absente (NAS débranché). Le scan rend ce qu'il a pu lire.
        let trouvees = parcourir(&[
            PathBuf::from("/dossier/qui/nexiste/pas"),
            fixtures(),
        ]);
        assert!(!trouvees.is_empty(), "la racine lisible doit quand même être scannée");
    }

    #[test]
    fn deux_racines_imbriquees_ne_dupliquent_pas_une_piste() {
        // Thomas peut très bien ajouter un dossier ET son parent (B04).
        let trouvees = parcourir(&[fixtures(), fixtures().join("golden")]);
        let mut chemins: Vec<_> = trouvees.iter().map(|t| t.chemin.clone()).collect();
        let avant = chemins.len();
        chemins.sort();
        chemins.dedup();
        assert_eq!(avant, chemins.len(), "une piste ne doit apparaître qu'une fois");
    }

    #[test]
    fn le_detecteur_dextension_reconnait_les_formats_annonces() {
        for ext in EXTENSIONS {
            assert!(est_audio(Path::new(&format!("a.{ext}"))), ".{ext}");
            assert!(est_audio(Path::new(&format!("a.{}", ext.to_uppercase()))), "casse");
        }
        // Les formats explicitement NON supportés (Cargo.toml) le restent.
        for ext in ["wma", "dsf", "opus", "mqa", "md", "jpg", ""] {
            assert!(!est_audio(Path::new(&format!("a.{ext}"))), ".{ext}");
        }
    }
}
