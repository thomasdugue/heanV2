//! Un partage SMB en mémoire, **partagé par tous les tests réseau**.
//!
//! Pourquoi un seul, et pas un par fichier de test : il implémente `network::client::Smb`,
//! le trait réel. Un changement de signature côté `client.rs` casse donc la compilation ici,
//! une fois — alors que trois faux écrits à la main dériveraient chacun de leur côté sans
//! que rien ne rougisse. C'est le même raisonnement que `reprise_v1::config_de_v1` : un faux
//! qui dérive de la vraie chose est un harnais, un faux libre de dériver est un décor.
//!
//! Il sait aussi **tomber en panne** : `refuser` fait échouer la liste d'un dossier, ce qui
//! est la seule façon d'éprouver ce que fait le scan d'un NAS qui répond mal.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use hean_lib::network::client::{Entree, Erreur, Smb};

pub struct FauxPartage {
    /// chemin de dossier → (nom, est-ce un dossier, taille, mtime en nanosecondes)
    dossiers: HashMap<String, Vec<(String, bool, u64, i64)>>,
    fichiers: HashMap<String, Vec<u8>>,
    refuses: HashSet<String>,
    /// Les chemins lus, dans l'ordre. C'est ce qui permet de prouver qu'une lecture n'a
    /// **pas** eu lieu — une absence ne se voit pas autrement (le scan différentiel).
    lectures: RefCell<Vec<String>>,
    /// Les tailles demandées, pour prouver qu'un en-tête est bien un en-tête.
    tailles_demandees: RefCell<Vec<Option<usize>>>,
}

impl FauxPartage {
    pub fn nouveau() -> Self {
        FauxPartage {
            dossiers: HashMap::new(),
            fichiers: HashMap::new(),
            refuses: HashSet::new(),
            lectures: RefCell::new(Vec::new()),
            tailles_demandees: RefCell::new(Vec::new()),
        }
    }

    /// Déclare un dossier et son contenu. Taille et date valent zéro, c'est-à-dire
    /// « inconnues » — le cas d'un serveur sans `readdirplus`, qui est aussi ce que V1
    /// observait sur le NAS de Thomas (ses 3 137 entrées de cache portent `size: 0`).
    pub fn dossier(mut self, chemin: &str, contenu: &[(&str, bool)]) -> Self {
        self.dossiers.insert(
            chemin.to_string(),
            contenu.iter().map(|(n, d)| ((*n).to_string(), *d, 0, 0)).collect(),
        );
        self
    }

    /// Un dossier dont le serveur donne taille et date (le cas `readdirplus`).
    pub fn dossier_avec_stats(mut self, chemin: &str, contenu: &[(&str, bool, u64, i64)]) -> Self {
        self.dossiers.insert(
            chemin.to_string(),
            contenu.iter().map(|(n, d, t, m)| ((*n).to_string(), *d, *t, *m)).collect(),
        );
        self
    }

    pub fn fichier(mut self, chemin: &str, octets: Vec<u8>) -> Self {
        self.fichiers.insert(chemin.to_string(), octets);
        self
    }

    /// Fait échouer la liste de ce dossier — l'injection de panne.
    pub fn refuser(mut self, chemin: &str) -> Self {
        self.refuses.insert(chemin.to_string());
        self
    }

    pub fn lectures(&self) -> Vec<String> {
        self.lectures.borrow().clone()
    }

    pub fn tailles_demandees(&self) -> Vec<Option<usize>> {
        self.tailles_demandees.borrow().clone()
    }
}

impl Smb for FauxPartage {
    fn lister(&self, chemin: &str) -> Result<Vec<Entree>, Erreur> {
        if self.refuses.contains(chemin) {
            return Err(Erreur::Dossier {
                hote: "faux".into(),
                chemin: chemin.into(),
                detail: "accès refusé".into(),
            });
        }
        let contenu = self.dossiers.get(chemin).ok_or_else(|| Erreur::Dossier {
            hote: "faux".into(),
            chemin: chemin.into(),
            detail: "introuvable".into(),
        })?;
        Ok(contenu
            .iter()
            .map(|(nom, dossier, taille, mtime_ns)| Entree {
                nom: nom.clone(),
                dossier: *dossier,
                taille: *taille,
                mtime_ns: *mtime_ns,
            })
            .collect())
    }

    fn lire(&self, chemin: &str, max: Option<usize>) -> Result<Vec<u8>, Erreur> {
        self.lectures.borrow_mut().push(chemin.to_string());
        self.tailles_demandees.borrow_mut().push(max);
        let octets = self.fichiers.get(chemin).ok_or_else(|| Erreur::Fichier {
            hote: "faux".into(),
            chemin: chemin.into(),
            detail: "introuvable".into(),
        })?;
        Ok(match max {
            Some(n) => octets.iter().copied().take(n).collect(),
            None => octets.clone(),
        })
    }
}

/// Le partage **par référence**, pour que le test garde la main dessus pendant que
/// `network::scanner_avec` le consomme.
///
/// Cette fonction-là est la raison d'être de ce bloc : `scanner_avec` prend un
/// `Box<dyn Smb>`, donc sans ce renvoi il faudrait céder le faux à la boîte — et on ne
/// pourrait plus lui demander ce qu'il a lu une fois le scan fini. Or compter les lectures
/// est ce qui prouve qu'une lecture **n'a pas** eu lieu.
impl Smb for &FauxPartage {
    fn lister(&self, chemin: &str) -> Result<Vec<Entree>, Erreur> {
        (**self).lister(chemin)
    }

    fn lire(&self, chemin: &str, max: Option<usize>) -> Result<Vec<u8>, Erreur> {
        (**self).lire(chemin, max)
    }
}
