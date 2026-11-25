//! Reprendre les **sources réseau** que V1 gardait dans `network_sources.json`.
//!
//! Même raisonnement que `library::reprise_v1`, et la même discipline d'erreur : la
//! migration de l'ADR 0003 déplace les octets, mais V2 ne lit aucun JSON de V1 à l'usage.
//! Sans cette reprise, Thomas trouverait au premier lancement une liste de sources **vide**,
//! et devrait resaisir à la main un hôte, un partage, un chemin et un utilisateur — et
//! surtout, **une source recréée reçoit un nouvel identifiant**, ce qui rendrait orphelines
//! les 3 137 URI de sa bibliothèque et celles de ses playlists.
//!
//! D'où la règle du module : **l'identifiant de V1 est conservé tel quel.** C'est la seule
//! chose ici qui ne peut pas se rattraper plus tard.
//!
//! Ce qui n'est **pas** repris : `last_connected` et `last_scan`. Ce sont des traces de ce
//! que V1 a fait, pas des réglages ; les reprendre ferait croire à V2 qu'elle a déjà scanné.

use crate::library::{Depot, SourceReseau};

/// Le sujet, tel qu'il est noté dans la table `reprises`.
const SUJET: &str = "network_sources.json de V1";

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    /// Le JSON ne se lit pas. **Le marqueur n'est pas posé** : la reprise sera retentée au
    /// démarrage suivant. V1 écrivait ses six fichiers JSON sans écriture atomique — un
    /// fichier tronqué est le cas réel, pas l'exception — et conclure « cet utilisateur
    /// n'avait aucune source réseau » d'un fichier qu'on n'a pas su lire lui cacherait 91 %
    /// de sa bibliothèque, **définitivement**, puisque le marqueur interdirait toute
    /// nouvelle tentative.
    #[error("le network_sources.json de V1 ne se lit pas comme du JSON ({source}) — aucune source réseau n'a été reprise, on retentera au prochain démarrage")]
    Illisible {
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Depot(#[from] crate::library::repository::Erreur),
}

/// Ce qu'une reprise a ramené.
#[derive(Debug, Default, PartialEq)]
pub struct Reprise {
    pub sources: u32,
    /// Les entrées sautées parce qu'il leur manquait un identifiant, un hôte ou un partage.
    /// Comptées plutôt que fatales : une source abîmée ne doit pas faire perdre les autres.
    pub incompletes: u32,
    pub deja_faite: bool,
}

/// La forme exacte d'une entrée de `network_sources.json`, telle que V1 l'écrit.
///
/// Les noms de champs sont ceux de V1 et **ne se traduisent pas** : c'est un format de
/// fichier existant, pas un choix. Tout est `Option` parce qu'un JSON réel n'a pas la forme
/// qu'on croit — la leçon de `"sentry_enabled": null` dans le `config.json`.
#[derive(serde::Deserialize)]
struct EntreeV1 {
    id: Option<String>,
    name: Option<String>,
    host: Option<String>,
    share: Option<String>,
    remote_path: Option<String>,
    credentials: Option<IdentifiantsV1>,
    enabled: Option<bool>,
}

#[derive(serde::Deserialize)]
struct IdentifiantsV1 {
    username: Option<String>,
    domain: Option<String>,
    is_guest: Option<bool>,
}

/// Reprend les sources réseau de V1, une seule fois.
pub fn reprendre(depot: &mut Depot, json: &str) -> Result<Reprise, Erreur> {
    if depot.reprise_deja_faite(SUJET)? {
        return Ok(Reprise { deja_faite: true, ..Default::default() });
    }
    let (sources, incompletes) = analyser(json)?;
    for s in &sources {
        depot.enregistrer_source_reseau(s)?;
    }
    // Le marqueur en dernier : si l'application meurt au milieu, la reprise recommence au
    // lancement suivant, et chaque enregistrement est idempotent (la clé est l'identifiant
    // de V1, pas un nouveau).
    depot.noter_la_reprise(SUJET)?;
    let reprise = Reprise {
        sources: u32::try_from(sources.len()).unwrap_or(u32::MAX),
        incompletes,
        deja_faite: false,
    };
    tracing::info!(?reprise, "sources réseau de V1 reprises");
    Ok(reprise)
}

/// Lit les sources d'un `network_sources.json` de V1, et compte celles qu'il a fallu sauter.
///
/// **Rend une erreur si le JSON ne se parse pas**, et c'est la distinction qui compte :
/// « lu, rien dedans » n'est pas « pas lisible ». La première conclusion est légitime, la
/// seconde ne doit jamais être tirée.
pub fn analyser(json: &str) -> Result<(Vec<SourceReseau>, u32), Erreur> {
    let lues: Vec<EntreeV1> =
        serde_json::from_str(json).map_err(|source| Erreur::Illisible { source })?;
    let mut sources = Vec::new();
    let mut incompletes = 0;
    for e in lues {
        let creds = e.credentials;
        match (e.id, e.host, e.share) {
            (Some(id), Some(hote), Some(partage))
                if !id.trim().is_empty() && !hote.trim().is_empty() =>
            {
                sources.push(SourceReseau {
                    nom: e.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| hote.clone()),
                    id,
                    hote,
                    partage,
                    chemin_distant: e.remote_path.unwrap_or_else(|| "/".to_string()),
                    utilisateur: creds
                        .as_ref()
                        .and_then(|c| c.username.clone())
                        .unwrap_or_default(),
                    domaine: creds
                        .as_ref()
                        .and_then(|c| c.domain.clone())
                        .filter(|d| !d.trim().is_empty()),
                    invite: creds.as_ref().and_then(|c| c.is_guest).unwrap_or(false),
                    // Une source désactivée dans V1 le reste : c'est un réglage de
                    // l'utilisateur, pas un état interne.
                    active: e.enabled.unwrap_or(true),
                });
            }
            _ => incompletes += 1,
        }
    }
    Ok((sources, incompletes))
}

