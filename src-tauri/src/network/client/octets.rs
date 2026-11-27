//! Les octets d'un fichier distant.
//!
//! Module **enfant** de `client`, et pas un voisin : ces fonctions touchent `libsmbclient`,
//! or l'en-tête de `client.rs` dit que ce module est le seul endroit du projet qui y touche.
//! Être son enfant leur donne aussi accès à son verrou et à sa reprise, qui sont privés —
//! c'est-à-dire que PLAT-2 reste tenu par un seul `static`, ici comme avant.

use std::io::{Read, Seek, SeekFrom};

use super::{ClientSmb, Erreur};

/// Une tranche lue, **et la taille totale du fichier dont elle vient**.
///
/// Les deux ensemble parce qu'elles s'obtiennent ensemble : voir `tranche`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tranche {
    pub octets: Vec<u8>,
    pub taille: u64,
}

/// Les octets d'un fichier, au plus `max` s'il est donné. Lit depuis le début.
pub(super) fn lire(
    client: &ClientSmb,
    chemin: &str,
    max: Option<usize>,
) -> Result<Vec<u8>, Erreur> {
    client.avec_reprise(|| {
        client.avec(|smb| {
            let echec = echec_de(client, chemin);
            let fichier = smb
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
    })
}

/// Une tranche à partir de `debut`, et la taille totale, **en une seule ouverture**.
///
/// La taille vient d'un positionnement en fin de fichier fait DANS cette ouverture, et c'est
/// la raison pour laquelle cette fonction rend deux choses au lieu d'une : `pavao::SmbFile`
/// emprunte le client, donc un handle ne survit pas à la sortie du verrou de PLAT-2 et une
/// taille demandée à part coûterait une ouverture de plus par piste (ADR 0022).
///
/// **Lire après la fin n'est pas une panne** : un décodeur qui sonde la fin d'un flux le fait
/// légitimement. On rend zéro octet, ce qui le dit sans fabriquer d'erreur — une erreur ici
/// ferait passer un sondage normal pour un fichier illisible.
pub(super) fn tranche(
    client: &ClientSmb,
    chemin: &str,
    debut: u64,
    max: usize,
) -> Result<Tranche, Erreur> {
    client.avec_reprise(|| {
        client.avec(|smb| {
            let echec = echec_de(client, chemin);
            let mut fichier = smb
                .open_with(chemin, pavao::SmbOpenOptions::default().read(true))
                .map_err(|e| echec(e.to_string()))?;
            let taille = fichier
                .seek(SeekFrom::End(0))
                .map_err(|e| echec(e.to_string()))?;
            if debut >= taille {
                return Ok(Tranche {
                    octets: Vec::new(),
                    taille,
                });
            }
            fichier
                .seek(SeekFrom::Start(debut))
                .map_err(|e| echec(e.to_string()))?;
            let a_lire = (taille - debut).min(max as u64);
            let mut octets = Vec::with_capacity(a_lire as usize);
            fichier
                .take(a_lire)
                .read_to_end(&mut octets)
                .map_err(|e| echec(e.to_string()))?;
            Ok(Tranche { octets, taille })
        })
    })
}

/// L'erreur de fichier, qui nomme l'hôte et le chemin. Écrite une fois : les deux fonctions
/// la fabriquaient à l'identique, et un message qui diverge envoie chercher au mauvais
/// endroit — ce qui a déjà coûté quatre saisies de mot de passe (cf. `erreur.rs`).
fn echec_de<'a>(client: &'a ClientSmb, chemin: &'a str) -> impl Fn(String) -> Erreur + 'a {
    move |detail| Erreur::Fichier {
        hote: client.hote.clone(),
        chemin: chemin.to_string(),
        detail,
    }
}
