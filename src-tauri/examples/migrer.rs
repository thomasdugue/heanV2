//! Lance la **séquence de démarrage** sur deux chemins donnés. Sert à
//! `tools/verifier-migration-reelle.sh`, qui l'exécute sur une COPIE du vrai dossier de
//! données.
//!
//! Un exemple et non un test : un test ne prend pas d'argument, et la vérification réelle a
//! besoin de désigner un bac temporaire. Il n'est pas compilé dans l'application — c'est sur
//! quoi tient sa dérogation à INV-21 (ADR 0011) : ses `println!` ne peuvent pas rendre l'app
//! aveugle chez un testeur, puisqu'ils n'y sont pas.
//!
//! **Il appelle `demarrage::preparer`, et non les trois étapes à la main.** Il les
//! reproduisait jusqu'au 2026-11-23, et c'était le défaut corrigé par l'ADR 0012 présent
//! dans l'outil censé le vérifier : la seule vérification qui touche de vraies données
//! éprouvait une séquence qui n'était pas celle du produit. Une vérification qui n'exécute
//! pas ce que l'utilisateur exécute ne vérifie que son propre scénario.

fn main() {
    // **Un abonné `tracing`, sinon l'outil est aveugle à ce qu'il vérifie.** La migration dit
    // beaucoup de choses par ce canal — fichier caché non déplaçable, ancien dossier non
    // supprimé, temps passé à forcer la copie sur le disque — et sans abonné elles partent
    // toutes dans le vide, y compris pendant la seule vérification qui touche de vraies
    // données. C'est le mode de défaut d'INV-21 dans l'outil chargé de voir.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let [source, cible] = [args.first(), args.get(1)].map(|a| a.cloned().unwrap_or_default());
    if source.is_empty() || cible.is_empty() {
        eprintln!("usage : migrer <dossier-noir> <dossier-hean>");
        std::process::exit(2);
    }
    let (source, cible) = (std::path::PathBuf::from(&source), std::path::PathBuf::from(&cible));

    // Exactement ce que fait `run()` : migration, ouverture de la bibliothèque, reprise de la
    // configuration de V1 — et un échec n'écrit rien dans le dossier de données.
    let depot = match hean_lib::demarrage::preparer(&source, &cible) {
        Ok(depot) => depot,
        Err(e) => {
            eprintln!("    échec : {e}");
            std::process::exit(1);
        }
    };

    let temoin = cible.join(hean_lib::migration::inventaire::TEMOIN);
    println!(
        "    témoin : {}",
        if temoin.exists() { "écrit" } else { "ABSENT" }
    );
    match (depot.racines(), depot.chemins_exclus()) {
        (Ok(racines), Ok(exclus)) => println!(
            "    reprise : {} dossier(s) de bibliothèque, {} exclusion(s)",
            racines.len(),
            exclus.len()
        ),
        _ => {
            eprintln!("    la bibliothèque ne se relit pas après la migration");
            std::process::exit(1);
        }
    }
}
