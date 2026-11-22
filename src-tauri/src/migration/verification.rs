//! La vérification d'une copie. **C'est la pièce qui autorise la suppression de l'original.**
//!
//! L'ADR 0003 l'écrit en une phrase : « copie puis vérification puis suppression de
//! l'ancien, pas un `mv` sec ». La raison tient en deux temps : un `mv` entre deux dossiers
//! du même volume est instantané et atomique, donc tentant — mais il ne laisse **aucun
//! moyen de revenir** si V2 se révèle incapable de lire ce qu'il a déplacé. Une copie
//! vérifiée, elle, garde l'original jusqu'à la preuve que l'arrivée est complète.
//!
//! Ce qui est vérifié, et ce qui ne l'est pas, est un arbitrage assumé (`inventaire.rs`) :
//! chaque fichier par sa **taille**, et les fichiers irremplaçables par leur **contenu**.
//! Une pochette perdue se retélécharge ; une playlist perdue ne revient pas.

use std::path::{Path, PathBuf};

use super::inventaire;

/// Un écart entre la source et la copie. Nommé, parce qu'un « la vérification a échoué »
/// n'apprend rien à qui doit comprendre ce qui est arrivé à ses données.
#[derive(Debug, PartialEq)]
pub enum Ecart {
    Manquant(PathBuf),
    Taille { fichier: PathBuf, attendu: u64, trouve: u64 },
    Contenu(PathBuf),
    Illisible { fichier: PathBuf, erreur: String },
}

impl std::fmt::Display for Ecart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ecart::Manquant(p) => write!(f, "{} n'a pas été copié", p.display()),
            Ecart::Taille { fichier, attendu, trouve } => write!(
                f,
                "{} fait {trouve} octets au lieu de {attendu}",
                fichier.display()
            ),
            Ecart::Contenu(p) => write!(f, "{} a la bonne taille mais pas le même contenu", p.display()),
            Ecart::Illisible { fichier, erreur } => {
                write!(f, "{} est illisible : {erreur}", fichier.display())
            }
        }
    }
}

/// Compare la copie à la source, fichier par fichier. Rend **tous** les écarts, pas le
/// premier : si une migration a mal tourné, Thomas a besoin de l'étendue du problème, pas
/// d'un seul symptôme.
pub fn verifier(source: &Path, copie: &Path, relatifs: &[PathBuf]) -> Vec<Ecart> {
    let mut ecarts = Vec::new();
    for relatif in relatifs {
        let origine = source.join(relatif);
        let arrivee = copie.join(relatif);

        let taille_origine = match std::fs::metadata(&origine) {
            Ok(m) => m.len(),
            Err(e) => {
                ecarts.push(Ecart::Illisible { fichier: relatif.clone(), erreur: e.to_string() });
                continue;
            }
        };
        let taille_arrivee = match std::fs::metadata(&arrivee) {
            Ok(m) => m.len(),
            Err(_) => {
                ecarts.push(Ecart::Manquant(relatif.clone()));
                continue;
            }
        };
        if taille_origine != taille_arrivee {
            ecarts.push(Ecart::Taille {
                fichier: relatif.clone(),
                attendu: taille_origine,
                trouve: taille_arrivee,
            });
            continue;
        }
        // Le contenu, pour les seuls fichiers qu'on ne peut pas reconstruire. Une taille
        // identique ne prouve pas un contenu identique : un secteur abîmé ne change pas la
        // longueur d'un fichier.
        if inventaire::a_verifier_au_contenu(relatif) {
            match (std::fs::read(&origine), std::fs::read(&arrivee)) {
                (Ok(a), Ok(b)) if a != b => ecarts.push(Ecart::Contenu(relatif.clone())),
                (Err(e), _) | (_, Err(e)) => {
                    ecarts.push(Ecart::Illisible { fichier: relatif.clone(), erreur: e.to_string() });
                }
                _ => {}
            }
        }
    }
    ecarts
}

#[cfg(test)]
mod tests {
    use super::{verifier, Ecart};
    use std::path::{Path, PathBuf};

