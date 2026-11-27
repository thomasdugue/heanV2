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
//! ## La reprise d'INV-03 est dans `reprise.rs`
//!
//! Elle en est sortie le 2026-11-25, quand ce fichier a atteint 400 lignes en apprenant à ne
//! plus réutiliser une connexion ouverte avec de mauvais identifiants. Le découpage suit une
//! frontière réelle : ici on parle à `libsmbclient`, là-bas on décide quoi faire quand une
//! opération échoue — et c'est justement parce que cette décision ne connaît ni SMB ni le
//! réseau qu'elle s'éprouve par injection de panne.

// Les lectures d'octets vivent dans un module ENFANT, et non à côté : elles touchent
// `libsmbclient`, donc elles restent sous ce module, dont l'en-tête dit qu'il en est le seul
// endroit du projet. Sorties d'ici le 2026-11-28, quand `tranche` l'a mené à la limite des
// 400 lignes — et la frontière est réelle : ici la connexion et le parcours, là-bas les
// octets d'un fichier.
mod octets;
pub use octets::Tranche;

use std::sync::{LazyLock, Mutex};

use super::reprise::reessayer_une_fois;

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

/// Les erreurs du sous-système réseau vivent dans `erreur.rs`, et sont réexportées ici :
/// `client::Erreur` reste le nom par lequel tout le module les désigne. Elles en sont
/// sorties le 2026-11-25, quand ce fichier a franchi 400 lignes en apprenant à dire *pourquoi*
/// un dossier ne s'ouvre pas — le message qui a coûté quatre saisies de mot de passe.
pub use super::erreur::Erreur;

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

    /// Une tranche d'un fichier distant, **et sa taille totale**, en une seule ouverture.
    ///
    /// La primitive de la lecture en direct (ADR 0022). Elle rend la taille parce que
    /// `pavao::SmbFile` **emprunte** le client : un handle ne survit pas à la sortie du
    /// verrou de PLAT-2, donc demander la taille à part coûterait une ouverture de plus.
    fn tranche(&self, chemin: &str, debut: u64, max: usize) -> Result<Tranche, Erreur>;
}

/// La connexion SMB active. **Une seule pour tout le process** (PLAT-2).
static CONNEXION: LazyLock<Mutex<Option<Active>>> = LazyLock::new(|| Mutex::new(None));

struct Active {
    client: pavao::SmbClient,
    hote: String,
    partage: String,
    /// **L'empreinte des identifiants avec lesquels cette connexion a été ouverte.**
    ///
    /// Sans elle, la connexion était réutilisée dès que l'hôte et le partage
    /// correspondaient — **quels que soient les identifiants**. Conséquence mesurée le
    /// 2026-11-25 sur le NAS de Thomas : une faute de frappe dans le mot de passe ouvrait
    /// une connexion refusée par le NAS, et **corriger le mot de passe ne changeait rien**.
    /// La connexion fautive restait en cache pour toute la durée de l'application, et chaque
    /// scan repartait dessus. Il a saisi son mot de passe quatre fois en croyant se tromper.
    ///
    /// Ce qui l'a sauvé est INV-03 : la seconde tentative jette la connexion et en ouvre une
    /// neuve, donc le scan finissait par passer — **au coup d'après**. Un garde-fou a
    /// rattrapé le défaut sans le signaler, ce qui est la pire façon de le découvrir.
    ///
    /// C'est une empreinte et non les identifiants : un mot de passe n'a pas à vivre dans un
    /// `static` de ce module. Elle ne sort jamais du process et ne se compare qu'à elle-même,
    /// donc la stabilité du hachage entre deux versions de Rust n'a aucune importance ici.
    empreinte: u64,
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
            empreinte: self.empreinte(),
        });
        Ok(())
    }

    /// L'empreinte de ce à quoi on se connecte **et avec quoi**. Hôte, partage, utilisateur,
    /// domaine, drapeau invité, mot de passe : si l'un change, la connexion en cache n'est
    /// plus la bonne.
    fn empreinte(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.hote.hash(&mut h);
        self.partage.hash(&mut h);
        self.utilisateur.hash(&mut h);
        self.domaine.hash(&mut h);
        self.invite.hash(&mut h);
        self.mot_de_passe.hash(&mut h);
        h.finish()
    }

    /// Fait l'opération en tenant le verrou unique, en se connectant si besoin.
    fn avec<T>(&self, f: impl FnOnce(&pavao::SmbClient) -> Result<T, Erreur>) -> Result<T, Erreur> {
        let mut place = CONNEXION.lock().unwrap_or_else(|e| e.into_inner());
        // **L'empreinte fait partie de la question, et c'est la correction du 2026-11-25.**
        // Comparer l'hôte et le partage seuls faisait réutiliser une connexion ouverte avec
        // un mot de passe faux, donc corriger ce mot de passe n'avait aucun effet.
        let bonne = place.as_ref().is_some_and(|a| {
            a.hote == self.hote && a.partage == self.partage && a.empreinte == self.empreinte()
        });
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

    /// L'opération, avec la reprise d'INV-03 — **et sans laisser derrière soi une connexion
    /// qui a échoué deux fois.**
    ///
    /// Le second point est la moitié oubliée de la correction du 2026-11-25 : une connexion
    /// qui vient d'échouer restait en cache, donc l'opération suivante repartait dessus et
    /// payait à nouveau une tentative perdue avant de reconnecter. Deux échecs de suite
    /// veulent dire que cette connexion-là n'est pas bonne ; la prochaine opération doit
    /// partir propre.
    fn avec_reprise<T>(&self, op: impl Fn() -> Result<T, Erreur>) -> Result<T, Erreur> {
        let resultat = reessayer_une_fois(&op, || self.oublier_la_connexion());
        if resultat.is_err() {
            let _ = self.oublier_la_connexion();
        }
        resultat
    }
}

