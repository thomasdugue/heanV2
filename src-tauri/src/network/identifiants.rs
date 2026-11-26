//! Le mot de passe d'une source SMB. Il vit dans le **Keychain macOS** et nulle part
//! ailleurs : ni dans la base, ni dans un JSON, ni dans un type qui traverse l'IPC.
//!
//! **INV-04 (🔴) est la règle de ce fichier** : un échec de Keychain remonte une **erreur**,
//! jamais une chaîne vide. En V1, `retrieve_password().unwrap_or_default()` faisait tenter
//! au NAS une connexion avec un mot de passe vide — donc un risque de **verrouillage du
//! compte** de l'utilisateur, pour une erreur qui ne s'affichait nulle part. Un test
//! d'architecture refuse `unwrap_or_default()` sur toute ligne qui parle d'identifiant, et
//! les tests d'ici refusent qu'un mot de passe vide passe pour un mot de passe.
//!
//! **Le nom de service est tranché** : `com.tdugue.hean.smb`, et lui seul. Thomas a choisi
//! l'option B de l'
//! [ADR 0015](../../../docs/adr/0015-nom-de-service-keychain-pour-les-mots-de-passe-smb.md)
//! le 2026-11-26. V2 ne consulte plus `com.tdugue.noir.smb`, le nom écrit par V1, et n'y
//! écrit ni n'y supprime rien : les entrées que V1 y a laissées restent dans le trousseau,
//! simplement plus jamais lues.
//!
//! Le coût que cet ADR chiffrait — « Thomas retape son mot de passe une fois » — **avait
//! déjà été payé quand il a répondu** : son mot de passe est sous le nom de V2 depuis le
//! diagnostic de connexion du 2026-11-25, où il l'a ressaisi. Mesuré, pas supposé : son
//! journal du 2026-11-26 à 01:01Z porte `service="com.tdugue.hean.smb"`. Retirer le nom de
//! V1 ne lui demande donc aucun geste.
//!
//! V009 — *aucun dialogue Keychain au lancement* : rien ici n'est appelé au démarrage. Le
//! trousseau est consulté au **premier scan** d'une source, et le cache de session fait que
//! c'est une seule fois par source et par lancement.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Le nom de service Keychain, **le seul** que V2 lise et écrive (ADR 0015, option B). Le
/// compte, sous ce service, est l'identifiant de la source réseau.
///
/// V1 écrivait sous `com.tdugue.noir.smb` (`noir-tauri/src-tauri/src/network/credentials.rs`,
/// ligne 12) alors que son bundle identifier disait déjà `com.tdugue.hean` : c'était la
/// dernière chaîne du dépôt à dire « noir ». **Ne pas la remettre ici** — c'est la décision,
/// et le test de ce module refuse qu'un nom de service mentionne « noir ».
const SERVICE: &str = "com.tdugue.hean.smb";

/// Le cache de session. Il existe pour une raison précise, héritée de V1 : **sans lui,
/// macOS peut demander une autorisation à chaque lecture**. Un scan qui ouvre 3 000 fichiers
/// n'a le droit de poser la question qu'une fois.
///
/// Le verrou est tenu pendant **tout** l'appel — vérification, lecture, mise en cache — et
/// pas seulement pendant la vérification : deux threads voyant simultanément « absent »
/// déclencheraient chacun un dialogue. C'est la correction que V1 avait déjà faite.
static CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Les champs s'appellent `id_source` et non `source` : `thiserror` réserve ce nom à la
/// **cause** d'une erreur, et un champ `source: String` ne compile pas. Le détail est noté
/// ici parce que le message du compilateur, lui, parle de `as_dyn_error`.
#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    /// Rien sous ce service pour cette source. Le message **nomme le service consulté** :
    /// sans lui, la personne qui lit le journal ne peut pas savoir si c'est le mot de passe
    /// qui manque ou le nom de service qui a changé — et ce nom a changé une fois, le
    /// 2026-11-26 (ADR 0015), donc la question se posera.
    #[error(
        "aucun mot de passe pour la source {id_source} dans le trousseau macOS \
         (service consulté : {service}) — l'ouvrir dans Hean et le saisir à nouveau"
    )]
    Absent { id_source: String, service: String },
    /// Trouvé, mais **vide**. C'est INV-04 pris par l'autre bout : un mot de passe vide
    /// stocké est aussi dangereux qu'un `unwrap_or_default()`, et il ne se distingue d'une
    /// absence que si on le dit.
    #[error("le mot de passe enregistré pour la source {id_source} est vide — le saisir à nouveau")]
    Vide { id_source: String },
    #[error("écriture du mot de passe de la source {id_source} dans le trousseau macOS : {detail}")]
    Ecriture { id_source: String, detail: String },
}

