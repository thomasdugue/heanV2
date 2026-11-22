//! Store de bibliothèque — SQLite (ADR 0002).
//!
//! Phase 0 : aucun schéma, aucune migration (c'est la phase 2, et la migration des données
//! est une zone jamais déléguée). Ce module existe pour que le choix `rusqlite` + `bundled`
//! soit VÉRIFIÉ et non supposé : le test ci-dessous ouvre une base et lit la version de
//! SQLite embarquée. V1 a montré ce que coûte une affirmation de documentation non vérifiée.

use rusqlite::Connection;

/// Ouvre une base en mémoire. Sert au socle et aux tests ; la base de la bibliothèque
/// arrivera en phase 2, dans `~/Library/Application Support/hean/` (ADR 0003).
pub fn ouvrir_en_memoire() -> rusqlite::Result<Connection> {
    Connection::open_in_memory()
}

#[cfg(test)]
mod tests {
    #[test]
    fn sqlite_est_bien_embarque_et_transactionnel() {
        let mut c = super::ouvrir_en_memoire().expect("ouverture");
        c.execute_batch("CREATE TABLE t (x INTEGER)").expect("create");

        // Une transaction annulée ne laisse rien : c'est la propriété qui fait disparaître
        // les invariants de cohérence de cache de V1 (INV-08, INV-09, INV-10).
        let tx = c.transaction().expect("tx");
        tx.execute("INSERT INTO t VALUES (1)", []).expect("insert");
        tx.rollback().expect("rollback");

        let n: i64 = c
            .query_row("SELECT count(*) FROM t", [], |r| r.get(0))
            .expect("count");
        assert_eq!(n, 0);
    }
}
