//! Parler à un partage SMB. Ce module est le **seul** endroit du projet qui touche
//! `libsmbclient`.
//!
//! ## PLAT-2 (🔴) — `libsmbclient` est un singleton de process
//!
//! Deux `SmbClient` vivants en même temps rendent `EINVAL (os error 22)`. Ce n'est pas un
//! choix d'architecture, c'est une contrainte de la bibliothèque C : **tout passe par un
//! verrou unique**, et il n'existe qu'un `static` ici pour le porter. Aucun test ne prouvera
//! cette règle ; c'est pour ça qu'elle est écrite en haut du fichier qu'elle contraint.
//!
//! Conséquence directe sur l'IPC : un scan tient ce verrou pendant des minutes, donc il ne
//! doit **pas** tenir en même temps celui de la bibliothèque (ADR 0016 §5).
//!
//! ## INV-03 (🔴) — une connexion périmée se retente
//!
//! Un NAS ferme une session inactive, et `libsmbclient` ne s'en rend compte qu'à l'opération
//! suivante. La règle attendait la phase 5 et un conteneur Samba ; elle ne l'attend plus,
//! parce que la **réaction** à une connexion périmée est séparée de sa cause :
//! `reessayer_une_fois` ne connaît ni SMB ni le réseau, donc elle s'éprouve par injection de
//! panne (tests en bas de ce fichier). Ce qui demande un vrai serveur est la cause, pas la
//! réaction.

use std::sync::{LazyLock, Mutex};

use crate::library::SourceReseau;

/// Une entrée d'un dossier distant.
///
/// `taille` et `mtime_ns` valent **zéro quand le serveur ne les a pas données** —
/// `readdirplus` n'existe pas partout, et le repli ne rend que des noms. Zéro veut donc dire
/// « inconnu », pas « vide » : c'est l'inventaire qui en tire la conséquence, et il la tire
/// dans un seul endroit (`network::inventaire`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entree {
    pub nom: String,
    pub dossier: bool,
    pub taille: u64,
    pub mtime_ns: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error("connexion à {hote} (partage {partage}) : {detail}")]
    Connexion { hote: String, partage: String, detail: String },
    #[error("lecture du dossier {chemin} sur {hote} : {detail}")]
    Dossier { hote: String, chemin: String, detail: String },
    #[error("lecture du fichier {chemin} sur {hote} : {detail}")]
    Fichier { hote: String, chemin: String, detail: String },
}

/// Ce dont l'inventaire a besoin d'un partage SMB, et rien de plus.
///
/// C'est **la couture qui rend les tests possibles sans le NAS de Thomas** : il n'est pas
/// toujours allumé, et la CI n'y a pas accès. Le parcours, le scan différentiel et les
/// chemins d'échec s'éprouvent contre un faux qui implémente ce trait — y compris les
/// pannes, qu'on provoque au lieu de les affirmer.
pub trait Smb {
    /// Le contenu d'un dossier, chemin relatif à la racine du partage (`/` = racine).
    fn lister(&self, chemin: &str) -> Result<Vec<Entree>, Erreur>;

    /// Les octets d'un fichier, au plus `max` s'il est donné.
    ///
    /// `max` existe parce qu'un en-tête suffit pour lire des tags, et qu'un en-tête de
    /// 1 Mio coûte ~29 ms de transfert au lieu de plusieurs secondes pour un FLAC entier
    /// (ADR 0016 §3). Quels formats y ont droit est décidé par `network::entete`, pas ici.
    fn lire(&self, chemin: &str, max: Option<usize>) -> Result<Vec<u8>, Erreur>;
}

/// **INV-03.** Tente l'opération ; si elle échoue, rafraîchit la connexion et retente **une
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

/// La connexion SMB active. **Une seule pour tout le process** (PLAT-2).
static CONNEXION: LazyLock<Mutex<Option<Active>>> = LazyLock::new(|| Mutex::new(None));

struct Active {
    client: pavao::SmbClient,
    hote: String,
    partage: String,
}

