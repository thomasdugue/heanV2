//! Ce que les fixtures prouvent en dehors du bit-perfect.
//!
//! Chaque fixture correspond à un piège réel de V1 (`fixtures/audio/README.md`). Un fichier
//! commité qu'aucun test ne lit n'est pas une fixture, c'est du poids mort : ce fichier est
//! ce qui fait d'elles un actif.

use std::path::PathBuf;

use hean_lib::audio::{decoder_entierement, Decodeur, ErreurAudio, Profondeur};

fn fixture(nom: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/audio")).join(nom)
}

/// Les sept formats annoncés comme supportés le sont réellement.
///
/// V1 annonçait la liste dans un document ; ici elle est exécutée. Les features de
/// symphonia sont déclarées une à une dans `Cargo.toml`, donc en retirer une ferait échouer
/// ce test au lieu de dégrader l'app en silence chez un testeur.
#[test]
fn les_sept_formats_annonces_se_decodent_vraiment() {
    for nom in [
        "flac-16-44.flac",
        "wav.wav",
        "aiff.aiff",
        "alac.m4a",
        "mp3-320.mp3",
        "aac.m4a",
        "vorbis.ogg",
    ] {
        let t = decoder_entierement(&fixture(nom))
            .unwrap_or_else(|e| panic!("{nom} devrait se décoder : {e}"));
        assert_eq!(t.format.frequence, 44_100, "{nom} : fréquence");
        assert_eq!(t.format.canaux, 2, "{nom} : canaux");
        assert!(t.nombre_de_trames() > 1_000, "{nom} : trop peu de trames");
        // Un décodage qui ne rendrait que du silence passerait les assertions ci-dessus.
        let max = t.echantillons.iter().map(|e| e.abs()).max().unwrap_or(0);
        assert!(max > 1 << 20, "{nom} : signal quasi muet (max {max})");
    }
}

/// Les formats sans perte déclarent leur profondeur ; ALAC et les formats avec perte non.
/// C'est ce qui justifie `Profondeur::SANS_DECLARATION` : un repli explicite, et non deviné.
#[test]
fn seuls_les_formats_qui_declarent_leur_profondeur_la_donnent() {
    for (nom, attendue) in [
        ("flac-16-44.flac", Profondeur::Bits16),
        ("flac-24-192.flac", Profondeur::Bits24),
        ("wav.wav", Profondeur::Bits16),
        ("aiff.aiff", Profondeur::Bits16),
    ] {
        let t = decoder_entierement(&fixture(nom)).expect("décodage");
        assert_eq!(t.format.profondeur, attendue, "{nom}");
    }
    for nom in ["alac.m4a", "mp3-320.mp3", "aac.m4a", "vorbis.ogg"] {
        let t = decoder_entierement(&fixture(nom)).expect("décodage");
        assert_eq!(
            t.format.profondeur,
            Profondeur::SANS_DECLARATION,
            "{nom} : sans profondeur déclarée, le repli doit être celui documenté"
        );
    }
}

// --- Les trois visages d'un fichier abîmé -----------------------------------------------
//
// Mesuré sur symphonia, et CONTRE L'ATTENTE : sur les trois façons dont un fichier audio
// s'abîme dans la vraie vie, une seule produit une erreur. Le README des fixtures de
// phase 0 annonçait « doit produire une erreur propre » pour la troncature — c'était faux,
// et personne ne pouvait le savoir avant d'essayer. Une phrase dans un document ne peut
// pas échouer ; ces trois tests, si.
//
// Conséquence de conception : L'INTÉGRITÉ D'UN FICHIER NE SE DÉDUIT PAS DU DÉCODEUR. Quand
// la lecture depuis le NAS arrivera (phase 5), il faudra la vérifier autrement — la durée
// annoncée par le conteneur contre le nombre de trames réellement rendues.

/// Seul cas qui échoue : l'en-tête abîmé. Et il échoue dès la sonde, donc avant toute
/// lecture d'audio — un panic ici arrêterait la lecture sans message pour le testeur.
#[test]
fn un_entete_abime_est_refuse_avec_un_message() {
    let e = decoder_entierement(&fixture("corrupt-entete.flac"))
        .expect_err("un STREAMINFO incohérent doit être refusé");
    assert!(!e.to_string().is_empty(), "le message doit être exploitable");
    assert!(
        matches!(e, ErreurAudio::FormatInconnu { .. } | ErreurAudio::FluxCorrompu(_)),
        "erreur inattendue : {e:?}"
    );
}

