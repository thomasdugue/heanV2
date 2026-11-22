//! Lance la migration sur deux chemins donnés. Sert à `tools/verifier-migration-reelle.sh`,
//! qui l'exécute sur une COPIE du vrai dossier de données.
//!
//! Un exemple et non un test : un test ne prend pas d'argument, et la vérification réelle a
//! besoin de désigner un bac temporaire. Il n'est pas compilé dans l'application.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [source, cible] = [args.first(), args.get(1)].map(|a| a.cloned().unwrap_or_default());
    if source.is_empty() || cible.is_empty() {
        eprintln!("usage : migrer <dossier-noir> <dossier-hean>");
        std::process::exit(2);
    }
    let cible = std::path::PathBuf::from(&cible);
    match hean_lib::migration::migrer_si_necessaire(std::path::Path::new(&source), &cible) {
        Ok(rapport) => println!("    {rapport:?}"),
        Err(e) => {
            eprintln!("    échec : {e}");
            std::process::exit(1);
        }
    }

    // La reprise de la configuration, sur le vrai `config.json` : c'est la moitié de la
    // migration que déplacer des fichiers ne fait pas (ADR 0010). Sur données réelles, elle
    // dit combien de dossiers de bibliothèque et combien d'exclusions V1 avait vraiment.
    let Ok(config) = std::fs::read_to_string(cible.join("config.json")) else {
        println!("    pas de config.json à reprendre");
        return;
    };
    let mut depot = match hean_lib::library::Depot::ouvrir(&cible.join("library.db")) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("    ouverture de la base : {e}");
            std::process::exit(1);
        }
    };
    match hean_lib::library::reprise_v1::reprendre(&mut depot, &config) {
        Ok(reprise) => println!("    {reprise:?}"),
        Err(e) => {
            eprintln!("    reprise : {e}");
            std::process::exit(1);
        }
    }
}
