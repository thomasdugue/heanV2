//! Parcourir un partage SMB et rendre ce que la bibliothèque doit en savoir.
//!
//! Ce module **ne parle pas à SQLite** et ne connaît pas `libsmbclient` : il reçoit un
//! `&dyn Smb` et la liste des chemins qu'il est inutile de relire, et il rend un inventaire.
//! C'est ce qui le rend éprouvable sans le NAS de Thomas — qui n'est pas toujours allumé, et
//! que la CI ne voit pas.
//!
//! **Il tourne sans aucun verrou de bibliothèque** (ADR 0016 §5). C'est la raison de sa
//! forme : un scan de 3 137 pistes dure des minutes, et pendant ces minutes `list_tracks`
//! doit continuer de répondre.
//!
//! Ce qu'il ne fait pas, et qui n'est pas un oubli : il ne décide pas ce qui entre dans la
//! bibliothèque. Comme `library::scan`, il **trouve** et rien de plus — la décision
//! appartient au dépôt, et à lui seul (INV-06).

use std::collections::{HashMap, HashSet};

use super::client::{Erreur, Smb};
use super::{entete, uri};
use crate::library::scan::{est_audio, Entree, Perimetre, Trouvee};
use crate::library::{metadonnees, Depot, SourceReseau};

/// Ce que la bibliothèque sait déjà de ce périmètre, et qui décide **quoi relire**.
///
/// Les deux listes ne jouent pas le même rôle, et les confondre coûterait cher :
/// une piste **connue** est vue sans être relue (donc elle n'est pas effacée), une piste
/// **exclue** n'est ni vue ni relue (son exclusion est permanente — INV-06).
pub struct DejaConnu {
    /// URI → (taille, date) telles que la base les a enregistrées.
    pub empreintes: HashMap<String, (i64, i64)>,
    pub exclus: HashSet<String>,
}

impl DejaConnu {
    /// Lu en une fois, sous verrou **court**, avant que le parcours ne commence (ADR 0016 §5).
    pub fn depuis_le_depot(depot: &Depot, perimetre: &Perimetre) -> Result<Self, crate::library::repository::Erreur> {
        Ok(DejaConnu {
            empreintes: depot
                .empreintes_connues(perimetre)?
                .into_iter()
                .map(|(p, t, m)| (p, (t, m)))
                .collect(),
            exclus: depot.chemins_exclus()?.into_iter().collect(),
        })
    }

    /// Vrai si cette piste est connue **et** que rien n'indique qu'elle a changé.
    ///
    /// Deux cas, et c'est le second qui demande d'être écrit : quand le serveur n'a donné
    /// **ni taille ni date** (pas de `readdirplus`), il n'y a rien à comparer, et la
    /// présence suffit — c'est le comportement de V1, et le seul possible sans un
    /// aller-retour par fichier. Quand il les donne, elles sont comparées comme sur le
    /// disque, donc un fichier modifié depuis une autre machine est bien relu.
    fn inchangee(&self, uri: &str, vue: &Trouvee) -> bool {
        let Some(&(taille, mtime)) = self.empreintes.get(uri) else {
            return false;
        };
        if vue.taille == 0 && vue.mtime_ns == 0 {
            return true;
        }
        taille == i64::try_from(vue.taille).unwrap_or(i64::MAX) && mtime == vue.mtime_ns
    }
}

/// Ce qu'un parcours a rapporté.
///
/// `Debug` ne montre pas les entrées une par une — 3 137 pistes dans un message d'erreur
/// n'aident personne — mais les trois comptes, qui sont ce qu'on veut lire dans un journal.
pub struct Inventaire {
    /// Une entrée par fichier audio vu, qu'il ait été relu ou non.
    pub entrees: Vec<Entree>,
    /// Les fichiers **nouveaux** dont les tags ne se lisent pas. Comptés et non fatals : un
    /// fichier abîmé au milieu de 3 000 ne doit pas interrompre le scan. Un fichier déjà
    /// connu n'est jamais relu, donc jamais compté ici.
    pub illisibles: u32,
    /// Les dossiers que le serveur a refusé de lister. Comptés aussi, parce qu'un dossier
    /// sauté veut dire des pistes manquantes, et qu'un scan qui en saute un tiers sans le
    /// dire ressemble à un scan qui a marché.
    pub dossiers_illisibles: u32,
}

