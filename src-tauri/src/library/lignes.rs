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

/// Applique un inventaire déjà filtré par son appelant : écrit ce qui a changé, efface ce
/// qui a disparu, et rend le compte.
///
/// **Ce que ce module décide : rien.** Le périmètre (INV-27) et la liste des chemins exclus
/// (INV-06) sont calculés par `repository.rs` — le seul endroit qui a le droit de les
/// connaître — et arrivent ici en paramètres. Cette fonction ne sait que les appliquer. Le
/// découpage est celui du reste du fichier : `repository.rs` décide **ce qui** entre et sort,
/// ce module sait **comment** une ligne s'écrit et s'efface.
///
/// Elle est arrivée ici le 2026-11-25 pour la même raison que `inserer` : `repository.rs`
/// atteignait 400 lignes. La limite a fait son travail — elle a demandé un découpage, et le
/// découpage existait déjà.
pub(super) fn appliquer_inventaire(
    tx: &rusqlite::Transaction<'_>,
    entrees: &[super::scan::Entree],
    perimetre: &super::scan::Perimetre,
    exclus: &[String],
    connues: &[(String, i64, i64)],
    conclusion: super::scan::Conclusion,
) -> Result<super::types::RapportDeScan, super::repository::Erreur> {
    use super::scan::Entree;
    let mut rapport = super::types::RapportDeScan::default();
    let mut vus: Vec<String> = Vec::with_capacity(entrees.len());

    for entree in entrees {
        let t = entree.trouvee();
        let chemin = entree.chemin();
        // Une entrée hors périmètre est une erreur d'appel, pas une donnée : l'ignorer en
        // silence laisserait un inventaire mal construit écrire n'importe où, et le
        // périmètre ne protégerait plus que la suppression.
        if !perimetre.contient(&chemin) {
            tracing::warn!(chemin = %chemin, "entrée hors du périmètre du scan, ignorée");
            continue;
        }
        if exclus.contains(&chemin) {
            continue;
        }
        vus.push(chemin.clone());
        // Une piste distante vue et inchangée : rien à relire, rien à écrire, et surtout
        // **pas** à effacer — c'est tout l'objet du `None`.
        if matches!(entree, Entree::Distante(_, None)) {
            continue;
        }
        match connues.iter().find(|(p, _, _)| *p == chemin) {
            Some((_, taille, mtime)) if *taille == en_i64(t.taille) && *mtime == t.mtime_ns => continue,
            existe => {
                // Les tags d'un fichier local se lisent ici. Ceux d'une piste distante sont
                // déjà lus : le réseau n'a été touché qu'une fois, et hors de tout verrou.
                let lu;
                let lues: &super::metadonnees::Lues = match entree {
                    Entree::Distante(_, Some(deja)) => deja,
                    // Traité juste au-dessus ; cette branche est inatteignable et le dit.
                    Entree::Distante(_, None) => continue,
                    Entree::Disque(_) => match super::metadonnees::lire(&t.chemin) {
                        Ok(l) => {
                            lu = l;
                            &lu
                        }
                        Err(e) => {
                            // Un fichier abîmé n'interrompt pas le scan. Il est compté, et
                            // journalisé via tracing — jamais via println! (INV-21).
                            tracing::warn!(chemin = %chemin, erreur = %e, "tags illisibles, piste ignorée");
                            rapport.illisibles += 1;
                            continue;
                        }
                    },
                };
                inserer(tx, &lues.track, &lues.cle, t)?;
                if existe.is_some() {
                    rapport.modifiees += 1;
                } else {
                    rapport.ajoutees += 1;
                }
            }
        }
    }

    // Les pistes de CE périmètre dont le fichier a disparu. Ce n'est PAS une exclusion : si
    // le fichier revient (NAS rebranché), la piste revient avec lui.
    //
    // **Et rien ne disparaît quand l'inventaire est partiel** : un dossier qu'on n'a pas pu
    // lire n'est pas un dossier vide. Le compte des sautés est déjà journalisé par le
    // parcours ; ce qui se décide ici est de ne pas transformer un refus du serveur en
    // suppression de pistes (B17).
    if conclusion == super::scan::Conclusion::Partielle {
        tracing::warn!(
            vues = vus.len(),
            "inventaire partiel : aucune piste n'est retirée de la bibliothèque ce tour-ci"
        );
        return Ok(rapport);
    }
    for (chemin, _, _) in connues {
        if !vus.contains(chemin) {
            tx.execute("DELETE FROM tracks WHERE path = ?1", params![chemin])?;
            rapport.disparues += 1;
        }
    }
    Ok(rapport)
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
