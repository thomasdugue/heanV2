//! Le point de substitution du moteur audio.
//!
//! Ce trait existait déjà en V1, où il avait été prévu pour un éventuel portage WASAPI. Il
//! sert ici à autre chose, et de plus utile : brancher un backend de test qui **écrit dans
//! un fichier WAV** au lieu de parler au HAL (`wav_backend.rs`). C'est ce qui fait passer
//! le bit-perfect de promesse à assertion, vérifiable en CI et sans matériel.
//!
//! # Ce que ce trait ne contient pas, et pourquoi
//!
//! **Aucune méthode de choix de périphérique.** La tentation serait un
//! `set_device(id) -> Result<()>`, et ce serait un piège : d'après PLAT-1 (🔴), un
//! périphérique AirPlay n'existe dans CoreAudio **que tant qu'il est le défaut système**.
//! Dès qu'il perd ce statut, macOS tue la session et le périphérique disparaît du HAL —
//! et `set_system_default_device` sur un périphérique périmé **accepte silencieusement sans
//! rien faire**. Un setter qui rend `Ok(())` alors que rien n'a eu lieu est exactement la
//! dégradation invisible que ce projet refuse.
//!
//! Le routage AirPlay ne s'exprime donc pas comme un setter : quand on quitte AirPlay on ne
//! change **pas** le défaut système, et l'audio est routé par assignement explicite sur
//! l'AudioUnit. Cette logique appartient au backend CoreAudio, avec son cache de session,
//! et pas à une interface que le backend WAV devrait faire semblant d'implémenter. Stratégie
//! complète dans `bootstrap/04-invariants/SAVOIR-AUDIO.md`.

use super::error::ErreurAudio;
use super::format::FormatAudio;

/// Ce que doit savoir faire une sortie audio, qu'elle soit un DAC ou un fichier.
pub trait BackendAudio {
    /// Prépare la sortie pour ce format, et rend le format **réellement obtenu**.
    ///
    /// Le type de retour est le cœur de l'affaire. Un `Result<(), _>` laisserait croire
    /// qu'un `Ok` signifie « le périphérique joue à 192 kHz », alors qu'un DAC peut très
    /// bien accepter la demande et tourner à 48 kHz. En rendant le format obtenu, un
    /// appelant peut **constater** l'écart — et le refuser, puisque le bit-perfect interdit
    /// de rééchantillonner (ADR 0002). C'est aussi le cas AirPlay, où le sample rate ne
    /// doit surtout pas être changé et où le format rendu est le natif du périphérique.
    fn preparer(&mut self, format: FormatAudio) -> Result<FormatAudio, ErreurAudio>;

    /// Pousse des trames entrelacées, en `i32` mis à l'échelle sur 32 bits
    /// (voir `samples.rs`). Rend le nombre d'**échantillons** consommés.
    fn ecrire(&mut self, echantillons: &[i32]) -> Result<usize, ErreurAudio>;

    /// Termine : vide les tampons et libère la sortie.
    ///
    /// Explicite, et non confié à `Drop` : pour un fichier, c'est ici que l'en-tête est
    /// réécrit avec la taille réelle, et une erreur à ce moment doit remonter. Un `Drop` ne
    /// peut rien signaler — ce serait une perte de données en silence.
    fn terminer(&mut self) -> Result<(), ErreurAudio>;
}
