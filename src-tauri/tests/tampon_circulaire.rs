//! Le tampon circulaire entre le décodeur et le callback temps réel.
//!
//! # Pourquoi ces tests sont ici et pas à côté du code
//!
//! Le tampon est la **seule** partie du moteur de sortie que l'intégration continue puisse
//! prouver : tout le reste parle à CoreAudio, donc à du matériel qu'elle n'a pas. Ses tests
//! méritaient donc d'être nombreux, et ils ont fait passer `tampon.rs` au-dessus de la limite
//! de 400 lignes. Les sortir plutôt que les réduire était le seul arbitrage acceptable — et
//! il se trouve qu'il vaut mieux : ils n'utilisent que l'interface publique du tampon, donc
//! ils prouvent au passage qu'elle suffit à s'en servir.
//!
//! # Ce qu'ils tiennent
//!
//! Deux propriétés, et la seconde est celle dont une faute serait **inaudible ici et
//! parfaitement audible chez Thomas** :
//!
//! 1. rien ne se perd, ne se double ni ne se désordonne, y compris entre deux vrais fils ;
//! 2. **les canaux ne s'intervertissent jamais.** Une seule demi-trame publiée suffirait à
//!    faire attaquer le tampon de CoreAudio à l'index 0 avec un échantillon de canal droit,
//!    et L/R resteraient inversés pour tout le reste du flux — sans qu'aucun compteur ne
//!    bouge, puisque le tampon n'est pas à vide : il est décalé.


use hean_lib::audio::coreaudio::tampon::nouveau;

#[test]
fn un_tampon_neuf_ne_rend_rien() {
    let (p, mut c) = nouveau(4, 2);
    let mut sortie = [0i32; 4];
    assert_eq!(c.tirer(&mut sortie), 0);
    assert_eq!(p.libre(), 8);
}

/// LA propriété qui compte : ce qui sort est exactement ce qui est entré, dans l'ordre,
/// sans qu'un seul bit change. Le tampon est sur le chemin bit-perfect.
#[test]
fn ce_qui_sort_est_exactement_ce_qui_est_entre() {
    let (mut p, mut c) = nouveau(8, 2);
    let entree = [i32::MIN, -1, 0, 1, 1 << 16, i32::MAX];
    assert_eq!(p.pousser(&entree), entree.len());
    let mut sortie = [0i32; 6];
    assert_eq!(c.tirer(&mut sortie), 6);
    assert_eq!(sortie, entree);
}

#[test]
fn un_tampon_plein_nen_prend_pas_un_de_plus() {
    let (mut p, mut c) = nouveau(2, 2);
    assert_eq!(p.pousser(&[1, 2, 3, 4, 5, 6]), 4, "seul ce qui rentre est pris");
    assert_eq!(p.libre(), 0);
    assert_eq!(p.pousser(&[7]), 0, "rien n'est écrasé");
    let mut sortie = [0i32; 4];
    c.tirer(&mut sortie);
    assert_eq!(sortie, [1, 2, 3, 4], "les premiers arrivés, intacts");
}

/// L'enroulement : c'est là que vit la faute classique d'un tampon circulaire, et elle
/// ne se voit pas tant qu'on ne fait pas plus d'un tour.
#[test]
fn lordre_survit_a_plusieurs_tours() {
    let (mut p, mut c) = nouveau(5, 1);
    let mut recus = Vec::new();
    let mut sortie = [0i32; 3];
    let mut prochain = 0i32;
    for _ in 0..20 {
        while p.libre() > 0 {
            if p.pousser(&[prochain]) == 1 {
                prochain += 1;
            } else {
                break;
            }
        }
        let n = c.tirer(&mut sortie);
        recus.extend_from_slice(&sortie[..n]);
    }
    let attendu: Vec<i32> = (0..recus.len() as i32).collect();
    assert_eq!(recus, attendu, "aucune perte, aucun doublon, aucun désordre");
    assert!(recus.len() > 5 * 3, "le test doit avoir fait plusieurs tours");
}

