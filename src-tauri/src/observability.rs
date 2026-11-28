//! Journalisation. `println!` / `eprintln!` sont interdits dans ce crate (INV-21) :
//! V1 en comptait 252 contre 3 appels `tracing::`, et son `logging.rs` ne capturait pas
//! stdout — l'app installée a produit 63 lignes de journal dans toute sa vie, et les trois
//! bugs trouvés le 2026-11-21 étaient annoncés en clair en dev et invisibles en production.
//!
//! **Et V2 a hérité de l'autre moitié du défaut jusqu'au 2026-11-28** : tout partait sur
//! stdout, donc rien ne restait. Mesuré sur la semaine du 21 au 28 novembre, ça coûtait un
//! aller-retour humain à chaque test matériel — Thomas, qui ne lit pas le code, recopiait
//! des blocs de journal dans la conversation, a dû écrire « je souhaite que tu verifies toi
//! meme le terminal », et l'agent a fini par écrire `journaliser_les_anomalies()` pour
//! relire ses propres compteurs audio faute de pouvoir les retrouver après coup.
//!
//! D'où la couche fichier : `~/Library/Application Support/hean/logs/`, rotation
//! journalière, quinze jours gardés. La console reste — elle sert pendant `tauri dev`.

use std::io;
use std::path::Path;
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Le garde de l'écrivain non bloquant. **Il doit vivre aussi longtemps que le process** :
/// lâché, il emporte le fil d'écriture et les lignes en attente — un journal muet, qui est
/// exactement le défaut que ce module existe pour fermer.
static GARDE: OnceLock<WorkerGuard> = OnceLock::new();

/// Installe le souscripteur global. Idempotent : un second appel est ignoré.
pub fn init() {
    init_avec(crate::library::dossier::dossier_de_donnees().as_deref());
}

/// La même chose, mais le dossier de données passé en paramètre — c'est ce qui rend le cas
/// « pas de dossier » testable sans toucher à l'environnement du processus de test, et ce
/// qui évite qu'un `cargo test` écrive dans le vrai dossier de l'application.
///
/// `None` (HOME absent, cf. `library::dossier`) laisse la console seule : un journal qui
/// manque est un désagrément, un démarrage refusé serait une panne.
pub fn init_avec(base: Option<&Path>) {
    let filtre = EnvFilter::try_from_env("HEAN_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let couche_fichier = base.and_then(|base| journal(base).ok()).map(|ecrivain| {
        let (ecrivain, garde) = tracing_appender::non_blocking(ecrivain);
        // `set` échoue si `init` est rappelé : le premier garde reste, et c'est le bon.
        let _ = GARDE.set(garde);
        // Pas de couleurs dans un fichier : les codes ANSI rendent `grep` illisible.
        fmt::layer()
            .with_target(true)
            .with_ansi(false)
            .with_writer(ecrivain)
    });

    let _ = tracing_subscriber::registry()
        .with(filtre)
        .with(fmt::layer().with_target(true))
        .with(couche_fichier)
        .try_init();
}

/// L'écrivain tournant de `<base>/logs/hean.AAAA-MM-JJ.log` — la date se place ENTRE le
/// préfixe et le suffixe, pas à la fin : vérifié par le test, pas supposé.
///
/// Quinze fichiers gardés : assez pour qu'un bug signalé « la semaine dernière » soit
/// encore lisible, assez peu pour que le dossier ne grossisse pas sans fin. La rotation est
/// journalière et non par taille — on cherche un jour de test, pas un volume.
pub fn journal(base: &Path) -> io::Result<rolling::RollingFileAppender> {
    let dossier = base.join("logs");
    std::fs::create_dir_all(&dossier)?;
    rolling::Builder::new()
        .filename_prefix("hean")
        .filename_suffix("log")
        .rotation(rolling::Rotation::DAILY)
        .max_log_files(15)
        .build(dossier)
        .map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    /// `None` : aucune couche fichier, donc aucun fichier touché par `cargo test`.
    #[test]
    fn init_est_idempotent() {
        super::init_avec(None);
        super::init_avec(None);
    }

    /// Éprouver plutôt que supposer : un `create_dir_all` qui réussit ne prouve pas qu'une
    /// ligne arrive sur le disque. Celui-ci écrit, puis relit.
    #[test]
    fn le_journal_atterrit_dans_un_fichier() {
        let base = std::env::temp_dir().join(format!("hean-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);

        let mut ecrivain = super::journal(&base).expect("l'écrivain doit s'ouvrir");
        writeln!(ecrivain, "ligne de preuve").expect("l'écriture doit passer");
        ecrivain.flush().expect("le vidage doit passer");

        let fichiers: Vec<_> = std::fs::read_dir(base.join("logs"))
            .expect("le dossier logs doit exister")
            .flatten()
            .collect();
        assert_eq!(fichiers.len(), 1, "un seul fichier du jour");
        let nom = fichiers[0].file_name().to_string_lossy().into_owned();
        assert!(
            nom.starts_with("hean.") && nom.ends_with(".log"),
            "nom inattendu : {nom}"
        );
        let texte = std::fs::read_to_string(fichiers[0].path()).expect("relecture");
        assert!(texte.contains("ligne de preuve"), "la ligne n'est pas arrivée");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Un dossier impossible ne doit pas faire paniquer le démarrage.
    #[test]
    fn un_dossier_impossible_rend_une_erreur() {
        assert!(super::journal(std::path::Path::new("/dev/null/impossible")).is_err());
    }
}
