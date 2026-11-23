//! Les écritures et les lectures d'une ligne de piste, et les conversions que SQLite impose.
//!
//! Extrait de `repository.rs` quand celui-ci a dépassé 400 lignes — la limite dure du projet
//! (`./check` la refuse ; en V1, `lib.rs` faisait 4 870 lignes). Le découpage suit une
//! frontière réelle : `repository.rs` décide **ce qui** entre et sort de la bibliothèque,
//! ce module-ci sait **comment** une piste s'écrit et se relit.
//!
//! Il ne nomme pas la table des chemins exclus, et il ne doit pas le faire : INV-06 veut
//! qu'un seul fichier la nomme, et c'est `repository.rs`.

use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::params;

use super::cle::AlbumKey;
use super::scan::Trouvee;
use super::types::Track;

/// Vrai si **aucune** table de la base ne contient de ligne.
///
/// Les tables sont lues dans `sqlite_master`, **jamais énumérées ici**, et c'est tout
/// l'objet de cette fonction. Relevé en relecture du 2026-11-23 : le verdict « cette base
/// est vierge » autorise sa **suppression** (`demarrage::ecarter_une_bibliotheque_vierge`),
/// et il était rendu en consultant trois listes nommées à la main — les pistes, les racines,
/// les exclusions. Vrai à la date d'écriture, et **faux en silence** le jour où la phase 3
/// ajoute `playlists`, `favoris` ou `historique` : une base contenant trois playlists et
/// aucune piste aurait été déclarée vide, donc effacée, sans qu'aucun test ne rougisse.
///
/// C'est la forme exacte du défaut qu'INV-06 a coûté à V1 — « n'oublie pas de répéter ça
/// partout » — appliquée cette fois à une destruction. En dérivant du schéma, la question
/// n'a plus de liste à tenir à jour : une table qui arrive est vue le jour où elle arrive.
///
/// Le nom de table interpolé vient de `sqlite_master`, donc du schéma que nous avons écrit,
/// jamais d'une entrée de l'utilisateur. Même justification que le `PRAGMA user_version`
/// de `migrations.rs`, qui ne peut pas être un paramètre lié non plus.
pub(super) fn est_vierge(conn: &rusqlite::Connection) -> Result<bool, rusqlite::Error> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for table in tables {
        let lignes: i64 =
            conn.query_row(&format!("SELECT count(*) FROM \"{table}\""), [], |r| r.get(0))?;
        if lignes > 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

/// L'écriture d'une piste. `INSERT OR REPLACE` et **jamais** un `DELETE` suivi d'un
/// `INSERT` : c'est la leçon d'INV-08, où supprimer au lieu de mettre à jour ouvrait une
/// fenêtre pendant laquelle la piste n'existait nulle part. Ici la fenêtre n'existe pas,
/// mais le réflexe reste le bon.
pub(super) fn inserer(tx: &rusqlite::Transaction<'_>, t: &Track, cle: &AlbumKey, f: &Trouvee) -> Result<(), rusqlite::Error> {
    tx.execute(
        "INSERT INTO tracks (path, titre, artiste, album, album_artiste, cle_album, annee, piste,
                             disque, genre, duree_ms, format, echantillonnage, bits, debit,
                             taille, mtime_ns, ajoutee_le)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,
                 COALESCE((SELECT ajoutee_le FROM tracks WHERE path = ?1), ?18))
         ON CONFLICT(path) DO UPDATE SET
           titre=?2, artiste=?3, album=?4, album_artiste=?5, cle_album=?6, annee=?7, piste=?8,
           disque=?9, genre=?10, duree_ms=?11, format=?12, echantillonnage=?13, bits=?14,
           debit=?15, taille=?16, mtime_ns=?17",
        params![
            t.path, t.titre, t.artiste, t.album, t.album_artiste, cle.as_str(), t.annee,
            t.piste, t.disque, t.genre, t.duree_ms, t.format, t.echantillonnage, t.bits,
            t.debit, en_i64(f.taille), f.mtime_ns, maintenant()
        ],
    )?;
    Ok(())
}

pub(super) fn ligne_en_track(r: &rusqlite::Row<'_>) -> Result<Track, rusqlite::Error> {
    Ok(Track {
        path: r.get(0)?,
        titre: r.get(1)?,
        artiste: r.get(2)?,
        album: r.get(3)?,
        album_artiste: r.get(4)?,
        cle_album: r.get(5)?,
        annee: r.get(6)?,
        piste: r.get(7)?,
        disque: r.get(8)?,
        genre: r.get(9)?,
        duree_ms: r.get(10)?,
        format: r.get(11)?,
        echantillonnage: r.get(12)?,
        bits: r.get(13)?,
        debit: r.get(14)?,
    })
}

/// Une taille de fichier pour SQLite, qui n'a pas d'entier non signé. Un fichier de plus
/// de 8 exaoctets est hors de ce monde ; la saturation est préférable à une troncature
/// silencieuse, qui donnerait une taille plausible et fausse.
pub(super) fn en_i64(taille: u64) -> i64 {
    i64::try_from(taille).unwrap_or(i64::MAX)
}

/// L'heure, en secondes depuis l'époque. Jamais `unwrap()` : une horloge réglée avant 1970
/// ne doit pas faire paniquer l'app de quelqu'un.
pub(super) fn maintenant() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(0))
        .unwrap_or(0)
}
