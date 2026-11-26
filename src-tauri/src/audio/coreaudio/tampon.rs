//! Le tampon circulaire entre le décodeur et le callback de rendu.
//!
//! # Pourquoi écrit à la main, et pas une dépendance
//!
//! V1 prenait `ringbuf`. Ici c'est 120 lignes, et ces 120 lignes sont **la seule partie du
//! moteur de sortie que la CI peut prouver** : tout le reste parle à CoreAudio, donc ne
//! s'éprouve qu'avec un DAC branché. Un tampon mal réglé est exactement ce qui fait
//! « couper » une lecture — le défaut que Thomas entendrait et qu'aucun test ne verrait. Le
//! sortir d'une dépendance aurait mis cette preuve hors de portée pour économiser 120 lignes.
//!
//! # La contrainte qui dicte la forme : le callback est temps réel
//!
//! Le callback de rendu de CoreAudio tourne sur un fil temps réel. Il ne doit **jamais**
//! allouer, ni prendre un verrou, ni appeler quoi que ce soit qui puisse attendre : un
//! blocage de quelques millisecondes se traduit par un trou dans le son. D'où un seul
//! producteur, un seul consommateur, et deux compteurs atomiques — pas un `Mutex`.
//!
//! Les compteurs sont **absolus** et ne reviennent jamais en arrière. C'est ce qui lève
//! l'ambiguïté classique entre « plein » et « vide » sans sacrifier une case, et ça donne en
//! prime la position de lecture : le nombre d'échantillons sortis depuis le début. À
//! 384 kHz en stéréo, un `usize` de 64 bits déborderait après environ 1,5 million d'années.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct Interieur {
    /// Les cases. Seuls le producteur (dans `[ecrit, lu + capacite)`) et le consommateur
    /// (dans `[lu, ecrit)`) y touchent, et ces deux intervalles sont disjoints par
    /// construction — c'est ce qui rend l'`UnsafeCell` sûr ici.
    cases: UnsafeCell<Box<[i32]>>,
    capacite: usize,
    /// Nombre total d'échantillons jamais écrits. Publié en `Release` par le producteur.
    ecrit: AtomicUsize,
    /// Nombre total d'échantillons jamais lus. Publié en `Release` par le consommateur.
    lu: AtomicUsize,
}

// SÛRETÉ : `Interieur` est partagé entre trois rôles, et un seul écrit dans chaque zone.
// Le producteur n'écrit que dans les cases libres, qu'il détermine à partir de `lu` lu en
// `Acquire` ; le consommateur ne lit que les cases pleines, déterminées à partir de `ecrit`
// lu en `Acquire`. Les deux intervalles ne se recouvrent jamais, et les paires
// `Release`/`Acquire` garantissent que les données écrites avant la publication d'un
// compteur sont visibles après sa lecture. La `Jauge`, troisième rôle, ne touche QUE les
// deux atomiques et jamais les cases.
unsafe impl Send for Interieur {}
unsafe impl Sync for Interieur {}

impl Interieur {
    fn pleines(&self) -> usize {
        // Toujours ≥ 0 : `lu` ne dépasse jamais `ecrit`.
        self.ecrit
            .load(Ordering::Acquire)
            .wrapping_sub(self.lu.load(Ordering::Acquire))
    }
}

/// Le bout où le décodeur pousse. Un seul exemplaire, sur un seul fil.
pub struct Producteur {
    interieur: Arc<Interieur>,
    /// Copie locale de `ecrit` : le producteur en est le seul auteur, donc il n'a pas besoin
    /// de le relire atomiquement pour savoir où il en est.
    position: usize,
}

/// Le bout où le callback tire. Un seul exemplaire, sur le fil temps réel.
pub struct Consommateur {
    interieur: Arc<Interieur>,
    position: usize,
}

/// Un lecteur des compteurs, et de rien d'autre. Clonable et partageable : c'est par là que
/// le fil qui publie la position de lecture observe l'avancement sans gêner les deux autres.
#[derive(Clone)]
pub struct Jauge(Arc<Interieur>);

/// Crée un tampon de `capacite` échantillons et rend ses deux bouts.
pub fn nouveau(capacite: usize) -> (Producteur, Consommateur) {
    assert!(capacite > 0, "un tampon de capacité nulle ne transporte rien");
    let interieur = Arc::new(Interieur {
        cases: UnsafeCell::new(vec![0i32; capacite].into_boxed_slice()),
        capacite,
        ecrit: AtomicUsize::new(0),
        lu: AtomicUsize::new(0),
    });
    (
        Producteur {
            interieur: Arc::clone(&interieur),
            position: 0,
        },
        Consommateur {
            interieur,
            position: 0,
        },
    )
}

