//! Les erreurs du sous-système réseau, et **ce qu'elles disent à qui les lit**.
//!
//! Elles sont dans leur propre fichier parce que l'une d'elles a coûté une soirée :
//! `libsmbclient` ne dit pas *pourquoi* un `opendir` échoue, et `pavao` traduit tous les cas
//! par « server returned a bad file descriptor ». Devant ce message, Thomas a saisi son mot
//! de passe quatre fois le 2026-11-25 — sans moyen de savoir si c'était le mot de passe, les
//! droits, ou le chemin. Un message d'erreur est une fonctionnalité ; il s'écrit comme telle,
//! et il a sa place.
//!
//! La règle que ce fichier applique : **quand la cause n'est pas récupérable, on ne la
//! devine pas — on nomme les possibilités dans l'ordre où il faut les vérifier.**

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error("connexion à {hote} (partage {partage}) : {detail}")]
    Connexion { hote: String, partage: String, detail: String },
    #[error("lecture du dossier {chemin} sur {hote} : {detail}")]
    Dossier { hote: String, chemin: String, detail: String },
    #[error("lecture du fichier {chemin} sur {hote} : {detail}")]
    Fichier { hote: String, chemin: String, detail: String },
    /// L'échec au **départ** d'un scan, et le seul que l'utilisateur lira sans journal.
    ///
    /// `libsmbclient` ne dit pas pourquoi `opendir` a échoué — `pavao` rend « server
    /// returned a bad file descriptor » pour un accès refusé comme pour un chemin absent.
    /// Alors on ne devine pas : on nomme les trois causes dans l'ordre où il faut les
    /// vérifier, et on dit que le NAS a bien répondu, pour que personne ne cherche du côté
    /// du réseau.
    #[error(
        "le NAS {hote} a répondu, mais le dossier « {chemin} » du partage « {partage} » n'a \
         pas pu être ouvert. Trois causes possibles, dans cet ordre : (1) le mot de passe \
         est refusé — le ressaisir puis relancer le scan ; (2) le compte n'a pas le droit de \
         lire ce dossier ; (3) le dossier n'existe plus sous ce nom. SMB ne dit pas laquelle \
         ({cause})"
    )]
    Racine { hote: String, partage: String, chemin: String, cause: String },
}
