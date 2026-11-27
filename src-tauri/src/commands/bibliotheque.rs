//! Les commandes IPC de la bibliothèque.
//!
//! Elles sont **minces, et c'est le point** : chacune verrouille le dépôt, délègue, et
//! traduit l'erreur. Toute la logique est dans `library::repository`, qui est testable sans
//! Tauri — V1 avait l'inverse, un `lib.rs` de 4 870 lignes où la logique vivait dans les
//! commandes, donc ne se testait qu'en lançant l'application.
//!
//! **Un seul écrivain.** L'état partagé est un `Mutex<Depot>` : une base, une connexion, un
//! verrou. C'est ce qui remplace les sept caches globaux de V1 (`METADATA_CACHE`,
//! `TRACKS_CACHE`, `COVER_CACHE`…) modifiés concurremment sans transaction, et donc ce qui
//! raye INV-08, 09, 10 et 14. Ajouter ici un second état mutable rouvrirait ces portes
//! (ADR 0002) — c'est BLOQUANT.
//!
//! **Les erreurs remontent en texte** jusqu'à l'interface, et sont journalisées par
//! `tracing` avant de partir (INV-21). Une erreur IPC typée serait mieux ; elle attendra
//! d'avoir un affichage d'erreur à nourrir (phase 3). Ce qui compte dès maintenant est
//! qu'aucune erreur ne se perde en silence, ce que V1 faisait avec `unwrap_or_default()`.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

use crate::library::{Depot, Edition, RapportDeScan, Track};

/// L'état partagé : le dépôt, derrière un verrou.
pub type Bibliotheque = std::sync::Arc<Mutex<Depot>>;

/// Traduit une erreur interne en message pour l'interface, après l'avoir journalisée.
/// Le `format!` final est volontairement complet (`{:#}` suit la chaîne des causes) :
/// « une erreur est survenue » est ce qui a coûté le plus de temps de diagnostic en V1.
fn echec(contexte: &str, e: impl std::fmt::Display) -> String {
    tracing::error!(contexte, erreur = %e, "commande de bibliothèque en échec");
    format!("{contexte} : {e}")
}

/// Verrouille le dépôt. Un verrou empoisonné (un thread a paniqué en le tenant) ne doit pas
/// faire paniquer l'app à son tour : on reprend la donnée et on continue.
fn depot<'a>(etat: &'a State<'_, Bibliotheque>) -> std::sync::MutexGuard<'a, Depot> {
    etat.inner().lock().unwrap_or_else(|e| e.into_inner())
}

/// Les chemins en texte : l'IPC ne transporte pas de `PathBuf`, et une conversion faite à
/// un seul endroit ne peut pas diverger d'une commande à l'autre.
fn en_texte(chemins: Vec<PathBuf>) -> Vec<String> {
    chemins.iter().map(|p| p.display().to_string()).collect()
}

/// Toutes les pistes de la bibliothèque, chemins exclus filtrés (INV-06).
#[tauri::command]
#[specta::specta]
pub fn list_tracks(etat: State<'_, Bibliotheque>) -> Result<Vec<Track>, String> {
    depot(&etat).lister_pistes().map_err(|e| echec("lecture de la bibliothèque", e))
}

/// Scanne les dossiers enregistrés et rend ce qui a changé.
///
/// Le rapport est **ce qui permet à l'interface de ne rien recharger pour rien** (V004) :
/// en V1, `scan_complete` déclenchait un rechargement inconditionnel, qui entrait en course
/// avec le premier rendu de la home.
#[tauri::command]
#[specta::specta]
pub fn scan_library(etat: State<'_, Bibliotheque>) -> Result<RapportDeScan, String> {
    depot(&etat).scanner().map_err(|e| echec("scan de la bibliothèque", e))
}

/// Ajoute un dossier de bibliothèque (B04) et rend la liste complète des racines.
#[tauri::command]
#[specta::specta]
pub fn add_library_root(etat: State<'_, Bibliotheque>, path: String) -> Result<Vec<String>, String> {
    let mut d = depot(&etat);
    d.ajouter_racine(&PathBuf::from(&path))
        .map_err(|e| echec("ajout d'un dossier de bibliothèque", e))?;
    Ok(en_texte(d.racines().map_err(|e| echec("lecture des dossiers de bibliothèque", e))?))
}

/// Les dossiers de bibliothèque enregistrés.
#[tauri::command]
#[specta::specta]
pub fn library_roots(etat: State<'_, Bibliotheque>) -> Result<Vec<String>, String> {
    Ok(en_texte(
        depot(&etat)
            .racines()
            .map_err(|e| echec("lecture des dossiers de bibliothèque", e))?,
    ))
}

/// Écrit une édition de métadonnées **dans le fichier audio**, puis met la base à jour
/// (INV-07). Rend la piste telle que la relecture du fichier l'a donnée.
#[tauri::command]
#[specta::specta]
pub fn write_metadata(
    etat: State<'_, Bibliotheque>,
    path: String,
    edition: Edition,
) -> Result<Track, String> {
    depot(&etat)
        .editer(&path, &edition)
        .map_err(|e| echec("écriture des métadonnées", e))
}

/// Le bulk edit (B08) : les champs laissés vides ne sont pas touchés.
#[tauri::command]
#[specta::specta]
pub fn bulk_edit(
    etat: State<'_, Bibliotheque>,
    paths: Vec<String>,
    edition: Edition,
) -> Result<Vec<Track>, String> {
    depot(&etat)
        .editer_en_lot(&paths, &edition)
        .map_err(|e| echec("édition en lot", e))
}

/// Exclut une piste de la bibliothèque, **définitivement** (INV-06, B09).
#[tauri::command]
#[specta::specta]
pub fn exclude_track(etat: State<'_, Bibliotheque>, path: String) -> Result<(), String> {
    depot(&etat)
        .exclure(&path)
        .map_err(|e| echec("exclusion d'une piste", e))
}

#[cfg(test)]
mod tests {
    /// Le message d'erreur porte son contexte. Sans contexte, l'utilisateur voit « erreur »
    /// et Thomas reçoit une capture d'écran inexploitable.
    #[test]
    fn un_echec_est_lisible() {
        let m = super::echec("scan de la bibliothèque", "fichier introuvable");
        assert_eq!(m, "scan de la bibliothèque : fichier introuvable");
    }
}
