//! Les commandes IPC des sources réseau. Minces, comme celles de la bibliothèque : chacune
//! délègue et traduit l'erreur. Toute la logique est dans `network`, qui se teste sans Tauri.
//!
//! **Aucune de ces commandes ne rend de piste**, et c'est ce qui les garde hors du périmètre
//! du détecteur d'INV-06 : un scan rend un `RapportDeScan`, et c'est le dépôt du frontend
//! qui recharge la liste si ce rapport dit que quelque chose a bougé (V004).
//!
//! **Et aucune ne transporte un mot de passe vers l'interface.** `save_network_password`
//! n'en prend un que dans le sens descendant ; rien ne le rend, parce qu'un mot de passe
//! qui remonte apparaît dans les outils de développement de la WebView (ADR 0015).

use tauri::State;

use crate::library::{RapportDeScan, SourceReseau};
use crate::network;

use super::bibliotheque::Bibliotheque;

/// Traduit une erreur interne en message pour l'interface, après l'avoir journalisée.
/// `{:#}` suit la chaîne des causes : « une erreur est survenue » est ce qui a coûté le plus
/// de temps de diagnostic en V1.
fn echec(contexte: &str, e: impl std::fmt::Display) -> String {
    tracing::error!(contexte, erreur = %e, "commande réseau en échec");
    format!("{contexte} : {e}")
}

fn depot<'a>(etat: &'a State<'_, Bibliotheque>) -> std::sync::MutexGuard<'a, crate::library::Depot> {
    etat.inner().lock().unwrap_or_else(|e| e.into_inner())
}

/// Les sources réseau enregistrées. Pas de mot de passe dedans : il n'y en a pas dans
/// `SourceReseau`.
#[tauri::command]
#[specta::specta]
pub fn network_sources(etat: State<'_, Bibliotheque>) -> Result<Vec<SourceReseau>, String> {
    depot(&etat)
        .sources_reseau()
        .map_err(|e| echec("lecture des sources réseau", e))
}

/// Ajoute une source réseau, ou met à jour celle qui porte déjà le même identifiant.
///
/// **L'identifiant est dérivé de l'hôte, du partage et du chemin** — il n'est pas tiré au
/// hasard. Conséquence voulue : ajouter deux fois le même partage rend la **même** source,
/// donc les pistes déjà scannées ne deviennent pas orphelines. Un UUID aurait l'effet
/// inverse, et c'est un geste qu'on fait par erreur plus souvent qu'à son tour.
#[tauri::command]
#[specta::specta]
pub fn add_network_source(
    etat: State<'_, Bibliotheque>,
    nom: String,
    hote: String,
    partage: String,
    chemin_distant: String,
    utilisateur: String,
) -> Result<Vec<SourceReseau>, String> {
    let source = SourceReseau {
        id: network::uri::identifiant_derive(&hote, &partage, &chemin_distant),
        nom: if nom.trim().is_empty() { hote.clone() } else { nom },
        hote,
        partage,
        chemin_distant,
        utilisateur,
        domaine: None,
        invite: false,
        active: true,
    };
    let mut d = depot(&etat);
    d.enregistrer_source_reseau(&source)
        .map_err(|e| echec("ajout d'une source réseau", e))?;
    d.sources_reseau()
        .map_err(|e| echec("lecture des sources réseau", e))
}

/// Enregistre le mot de passe d'une source dans le trousseau macOS.
///
/// Il descend ici et ne remonte jamais. Un mot de passe vide est **refusé** et non enregistré
/// (INV-04) : stocker une chaîne vide ferait tenter au NAS une connexion anonyme, donc
/// risquerait le verrouillage du compte.
#[tauri::command]
#[specta::specta]
pub fn save_network_password(source_id: String, mot_de_passe: String) -> Result<(), String> {
    network::identifiants::enregistrer(&source_id, &mot_de_passe)
        .map_err(|e| echec("enregistrement du mot de passe de la source réseau", e))
}

/// Scanne une source réseau et rend ce qui a changé.
///
/// ⚠️ **Cette commande dure des minutes** au premier scan d'une grande source : elle lit
/// l'en-tête de chaque fichier sur le NAS (ADR 0016 §3). Elle ne tient pas le verrou de la
/// bibliothèque pendant ce temps — `network::scanner_une_source` le prend deux fois, court
/// — donc l'affichage de la bibliothèque continue de répondre.
#[tauri::command]
#[specta::specta]
pub fn scan_network_source(
    etat: State<'_, Bibliotheque>,
    source_id: String,
) -> Result<RapportDeScan, String> {
    network::scanner_une_source(etat.inner(), &source_id)
        .map_err(|e| echec("scan d'une source réseau", e))
}

#[cfg(test)]
mod tests {
    /// Le message d'erreur porte son contexte, comme celui des commandes de bibliothèque :
    /// sans contexte, Thomas envoie une capture d'écran inexploitable.
    #[test]
    fn un_echec_est_lisible() {
        let m = super::echec("scan d'une source réseau", "NAS injoignable");
        assert_eq!(m, "scan d'une source réseau : NAS injoignable");
    }
}
