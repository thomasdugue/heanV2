//! Les types de la bibliothèque qui traversent l'IPC.
//!
//! Ils portent `#[derive(specta::Type)]`, donc `src/ipc/bindings.ts` les reflète :
//! renommer un champ ici casse la compilation du frontend. V1 appelait ses 86 commandes par
//! des chaînes de caractères, et neuf bugs d'une seule session (2026-03-09) étaient des noms
//! de champs faux — `data.mp3_count` au lieu de `data.stats.mp3_count`.
//!
//! **Aucun `u64` ni `i64` ici, et ce n'est pas un détail** : `specta_typescript` refuse de
//! les exporter par défaut (ils ne tiennent pas dans un `number` JavaScript). La contrainte
//! est bonne à prendre — elle force à choisir l'unité au lieu de laisser l'imprécision
//! arriver en silence côté JS. La taille et la date de modification des fichiers, elles,
//! restent en `u64` **dans la base** : elles servent au scan différentiel, pas à l'affichage.

use serde::{Deserialize, Serialize};

// Chaque type ne dérive que le sens de sérialisation qu'il emprunte vraiment : `Track` ne
// remonte que vers l'interface, `Edition` n'en descend que. Ce n'est pas de la cosmétique —
// un type qu'on peut désérialiser est un type que l'interface peut fabriquer.
//
// À savoir pour qui lit les types générés : **dès qu'un type porte un champ `Option`,
// specta en émet deux versions**, `Track_Serialize` et `Track_Deserialize` (un champ
// optionnel ne s'écrit pas pareil à l'aller — `x: T | null` — et au retour — `x?: T | null`),
// plus un alias `Track` qui est l'union des deux. Les dérives n'y changent rien. Ça a une
// conséquence sur le harnais : le détecteur d'INV-06 cherchait `\bTrack\b` dans le type de
// retour des commandes, donc il n'aurait **rien vu** sur `Track_Serialize[]` — règle
// désarmée en silence, trouvée en générant les types pour de vrai. Le détecteur reconnaît
// désormais les deux formes, et son auto-test les porte toutes les deux.

/// Une piste, telle que l'interface la voit. L'identité d'une piste est son **chemin** —
/// la question « un morceau logique avec N fichiers ? » est ouverte et tranchée en phase 3
/// (voir `docs/adr/README.md`), 21 % de la bibliothèque de Thomas étant en double.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
pub struct Track {
    pub path: String,
    pub titre: String,
    pub artiste: String,
    pub album: String,
    pub album_artiste: Option<String>,
    /// La clé de regroupement, produite par `AlbumKey::new` (INV-01). Le frontend la
    /// **reçoit** et ne la recalcule jamais : c'est ce qui garantit qu'il n'existe qu'une
    /// normalisation dans tout le projet.
    pub cle_album: String,
    pub annee: Option<u32>,
    pub piste: Option<u32>,
    pub disque: Option<u32>,
    pub genre: Option<String>,
    pub duree_ms: u32,
    pub format: String,
    pub echantillonnage: Option<u32>,
    pub bits: Option<u32>,
    pub debit: Option<u32>,
}

/// Une édition de métadonnées. `None` = **ne pas toucher à ce champ**, ce qui est ce dont
/// le bulk edit a besoin : la modale de V1 affichait « mixed values » pour les champs qui
/// diffèrent entre les pistes sélectionnées, et ne devait écrire que ceux qu'on a remplis.
/// Pour vider un tag, passer `Some("")`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, specta::Type)]
pub struct Edition {
    pub titre: Option<String>,
    pub artiste: Option<String>,
    pub album: Option<String>,
    pub annee: Option<u32>,
    pub piste: Option<u32>,
    pub genre: Option<String>,
}

impl Edition {
    /// Vrai si l'édition ne demande rien. Une édition vide ne doit pas réécrire le fichier
    /// audio : réécrire un FLAC pour rien, c'est déplacer des mégaoctets sur le disque de
    /// l'utilisateur — et sur un NAS, c'est pire.
    pub fn est_vide(&self) -> bool {
        *self == Edition::default()
    }
}

/// Ce qu'un scan a trouvé. V004 en dépend : **si tout est à zéro, l'interface ne recharge
/// rien.** En V1, `scan_complete` rechargeait toujours la bibliothèque, ce qui créait une
/// course avec le premier rendu de la home.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, specta::Type)]
pub struct RapportDeScan {
    pub ajoutees: u32,
    pub modifiees: u32,
    pub disparues: u32,
    /// Les fichiers vus mais illisibles (tags corrompus, permissions). Comptés et non
    /// fatals : un fichier abîmé au milieu d'une bibliothèque de 10 000 pistes ne doit pas
    /// interrompre le scan. Le détail part dans le journal, via `tracing` (INV-21).
    pub illisibles: u32,
}

impl RapportDeScan {
    /// Vrai si le scan n'a **rien** changé. C'est la condition de V004.
    pub fn sans_changement(&self) -> bool {
        self.ajoutees == 0 && self.modifiees == 0 && self.disparues == 0
    }
}

#[cfg(test)]
mod tests {
    use super::{Edition, RapportDeScan};

    #[test]
    fn un_rapport_vide_ne_demande_aucun_rechargement() {
        assert!(RapportDeScan::default().sans_changement());
        // Un fichier illisible n'est pas un changement : rien de nouveau n'est entré dans
        // la bibliothèque, donc l'interface n'a rien à recharger (V004).
        assert!(RapportDeScan { illisibles: 3, ..Default::default() }.sans_changement());
        assert!(!RapportDeScan { ajoutees: 1, ..Default::default() }.sans_changement());
        assert!(!RapportDeScan { disparues: 1, ..Default::default() }.sans_changement());
    }

    #[test]
    fn une_edition_sans_champ_ne_demande_rien() {
        assert!(Edition::default().est_vide());
        assert!(!Edition { titre: Some(String::new()), ..Default::default() }.est_vide());
    }
}
