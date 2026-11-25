//! La bibliothèque : scanner le disque, ranger dans SQLite, éditer des métadonnées.
//!
//! Ce module fait disparaître par construction quatre invariants classés BLOQUANTS en V1
//! (INV-08, 09, 10 et 14) : ils décrivaient tous le même défaut — plusieurs copies de la
//! même donnée, plusieurs écrivains, pas de transaction. Ici il y a une base, un écrivain,
//! des transactions (ADR 0002).
//!
//! Et il transforme deux règles en mécanismes :
//!   - **INV-01** devient un type. La seule façon d'obtenir une clé d'album est
//!     `cle::AlbumKey::new`, qui normalise (trim + NFC). Voir `cle.rs`.
//!   - **INV-07** devient un ordre d'écriture : le fichier audio d'abord, la base ensuite,
//!     et avec ce que la relecture du fichier a rendu. Voir `metadonnees.rs`.
//!
//! La répartition des responsabilités est ce qui tient INV-06 : `scan` trouve des fichiers
//! sans jamais décider lesquels comptent, `repository` est le seul à construire une liste de
//! pistes et le seul à connaître les chemins exclus.

pub mod cle;
pub mod dossier;
pub mod lignes;
pub mod metadonnees;
pub mod migrations;
pub mod reglages;
pub mod repository;
pub mod reprise_v1;
pub mod scan;
pub mod types;

pub use cle::AlbumKey;
pub use repository::Depot;
pub use reprise_v1::Reprise;
pub use types::{Edition, RapportDeScan, SourceReseau, Track};
