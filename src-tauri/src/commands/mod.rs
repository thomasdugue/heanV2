//! Commandes IPC. Chacune porte `#[tauri::command]` ET `#[specta::specta]`, et doit être
//! inscrite dans `collect_commands!` (src/lib.rs) — sinon elle n'existe pas côté TypeScript.
//!
//! `app_version` sert à prouver la chaîne complète Rust → tauri-specta → bindings.ts →
//! client.ts → Svelte. Les commandes de la bibliothèque sont dans `bibliotheque.rs`, celles
//! de la lecture dans `lecture.rs`.

pub mod bibliotheque;
pub mod lecture;
pub mod reseau;

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