#[cfg(test)]
mod tests {
    use super::{analyser, Erreur};

    /// Le `network_sources.json` **réel** de Thomas, copié octet pour octet de son dossier
    /// de données migré le 2026-11-25. C'est la forme sur laquelle cette reprise doit
    /// marcher ; une forme idéale écrite à la main ne prouverait rien.
    const REEL: &str = r#"[
  {
    "id": "dcdd24af-ca15-48a4-adde-18b5fcfa92f0",
    "name": "dugstation",
    "host": "dugstation.local",
    "share": "music",
    "remote_path": "/LOSSLESS/complete",
    "credentials": {
      "username": "admin",
      "domain": null,
      "is_guest": false
    },
    "enabled": true,
    "last_connected": null,
    "last_scan": null
  }
]"#;

    #[test]
    fn la_source_reelle_de_thomas_se_relit_champ_par_champ() {
        let (sources, incompletes) = analyser(REEL).expect("du JSON réel doit se lire");
        assert_eq!(incompletes, 0);
        assert_eq!(sources.len(), 1);
        let s = &sources[0];
        // L'identifiant d'abord : c'est lui qui est écrit dans ses 3 137 URI de pistes.
        assert_eq!(s.id, "dcdd24af-ca15-48a4-adde-18b5fcfa92f0");
        assert_eq!(s.nom, "dugstation");
        assert_eq!(s.hote, "dugstation.local");
        assert_eq!(s.partage, "music");
        assert_eq!(s.chemin_distant, "/LOSSLESS/complete");
        assert_eq!(s.utilisateur, "admin");
        assert_eq!(s.domaine, None, "`domain: null` n'est pas un domaine vide");
        assert!(!s.invite);
        assert!(s.active);
    }

    /// Le test qui manquait à la reprise du `config.json` jusqu'au 2026-11-23, écrit ici du
    /// premier coup : un fichier tronqué n'est pas « aucune source ».
    #[test]
    fn un_json_illisible_est_une_erreur_et_non_une_liste_vide() {
        for json in ["", "pas du json", "[{\"id\":\"abc\",\"host\":\"du", "{}"] {
            let e = analyser(json).expect_err("doit être une erreur");
            assert!(matches!(e, Erreur::Illisible { .. }), "{json:?} : {e}");
            assert!(e.to_string().contains("retentera"), "l'erreur dit la suite : {e}");
        }
    }

    #[test]
    fn une_liste_vide_est_une_liste_vide() {
        let (sources, incompletes) = analyser("[]").expect("lu");
        assert!(sources.is_empty());
        assert_eq!(incompletes, 0);
    }

    /// Une entrée sans identifiant, sans hôte ou sans partage est **sautée et comptée** : on
    /// ne peut ni la joindre ni reconnaître ses pistes. L'inventer serait pire que l'omettre.
    #[test]
    fn une_entree_incomplete_est_sautee_sans_faire_perdre_les_autres() {
        let json = r#"[
          {"name":"sans id","host":"a.local","share":"m"},
          {"id":"   ","host":"b.local","share":"m"},
          {"id":"ok","host":"c.local","share":"m"}
        ]"#;
        let (sources, incompletes) = analyser(json).expect("lu");
        assert_eq!(incompletes, 2);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "ok");
    }

    /// Les replis quand un champ optionnel manque. Le nom retombe sur l'hôte plutôt que sur
    /// une chaîne vide : une source sans nom dans une liste est inutilisable à l'écran.
    #[test]
    fn les_champs_absents_ont_des_replis_utilisables() {
        let json = r#"[{"id":"x","host":"nas.local","share":"music"}]"#;
        let (sources, _) = analyser(json).expect("lu");
        let s = &sources[0];
        assert_eq!(s.nom, "nas.local");
        assert_eq!(s.chemin_distant, "/");
        assert_eq!(s.utilisateur, "");
        assert!(!s.invite);
        assert!(s.active, "une source sans `enabled` est active");
    }

    /// Et une source que V1 avait désactivée le reste : c'est un réglage de l'utilisateur.
    #[test]
    fn une_source_desactivee_dans_v1_le_reste() {
        let json = r#"[{"id":"x","host":"n.local","share":"m","enabled":false}]"#;
        let (sources, _) = analyser(json).expect("lu");
        assert!(!sources[0].active);
    }
}
