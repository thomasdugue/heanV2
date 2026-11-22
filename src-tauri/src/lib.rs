//! Socle de phase 0. Aucune feature : voir bootstrap/01-produit/FEATURE-SET.md.
//!
//! Limite dure : aucun fichier de ce crate ne dépasse 400 lignes (lib.rs de V1 en faisait
//! 4 870). `./check` le vérifie.

pub mod audio;
pub mod commands;
pub mod observability;
pub mod secrets;
pub mod store;

use tauri_specta::{collect_commands, Builder};

/// Le catalogue IPC, en un seul endroit. C'est lui qui produit `src/ipc/bindings.ts`.
pub fn ipc_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![commands::app_version])
}

/// Point d'entrée de l'application.
pub fn run() {
    observability::init();
    let builder = ipc_builder();

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "Hean démarre");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("échec du démarrage de Tauri");
}
