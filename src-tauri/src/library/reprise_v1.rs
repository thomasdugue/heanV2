//! Reprendre ce que V1 gardait dans son `config.json` : les **dossiers de bibliothèque** et
//! la **liste d'exclusion** (ADR 0010).
//!
//! **Pourquoi ce module existe du tout.** La migration de l'ADR 0003 déplace des fichiers,
//! et c'est nécessaire — mais V2 ne lit aucun JSON de V1. Sans cette reprise, un testeur
//! migré trouverait au premier lancement une bibliothèque **vide**, ses dossiers de musique
//! oubliés, et — bien pire — **ses pistes supprimées de retour** au premier scan. C'est
//! exactement ce qu'INV-06 (🔴 BLOQUANT) interdit : la suppression d'une piste est
//! permanente, y compris à travers une migration. Déplacer les octets n'est pas migrer.
//!
//! **Pourquoi ce module ne touche pas à SQL.** Il ne passe que par l'API publique du dépôt
//! (`ajouter_racine`, `exclure`). INV-06 veut qu'un seul fichier au monde nomme la table des
//! chemins exclus ; y écrire une insertion directe ferait de ce module un deuxième endroit
//! où l'exclusion vit, et le test d'architecture le refuserait — à juste titre. Le nom de
//! cette table n'est donc pas écrit ici, **pas même en commentaire** : le détecteur le
//! cherche à la lettre dans tout le backend, et c'est précisément ce qui lui donne ses
//! dents. (Il l'a prouvé deux fois en phase 2, sur `scan.rs` puis sur ce fichier-ci.)
//!
//! **Ce qui n'est PAS repris** : playlists, favoris, historique d'écoute. Leurs fichiers
//! sont migrés et attendent les phases qui créeront leurs tables ; inventer leur forme
//! maintenant serait deviner. La bibliothèque elle-même n'est pas reprise non plus — elle se
//! rescanne depuis les dossiers, ce qui est plus sûr que de traduire `tracks_cache.json`
//! dans un schéma différent, et c'est automatique.

use std::path::Path;

use super::repository::{Depot, Erreur, CLE_V1_EXCLUSIONS, CLE_V1_RACINES};

/// Le sujet, tel qu'il est noté dans la table `reprises`.
const SUJET: &str = "config.json de V1";

/// Ce qu'une reprise a ramené.
#[derive(Debug, Default, PartialEq)]
pub struct Reprise {
    pub racines: u32,
    pub exclusions: u32,
    /// Vrai si la reprise avait déjà eu lieu : elle ne se rejoue pas.
    pub deja_faite: bool,
}

/// Reprend les deux listes, une seule fois.
///
/// Un `config.json` illisible n'empêche ni la reprise de ce qui est lisible, ni le démarrage
/// de l'application : on prend ce qu'on trouve, et le journal dit ce qu'on a trouvé.
pub fn reprendre(depot: &mut Depot, config_json: &str) -> Result<Reprise, Erreur> {
    if depot.reprise_deja_faite(SUJET)? {
        return Ok(Reprise { deja_faite: true, ..Default::default() });
    }

    let racines = liste(config_json, CLE_V1_RACINES);
    let exclusions = liste(config_json, CLE_V1_EXCLUSIONS);

    for path in &racines {
        depot.ajouter_racine(Path::new(path))?;
    }
    for path in &exclusions {
        depot.exclure(path)?;
    }
    // Le marqueur en dernier : si l'application meurt au milieu, la reprise recommence au
    // lancement suivant, et chaque insertion est idempotente.
    depot.noter_la_reprise(SUJET)?;

    let reprise = Reprise {
        racines: u32::try_from(racines.len()).unwrap_or(u32::MAX),
        exclusions: u32::try_from(exclusions.len()).unwrap_or(u32::MAX),
        deja_faite: false,
    };
    tracing::info!(?reprise, "configuration de V1 reprise");
    Ok(reprise)
}

/// Les chaînes d'un tableau du `config.json`, ou rien si la clé manque ou n'est pas un
/// tableau de chaînes. V1 écrivait `"sentry_enabled": null` : un JSON de V1 n'a pas la forme
/// qu'on croit, et une reprise ne doit pas échouer sur un champ qui ne la concerne pas.
fn liste(config_json: &str, cle: &str) -> Vec<String> {
    let lu: serde_json::Value = serde_json::from_str(config_json).unwrap_or(serde_json::Value::Null);
    lu.get(cle)
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{liste, CLE_V1_EXCLUSIONS, CLE_V1_RACINES};

    /// Un `config.json` de V1, composé avec les constantes plutôt qu'avec les noms en clair
    /// (voir `repository.rs` : un seul fichier nomme la liste d'exclusion).
    fn config(racines: &str, exclusions: &str) -> String {
        format!(
            "{{\"{CLE_V1_RACINES}\":{racines},\"{CLE_V1_EXCLUSIONS}\":{exclusions},\"sentry_enabled\":null}}"
        )
    }

    #[test]
    fn les_deux_listes_se_lisent() {
        let c = config("[\"/Musique\",\"/NAS\"]", "[\"/Musique/x.flac\"]");
        assert_eq!(liste(&c, CLE_V1_RACINES), vec!["/Musique", "/NAS"]);
        assert_eq!(liste(&c, CLE_V1_EXCLUSIONS), vec!["/Musique/x.flac"]);
    }

    #[test]
    fn un_config_vide_ou_illisible_ne_fait_pas_echouer_la_lecture() {
        for c in ["", "{}", "pas du json", "null", "[]"] {
            assert_eq!(liste(c, CLE_V1_RACINES), Vec::<String>::new(), "{c:?}");
        }
    }

    #[test]
    fn une_cle_qui_nest_pas_un_tableau_de_chaines_est_ignoree() {
        // V1 écrivait `"sentry_enabled": null`. Un JSON réel n'a pas la forme qu'on croit.
        let c = config("\"pas-un-tableau\"", "[1,2,null,\"/vrai.flac\"]");
        assert_eq!(liste(&c, CLE_V1_RACINES), Vec::<String>::new());
        assert_eq!(liste(&c, CLE_V1_EXCLUSIONS), vec!["/vrai.flac"]);
    }
}
