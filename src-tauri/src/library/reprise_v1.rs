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
//!
//! ## Deux défauts corrigés le 2026-11-23, tous deux du même genre
//!
//! Les deux faisaient perdre la liste d'exclusion — donc violaient INV-06 — par des chemins
//! que la relecture de la PR a trouvés et que les tests affirmaient sans les voir :
//!
//! 1. un `config.json` **illisible** posait le marqueur quand même, donc la reprise ne se
//!    retentait jamais. V1 écrivait ses six fichiers JSON **sans écriture atomique** : un
//!    fichier tronqué est le cas réel, pas l'exception ;
//! 2. les **racines** étaient posées avant les **exclusions**, et chaque insertion est sa
//!    propre transaction.
//!
//! Les deux commentaires ci-dessous disent ce que chaque correction ferme.

use std::path::Path;

use super::repository::{Depot, CLE_V1_EXCLUSIONS, CLE_V1_RACINES};

/// Le sujet, tel qu'il est noté dans la table `reprises`.
const SUJET: &str = "config.json de V1";

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    /// Le `config.json` de V1 n'est pas du JSON lisible. **Le marqueur n'est pas posé** : la
    /// reprise sera retentée au démarrage suivant, et l'erreur est journalisée.
    ///
    /// C'est le cœur de la correction : conclure « cet utilisateur n'avait aucune exclusion »
    /// d'un fichier qu'on n'a pas su lire ferait revenir ses pistes supprimées, et **pour
    /// toujours**, puisque le marqueur interdit toute nouvelle tentative. Mieux vaut une
    /// erreur à chaque démarrage, visible dans le journal que « Attach recent logs » envoie,
    /// qu'une perte silencieuse — c'est le raisonnement d'INV-21.
    #[error("le config.json de V1 ne se lit pas comme du JSON ({source}) — rien n'a été repris, on retentera au prochain démarrage")]
    ConfigIllisible {
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Depot(#[from] super::repository::Erreur),
}

/// Ce qu'une reprise a ramené.
#[derive(Debug, Default, PartialEq)]
pub struct Reprise {
    pub racines: u32,
    pub exclusions: u32,
    /// Vrai si la reprise avait déjà eu lieu : elle ne se rejoue pas.
    pub deja_faite: bool,
}

/// Les deux listes qu'un `config.json` de V1 porte pour nous.
#[derive(Debug, Default, PartialEq)]
pub struct Config {
    pub racines: Vec<String>,
    pub exclusions: Vec<String>,
}

/// Reprend les deux listes, une seule fois.
pub fn reprendre(depot: &mut Depot, config_json: &str) -> Result<Reprise, Erreur> {
    if depot.reprise_deja_faite(SUJET)? {
        return Ok(Reprise { deja_faite: true, ..Default::default() });
    }

    let config = analyser(config_json)?;

    // **Les exclusions d'abord, et c'est tout l'objet de la correction.** Chaque insertion
    // est sa propre transaction, donc un échec au milieu — base verrouillée, disque plein —
    // laisse la base dans l'état atteint, et un échec de reprise n'empêche pas l'application
    // de démarrer. Avec les racines en premier, cet état était « des dossiers de musique et
    // aucune exclusion » : le premier scan ramenait les pistes supprimées (INV-06 🔴). Dans
    // cet ordre-ci, le pire état possible est « des exclusions et aucune racine » — rien à
    // scanner, donc rien à ressusciter — et le démarrage suivant reprend où on s'est arrêté.
    for path in &config.exclusions {
        depot.exclure(path)?;
    }
    for path in &config.racines {
        depot.ajouter_racine(Path::new(path))?;
    }
    // Le marqueur en dernier : si l'application meurt au milieu, la reprise recommence au
    // lancement suivant, et chaque insertion est idempotente.
    depot.noter_la_reprise(SUJET)?;

    let reprise = Reprise {
        racines: u32::try_from(config.racines.len()).unwrap_or(u32::MAX),
        exclusions: u32::try_from(config.exclusions.len()).unwrap_or(u32::MAX),
        deja_faite: false,
    };
    tracing::info!(?reprise, "configuration de V1 reprise");
    Ok(reprise)
}

/// Lit les deux listes du `config.json` de V1.
///
/// **Rend une erreur si le JSON ne se parse pas**, et c'est la distinction qui manquait :
/// « lu, rien dedans » n'est pas « pas lisible ». La première conclusion est légitime, la
/// seconde ne doit jamais être tirée.
pub fn analyser(config_json: &str) -> Result<Config, Erreur> {
    let lu: serde_json::Value =
        serde_json::from_str(config_json).map_err(|source| Erreur::ConfigIllisible { source })?;
    Ok(Config {
        racines: liste(&lu, CLE_V1_RACINES),
        exclusions: liste(&lu, CLE_V1_EXCLUSIONS),
    })
}

/// Les chaînes d'un tableau du JSON lu, ou rien si la clé manque ou n'est pas un tableau de
/// chaînes. V1 écrivait `"sentry_enabled": null` : un JSON de V1 n'a pas la forme qu'on
/// croit, et une clé qui ne concerne pas la reprise ne doit pas la faire échouer.
fn liste(lu: &serde_json::Value, cle: &str) -> Vec<String> {
    lu.get(cle)
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{analyser, Erreur};

    /// Un `config.json` de V1, composé avec les constantes plutôt qu'avec les noms en clair
    /// (voir `repository.rs` : un seul fichier nomme la liste d'exclusion).
    fn config(racines: &str, exclusions: &str) -> String {
        format!(
            "{{\"{}\":{racines},\"{}\":{exclusions},\"sentry_enabled\":null}}",
            super::CLE_V1_RACINES,
            super::CLE_V1_EXCLUSIONS
        )
    }

    #[test]
    fn les_deux_listes_se_lisent() {
        let c = analyser(&config("[\"/Musique\",\"/NAS\"]", "[\"/Musique/x.flac\"]")).expect("lu");
        assert_eq!(c.racines, vec!["/Musique", "/NAS"]);
        assert_eq!(c.exclusions, vec!["/Musique/x.flac"]);
    }

    #[test]
    fn un_json_valide_mais_sans_les_cles_ne_ramene_rien_et_ne_se_plaint_pas() {
        for c in ["{}", "null", "[]", "{\"autre\":1}"] {
            let lu = analyser(c).unwrap_or_else(|e| panic!("{c:?} est du JSON valide : {e}"));
            assert_eq!(lu, super::Config::default(), "{c:?}");
        }
    }

    /// **Le test qui manquait.** V1 écrivait ses JSON sans écriture atomique : un fichier
    /// coupé en deux est le cas réel. Il ne doit pas passer pour « aucune exclusion ».
    #[test]
    fn un_json_illisible_est_une_erreur_et_non_deux_listes_vides() {
        for c in ["", "pas du json", "{\"library_paths\":[\"/Musi"] {
            let e = analyser(c).expect_err("doit être une erreur");
            assert!(matches!(e, Erreur::ConfigIllisible { .. }), "{c:?} : {e}");
            assert!(e.to_string().contains("retentera"), "l'erreur doit dire la suite : {e}");
        }
    }

    #[test]
    fn une_cle_qui_nest_pas_un_tableau_de_chaines_est_ignoree() {
        // V1 écrivait `"sentry_enabled": null`. Un JSON réel n'a pas la forme qu'on croit.
        let c = analyser(&config("\"pas-un-tableau\"", "[1,2,null,\"/vrai.flac\"]")).expect("lu");
        assert_eq!(c.racines, Vec::<String>::new());
        assert_eq!(c.exclusions, vec!["/vrai.flac"]);
    }
}