/// **Des trames abîmées ne produisent AUCUNE erreur** : symphonia saute les trames
/// invalides et continue. Mesuré : un seul octet abîmé suffit à faire disparaître un bloc
/// entier — 4 608 trames, un dixième de seconde — sans que rien le signale.
///
/// Ce test ne décrit pas ce qu'on voudrait, il fixe ce qui est. Le jour où l'app devra
/// prévenir qu'un fichier est incomplet, c'est ce test qu'il faudra changer — et son échec
/// sera alors le signal que le nouveau garde-fou fonctionne.
#[test]
fn des_trames_abimees_perdent_de_laudio_en_silence_et_cest_une_limite_connue() {
    let abime = decoder_entierement(&fixture("corrupt-trames.flac"))
        .expect("symphonia ne signale PAS des trames abîmées");
    let sain = decoder_entierement(&fixture("flac-16-44.flac")).expect("fichier sain");
    assert!(
        abime.nombre_de_trames() > 0,
        "ce qui reste lisible doit tout de même sortir"
    );
    assert!(
        abime.nombre_de_trames() < sain.nombre_de_trames(),
        "de l'audio doit avoir disparu : {} trames contre {} — si les deux étaient égaux, \
         la fixture n'abîmerait pas de vraies trames (une première version visait le bloc \
         PADDING et ne prouvait rien)",
        abime.nombre_de_trames(),
        sain.nombre_de_trames()
    );
}

/// Un fichier tronqué — une copie interrompue depuis le NAS — ne produit pas d'erreur non
/// plus : il joue simplement plus court, sans que rien le signale.
#[test]
fn un_fichier_tronque_se_decode_sans_erreur_mais_plus_court() {
    let tronque = decoder_entierement(&fixture("corrupt-tronque.flac"))
        .expect("la troncature n'est PAS signalée par symphonia");
    let entier = decoder_entierement(&fixture("flac-16-44.flac")).expect("fichier sain");
    assert!(
        tronque.nombre_de_trames() > 0,
        "l'audio lisible doit tout de même sortir"
    );
    assert!(
        tronque.nombre_de_trames() < entier.nombre_de_trames(),
        "le fichier tronqué doit rendre moins de trames que le fichier entier"
    );
}

#[test]
fn un_fichier_absent_produit_une_erreur_de_lecture_nommee() {
    let e = decoder_entierement(&fixture("ce-fichier-nexiste-pas.flac"))
        .expect_err("un fichier absent ne peut pas se décoder");
    assert!(matches!(e, ErreurAudio::Lecture { .. }), "{e:?}");
}

/// Une pochette embarquée ajoute une piste image au conteneur. `default_track()` pourrait
/// la désigner ; le décodeur retient donc la première piste **audio**.
#[test]
fn une_pochette_embarquee_ne_fait_pas_prendre_limage_pour_laudio() {
    let t = decoder_entierement(&fixture("huge-cover.flac")).expect("décodage");
    assert!(t.nombre_de_trames() > 1_000, "l'audio doit être trouvé");
    assert_eq!(t.format.canaux, 1);
}

#[test]
fn un_fichier_sans_aucun_tag_se_decode_quand_meme() {
    // Les métadonnées sont le problème de la bibliothèque, pas du moteur audio : leur
    // absence ne doit pas empêcher de jouer.
    let t = decoder_entierement(&fixture("empty-tags.flac")).expect("décodage");
    assert!(t.nombre_de_trames() > 1_000);
}

/// La paire gapless est découpée à l'échantillon exact dans un même signal continu : la
/// concaténation des deux décodages doit redonner **exactement** le fichier entier.
///
/// C'est la référence qui manquait à V1, où le bug gapless connu (`currentTrackIndex`
/// inchangé quand la vue Pistes n'avait jamais été rendue) ne se voyait qu'à l'usage.
#[test]
fn la_paire_gapless_se_recolle_a_lechantillon_exact() {
    let a = decoder_entierement(&fixture("gapless-a.flac")).expect("A");
    let b = decoder_entierement(&fixture("gapless-b.flac")).expect("B");
    let entier = decoder_entierement(&fixture("flac-16-44.flac")).expect("entier");

    let mut recolle = a.echantillons.clone();
    recolle.extend_from_slice(&b.echantillons);
    assert_eq!(
        recolle.len(),
        entier.echantillons.len(),
        "A + B doit faire la longueur du fichier entier"
    );
    assert_eq!(recolle, entier.echantillons, "A + B doit redonner le signal exact");
}

