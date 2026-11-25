//! Ce que la part réseau de la bibliothèque garantit (ADR 0016).
//!
//! **Aucun de ces tests ne touche le NAS de Thomas**, et ce n'est pas une commodité : il
//! n'est pas toujours allumé, et la CI n'y a aucun accès. Ce qui est éprouvé ici l'est donc
//! derrière le trait `network::client::Smb`, avec un faux partage en mémoire et des pannes
//! **provoquées** — la doctrine du projet est *généré > apparié > réclamé*, et une panne
//! provoquée vaut mieux qu'une assertion.
//!
//! Ce que ces tests ne prouvent pas, et qu'il faut savoir : que `libsmbclient` fasse ce que
//! sa documentation dit. C'est la part irréductible (PLAT-2, INV-03), et elle se mesure sur
//! le vrai NAS, une fois, avec le protocole de la fin de `docs/HISTORY.md`.

// `allow(dead_code)` : le bac à sable est partagé avec les tests de bibliothèque, et cette
// cible-ci n'en utilise pas toutes les méthodes. Le `allow` est sur l'import et non dans
// `commun/mod.rs` : la même ligne là-bas désarmerait la détection pour toutes les cibles.
#[allow(dead_code)]
#[path = "../commun/mod.rs"]
mod commun;

mod faux_partage;

mod entete;
mod perimetre;
mod uri;
