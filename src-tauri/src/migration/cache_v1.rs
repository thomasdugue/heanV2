//! Le cache jetable de V1 : ce qu'il est, et pourquoi la migration le supprime alors qu'elle
//! garde tout le reste.
//!
//! **Deux décisions de Thomas, le 2026-11-25, et il faut les tenir ensemble** (ADR 0014) :
//!
//! - « Je fais garder ton dossier V1 au lieu de le supprimer ? » → **« Oui, garde-le »**. Le
//!   dossier de V1 reste en place comme retour arrière vers la V1 (INV-26).
//! - « Je fais supprimer le cache de 289 Mo en migrant, et garder le reste ? » → **« Oui, jette
//!   le cache »**. Ce module-ci.
//!
//! **Le périmètre est exactement un dossier, et c'est voulu.** Trois autres fichiers de `noir/`
//! portent le mot *cache* — `tracks_cache.json` (1,7 Mo), `added_dates_cache.json` (512 Ko),
//! `network_scan_cache.json` (2,2 Mo) — et ce ne sont **pas** des caches jetables : c'est l'état
//! de la bibliothèque V1. C'est `tracks_cache.json` qui porte les 3 443 pistes de Thomas, donc
//! c'est lui qui rend le retour arrière utile. Les supprimer viderait le filet qu'on vient de
//! décider de garder, pour 4 Mo. Thomas n'a pas arbitré ces trois-là, et ils ne se déduisent pas
//! de « jette le cache ».
//!
//! **Ce que `smb_buffer/` est, mesuré** : 289 Mio sur les 572 Mio apparents de `noir/`, en trois
//! fichiers FLAC téléchargés depuis le NAS le 21 novembre et nommés par empreinte. V1 les écrit
//! pendant la lecture progressive d'un fichier distant et les supprime elle-même ensuite ; ceux
//! de Thomas avaient quatre jours, donc ce ménage n'avait pas eu lieu. Les reperdre ne perd
//! rien : ils se retéléchargent depuis le NAS. C'est la seule chose de `noir/` dont on puisse
//! dire ça.
//!
//! **Et c'est tout le coût du retour arrière.** Les 275 Mio de `covers/`, eux, sont migrés donc
//! **clonés** : ils partagent leurs blocs avec `hean/` et ne sont pas comptés deux fois (voir
//! `verification.rs`). Une fois ce cache parti, garder `noir/` ne coûte presque plus rien.

use std::path::Path;

/// Le nom du dossier de cache, **écrit une seule fois dans le projet**.
///
/// `inventaire::IGNORES` le reprend d'ici au lieu de le réécrire : un dossier qui serait
/// « ignoré » sous une orthographe et « supprimé » sous une autre est la forme de défaut qu'INV-06
/// a coûtée à V1. Et c'est un chemin qui mène à un `remove_dir_all` chez cinquante personnes.
pub const CHEMIN: &str = "smb_buffer";

/// Supprime le cache jetable du dossier de V1 et rend les octets libérés.
///
/// **Nomme `CHEMIN` et rien d'autre.** Pas « les dossiers ignorés », pas « ce qui n'a pas été
/// migré » : si `inventaire::IGNORES` s'allongeait demain, le nouveau venu ne serait pas couvert
/// par la décision de Thomas, et une suppression chez quelqu'un ne s'étend pas par déduction.
///
/// Rend `Ok(0)` quand il n'y a rien à supprimer — un testeur qui n'a jamais utilisé de NAS n'a
/// pas ce dossier, et ce n'est pas une erreur.
pub fn jeter(source: &Path) -> std::io::Result<u64> {
    let cache = source.join(CHEMIN);
    if !cache.is_dir() {
        return Ok(0);
    }
    let octets = octets_de(&cache);
    std::fs::remove_dir_all(&cache)?;
    Ok(octets)
}

/// Ce que pèse un dossier, pour pouvoir dire combien on a rendu. Un fichier qu'on n'arrive pas à
/// mesurer compte pour zéro : ce nombre va dans un journal, il ne décide de rien.
fn octets_de(dossier: &Path) -> u64 {
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return 0;
    };
    entrees
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => octets_de(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{jeter, CHEMIN};
    use std::path::PathBuf;

    fn bac(nom: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("hean-cache-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join(CHEMIN)).expect("bac");
        base
    }

    #[test]
    fn le_cache_part_et_les_octets_sont_comptes() {
        let base = bac("part");
        std::fs::write(base.join(CHEMIN).join("a.flac"), b"douze octets").expect("a");
        std::fs::write(base.join(CHEMIN).join("b.flac"), b"quatre").expect("b");
        std::fs::write(base.join("playlists.json"), b"[]").expect("playlists");

        assert_eq!(jeter(&base).expect("suppression"), 18);
        assert!(!base.join(CHEMIN).exists(), "le cache est parti");
        assert!(base.join("playlists.json").exists(), "et RIEN D'AUTRE n'a bougé");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Un testeur qui n'a jamais branché de NAS n'a pas ce dossier. Ce n'est pas une erreur, et
    /// surtout ça ne doit pas faire échouer sa migration.
    #[test]
    fn pas_de_cache_nest_pas_une_erreur() {
        let base = bac("absent");
        std::fs::remove_dir_all(base.join(CHEMIN)).expect("retrait");
        assert_eq!(jeter(&base).expect("rien à faire"), 0);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn les_sous_dossiers_du_cache_sont_comptes_et_partent() {
        let base = bac("profond");
        std::fs::create_dir_all(base.join(CHEMIN).join("en-cours")).expect("sous-dossier");
        std::fs::write(base.join(CHEMIN).join("en-cours").join("c.part"), b"sept ..").expect("c");
        assert_eq!(jeter(&base).expect("suppression"), 7);
        assert!(!base.join(CHEMIN).exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
