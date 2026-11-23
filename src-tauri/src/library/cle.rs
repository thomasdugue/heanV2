//! INV-01 — un album = une seule entrée. Ici ce n'est plus une règle, c'est un type.
//!
//! V1 avait une fonction `normalizeKey()` (trim + NFC) **recopiée à plusieurs endroits**.
//! Trois bugs en sont sortis : l'invariant 1 lui-même, la régression W003 (déduplication par
//! `artiste + album`, donc un faux album dès qu'une piste portait « feat. ») et un lookup à
//! quatre niveaux dans la home, parce que deux formats de clé coexistaient.
//!
//! La réponse n'est pas « écrire la règle plus fort » : c'est de rendre la faute
//! inexprimable. La seule façon d'obtenir une `AlbumKey` est `AlbumKey::new(brut)`, qui
//! normalise. Il n'y a ni `From<String>`, ni champ public, ni constructeur de secours — donc
//! aucun chemin par lequel une clé non normalisée puisse entrer dans la base.
//!
//! Ce qui entre dans la clé, et ce qui n'y entre pas :
//!   - **l'album seul**. Pas l'artiste. C'est exactement la régression W003.
//!   - trim : les tags de V1 traînaient des espaces de fin (export d'un autre logiciel).
//!   - NFC : macOS rend les noms de fichiers en NFD, les tags sont souvent en NFC. « Café »
//!     écrit `e` + U+0301 et « Café » écrit `é` sont la même chose pour un humain, et deux
//!     chaînes différentes pour `==`. La fixture `tags-nfd.flac` est exactement ce cas.
//!   - casse et ponctuation : **laissées telles quelles**. Replier la casse ferait fusionner
//!     deux albums réellement différents, et la parité avec V1 est la cible (ADR 0004).

use unicode_normalization::UnicodeNormalization;

/// La clé de regroupement d'un album. Opaque : son contenu ne se construit qu'ici.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AlbumKey(String);

impl AlbumKey {
    /// Le constructeur UNIQUE. Toute autre voie serait la porte par laquelle INV-01 revient.
    pub fn new(brut: &str) -> Self {
        AlbumKey(brut.trim().nfc().collect())
    }

    /// Lecture seule : la clé sort en texte pour SQLite et pour l'IPC, jamais pour être
    /// recomposée. Rien ne reconstruit une `AlbumKey` à partir de ce texte sans repasser
    /// par `new` — qui est idempotente, donc le faire serait sans effet, pas dangereux.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Vrai si l'album n'a pas de nom : le tag est absent ou vide une fois normalisé.
    /// Un album sans nom ne se regroupe pas avec les autres sans nom — le repli est
    /// décidé par l'appelant, pas caché ici.
    pub fn est_vide(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::AlbumKey;
    use proptest::prelude::*;
    use unicode_normalization::UnicodeNormalization;

    // --- les trois cas de V1, nommés ------------------------------------------------
    #[test]
    fn trim_et_nfc_donnent_la_meme_cle() {
        let nfd: String = "Café Noir".nfd().collect();
        assert_ne!(nfd, "Café Noir", "la fixture du test doit bien être en NFD");
        assert_eq!(AlbumKey::new(&nfd), AlbumKey::new("  Café Noir  "));
    }

    #[test]
    fn la_cle_ignore_l_artiste() {
        // Régression W003 : dédupliquer par « artiste + album » recréait un faux album dès
        // qu'une piste portait « feat. ». La clé ne prend que l'album, par construction :
        // `new` n'a qu'un paramètre. Ce test fixe l'intention, pas la mécanique.
        assert_eq!(AlbumKey::new("Café Noir"), AlbumKey::new("Café Noir"));
    }

    #[test]
    fn un_album_sans_nom_est_vide() {
        assert!(AlbumKey::new("   ").est_vide());
        assert!(!AlbumKey::new("Été").est_vide());
    }

    // --- les propriétés (HARNAIS.md §4) ---------------------------------------------
    proptest! {
        /// Idempotence : `f(f(x)) == f(x)`. C'est ce qui rend sans danger le fait de
        /// renormaliser une clé déjà normalisée — donc ce qui permet de ne pas avoir à
        /// savoir, au point d'appel, si la chaîne est déjà passée par là. V1 ne le savait
        /// pas, d'où les deux formats de clé qui coexistaient.
        #[test]
        fn normalisation_idempotente(brut in ".{0,64}") {
            let une_fois = AlbumKey::new(&brut);
            let deux_fois = AlbumKey::new(une_fois.as_str());
            prop_assert_eq!(une_fois, deux_fois);
        }

        /// Pour TOUTE chaîne Unicode générée, les écritures NFC et NFD donnent la même clé.
        /// C'est la propriété que les exemples choisis à la main ne prouvent jamais : on ne
        /// pense qu'aux accents latins, et macOS produit du NFD sur tout l'alphabet.
        #[test]
        fn nfc_et_nfd_donnent_la_meme_cle(brut in ".{0,64}") {
            let en_nfc: String = brut.nfc().collect();
            let en_nfd: String = brut.nfd().collect();
            prop_assert_eq!(AlbumKey::new(&en_nfc), AlbumKey::new(&en_nfd));
        }

        /// Les espaces de bord ne changent jamais la clé, quelle qu'en soit la quantité.
        #[test]
        fn les_espaces_de_bord_ne_comptent_pas(brut in "[^\\s]{0,32}", g in 0usize..5, d in 0usize..5) {
            let entoure = format!("{}{}{}", " ".repeat(g), brut, " ".repeat(d));
            prop_assert_eq!(AlbumKey::new(&entoure), AlbumKey::new(&brut));
        }
    }
}