/// Vrai si le mot de passe de cette source est déjà en mémoire pour cette session.
///
/// Sert à répondre « non » **sans toucher au trousseau** : une fonction qui voudrait savoir
/// si une source est joignable ne doit pas provoquer un dialogue pour le découvrir (V009).
pub fn en_session(source_id: &str) -> bool {
    verrou().contains_key(source_id)
}

/// Le mot de passe d'une source : cache de session, puis le trousseau.
///
/// Ne rend **jamais** `Ok("")`. Les deux chemins d'échec sont des variantes d'erreur
/// distinctes, parce que « pas enregistré » et « enregistré vide » demandent le même geste à
/// l'utilisateur mais pas la même enquête à qui lit le journal.
pub fn mot_de_passe(source_id: &str) -> Result<String, Erreur> {
    let mut cache = verrou();
    if let Some(connu) = cache.get(source_id) {
        return Ok(connu.clone());
    }
    match lire_du_trousseau(SERVICE, source_id) {
        Some(mdp) if mdp.is_empty() => Err(Erreur::Vide { id_source: source_id.to_string() }),
        Some(mdp) => {
            tracing::info!(service = SERVICE, "mot de passe SMB trouvé dans le trousseau");
            cache.insert(source_id.to_string(), mdp.clone());
            Ok(mdp)
        }
        None => Err(Erreur::Absent {
            id_source: source_id.to_string(),
            service: SERVICE.to_string(),
        }),
    }
}

/// Enregistre un mot de passe, **sous le nom de service de V2 seulement**.
pub fn enregistrer(source_id: &str, mot_de_passe: &str) -> Result<(), Erreur> {
    if mot_de_passe.is_empty() {
        return Err(Erreur::Vide { id_source: source_id.to_string() });
    }
    ecrire_au_trousseau(SERVICE, source_id, mot_de_passe).map_err(|detail| Erreur::Ecriture {
        id_source: source_id.to_string(),
        detail,
    })?;
    verrou().insert(source_id.to_string(), mot_de_passe.to_string());
    tracing::info!(service = SERVICE, "mot de passe SMB enregistré dans le trousseau");
    Ok(())
}

/// Le cache, en reprenant la donnée d'un verrou empoisonné plutôt qu'en paniquant : un
/// thread qui a paniqué en tenant ce verrou ne doit pas empêcher la source d'être lue.
fn verrou() -> std::sync::MutexGuard<'static, HashMap<String, String>> {
    CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

/// La lecture réelle. `None` = absent **de ce service** ; ce n'est pas une erreur, c'est le
/// cas normal d'une source dont le mot de passe n'a jamais été saisi dans V2.
///
/// Un échec qui n'est pas une absence — trousseau verrouillé, autorisation refusée — est
/// journalisé **et** rendu comme absent, ce qui fait tomber l'appelant sur `Erreur::Absent`.
/// La distinction que `security-framework` rend ici ne porte pas assez pour valoir une
/// variante d'erreur de plus ; ce qui compte pour INV-04 est qu'aucun de ces cas ne
/// produise un mot de passe.
#[cfg(target_os = "macos")]
fn lire_du_trousseau(service: &str, compte: &str) -> Option<String> {
    match security_framework::passwords::get_generic_password(service, compte) {
        Ok(octets) => match String::from_utf8(octets) {
            Ok(mdp) => Some(mdp),
            Err(e) => {
                tracing::warn!(service, erreur = %e, "mot de passe du trousseau non UTF-8");
                None
            }
        },
        Err(e) => {
            tracing::debug!(service, erreur = %e, "pas de mot de passe sous ce service");
            None
        }
    }
}