impl Smb for ClientSmb {
    fn lister(&self, chemin: &str) -> Result<Vec<Entree>, Erreur> {
        self.avec_reprise(
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
        )
    }

    fn lire(&self, chemin: &str, max: Option<usize>) -> Result<Vec<u8>, Erreur> {
        octets::lire(self, chemin, max)
    }

    fn tranche(&self, chemin: &str, debut: u64, max: usize) -> Result<Tranche, Erreur> {
        octets::tranche(self, chemin, debut, max)
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
    use super::{en_nanos, url_du_serveur, ClientSmb};
    use crate::library::SourceReseau;

    fn source() -> SourceReseau {
        SourceReseau {
            id: "src".into(),
            nom: "nas".into(),
            hote: "nas.local".into(),
            partage: "music".into(),
            chemin_distant: "/".into(),
            utilisateur: "admin".into(),
            domaine: None,
            invite: false,
            active: true,
        }
    }

    /// **La propriété sur laquelle tient la correction du 2026-11-25.** La connexion en cache
    /// n'est réutilisée que si son empreinte correspond ; si l'un des six champs change, elle
    /// doit être rouverte.
    ///
    /// Ce qu'il manquait coûtait ceci, mesuré sur le NAS de Thomas : une faute de frappe dans
    /// le mot de passe ouvrait une connexion refusée, **et corriger le mot de passe ne
    /// changeait rien** — la connexion fautive restait en cache pour toute la durée de
    /// l'application. Il l'a saisi quatre fois en croyant se tromper.
    ///
    /// Ce que ce test ne prouve pas, et c'est la part irréductible : que `libsmbclient`
    /// refuse bien la connexion. Ce qu'il prouve est que l'app ne la réutilise plus.
    #[test]
    fn un_mot_de_passe_corrige_change_lempreinte_de_la_connexion() {
        let s = source();
        let bon = ClientSmb::nouveau(&s, "le-bon".into());
        let faux = ClientSmb::nouveau(&s, "le-faux".into());
        assert_ne!(
            bon.empreinte(),
            faux.empreinte(),
            "deux mots de passe différents ne doivent pas partager une connexion"
        );
        // Et deux fois les mêmes identifiants réutilisent bien la connexion : sans ça, un
        // scan de 3 137 fichiers rouvrirait une session SMB à chaque fichier.
        assert_eq!(bon.empreinte(), ClientSmb::nouveau(&s, "le-bon".into()).empreinte());
    }

    /// Chacun des six champs compte. Écrit comme une boucle sur les six plutôt que sur celui
    /// qui a mordu : c'est le mot de passe qui a coûté la soirée, mais changer d'utilisateur
    /// ou passer en invité sur la même connexion aurait le même effet.
    #[test]
    fn chacun_des_six_champs_entre_dans_lempreinte() {
        let base = ClientSmb::nouveau(&source(), "mdp".into()).empreinte();
        let variantes: Vec<(&str, SourceReseau)> = vec![
            ("hôte", SourceReseau { hote: "autre.local".into(), ..source() }),
            ("partage", SourceReseau { partage: "photo".into(), ..source() }),
            ("utilisateur", SourceReseau { utilisateur: "invite".into(), ..source() }),
            ("domaine", SourceReseau { domaine: Some("WORKGROUP".into()), ..source() }),
            ("invité", SourceReseau { invite: true, ..source() }),
        ];
        for (quoi, s) in variantes {
            assert_ne!(ClientSmb::nouveau(&s, "mdp".into()).empreinte(), base, "{quoi}");
        }
        // Le sixième : le mot de passe, à source identique.
        assert_ne!(ClientSmb::nouveau(&source(), "autre".into()).empreinte(), base, "mot de passe");
    }

    #[test]
    fn une_adresse_ipv6_nue_est_mise_entre_crochets() {
        assert_eq!(url_du_serveur("nas.local"), "smb://nas.local");
        assert_eq!(url_du_serveur("192.168.1.10"), "smb://192.168.1.10");
        assert_eq!(url_du_serveur("fe80::1"), "smb://[fe80::1]");
        assert_eq!(url_du_serveur("[fe80::1]"), "smb://[fe80::1]");
    }

    #[test]
    fn une_date_absurde_vaut_inconnue_et_ne_panique_pas() {
        assert_eq!(en_nanos(std::time::UNIX_EPOCH), 0);
        assert!(en_nanos(std::time::SystemTime::now()) > 0);
        let avant = std::time::UNIX_EPOCH - std::time::Duration::from_secs(1);
        assert_eq!(en_nanos(avant), 0);
    }
}
