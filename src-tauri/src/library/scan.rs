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

/// Le schéma d'URI d'une piste qui n'est pas un fichier de ce Mac (ADR 0016).
///
/// Il vit **ici**, à côté des extensions, et pas dans `network/` : c'est tout ce que la
/// bibliothèque a besoin de savoir d'une piste distante — que son chemin n'est pas un
/// fichier. L'hôte, le partage et les identifiants sont l'affaire de `network/`, qui
/// assemble ses URI **à partir de cette constante**. Un seul endroit la nomme, donc le
/// jour où un second protocole arrive, il n'y a qu'une liste à allonger.
pub const SCHEME_RESEAU: &str = "smb://";

/// Vrai si ce chemin désigne une piste distante et non un fichier du disque.
pub fn est_distant(chemin: &str) -> bool {
    chemin.starts_with(SCHEME_RESEAU)
}

/// **INV-27 — le périmètre d'un scan.** Les pistes dont ce scan a le droit de conclure la
/// disparition, et elles seules.
///
/// Sans ce type, le premier scan de disque effaçait les 3 137 pistes du NAS de Thomas —
/// 91 % de sa bibliothèque — parce que `appliquer` efface toute piste connue qu'il n'a pas
/// revue, et qu'un parcours de disque ne revoit évidemment aucune piste SMB. En V1 les deux
/// scans écrivaient dans le même cache global et la règle était « n'oublie pas de filtrer » :
/// la forme de règle qu'INV-06 existe pour ne plus écrire.
///
/// C'est un **type sans variante « tout »**, et c'est le mécanisme : le dépôt ne peut pas
/// oublier de restreindre, puisqu'il n'a pas le choix de ne pas le faire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Perimetre {
    /// Les fichiers du disque — tout ce qui n'est pas une URI réseau.
    Disque,
    /// Une source réseau, désignée par le préfixe commun de ses URI (`smb://{id}/`).
    /// Le préfixe et non l'`id` seul : deux sources ne doivent pas pouvoir se recouvrir
    /// parce que l'une a un identifiant qui commence comme celui de l'autre.
    Source(String),
}

impl Perimetre {
    /// Vrai si cette piste tombe dans le périmètre.
    pub fn contient(&self, chemin: &str) -> bool {
        match self {
            Perimetre::Disque => !est_distant(chemin),
            Perimetre::Source(prefixe) => chemin.starts_with(prefixe.as_str()),
        }
    }
}

/// Ce qu'un inventaire autorise le dépôt à conclure.
///
/// **Né le 2026-11-25, d'un test qui refusait de passer.** `appliquer` efface toute piste de
/// son périmètre qu'il n'a pas revue — c'est ce qui fait disparaître de la bibliothèque un
/// fichier que l'utilisateur a supprimé. Mais un dossier qu'on n'a **pas pu lire** n'est pas
/// un dossier vide : un NAS qui refuse un partage, un disque externe éjecté, un dossier aux
/// permissions changées, et c'est toute une partie de la bibliothèque qui s'efface alors que
/// rien n'a bougé. C'est exactement ce que B17 dit — « un dossier absent ne doit pas faire
/// perdre la bibliothèque entière » — et le parcours le journalisait sans que personne n'en
/// tire la conséquence.
///
/// La règle est donc dans le type : un parcours qui a sauté quelque chose rend `Partielle`,
/// et rien ne disparaît ce tour-là. La seule chose qu'on perd est de voir une suppression
/// au scan **suivant**, quand le dossier répondra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conclusion {
    /// Tout le périmètre a été vu. Ce qui n'y est plus a disparu.
    Complete,
    /// Une partie n'a pas pu être lue. **Rien ne disparaît.**
    Partielle,
}

impl Conclusion {
    /// `Partielle` dès qu'un dossier a résisté. Une seule façon de tirer cette conclusion,
    /// pour que le disque et le réseau ne divergent pas.
    pub fn depuis(dossiers_illisibles: u32) -> Self {
        if dossiers_illisibles == 0 {
            Conclusion::Complete
        } else {
            Conclusion::Partielle
        }
    }
}