#[test]
fn vider_jette_ce_qui_attend_et_avance_la_position() {
    let (mut p, mut c) = nouveau(8, 1);
    p.pousser(&[1, 2, 3, 4, 5]);
    assert_eq!(c.vider(), 5);
    assert_eq!(c.total_lu(), 5, "la position avance : les 5 sont considérés sortis");
    let mut sortie = [0i32; 4];
    assert_eq!(c.tirer(&mut sortie), 0, "plus rien n'attend");
    assert_eq!(p.libre(), 8, "et les cases sont rendues au producteur");
}

#[test]
fn la_jauge_voit_les_deux_bouts() {
    let (mut p, mut c) = nouveau(8, 1);
    let j = p.jauge();
    assert_eq!((j.total_ecrit(), j.total_lu(), j.en_attente()), (0, 0, 0));
    p.pousser(&[1, 2, 3]);
    assert_eq!((j.total_ecrit(), j.total_lu(), j.en_attente()), (3, 0, 3));
    let mut sortie = [0i32; 2];
    c.tirer(&mut sortie);
    assert_eq!((j.total_ecrit(), j.total_lu(), j.en_attente()), (3, 2, 1));
}

/// Deux vrais fils, comme en lecture : le décodeur pousse, le callback tire. Si la
/// synchronisation était fausse, le résultat montrerait un trou, un doublon ou une
/// inversion — c'est-à-dire précisément ce qui s'entendrait comme un craquement.
#[test]
fn deux_fils_concurrents_ne_perdent_ni_ne_doublent_rien() {
    const COMBIEN: i32 = 200_000;
    let (mut p, mut c) = nouveau(1024, 1);

    let producteur = std::thread::spawn(move || {
        let mut prochain = 0i32;
        while prochain < COMBIEN {
            // Des lots de taille variable : un décodeur ne rend pas des blocs réguliers.
            let lot: Vec<i32> = (prochain..(prochain + 97).min(COMBIEN)).collect();
            let mut pousses = 0;
            while pousses < lot.len() {
                pousses += p.pousser(&lot[pousses..]);
                if pousses < lot.len() {
                    std::thread::yield_now();
                }
            }
            prochain += lot.len() as i32;
        }
    });

    let consommateur = std::thread::spawn(move || {
        let mut recus: Vec<i32> = Vec::with_capacity(COMBIEN as usize);
        // Des tailles de lecture qui ne divisent pas la capacité, pour forcer des
        // enroulements à des positions quelconques.
        let mut sortie = [0i32; 61];
        while recus.len() < COMBIEN as usize {
            let n = c.tirer(&mut sortie);
            if n == 0 {
                std::thread::yield_now();
                continue;
            }
            recus.extend_from_slice(&sortie[..n]);
        }
        recus
    });

    producteur.join().expect("fil producteur");
    let recus = consommateur.join().expect("fil consommateur");
    assert_eq!(recus.len(), COMBIEN as usize);
    // Comparer à la suite attendue vérifie d'un coup l'absence de perte, de doublon et
    // de désordre : seule la suite exacte 0, 1, 2… satisfait les trois.
    let attendu: Vec<i32> = (0..COMBIEN).collect();
    assert_eq!(recus, attendu);
}

// ================= L'ALIGNEMENT DES CANAUX =================
// Ces quatre tests tiennent la seule propriété du tampon dont une faute serait
// **inaudible en intégration continue et parfaitement audible chez Thomas** : si une
// demi-trame passait, L et R resteraient intervertis pour tout le reste du flux, sans
// qu'aucun compteur ne bouge — le tampon n'est pas à vide, il est décalé.

