//! LE SEUL module qui lit un secret compile-time.
//!
//! `option_env!()` n'apparaît nulle part ailleurs dans le crate — vérifié par
//! tests/architecture/secrets-un-seul-module.test.ts. Le garde-fou de build.rs est inclus
//! ici : en profil release sans secrets, la ligne suivante devient un `compile_error!()`.

include!(concat!(env!("OUT_DIR"), "/secrets_guard.rs"));

/// DSN Sentry (région EU). `None` en développement : Sentry reste inactif.
pub fn sentry_dsn() -> Option<&'static str> {
    option_env!("HEAN_SENTRY_DSN").filter(|s| !s.is_empty())
}

/// URL du worker Cloudflare qui porte le feedback (aucun token GitHub dans le binaire).
pub fn worker_url() -> Option<&'static str> {
    option_env!("NOIR_WORKER_URL").filter(|s| !s.is_empty())
}

/// Secret partagé app ↔ worker (en-tête `X-Noir-Secret`).
pub fn worker_secret() -> Option<&'static str> {
    option_env!("NOIR_WORKER_SECRET").filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// En test (profil debug) les secrets sont absents : les accesseurs doivent rendre `None`
    /// sans paniquer. C'est aussi ce qui prouve que le garde-fou ne s'applique QU'EN release.
    #[test]
    fn les_secrets_absents_ne_paniquent_pas() {
        let _ = (sentry_dsn(), worker_url(), worker_secret());
    }
}
