//! Le journal fichier, éprouvé sur la composition entière et non sur son écrivain seul.
//!
//! Le test unitaire d'`observability` prouve que l'écrivain pose une ligne sur le disque.
//! Il ne prouve pas que le souscripteur global y est branché — et c'est ce raccord, une
//! seule ligne dans `init`, qui décide si un `tracing::info!` du vrai code atterrit quelque
//! part. D'où un test dans SON PROCESSUS : `try_init` n'installe qu'un souscripteur par
//! processus, donc la preuve n'est juste que si personne ne l'a devancé.

use std::path::PathBuf;

fn dossier_dessai() -> PathBuf {
    std::env::temp_dir().join(format!("hean-journal-integration-{}", std::process::id()))
}

#[test]
fn un_info_du_code_reel_atterrit_dans_le_fichier() {
    let base = dossier_dessai();
    let _ = std::fs::remove_dir_all(&base);

    hean_lib::observability::init_avec(Some(&base));
    tracing::info!(preuve = "oui", "le raccord tient");

    // L'écriture est synchrone : la ligne est sur le disque quand `info!` rend la main. Rien
    // à attendre, aucun garde à lâcher — c'est tout l'intérêt du choix (voir `init`).
    let texte: String = std::fs::read_dir(base.join("logs"))
        .expect("le dossier logs doit exister")
        .flatten()
        .map(|e| std::fs::read_to_string(e.path()).unwrap_or_default())
        .collect();

    assert!(
        texte.contains("le raccord tient"),
        "le journal fichier n'a rien reçu du souscripteur global ; lu : {texte:?}"
    );
    assert!(
        texte.contains("hean_lib::observability") || texte.contains("journal_fichier"),
        "la cible devrait être journalisée : {texte:?}"
    );

    let _ = std::fs::remove_dir_all(&base);
}
