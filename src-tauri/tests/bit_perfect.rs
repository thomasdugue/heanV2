//! « Bit-perfect » cesse d'être une promesse.
//!
//! En V1, la chaîne audio n'avait aucun test : vérifier le bit-perfect demandait un DAC
//! branché et une oreille. Or un défaut de bit-perfect ne fait échouer aucun test — ça
//! joue, un peu moins bien — donc il pouvait vivre indéfiniment sans que personne le voie.
//!
//! Ce fichier décode une fixture FLAC vers un WAV **à travers le backend de test**, chaîne
//! transparente (égaliseur absent, volume à 100 %), et compare le résultat **octet par
//! octet** à un fichier de référence commité.
//!
//! Ce qui donne sa force au test : le golden a été écrit par **ffmpeg**, donc par un
//! décodeur FLAC et un encodeur WAV indépendants des nôtres. Comparer notre sortie à
//! elle-même ne prouverait rien ; la comparer à celle d'un tiers prouve que le pipeline ne
//! modifie aucun bit entre le fichier et la sortie.

use std::fs;
use std::path::PathBuf;

use hean_lib::audio::{decoder_entierement, wav, BackendAudio, BackendWav, FormatAudio, Profondeur};

fn fixture(nom: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/audio")).join(nom)
}

fn temporaire(nom: &str) -> PathBuf {
    std::env::temp_dir().join(format!("hean-{}-{nom}", std::process::id()))
}

/// Localise le premier octet qui diffère et dit **où** il se trouve : dans l'en-tête ou
/// dans l'audio, et à quel échantillon. Un « les fichiers diffèrent » sec obligerait à
/// ressortir un éditeur hexadécimal à chaque échec.
fn premier_ecart(produit: &[u8], golden: &[u8], format: &FormatAudio) -> Option<String> {
    let i = produit.iter().zip(golden).position(|(a, b)| a != b)?;
    // La frontière se DÉDUIT du format : 44 octets en PCM canonique, 68 en extensible
    // au-delà de 16 bits. La coder en dur donnerait un diagnostic faux une fois sur deux.
    let entete = if format.profondeur.bits() > 16 {
        wav::TAILLE_ENTETE_EXTENSIBLE
    } else {
        wav::TAILLE_ENTETE_CANONIQUE
    };
    Some(if i < entete {
        format!(
            "premier écart à l'octet {i}, dans l'EN-TÊTE : produit {:#04x}, attendu {:#04x}",
            produit[i], golden[i]
        )
    } else {
        let trame = (i - entete) / format.octets_par_trame().max(1);
        format!(
            "premier écart à l'octet {i}, dans l'AUDIO (trame {trame}) : \
             produit {:#04x}, attendu {:#04x}",
            produit[i], golden[i]
        )
    })
}

/// Décode `source` vers un WAV par le backend de test, puis compare à `reference`.
fn comparer_au_golden(source: &str, reference: &str, profondeur_attendue: Profondeur) {
    let trames = decoder_entierement(&fixture(source)).expect("le décodage doit réussir");
    assert_eq!(
        trames.format.profondeur, profondeur_attendue,
        "la profondeur lue dans le conteneur n'est pas celle attendue — \
         tout le bit-perfect en dépend"
    );
    assert!(trames.nombre_de_trames() > 0, "aucune trame décodée");

    let sortie = temporaire(reference);
    let mut backend = BackendWav::nouveau(&sortie);

    // Un fichier sert n'importe quel format : il rend donc exactement celui demandé.
    // Le jour où un DAC rendra autre chose, l'écart sera imputable au matériel.
    let obtenu = backend.preparer(trames.format).expect("préparation");
    assert_eq!(obtenu, trames.format, "le backend WAV doit servir le format demandé");

    let ecrits = backend.ecrire(&trames.echantillons).expect("écriture");
    assert_eq!(ecrits, trames.echantillons.len(), "tout doit être consommé");
    backend.terminer().expect("terminaison");

    let produit = fs::read(&sortie).expect("relecture de la sortie");
    let golden = fs::read(fixture(&format!("golden/{reference}"))).expect("lecture du golden");

    assert_eq!(
        produit.len(),
        golden.len(),
        "taille différente du golden : {} octets produits, {} attendus",
        produit.len(),
        golden.len()
    );
    if let Some(ou) = premier_ecart(&produit, &golden, &trames.format) {
        panic!("la sortie n'est pas identique au golden — {ou}");
    }
    let _ = fs::remove_file(&sortie);
}

