// Plafonds d'écriture — ADR 0017.
//
// Le harnais mesurait la taille du CODE (400 lignes par fichier) et jamais celle de ce qu'on
// écrit AUTOUR. Résultat au 2026-11-25 : 1 390 lignes de journal et 16 ADR en cinq jours, pour
// 5 530 lignes d'application. C'est le défaut de V1 que ce projet cite pour justifier sa
// réécriture, reparu dans sa documentation.
//
// Les entrées et les ADR écrits AVANT ce plafond ne sont pas recadrés : ce sont des relevés,
// et les réécrire falsifierait l'historique. Le plafond vaut pour ce qui s'écrit à partir de
// maintenant — d'où le seuil de date, qui est la seule façon honnête de poser une règle
// nouvelle sur un journal en ajout seul.
import { readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';

const RACINE = resolve(import.meta.dirname, '..');
const DEPUIS = '2026-11-26'; // les entrées antérieures sont des relevés, pas des manquements
const PLAFOND_ENTREE = 15;
const PLAFOND_ADR = 120; // « une page »
const PREMIER_ADR_PLAFONNE = 17;

const erreurs = [];

// --- docs/HISTORY.md : une entrée = 15 lignes ---------------------------------------------
const lignes = readFileSync(join(RACINE, 'docs/HISTORY.md'), 'utf8').split('\n');
const debuts = [];
lignes.forEach((l, i) => {
  const d = /^- \*\*(\d{4}-\d{2}-\d{2})\*\*/.exec(l);
  if (d) debuts.push({ i, date: d[1] });
});
debuts.forEach(({ i, date }, k) => {
  if (date < DEPUIS) return;
  const fin = k + 1 < debuts.length ? debuts[k + 1].i : lignes.length;
  let n = fin - i;
  while (n > 0 && lignes[i + n - 1].trim() === '') n--; // les blancs de séparation ne comptent pas
  if (n > PLAFOND_ENTREE) {
    erreurs.push(
      `docs/HISTORY.md ligne ${i + 1} : l'entrée du ${date} fait ${n} lignes (> ${PLAFOND_ENTREE}). ` +
        `Ce qui déborde est soit une décision (un ADR), soit du savoir de zone (un skill), soit superflu.`,
    );
  }
});

// --- docs/adr/NNNN-*.md : une page ---------------------------------------------------------
for (const f of readdirSync(join(RACINE, 'docs/adr')).sort()) {
  const num = /^(\d{4})-/.exec(f);
  if (!num || Number(num[1]) < PREMIER_ADR_PLAFONNE) continue;
  const n = readFileSync(join(RACINE, 'docs/adr', f), 'utf8').trimEnd().split('\n').length;
  if (n > PLAFOND_ADR) {
    erreurs.push(`docs/adr/${f} : ${n} lignes (> ${PLAFOND_ADR}). Un ADR tient sur une page.`);
  }
}

if (erreurs.length) {
  for (const e of erreurs) console.log(`  ${e}`);
  console.log('check-plafonds : ÉCHEC');
  process.exit(1);
}
console.log(`check-plafonds : OK (entrées ≤ ${PLAFOND_ENTREE} lignes, ADR ≤ ${PLAFOND_ADR})`);
