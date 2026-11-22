// Garde ANTI-ORPHELIN des invariants (HARNAIS.md §6).
//
// V1 : 3 invariants sur 20 couverts par un test, et un compte annoncé (« 19 ») faux dans
// cinq documents parce que rien ne le vérifiait. Ce script ferme les deux trous :
//   - chaque invariant a une pile, une gravité, une couverture et un pourquoi ;
//   - un invariant qui annonce un test doit être cité par un test qui existe ;
//   - un test qui cite un invariant disparu échoue ;
//   - les compteurs du tableau du skill doivent correspondre à la réalité.
//
// Une couverture « planifiée (phase N) » est une dette ASSUMÉE et VISIBLE : le script la
// compte et l'affiche, il ne la cache pas. C'est la porte de sortie explicite.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const RACINE = resolve(import.meta.dirname, '..');
const SKILL = '.claude/skills/hean-invariants/SKILL.md';
const PILES = ['rayé', 'test', 'irréductible'];

const erreurs = [];
const skill = readFileSync(join(RACINE, SKILL), 'utf8');

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
if (invariants.size === 0) erreurs.push(`${SKILL} : aucun invariant trouvé — le format a changé`);

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

// --- 4. les comptes que CLAUDE.md RÉCLAME ---------------------------------------------
// CLAUDE.md est lu en entier à chaque session : un compte faux y est lu par tout le monde.
// Ces quatre nombres étaient au niveau « réclamé » — ils sont devenus faux à la minute où
// INV-22 est née. C'est exactement la dérive que la section 3 interdit déjà au skill
// (ADR 0005 : tout défaut repart avec la vérification qui l'aurait vu).
const claudeMd = readFileSync(join(RACINE, 'CLAUDE.md'), 'utf8');
const planifiesN = [...invariants.values()].filter((i) => (i.couverture ?? '').startsWith('planifiée')).length;
for (const [motif, attendu, quoi] of [
  [/\*\*(\d+) règles\*\*/, invariants.size, 'règles au total'],
  [/\*\*Rayées \((\d+)\)\*\*/, reels['rayé'], 'rayées'],
  [/\*\*Devenues des tests \((\d+)\)\*\*/, reels['test'], 'devenues des tests'],
  [/\*\*Irréductibles \((\d+) /, reels['irréductible'], 'irréductibles'],
  [/\*\*(\d+) « planifiées »\*\*/, planifiesN, 'planifiées'],
]) {
  const annonce = motif.exec(claudeMd)?.[1];
  if (annonce === undefined) erreurs.push(`CLAUDE.md : n'annonce plus de compte de « ${quoi} » (motif ${motif})`);
  else if (Number(annonce) !== attendu) erreurs.push(`CLAUDE.md : annonce ${annonce} ${quoi}, il y en a ${attendu}`);
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
