//! Le scan de la vraie bibliothèque, par le vrai code de la V2.
//!
//! Rien n'est réimplémenté ici. `hean_lib::library::scan::parcourir` trouve les fichiers,
//! `hean_lib::library::Depot` les indexe et les relit. Ce module ne fait que leur passer la
//! racine et sérialiser ce qu'ils rendent.

use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;
use std::time::Instant;

use hean_lib::library::{scan, Depot, RapportDeScan, Track};
use lofty::prelude::TaggedFileExt;
use serde::Serialize;

/// Ce que l'outil écrit pour le script de rapport. Les pistes sont les `Track` de la V2,
/// tels quels : c'est la base qui les rend, donc c'est bien ce que l'application verrait.
#[derive(Serialize)]
struct Sortie {
    racine: String,
    secondes: f64,
    rapport: RapportDeScan,
    /// Les chemins que le parcours a trouvés sur le disque, avant toute décision.
    trouvees: Vec<String>,
    /// Les pistes effectivement indexées, lues depuis SQLite.
    pistes: Vec<Track>,
    /// Les fichiers trouvés mais absents de la base : la V2 n'a pas su lire leurs tags.
    /// Déduit, et non recompté — `RapportDeScan::illisibles` donne le nombre, cette liste
    /// donne les noms, et les deux viennent du même scan.
    illisibles: Vec<String>,
    /// Pochette intégrée au fichier, par chemin. **Mesure de l'outil, pas de la V2** : la
    /// phase 2 n'indexe pas les pochettes, donc il n'y a aucun chiffre de la V2 à comparer.
    pochette_integree: Vec<String>,
}

pub fn executer(racine: &Path, base: &Path, sortie: &Path) -> Result<(), Box<dyn Error>> {
    if !racine.is_dir() {
        return Err(format!("{} n'est pas un dossier", racine.display()).into());
    }
    // Garde-fou : la base jetable ne doit JAMAIS être celle de l'application. Un outil de
    // diagnostic qui écrirait dans la vraie bibliothèque serait pire qu'inutile.
    if let Some(reel) = hean_lib::library::dossier::dossier_de_donnees() {
        if base.starts_with(&reel) {
            return Err(format!(
                "refus d'écrire dans le dossier de données de l'application ({})",
                reel.display()
            )
            .into());
        }
    }
    // Cet outil ne supprime et n'écrase RIEN, pas même un fichier qui porte le nom de sa
    // propre base jetable : il refuse. C'était la seule ligne de tout l'outil capable
    // d'effacer un fichier, et un garde-fou qui n'autorise que le bon dossier n'est pas la
    // même chose qu'une absence de suppression. L'appelant passe un nom neuf à chaque fois
    // (tools/passe-de-verite.sh y met le numéro de processus), donc le cas ne se présente
    // pas — et s'il se présentait, ce serait le signe qu'on est en train de viser un
    // fichier qui n'est pas à nous.
    if base.exists() {
        return Err(format!(
            "{} existe déjà. Cet outil n'écrase aucun fichier : donne-lui un autre chemin.",
            base.display()
        )
        .into());
    }

    let depart = Instant::now();
    let mut depot = Depot::ouvrir(base)?;
    depot.ajouter_racine(racine)?;
    let trouvees = scan::parcourir(&[racine.to_path_buf()]);
    let rapport = depot.scanner()?;
    let pistes = depot.lister_pistes()?;
    let secondes = depart.elapsed().as_secs_f64();

    let indexees: BTreeSet<String> = pistes.iter().map(|p| p.path.clone()).collect();
    let chemins: Vec<String> = trouvees
        .iter()
        .map(|t| t.chemin.display().to_string())
        .collect();
    let illisibles: Vec<String> = chemins
        .iter()
        .filter(|c| !indexees.contains(*c))
        .cloned()
        .collect();

    let pochette_integree = pistes
        .iter()
        .filter(|p| a_une_pochette(Path::new(&p.path)))
        .map(|p| p.path.clone())
        .collect();

    let json = serde_json::to_vec_pretty(&Sortie {
        racine: racine.display().to_string(),
        secondes,
        rapport,
        trouvees: chemins,
        pistes,
        illisibles,
        pochette_integree,
    })?;
    if let Some(parent) = sortie.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(sortie, json)?;

    // Ces deux lignes partent sur stdout pour que le script appelant les relaie à Thomas.
    println!("scan terminé en {secondes:.1} s");
    println!(
        "{} fichiers trouvés, {} pistes indexées, {} illisibles",
        trouvees.len(),
        indexees.len(),
        rapport.illisibles
    );
    Ok(())
}

/// Vrai si le fichier porte une image. En lecture seule, et sans jamais paniquer sur un
/// fichier abîmé : un diagnostic qui s'arrête au premier fichier douteux ne diagnostique
/// rien.
fn a_une_pochette(chemin: &Path) -> bool {
    let Ok(fichier) = lofty::read_from_path(chemin) else {
        return false;
    };
    fichier
        .primary_tag()
        .or_else(|| fichier.first_tag())
        .is_some_and(|t| !t.pictures().is_empty())
}

