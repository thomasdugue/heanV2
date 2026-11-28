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

    // Le garde se LIE. Écrit `init_avec(...);` sans le lier, ce test échoue — vérifié : le
    // `Drop` immédiat arrête le fil d'écriture avant qu'il ait posé la ligne. C'est la preuve
    // que le garde n'est pas décoratif, et la raison du `#[must_use]` sur `init`.
    let _garde = hean_lib::observability::init_avec(Some(&base));
    assert!(_garde.is_some(), "un dossier donné doit rendre un garde");
    tracing::info!(preuve = "oui", "le raccord tient");

    // L'écrivain est non bloquant : la ligne traverse un fil. On laisse au garde le temps
    // de la vider plutôt que de supposer qu'elle est déjà là.
    let fichier = base.join("logs");
    let mut texte = String::new();
    for _ in 0..50 {
        texte = std::fs::read_dir(&fichier)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| std::fs::read_to_string(e.path()).unwrap_or_default())
            .collect();
        if texte.contains("le raccord tient") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

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
