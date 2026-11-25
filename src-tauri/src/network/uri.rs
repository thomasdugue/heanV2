//! L'URI d'une piste distante. **Le format est hérité de V1 et ne se choisit pas** (ADR 0016).
//!
//! ```text
//! smb://{id de la source}/{partage}/{chemin dans le partage, sans slash initial}
//! ```
//!
//! Les 3 137 pistes réseau de Thomas portent déjà cette forme dans le cache de V1, migré
//! dans son dossier de données. En assembler une autre rendrait sa bibliothèque invisible
//! **deux fois** : en V2, qui ne reconnaîtrait pas ses chemins, et en revenant à V1, qui ne
//! reconnaîtrait pas les nôtres. C'est pour ça que cette poignée de fonctions a ses propres
//! tests avec ses **valeurs réelles** : un format d'URI ne se relit pas dans un diff.
//!
//! Le schéma lui-même n'est pas écrit ici : il vient de `library::scan::SCHEME_RESEAU`, qui
//! est le seul endroit du projet où il est nommé — la bibliothèque a besoin de le connaître
//! pour savoir qu'une piste n'est pas un fichier (INV-27), et deux copies finiraient par
//! diverger.

use std::path::PathBuf;

use crate::library::scan::{Trouvee, SCHEME_RESEAU};
use crate::library::SourceReseau;

/// Le préfixe commun à toutes les URI d'une source : `smb://{id}/`.
///
/// **Le slash final est la raison d'être de cette fonction.** Sans lui, le périmètre d'une
/// source dont l'identifiant commence comme celui d'une autre attraperait les pistes de sa
/// voisine — et un périmètre qui déborde efface les pistes du voisin (INV-27).
pub fn prefixe(source_id: &str) -> String {
    format!("{SCHEME_RESEAU}{source_id}/")
}

/// L'URI d'un fichier du partage. `chemin_dans_le_partage` est relatif à la racine du
/// partage et peut commencer par un slash : il est retiré, puisque le préfixe en porte un.
pub fn de_fichier(source_id: &str, partage: &str, chemin_dans_le_partage: &str) -> String {
    format!(
        "{}{partage}/{}",
        prefixe(source_id),
        chemin_dans_le_partage.trim_start_matches('/')
    )
}

/// Une piste distante vue comme une entrée de scan.
///
/// **Taille et date à zéro, et ce n'est pas un oubli** : `libsmbclient` ne les rend pas en
/// listant un dossier (ADR 0016 §4). Les demander coûterait un aller-retour réseau par
/// fichier. Zéro des deux côtés — dans la base comme ici — fait que la comparaison
/// d'empreintes du dépôt conclut « inchangée » sans avoir besoin d'un cas particulier pour
/// le réseau.
///
/// Le chemin est rangé dans un `PathBuf` qui contient une URI. C'est volontaire et c'est le
/// prix d'avoir **un seul** type de chemin dans la bibliothèque : `Path` ne normalise rien à
/// l'affichage, donc l'URI ressort au caractère près.
pub fn en_trouvee(uri: &str) -> Trouvee {
    Trouvee {
        chemin: PathBuf::from(uri),
        taille: 0,
        mtime_ns: 0,
    }
}

/// Un identifiant de source **dérivé** de ce qui la désigne, et non tiré au hasard.
///
/// Pourquoi pas un UUID : ajouter deux fois le même partage — un geste qu'on fait par
/// erreur plus souvent qu'à son tour — donnerait deux sources, donc deux jeux d'URI pour les
/// mêmes fichiers, donc une bibliothèque en double et des pistes orphelines à la première
/// suppression. Dérivé, le même partage rend toujours la même source.
///
/// La forme est lisible exprès : elle apparaît dans chaque chemin de piste, et un journal
/// qui dit `smb://dugstation-local_music_lossless-complete/…` se lit, là où un UUID
/// demande d'ouvrir la base. Tout ce qui n'est pas alphanumérique devient `-`, parce que le
/// résultat est un **segment d'URI** : un `/` dedans casserait le découpage.
///
/// Une source qui vient de V1 garde l'identifiant de V1 et ne passe jamais par ici.
pub fn identifiant_derive(hote: &str, partage: &str, chemin_distant: &str) -> String {
    let propre = |s: &str| {
        s.to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    };
    format!(
        "{}_{}_{}",
        propre(hote),
        propre(partage),
        propre(chemin_distant)
    )
    .trim_matches('_')
    .to_string()
}

