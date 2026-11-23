//! Socle de phase 0. Aucune feature : voir bootstrap/01-produit/FEATURE-SET.md.
//!
//! Limite dure : aucun fichier de ce crate ne dépasse 400 lignes (lib.rs de V1 en faisait
//! 4 870). `./check` le vérifie.

pub mod audio;
pub mod commands;
pub mod library;
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
    let base = library::dossier::base_dans(&dossier);
    let depot = library::Depot::ouvrir(&base)
        .unwrap_or_else(|e| panic!("ouverture de la bibliothèque {} : {e}", base.display()));

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