impl Producteur {
    /// Pousse ce qui rentre et rend combien d'échantillons ont été pris. **Ne bloque
    /// jamais** : c'est à l'appelant de rappeler plus tard avec le reste.
    pub fn pousser(&mut self, echantillons: &[i32]) -> usize {
        let libre = self.libre();
        let combien = libre.min(echantillons.len());
        if combien == 0 {
            return 0;
        }
        // SÛRETÉ : on n'écrit que dans des cases libres — `libre` les a comptées à partir de
        // `lu`, que le consommateur publie en `Release`. Le consommateur ne lit pas cette
        // zone tant que `ecrit` n'a pas été publié, ce qui n'arrive qu'après ces écritures.
        unsafe {
            let cases = &mut *self.interieur.cases.get();
            for (i, &e) in echantillons[..combien].iter().enumerate() {
                cases[(self.position + i) % self.interieur.capacite] = e;
            }
        }
        self.position += combien;
        self.interieur.ecrit.store(self.position, Ordering::Release);
        combien
    }

    /// Le nombre de cases libres en ce moment.
    pub fn libre(&self) -> usize {
        self.interieur.capacite - self.interieur.pleines()
    }

    pub fn capacite(&self) -> usize {
        self.interieur.capacite
    }

    /// Le total écrit depuis le début.
    pub fn total_ecrit(&self) -> usize {
        self.position
    }

    pub fn jauge(&self) -> Jauge {
        Jauge(Arc::clone(&self.interieur))
    }
}

impl Consommateur {
    /// Remplit `sortie` avec ce qui est disponible et rend combien d'échantillons ont été
    /// écrits dedans. Ne bloque jamais, n'alloue pas : appelable depuis le fil temps réel.
    pub fn tirer(&mut self, sortie: &mut [i32]) -> usize {
        let combien = self.interieur.pleines().min(sortie.len());
        if combien == 0 {
            return 0;
        }
        // SÛRETÉ : on ne lit que des cases pleines — `pleines()` les compte à partir de
        // `ecrit`, publié en `Release` après que le producteur les ait toutes écrites. Le
        // producteur ne les réécrira pas avant la publication de `lu`, faite juste après.
        unsafe {
            let cases = &*self.interieur.cases.get();
            for (i, s) in sortie[..combien].iter_mut().enumerate() {
                *s = cases[(self.position + i) % self.interieur.capacite];
            }
        }
        self.position += combien;
        self.interieur.lu.store(self.position, Ordering::Release);
        combien
    }

    /// Jette tout ce qui attend, et rend combien a été jeté. Sert au déplacement dans la
    /// piste : les échantillons déjà décodés sont ceux d'avant le saut.
    pub fn vider(&mut self) -> usize {
        let combien = self.interieur.pleines();
        self.position += combien;
        self.interieur.lu.store(self.position, Ordering::Release);
        combien
    }

    /// Le total lu depuis le début, c'est-à-dire la position de lecture en échantillons.
    pub fn total_lu(&self) -> usize {
        self.position
    }
}

impl Jauge {
    /// Le total sorti du tampon depuis le début : la position de lecture, en échantillons.
    pub fn total_lu(&self) -> usize {
        self.0.lu.load(Ordering::Acquire)
    }

    /// Le total entré depuis le début.
    pub fn total_ecrit(&self) -> usize {
        self.0.ecrit.load(Ordering::Acquire)
    }

    /// Ce qui attend d'être joué.
    pub fn en_attente(&self) -> usize {
        self.0.pleines()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_tampon_neuf_ne_rend_rien() {
        let (p, mut c) = nouveau(8);
        let mut sortie = [0i32; 4];
        assert_eq!(c.tirer(&mut sortie), 0);
        assert_eq!(p.libre(), 8);
    }

    /// LA propriété qui compte : ce qui sort est exactement ce qui est entré, dans l'ordre,
    /// sans qu'un seul bit change. Le tampon est sur le chemin bit-perfect.
    #[test]
    fn ce_qui_sort_est_exactement_ce_qui_est_entre() {
        let (mut p, mut c) = nouveau(16);
        let entree = [i32::MIN, -1, 0, 1, 1 << 16, i32::MAX];
        assert_eq!(p.pousser(&entree), entree.len());
        let mut sortie = [0i32; 6];
        assert_eq!(c.tirer(&mut sortie), 6);
        assert_eq!(sortie, entree);
    }

    #[test]
    fn un_tampon_plein_nen_prend_pas_un_de_plus() {
        let (mut p, mut c) = nouveau(4);
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
        let (mut p, mut c) = nouveau(5);
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
        let (mut p, mut c) = nouveau(8);
        p.pousser(&[1, 2, 3, 4, 5]);
        assert_eq!(c.vider(), 5);
        assert_eq!(c.total_lu(), 5, "la position avance : les 5 sont considérés sortis");
        let mut sortie = [0i32; 4];
        assert_eq!(c.tirer(&mut sortie), 0, "plus rien n'attend");
        assert_eq!(p.libre(), 8, "et les cases sont rendues au producteur");
    }

    #[test]
    fn la_jauge_voit_les_deux_bouts() {
        let (mut p, mut c) = nouveau(8);
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
        let (mut p, mut c) = nouveau(1024);

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
}