impl std::fmt::Debug for Inventaire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inventaire")
            .field("vues", &self.entrees.len())
            .field("illisibles", &self.illisibles)
            .field("dossiers_illisibles", &self.dossiers_illisibles)
            .finish()
    }
}

/// Les noms qu'un partage SMB expose et qui ne sont pas de la musique.
///
/// `@eaDir` (Synology), `#recycle` et `$RECYCLE.BIN` contiennent des **copies** de fichiers
/// audio : les scanner doublerait la bibliothèque, pas seulement la ralentirait. Repris de
/// V1, où la liste a été construite sur le NAS de Thomas.
fn a_ignorer(nom: &str) -> bool {
    nom.starts_with('.')
        || nom.starts_with('@')
        || nom.starts_with('#')
        || nom.eq_ignore_ascii_case("$RECYCLE.BIN")
}

/// Parcourt la source et rend son inventaire.
///
/// `deja` porte ce que la base sait déjà : une piste connue et inchangée est vue sans être
/// relue. C'est le scan différentiel d'une source distante (ADR 0016 §4), et c'est ce qui
/// fait qu'un deuxième scan ne coûte que le parcours des dossiers.
///
/// Une erreur n'est rendue que si **la racine** ne se liste pas : sans elle on ne sait rien,
/// et conclure « zéro fichier » ferait effacer la source entière à l'application (INV-27 ne
/// protège que du périmètre voisin, pas de ça). Un sous-dossier illisible, lui, est journalisé,
/// compté et sauté — c'est le cas B17 appliqué au réseau.
pub fn inventorier(
    smb: &dyn Smb,
    source: &SourceReseau,
    deja: &DejaConnu,
) -> Result<Inventaire, Erreur> {
    let depart = uri::depart(source);
    // **Le message compte autant que l'échec, et ça s'est payé le 2026-11-25.** Quand la
    // racine ne se liste pas, `libsmbclient` rend un `opendir` nul et `pavao` le traduit par
    // « server returned a bad file descriptor » — une phrase qui ne dit **ni** que l'accès a
    // été refusé, **ni** que le chemin pourrait ne pas exister. Thomas a saisi son mot de
    // passe quatre fois devant ce message. La cause n'est pas récupérable depuis pavao, donc
    // on ne la devine pas : on **nomme les trois possibilités**, dans l'ordre où il faut les
    // vérifier, et on dit ce qui a quand même été joint.
    let racine = smb.lister(&depart).map_err(|cause| Erreur::Racine {
        hote: source.hote.clone(),
        partage: source.partage.clone(),
        chemin: depart.clone(),
        cause: cause.to_string(),
    })?;
    let mut inv = Inventaire {
        entrees: Vec::new(),
        illisibles: 0,
        dossiers_illisibles: 0,
    };
    descendre(smb, source, &depart, racine, deja, &mut inv);
    tracing::info!(
        source = %source.nom,
        vues = inv.entrees.len(),
        illisibles = inv.illisibles,
        dossiers_illisibles = inv.dossiers_illisibles,
        "inventaire d'une source réseau terminé"
    );
    Ok(inv)
}