/// Une piste candidate à l'entrée en bibliothèque, et **d'où viennent ses métadonnées**.
///
/// La différence entre les deux provenances n'est pas le format du chemin : c'est **qui lit
/// les tags**. Un fichier local, le dépôt le lit lui-même, et seulement s'il a changé. Une
/// piste distante, non : un aller-retour réseau coûte des dizaines de millisecondes, il a
/// lieu une seule fois, hors de tout verrou (ADR 0016 §5), et ce qui arrive ici est déjà lu.
pub enum Entree {
    /// Un fichier du disque. Le dépôt lira ses tags s'il a changé.
    Disque(Trouvee),
    /// Une piste distante. Les métadonnées sont **déjà lues** quand il a fallu les lire, et
    /// `None` veut dire « vue, inchangée, rien n'a été relu » — pas « illisible ».
    ///
    /// La distinction porte une propriété qui compte : une piste vue reste **vue**, donc
    /// elle n'est pas effacée comme disparue. Sans ce `None`, une piste déjà connue devrait
    /// être relue pour simplement rester en place, et un scan de 3 137 pistes relirait
    /// 3 137 fichiers à chaque fois.
    Distante(Trouvee, Option<Box<super::metadonnees::Lues>>),
}

impl Entree {
    pub fn trouvee(&self) -> &Trouvee {
        match self {
            Entree::Disque(t) | Entree::Distante(t, _) => t,
        }
    }

    /// Le chemin tel qu'il entre dans la base : un chemin de fichier, ou une URI.
    pub fn chemin(&self) -> String {
        self.trouvee().chemin.display().to_string()
    }
}

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
pub fn parcourir(racines: &[PathBuf]) -> Parcours {
    let mut p = Parcours { trouvees: Vec::new(), dossiers_illisibles: 0 };
    for racine in racines {
        descendre(racine, &mut p);
    }
    p.trouvees.sort_by(|a, b| a.chemin.cmp(&b.chemin));
    p.trouvees.dedup_by(|a, b| a.chemin == b.chemin);
    p
}

/// Ce qu'un parcours a trouvé, **et ce qu'il n'a pas pu lire**.
///
/// Le second compte n'est pas de la statistique : c'est lui qui décide de la `Conclusion`,
/// donc de si une piste peut disparaître ce tour-ci. Avant le 2026-11-25, le parcours se
/// contentait de journaliser un dossier illisible — et un dossier illisible effaçait ses
/// pistes.
pub struct Parcours {
    pub trouvees: Vec<Trouvee>,
    pub dossiers_illisibles: u32,
}