/// Le dossier de départ d'un scan, dans le partage. Toujours un chemin absolu dans le
/// partage : `libsmbclient` veut `/` pour la racine, pas la chaîne vide.
pub fn depart(source: &SourceReseau) -> String {
    let c = source.chemin_distant.trim();
    if c.is_empty() || c == "/" {
        "/".to_string()
    } else {
        format!("/{}", c.trim_matches('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::{de_fichier, depart, en_trouvee, prefixe};
    use crate::library::scan::Perimetre;
    use crate::library::SourceReseau;

    /// La source **réelle** de Thomas, relevée dans son `network_sources.json` migré le
    /// 2026-11-25. Les tests travaillent sur ces valeurs-là et pas sur un exemple inventé :
    /// c'est le seul moyen que l'assemblage d'URI soit vérifié contre ce qui existe.
    fn dugstation() -> SourceReseau {
        SourceReseau {
            id: "dcdd24af-ca15-48a4-adde-18b5fcfa92f0".into(),
            nom: "dugstation".into(),
            hote: "dugstation.local".into(),
            partage: "music".into(),
            chemin_distant: "/LOSSLESS/complete".into(),
            utilisateur: "admin".into(),
            domaine: None,
            invite: false,
            active: true,
        }
    }

    /// **L'assertion la plus importante du module** : une URI assemblée par V2 est
    /// identique, caractère pour caractère, à celle que V1 a écrite dans son cache pour le
    /// même fichier. Le chemin de droite est copié d'une entrée réelle de
    /// `network_scan_cache.json`.
    #[test]
    fn une_uri_assemblee_par_v2_est_celle_que_v1_a_ecrite() {
        let s = dugstation();
        let attendu = "smb://dcdd24af-ca15-48a4-adde-18b5fcfa92f0/music/LOSSLESS/complete/\
                       To Pimp a Butterfly (2015)/Kendrick Lamar - To Pimp a Butterfly - 16 - Mortal Man.mp3";
        let chemin_dans_le_partage =
            "/LOSSLESS/complete/To Pimp a Butterfly (2015)/Kendrick Lamar - To Pimp a Butterfly - 16 - Mortal Man.mp3";
        assert_eq!(de_fichier(&s.id, &s.partage, chemin_dans_le_partage), attendu);
        // Et le slash initial du chemin distant ne doit pas créer un double slash.
        assert!(!de_fichier(&s.id, &s.partage, chemin_dans_le_partage).contains("//LOSSLESS"));
    }

    /// L'écart apparent du brief, résolu et épinglé : le préfixe relevé dans le cache de V1
    /// (`…/music/LOSSLESS`) est simplement **plus court** que `remote_path`
    /// (`/LOSSLESS/complete`). Les deux doivent rester vrais ensemble.
    #[test]
    fn le_prefixe_court_du_cache_de_v1_et_le_chemin_distant_ne_se_contredisent_pas() {
        let s = dugstation();
        let uri = de_fichier(&s.id, &s.partage, "/LOSSLESS/complete/album/x.flac");
        assert!(uri.starts_with("smb://dcdd24af-ca15-48a4-adde-18b5fcfa92f0/music/LOSSLESS"));
        assert!(uri.starts_with(&format!(
            "{}{}{}",
            prefixe(&s.id),
            s.partage,
            s.chemin_distant
        )));
    }

    #[test]
    fn le_perimetre_dune_source_contient_ses_uri_et_pas_celles_dune_autre() {
        let s = dugstation();
        let p = Perimetre::Source(prefixe(&s.id));
        assert!(p.contient(&de_fichier(&s.id, &s.partage, "/LOSSLESS/complete/x.flac")));
        assert!(!p.contient(&de_fichier("une-autre", &s.partage, "/LOSSLESS/complete/x.flac")));
    }

    #[test]
    fn le_depart_dun_scan_est_toujours_absolu_dans_le_partage() {
        assert_eq!(depart(&dugstation()), "/LOSSLESS/complete");
        let racine = SourceReseau { chemin_distant: String::new(), ..dugstation() };
        assert_eq!(depart(&racine), "/");
        let slash = SourceReseau { chemin_distant: "/".into(), ..dugstation() };
        assert_eq!(depart(&slash), "/");
        // Les slashs en trop sont normalisés, parce qu'un `network_sources.json` écrit à la
        // main en portera.
        let sale = SourceReseau { chemin_distant: "/LOSSLESS/complete/".into(), ..dugstation() };
        assert_eq!(depart(&sale), "/LOSSLESS/complete");
        let sans = SourceReseau { chemin_distant: "LOSSLESS".into(), ..dugstation() };
        assert_eq!(depart(&sans), "/LOSSLESS");
    }

    /// Un identifiant dérivé est **stable** et **sans slash**, et deux partages différents
    /// n'en partagent pas un. Le slash est la propriété critique : un `/` dans un
    /// identifiant décalerait le découpage de toutes les URI de la source.
    #[test]
    fn un_identifiant_derive_est_stable_lisible_et_sans_slash() {
        let a = super::identifiant_derive("dugstation.local", "music", "/LOSSLESS/complete");
        assert_eq!(a, "dugstation-local_music_lossless-complete");
        assert_eq!(a, super::identifiant_derive("DugStation.local", "Music", "/LOSSLESS/complete"));
        assert!(!a.contains('/'), "un / casserait le découpage des URI");
        // Deux chemins différents sur le même partage sont deux sources.
        assert_ne!(a, super::identifiant_derive("dugstation.local", "music", "/LOSSY"));
        // Et le périmètre dérivé reste étanche.
        let p = Perimetre::Source(prefixe(&a));
        assert!(p.contient(&de_fichier(&a, "music", "/LOSSLESS/complete/x.flac")));
        assert!(!p.contient(&de_fichier("dcdd24af", "music", "/LOSSLESS/complete/x.flac")));
    }

    #[test]
    fn une_piste_distante_a_une_empreinte_nulle_des_deux_cotes() {
        let t = en_trouvee("smb://abc/music/x.flac");
        assert_eq!(t.taille, 0);
        assert_eq!(t.mtime_ns, 0);
        // L'URI ressort au caractère près : `Path` ne normalise rien à l'affichage.
        assert_eq!(t.chemin.display().to_string(), "smb://abc/music/x.flac");
    }
}