/// Le client réel. Il ne garde pas de connexion : la connexion est dans le `static`, parce
/// que c'est la bibliothèque C qui est unique, pas ce type-ci.
pub struct ClientSmb {
    hote: String,
    partage: String,
    utilisateur: String,
    mot_de_passe: String,
    domaine: Option<String>,
    invite: bool,
}

impl ClientSmb {
    /// Le mot de passe arrive **en paramètre** et n'est pas lu ici : ce module ne connaît
    /// pas le trousseau, et `network::identifiants` ne connaît pas SMB. Un mot de passe vide
    /// serait refusé par l'appelant (INV-04) ; le refuser aussi ici serait un second endroit
    /// où la règle vit.
    pub fn nouveau(source: &SourceReseau, mot_de_passe: String) -> Self {
        ClientSmb {
            hote: source.hote.clone(),
            partage: source.partage.clone(),
            utilisateur: source.utilisateur.clone(),
            mot_de_passe,
            domaine: source.domaine.clone(),
            invite: source.invite,
        }
    }

    /// Ouvre une connexion neuve, en remplaçant celle qui existe. L'ancienne est relâchée
    /// **avant** que la nouvelle soit créée : deux clients vivants en même temps, c'est
    /// exactement `EINVAL`.
    fn connecter(&self, place: &mut Option<Active>) -> Result<(), Erreur> {
        *place = None;
        let serveur = url_du_serveur(&self.hote);
        let mut creds = pavao::SmbCredentials::default()
            .server(&serveur)
            .share(format!("/{}", self.partage.trim_matches('/')));
        if !self.invite {
            creds = creds.username(&self.utilisateur).password(&self.mot_de_passe);
            if let Some(d) = &self.domaine {
                creds = creds.workgroup(d);
            }
        }
        let client = pavao::SmbClient::new(
            creds,
            pavao::SmbOptions::default().one_share_per_server(true),
        )
        .map_err(|e| Erreur::Connexion {
            hote: self.hote.clone(),
            partage: self.partage.clone(),
            detail: e.to_string(),
        })?;
        *place = Some(Active {
            client,
            hote: self.hote.clone(),
            partage: self.partage.clone(),
        });
        Ok(())
    }

    /// Fait l'opération en tenant le verrou unique, en se connectant si besoin.
    fn avec<T>(&self, f: impl FnOnce(&pavao::SmbClient) -> Result<T, Erreur>) -> Result<T, Erreur> {
        let mut place = CONNEXION.lock().unwrap_or_else(|e| e.into_inner());
        let bonne = place
            .as_ref()
            .is_some_and(|a| a.hote == self.hote && a.partage == self.partage);
        if !bonne {
            self.connecter(&mut place)?;
        }
        let active = place.as_ref().ok_or_else(|| Erreur::Connexion {
            hote: self.hote.clone(),
            partage: self.partage.clone(),
            detail: "aucune connexion active après connexion".to_string(),
        })?;
        f(&active.client)
    }

    /// Jette la connexion courante : la prochaine opération en ouvrira une neuve. C'est le
    /// `rafraichir` d'INV-03.
    fn oublier_la_connexion(&self) -> Result<(), Erreur> {
        *CONNEXION.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }
}

