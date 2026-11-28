//! Socle de phase 0. Aucune feature : voir bootstrap/01-produit/FEATURE-SET.md.
//!
//! Limite dure : aucun fichier de ce crate ne dépasse 400 lignes (lib.rs de V1 en faisait
//! 4 870). `./check` le vérifie.

pub mod audio;
pub mod commands;
pub mod demarrage;
pub mod library;
pub mod migration;
pub mod network;
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
        commands::reseau::network_sources,
        commands::reseau::add_network_source,
        commands::reseau::save_network_password,
        commands::reseau::scan_network_source,
        commands::lecture::play_tracks,
        commands::lecture::playback_set_paused,
        commands::lecture::playback_stop,
        commands::lecture::playback_seek,
        commands::lecture::playback_skip,
        commands::lecture::set_volume,
        commands::lecture::set_audio_device,
        commands::lecture::set_exclusive_mode,
        commands::lecture::refresh_audio_devices,
        commands::lecture::playback_state,
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

    // Le fil de lecture démarre avec l'application, mais il ne touche à AUCUN matériel
    // avant le premier `play_tracks` : lancer Hean ne doit ni réserver le DAC, ni changer sa
    // fréquence, ni réveiller un AirPlay.
    // Le dépôt est partagé, et pas seulement confié à Tauri : la fabrique d'ouverture
    // ci-dessous doit lire les sources réseau du MÊME dépôt que les commandes.
    let depot = std::sync::Arc::new(Mutex::new(depot));
    let pour_ouvrir = std::sync::Arc::clone(&depot);

    // LE SEUL ENDROIT QUI CONNAÎT LES DEUX ZONES, et c'est pour ça qu'il est ici (ADR 0022) :
    // le fil de lecture ignore qu'une piste peut vivre sur un NAS, le réseau ignore qu'il
    // existe un décodeur. Chacun reste chez lui, et le câblage se lit en un endroit.
    let lecteur = audio::Lecteur::demarrer_avec(std::sync::Arc::new(
        move |chemin: &std::path::Path| {
            let texte = chemin.to_string_lossy();
            // `est_distant` et non `starts_with("smb://")` : la forme d'une URI est déjà
            // écrite dans `library::scan`, et l'écrire une troisième fois est la forme de
            // défaut d'INV-01 et d'INV-06 — une normalisation répartie en plusieurs endroits,
            // trois bugs pour une cause.
            if !library::scan::est_distant(&texte) {
                return audio::Decodeur::ouvrir(chemin);
            }
            let distant = network::ouvrir_pour_lecture(&pour_ouvrir, &texte).map_err(|e| {
                // Le texte de l'erreur réseau est conservé tel quel : c'est lui qui nomme
                // l'hôte, le partage et la cause probable, et c'est ce que l'écran affiche.
                audio::ErreurAudio::Lecture {
                    chemin: chemin.to_path_buf(),
                    source: std::io::Error::other(e.to_string()),
                }
            })?;
            let taille = distant.taille();
            audio::Decodeur::ouvrir_flux(
                Box::new(audio::source::SourceMesuree::nouvelle(distant, taille)),
                chemin,
            )
        },
    ));
    let _ = lecteur.envoyer(audio::Commande::RafraichirPeripheriques);

    tauri::Builder::default()
        .manage(depot as commands::bibliotheque::Bibliotheque)
        .manage(Mutex::new(lecteur) as commands::lecture::Lecture)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            tracing::info!(version = env!("CARGO_PKG_VERSION"), base = %base.display(), "Hean démarre");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("échec du démarrage de Tauri");
}
