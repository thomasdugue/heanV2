//! Le lecteur : ce qui transforme « appuie sur lecture » en son.
//!
//! # La forme, et la raison de cette forme
//!
//! Un seul fil possède le décodeur et le backend ; tout le reste lui parle par messages, et
//! l'observe par des atomiques. C'est ce qui permet à une commande IPC de rendre la main
//! **immédiatement** : décoder un paquet FLAC 24/192 prend quelques millisecondes, ouvrir un
//! fichier sur le NAS peut prendre une seconde, et faire ça dans le fil des commandes gèlerait
//! l'interface — le défaut W005 de V1, mesuré sur `get_cover` et `write_metadata`.
//!
//! ```text
//!   IPC ──Commande──▶ [fil de lecture] ──ecrire()──▶ tampon ──▶ callback temps réel ──▶ DAC
//!    ▲                      │
//!    └───── EtatPartage ◀───┘   (atomiques : position, famines, verdict…)
//! ```
//!
//! # L'enchaînement d'un album sans trou
//!
//! Quand la piste suivante a **exactement le même format**, le fil continue de pousser dans
//! le même tampon : il n'y a aucune pause, parce qu'il ne se passe rien. C'est le vrai
//! gapless, et il ne coûte aucune machinerie. Quand le format diffère, il faut laisser le
//! tampon se vider, refermer le flux et renégocier la fréquence du DAC — environ 200 ms de
//! silence, et c'est irréductible : le matériel doit reverrouiller sa PLL.
//!
//! # Ce qui n'est pas ici
//!
//! La lecture d'un fichier du NAS. Elle demande de toucher à `src-tauri/src/network/`, hors
//! du périmètre de cette session (ADR 0013) : voir `docs/HISTORY.md`.

mod fil;
mod pilotage;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use coreaudio_sys::AudioObjectID;

use super::chain::Volume;
use super::coreaudio::InfoPeripherique;
use super::error::ErreurAudio;

/// Ce que le fil de lecture fait, sur demande.
pub enum Commande {
    /// Remplace la file et démarre à `depart`.
    Jouer { file: Vec<PathBuf>, depart: usize },
    Pause,
    Reprendre,
    Arreter,
    /// Déplacement dans la piste, en millisecondes.
    Aller(u64),
    Suivante,
    Precedente,
    Volume(Volume),
    Peripherique(AudioObjectID),
    ModeExclusif(bool),
    /// Relit la liste des périphériques.
    RafraichirPeripheriques,
    /// Arrête le fil. Envoyée par `Drop`.
    Fin,
}

/// Tout ce que l'interface peut observer, sans verrou sur le chemin chaud.
#[derive(Default)]
pub struct EtatPartage {
    pub en_lecture: AtomicBool,
    pub en_pause: AtomicBool,
    pub position_ms: AtomicU64,
    pub duree_ms: AtomicU64,
    /// Index dans la file de la piste **qu'on entend**, qui n'est pas forcément celle que le
    /// décodeur est en train de lire : jusqu'à cinq secondes les séparent.
    pub index: AtomicUsize,
    pub total: AtomicUsize,
    /// Le volume, en bits d'un `f32`.
    volume: AtomicU32,
    /// Callbacks servis à vide : chaque unité est un trou audible. **Zéro est la seule
    /// valeur acceptable**, et c'est ce qui rend « ça ne coupe pas » vérifiable.
    pub famines: AtomicU64,
    /// Vrai quand les quatre conditions du bit-perfect sont réunies. Les trois qui suivent
    /// disent **laquelle** manque : « pas bit-perfect » sans dire pourquoi ne se corrige pas.
    pub bit_perfect: AtomicBool,
    /// Le périphérique tourne à la fréquence du fichier : rien n'est rééchantillonné.
    pub sans_reechantillonnage: AtomicBool,
    pub mode_exclusif: AtomicBool,
    /// Le chemin de la piste qu'on entend.
    pub piste: Mutex<Option<String>>,
    /// Le nom du périphérique de sortie.
    pub peripherique: Mutex<String>,
    /// La dernière chose à dire à Thomas : un refus de lecture, un avertissement AirPlay.
    /// Pas une erreur technique à avaler en silence — c'est le canal qui manquait à V1.
    pub message: Mutex<Option<String>>,
    pub peripheriques: Mutex<Vec<InfoPeripherique>>,
}

impl EtatPartage {
    pub fn volume(&self) -> Volume {
        Volume::nouveau(f32::from_bits(self.volume.load(Ordering::Relaxed)))
    }

    pub fn definir_volume(&self, volume: Volume) {
        self.volume
            .store(volume.valeur().to_bits(), Ordering::Relaxed);
    }

    /// Remplace le message destiné à l'écran. Journalisé au passage : un message que
    /// l'interface écraserait avant que Thomas le voie doit rester dans le journal.
    pub fn dire(&self, texte: impl Into<String>) {
        let texte = texte.into();
        tracing::info!(message = %texte, "message de lecture");
        if let Ok(mut m) = self.message.lock() {
            *m = Some(texte);
        }
    }

    fn effacer_le_message(&self) {
        if let Ok(mut m) = self.message.lock() {
            *m = None;
        }
    }
}

/// La poignée que tient le reste de l'application.
pub struct Lecteur {
    commandes: mpsc::Sender<Commande>,
    etat: Arc<EtatPartage>,
}

impl Lecteur {
    /// Démarre le fil de lecture. **N'ouvre aucun périphérique** : le matériel n'est touché
    /// qu'au premier `Jouer`, pour qu'un lancement de Hean ne réserve pas le DAC et ne
    /// change pas sa fréquence sans que personne ait rien demandé.
    pub fn demarrer() -> Self {
        let etat = Arc::new(EtatPartage::default());
        etat.definir_volume(Volume::UNITE);
        let (envoi, reception) = mpsc::channel();
        let pour_le_fil = Arc::clone(&etat);
        std::thread::Builder::new()
            .name("hean-lecture".to_owned())
            .spawn(move || fil::tourner(reception, pour_le_fil))
            .expect("le fil de lecture ne démarre pas");
        Self {
            commandes: envoi,
            etat,
        }
    }

    pub fn etat(&self) -> &Arc<EtatPartage> {
        &self.etat
    }

    /// Envoie une commande. Une erreur ici veut dire que le fil de lecture est mort, ce qui
    /// n'arrive que sur un panic : le dire plutôt que l'ignorer.
    pub fn envoyer(&self, commande: Commande) -> Result<(), ErreurAudio> {
        if matches!(commande, Commande::Jouer { .. }) {
            // Un nouveau départ efface le message précédent : laisser affiché « ton DAC ne
            // sait pas jouer ce fichier » pendant la lecture d'un autre serait un mensonge.
            self.etat.effacer_le_message();
        }
        self.commandes.send(commande).map_err(|_| ErreurAudio::CoreAudio {
            operation: "joindre le fil de lecture (il s'est arrêté)".to_owned(),
            code: 0,
        })
    }
}

impl Drop for Lecteur {
    fn drop(&mut self) {
        // Le fil rend le matériel en sortant : fréquence d'origine remise, réservation
        // relâchée. Sans ce message il resterait bloqué sur `recv()` et ne le ferait jamais.
        let _ = self.commandes.send(Commande::Fin);
    }
}
