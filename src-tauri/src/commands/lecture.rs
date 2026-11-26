//! Les commandes IPC de la lecture.
//!
//! Minces, comme celles de la bibliothèque : chacune envoie un message au fil de lecture et
//! rend la main. **Aucune n'attend que le son sorte** — ouvrir un fichier et négocier la
//! fréquence du DAC prend des centaines de millisecondes, et les faire dans le fil des
//! commandes gèlerait la fenêtre (W005).
//!
//! # Pourquoi l'interface interroge plutôt qu'elle écoute
//!
//! V1 poussait la progression par `emit` à 30 images par seconde. Ici l'interface appelle
//! `playback_state` toutes les 100 ms. C'est moins élégant et c'est assumé : les événements
//! typés de tauri-specta sont une machinerie de plus pour une coquille jetable (ADR 0004 :
//! le design est la phase 4), et un sondage ne peut pas se désynchroniser de l'état réel
//! — il **est** l'état réel. Le jour où la barre de progression doit être fluide, c'est un
//! `collect_events!` à ajouter, pas une architecture à refaire.
//!
//! **Aucun `u64` ni `i64` ici** : `specta_typescript` les rend en `bigint`, que `JSON.parse`
//! ne sait pas relire. Les millisecondes tiennent dans un `u32` jusqu'à 49 jours.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

use crate::audio::chain::Volume;
use crate::audio::lecteur::{Commande, Lecteur};

/// L'état partagé : le lecteur, derrière un verrou — comme la bibliothèque, et pour la même
/// raison (un seul écrivain, ADR 0002).
pub type Lecture = Mutex<Lecteur>;

/// Un périphérique de sortie, tel que l'interface le montre.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct PeripheriqueAudio {
    pub id: u32,
    pub nom: String,
    pub par_defaut: bool,
    /// Les fréquences standard annoncées, pour l'affichage. La décision de jouer, elle,
    /// interroge le matériel (voir `audio::coreaudio::frequence::est_servie`).
    pub frequences: Vec<u32>,
    pub frequence_actuelle: u32,
    pub canaux_max: u32,
    /// Un AirPlay ne peut **jamais** être bit-perfect (PLAT-1) : l'interface doit pouvoir le
    /// dire avant que Thomas le choisisse, pas après.
    pub airplay: bool,
    pub bluetooth: bool,
}

/// Tout ce que la fenêtre a besoin de savoir sur la lecture en cours.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct EtatLecture {
    pub en_lecture: bool,
    pub en_pause: bool,
    pub position_ms: u32,
    /// Zéro quand le conteneur ne déclare pas de durée (un MP3 sans en-tête Xing). À
    /// afficher comme « durée inconnue », pas comme zéro seconde.
    pub duree_ms: u32,
    pub index: u32,
    pub total: u32,
    pub volume: f32,
    /// Nombre de trous dans le son depuis le début de la lecture. **Zéro est la seule valeur
    /// acceptable** : c'est la mesure de « ça ne coupe pas », et V1 n'en avait aucune.
    pub famines: u32,
    pub bit_perfect: bool,
    /// Les deux conditions que Hean peut constater séparément. « Pas bit-perfect » sans dire
    /// laquelle manque ne se corrige pas.
    pub sans_reechantillonnage: bool,
    pub mode_exclusif: bool,
    pub piste: Option<String>,
    pub peripherique: String,
    /// L'identifiant de la sortie active : c'est par lui que l'interface sait quelle ligne
    /// de sa liste est sélectionnée. Par le nom, deux DAC identiques seraient confondus.
    pub peripherique_id: u32,
    /// Ce qu'il y a à dire à Thomas : un refus de lecture, un avertissement AirPlay. C'est
    /// le canal qui manquait entièrement à V1 (`docs/MESURES-V1.md` §8).
    pub message: Option<String>,
}

fn lecteur<'a>(etat: &'a State<'_, Lecture>) -> std::sync::MutexGuard<'a, Lecteur> {
    etat.inner().lock().unwrap_or_else(|e| e.into_inner())
}

/// Traduit une erreur en message pour l'interface, après l'avoir journalisée.
fn echec(contexte: &str, e: impl std::fmt::Display) -> String {
    tracing::error!(contexte, erreur = %e, "commande de lecture en échec");
    format!("{contexte} : {e}")
}

fn envoyer(etat: &State<'_, Lecture>, contexte: &str, commande: Commande) -> Result<(), String> {
    lecteur(etat)
        .envoyer(commande)
        .map_err(|e| echec(contexte, e))
}

/// Remplace la file de lecture et démarre à `depart`.
#[tauri::command]
#[specta::specta]
pub fn play_tracks(
    etat: State<'_, Lecture>,
    chemins: Vec<String>,
    depart: u32,
) -> Result<(), String> {
    let file: Vec<PathBuf> = chemins.into_iter().map(PathBuf::from).collect();
    envoyer(
        &etat,
        "lancer la lecture",
        Commande::Jouer {
            file,
            depart: depart as usize,
        },
    )
}

