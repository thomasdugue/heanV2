# Catalogue IPC

<!-- FICHIER GÉNÉRÉ — NE PAS ÉDITER. Source : les annotations `#[specta::specta]` du Rust. Régénérer : npm run gen -->

V1 appelait ses 86 commandes et ses 17 events par des chaînes de caractères, vérifiées nulle part.
Ici, renommer une commande côté Rust casse la compilation du frontend.

## Commandes (8)

| TypeScript | Rust | Retour |
|---|---|---|
| `appVersion` | `app_version` | `string` |
| `listTracks` | `list_tracks` | `Track_Serialize[]` |
| `scanLibrary` | `scan_library` | `RapportDeScan` |
| `addLibraryRoot` | `add_library_root` | `string[]` |
| `libraryRoots` | `library_roots` | `string[]` |
| `writeMetadata` | `write_metadata` | `Track_Serialize` |
| `bulkEdit` | `bulk_edit` | `Track_Serialize[]` |
| `excludeTrack` | `exclude_track` | `null` |

## Events (0)

Aucun pour l'instant.