fn descendre(dossier: &Path, p: &mut Parcours) {
    let entrees = match std::fs::read_dir(dossier) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(dossier = %dossier.display(), erreur = %e, "dossier illisible, ignoré");
            p.dossiers_illisibles += 1;
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
            Ok(t) if t.is_dir() => descendre(&chemin, p),
            Ok(t) if t.is_file() => {
                if let Some(trouvee) = examiner(&chemin) {
                    p.trouvees.push(trouvee);
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
    Some(empreinte(chemin))
}

/// Taille et date de modification d'un fichier, sans se demander si c'est de la musique.
///
/// Le dépôt s'en sert après avoir réécrit les tags d'un fichier : la taille et la date ont
/// changé, et ce sont celles du disque qui doivent entrer dans la base — sinon le scan
/// suivant croirait la piste modifiée et la relirait pour rien, ce qui ferait tomber V004.
/// Une seule façon de prendre une empreinte, donc, et pas deux qui se mettraient à diverger.
pub fn empreinte(chemin: &Path) -> Trouvee {
    let meta = std::fs::metadata(chemin).ok();
    Trouvee {
        chemin: chemin.to_path_buf(),
        taille: meta.as_ref().map(|m| m.len()).unwrap_or(0),
        mtime_ns: meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| i64::try_from(d.as_nanos()).ok())
            .unwrap_or(0),
    }
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
    use super::{est_audio, est_distant, parcourir, Conclusion, Perimetre, EXTENSIONS, SCHEME_RESEAU};
    use std::path::{Path, PathBuf};

    /// INV-27 — un périmètre ne contient que ce qui est à lui. C'est la propriété qui
    /// empêche un scan de disque d'effacer 91 % de la bibliothèque de Thomas.
    #[test]
    fn un_perimetre_de_disque_ne_contient_aucune_piste_distante() {
        let disque = Perimetre::Disque;
        assert!(disque.contient("/Musique/a.flac"));
        assert!(!disque.contient("smb://abc/music/LOSSLESS/a.flac"));
        // Un chemin local qui *parle* de smb n'est pas distant : c'est le schéma qui compte.
        assert!(disque.contient("/Musique/smb/a.flac"));
        assert!(disque.contient("/Volumes/music/a.flac"));
    }

    #[test]
    fn un_perimetre_de_source_ne_contient_que_la_sienne() {
        let source = Perimetre::Source(format!("{SCHEME_RESEAU}abc/"));
        assert!(source.contient("smb://abc/music/LOSSLESS/a.flac"));
        assert!(!source.contient("smb://def/music/LOSSLESS/a.flac"));
        assert!(!source.contient("/Musique/a.flac"));
        // Le slash final du préfixe est ce qui empêche deux sources de se recouvrir : sans
        // lui, « abc » attraperait aussi « abcdef ».
        assert!(!source.contient("smb://abcdef/music/a.flac"));
    }

    #[test]
    fn les_deux_perimetres_ne_se_recouvrent_jamais() {
        // La propriété qui compte, dite comme une partition : toute piste appartient à
        // exactement un des deux périmètres.
        let disque = Perimetre::Disque;
        let source = Perimetre::Source(format!("{SCHEME_RESEAU}abc/"));
        for chemin in [
            "/Musique/a.flac",
            "smb://abc/music/a.flac",
            "smb://def/music/a.flac",
            "",
        ] {
            assert!(
                !(disque.contient(chemin) && source.contient(chemin)),
                "{chemin} tombe dans les deux périmètres"
            );
        }
        assert!(est_distant("smb://abc/x"));
        assert!(!est_distant("/tmp/x"));
    }

    fn fixtures() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/audio"))
    }

    #[test]
    fn le_scan_trouve_les_fixtures_et_rien_dautre() {
        let trouvees = parcourir(&[fixtures()]).trouvees;
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
        let parcours = parcourir(&[
            PathBuf::from("/dossier/qui/nexiste/pas"),
            fixtures(),
        ]);
        assert!(!parcours.trouvees.is_empty(), "la racine lisible doit quand même être scannée");
        // **Et le parcours le DIT.** C'est ce qui manquait : la racine absente était
        // journalisée et rien n'en tenait compte, donc les pistes de ce dossier étaient
        // effacées comme disparues — un NAS débranché faisait perdre sa part de la
        // bibliothèque, ce que B17 interdit depuis V1.
        assert_eq!(parcours.dossiers_illisibles, 1);
        assert_eq!(
            super::Conclusion::depuis(parcours.dossiers_illisibles),
            super::Conclusion::Partielle
        );
    }

    #[test]
    fn deux_racines_imbriquees_ne_dupliquent_pas_une_piste() {
        // Thomas peut très bien ajouter un dossier ET son parent (B04).
        let trouvees = parcourir(&[fixtures(), fixtures().join("golden")]).trouvees;
        let mut chemins: Vec<_> = trouvees.iter().map(|t| t.chemin.clone()).collect();
        let avant = chemins.len();
        chemins.sort();
        chemins.dedup();
        assert_eq!(avant, chemins.len(), "une piste ne doit apparaître qu'une fois");
    }

    /// Un parcours complet autorise la disparition ; un parcours partiel ne l'autorise pas.
    /// La fonction est unique pour que le disque et le réseau ne divergent jamais.
    #[test]
    fn une_conclusion_se_derive_du_nombre_de_dossiers_illisibles() {
        assert_eq!(Conclusion::depuis(0), Conclusion::Complete);
        for n in [1, 2, 300] {
            assert_eq!(Conclusion::depuis(n), Conclusion::Partielle, "{n}");
        }
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