#[tauri::command]
#[specta::specta]
pub fn playback_pause(etat: State<'_, Lecture>) -> Result<(), String> {
    envoyer(&etat, "mettre en pause", Commande::Pause)
}

#[tauri::command]
#[specta::specta]
pub fn playback_resume(etat: State<'_, Lecture>) -> Result<(), String> {
    envoyer(&etat, "reprendre la lecture", Commande::Reprendre)
}

#[tauri::command]
#[specta::specta]
pub fn playback_stop(etat: State<'_, Lecture>) -> Result<(), String> {
    envoyer(&etat, "arrêter la lecture", Commande::Arreter)
}

#[tauri::command]
#[specta::specta]
pub fn playback_seek(etat: State<'_, Lecture>, position_ms: u32) -> Result<(), String> {
    envoyer(
        &etat,
        "se déplacer dans la piste",
        Commande::Aller(u64::from(position_ms)),
    )
}

#[tauri::command]
#[specta::specta]
pub fn playback_next(etat: State<'_, Lecture>) -> Result<(), String> {
    envoyer(&etat, "passer à la piste suivante", Commande::Suivante)
}

#[tauri::command]
#[specta::specta]
pub fn playback_previous(etat: State<'_, Lecture>) -> Result<(), String> {
    envoyer(&etat, "revenir à la piste précédente", Commande::Precedente)
}

/// Le volume, de 0 à 1. **Seul 1,0 exactement est bit-perfect** (ADR 0007) : toute autre
/// valeur fait multiplier chaque échantillon, donc arrondir.
#[tauri::command]
#[specta::specta]
pub fn set_volume(etat: State<'_, Lecture>, volume: f32) -> Result<(), String> {
    envoyer(
        &etat,
        "régler le volume",
        Commande::Volume(Volume::nouveau(volume)),
    )
}

#[tauri::command]
#[specta::specta]
pub fn set_audio_device(etat: State<'_, Lecture>, id: u32) -> Result<(), String> {
    envoyer(&etat, "changer de sortie audio", Commande::Peripherique(id))
}

/// Le mode exclusif : Hean réserve le DAC, aucune autre application ne peut y mélanger du
/// son. C'est l'une des quatre conditions du bit-perfect. **Refusé en AirPlay.**
#[tauri::command]
#[specta::specta]
pub fn set_exclusive_mode(etat: State<'_, Lecture>, actif: bool) -> Result<(), String> {
    envoyer(&etat, "changer le mode exclusif", Commande::ModeExclusif(actif))
}

/// Relit la liste des périphériques. La liste elle-même remonte par `playback_state` : c'est
/// le fil de lecture qui parle au HAL, et lui seul.
#[tauri::command]
#[specta::specta]
pub fn refresh_audio_devices(etat: State<'_, Lecture>) -> Result<(), String> {
    envoyer(
        &etat,
        "relire les sorties audio",
        Commande::RafraichirPeripheriques,
    )
}

#[tauri::command]
#[specta::specta]
pub fn audio_devices(etat: State<'_, Lecture>) -> Vec<PeripheriqueAudio> {
    let partage = lecteur(&etat);
    let liste = partage.etat().peripheriques.lock();
    liste
        .map(|l| {
            l.iter()
                .map(|p| PeripheriqueAudio {
                    id: p.id,
                    nom: p.nom.clone(),
                    par_defaut: p.par_defaut,
                    frequences: p.frequences.clone(),
                    frequence_actuelle: p.frequence_actuelle,
                    canaux_max: u32::from(p.canaux_max),
                    airplay: p.est_airplay(),
                    bluetooth: p.est_bluetooth(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// L'état de la lecture, interrogé par la fenêtre toutes les 100 ms.
#[tauri::command]
#[specta::specta]
pub fn playback_state(etat: State<'_, Lecture>) -> EtatLecture {
    use std::sync::atomic::Ordering::Relaxed;
    let partage = lecteur(&etat);
    let e = partage.etat();
    EtatLecture {
        en_lecture: e.en_lecture.load(Relaxed),
        en_pause: e.en_pause.load(Relaxed),
        position_ms: e.position_ms.load(Relaxed).min(u64::from(u32::MAX)) as u32,
        duree_ms: e.duree_ms.load(Relaxed).min(u64::from(u32::MAX)) as u32,
        index: e.index.load(Relaxed) as u32,
        total: e.total.load(Relaxed) as u32,
        volume: e.volume().valeur(),
        famines: e.famines.load(Relaxed).min(u64::from(u32::MAX)) as u32,
        bit_perfect: e.bit_perfect.load(Relaxed),
        sans_reechantillonnage: e.sans_reechantillonnage.load(Relaxed),
        mode_exclusif: e.mode_exclusif.load(Relaxed),
        piste: e.piste.lock().ok().and_then(|p| p.clone()),
        peripherique: e
            .peripherique
            .lock()
            .map(|p| p.clone())
            .unwrap_or_default(),
        peripherique_id: e.peripherique_id.load(Relaxed),
        message: e.message.lock().ok().and_then(|m| m.clone()),
    }
}
