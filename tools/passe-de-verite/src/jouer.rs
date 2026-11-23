//! Faire sortir du son par le vrai chemin audio de la V2.
//!
//! # Ce que ce module prouve, et ce qu'il ne prouve pas
//!
//! Il prouve que le **décodage** et la **chaîne** de la V2 produisent, sur un vrai fichier
//! de la bibliothèque de Thomas, un flux PCM qu'une oreille reconnaît comme le morceau.
//! C'est la moitié qu'aucun test ne pouvait donner : les tests comparent des octets, ils
//! n'entendent rien.
//!
//! Il ne prouve **pas** le bit-perfect en sortie de DAC, et il ne peut pas : le backend
//! CoreAudio n'existe pas encore (voir `src-tauri/src/audio/mod.rs`, « ce que ce module ne
//! contient pas encore »). Le seul `BackendAudio` implémenté est `BackendWav`, qui écrit un
//! fichier. Le dernier saut — du PCM vers le périphérique — est donc fait par `afplay`,
//! côté système, et c'est le script appelant qui s'en charge. Prétendre le contraire serait
//! exactement la promesse non mesurée que cette passe existe pour supprimer.
//!
//! La chaîne employée est `Chaine::TRANSPARENTE` (volume à 100 %, aucun traitement), c'est-
//! à-dire le chemin bit-perfect : `BackendWav::nouveau` la prend par défaut.

use std::error::Error;
use std::path::Path;

use hean_lib::audio::{BackendAudio, BackendWav, Decodeur};

pub fn executer(fichier: &Path, sortie: &Path) -> Result<(), Box<dyn Error>> {
    let mut decodeur = Decodeur::ouvrir(fichier)?;
    let format = decodeur.format();

    let mut backend = BackendWav::nouveau(sortie);
    let obtenu = backend.preparer(format)?;
    // La raison d'être du type de retour de `preparer` : constater l'écart au lieu de le
    // supposer absent. Un fichier sert n'importe quel format, donc tout écart ici serait un
    // défaut de la chaîne et non du matériel.
    if obtenu != format {
        return Err(format!("format demandé {format}, obtenu {obtenu}").into());
    }

    let mut echantillons = 0usize;
    while let Some(bloc) = decodeur.suivant()? {
        echantillons += backend.ecrire(&bloc.echantillons)?;
    }
    backend.terminer()?;

    let trames = echantillons / usize::from(format.canaux.max(1));
    let secondes = trames as f64 / f64::from(format.frequence.max(1));
    println!("format : {format}");
    println!("durée décodée : {secondes:.1} s ({trames} trames)");
    println!("chaîne : transparente (volume 100 %, aucun traitement)");
    println!("pcm : {}", sortie.display());
    Ok(())
}
