//! Ce qui se passe entre le lancement et la première fenêtre : migrer le dossier de V1 s'il
//! y en a un, puis ouvrir la bibliothèque, puis reprendre la configuration de V1.
//!
//! **Pourquoi un module, et pas six lignes dans `run()`.** Parce que l'ordre de ces trois
//! étapes est lui-même un invariant, et qu'un invariant qui vit dans `run()` n'est pas
//! testable : `run()` lance Tauri. C'est exactement ce qui a laissé passer le défaut
//! ci-dessous — un test appelait la migration deux fois de suite, sans l'ouverture de la base
//! que `run()` faisait entre les deux, et affirmait donc une propriété que le code n'avait
//! pas. Ici, la séquence est une fonction, et un test la rejoue en entier.
//!
//! ## Le défaut corrigé le 2026-11-23 : un échec de migration devenait définitif
//!
//! `run()` ouvrait la bibliothèque **quoi qu'il arrive**, y compris juste après un échec de
//! migration. Or ouvrir la bibliothèque crée le dossier de données et le fichier de base.
//! Donc : un seul fichier de pochette illisible, ou un disque momentanément plein, et le
//! lancement suivant trouvait un dossier de données « déjà peuplé » — par la base vide que
//! V2 venait de créer lui-même — et n'essayait **plus jamais** de migrer. Les 572 Mo du
//! dossier de V1 restaient en place, invisibles, et la liste d'exclusion n'étant jamais
//! reprise, les pistes supprimées revenaient au premier scan : **INV-06 (🔴) violé à travers
//! la migration**, et de façon permanente.
//!
//! Deux règles en sortent, et ce sont les deux que ce module tient :
//!
//! 1. **tant qu'une migration reste à faire, rien n'est créé dans le dossier de données.**
//!    Un échec de migration empêche donc le démarrage — l'ADR 0012 remplace sur ce point la
//!    conséquence acceptée de l'ADR 0010. Ne pas démarrer est rattrapable : les données sont
//!    intactes, le message dit quoi faire, et le lancement suivant retente. Démarrer, non ;
//! 2. **une bibliothèque vierge n'est pas une bibliothèque.** Un dossier de données qui ne
//!    contient qu'une base sans aucune piste, sans racine et sans exclusion est le reliquat
//!    d'un démarrage, pas le travail de quelqu'un : il est écarté au lieu de bloquer la
//!    migration. C'est ce qui rattrape les machines — celle de Thomas, celles des testeurs
//!    ayant reçu une build intermédiaire — où V2 a déjà tourné une fois avant d'avoir cette
//!    migration.
//!
//! Ce qui ne change pas : **deux bibliothèques ne se fusionnent jamais** (ADR 0010 §6). Une
//! base qui contient quoi que ce soit de réel arrête la migration, et le dit.

use std::path::Path;

use crate::library::{dossier, reprise_v1, Depot};
use crate::migration::{self, Etat};

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    /// Fatal **par choix** : voir la règle 1 en tête de module.
    ///
    /// Le message est écrit pour être lu par quelqu'un qui ne lit pas de code, et il **nomme
    /// le dossier** où les données sont restées : « dans leur dossier d'origine » n'aide
    /// personne à le retrouver.
    ///
    /// **Mais il n'a pas encore de fenêtre pour s'afficher**, et c'est une réserve relevée en
    /// relecture qu'il vaut mieux écrire que taire : l'échec se produit avant que Tauri ne
    /// démarre, donc il n'y a ni fenêtre ni dialogue, et `observability` n'écrit aujourd'hui
    /// que sur stdout — la journalisation dans un fichier est datée de la phase 6. Pour un
    /// testeur qui double-clique l'application, elle rebondit dans le Dock et s'arrête, sans
    /// un mot visible. C'est le mode de défaut d'INV-21 appliqué au message censé le réparer.
    /// Le choix de ne pas démarrer reste le bon — les données sont intactes et relancer
    /// réessaie — mais **le canal manque**, et il arrive avec la phase 6.
    #[error(
        "la migration de vos données depuis l'ancienne version a échoué, et Hean n'a donc \
         rien créé : vos données sont intactes dans {ancien}. Relancer Hean réessaiera. \
         Cause : {source}"
    )]
    Migration {
        ancien: String,
        #[source]
        source: migration::Erreur,
    },
    #[error("ouverture de la bibliothèque {base} : {source}")]
    Base {
        base: String,
        #[source]
        source: crate::library::repository::Erreur,
    },
}

