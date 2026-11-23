//! Ce qui se migre, ce qui ne se migre pas, et pourquoi.
//!
//! La liste est **explicite et non pas « tout le dossier »**. Un `cp -R` migrerait aussi les
//! caches jetables, ce qui double le volume à copier et ressuscite des fichiers que V1 était
//! censé supprimer. Mesuré sur le dossier réel de Thomas le 2026-11-22 : **572 Mo, dont
//! 289 Mo de `smb_buffer/`** — la moitié du dossier est un cache de téléchargement SMB que
//! V1 devait nettoyer à chaque lancement (comportement W008, correctif de V1 au commit
//! `eaee676`) et qui contenait encore trois fichiers de la veille.

use std::path::Path;

/// Les dossiers que la migration **ignore**, avec la raison.
///
/// `smb_buffer/` : fichiers temporaires du téléchargement progressif SMB. Les recopier
/// serait copier 289 Mo pour les supprimer au premier lancement — et si V2 oubliait de les
/// supprimer à son tour, on aurait transporté le bug de V1 avec ses données.
pub const IGNORES: &[&str] = &["smb_buffer"];

/// Le fichier témoin écrit dans le dossier d'arrivée, une fois la migration vérifiée.
/// Il porte le compte des fichiers et des octets : un témoin qui ne dit que « c'est fait »
/// ne permet pas de constater après coup qu'il manque quelque chose.
pub const TEMOIN: &str = "migration-depuis-noir.json";

/// Vrai si ce nom de fichier est caché, donc n'est pas une donnée de l'application.
///
/// **Un seul endroit le dit, et c'est voulu.** La convention était écrite trois fois — ici,
/// dans le test « ce dossier contient-il des données ? » et dans le déplacement avant
/// l'échange — avec un commentaire disant que les trois devaient s'accorder, et rien pour
/// l'imposer. Relevé en relecture : c'est la forme exacte du défaut qu'INV-06 a coûté à V1,
/// « n'oublie pas de répéter ça partout ». Celui des trois qui aurait changé d'avis aurait
/// fait de ces fichiers une donnée d'un côté et pas de l'autre — et l'un des trois chemins
/// supprime.
pub fn est_cache(nom: &std::ffi::OsStr) -> bool {
    nom.to_string_lossy().starts_with('.')
}

/// Vrai si ce chemin relatif, sous le dossier source, doit être migré.
pub fn a_migrer(relatif: &Path) -> bool {
    let premier = relatif.components().next();
    match premier {
        Some(std::path::Component::Normal(n)) => {
            // Les fichiers cachés (`.DS_Store`) ne sont pas des données de l'application.
            !IGNORES.contains(&n.to_string_lossy().as_ref()) && !est_cache(n)
        }
        _ => false,
    }
}

/// Les fichiers dont le contenu est vérifié **octet par octet** après la copie, et pas
/// seulement par leur taille.
///
/// Le partage est assumé : ce sont les fichiers qu'on ne peut pas reconstruire. Une
/// pochette perdue se retélécharge ; une playlist perdue ne revient pas. Vérifier les
/// 275 Mo de `covers/` par leur contenu coûterait quelques secondes de lecture
/// supplémentaires au premier lancement pour protéger des données **régénérables**.
pub fn a_verifier_au_contenu(relatif: &Path) -> bool {
    relatif
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json") || e.eq_ignore_ascii_case("db"))
}

#[cfg(test)]
mod tests {
    use super::{a_migrer, a_verifier_au_contenu};
    use std::path::Path;

    #[test]
    fn le_cache_smb_ne_se_migre_pas() {
        assert!(!a_migrer(Path::new("smb_buffer/2db60c33d8867478.flac")));
        assert!(!a_migrer(Path::new("smb_buffer")));
    }

    #[test]
    fn les_donnees_de_lutilisateur_se_migrent() {
        for p in [
            "playlists.json",
            "config.json",
            "listening_history.json",
            "network_sources.json",
            "covers/une-pochette.jpg",
            "thumbnails/abc_thumb.jpg",
            "logs/noir.log.2026-11-21",
            "feedback/feedback_1777844906.json",
            "dap_sync.db",
        ] {
            assert!(a_migrer(Path::new(p)), "{p} doit se migrer");
        }
    }

    #[test]
    fn les_fichiers_caches_ne_se_migrent_pas() {
        assert!(!a_migrer(Path::new(".DS_Store")));
    }

    #[test]
    fn seuls_les_fichiers_irremplacables_sont_verifies_au_contenu() {
        assert!(a_verifier_au_contenu(Path::new("playlists.json")));
        assert!(a_verifier_au_contenu(Path::new("dap_sync.db")));
        // Régénérable : une pochette se retélécharge.
        assert!(!a_verifier_au_contenu(Path::new("covers/pochette.jpg")));
    }
}
