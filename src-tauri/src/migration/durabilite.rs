//! Forcer sur le disque ce qui vient d'être copié, **avant** de supprimer l'original.
//!
//! ## Le défaut, relevé en relecture le 2026-11-23 et corrigé le 2026-11-25
//!
//! La migration copiait, vérifiait, échangeait, puis supprimait `noir/` — dans le bon ordre,
//! et sans jamais rien forcer sur le disque. Or `std::fs::copy` et `std::fs::rename` rendent
//! la main dès que le **cache du système** est à jour, pas quand le disque l'est. Entre les
//! deux il y a une fenêtre de quelques secondes, sans borne garantie.
//!
//! Ce que cela coûte : la machine s'éteint brutalement — panique noyau, coupure de courant,
//! batterie à plat sur un Mac dont le câble a sauté — dans les secondes qui suivent la
//! migration. Au redémarrage, `hean/` est là avec son témoin, donc la migration se croit
//! faite ; mais les fichiers qu'elle contient peuvent être vides ou absents, et `noir/` a
//! été supprimé. **Les playlists, l'historique d'écoute et les sources NAS sont perdus, et
//! aucun code ne saura plus qu'ils ont existé.**
//!
//! La probabilité est faible. L'exposition, elle, est maximale : ce chemin ne s'exécute
//! **qu'une seule fois** par testeur, juste après une mise à jour, c'est-à-dire précisément
//! au moment où l'on redémarre souvent sa machine. Et il n'y a pas de deuxième chance :
//! l'original a disparu. C'est INV-25.
//!
//! ## Pourquoi deux niveaux de synchronisation, et pas un
//!
//! `File::sync_data` est, sur macOS, le `fsync` ordinaire : il remet les pages sales au
//! périphérique. `File::sync_all` y est `fcntl(F_FULLFSYNC)` : il demande en plus au disque
//! de vider **son propre cache** vers le support — c'est la seule forme qui résiste à une
//! coupure de courant, et elle est bien plus lente.
//!
//! D'où la forme choisie : `sync_data` sur chaque fichier (les octets partent au disque),
//! puis `sync_all` sur chaque dossier **après son contenu** (son entrée devient durable, et
//! sans elle les fichiers ne seraient pas retrouvables), le dossier racine en dernier — donc
//! un `F_FULLFSYNC` final, qui vide le cache du périphérique pour tout ce qui précède.
//!
//! Mesuré le 2026-11-25 sur une copie du dossier réel de Thomas (2 443 fichiers, 283 Mo) :
//! voir `docs/HISTORY.md`. Si ce coût devenait un problème, ce qui se négocie est la liste
//! des fichiers synchronisés — jamais l'ordre « durable, puis supprimer ».

use std::fs::File;
use std::io;
use std::path::Path;

/// Rend durable tout le contenu de `dossier`. **Au retour, et seulement alors, ce qui est
/// dedans survivra à une coupure de courant.**
///
/// Le temps passé est journalisé, et ce n'est pas de la décoration : c'est le seul coût que
/// cette règle ajoute au premier lancement d'un testeur, et la seule façon de savoir s'il
/// reste raisonnable sur une machine qui n'est pas celle de Thomas.
pub fn rendre_durable(dossier: &Path) -> io::Result<()> {
    let debut = std::time::Instant::now();
    let fichiers = descendre(dossier)?;
    tracing::info!(
        fichiers,
        millisecondes = debut.elapsed().as_millis(),
        "copie forcée sur le disque"
    );
    Ok(())
}

fn descendre(dossier: &Path) -> io::Result<u32> {
    let mut fichiers = 0u32;
    for entree in std::fs::read_dir(dossier)? {
        let entree = entree?;
        // Le type vient de l'entrée de dossier et non d'un `is_dir()` sur le chemin : celui-ci
        // suit les liens symboliques, et la migration n'en copie aucun (ils sont comptés comme
        // ignorés). Ce qui n'est ni fichier ni dossier n'a donc rien à faire ici : on ne le
        // synchronise pas plutôt que de deviner.
        match entree.file_type() {
            Ok(t) if t.is_dir() => fichiers += descendre(&entree.path())?,
            Ok(t) if t.is_file() => {
                File::open(entree.path())?.sync_data()?;
                fichiers += 1;
            }
            _ => {}
        }
    }
    // Le dossier après son contenu. Un dossier s'ouvre en lecture seule comme un fichier, et
    // le synchroniser rend durables les entrées qui pointent vers ce qu'on vient d'écrire.
    File::open(dossier)?.sync_all()?;
    Ok(fichiers)
}

/// Rend durable le `rename` qui a mis le dossier en place : c'est l'entrée du **parent** qui
/// porte le nouveau nom, pas le dossier lui-même.
///
/// Appelée après l'échange et avant la suppression de l'original. Sans elle, le dossier
/// pourrait porter encore son nom provisoire sur le disque au moment où `noir/` disparaît —
/// et le démarrage suivant ne trouverait ni l'un ni l'autre.
pub fn rendre_durable_le_renommage(cible: &Path) -> io::Result<()> {
    let parent = cible.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} n'a pas de dossier parent", cible.display()),
        )
    })?;
    File::open(parent)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::{rendre_durable, rendre_durable_le_renommage};
    use std::path::PathBuf;

    fn bac(nom: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("hean-dur-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("bac");
        base
    }

    /// **Le test qui mesure, et non qui suppose.** Il y avait deux inconnues à la lecture de
    /// la documentation d'Apple : qu'un dossier s'ouvre bien avec `File::open`, et que
    /// `F_FULLFSYNC` l'accepte. Si l'une des deux était fausse, toute migration échouerait —
    /// au pire endroit du produit. Elles sont donc éprouvées, pas pariées.
    #[test]
    fn un_arbre_de_fichiers_se_rend_durable() {
        let base = bac("arbre");
        std::fs::create_dir_all(base.join("covers/plus/profond")).expect("dossiers");
        std::fs::write(base.join("playlists.json"), b"[{\"nom\":\"Jazz\"}]").expect("json");
        std::fs::write(base.join("covers/a.jpg"), b"AAAA").expect("jpg");
        std::fs::write(base.join("covers/plus/profond/b.jpg"), b"BBBB").expect("jpg");

        rendre_durable(&base).expect("un arbre réel doit se synchroniser");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Un dossier vide aussi : un `noir/` ne contenant que des fichiers ignorés ne copie rien.
    #[test]
    fn un_dossier_vide_se_rend_durable() {
        let base = bac("vide");
        rendre_durable(&base).expect("vide");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Et l'échec remonte : il doit empêcher la suppression de l'original, donc il ne peut pas
    /// être avalé.
    #[test]
    fn un_dossier_absent_est_une_erreur() {
        let base = bac("absent");
        let fantome = base.join("jamais-cree");
        assert!(rendre_durable(&fantome).is_err(), "un dossier absent ne se synchronise pas");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn le_renommage_se_rend_durable_par_le_parent() {
        let base = bac("renommage");
        let cible = base.join("hean");
        std::fs::create_dir_all(&cible).expect("cible");
        rendre_durable_le_renommage(&cible).expect("le parent existe");
        let _ = std::fs::remove_dir_all(&base);
    }
}
