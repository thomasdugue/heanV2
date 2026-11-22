//! Socle de phase 0. Aucune feature : voir bootstrap/01-produit/FEATURE-SET.md.
//!
//! Limite dure : aucun fichier de ce crate ne dépasse 400 lignes (lib.rs de V1 en faisait
//! 4 870). `./check` le vérifie.

pub mod audio;
pub mod commands;
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

    // La migration des données de V1, AVANT d'ouvrir la base : elle doit avoir déplacé le
    // dossier avant que quoi que ce soit n'écrive dedans (ADR 0003). Un échec n'empêche pas
    // de démarrer — l'ancien dossier est alors intact et la tentative reprendra au
    // lancement suivant — mais il est journalisé en `error!`, pas avalé.
    let ancien = library::dossier::ancien_dossier_de_donnees()
        .expect("HOME absent : vérifié juste au-dessus");
    match migration::migrer_si_necessaire(&ancien, &dossier) {
        Ok(rapport) => tracing::info!(?rapport, "migration des données de V1"),
        Err(e) => tracing::error!(erreur = %e, "la migration des données de V1 a échoué"),
    }

    let base = library::dossier::base_dans(&dossier);
    let mut depot = library::Depot::ouvrir(&base)
        .unwrap_or_else(|e| panic!("ouverture de la bibliothèque {} : {e}", base.display()));

    // Déplacer les fichiers ne suffit pas : V2 ne lit aucun JSON de V1. Sans cette reprise,
    // un testeur migré trouverait une bibliothèque vide et ses pistes supprimées de retour
    // au premier scan — ce qu'INV-06 interdit (ADR 0010). Elle est idempotente, donc
    // appelée à chaque démarrage sans condition : un échec de reprise au lancement
    // précédent se rattrape ainsi tout seul.
    match std::fs::read_to_string(dossier.join("config.json")) {
        Ok(config) => match library::reprise_v1::reprendre(&mut depot, &config) {
            Ok(reprise) => tracing::info!(?reprise, "reprise de la configuration de V1"),
            Err(e) => tracing::error!(erreur = %e, "la reprise de la configuration de V1 a échoué"),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("pas de config.json de V1 : rien à reprendre");
        }
        Err(e) => tracing::error!(erreur = %e, "config.json de V1 illisible"),
    }

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
