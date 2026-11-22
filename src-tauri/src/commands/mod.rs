//! Commandes IPC. Chacune porte `#[tauri::command]` ET `#[specta::specta]`, et doit être
//! inscrite dans `collect_commands!` (src/lib.rs) — sinon elle n'existe pas côté TypeScript.
//!
//! Phase 0 : une seule commande, qui sert à prouver la chaîne complète
//! Rust → tauri-specta → bindings.ts → client.ts → Svelte.

/// Version de l'application, telle que déclarée dans `Cargo.toml`.
#[tauri::command]
#[specta::specta]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_version_rend_la_version_du_crate() {
        assert_eq!(super::app_version(), env!("CARGO_PKG_VERSION"));
    }
}