#[cfg(target_os = "macos")]
fn ecrire_au_trousseau(service: &str, compte: &str, mdp: &str) -> Result<(), String> {
    security_framework::passwords::set_generic_password(service, compte, mdp.as_bytes())
        .map_err(|e| e.to_string())
}

// L'app est macOS-only (CoreAudio, media keys, libsmbclient). Ces deux replis existent pour
// que le crate compile ailleurs — un test lancé sous Linux ne doit pas échouer sur un
// `cfg` manquant — et ils rendent « absent », donc une erreur chez l'appelant.
#[cfg(not(target_os = "macos"))]
fn lire_du_trousseau(_service: &str, _compte: &str) -> Option<String> {
    None
}

#[cfg(not(target_os = "macos"))]
fn ecrire_au_trousseau(_service: &str, _compte: &str, _mdp: &str) -> Result<(), String> {
    Err("le trousseau macOS n'existe pas sur cette plateforme".to_string())
}

#[cfg(test)]
mod tests {
    use super::{Erreur, SERVICE};

    /// Les tests de ce module **ne touchent pas au trousseau de la machine**, et c'est une
    /// contrainte et non une commodité : un test qui lirait le vrai Keychain ouvrirait un
    /// dialogue macOS en CI, et chez Thomas pendant `./check`. Ce qui s'éprouve ici est donc
    /// le **nom de service** et la forme des erreurs — ce qui est précisément ce qu'INV-04
    /// protège. Ce qui ne s'éprouve pas : que `get_generic_password` fasse ce que sa doc dit.
    #[test]
    fn le_seul_nom_de_service_est_celui_de_v2() {
        assert_eq!(SERVICE, "com.tdugue.hean.smb");
        // La seconde assertion est celle qui tient la décision, et elle n'est pas
        // redondante : la première interdit *ce* changement-là, la seconde interdit **toute
        // remise du nom de V1**, sous quelque forme que ce soit. L'ADR 0015 a tranché pour
        // l'option B le 2026-11-26 ; sans cette ligne, « ça marchait avant avec noir » est
        // une correction d'une minute que rien ne contredit.
        assert!(
            !SERVICE.contains("noir"),
            "le nom de service de V1 est revenu : {SERVICE} (ADR 0015, option B)"
        );
    }

    /// INV-04 — un mot de passe vide n'est jamais rendu comme un mot de passe.
    #[test]
    fn un_mot_de_passe_vide_est_une_erreur_a_lecriture() {
        let e = super::enregistrer("source-de-test", "").expect_err("doit refuser");
        assert!(matches!(e, Erreur::Vide { .. }), "{e}");
    }

    /// Les deux messages d'échec disent quoi faire, et celui de l'absence **nomme le service
    /// consulté** : sans ça, personne ne peut distinguer « mot de passe effacé » de « nom de
    /// service changé », et l'ADR 0015 se rediscuterait à l'aveugle.
    #[test]
    fn les_messages_derreur_nomment_ce_quil_faut_pour_enqueter() {
        let absent = Erreur::Absent {
            id_source: "dcdd24af".into(),
            service: SERVICE.to_string(),
        }
        .to_string();
        for attendu in ["dcdd24af", "com.tdugue.hean.smb", "saisir"] {
            assert!(absent.contains(attendu), "« {attendu} » absent de : {absent}");
        }
        let vide = Erreur::Vide { id_source: "dcdd24af".into() }.to_string();
        assert!(vide.contains("vide"), "{vide}");
    }

    /// La lecture d'un service qui n'existe pas rend `None` et **ne panique pas**. C'est le
    /// seul appel réel au trousseau que ces tests se permettent, et il est inoffensif : un
    /// service inexistant ne déclenche pas de dialogue, il échoue tout de suite.
    #[test]
    fn un_service_inconnu_rend_none_sans_paniquer() {
        assert!(super::lire_du_trousseau(
            "com.tdugue.hean.service-qui-nexiste-pas.test",
            "compte-qui-nexiste-pas"
        )
        .is_none());
    }
}