#[test]
fn un_flac_16_bits_44_1_khz_ressort_identique_octet_pour_octet() {
    comparer_au_golden("flac-16-44.flac", "flac-16-44.wav", Profondeur::Bits16);
}

#[test]
fn un_flac_24_bits_192_khz_ressort_identique_octet_pour_octet() {
    // Le cas d'usage central de l'app, et celui où une troncature à 16 bits serait
    // inaudible sur la plupart des morceaux : exactement le défaut qu'aucune oreille ne
    // garantit d'attraper.
    comparer_au_golden("flac-24-192.flac", "flac-24-192.wav", Profondeur::Bits24);
}

/// Ce test ne vérifie pas notre code mais une **hypothèse sur symphonia** : que ses tampons
/// `S32` portent l'échantillon natif décalé vers les bits hauts. Si une mise à jour changeait
/// cette convention, le son serait 65 536 fois trop faible et aucun test de lecture ne
/// casserait. Ici l'échec est immédiat et nommé.
#[test]
fn la_convention_dechelle_de_symphonia_est_celle_quon_croit() {
    for (nom, profondeur) in [
        ("flac-16-44.flac", Profondeur::Bits16),
        ("flac-24-192.flac", Profondeur::Bits24),
    ] {
        let t = decoder_entierement(&fixture(nom)).expect("décodage");
        let decalage = profondeur.decalage();
        let masque = (1_i32 << decalage) - 1;
        let fautifs = t
            .echantillons
            .iter()
            .filter(|&&e| e & masque != 0)
            .count();
        assert_eq!(
            fautifs, 0,
            "{nom} : {fautifs} échantillons ont des bits sous le décalage de {decalage} — \
             symphonia ne met plus les tampons S32 à l'échelle 32 bits. Le son sortirait \
             {} fois trop faible. Corriger audio/convert.rs.",
            1_i64 << decalage
        );
        // Et le signal n'est pas qu'un silence : un fichier muet passerait le test ci-dessus.
        let max = t.echantillons.iter().map(|e| e.abs()).max().unwrap_or(0);
        assert!(
            max > (1 << 24),
            "{nom} : amplitude maximale {max}, trop faible pour un signal à -3 dBFS — \
             la mise à l'échelle est probablement perdue"
        );
    }
}

/// La chaîne transparente est le chemin bit-perfect ; dès qu'elle ne l'est plus, la sortie
/// doit changer. Sans ce test, un `est_transparente()` qui rendrait toujours `true`
/// passerait les deux tests ci-dessus sans que le volume fonctionne.
#[test]
fn un_volume_qui_nest_pas_a_cent_pour_cent_change_bien_la_sortie() {
    use hean_lib::audio::{Chaine, Volume};

    let trames = decoder_entierement(&fixture("flac-16-44.flac")).expect("décodage");
    let golden = fs::read(fixture("golden/flac-16-44.wav")).expect("golden");

    let sortie = temporaire("volume-moitie.wav");
    let mut backend = BackendWav::avec_chaine(&sortie, Chaine::avec_volume(Volume::nouveau(0.5)));
    backend.preparer(trames.format).expect("préparation");
    backend.ecrire(&trames.echantillons).expect("écriture");
    backend.terminer().expect("terminaison");

    let produit = fs::read(&sortie).expect("relecture");
    assert_eq!(produit.len(), golden.len(), "même taille, seul le contenu change");
    assert_ne!(
        produit, golden,
        "un volume à 50 % doit produire autre chose que le golden — \
         sinon la chaîne est court-circuitée"
    );
    let _ = fs::remove_file(&sortie);
}
