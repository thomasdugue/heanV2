// GÉNÉRATEUR niveau 1 — tokens CSS.
//
// Source de vérité : bootstrap/02-design/tokens.css, la référence de parité fixée par
// l'ADR 0004 (redesign v3, commit e12a200). Ce fichier-là est un DOCUMENT : il contient un
// @font-face au mauvais chemin et une section de pseudo-déclarations `<runtime>` hors bloc.
// Il n'est donc pas servable tel quel.
//
// Ce script en extrait les seules déclarations réelles des blocs `:root` et produit :
//   - src/styles/tokens.css          la feuille réellement servie
//   - src/styles/tokens.generated.ts les mêmes valeurs, typées, lisibles depuis TS
//   - docs/generated/tokens.md       le catalogue
//
// Conséquence : les tokens de V2 NE PEUVENT PAS diverger de la cible de parité. Les changer
// demande de changer la cible, donc un ADR. C'est le niveau 1 de ENTRETIEN-DU-HARNAIS.md.
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const REF = 'bootstrap/02-design/tokens.css';

// Les 10 variables posées par JS à l'exécution (relevé fait dans la référence elle-même).
// Elles DOIVENT avoir une valeur par défaut dans :root — sans quoi toute déclaration qui les
// lit est invalidée en silence avant le premier setProperty(). C'est la mécanique exacte de
// l'invariant INV-20 (--glass-* utilisées 45 fois sans définition).
const RUNTIME = {
  '--ambient-gradient-opacity': '0',
  '--ambient-luminosity': '0',
  '--ambient-watermark-opacity': '0',
  '--color-ambient-r': '10',
  '--color-ambient-g': '10',
  '--color-ambient-b': '10',
  '--color-ambient-2-r': '10',
  '--color-ambient-2-g': '10',
  '--color-ambient-2-b': '10',
};

/** Les déclarations `--x: valeur;` des blocs `:root` de la référence, dans l'ordre. */
export function parseRootTokens(css) {
  const out = [];
  const re = /:root\s*\{([\s\S]*?)\}/g;
  let block;
  while ((block = re.exec(css)) !== null) {
    for (const line of block[1].split('\n')) {
      const m = /^\s*(--[a-z0-9-]+)\s*:\s*([^;]+);/i.exec(line);
      if (m && !m[2].includes('<runtime>')) out.push([m[1], m[2].trim()]);
    }
  }
  return out;
}

const tokens = parseRootTokens(readFileSync(resolve(root, REF), 'utf8'));
if (tokens.length < 50) throw new Error(`${REF} : ${tokens.length} tokens, c'est trop peu — la référence a changé de forme`);

const ENTETE = `/* FICHIER GÉNÉRÉ — NE PAS ÉDITER À LA MAIN.
   Source : ${REF} (cible de parité, ADR 0004). Régénérer : npm run gen */`;

writeFileSync(
  resolve(root, 'src/styles/tokens.css'),
  `${ENTETE}\n\n:root {\n${tokens.map(([n, v]) => `  ${n}: ${v};`).join('\n')}\n\n` +
    `  /* Posées par JS à l'exécution — une valeur par défaut est OBLIGATOIRE (INV-20). */\n` +
    `${Object.entries(RUNTIME).map(([n, v]) => `  ${n}: ${v};`).join('\n')}\n}\n`,
);

const ts = tokens.map(([n, v]) => `  '${n}': ${JSON.stringify(v)},`).join('\n');
writeFileSync(
  resolve(root, 'src/styles/tokens.generated.ts'),
  `// FICHIER GÉNÉRÉ — NE PAS ÉDITER À LA MAIN. Source : ${REF}. Régénérer : npm run gen\n` +
    `export const TOKENS = {\n${ts}\n} as const;\n\n` +
    `export type TokenName = keyof typeof TOKENS;\n`,
);

const md = tokens.map(([n, v]) => `| \`${n}\` | \`${v}\` |`).join('\n');
writeFileSync(
  resolve(root, 'docs/generated/tokens.md'),
  `# Tokens CSS — catalogue\n\n<!-- FICHIER GÉNÉRÉ — NE PAS ÉDITER. Source : ${REF}. Régénérer : npm run gen -->\n\n` +
    `${tokens.length} variables, extraites de la cible de parité (ADR 0004).\n` +
    `S'y ajoutent ${Object.keys(RUNTIME).length} variables posées par JS à l'exécution, ` +
    `déclarées avec une valeur par défaut dans \`src/styles/tokens.css\`.\n\n` +
    `| Variable | Valeur |\n|---|---|\n${md}\n`,
);

console.log(`tokens : ${tokens.length} variables + ${Object.keys(RUNTIME).length} runtime`);
