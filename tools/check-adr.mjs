// Les décisions (ADR) : un numéro unique, une présence dans l'index, un état valide, et
// aucun renvoi qui ne désigne rien.
//
// Né le 2026-11-22 : deux sessions ont numéroté en parallèle, chacune prenant « le suivant
// après 0006 » sans voir l'autre. Deux décisions différentes ont porté le même numéro
// pendant une heure, l'index n'en listait qu'une, et le même `README` disait « ADR 0007 »
// de deux choses différentes à dix-sept lignes d'écart. Rien ne regardait les ADR, alors
// qu'ils ont les trois propriétés qui rendent un contrôle rentable.
//
// La collision se reproduira : deux branches vivantes en même temps est le mode de travail
// normal de ce projet. Ce script la refuse.
//
// ⚠️ Il lit AUSSI les renvois en texte nu (« ADR » suivi d'un numéro, dans un .toml, un .sh,
// un .yml). Quatre des huit renvois de la collision étaient de cette forme : aucun
// vérificateur de liens ne les voit, et c'est ce qui a failli être oublié.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';

const RACINE = resolve(import.meta.dirname, '..');
const INDEX = 'docs/adr/README.md';
const ETATS = ['Acceptée', 'Proposée', 'Remplacée', 'Rejetée'];
const BINAIRES = /\.(png|jpe?g|gif|webp|woff2?|icns|ico|wav|flac|mp3|m4a|aiff?|pdf|lock)$/i;

const lire = (p) => readFileSync(join(RACINE, p), 'utf8');
const erreurs = [];

/** Les fichiers SUIVIS par git — donc jamais ceux d'un worktree de session. */
const suivis = execFileSync('git', ['ls-files', '-z'], { cwd: RACINE, encoding: 'utf8' })
  .split('\0')
  .filter((f) => f && !BINAIRES.test(f));

// --- 1. un numéro, un fichier ----------------------------------------------------------
const parNumero = new Map();
for (const f of suivis) {
  const m = /^docs\/adr\/(\d{4})-[a-z0-9-]+\.md$/.exec(f);
  if (!m) continue;
  const n = m[1];
  if (parNumero.has(n)) {
    erreurs.push(`numéro ${n} en double : ${basename(parNumero.get(n))} et ${basename(f)}`);
    erreurs.push(`  → le plus récemment arrivé prend le numéro suivant libre (voir ${INDEX})`);
  } else parNumero.set(n, f);
}
if (parNumero.size === 0) erreurs.push(`aucun ADR trouvé — docs/adr/ a changé de forme ?`);

// --- 2. état valide, et présence dans l'index ------------------------------------------
const index = lire(INDEX);
for (const [, f] of [...parNumero].sort()) {
  const etat = /^-\s+\*\*État\*\*\s*:\s*(.+)$/m.exec(lire(f))?.[1]?.trim();
  if (!etat) erreurs.push(`${basename(f)} : pas de ligne « - **État** : … »`);
  else if (!ETATS.includes(etat)) erreurs.push(`${basename(f)} : état « ${etat} » hors de ${ETATS.join(' / ')}`);
  if (!index.includes(basename(f))) {
    erreurs.push(`${basename(f)} n'est pas dans ${INDEX} — une décision acceptée mais invisible`);
  }
}

// --- 3. tout renvoi désigne un ADR qui existe ------------------------------------------
// Deux formes : « ADR 0008 » / « ADR [0008] » en texte, et un chemin vers le fichier.
const EN_TEXTE = /\bADR\s*\[?(\d{4})\]?/g;
const EN_CHEMIN = /\badr\/(\d{4})-[a-z0-9-]+\.md\b/g;
for (const f of suivis) {
  if (f === INDEX) continue; // l'index les cite tous, par construction
  let texte;
  try {
    texte = lire(f);
  } catch {
    continue; // lien symbolique ou fichier illisible : hors sujet
  }
  const vus = new Set();
  for (const re of [EN_TEXTE, EN_CHEMIN]) {
    re.lastIndex = 0;
    for (const m of texte.matchAll(re)) vus.add(m[1]);
  }
  for (const n of vus) {
    if (!parNumero.has(n)) erreurs.push(`${f} renvoie à l'ADR ${n}, qui n'existe pas`);
  }
}

if (erreurs.length) {
  for (const e of erreurs) console.log(`  ${e}`);
  console.log('check-adr : ÉCHEC');
  process.exit(1);
}
console.log(`check-adr : OK (${parNumero.size} décisions, numéros uniques, index à jour)`);
