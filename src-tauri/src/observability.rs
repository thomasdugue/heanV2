//! Journalisation. `println!` / `eprintln!` sont interdits dans ce crate (INV-21) :
//! V1 en comptait 252 contre 3 appels `tracing::`, et son `logging.rs` ne capturait pas
//! stdout — l'app installée a produit 63 lignes de journal dans toute sa vie, et les trois
//! bugs trouvés le 2026-11-21 étaient annoncés en clair en dev et invisibles en production.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Installe le souscripteur global. Idempotent : un second appel est ignoré.
pub fn init() {
    let filtre = EnvFilter::try_from_env("HEAN_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::registry()
        .with(filtre)
        .with(fmt::layer().with_target(true))
        .try_init();
}

#[cfg(test)]
mod tests {
    #[test]
    fn init_est_idempotent() {
        super::init();
        super::init();
    }
}
