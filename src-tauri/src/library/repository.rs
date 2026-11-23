//! **LE** dépôt de la bibliothèque. Un seul écrivain, un seul constructeur de liste de
//! pistes, un seul endroit où vit le filtre d'exclusion.
//!
//! INV-06 — en V1, « une piste supprimée ne revient jamais » était tenu par défense en
//! profondeur : le filtre `excluded_paths` était recopié dans `init_cache()`,
//! `start_background_scan()` et `scan_network_source_cmd()`, et l'invariant disait « et dans
//! tout nouveau point d'entrée ». Une règle formulée comme « n'oublie pas de répéter ça
//! partout » est violée au quatrième point d'entrée. Ici il n'y en a **qu'un**, et
//! `tests/architecture/une-seule-source-de-pistes.test.ts` refuse que le mot
//! `excluded_paths` apparaisse dans un autre fichier Rust que celui-ci.
//!
//! INV-08, 09, 10 et 14 sont rayées par ce module : il n'y a plus sept caches globaux
//! modifiés concurremment, mais une base, un écrivain, et des transactions (ADR 0002).
//! Si du code revenait poser ici un cache global mutable, il rouvrirait ces quatre portes.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

use super::cle::AlbumKey;
use super::migrations::{self, Migration};
use super::metadonnees;
use super::scan::{self, Trouvee};
use super::types::{Edition, RapportDeScan, Track};

/// Le schéma, versionné dès la première migration (ADR 0002, conséquence acceptée).
///
/// Il vit ici et non dans `migrations.rs` parce que c'est ici que `excluded_paths` a le
/// droit d'exister : séparer le mécanisme (appliquer une version) de son contenu (le
/// schéma) garde la règle d'INV-06 mécanique au lieu de déclarative.
///
/// `STRICT` : SQLite accepte par défaut un texte dans une colonne entière. Déclarer les
/// tables strictes transforme une erreur de type en échec immédiat, au lieu d'une donnée
/// absurde découverte trois écrans plus loin.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    nom: "bibliothèque initiale",
    sql: "
        CREATE TABLE tracks (
          path            TEXT    PRIMARY KEY NOT NULL,
          titre           TEXT    NOT NULL,
          artiste         TEXT    NOT NULL,
          album           TEXT    NOT NULL,
          album_artiste   TEXT,
          cle_album       TEXT    NOT NULL,
          annee           INTEGER,
          piste           INTEGER,
          disque          INTEGER,
          genre           TEXT,
          duree_ms        INTEGER NOT NULL,
          format          TEXT    NOT NULL,
          echantillonnage INTEGER,
          bits            INTEGER,
          debit           INTEGER,
          taille          INTEGER NOT NULL,
          mtime_ns        INTEGER NOT NULL,
          ajoutee_le      INTEGER NOT NULL
        ) STRICT;
        CREATE INDEX tracks_par_album ON tracks (cle_album);

        CREATE TABLE excluded_paths (
          path      TEXT    PRIMARY KEY NOT NULL,
          exclue_le INTEGER NOT NULL
        ) STRICT;

        CREATE TABLE racines (
          path       TEXT    PRIMARY KEY NOT NULL,
          ajoutee_le INTEGER NOT NULL
        ) STRICT;
    ",
}];

/// Les colonnes d'une piste telle que l'IPC la rend. Écrites une fois, lues une fois.
const COLONNES: &str = "path, titre, artiste, album, album_artiste, cle_album, annee, piste, \
                        disque, genre, duree_ms, format, echantillonnage, bits, debit";