/// Le saut d'amplitude à la jonction n'est pas plus grand qu'ailleurs dans le signal.
///
/// Ce test ne prouve rien de plus que le précédent **aujourd'hui** — mais c'est lui qui
/// attrapera un défaut de moteur le jour où le gapless sera implémenté : un échantillon
/// perdu, dupliqué, ou un tampon vidé entre deux pistes produisent un saut, et un saut
/// s'entend comme un clic.
#[test]
fn la_jonction_gapless_ne_presente_aucune_discontinuite() {
    let a = decoder_entierement(&fixture("gapless-a.flac")).expect("A");
    let b = decoder_entierement(&fixture("gapless-b.flac")).expect("B");
    let canaux = a.format.canaux as usize;

    let mut tout = a.echantillons.clone();
    tout.extend_from_slice(&b.echantillons);

    let saut = |i: usize| (i64::from(tout[i]) - i64::from(tout[i - canaux])).abs();
    let jonction = a.echantillons.len();
    let saut_jonction = (0..canaux).map(|c| saut(jonction + c)).max().unwrap_or(0);
    let saut_max_ailleurs = (canaux..tout.len())
        .filter(|i| !(jonction..jonction + canaux).contains(i))
        .map(saut)
        .max()
        .unwrap_or(0);

    assert!(
        saut_jonction <= saut_max_ailleurs,
        "discontinuité à la jonction : saut {saut_jonction} contre {saut_max_ailleurs} \
         au maximum ailleurs dans le signal"
    );
}

/// Le décodeur rend des blocs : le moteur ne chargera pas un album de 24/192 en mémoire.
#[test]
fn le_decodeur_rend_plusieurs_blocs_et_leur_somme_fait_le_tout() {
    let mut d = Decodeur::ouvrir(&fixture("flac-24-192.flac")).expect("ouverture");
    let mut blocs = 0;
    let mut total = 0;
    while let Some(bloc) = d.suivant().expect("bloc") {
        assert!(!bloc.echantillons.is_empty(), "aucun bloc vide ne doit sortir");
        blocs += 1;
        total += bloc.echantillons.len();
    }
    assert!(blocs > 1, "un seul bloc : le décodage n'est pas en flux");
    let entier = decoder_entierement(&fixture("flac-24-192.flac")).expect("entier");
    assert_eq!(total, entier.echantillons.len());
}

/// Des métadonnées inhabituelles, ou un nom de fichier accentué, n'empêchent pas de jouer.
///
/// Ces quatre fixtures existent d'abord pour la bibliothèque — la normalisation de la clé
/// d'album (INV-01), la déduplication des variations « feat. » (régression W003), le repli
/// du pipeline de covers (INV-12). Leur raison d'être sera vraiment éprouvée en phase 2.
///
/// Mais il y a ici une question qui est bien celle du moteur, et V1 l'avait ratée : un
/// fichier dont le NOM porte des accents doit s'ouvrir. Sur macOS, un nom peut être stocké
/// en NFD sur le disque et demandé en NFC par le code — si l'ouverture échouait, la piste
/// serait simplement absente de la bibliothèque, sans message.
#[test]
fn des_metadonnees_inhabituelles_nempechent_pas_de_decoder() {
    for nom in [
        "tags-nfd.flac",
        "artist-feat.flac",
        "accents-éàü.flac",
        "no-cover.flac",
    ] {
        let t = decoder_entierement(&fixture(nom))
            .unwrap_or_else(|e| panic!("{nom} doit se décoder malgré ses tags : {e}"));
        assert!(t.nombre_de_trames() > 1_000, "{nom} : audio trop court");
        assert_eq!(t.format.frequence, 44_100, "{nom}");
    }
}

/// Les signaux connus servent aux assertions sur le volume et l'égaliseur (HARNAIS.md §1).
#[test]
fn les_signaux_connus_sont_bien_ce_quils_annoncent() {
    let silence = decoder_entierement(&fixture("silence.flac")).expect("silence");
    assert!(
        silence.echantillons.iter().all(|&e| e == 0),
        "le silence doit être exactement nul"
    );

    let dc = decoder_entierement(&fixture("dc.flac")).expect("dc");
    let premier = dc.echantillons[0];
    assert_ne!(premier, 0, "le DC n'est pas un silence");
    assert!(
        dc.echantillons.iter().all(|&e| e == premier),
        "le DC doit être une constante"
    );

    let sinus = decoder_entierement(&fixture("sine-997.flac")).expect("sinus");
    // Un sinus traverse le zéro : il a des valeurs des deux signes, contrairement au DC.
    assert!(sinus.echantillons.iter().any(|&e| e > 0));
    assert!(sinus.echantillons.iter().any(|&e| e < 0));
}
