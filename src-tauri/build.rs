// Garde-fou des secrets compile-time.
//
// Piège V1 (invariant INV-17) : `option_env!()` renvoie `None` EN SILENCE quand la variable
// manque. Le binaire release partait alors sans Sentry et sans feedback, sans aucune erreur —
// on ne le découvrait que chez les testeurs. V1 s'en protégeait avec un wrapper `release.sh`
// qu'il suffisait d'oublier une fois.
//
// En V2 le compilateur refuse : ce script écrit un `compile_error!()` dans OUT_DIR, que
// src/secrets.rs inclut. Un build release sans secrets NE COMPILE PAS.
//
// Porte de sortie explicite : HEAN_ALLOW_MISSING_SECRETS=1 dégrade l'erreur en avertissement,
// pour un build release local de test. Elle est tracée dans la sortie de compilation.
use std::{env, fs, path::PathBuf};

const SECRETS: [&str; 3] = ["HEAN_SENTRY_DSN", "NOIR_WORKER_URL", "NOIR_WORKER_SECRET"];

fn main() {
    tauri_build::build();

    for name in SECRETS {
        println!("cargo::rerun-if-env-changed={name}");
    }
    println!("cargo::rerun-if-env-changed=HEAN_ALLOW_MISSING_SECRETS");

    let release = env::var("PROFILE").as_deref() == Ok("release");
    let manquants: Vec<&str> = SECRETS
        .iter()
        .copied()
        .filter(|n| env::var_os(n).is_none_or(|v| v.is_empty()))
        .collect();
    let derogation = env::var_os("HEAN_ALLOW_MISSING_SECRETS").is_some_and(|v| v == "1");

    let garde = if release && !manquants.is_empty() && !derogation {
        format!(
            "compile_error!(\"secrets compile-time manquants en profil release : {}. \
             Les renseigner (voir bootstrap/06-setup-v2/ci/secrets.env.template), \
             ou poser HEAN_ALLOW_MISSING_SECRETS=1 pour un build de test sans eux.\");\n",
            manquants.join(", ")
        )
    } else {
        if release && !manquants.is_empty() {
            println!(
                "cargo::warning=build release SANS les secrets {} \
                 (HEAN_ALLOW_MISSING_SECRETS=1). Ne pas distribuer ce binaire.",
                manquants.join(", ")
            );
        }
        String::new()
    };

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("secrets_guard.rs");
    fs::write(&out, garde).expect("écriture du garde-fou des secrets");
}