impl Smb for ClientSmb {
    fn lister(&self, chemin: &str) -> Result<Vec<Entree>, Erreur> {
        reessayer_une_fois(
            || {
                self.avec(|client| {
                    // `list_dirplus` rend taille et date **en un seul aller-retour par
                    // dossier** (readdirplus de SMB2), ce qui donne un scan différentiel
                    // réel sans payer un `stat` par fichier. V1 ne s'en servait pas : ses
                    // 3 137 entrées de cache portent toutes `size: 0, modified: 0`.
                    match client.list_dirplus(chemin) {
                        Ok(entrees) => Ok(entrees
                            .into_iter()
                            .map(|e| Entree {
                                dossier: matches!(e.get_type(), pavao::SmbDirentType::Dir),
                                nom: e.name.clone(),
                                taille: e.size,
                                mtime_ns: en_nanos(e.mtime),
                            })
                            .collect()),
                        Err(plus) => {
                            // Tous les serveurs ne répondent pas à readdirplus. Le repli ne
                            // rend que des noms : taille et date restent inconnues, et
                            // l'inventaire sait quoi en faire.
                            tracing::debug!(chemin, erreur = %plus, "readdirplus indisponible, repli sur les noms seuls");
                            let entrees = client.list_dir(chemin).map_err(|e| Erreur::Dossier {
                                hote: self.hote.clone(),
                                chemin: chemin.to_string(),
                                detail: e.to_string(),
                            })?;
                            Ok(entrees
                                .into_iter()
                                .filter(|e| e.name() != "." && e.name() != "..")
                                .map(|e| Entree {
                                    dossier: matches!(e.get_type(), pavao::SmbDirentType::Dir),
                                    nom: e.name().to_string(),
                                    taille: 0,
                                    mtime_ns: 0,
                                })
                                .collect())
                        }
                    }
                })
            },
            || self.oublier_la_connexion(),
        )
    }

    fn lire(&self, chemin: &str, max: Option<usize>) -> Result<Vec<u8>, Erreur> {
        reessayer_une_fois(
            || {
                self.avec(|client| {
                    use std::io::Read;
                    let echec = |detail: String| Erreur::Fichier {
                        hote: self.hote.clone(),
                        chemin: chemin.to_string(),
                        detail,
                    };
                    let fichier = client
                        .open_with(chemin, pavao::SmbOpenOptions::default().read(true))
                        .map_err(|e| echec(e.to_string()))?;
                    let mut octets = Vec::new();
                    match max {
                        Some(n) => fichier
                            .take(n as u64)
                            .read_to_end(&mut octets)
                            .map_err(|e| echec(e.to_string()))?,
                        None => {
                            let mut f = fichier;
                            f.read_to_end(&mut octets).map_err(|e| echec(e.to_string()))?
                        }
                    };
                    Ok(octets)
                })
            },
            || self.oublier_la_connexion(),
        )
    }
}

/// L'URL du serveur. Une adresse IPv6 nue se met entre crochets, sinon `libsmbclient`
/// prend le `:` pour un séparateur de port. Repris de V1, où le cas avait été rencontré.
fn url_du_serveur(hote: &str) -> String {
    if hote.contains(':') && !hote.starts_with('[') {
        format!("smb://[{hote}]")
    } else {
        format!("smb://{hote}")
    }
}

/// Une date SMB en nanosecondes depuis l'époque, dans la même unité que le scan local —
/// sinon les deux provenances ne se comparent pas. Une date antérieure à 1970 ou absurde
/// donne zéro, c'est-à-dire « inconnue », jamais un `unwrap()`.
fn en_nanos(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_nanos()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{en_nanos, reessayer_une_fois, url_du_serveur, Erreur};
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

    #[test]
    fn une_adresse_ipv6_nue_est_mise_entre_crochets() {
        assert_eq!(url_du_serveur("dugstation.local"), "smb://dugstation.local");
        assert_eq!(url_du_serveur("192.168.1.10"), "smb://192.168.1.10");
        assert_eq!(url_du_serveur("fe80::1"), "smb://[fe80::1]");
        assert_eq!(url_du_serveur("[fe80::1]"), "smb://[fe80::1]");
    }

    #[test]
    fn une_date_absurde_vaut_inconnue_et_ne_panique_pas() {
        assert_eq!(en_nanos(std::time::UNIX_EPOCH), 0);
        assert!(en_nanos(std::time::SystemTime::now()) > 0);
        // Avant 1970 : `duration_since` échoue, et on rend « inconnue ».
        let avant = std::time::UNIX_EPOCH - std::time::Duration::from_secs(1);
        assert_eq!(en_nanos(avant), 0);
    }
}
