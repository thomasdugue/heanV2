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

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

use crate::audio::chain::Volume;
use crate::audio::lecteur::{Commande, Lecteur};

/// L'état partagé : le lecteur, derrière un verrou — comme la bibliothèque, et pour la même
/// raison (un seul écrivain, ADR 0002).
pub type Lecture = Mutex<Lecteur>;

/// Un périphérique de sortie, tel que l'interface le montre.
///
/// Les champs n'ont **aucun commentaire**, et c'est voulu : `specta` recopie les
/// doc-commentaires dans `src/ipc/bindings.ts`, pour chaque variante du type — le pourquoi y
/// serait donc écrit deux fois, dans un fichier généré que personne ne lit. Il vit là où les
/// valeurs sont produites : `audio/coreaudio/peripheriques.rs` pour `airplay` (PLAT-1 : un
/// AirPlay n'est jamais bit-perfect, et l'interface doit pouvoir le dire **avant** qu'on le
/// choisisse) et `frequence.rs` pour `frequences`, la liste sur laquelle le repli choisit.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct PeripheriqueAudio {
    pub id: u32,
    pub nom: String,
    pub par_defaut: bool,
    pub frequences: Vec<u32>,
    pub frequence_actuelle: u32,
    pub canaux_max: u32,
    pub airplay: bool,
    pub bluetooth: bool,
}

impl PeripheriqueAudio {
    fn depuis(p: &crate::audio::coreaudio::InfoPeripherique) -> Self {
        Self {
            id: p.id,
            nom: p.nom.clone(),
            par_defaut: p.par_defaut,
            frequences: p.frequences.clone(),
            frequence_actuelle: p.frequence_actuelle,
            canaux_max: u32::from(p.canaux_max),
            airplay: p.est_airplay(),
            bluetooth: p.est_bluetooth(),
        }
    }
}

/// Tout ce que la fenêtre a besoin de savoir sur la lecture en cours.
///
/// # Deux choses à savoir avant d'y toucher
///
/// **Aucun `u64` ni `i64`** : `specta_typescript` les rend en `bigint`, que `JSON.parse` ne
/// sait pas relire. Les millisecondes tiennent dans un `u32` jusqu'à 49 jours.
///
/// **Aucun `Option`, et aucun commentaire sur les champs.** `specta` émet **deux** types dès
/// qu'un champ est optionnel (`_Serialize` et `_Deserialize`), et recopie les
/// doc-commentaires dans chacun : cette structure-ci pesait à elle seule 122 des 436 lignes
/// de `src/ipc/bindings.ts`, qui a une limite de 400. Une chaîne vide dit « rien » aussi bien
/// qu'un `None` pour un chemin de piste ou un message, et le pourquoi de chaque valeur vit là
/// où elle est produite — `audio/lecteur/mod.rs` pour l'état, `audio/coreaudio/negociation.rs`
/// pour les deux fréquences et le badge bit-perfect.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct EtatLecture {
    pub en_lecture: bool,
    pub en_pause: bool,
    pub position_ms: u32,
    pub duree_ms: u32,
    pub index: u32,
    pub total: u32,
    pub volume: f32,
    pub famines: u32,
    pub bit_perfect: bool,
    pub sans_reechantillonnage: bool,
    pub mode_exclusif: bool,
    pub source_hz: u32,
    pub source_bits: u32,
    pub sortie_hz: u32,
    pub airplay: bool,
    /// Vide quand rien ne joue.
    pub piste: String,
    pub peripherique: String,
    pub peripherique_id: u32,
    /// Vide quand il n'y a rien à dire.
    pub message: String,
    pub peripheriques: Vec<PeripheriqueAudio>,
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

/// Pause ou reprise. **Un seul booléen** plutôt que deux commandes : l'appel est idempotent,
/// donc deux clics rapides ne peuvent pas laisser l'état à l'envers.
#[tauri::command]
#[specta::specta]
pub fn playback_set_paused(etat: State<'_, Lecture>, en_pause: bool) -> Result<(), String> {
    let (quoi, commande) = if en_pause {
        ("mettre en pause", Commande::Pause)
    } else {
        ("reprendre la lecture", Commande::Reprendre)
    };
    envoyer(&etat, quoi, commande)
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

/// Change de piste de `pas` rangs : `1` pour la suivante, `-1` pour la précédente. Une seule
/// commande, parce que le fil de lecture n'a de toute façon qu'une seule opération dessous.
#[tauri::command]
#[specta::specta]
pub fn playback_skip(etat: State<'_, Lecture>, pas: i32) -> Result<(), String> {
    envoyer(&etat, "changer de piste", Commande::Changer(i64::from(pas)))
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
        source_hz: e.source_hz.load(Relaxed),
        source_bits: e.source_bits.load(Relaxed),
        sortie_hz: e.sortie_hz.load(Relaxed),
        airplay: e.airplay.load(Relaxed),
        piste: e.piste.lock().ok().and_then(|p| p.clone()).unwrap_or_default(),
        peripherique: e
            .peripherique
            .lock()
            .map(|p| p.clone())
            .unwrap_or_default(),
        peripherique_id: e.peripherique_id.load(Relaxed),
        message: e.message.lock().ok().and_then(|m| m.clone()).unwrap_or_default(),
        peripheriques: e
            .peripheriques
            .lock()
            .map(|l| l.iter().map(PeripheriqueAudio::depuis).collect())
            .unwrap_or_default(),
    }
}
