//! Socle de phase 0. Aucune feature : voir bootstrap/01-produit/FEATURE-SET.md.
//!
//! Limite dure : aucun fichier de ce crate ne dépasse 400 lignes (lib.rs de V1 en faisait
//! 4 870). `./check` le vérifie.

pub mod audio;
pub mod commands;
pub mod demarrage;
pub mod library;
pub mod migration;
pub mod observability;
pub mod secrets;
pub mod store;

use std::sync::Mutex;

use tauri_specta::{collect_commands, Builder};

/// Le catalogue IPC, en un seul endroit. C'est lui qui produit `src/ipc/bindings.ts`.
pub fn ipc_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::app_version,
        commands::bibliotheque::list_tracks,
        commands::bibliotheque::scan_library,
        commands::bibliotheque::add_library_root,
        commands::bibliotheque::library_roots,
        commands::bibliotheque::write_metadata,
        commands::bibliotheque::bulk_edit,
        commands::bibliotheque::exclude_track,
    ])
}

/// Point d'entrée de l'application.
pub fn run() {
    observability::init();
    let builder = ipc_builder();

    // La bibliothèque s'ouvre AU DÉMARRAGE et une seule fois : un seul écrivain pour une
    // seule base (ADR 0002). Si elle ne s'ouvre pas, l'app ne démarre pas — ouvrir une base
    // de secours en mémoire donnerait une bibliothèque vide d'apparence normale, et
    // l'utilisateur croirait avoir tout perdu.
    let dossier = library::dossier::dossier_de_donnees()
        .expect("HOME absent ou relatif : impossible de localiser le dossier de données (ADR 0003)");

    // Migration du dossier de V1, ouverture de la base, reprise de la configuration : la
    // séquence est dans `demarrage`, pas ici, parce que son ORDRE est un invariant et qu'un
    // invariant écrit dans `run()` n'est pas testable (ADR 0012). Elle échoue si et seulement
    // si démarrer causerait une perte — un échec de migration en est un, parce qu'ouvrir la
    // base rendrait cet échec définitif.
    let ancien = library::dossier::ancien_dossier_de_donnees()
        .expect("HOME absent : vérifié juste au-dessus");
    let base = library::dossier::base_dans(&dossier);
    let depot = match demarrage::preparer(&ancien, &dossier) {
        Ok(depot) => depot,
        Err(e) => {
            // Journalisé AVANT de paniquer : un panic part sur stderr, que l'app installée
            // ne capture pas (INV-21). Ce message-ci doit se retrouver dans le journal que
            // « Attach recent logs » envoie.
            tracing::error!(erreur = %e, "Hean ne démarre pas");
            panic!("{e}");
        }
    };

    tauri::Builder::default()
        .manage(Mutex::new(depot) as commands::bibliotheque::Bibliotheque)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            tracing::info!(version = env!("CARGO_PKG_VERSION"), base = %base.display(), "Hean démarre");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("échec du démarrage de Tauri");
}
