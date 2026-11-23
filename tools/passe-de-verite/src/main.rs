//! # OUTIL TEMPORAIRE — « passe de vérité ». À jeter.
//!
//! Il n'est pas un morceau du produit et ne doit jamais en devenir un : pas de commande IPC,
//! pas d'interface, rien qui entre dans l'application livrée (voir `Cargo.toml`). Le jour où
//! l'interface existe, cet outil et son dossier disparaissent du dépôt.
//!
//! ## À quoi il sert
//!
//! Tout ce que les phases 0 à 2 ont construit a été vérifié par des tests écrits par les
//! mêmes sessions qui ont écrit le code, sur des fixtures fabriquées pour l'occasion.
//! Personne n'avait jamais fait tourner la V2 sur la vraie bibliothèque de Thomas, et
//! personne n'avait jamais entendu un son en sortir. Cet outil fait les deux, en ligne de
//! commande, avant qu'on construise une interface par-dessus une fondation non mesurée.
//!
//! ## Deux sous-commandes
//!
//! - `scanner <racine> <base.db> <sortie.json>` — indexe `racine` avec le **vrai** code de
//!   la V2 (`hean_lib::library`) et écrit ce qu'il a trouvé en JSON, que
//!   `tools/passe-de-verite-rapport.py` compare ensuite aux données de la V1.
//! - `jouer <fichier> <sortie.wav>` — décode `fichier` par le **vrai** chemin audio de la
//!   V2 (`Decodeur` → chaîne transparente → `BackendAudio`) et écrit le PCM obtenu.
//!
//! ## Lecture seule, sans exception
//!
//! Les fichiers de musique sont **lus** et jamais écrits : `hean_lib::library::metadonnees`
//! sait aussi écrire des tags, et cet outil n'appelle jamais `ecrire`. La base SQLite qu'il
//! crée est jetable et son chemin est donné en argument — jamais
//! `~/Library/Application Support/hean/`, pour que l'outil ne puisse pas abîmer la
//! bibliothèque de l'application, **ni même la créer** : ce dossier n'existe pas encore chez
//! Thomas, et la migration `noir/` → `hean/` s'éteindrait définitivement s'il apparaissait.
//! Rien ici ne touche aux données de la V1 : c'est le script Python qui les lit, et en
//! lecture seule.
//!
//! Et aucune suppression, nulle part : pas un `remove`, pas un `rename`. Le scanner
//! **refuse** un chemin de base déjà occupé au lieu de l'écraser. La seule écriture du
//! `jouer` est celle du `BackendAudio` dans le WAV temporaire — `backend.ecrire`, qui n'a
//! du nom de `metadonnees::ecrire` que la ressemblance.
//!
//! Aucun chemin de machine n'est écrit dans ce dossier : la racine à scanner arrive en
//! argument (`tests/architecture/aucun-chemin-machine.test.ts` l'exige, et il a raison).

mod jouer;
mod scanner;

use std::path::PathBuf;
use std::process::ExitCode;

const MODE_DEMPLOI: &str = "\
passe-de-verite — outil temporaire, hors produit

  passe-de-verite scanner <racine> <base.db> <sortie.json>
  passe-de-verite jouer   <fichier-audio> <sortie.wav>
";

fn main() -> ExitCode {
    // L'outil n'initialise PAS `observability` : les journaux de l'app partent dans un
    // fichier, et ici on veut les avertissements du scan sous les yeux, sur stderr.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let resultat = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["scanner", racine, base, sortie] => scanner::executer(
            &PathBuf::from(racine),
            &PathBuf::from(base),
            &PathBuf::from(sortie),
        ),
        ["jouer", fichier, sortie] => {
            jouer::executer(&PathBuf::from(fichier), &PathBuf::from(sortie))
        }
        _ => {
            eprint!("{MODE_DEMPLOI}");
            return ExitCode::FAILURE;
        }
    };

    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("échec : {e}");
            ExitCode::FAILURE
        }
    }
}
