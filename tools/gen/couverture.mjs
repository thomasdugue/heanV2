// GÉNÉRATEUR niveau 1 — les deux tableaux de couverture.
//   « invariant → ce qui le protège »  (HARNAIS.md §9)
//   « comportement du contrat → son test »
// Dérivés du skill hean-invariants et de tests/ux/_contrat.ts. Les écrire à la main
// reviendrait à recopier à la main ce que la machine sait déjà : c'est ainsi que V1 s'est
// retrouvé avec 11 affirmations fausses dans sa documentation.
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const RACINE = resolve(import.meta.dirname, '../..');
const lire = (p) => readFileSync(resolve(RACINE, p), 'utf8');

// --- invariants ---
// SKILL.md (tri + pièges de plateforme) et CATALOGUE.md (les 27 fiches), depuis le 2026-11-25.
const skill =
  lire('.claude/skills/hean-invariants/SKILL.md') +
  '\n' +
  lire('.claude/skills/hean-invariants/CATALOGUE.md');
const invs = skill
  .split(/^### /m)
  .slice(1)
  .map((b) => ({
    id: /^(INV-\d{2})/.exec(b)?.[1],
    titre: (/^INV-\d{2}\s+—\s+(.+)$/m.exec(b)?.[1] ?? '').trim(),
    pile: /^- pile\s*:\s*(.+)$/m.exec(b)?.[1]?.trim(),
    gravite: /^- gravité\s*:\s*(.+)$/m.exec(b)?.[1]?.trim(),
    couverture: /^- couverture\s*:\s*(.+)$/m.exec(b)?.[1]?.trim(),
  }))
  .filter((x) => x.id);

const PILES = { rayé: '✂️ rayé', test: '🧪 test', irréductible: '🪨 irréductible' };
const compte = (p) => invs.filter((i) => i.pile === p).length;

writeFileSync(
  resolve(RACINE, 'docs/generated/invariants.md'),
  `# Invariants — ce qui protège quoi\n\n<!-- FICHIER GÉNÉRÉ — NE PAS ÉDITER. Source : .claude/skills/hean-invariants/SKILL.md + CATALOGUE.md. Régénérer : npm run gen -->\n\n` +
    `${invs.length} règles : ${compte('rayé')} rayées par construction, ${compte('test')} devenues des tests, ` +
    `${compte('irréductible')} irréductibles.\n` +
    `En V1, **3 sur 20** étaient couvertes par un test.\n\n` +
    `| # | Règle | Pile | Gravité | Ce qui la protège |\n|---|---|---|---|---|\n` +
    invs
      .map((i) => `| ${i.id} | ${i.titre} | ${PILES[i.pile] ?? i.pile} | ${i.gravite} | ${i.couverture} |`)
      .join('\n') +
    `\n`,
);

// --- comportements UX ---
const contrat = lire('tests/ux/_contrat.ts');
const comps = [...contrat.matchAll(/^\s*c\('([VW]\d{3})',\s*'([^']*)',\s*\n?\s*'([^']*)',\s*(true|false),\s*(\d)\)/gm)].map(
  (m) => ({ id: m[1], attendu: m[2], plateforme: m[4] === 'true', phase: Number(m[5]) }),
);
if (comps.length !== 24) throw new Error(`contrat : ${comps.length} comportements lus, 24 attendus`);

writeFileSync(
  resolve(RACINE, 'docs/generated/ux.md'),
  `# Contrat anti-régression — avancement\n\n<!-- FICHIER GÉNÉRÉ — NE PAS ÉDITER. Source : tests/ux/_contrat.ts. Régénérer : npm run gen -->\n\n` +
    `Les ${comps.length} comportements cassés au moins une fois en V1. \`./check\` affiche le compte vert/rouge ; ` +
    `il est **normal qu'ils soient tous rouges tant que la phase correspondante n'est pas faite**.\n` +
    `${comps.filter((x) => x.plateforme).length} sur ${comps.length} sont des pièges de plateforme : ils reviendront quelle que soit la techno.\n\n` +
    `| # | Attendu | Phase | Piège de plateforme |\n|---|---|---|---|\n` +
    comps.map((x) => `| ${x.id} | ${x.attendu} | ${x.phase} | ${x.plateforme ? 'oui' : '—'} |`).join('\n') +
    `\n`,
);

console.log(`couverture : ${invs.length} invariants, ${comps.length} comportements`);
