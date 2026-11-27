// Garde ANTI-ORPHELIN des invariants (HARNAIS.md §6).
//
// V1 : 3 invariants sur 20 couverts par un test, et un compte annoncé (« 19 ») faux dans
// cinq documents parce que rien ne le vérifiait. Ce script ferme les deux trous :
//   - chaque invariant a une pile, une gravité, une couverture et un pourquoi ;
//   - un invariant qui annonce un test doit être cité par un test qui existe ;
//   - un test qui cite un invariant disparu échoue ;
//   - les compteurs du tableau du skill doivent correspondre à la réalité.
//
// Les comptes recopiés EN PROSE (CLAUDE.md, README) sont tenus par `tools/check-comptes.mjs`,
// pas ici : un même nombre ne se vérifie qu'à un seul endroit.
//
// Une couverture « planifiée (phase N) » est une dette ASSUMÉE et VISIBLE : le script la
// compte et l'affiche, il ne la cache pas. C'est la porte de sortie explicite.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const RACINE = resolve(import.meta.dirname, '..');
const SKILL = '.claude/skills/hean-invariants/SKILL.md';
// Les 29 fiches ont quitté SKILL.md pour CATALOGUE.md le 2026-11-25 : une session qui écrit
// du code lisait 617 lignes pour des règles déjà tenues par des tests. SKILL.md garde le
// tableau de tri et les pièges de plateforme — ce qu'aucun test ne prouvera. La source de
// vérité, pour cette vérification, reste LES DEUX fichiers : en lire un seul laisserait
// passer une fiche orpheline du côté non lu.
const CATALOGUE = '.claude/skills/hean-invariants/CATALOGUE.md';
const PILES = ['rayé', 'test', 'irréductible'];

const erreurs = [];
const skill =
  readFileSync(join(RACINE, SKILL), 'utf8') + '\n' + readFileSync(join(RACINE, CATALOGUE), 'utf8');

// --- 1. les invariants déclarés -------------------------------------------------------
const invariants = new Map();
for (const bloc of skill.split(/^### /m).slice(1)) {
  const id = /^(INV-\d{2})/.exec(bloc)?.[1];
  if (!id) continue;
  const champ = (n) => new RegExp(`^- ${n}\\s*:\\s*(.+)$`, 'm').exec(bloc)?.[1]?.trim();
  invariants.set(id, {
    titre: (/^INV-\d{2}\s+—\s+(.+)$/m.exec(bloc)?.[1] ?? '').trim(),
    pile: champ('pile'),
    gravite: champ('gravité'),
    couverture: champ('couverture'),
    pourquoi: champ('pourquoi'),
  });
}
if (invariants.size === 0) erreurs.push(`${SKILL} + ${CATALOGUE} : aucun invariant trouvé — le format a changé`);

for (const [id, inv] of invariants) {
  if (!PILES.includes(inv.pile)) erreurs.push(`${id} : pile « ${inv.pile} » inconnue (${PILES.join(', ')})`);
  if (!/BLOQUANT|AVERTISSEMENT/.test(inv.gravite ?? '')) erreurs.push(`${id} : gravité absente — une règle non classée est BLOQUANTE par défaut`);
  if (!inv.pourquoi) erreurs.push(`${id} : pas de « pourquoi » — la règle sera supprimée par quelqu'un qui la trouve arbitraire`);
  const c = inv.couverture ?? '';
  if (inv.pile === 'rayé' && !c.startsWith('sans objet')) erreurs.push(`${id} : rayé mais couverture « ${c} » (attendu « sans objet — <justification> »)`);
  if (inv.pile === 'irréductible' && !c.startsWith('manuelle')) erreurs.push(`${id} : irréductible mais couverture « ${c} » (attendu « manuelle — … »)`);
  if (inv.pile === 'test' && !/^(test |architecture |planifiée \(phase \d\))/.test(c)) {
    erreurs.push(`${id} : couverture « ${c} » invalide (« test X », « architecture X » ou « planifiée (phase N) »)`);
  }
}

// --- 2. les citations dans les tests ---------------------------------------------------
const citations = new Map();
const parcourir = (d) => {
  for (const e of readdirSync(d)) {
    const p = join(d, e);
    if (statSync(p).isDirectory()) parcourir(p);
    else if (e.endsWith('.ts') || e.endsWith('.rs')) {
      for (const m of readFileSync(p, 'utf8').matchAll(/\bINV-\d{2}\b/g)) {
        (citations.get(m[0]) ?? citations.set(m[0], new Set()).get(m[0])).add(e);
      }
    }
  }
};
parcourir(join(RACINE, 'tests'));
parcourir(join(RACINE, 'src-tauri/src'));
// `src-tauri/tests/` : même angle mort que celui trouvé sur INV-21 dans `backend.test.ts`.
// Tant que ce dossier ne contenait qu'un fichier, il ne se voyait pas ; une règle citée là y
// aurait été invisible, donc déclarée orpheline à tort au moment de la brancher sur un test.
parcourir(join(RACINE, 'src-tauri/tests'));

for (const id of citations.keys()) {
  if (!invariants.has(id)) erreurs.push(`${id} : cité par un test mais absent de ${SKILL} (test orphelin)`);
}
for (const [id, inv] of invariants) {
  const annonce = /^(test|architecture) (\S+)/.exec(inv.couverture ?? '');
  if (!annonce) continue;
  const nom = annonce[2].replace(/`/g, '');
  const cite = citations.get(id);
  if (!cite) erreurs.push(`${id} : annonce « ${inv.couverture} » mais aucun test ne cite ${id}`);
  else if (!cite.has(nom)) erreurs.push(`${id} : annonce ${nom}, mais ${id} n'est cité que par ${[...cite].join(', ')}`);
}

// --- 3. les compteurs annoncés dans le tableau du skill --------------------------------
const reels = { rayé: 0, test: 0, irréductible: 0 };
for (const inv of invariants.values()) if (inv.pile in reels) reels[inv.pile]++;
for (const pile of PILES) {
  const annonce = new RegExp(`\\|\\s*\\*\\*${pile}\\*\\*\\s*\\|[^|]*\\|\\s*(\\d+)`).exec(skill)?.[1];
  if (annonce === undefined) erreurs.push(`${SKILL} : le tableau de tri n'annonce pas de compte pour « ${pile} »`);
  else if (Number(annonce) !== reels[pile]) erreurs.push(`${SKILL} : le tableau annonce ${annonce} « ${pile} », il y en a ${reels[pile]}`);
}

// --- verdict ---------------------------------------------------------------------------
const planifies = [...invariants].filter(([, i]) => (i.couverture ?? '').startsWith('planifiée'));
if (erreurs.length) {
  for (const e of erreurs) console.log(`  ${e}`);
  console.log('check-invariants : ÉCHEC');
  process.exit(1);
}
console.log(
  `check-invariants : OK (${invariants.size} invariants — ${reels['rayé']} rayés, ` +
    `${reels['test']} tests dont ${planifies.length} planifiés, ${reels['irréductible']} irréductibles)`,
);
if (planifies.length) console.log(`  dette visible : ${planifies.map(([id]) => id).join(', ')}`);
