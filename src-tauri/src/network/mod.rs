//! La part **réseau** de la bibliothèque : les partages SMB dont les pistes rejoignent les
//! pistes locales dans la même base (ADR 0016).
//!
//! 91 % de la bibliothèque de Thomas est là — 3 137 pistes sur 3 443
//! ([`docs/MESURES-V1.md`](../../../docs/MESURES-V1.md) §1). Ce n'est donc pas une annexe de
//! la bibliothèque, c'est sa plus grande partie.
//!
//! ```text
//! identifiants.rs   le mot de passe, dans le trousseau macOS (INV-04, ADR 0015)
//! client.rs         libsmbclient, derrière UN verrou de process (PLAT-2) + INV-03
//! uri.rs            smb://{id}/{partage}/{chemin} — le format hérité de V1
//! entete.rs         combien d'octets lire, et pour quels formats c'est légitime
//! inventaire.rs     le parcours différentiel — sans verrou de bibliothèque
//! reprise_v1.rs     les sources de V1, reprises une fois, identifiants conservés
//! ```
//!
//! **Ce module dépend de `library`, jamais l'inverse.** Une source réseau est une donnée de
//! bibliothèque (`library::types::SourceReseau`) ; ce qu'on en fait avec SMB est d'ici. Et
//! il n'écrit **jamais** dans la table des pistes : il rend un inventaire, et c'est le dépôt
//! qui décide ce qui entre (INV-06).
//!
//! Ce que ce module ne contient pas encore, et qui n'est pas un oubli : la **lecture audio**
//! depuis le NAS. Le backend de sortie CoreAudio n'existe pas — « le son n'est pas encore
//! branché » est écrit dans la fenêtre — donc du téléchargement progressif ne serait
//! démontrable par rien (ADR 0016, hors périmètre).

pub mod client;
pub mod entete;
pub mod identifiants;
pub mod inventaire;
pub mod reprise_v1;
pub mod uri;

use std::sync::Mutex;

