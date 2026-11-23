//! Ce que la bibliothèque garantit, prouvé sur de vrais fichiers.
//!
//! Ces tests ne vérifient pas « le code fait ce qu'il dit » : ils rejouent les régressions
//! qui ont coûté le plus cher en V1, sur les fixtures faites pour elles.
//!
//! Une seule cible de test (`cargo test --test bibliotheque`), découpée en modules parce que
//! ce fichier a dépassé la limite de 400 lignes d'`./check`. Le bac à sable est partagé
//! dans `../commun/`. Les nouveaux tests d'intégration de la bibliothèque s'ajoutent dans le
//! module du thème qu'ils éprouvent, ou dans un nouveau module : pas ici.

#[path = "../commun/mod.rs"]
mod commun;

mod album;
mod edition;
mod reprise;
mod scan;
