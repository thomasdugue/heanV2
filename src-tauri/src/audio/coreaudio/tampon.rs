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
//! # Le tampon compte en TRAMES, pas en échantillons
//!
//! Il connaît le nombre de canaux, et ni `pousser` ni `tirer` ne laissent passer une demi-trame.
//! Ce n'est pas de la précaution décorative : **une seule publication tombant au milieu d'une
//! trame intervertirait L et R pour tout le reste du flux.** Le callback suivant attaquerait le
//! tampon de CoreAudio à l'index 0 avec un échantillon de canal droit, et rien ne le dirait —
//! le compteur de famines ne bougerait pas, puisque le tampon n'est pas à vide : il est décalé.
//! Inaudible en intégration continue, parfaitement audible chez Thomas.
//!
//! ⚠️ **Avant le 2026-11-26 la garde n'existait pas, et la justesse des canaux reposait
//! entièrement sur le fait qu'une constante soit un nombre entier** : la capacité valait
//! `ceil(frequence × canaux × SECONDES_DE_TAMPON)`, et avec `5.0` elle tombait toujours sur un
//! multiple du nombre de canaux. Avec `0.333`, `ceil(44100 × 2 × 0.333)` vaut 29 371 — impair,
//! donc une demi-trame devient publiable. Le mode de défaut était pourtant déjà **écrit** dans
//! `rendu.rs`, qui explique refuser de reprendre à moitié le chemin désentrelacé de V1 pour
//! exactement cette raison. Identifié un étage, laissé sans garde l'étage d'en dessous.
//!
//! D'où la forme actuelle : la capacité est donnée en **trames**, donc elle est un multiple du
//! nombre de canaux par construction, et les deux bouts arrondissent à la trame entière. La
//! faute cesse d'être improbable pour devenir inexprimable. Relevé en relecture.
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
    /// Le nombre de canaux. C'est lui qui fait du tampon un tampon de trames.
    canaux: usize,
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
    /// Arrondit un nombre d'échantillons à la trame entière inférieure.
    fn a_la_trame(&self, echantillons: usize) -> usize {
        echantillons - echantillons % self.canaux
    }

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

/// Crée un tampon de `trames` trames de `canaux` canaux, et rend ses deux bouts.
///
/// La capacité est exprimée en **trames** et non en échantillons, exprès : c'est ce qui
/// garantit qu'elle est un multiple du nombre de canaux, donc que l'enroulement ne peut pas
/// décaler l'alignement des canaux. Voir l'en-tête du module.
pub fn nouveau(trames: usize, canaux: usize) -> (Producteur, Consommateur) {
    assert!(trames > 0, "un tampon de capacité nulle ne transporte rien");
    assert!(canaux > 0, "un flux sans canal n'existe pas");
    let capacite = trames * canaux;
    let interieur = Arc::new(Interieur {
        cases: UnsafeCell::new(vec![0i32; capacite].into_boxed_slice()),
        capacite,
        canaux,
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
        // Arrondi à la trame entière : publier une demi-trame intervertirait les canaux pour
        // tout le reste du flux (en-tête du module).
        let combien = self.interieur.a_la_trame(self.libre().min(echantillons.len()));
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

    pub fn canaux(&self) -> usize {
        self.interieur.canaux
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
        // Même raison qu'à l'écriture : on ne consomme que des trames entières.
        let combien = self
            .interieur
            .a_la_trame(self.interieur.pleines().min(sortie.len()));
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