use crate::library::scan::{Conclusion, Perimetre};
use crate::library::{Depot, RapportDeScan};

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error("aucune source réseau ne porte l'identifiant {0}")]
    SourceInconnue(String),
    #[error("la source réseau « {0} » est désactivée")]
    SourceDesactivee(String),
    #[error(transparent)]
    Identifiants(#[from] identifiants::Erreur),
    #[error(transparent)]
    Smb(#[from] client::Erreur),
    #[error(transparent)]
    Depot(#[from] crate::library::repository::Erreur),
}

/// Scanne une source réseau et met la bibliothèque à jour.
///
/// **La forme en trois temps est la décision de cette fonction** (ADR 0016 §5), et elle n'est
/// pas esthétique : le parcours SMB dure des minutes, et `libsmbclient` est un singleton de
/// process. Si le verrou de la bibliothèque était tenu pendant ce temps, `list_tracks`
/// resterait bloqué et la fenêtre gèlerait — c'est le comportement W005, pris par la cause.
///
/// 1. verrou **court** : lire la source et les chemins qu'il est inutile de relire ;
/// 2. **sans verrou** : parcourir le partage et lire les tags de ce qui est nouveau ;
/// 3. verrou **court** : appliquer, en une transaction, dans le périmètre de cette source.
///
/// Le rapport additionne les illisibles de l'inventaire à ceux de l'application : un fichier
/// distant dont les tags ne se lisent pas est compté à l'étape 2, pas à l'étape 3, et ne pas
/// l'additionner ferait annoncer « rien à signaler » après un scan qui a sauté des pistes.
pub fn scanner_une_source(
    bibliotheque: &Mutex<Depot>,
    source_id: &str,
) -> Result<RapportDeScan, Erreur> {
    scanner_avec(bibliotheque, source_id, |source| {
        Ok(Box::new(client::ClientSmb::nouveau(
            source,
            mot_de_passe_si_necessaire(source)?,
        )))
    })
}

/// Le mot de passe d'une source, **ou la chaîne vide si elle est en accès invité**.
///
/// Lu ICI et pas au démarrage : V009 — aucun dialogue de trousseau au lancement. Un échec
/// remonte une erreur, jamais un mot de passe vide (INV-04).
///
/// **Et pas du tout pour une source invité.** Lire le trousseau sans égard pour
/// `source.invite` rendait une source `is_guest` **impossible à scanner** : elle échouait sur
/// « aucun mot de passe pour la source … — l'ouvrir dans Hean et le saisir à nouveau »,
/// c'est-à-dire un message qui envoie chercher au mauvais endroit, pour un mot de passe que
/// `client.rs` n'emploie même pas quand `invite` est vrai. Chemin mort relevé par
/// `invariant-guardian`, et `network::reprise_v1` sait reprendre de telles sources depuis V1.
///
/// C'est une fonction et non deux lignes dans la fermeture ci-dessus pour qu'elle
/// s'éprouve : la fermeture, elle, fabrique un vrai client SMB, donc la tester voudrait dire
/// toucher un réseau.
fn mot_de_passe_si_necessaire(source: &crate::library::SourceReseau) -> Result<String, Erreur> {
    if source.invite {
        // Pas d'appel au trousseau du tout : il n'y a rien à y chercher, et le chercher
        // quand même déclencherait un dialogue macOS pour rien.
        return Ok(String::new());
    }
    Ok(identifiants::mot_de_passe(&source.id)?)
}

/// Le même scan, mais le **partage est fabriqué par l'appelant**. C'est la couture qui rend
/// la séquence de production éprouvable.
///
/// **Pourquoi elle existe, et c'est la leçon d'INV-24.** Les tests d'INV-27 réassemblaient
/// les trois temps à la main, parce que `scanner_une_source` fabrique son client et lit le
/// trousseau en interne. Résultat : la règle était éprouvée, mais son **câblage** ne l'était
/// pas — quelqu'un qui écrirait `Perimetre::Source(source_id.to_string())` sans le slash
/// final, ou `Conclusion::Complete` en dur, ferait disparaître les pistes d'une source
/// voisine ou celles d'un partage momentanément refusé, **et tous les tests resteraient
/// verts**. « Un test qui saute une étape de la séquence réelle n'éprouve pas le produit. »
/// Relevé par invariant-guardian.
///
/// La fabrique est appelée **après** les vérifications de l'étape 1 et **hors** du verrou :
/// c'est ce qui fait qu'une source inconnue ou désactivée ne demande aucun mot de passe.
pub fn scanner_avec<'a>(
    bibliotheque: &Mutex<Depot>,
    source_id: &str,
    partage: impl FnOnce(&crate::library::SourceReseau) -> Result<Box<dyn client::Smb + 'a>, Erreur>,
) -> Result<RapportDeScan, Erreur> {
    let perimetre = Perimetre::Source(uri::prefixe(source_id));

    // --- 1. verrou court -------------------------------------------------------------
    let (source, deja) = {
        let depot = verrou(bibliotheque);
        let source = depot
            .source_reseau(source_id)?
            .ok_or_else(|| Erreur::SourceInconnue(source_id.to_string()))?;
        if !source.active {
            return Err(Erreur::SourceDesactivee(source.nom));
        }
        let deja = inventaire::DejaConnu::depuis_le_depot(&depot, &perimetre)?;
        (source, deja)
    };

    // --- 2. sans verrou --------------------------------------------------------------
    let smb = partage(&source)?;
    let inventaire = inventaire::inventorier(smb.as_ref(), &source, &deja)?;

    // --- 3. verrou court -------------------------------------------------------------
    let mut rapport = {
        let mut depot = verrou(bibliotheque);
        // Un dossier que le NAS a refusé rend la conclusion partielle : aucune piste de
        // cette source n'est retirée ce tour-ci (B17). Sans ça, un partage momentanément
        // inaccessible effacerait les pistes qu'il contient.
        depot.appliquer(
            &inventaire.entrees,
            &perimetre,
            Conclusion::depuis(inventaire.dossiers_illisibles),
        )?
    };
    rapport.illisibles += inventaire.illisibles;
    tracing::info!(
        source = %source.nom,
        ?rapport,
        dossiers_illisibles = inventaire.dossiers_illisibles,
        "scan d'une source réseau terminé"
    );
    Ok(rapport)
}

/// Le verrou de la bibliothèque, en reprenant la donnée s'il est empoisonné. Même
/// raisonnement que dans les commandes : un thread qui a paniqué en le tenant ne doit pas
/// faire paniquer l'app à son tour.
fn verrou(bibliotheque: &Mutex<Depot>) -> std::sync::MutexGuard<'_, Depot> {
    bibliotheque.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::{scanner_une_source, Erreur};
    use crate::library::{Depot, SourceReseau};
    use std::sync::Mutex;

    fn bibliotheque_avec(source: Option<SourceReseau>) -> Mutex<Depot> {
        let mut depot = Depot::en_memoire().expect("base en mémoire");
        if let Some(s) = source {
            depot.enregistrer_source_reseau(&s).expect("source enregistrée");
        }
        Mutex::new(depot)
    }

    fn source(active: bool) -> SourceReseau {
        SourceReseau {
            id: "src".into(),
            nom: "faux-nas".into(),
            hote: "faux.local".into(),
            partage: "music".into(),
            chemin_distant: "/".into(),
            utilisateur: "admin".into(),
            domaine: None,
            invite: false,
            active,
        }
    }

    /// Une source inconnue est une erreur **nommée**, et elle est rendue avant toute
    /// tentative réseau : sans ça, un identifiant faux se transformerait en « NAS
    /// injoignable », qui envoie chercher au mauvais endroit.
    #[test]
    fn une_source_inconnue_echoue_sans_toucher_au_reseau() {
        let b = bibliotheque_avec(None);
        let e = scanner_une_source(&b, "inconnue").expect_err("doit échouer");
        assert!(matches!(e, Erreur::SourceInconnue(_)), "{e}");
        assert!(e.to_string().contains("inconnue"), "{e}");
    }

    /// Une source désactivée ne se scanne pas, et c'est dit avec son **nom** : c'est le nom
    /// que Thomas voit à l'écran, pas l'identifiant.
    #[test]
    fn une_source_desactivee_ne_se_scanne_pas() {
        let b = bibliotheque_avec(Some(source(false)));
        let e = scanner_une_source(&b, "src").expect_err("doit échouer");
        assert!(matches!(e, Erreur::SourceDesactivee(_)), "{e}");
        assert!(e.to_string().contains("faux-nas"), "{e}");
    }

    /// Une source en accès **invité** ne demande aucun mot de passe, et c'était un chemin
    /// mort : elle échouait sur « aucun mot de passe pour la source … » alors que `client.rs`
    /// n'emploie ni utilisateur ni mot de passe quand `invite` est vrai. Le message envoyait
    /// chercher au mauvais endroit, pour une donnée dont personne n'avait besoin.
    #[test]
    fn une_source_invite_ne_demande_aucun_mot_de_passe() {
        let mut s = source(true);
        s.invite = true;
        s.utilisateur = String::new();
        s.id = "source-invite-sans-rien-au-trousseau-test".into();
        assert_eq!(
            super::mot_de_passe_si_necessaire(&s).expect("une source invité n'a rien à chercher"),
            ""
        );
    }

    /// Et une source qui n'est **pas** en invité va bien chercher — et échoue proprement
    /// quand il n'y a rien, au lieu de rendre une chaîne vide (INV-04).
    #[test]
    fn une_source_non_invite_va_chercher_et_echoue_proprement() {
        let mut s = source(true);
        s.id = "source-non-invite-sans-rien-au-trousseau-test".into();
        let e = super::mot_de_passe_si_necessaire(&s).expect_err("rien au trousseau");
        assert!(matches!(e, Erreur::Identifiants(_)), "{e}");
    }

    /// Et l'ordre des étapes se voit : une source **active** sans mot de passe au trousseau
    /// échoue sur les identifiants, donc après l'étape 1 et avant tout appel SMB. C'est ce
    /// qui prouve que le mot de passe n'est pas demandé pour rien quand la source est
    /// désactivée ou inconnue.
    #[test]
    fn une_source_active_sans_mot_de_passe_echoue_sur_le_trousseau() {
        let mut s = source(true);
        s.id = "source-sans-mot-de-passe-au-trousseau-test".into();
        let b = bibliotheque_avec(Some(s.clone()));
        let e = scanner_une_source(&b, &s.id).expect_err("doit échouer");
        assert!(matches!(e, Erreur::Identifiants(_)), "{e}");
    }
}
