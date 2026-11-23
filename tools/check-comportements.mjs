// Les comportements PORTÉS entrent dans le verdict. Les autres restent le tableau de bord.
//
// POURQUOI ce script existe — relevé par l'agent invariant-guardian à la relecture de la
// phase 2, et c'est un trou que cette phase venait d'ouvrir. Les 24 comportements de
// `UX-ANTI-REGRESSION.md` tournaient dans une suite à part, dont `./check` affichait le
// compte **sans jamais toucher son verdict**. C'était sans conséquence tant qu'aucun n'était
// porté : ils étaient tous rouges, et c'était l'avancement du projet, pas une régression.
//
// Dès qu'un comportement est porté, la même indulgence devient un angle mort : V012 ou V004
// pourraient régresser en phase 3, `./check` resterait **VERT**, et le tableau de bord
// relabelliserait la régression en dette (« 19 restent à porter »). C'est le mode de défaut
// d'INV-21 appliqué au harnais : un défaut qui ne se voit pas se propage. Et le rattraper
// par le protocole de session serait de la discipline, ce que l'ADR 0005 refuse par principe.
//
// La règle, donc : **un comportement porté doit passer ; un comportement à porter doit
// échouer.** La seconde moitié compte autant que la première — un `aPorter()` sur un
// comportement qui passe voudrait dire que le tableau de bord sous-compte l'avancement.
//
// Rien n'est écrit à la main ici : le total vient du contrat, la liste des portés se déduit
// des fichiers de test, et l'état vient de la suite elle-même.
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';

const RACINE = resolve(import.meta.dirname, '..');
const UX = join(RACINE, 'tests/ux');
const erreurs = [];

// --- 1. ce que le contrat annonce, et ce que les tests disent ---------------------------
const contrat = readFileSync(join(UX, '_contrat.ts'), 'utf8');
const TOUS = [...contrat.matchAll(/^\s*c\('([VW]\d{3})'/gm)].map((m) => m[1]);

const fichiers = readdirSync(UX).filter((f) => f.endsWith('.test.ts') && f !== 'contrat-complet.test.ts');
const cites = new Map(); // id → titre complet du test
const aPorter = new Set();
for (const f of fichiers) {
  const texte = readFileSync(join(UX, f), 'utf8');
  for (const m of texte.matchAll(/^\s*it\('(([VW]\d{3})[^']*)'/gm)) cites.set(m[2], m[1]);
  for (const m of texte.matchAll(/aPorter\('([VW]\d{3})'\)/g)) aPorter.add(m[1]);
}
const portes = TOUS.filter((id) => cites.has(id) && !aPorter.has(id));

if (TOUS.length === 0) erreurs.push('tests/ux/_contrat.ts : aucun comportement lu — le format a changé');
for (const id of TOUS) {
  if (!cites.has(id)) erreurs.push(`${id} est au contrat mais aucun test ne le cite`);
}

// --- 2. l'état réel de la suite ---------------------------------------------------------
// Le rapport JSON de vitest, et non le comptage d'une ligne de résumé : on a besoin de
// savoir QUEL comportement est rouge, pas combien.
const bac = mkdtempSync(join(tmpdir(), 'hean-ux-'));
const rapport = join(bac, 'ux.json');
try {
  try {
    execFileSync('npm', ['run', '--silent', 'test:ux', '--', '--reporter=json', `--outputFile=${rapport}`], {
      cwd: RACINE,
      stdio: 'pipe',
    });
  } catch {
    // Une suite avec des rouges fait sortir vitess en échec : c'est ATTENDU ici, tant que
    // les rouges sont les comportements encore à porter. Le rapport, lui, est écrit.
  }

  let json;
  try {
    json = JSON.parse(readFileSync(rapport, 'utf8'));
  } catch {
    erreurs.push('la suite UX n\'a pas produit de rapport : elle a échoué avant de tourner');
    json = { testResults: [] };
  }

  const etat = new Map(); // titre → 'passed' | 'failed' | …
  for (const fichier of json.testResults ?? []) {
    for (const r of fichier.assertionResults ?? []) etat.set(r.title, r.status);
  }

  for (const id of portes) {
    const titre = cites.get(id);
    const s = etat.get(titre);
    if (s === undefined) erreurs.push(`${id} est porté mais son test n'a pas tourné`);
    else if (s !== 'passed') erreurs.push(`RÉGRESSION — ${id} est porté et son test est ${s} : ${titre}`);
  }
  for (const id of TOUS.filter((x) => aPorter.has(x))) {
    const s = etat.get(cites.get(id));
    if (s === 'passed') {
      erreurs.push(`${id} passe alors qu'il est marqué « à porter » : retirer son aPorter()`);
    }
  }
} finally {
  rmSync(bac, { recursive: true, force: true });
}

// --- verdict + tableau de bord ----------------------------------------------------------
if (erreurs.length) {
  for (const e of erreurs) console.log(`  ${e}`);
  console.log('check-comportements : ÉCHEC');
  process.exit(1);
}
console.log(
  `check-comportements : OK (${portes.length} portés sur ${TOUS.length}, ` +
    `${TOUS.length - portes.length} restent à porter)`,
);
if (portes.length) console.log(`  portés, donc dans le verdict : ${portes.join(', ')}`);