/// **Le filtre d'INV-06.** Il n'existe qu'ici, et toute requête qui rend des pistes le
/// porte. Une piste exclue est aussi effacée de `tracks` au scan suivant — la double
/// précaution est voulue : l'effacement garde la base petite, le filtre garantit qu'une
/// piste exclue reste invisible même si un chemin d'écriture oubliait de l'effacer.
const NON_EXCLUE: &str = "path NOT IN (SELECT path FROM excluded_paths)";

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Migration(#[from] migrations::Erreur),
    #[error(transparent)]
    Metadonnees(#[from] metadonnees::Erreur),
    #[error("piste inconnue dans la bibliothèque : {0}")]
    PisteInconnue(String),
}

pub struct Depot {
    conn: Connection,
}

impl Depot {
    /// Ouvre la base du dossier de données et applique les migrations.
    pub fn ouvrir(fichier: &Path) -> Result<Self, Erreur> {
        if let Some(parent) = fichier.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        Self::depuis(Connection::open(fichier)?)
    }

    /// Une base en mémoire. Sert aux tests, et à rien d'autre : la bibliothèque de
    /// l'utilisateur vit dans un fichier (ADR 0003).
    pub fn en_memoire() -> Result<Self, Erreur> {
        Self::depuis(Connection::open_in_memory()?)
    }

    fn depuis(mut conn: Connection) -> Result<Self, Erreur> {
        // Les clés étrangères et le WAL se demandent à chaque ouverture, pas une fois pour
        // toutes : ce sont des réglages de connexion, pas de fichier.
        conn.execute_batch("PRAGMA foreign_keys = ON")?;
        migrations::appliquer(&mut conn, MIGRATIONS)?;
        Ok(Depot { conn })
    }

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

    // --- lecture : LE constructeur de liste de pistes (INV-06) -------------------------

    /// La seule fonction du projet qui construise une liste de pistes.
    ///
    /// Le tri est fait par SQLite et il est **total** : album, puis disque, puis numéro de
    /// piste, puis chemin. Le chemin en dernier rend l'ordre déterministe même quand deux
    /// pistes partagent tout le reste — sans quoi l'ordre dépendrait de l'ordre d'insertion,
    /// c'est-à-dire de l'ordre du scan, c'est-à-dire de rien.
    pub fn lister_pistes(&self) -> Result<Vec<Track>, Erreur> {
        let sql = format!(
            "SELECT {COLONNES} FROM tracks WHERE {NON_EXCLUE} \
             ORDER BY cle_album, disque, piste, path"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let out = stmt.query_map([], ligne_en_track)?.collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }

    /// Une piste précise, ou `None` si elle est absente ou exclue. Passe par le même filtre.
    pub fn piste(&self, chemin: &str) -> Result<Option<Track>, Erreur> {
        let sql = format!("SELECT {COLONNES} FROM tracks WHERE path = ?1 AND {NON_EXCLUE}");
        Ok(self
            .conn
            .query_row(&sql, params![chemin], ligne_en_track)
            .optional()?)
    }

    // --- suppression permanente (INV-06, B09) -----------------------------------------

    /// Exclut une piste **définitivement**. Le fichier reste sur le disque ; c'est la
    /// bibliothèque qui ne le reprendra plus, y compris après un scan.
    pub fn exclure(&mut self, chemin: &str) -> Result<(), Erreur> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO excluded_paths (path, exclue_le) VALUES (?1, ?2)",
            params![chemin, maintenant()],
        )?;
        tx.execute("DELETE FROM tracks WHERE path = ?1", params![chemin])?;
        tx.commit()?;
        Ok(())
    }

    pub fn chemins_exclus(&self) -> Result<Vec<String>, Erreur> {
        let mut stmt = self.conn.prepare("SELECT path FROM excluded_paths ORDER BY path")?;
        let out = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }

    // --- scan (B02, B03) ---------------------------------------------------------------

    /// Scanne les racines enregistrées et met la base à jour, **en une transaction**.
    ///
    /// Différentiel : un fichier dont la taille et la date n'ont pas bougé n'est pas relu
    /// (B03). C'est ce qui rend V004 possible — un deuxième scan ne trouve rien, donc
    /// l'interface n'a rien à recharger.
    pub fn scanner(&mut self) -> Result<RapportDeScan, Erreur> {
        let racines = self.racines()?;
        let trouvees = scan::parcourir(&racines);
        self.appliquer_scan(&trouvees)
    }

    fn appliquer_scan(&mut self, trouvees: &[Trouvee]) -> Result<RapportDeScan, Erreur> {
        let mut rapport = RapportDeScan::default();
        let exclus = self.chemins_exclus()?;
        let connues = self.empreintes()?;
        let mut vus: Vec<String> = Vec::with_capacity(trouvees.len());

        let tx = self.conn.transaction()?;
        for t in trouvees {
            let chemin = t.chemin.display().to_string();
            // LE filtre, appliqué à l'entrée comme à la sortie (INV-06).
            if exclus.contains(&chemin) {
                continue;
            }
            vus.push(chemin.clone());
            match connues.iter().find(|(p, _, _)| *p == chemin) {
                Some((_, taille, mtime)) if *taille == en_i64(t.taille) && *mtime == t.mtime_ns => continue,
                existe => {
                    let lues = match metadonnees::lire(&t.chemin) {
                        Ok(l) => l,
                        Err(e) => {
                            // Un fichier abîmé n'interrompt pas le scan. Il est compté, et
                            // journalisé via tracing — jamais via println! (INV-21).
                            tracing::warn!(chemin = %chemin, erreur = %e, "tags illisibles, piste ignorée");
                            rapport.illisibles += 1;
                            continue;
                        }
                    };
                    inserer(&tx, &lues.track, &lues.cle, t)?;
                    if existe.is_some() {
                        rapport.modifiees += 1;
                    } else {
                        rapport.ajoutees += 1;
                    }
                }
            }
        }

        // Les pistes dont le fichier a disparu. Ce n'est PAS une exclusion : si le fichier
        // revient (NAS rebranché), la piste revient avec lui.
        for (chemin, _, _) in &connues {
            if !vus.contains(chemin) {
                tx.execute("DELETE FROM tracks WHERE path = ?1", params![chemin])?;
                rapport.disparues += 1;
            }
        }
        tx.commit()?;
        tracing::info!(
            ajoutees = rapport.ajoutees,
            modifiees = rapport.modifiees,
            disparues = rapport.disparues,
            illisibles = rapport.illisibles,
            "scan terminé"
        );
        Ok(rapport)
    }

    /// Taille et date de modification des pistes connues. En `i64` et non en `u64` :
    /// SQLite n'a pas d'entier non signé, et rusqlite refuse `u64` — forcer la conversion
    /// ici évite qu'elle se fasse à l'aveugle dans une requête.
    fn empreintes(&self) -> Result<Vec<(String, i64, i64)>, Erreur> {
        let mut stmt = self.conn.prepare("SELECT path, taille, mtime_ns FROM tracks")?;
        let out = stmt
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }

    // --- édition (B06, B07, B08 — INV-07) ----------------------------------------------

    /// Écrit une édition dans le **fichier audio**, puis met la base à jour avec ce que la
    /// relecture du fichier a rendu (INV-07). L'ordre est l'invariant.
    pub fn editer(&mut self, chemin: &str, edition: &Edition) -> Result<Track, Erreur> {
        // Une édition vide est un non-événement, pas une erreur : on rend la piste telle
        // qu'elle est. Sans ce court-circuit, `editer_en_lot` rendait un vecteur vide et le
        // `pop()` levait « piste inconnue » — un message qui accuse la piste alors que rien
        // n'avait été demandé. Relevé en relecture ; un message d'erreur faux coûte plus
        // cher qu'une absence de message, parce qu'il envoie chercher au mauvais endroit.
        if edition.est_vide() {
            return self
                .piste(chemin)?
                .ok_or_else(|| Erreur::PisteInconnue(chemin.to_string()));
        }
        let mut pistes = self.editer_en_lot(std::slice::from_ref(&chemin.to_string()), edition)?;
        pistes.pop().ok_or_else(|| Erreur::PisteInconnue(chemin.to_string()))
    }

    /// Le bulk edit de V1 (B08) : N pistes, les champs laissés à `None` ne bougent pas.
    ///
    /// Les fichiers sont écrits un par un — c'est le disque, on ne peut pas faire autrement
    /// — mais la base est mise à jour **en une seule transaction**. Si une écriture de
    /// fichier échoue au milieu, les précédentes sont déjà sur le disque : la transaction
    /// est alors annulée et le scan suivant rattrapera ces fichiers, puisque leur date de
    /// modification a changé. Le contraire — base à jour, fichier non écrit — serait la
    /// perte silencieuse d'INV-07.
    pub fn editer_en_lot(&mut self, chemins: &[String], edition: &Edition) -> Result<Vec<Track>, Erreur> {
        if edition.est_vide() {
            return Ok(Vec::new());
        }
        let mut ecrites = Vec::with_capacity(chemins.len());
        for chemin in chemins {
            // On n'édite que ce que la bibliothèque connaît. Éditer un chemin arbitraire
            // venu de l'interface voudrait dire écrire dans un fichier que l'utilisateur
            // n'a jamais ajouté — et le filtre d'exclusion s'applique aussi ici (INV-06),
            // puisque `piste()` est la lecture unitaire du dépôt.
            if self.piste(chemin)?.is_none() {
                return Err(Erreur::PisteInconnue(chemin.clone()));
            }
            ecrites.push(metadonnees::ecrire(Path::new(chemin), edition)?);
        }

        let tx = self.conn.transaction()?;
        let mut out = Vec::with_capacity(ecrites.len());
        for lues in &ecrites {
            // Le fichier vient d'être réécrit : sa taille et sa date ont changé, et c'est
            // celles du disque qui entrent dans la base. Sinon le scan suivant croirait la
            // piste modifiée et la relirait pour rien — et V004 tomberait.
            let meta = std::fs::metadata(&lues.track.path).ok();
            let trouvee = Trouvee {
                chemin: PathBuf::from(&lues.track.path),
                taille: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                mtime_ns: meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .and_then(|d| i64::try_from(d.as_nanos()).ok())
                    .unwrap_or(0),
            };
            inserer(&tx, &lues.track, &lues.cle, &trouvee)?;
            out.push(lues.track.clone());
        }
        tx.commit()?;
        Ok(out)
    }
}

/// L'écriture d'une piste. `INSERT OR REPLACE` et **jamais** un `DELETE` suivi d'un
/// `INSERT` : c'est la leçon d'INV-08, où supprimer au lieu de mettre à jour ouvrait une
/// fenêtre pendant laquelle la piste n'existait nulle part. Ici la fenêtre n'existe pas,
/// mais le réflexe reste le bon.
fn inserer(tx: &rusqlite::Transaction<'_>, t: &Track, cle: &AlbumKey, f: &Trouvee) -> Result<(), rusqlite::Error> {
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

fn ligne_en_track(r: &rusqlite::Row<'_>) -> Result<Track, rusqlite::Error> {
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
fn en_i64(taille: u64) -> i64 {
    i64::try_from(taille).unwrap_or(i64::MAX)
}

/// L'heure, en secondes depuis l'époque. Jamais `unwrap()` : une horloge réglée avant 1970
/// ne doit pas faire paniquer l'app de quelqu'un.
fn maintenant() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(0))
        .unwrap_or(0)
}
