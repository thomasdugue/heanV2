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
//! **Le nom de service est tranché** — option B de
//! [l'ADR 0015](../../../docs/adr/0015-nom-de-service-keychain-pour-les-mots-de-passe-smb.md),
//! choisie par Thomas le 2026-11-26 : un seul nom, celui de V2. Le nom de V1
//! (`com.tdugue.noir.smb`) **n'est plus lu**, et n'a jamais été écrit ni supprimé par V2 —
//! ce qu'il contient reste dans le trousseau de l'utilisateur, intact, à lui d'en disposer.
//! **Ce que ça coûte : rien.** L'ADR chiffrait « une saisie de mot de passe » ; ce coût était
//! déjà payé quand Thomas a répondu. Son trousseau porte le compte de la source vivante sous
//! `com.tdugue.hean.smb` depuis le 2026-11-25 21:55:37Z — il l'a ressaisi pendant le
//! diagnostic de connexion, et `enregistrer()` n'écrit que sous le nom de V2. Mesuré, pas
//! déduit : son journal du 2026-11-26 01:01Z porte `service="com.tdugue.hean.smb"`.
//!
//! V009 — *aucun dialogue Keychain au lancement* : rien ici n'est appelé au démarrage. La
//! cascade est consultée au **premier scan** d'une source, et le cache de session fait que
//! c'est une seule fois par source et par lancement.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Le nom de service, et il n'y en a **qu'un**. V1 écrivait sous `com.tdugue.noir.smb`
/// (`noir-tauri/src-tauri/src/network/credentials.rs`, ligne 12) alors que le bundle
/// identifier disait déjà `com.tdugue.hean` ; V2 ne porte pas ce nom-là.
///
/// **Reste une liste, et pas une constante seule, pour une raison** : la forme plurielle est
/// ce qui a permis d'hériter du nom de V1 en lecture pendant deux jours sans rien écrire, et
/// c'est la forme qu'il faudrait reprendre si un nom devait un jour être abandonné pour un
/// autre. La réduire à un `&str` ferait réécrire la cascade, ses messages et ses tests.
const SERVICES: &[&str] = &["com.tdugue.hean.smb"];

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
    /// La cascade est épuisée. Le message **nomme les services consultés** : sans eux, la
    /// personne qui lit le journal ne peut pas savoir si c'est le mot de passe qui manque
    /// ou le nom de service qui a changé.
    #[error(
        "aucun mot de passe pour la source {id_source} dans le trousseau macOS \
         (services consultés : {services}) — l'ouvrir dans Hean et le saisir à nouveau"
    )]
    Absent { id_source: String, services: String },
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

/// Le mot de passe d'une source : cache de session, puis la cascade de services.
///
/// Ne rend **jamais** `Ok("")`. Les deux chemins d'échec sont des variantes d'erreur
/// distinctes, parce que « pas enregistré » et « enregistré vide » demandent le même geste à
/// l'utilisateur mais pas la même enquête à qui lit le journal.
pub fn mot_de_passe(source_id: &str) -> Result<String, Erreur> {
    let mut cache = verrou();
    if let Some(connu) = cache.get(source_id) {
        return Ok(connu.clone());
    }
    for service in SERVICES {
        match lire_du_trousseau(service, source_id) {
            Some(mdp) if mdp.is_empty() => {
                return Err(Erreur::Vide { id_source: source_id.to_string() })
            }
            Some(mdp) => {
                tracing::info!(service, "mot de passe SMB trouvé dans le trousseau");
                cache.insert(source_id.to_string(), mdp.clone());
                return Ok(mdp);
            }
            None => continue,
        }
    }
    Err(Erreur::Absent {
        id_source: source_id.to_string(),
        services: SERVICES.join(", "),
    })
}

/// Enregistre un mot de passe, **sous le nom de service de V2 seulement**.
pub fn enregistrer(source_id: &str, mot_de_passe: &str) -> Result<(), Erreur> {
    if mot_de_passe.is_empty() {
        return Err(Erreur::Vide { id_source: source_id.to_string() });
    }
    let service = SERVICES[0];
    ecrire_au_trousseau(service, source_id, mot_de_passe).map_err(|detail| Erreur::Ecriture {
        id_source: source_id.to_string(),
        detail,
    })?;
    verrou().insert(source_id.to_string(), mot_de_passe.to_string());
    tracing::info!(service, "mot de passe SMB enregistré dans le trousseau");
    Ok(())
}

/// Le cache, en reprenant la donnée d'un verrou empoisonné plutôt qu'en paniquant : un
/// thread qui a paniqué en tenant ce verrou ne doit pas empêcher la source d'être lue.
fn verrou() -> std::sync::MutexGuard<'static, HashMap<String, String>> {
    CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

/// La lecture réelle. `None` = absent **de ce service** ; ce n'est pas une erreur en soi,
/// c'est ce que rend le trousseau d'une source dont le mot de passe n'a pas encore été saisi
/// sous le nom de V2 — le cas de chaque source héritée de V1, une fois.
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
    use super::{Erreur, SERVICES};

    /// Les tests de ce module **ne touchent pas au trousseau de la machine**, et c'est une
    /// contrainte et non une commodité : un test qui lirait le vrai Keychain ouvrirait un
    /// dialogue macOS en CI, et chez Thomas pendant `./check`. Ce qui s'éprouve ici est donc
    /// la **cascade** et la forme des erreurs — ce qui est précisément ce qu'INV-04 protège.
    /// Ce qui ne s'éprouve pas : que `get_generic_password` fasse ce que sa doc dit.
    #[test]
    fn un_seul_nom_de_service_et_celui_de_v1_nest_plus_lu() {
        assert_eq!(SERVICES, &["com.tdugue.hean.smb"]);
        // L'ADR 0015 est tranché (option B, Thomas, 2026-11-26). Ce test est ce qui empêche
        // quelqu'un de « réparer » un mot de passe introuvable en remettant le nom de V1 :
        // ce ne serait pas une correction, ce serait annuler la décision sans le dire.
        assert!(
            !SERVICES.iter().any(|s| s.contains("noir")),
            "le nom de V1 ne revient pas en lecture sans un nouvel ADR"
        );
    }

    /// INV-04 — un mot de passe vide n'est jamais rendu comme un mot de passe.
    #[test]
    fn un_mot_de_passe_vide_est_une_erreur_a_lecriture() {
        let e = super::enregistrer("source-de-test", "").expect_err("doit refuser");
        assert!(matches!(e, Erreur::Vide { .. }), "{e}");
    }

    /// Les deux messages d'échec disent quoi faire, et celui de l'absence **nomme les
    /// services consultés** : sans ça, personne ne peut distinguer « mot de passe effacé »
    /// de « nom de service changé », et l'ADR 0015 se rediscuterait à l'aveugle.
    #[test]
    fn les_messages_derreur_nomment_ce_quil_faut_pour_enqueter() {
        let absent = Erreur::Absent {
            id_source: "dcdd24af".into(),
            services: SERVICES.join(", "),
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
