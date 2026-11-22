// Pas de console au lancement en release (macOS/Windows).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    hean_lib::run();
}
