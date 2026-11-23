//! Le **mécanisme** de migration de schéma. Pas le schéma : il vit dans `repository.rs`,
//! avec le seul module autorisé à interroger la base.
//!
//! Les migrations sont versionnées **dès la première**, et c'est une conséquence acceptée de
//! l'ADR 0002 écrite noir sur blanc. La raison est celle de l'ADR 0003 : il y a ~50
//! beta-testeurs, et leur bibliothèque est le genre de donnée qu'on ne reconstruit pas. Une
//! base sans numéro de version est une base qu'on ne saura pas faire évoluer sans deviner.
//!
//! Trois propriétés, chacune éprouvée par un test ci-dessous :
//!   1. **une migration s'applique une fois**. Rouvrir la base ne la rejoue pas.
//!   2. **une migration qui échoue ne laisse rien derrière elle.** Elle tourne dans une
//!      transaction, donc soit le schéma avance d'une version complète, soit il ne bouge
//!      pas. C'est exactement la propriété que les six fichiers JSON de V1 n'avaient pas.
//!   3. **une version inconnue est refusée.** Une base écrite par une version plus récente
//!      de Hean ne se fait pas « réparer » par une version plus ancienne : elle refuse de
//!      s'ouvrir. Perdre des données en silence est pire que ne pas démarrer.
//!
//! `PRAGMA user_version` sert de compteur : c'est un entier que SQLite range dans l'en-tête
//! du fichier, donc il n'y a pas de table d'état à garder cohérente avec le reste.

use rusqlite::Connection;

/// Une étape de schéma. `sql` doit être idempotent-par-construction, c'est-à-dire ne
/// s'exécuter qu'une fois : c'est le rôle du numéro de version.
pub struct Migration {
    pub version: i64,
    pub nom: &'static str,
    pub sql: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error("migration {version} ({nom}) : {source}")]
    Echec {
        version: i64,
        nom: &'static str,
        #[source]
        source: rusqlite::Error,
    },
    #[error(
        "la base est en version {trouvee}, cette version de Hean ne connaît que \
         jusqu'à {connue} — elle a été écrite par une version plus récente"
    )]
    VersionInconnue { trouvee: i64, connue: i64 },
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

/// Applique les migrations manquantes, dans l'ordre. Rend la version atteinte.
pub fn appliquer(conn: &mut Connection, migrations: &[Migration]) -> Result<i64, Erreur> {
    let actuelle: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let derniere = migrations.last().map(|m| m.version).unwrap_or(0);
    if actuelle > derniere {
        return Err(Erreur::VersionInconnue {
            trouvee: actuelle,
            connue: derniere,
        });
    }

    for m in migrations.iter().filter(|m| m.version > actuelle) {
        let tx = conn.transaction()?;
        tx.execute_batch(m.sql).map_err(|source| Erreur::Echec {
            version: m.version,
            nom: m.nom,
            source,
        })?;
        // `user_version` n'accepte pas de paramètre lié : c'est un pragma, pas une requête.
        // La valeur vient d'un littéral du code, jamais d'une entrée utilisateur.
        tx.execute_batch(&format!("PRAGMA user_version = {}", m.version))
            .map_err(|source| Erreur::Echec {
                version: m.version,
                nom: m.nom,
                source,
            })?;
        tx.commit()?;
        tracing::info!(version = m.version, nom = m.nom, "migration appliquée");
    }
    Ok(derniere)
}

#[cfg(test)]
mod tests {
    use super::{appliquer, Erreur, Migration};
    use rusqlite::Connection;

    const UNE: &[Migration] = &[Migration {
        version: 1,
        nom: "table t",
        sql: "CREATE TABLE t (x INTEGER)",
    }];

    #[test]
    fn une_migration_ne_sapplique_quune_fois() {
        let mut c = Connection::open_in_memory().expect("base");
        assert_eq!(appliquer(&mut c, UNE).expect("1er passage"), 1);
        c.execute("INSERT INTO t VALUES (7)", []).expect("insert");
        // Deuxième ouverture : si la migration se rejouait, le CREATE TABLE échouerait —
        // ou pire, un `IF NOT EXISTS` masquerait le problème et effacerait la ligne.
        assert_eq!(appliquer(&mut c, UNE).expect("2e passage"), 1);
        let n: i64 = c
            .query_row("SELECT count(*) FROM t", [], |r| r.get(0))
            .expect("count");
        assert_eq!(n, 1, "les données doivent survivre au second passage");
    }

    #[test]
    fn une_migration_qui_echoue_ne_laisse_rien_derriere_elle() {
        // Le détecteur mis à l'épreuve : une migration volontairement fautive. Sa deuxième
        // instruction est invalide, donc la première doit être annulée avec elle.
        let fautive: &[Migration] = &[Migration {
            version: 1,
            nom: "fautive",
            sql: "CREATE TABLE a (x INTEGER); CREATE TABLE a (x INTEGER)",
        }];
        let mut c = Connection::open_in_memory().expect("base");
        let e = appliquer(&mut c, fautive).expect_err("la migration doit échouer");
        assert!(matches!(e, Erreur::Echec { version: 1, .. }), "{e}");

        let version: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).expect("version");
        assert_eq!(version, 0, "le schéma ne doit pas avoir avancé");
        let tables: i64 = c
            .query_row("SELECT count(*) FROM sqlite_master WHERE name = 'a'", [], |r| r.get(0))
            .expect("count");
        assert_eq!(tables, 0, "la table créée avant l'échec doit avoir disparu");
    }

    #[test]
    fn une_base_plus_recente_est_refusee_au_lieu_detre_reparee() {
        let mut c = Connection::open_in_memory().expect("base");
        c.execute_batch("PRAGMA user_version = 99").expect("version future");
        let e = appliquer(&mut c, UNE).expect_err("doit refuser");
        assert!(
            matches!(e, Erreur::VersionInconnue { trouvee: 99, connue: 1 }),
            "{e}"
        );
    }

    #[test]
    fn les_migrations_sappliquent_dans_lordre_depuis_une_base_vide() {
        let deux: &[Migration] = &[
            Migration { version: 1, nom: "a", sql: "CREATE TABLE a (x INTEGER)" },
            Migration { version: 2, nom: "b", sql: "ALTER TABLE a ADD COLUMN y TEXT" },
        ];
        let mut c = Connection::open_in_memory().expect("base");
        assert_eq!(appliquer(&mut c, deux).expect("les deux"), 2);
        c.execute("INSERT INTO a (x, y) VALUES (1, 'ok')", []).expect("les deux colonnes");
    }
}