/// Migre, ouvre la bibliothèque, reprend la configuration de V1. Rend le dépôt prêt à
/// servir, ou une erreur sur laquelle l'application ne doit **pas** démarrer.
pub fn preparer(ancien: &Path, dossier_de_donnees: &Path) -> Result<Depot, Erreur> {
    if migration_a_faire(ancien, dossier_de_donnees) {
        ecarter_une_bibliotheque_vierge(dossier_de_donnees)?;
        match migration::migrer_si_necessaire(ancien, dossier_de_donnees) {
            Ok(rapport) => {
                if rapport.etat == Etat::CibleDejaPeuplee {
                    // Ce n'est plus le cas d'une base vierge — elle vient d'être écartée —
                    // donc c'est une vraie bibliothèque V2. On ne fusionne pas, et on le dit
                    // assez fort pour que ça se retrouve dans un journal envoyé.
                    tracing::error!(
                        ancien = %ancien.display(),
                        "des données de l'ancienne version sont là, mais Hean a déjà une \
                         bibliothèque à lui : rien n'a été migré, et rien n'a été touché"
                    );
                } else {
                    tracing::info!(?rapport, "migration des données de V1");
                }
            }
            Err(source) => {
                return Err(Erreur::Migration {
                    ancien: ancien.display().to_string(),
                    source,
                })
            }
        }
    }

    let base = dossier::base_dans(dossier_de_donnees);
    let mut depot = Depot::ouvrir(&base).map_err(|source| Erreur::Base {
        base: base.display().to_string(),
        source,
    })?;

    reprendre_la_configuration(&mut depot, dossier_de_donnees);
    Ok(depot)
}

/// Vrai s'il y a un dossier de V1 et que la migration n'a pas encore eu lieu.
fn migration_a_faire(ancien: &Path, dossier_de_donnees: &Path) -> bool {
    ancien.is_dir() && !dossier_de_donnees.join(migration::inventaire::TEMOIN).exists()
}

/// Supprime la base si elle ne contient rien — voir la règle 2 en tête de module.
///
/// « Rien » se demande au dépôt, qui le **dérive du schéma** (`Depot::est_vierge`).
///
/// Première version relevée en relecture : elle consultait trois listes nommées ici — les
/// pistes, les racines, les exclusions. C'était juste à la date d'écriture et **faux en
/// silence** le jour où la phase 3 ajoute `playlists`, `favoris` ou `historique` : une base
/// contenant trois playlists et aucune piste aurait été déclarée vierge, donc **effacée**.
/// Exactement le défaut d'INV-06 — « n'oublie pas de répéter ça partout » — mais sur un
/// chemin qui supprime. La question est maintenant posée au schéma, pas à notre mémoire.
fn ecarter_une_bibliotheque_vierge(dossier_de_donnees: &Path) -> Result<(), Erreur> {
    let base = dossier::base_dans(dossier_de_donnees);
    if !base.exists() {
        return Ok(());
    }
    let echec = |source| Erreur::Base { base: base.display().to_string(), source };
    let vierge = {
        let depot = Depot::ouvrir(&base).map_err(echec)?;
        depot.est_vierge().map_err(echec)?
    };
    if !vierge {
        return Ok(());
    }
    // Les trois fichiers d'une base SQLite. Un échec de suppression n'est pas fatal : la
    // migration constatera simplement que la cible est peuplée, ce qui est l'ancien
    // comportement — jamais une perte.
    for suffixe in ["", "-wal", "-shm"] {
        let f = base.with_file_name(format!(
            "{}{suffixe}",
            base.file_name().unwrap_or_default().to_string_lossy()
        ));
        if f.exists() {
            if let Err(e) = std::fs::remove_file(&f) {
                tracing::warn!(fichier = %f.display(), erreur = %e, "bibliothèque vierge non effacée");
            }
        }
    }
    tracing::info!(
        "une bibliothèque vide a été écartée pour laisser la migration de V1 se faire"
    );
    Ok(())
}

/// Reprend la configuration de V1. **Jamais fatal** : l'application démarre, et l'erreur est
/// journalisée. Un `config.json` illisible n'est pas marqué comme repris, donc le démarrage
/// suivant retentera (voir `reprise_v1`).
fn reprendre_la_configuration(depot: &mut Depot, dossier_de_donnees: &Path) {
    match std::fs::read_to_string(dossier_de_donnees.join("config.json")) {
        Ok(config) => match reprise_v1::reprendre(depot, &config) {
            Ok(reprise) => tracing::info!(?reprise, "reprise de la configuration de V1"),
            Err(e) => tracing::error!(erreur = %e, "la reprise de la configuration de V1 a échoué"),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("pas de config.json de V1 : rien à reprendre");
        }
        Err(e) => tracing::error!(erreur = %e, "config.json de V1 illisible"),
    }
}
