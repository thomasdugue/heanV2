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
//! ⚠️ **Ce scénario a changé de gravité le 2026-11-25, et il faut le dire ici** : Thomas a
//! décidé que la migration ne supprime plus `noir/` (ADR 0014, INV-26). Les octets ne sont donc
//! plus perdus — ils sont dans l'ancien dossier, toujours là. Ce qui reste, et qui justifie
//! encore cette règle : le témoin certifierait une copie trouée, donc l'application démarrerait
//! sur une bibliothèque incomplète **sans jamais remigrer**, et son propriétaire n'aurait aucune
//! raison de soupçonner que ses données entières sont à côté. Réparable à la main par quelqu'un
//! qui sait ; invisible pour tous les autres.
//!
//! La probabilité est faible. L'exposition, elle, est maximale : ce chemin ne s'exécute
//! **qu'une seule fois** par testeur, juste après une mise à jour, c'est-à-dire précisément
//! au moment où l'on redémarre souvent sa machine. Et il n'y a pas de deuxième chance :
//! l'original a disparu. C'est INV-25.
//!
//! ## Deux appels, et ce qu'ils font vraiment sur macOS
//!
//! **Ce paragraphe disait le contraire pendant quelques heures, et la correction vaut la
//! lecture.** Il affirmait que `File::sync_data` serait le `fsync` ordinaire — les pages sales
//! poussées au périphérique — et que seul `File::sync_all` serait `fcntl(F_FULLFSYNC)`, la
//! barrière qui fait vider au disque son propre cache ; d'où une forme censée « finir » par un
//! `F_FULLFSYNC` global. **C'est faux.** Relevé en relecture, puis lu dans les sources de la
//! chaîne épinglée par l'ADR 0008 (`lib/rustlib/src/rust/library/std/src/sys/fs/unix.rs`,
//! Rust 1.93.1) : sous `#[cfg(target_vendor = "apple")]`, `os_fsync` **et** `os_datasync`
//! appellent l'un comme l'autre `libc::fcntl(fd, libc::F_FULLFSYNC)`.
//!
//! Donc, ici et aujourd'hui : **chaque fichier reçoit déjà une barrière disque complète.** Le
//! code est plus strict que ce qu'il annonçait, la sûreté n'était pas en jeu — mais le *pourquoi*
//! l'était, et un *pourquoi* faux fait optimiser sur un modèle mental inexact le jour où
//! quelqu'un voudra négocier ce coût (ce que cette règle autorise explicitement).
//!
//! Ce que les deux appels disent vraiment, du coup :
//!
//! - `sync_data` sur un **fichier** : « les octets doivent survivre, la date de modification
//!   m'est égale ». Sur Apple c'est `F_FULLFSYNC` ; ailleurs — la CI, un portage éventuel — c'est
//!   `fdatasync`, qui est le bon appel et le moins coûteux. On garde donc `sync_data` : il
//!   exprime l'intention, et il est correct des deux côtés ;
//! - `sync_all` sur un **dossier**, après son contenu : ce qu'on y cherche n'est pas le cache du
//!   périphérique, c'est l'**entrée de dossier**. Sans elle, les octets sont sur le disque et le
//!   fichier n'est retrouvable sous aucun nom.
//!
//! Et c'est ce qui explique la mesure, au lieu de la contredire : **90 ms** pour 2 444 fichiers
//! (une copie du dossier réel de Thomas, 283 Mo, le 2026-11-25 — le témoin compris, voir
//! `docs/HISTORY.md`). Un `F_FULLFSYNC` par fichier serait ruineux sur des fichiers fraîchement
//! écrits ; sur un **clone** APFS il n'y a presque aucune page sale à pousser. Si ce coût
//! devenait un problème, ce qui se négocie est la liste des fichiers synchronisés — jamais
//! l'ordre « durable, puis certifié ».

use std::fs::File;
use std::io;
use std::path::Path;

/// Rend durable tout le contenu de `dossier` : **au retour, ce qui est dedans survivra à une
/// coupure de courant.**
///
/// « Ce qui est dedans **au moment de l'appel** », et la nuance a été relevée en relecture : la
/// migration déplace ensuite les fichiers cachés du dossier d'arrivée dans celui-ci, et ceux-là
/// ne sont pas re-synchronisés. Ce sont des `.DS_Store` que le Finder réécrit, et `migration/mod.rs`
/// assume déjà qu'ils peuvent se perdre — mais la phrase, prise au mot, cesse d'être vraie à
/// l'instant du `rename`, et autant qu'elle le dise elle-même.
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
/// Appelée après l'échange. Elle a d'abord existé pour tenir la suppression de l'original :
/// sans elle, le dossier pouvait porter encore son nom provisoire sur le disque au moment où
/// `noir/` disparaissait. **Depuis que l'original survit (ADR 0014), elle ne garde plus rien
/// d'irréversible** — un renommage non durable fait seulement remigrer au lancement suivant,
/// depuis un `noir/` toujours en place. Elle reste parce que la bibliothèque doit être sur le
/// disque avant que l'application commence à y écrire, et parce qu'elle coûte un appel.
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

    /// Et l'échec remonte : il doit arrêter la migration avant que le témoin ne certifie une
    /// copie qui n'est pas sur le disque, donc il ne peut pas être avalé.
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