/// La capacité se dit en trames, donc elle est multiple du nombre de canaux. C'est la
/// prémisse de tout le reste : avant le 2026-11-26 elle ne tenait qu'au fait qu'une
/// constante de `backend.rs` soit un nombre entier.
#[test]
fn la_capacite_est_toujours_un_multiple_du_nombre_de_canaux() {
    for (trames, canaux) in [(1, 2), (3, 2), (7, 3), (1000, 2), (441, 2)] {
        let (p, _c) = nouveau(trames, canaux);
        assert_eq!(p.capacite(), trames * canaux);
        assert_eq!(p.capacite() % canaux, 0, "{trames} trames x {canaux} canaux");
        assert_eq!(p.canaux(), canaux);
    }
}

/// Une demi-trame offerte n'est pas prise : le producteur garde l'échantillon orphelin
/// pour le prochain appel, où il repartira avec son compagnon de trame.
#[test]
fn pousser_nautorise_jamais_une_demi_trame() {
    let (mut p, _c) = nouveau(8, 2);
    assert_eq!(p.pousser(&[1, 2, 3]), 2, "la trame incomplète attend");
    assert_eq!(p.total_ecrit() % 2, 0);
    assert_eq!(p.pousser(&[3, 4, 5]), 2);
    assert_eq!(p.total_ecrit() % 2, 0);
    // Et sur un tampon presque plein, l'arrondi joue aussi dans l'autre sens.
    let (mut p, _c) = nouveau(2, 3);
    assert_eq!(p.pousser(&[0; 5]), 3, "une seule trame de 3 canaux tient dans 5");
}

/// Et le consommateur non plus, même quand sa destination est de taille impaire : c'est
/// le cas réel, puisque CoreAudio demande un nombre de trames et non d'échantillons.
#[test]
fn tirer_nautorise_jamais_une_demi_trame() {
    let (mut p, mut c) = nouveau(8, 2);
    p.pousser(&[1, 2, 3, 4, 5, 6]);
    let mut sortie = [0i32; 3];
    assert_eq!(c.tirer(&mut sortie), 2, "la place pour 1,5 trame n'en sert qu'une");
    assert_eq!(c.total_lu() % 2, 0);
    assert_eq!(&sortie[..2], &[1, 2]);
}

/// LA propriété, éprouvée de bout en bout : les canaux ne s'intervertissent jamais.
///
/// Les gauches sont pairs, les droites impaires. On pousse et on tire par tranches de
/// tailles **impaires et premières entre elles** avec le nombre de canaux et avec la
/// capacité, pour forcer des arrondis à chaque tour et des enroulements à des positions
/// quelconques. À l'arrivée, tout échantillon d'indice pair doit être une gauche.
#[test]
fn les_canaux_ne_sintervertissent_jamais_quelles_que_soient_les_tailles() {
    const TRAMES: usize = 5_000;
    let (mut p, mut c) = nouveau(37, 2);
    // Trame n : gauche = 2n, droite = 2n + 1. Donc pair = gauche, impair = droite.
    let source: Vec<i32> = (0..TRAMES as i32 * 2).collect();

    let mut ecrit = 0usize;
    let mut recus: Vec<i32> = Vec::with_capacity(source.len());
    let mut sortie = [0i32; 13]; // impair, et ne divise pas 74
    let mut tours = 0;
    while recus.len() < source.len() {
        if ecrit < source.len() {
            let fin = (ecrit + 9).min(source.len()); // 9 : impair aussi
            ecrit += p.pousser(&source[ecrit..fin]);
        }
        let n = c.tirer(&mut sortie);
        recus.extend_from_slice(&sortie[..n]);
        tours += 1;
        assert!(tours < 100_000, "la boucle ne progresse plus");
    }

    assert_eq!(recus.len(), source.len());
    assert_eq!(recus, source, "aucune perte, aucun doublon, aucun décalage");
    for (i, &e) in recus.iter().enumerate() {
        assert_eq!(
            e % 2,
            (i % 2) as i32,
            "l'échantillon {i} appartient au mauvais canal : les voies sont interverties"
        );
    }
}