fn descendre(
    smb: &dyn Smb,
    source: &SourceReseau,
    chemin: &str,
    entrees: Vec<super::client::Entree>,
    deja: &DejaConnu,
    inv: &mut Inventaire,
) {
    for e in entrees {
        if a_ignorer(&e.nom) {
            continue;
        }
        let enfant = format!("{}/{}", chemin.trim_end_matches('/'), e.nom);
        if e.dossier {
            match smb.lister(&enfant) {
                Ok(sous) => descendre(smb, source, &enfant, sous, deja, inv),
                Err(erreur) => {
                    tracing::warn!(chemin = %enfant, erreur = %erreur, "dossier distant illisible, sauté");
                    inv.dossiers_illisibles += 1;
                }
            }
            continue;
        }
        if !est_audio(std::path::Path::new(&e.nom)) {
            continue;
        }
        let uri = uri::de_fichier(&source.id, &source.partage, &enfant);
        let mut trouvee = uri::en_trouvee(&uri);
        // Taille et date quand le serveur les a données (readdirplus). Zéro sinon, ce qui
        // fait que la comparaison d'empreintes du dépôt conclut « inchangée » — le
        // comportement de V1, et le seul possible sans un aller-retour par fichier.
        trouvee.taille = e.taille;
        trouvee.mtime_ns = e.mtime_ns;

        // Une piste exclue n'est ni vue ni relue : son exclusion est permanente (INV-06),
        // et la lire coûterait un aller-retour réseau pour un résultat que le dépôt jette.
        if deja.exclus.contains(&uri) {
            continue;
        }
        if deja.inchangee(&uri, &trouvee) {
            inv.entrees.push(Entree::Distante(trouvee, None));
            continue;
        }
        match lire_les_tags(smb, &source.hote, &enfant, &uri) {
            Ok(lues) => inv.entrees.push(Entree::Distante(trouvee, Some(Box::new(lues)))),
            Err(erreur) => {
                tracing::warn!(uri = %uri, erreur = %erreur, "tags illisibles sur le NAS, piste ignorée");
                inv.illisibles += 1;
            }
        }
    }
}

/// Lit les tags d'un fichier distant : l'en-tête quand le format le permet, et **le fichier
/// entier en repli** si cet en-tête ne se parse pas.
///
/// L'hôte est passé jusqu'ici pour que le message d'erreur le **nomme**. Sans lui, un
/// échec de lecture de tags disait « lecture du fichier … sur  : … » — un message à trou,
/// c'est-à-dire un message qui envoie chercher au mauvais endroit.
///
/// Le repli est ce qui remplace le parseur de blocs FLAC de V1 (~200 lignes écrites parce
/// que lofty échouait sur un FLAC tronqué à 512 Kio). Il est journalisé à chaque fois : si
/// la mesure montre qu'il se déclenche souvent, c'est **elle** qui justifiera d'écrire ces
/// 200 lignes, pas la crainte de devoir le faire.
fn lire_les_tags(
    smb: &dyn Smb,
    hote: &str,
    chemin: &str,
    uri: &str,
) -> Result<metadonnees::Lues, Erreur> {
    let max = entete::a_lire(chemin);
    let octets = smb.lire(chemin, max)?;
    match metadonnees::lire_depuis(&octets, uri) {
        Ok(lues) => Ok(lues),
        Err(partielle) if max.is_some() => {
            tracing::debug!(uri, erreur = %partielle, "en-tête insuffisant, relecture du fichier entier");
            let tout = smb.lire(chemin, None)?;
            metadonnees::lire_depuis(&tout, uri).map_err(|e| Erreur::Fichier {
                hote: hote.to_string(),
                chemin: uri.to_string(),
                detail: e.to_string(),
            })
        }
        Err(e) => Err(Erreur::Fichier {
            hote: hote.to_string(),
            chemin: uri.to_string(),
            detail: e.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::a_ignorer;

    /// Le reste de ce module s'éprouve dans `src-tauri/tests/reseau/`, contre un faux
    /// partage **partagé** avec les autres tests réseau. Ce n'est pas un détail
    /// d'organisation : un faux de plus par fichier de test, c'est un faux qui dérive de la
    /// forme réelle du trait sans que rien ne rougisse — la famille de défaut que ce projet
    /// paie le plus cher. Ici ne reste que ce qui n'a besoin de rien.
    #[test]
    fn les_noms_a_ignorer_sont_ceux_quun_nas_expose() {
        for nom in ["@eaDir", "#recycle", "#snapshot", "$RECYCLE.BIN", ".DS_Store", "._a.flac"] {
            assert!(a_ignorer(nom), "{nom}");
        }
        for nom in ["album", "a.flac", "Café", "2 Many DJs"] {
            assert!(!a_ignorer(nom), "{nom}");
        }
    }
}
