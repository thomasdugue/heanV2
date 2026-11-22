//! GÉNÉRATEUR niveau 1 — les types TypeScript de l'IPC.
//!
//! `cargo test --test export_bindings` réécrit `src/ipc/bindings.ts` depuis les annotations
//! `#[specta::specta]` du Rust. La CI régénère et échoue si le résultat diffère du commit
//! (tools/check-generated.sh), donc le catalogue IPC ne peut pas dériver du code.
//!
//! V1 : 86 commandes et 17 events appelés par des chaînes de caractères, vérifiées nulle part.

#[test]
fn export_bindings() {
    hean_lib::ipc_builder()
        .export(
            specta_typescript::Typescript::default(),
            "../src/ipc/bindings.ts",
        )
        .expect("export des bindings TypeScript");
}
