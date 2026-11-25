//! Ce que la base garde et qui n'est **pas** une piste : les dossiers de bibliothèque, les
//! sources réseau, et les marqueurs de reprise du dossier de V1.
//!
//! Extrait de `repository.rs` le 2026-11-25, quand celui-ci a atteint sa limite de 400
//! lignes en accueillant la part réseau de la bibliothèque (ADR 0016). Le découpage suit la
//! même frontière que `lignes.rs` : `repository.rs` décide **ce qui** entre et sort de la
//! bibliothèque — donc tout ce qui touche aux pistes et au filtre d'exclusion — ce module-ci
//! garde les réglages qui l'accompagnent.
//!
//! Comme `lignes.rs`, il ne nomme pas la table des chemins exclus, et il ne doit pas le
//! faire : INV-06 veut qu'un seul fichier la nomme, et c'est `repository.rs`.
//!
//! Les trois tables d'ici se ressemblent par une propriété utile : **aucune n'est une piste**,
//! donc aucune écriture de ce fichier ne passe par `lignes::inserer` et aucune n'a à
//! connaître le filtre d'exclusion. C'est ce qui fait que le détecteur d'INV-06 n'a pas eu à
//! être élargi pour ce découpage — vérifié, et pas supposé : les quatre règles Rust de
//! `tests/architecture/une-seule-source-de-pistes.test.ts` passent sans exception nouvelle.

use std::path::{Path, PathBuf};

use rusqlite::{params, OptionalExtension};

use super::lignes::{self, maintenant};
use super::repository::{Depot, Erreur};
use super::types::SourceReseau;

impl Depot {
    // --- racines de bibliothèque (B04) -------------------------------------------------

    pub fn ajouter_racine(&mut self, dossier: &Path) -> Result<(), Erreur> {
        self.conn.execute(
            "INSERT OR IGNORE INTO racines (path, ajoutee_le) VALUES (?1, ?2)",
            params![dossier.display().to_string(), maintenant()],
        )?;
        Ok(())
    }

    pub fn racines(&self) -> Result<Vec<PathBuf>, Erreur> {
        let mut stmt = self.conn.prepare("SELECT path FROM racines ORDER BY path")?;
        let out = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(out.into_iter().map(PathBuf::from).collect())
    }

    /// Vrai si la base ne contient **aucune** ligne, dans aucune table (ADR 0012).
    ///
    /// Sert au démarrage à distinguer « V2 a créé son fichier de base en démarrant » de
    /// « V2 a une bibliothèque », parce que le premier cas ne doit pas empêcher la migration
    /// du dossier de V1 et que le second l'arrête. Le verdict autorise une suppression :
    /// `lignes::est_vierge` le dérive donc du schéma au lieu de consulter une liste de
    /// tables écrite à la main, qui serait muette le jour où une table arrive. La table
    /// `sources_reseau`, arrivée le 2026-11-25, en est la première démonstration : elle
    /// compte dans le verdict sans qu'une ligne ait eu à être ajoutée ici.
    pub fn est_vierge(&self) -> Result<bool, Erreur> {
        Ok(lignes::est_vierge(&self.conn)?)
    }

    // --- marqueurs de reprise (ADR 0010) -----------------------------------------------

    /// Vrai si ce sujet a déjà été repris du dossier de V1.
    ///
    /// La reprise elle-même vit dans `reprise_v1.rs` et ne passe que par l'API publique de
    /// ce dépôt. Ce n'est pas un découpage esthétique : INV-06 veut qu'un seul fichier
    /// nomme la table des chemins exclus, donc tout ce qui touche à l'exclusion passe par
    /// `exclure()` — y compris la reprise des exclusions de V1.
    pub fn reprise_deja_faite(&self, sujet: &str) -> Result<bool, Erreur> {
        let deja: Option<i64> = self
            .conn
            .query_row("SELECT le FROM reprises WHERE sujet = ?1", params![sujet], |r| r.get(0))
            .optional()?;
        Ok(deja.is_some())
    }

    /// Note qu'un sujet a été repris. Une reprise rejouée ressusciterait des réglages que
    /// l'utilisateur a changés depuis dans V2 — par exemple une piste qu'il a cessé d'exclure.
    pub fn noter_la_reprise(&mut self, sujet: &str) -> Result<(), Erreur> {
        self.conn.execute(
            "INSERT OR IGNORE INTO reprises (sujet, le) VALUES (?1, ?2)",
            params![sujet, maintenant()],
        )?;
        Ok(())
    }

    // --- sources réseau (ADR 0016) -----------------------------------------------------

    /// Enregistre une source réseau, ou met à jour celle qui porte déjà cet identifiant.
    ///
    /// **L'identifiant vient de V1 et doit être conservé tel quel** : c'est lui qui est
    /// écrit dans les 3 137 URI de la bibliothèque de Thomas. En fabriquer un nouveau pour
    /// la même source rendrait toutes ses pistes orphelines, et un retour à V1 ne les
    /// retrouverait pas non plus.
    ///
    /// Le mot de passe n'est **pas** ici, et aucune colonne ne l'accueille : il vit dans le
    /// Keychain (ADR 0015). Une colonne « mot de passe » serait une invitation permanente.
    pub fn enregistrer_source_reseau(&mut self, s: &SourceReseau) -> Result<(), Erreur> {
        self.conn.execute(
            "INSERT INTO sources_reseau
               (id, nom, hote, partage, chemin_distant, utilisateur, domaine, invite, active, ajoutee_le)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,
                     COALESCE((SELECT ajoutee_le FROM sources_reseau WHERE id = ?1), ?10))
             ON CONFLICT(id) DO UPDATE SET
               nom=?2, hote=?3, partage=?4, chemin_distant=?5, utilisateur=?6, domaine=?7,
               invite=?8, active=?9",
            params![
                s.id,
                s.nom,
                s.hote,
                s.partage,
                s.chemin_distant,
                s.utilisateur,
                s.domaine,
                i64::from(s.invite),
                i64::from(s.active),
                maintenant()
            ],
        )?;
        Ok(())
    }

    /// Les sources réseau enregistrées, triées par nom pour que l'affichage soit stable.
    pub fn sources_reseau(&self) -> Result<Vec<SourceReseau>, Erreur> {
        let mut stmt = self.conn.prepare(
            "SELECT id, nom, hote, partage, chemin_distant, utilisateur, domaine, invite, active
             FROM sources_reseau ORDER BY nom, id",
        )?;
        let out = stmt
            .query_map([], |r| {
                Ok(SourceReseau {
                    id: r.get(0)?,
                    nom: r.get(1)?,
                    hote: r.get(2)?,
                    partage: r.get(3)?,
                    chemin_distant: r.get(4)?,
                    utilisateur: r.get(5)?,
                    domaine: r.get(6)?,
                    invite: r.get::<_, i64>(7)? != 0,
                    active: r.get::<_, i64>(8)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }

    /// Une source précise, ou `None` si elle n'existe pas.
    pub fn source_reseau(&self, id: &str) -> Result<Option<SourceReseau>, Erreur> {
        Ok(self.sources_reseau()?.into_iter().find(|s| s.id == id))
    }
}
