//! Les vérifications de la sortie qui demandent du **vrai matériel**.
//!
//! # Pourquoi elles sont `#[ignore]`, et ce que ça ne veut pas dire
//!
//! Elles ne tournent pas dans `./check`, et ce n'est pas de la complaisance : la CI
//! (`macos-latest`) est une machine virtuelle sans périphérique de sortie audio. Un test qui
//! y échouerait pour cause d'absence de matériel serait un rouge permanent, et un rouge
//! permanent finit désarmé — la famille de défaut que l'ADR 0005 refuse par principe.
//!
//! Elles se lancent donc à la main, sur une machine qui a une carte son. Le bloc de
//! lancement complet est dans `docs/PROTOCOLE-SON.md` ; depuis la racine du dépôt :
//!
//! ```bash
//! cargo test --manifest-path src-tauri/Cargo.toml --test sortie_coreaudio -- --ignored
//! ```
//!
//! # Elles ne produisent aucun son et ne changent aucun réglage
//!
//! C'est délibéré, et c'est ce qui les rend lançables sans prévenir : aucune ne démarre
//! l'AudioUnit, aucune n'écrit `kAudioDevicePropertyNominalSampleRate`, aucune ne réserve le
//! périphérique. Elles lisent, et elles ouvrent un flux sans le démarrer. Ce qui demande
//! d'entendre quelque chose est dans `docs/PROTOCOLE-SON.md`, et c'est Thomas qui le fait.

use hean_lib::audio::coreaudio::{cache::CachePeripheriques, flux::Flux, frequence, peripheriques, tampon};
use hean_lib::audio::format::{FormatAudio, Profondeur};

#[test]
#[ignore = "demande une carte son : voir l'en-tête du fichier"]
fn les_peripheriques_de_sortie_se_listent_avec_un_nom() {
    let mut cache = CachePeripheriques::default();
    cache.rafraichir().expect("rafraîchir le cache");
    let liste = cache.liste();
    assert!(!liste.is_empty(), "aucun périphérique de sortie trouvé");
    // Pas de `println!` : INV-21 (🔴) vaut aussi dans `src-tauri/tests/`, et la liste des
    // sorties est de toute façon sous les yeux de Thomas dans l'app — c'est l'étape 1 du
    // protocole. Ce que ce test ajoute, c'est que chacune a bien un nom lisible.
    let sans_nom: Vec<String> = liste
        .iter()
        .filter(|p| p.nom.trim().is_empty())
        .map(|p| p.id.to_string())
        .collect();
    assert!(
        sans_nom.is_empty(),
        "périphériques sans nom (identifiants : {}) — `kAudioObjectPropertyName` ne rend \
         plus ce qu'on croit",
        sans_nom.join(", ")
    );
    assert!(
        liste.iter().any(|p| p.par_defaut),
        "aucun périphérique marqué par défaut"
    );
}

#[test]
#[ignore = "demande une carte son : voir l'en-tête du fichier"]
fn la_frequence_du_peripherique_par_defaut_se_lit_et_fait_partie_de_ses_plages() {
    let id = peripheriques::peripherique_par_defaut().expect("périphérique par défaut");
    let hz = frequence::frequence_du_peripherique(id).expect("lire la fréquence");
    assert!(hz > 0, "fréquence nulle");
    assert!(
        frequence::est_servie(id, hz).expect("lire les plages"),
        "le périphérique tourne à {hz} Hz mais ne l'annonce pas dans ses plages : \
         le lecteur des plages ne lit pas ce qu'on croit"
    );
}

/// **LA vérification qui porte tout le reste.**
///
/// Le format interne de V2 est `i32` (ADR 0007), donc l'AudioUnit est déclarée en entier
/// signé 32 bits et non en flottant comme en V1. Si l'AUHAL refusait ce format, rien ne
/// jouerait — et aucun autre test ne le dirait, puisque tous les autres s'arrêtent avant
/// `AudioUnitSetProperty(kAudioUnitProperty_StreamFormat)`.
///
/// Le flux est ouvert et **jamais démarré** : il n'y a donc aucun son, et la fréquence du
/// périphérique n'est pas touchée.
#[test]
#[ignore = "demande une carte son : voir l'en-tête du fichier"]
fn le_hal_accepte_un_flux_declare_en_entier_32_bits() {
    let id = peripheriques::peripherique_par_defaut().expect("périphérique par défaut");
    let hz = frequence::frequence_du_peripherique(id).expect("lire la fréquence");
    // La fréquence actuelle du périphérique, justement pour n'avoir rien à changer.
    let format = FormatAudio::nouveau(hz, 2, Profondeur::Bits24);
    let (_producteur, consommateur) = tampon::nouveau(4096);
    let flux = Flux::ouvrir(Some(id), format, consommateur);
    let flux = flux.unwrap_or_else(|e| {
        panic!(
            "l'AUHAL a refusé un flux en entier signé 32 bits à {hz} Hz : {e}\n\
             → tout le moteur de sortie en dépend (ADR 0007). Si ce refus est reproductible, \
             c'est la déclaration de format de `audio/coreaudio/flux.rs` qu'il faut revoir, \
             PAS le format interne."
        )
    });
    assert_eq!(
        flux.format().frequence,
        hz,
        "le flux ne s'est pas ouvert à la fréquence demandée"
    );
}
