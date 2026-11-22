// GÉNÉRATEUR niveau 1 — catalogue des commandes et des events IPC.
// Dérivé de src/ipc/bindings.ts, lui-même généré depuis les annotations Rust par
// tauri-specta (src-tauri/tests/export_bindings.rs). Personne n'écrit ce catalogue.
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const RACINE = resolve(import.meta.dirname, '../..');
const b = readFileSync(resolve(RACINE, 'src/ipc/bindings.ts'), 'utf8');

const commandes = [...b.matchAll(/^\t(\w+):[^\n]*?__TAURI_INVOKE<([^>]*)>\("([^"]+)"/gm)].map((m) => ({
  ts: m[1],
  retour: m[2],
  rust: m[3],
}));
const events = [...b.matchAll(/^\t(\w+):\s*"([^"]+)"/gm)].map((m) => ({ ts: m[1], rust: m[2] }));

const ligne = (c) => `| \`${c.ts}\` | \`${c.rust}\` | \`${c.retour}\` |`;
writeFileSync(
  resolve(RACINE, 'docs/generated/ipc.md'),
  `# Catalogue IPC\n\n<!-- FICHIER GÉNÉRÉ — NE PAS ÉDITER. Source : les annotations \`#[specta::specta]\` du Rust. Régénérer : npm run gen -->\n\n` +
    `V1 appelait ses 86 commandes et ses 17 events par des chaînes de caractères, vérifiées nulle part.\n` +
    `Ici, renommer une commande côté Rust casse la compilation du frontend.\n\n` +
    `## Commandes (${commandes.length})\n\n| TypeScript | Rust | Retour |\n|---|---|---|\n${commandes.map(ligne).join('\n')}\n\n` +
    `## Events (${events.length})\n\n` +
    (events.length ? `| TypeScript | Rust |\n|---|---|\n${events.map((e) => `| \`${e.ts}\` | \`${e.rust}\` |`).join('\n')}\n` : `Aucun pour l'instant.\n`),
);
console.log(`ipc : ${commandes.length} commandes, ${events.length} events`);