    /// Deux dossiers jumeaux dans un bac temporaire. Rend (source, copie).
    fn bac(nom: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("hean-verif-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (s, c) = (base.join("source"), base.join("copie"));
        std::fs::create_dir_all(&s).expect("source");
        std::fs::create_dir_all(&c).expect("copie");
        (s, c)
    }

    fn ecrire(dossier: &Path, relatif: &str, contenu: &[u8]) {
        let p = dossier.join(relatif);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("dossier");
        }
        std::fs::write(p, contenu).expect("écriture");
    }

    #[test]
    fn une_copie_fidele_ne_produit_aucun_ecart() {
        let (s, c) = bac("fidele");
        for d in [&s, &c] {
            ecrire(d, "playlists.json", b"[{\"nom\":\"Jazz\"}]");
            ecrire(d, "covers/pochette.jpg", b"des octets de pochette");
        }
        let relatifs = vec![PathBuf::from("playlists.json"), PathBuf::from("covers/pochette.jpg")];
        assert_eq!(verifier(&s, &c, &relatifs), vec![]);
    }

    #[test]
    fn un_fichier_absent_de_la_copie_est_vu() {
        let (s, c) = bac("absent");
        ecrire(&s, "playlists.json", b"[]");
        let relatifs = vec![PathBuf::from("playlists.json")];
        assert_eq!(verifier(&s, &c, &relatifs), vec![Ecart::Manquant(PathBuf::from("playlists.json"))]);
    }

    #[test]
    fn une_copie_tronquee_est_vue() {
        // Le cas réel : une copie interrompue. C'est ce que la vérification existe pour
        // attraper AVANT que l'original soit supprimé.
        let (s, c) = bac("tronquee");
        ecrire(&s, "tracks_cache.json", b"les douze mille pistes");
        ecrire(&c, "tracks_cache.json", b"les douze");
        let ecarts = verifier(&s, &c, &[PathBuf::from("tracks_cache.json")]);
        assert_eq!(
            ecarts,
            vec![Ecart::Taille {
                fichier: PathBuf::from("tracks_cache.json"),
                attendu: 22,
                trouve: 9
            }]
        );
    }

    #[test]
    fn un_contenu_different_a_taille_egale_est_vu_sur_un_fichier_irremplacable() {
        // Un secteur abîmé ne change pas la longueur d'un fichier : seule la comparaison
        // du contenu le voit. D'où la vérification au contenu pour ce qui ne se reconstruit
        // pas — et son absence pour les pochettes, qui se retéléchargent.
        let (s, c) = bac("contenu");
        ecrire(&s, "playlists.json", b"[{\"nom\":\"Jazz\"}]");
        ecrire(&c, "playlists.json", b"[{\"nom\":\"Rock\"}]");
        ecrire(&s, "covers/a.jpg", b"AAAA");
        ecrire(&c, "covers/a.jpg", b"BBBB");

        let ecarts = verifier(
            &s,
            &c,
            &[PathBuf::from("playlists.json"), PathBuf::from("covers/a.jpg")],
        );
        assert_eq!(ecarts, vec![Ecart::Contenu(PathBuf::from("playlists.json"))]);
    }

    #[test]
    fn tous_les_ecarts_sont_rendus_et_pas_seulement_le_premier() {
        let (s, c) = bac("tous");
        ecrire(&s, "a.json", b"aaa");
        ecrire(&s, "b.json", b"bbb");
        ecrire(&c, "b.json", b"bb");
        let ecarts = verifier(&s, &c, &[PathBuf::from("a.json"), PathBuf::from("b.json")]);
        assert_eq!(ecarts.len(), 2, "{ecarts:?}");
    }

    #[test]
    fn un_ecart_se_lit_en_francais() {
        let e = Ecart::Taille { fichier: PathBuf::from("x.json"), attendu: 10, trouve: 3 };
        assert_eq!(e.to_string(), "x.json fait 3 octets au lieu de 10");
    }
}
