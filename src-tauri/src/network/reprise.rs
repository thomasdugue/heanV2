//! **INV-03 (🔴) — une connexion SMB périmée se retente.**
//!
//! Un NAS ferme une session inactive, et `libsmbclient` ne s'en rend compte qu'à l'opération
//! suivante. La règle attendait la phase 5 et un conteneur Samba ; elle ne l'attend plus,
//! parce que la **réaction** à une connexion périmée est séparée de sa **cause** : rien ici
//! ne connaît SMB ni le réseau — deux fermetures, l'opération et le rafraîchissement — donc
//! tout s'éprouve par **injection de panne**, sans NAS allumé et sans conteneur.
//!
//! Sorti de `client.rs` le 2026-11-25, quand celui-ci a atteint 400 lignes. Le découpage
//! suit la frontière qui rendait déjà la règle testable, donc il ne coûte rien.

use super::client::Erreur;

/// Tente l'opération ; si elle échoue, rafraîchit la connexion et retente **une
/// seule fois**.
///
/// Une seule fois, et pas « jusqu'à ce que ça marche » : une boucle transformerait un NAS
/// éteint en attente infinie, et c'est le comportement W005 pris par le mauvais bout. Si la
/// seconde tentative échoue, c'est **son** erreur qui remonte — celle de la connexion
/// fraîche décrit la panne réelle, là où la première décrit une session périmée.
pub fn reessayer_une_fois<T>(
    operation: impl Fn() -> Result<T, Erreur>,
    rafraichir: impl Fn() -> Result<(), Erreur>,
) -> Result<T, Erreur> {
    match operation() {
        Ok(v) => Ok(v),
        Err(premiere) => {
            tracing::warn!(erreur = %premiere, "opération SMB en échec, reconnexion et seconde tentative");
            rafraichir()?;
            operation()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::reessayer_une_fois;
    use crate::network::client::Erreur;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn panne(quoi: &str) -> Erreur {
        Erreur::Dossier {
            hote: "nas".into(),
            chemin: "/".into(),
            detail: quoi.to_string(),
        }
    }

    /// INV-03, par **injection de panne** : la première tentative échoue, la connexion est
    /// rafraîchie, la seconde réussit. C'est le cas d'un NAS qui a fermé une session
    /// inactive — le plus courant, et celui que V1 ne vérifiait qu'avec un vrai NAS dans le
    /// bon état (ses tests `network_source` étaient `#[ignore]`).
    #[test]
    fn une_premiere_tentative_en_echec_est_retentee_apres_reconnexion() {
        let essais = AtomicU32::new(0);
        let reconnexions = AtomicU32::new(0);
        let r = reessayer_une_fois(
            || {
                if essais.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(panne("session périmée"))
                } else {
                    Ok(42)
                }
            },
            || {
                reconnexions.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        );
        assert_eq!(r.expect("la seconde tentative réussit"), 42);
        assert_eq!(essais.load(Ordering::SeqCst), 2, "exactement deux tentatives");
        assert_eq!(reconnexions.load(Ordering::SeqCst), 1, "une seule reconnexion");
    }

    /// Et **une seule fois**. Une boucle « jusqu'à ce que ça marche » transformerait un NAS
    /// éteint en attente infinie : c'est W005 pris par le mauvais bout.
    #[test]
    fn un_echec_persistant_ne_boucle_pas() {
        let essais = AtomicU32::new(0);
        let r: Result<(), Erreur> = reessayer_une_fois(
            || {
                essais.fetch_add(1, Ordering::SeqCst);
                Err(panne("NAS éteint"))
            },
            || Ok(()),
        );
        let e = r.expect_err("doit échouer");
        assert!(e.to_string().contains("NAS éteint"), "{e}");
        assert_eq!(essais.load(Ordering::SeqCst), 2, "deux tentatives, pas plus");
    }

    /// Une reconnexion qui échoue elle-même remonte **son** erreur, et la seconde tentative
    /// n'a pas lieu : sans ça, l'utilisateur lirait « session périmée » alors que la vraie
    /// cause est « mot de passe refusé ».
    #[test]
    fn une_reconnexion_impossible_remonte_sa_propre_cause() {
        let essais = AtomicU32::new(0);
        let r: Result<(), Erreur> = reessayer_une_fois(
            || {
                essais.fetch_add(1, Ordering::SeqCst);
                Err(panne("session périmée"))
            },
            || {
                Err(Erreur::Connexion {
                    hote: "nas".into(),
                    partage: "music".into(),
                    detail: "mot de passe refusé".into(),
                })
            },
        );
        let e = r.expect_err("doit échouer");
        assert!(e.to_string().contains("mot de passe refusé"), "{e}");
        assert_eq!(essais.load(Ordering::SeqCst), 1, "la seconde tentative n'a pas lieu");
    }

    /// Un succès du premier coup ne reconnecte pas. L'assertion qui compte : le chemin
    /// normal — 3 137 fichiers — ne paie pas la règle.
    #[test]
    fn un_succes_immediat_ne_reconnecte_jamais() {
        let reconnexions = AtomicU32::new(0);
        let r = reessayer_une_fois(
            || Ok(7),
            || {
                reconnexions.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        );
        assert_eq!(r.expect("ok"), 7);
        assert_eq!(reconnexions.load(Ordering::SeqCst), 0);
    }


}

