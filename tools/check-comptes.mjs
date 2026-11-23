// Les comptes annoncés en prose doivent correspondre à la réalité.
//
// Née le 2026-11-22, relevée par la session de phase 1 : le README annonçait « 29 cas » de
// tests d'architecture alors qu'il y en avait 32. Le chiffre était vrai quand il a été
// écrit, et faux un commit plus tard. C'est la dérive de documentation du postmortem — et
// `check-links` validait les liens du README, pas ses affirmations.
//
// Règle tirée de l'ADR 0005 : un nombre écrit à la main que personne ne recompte est un
// mensonge en attente. Soit une machine le vérifie, soit il ne s'écrit pas.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const RACINE = resolve(import.meta.dirname, '..');
const lire = (p) => readFileSync(resolve(RACINE, p), 'utf8');
const erreurs = [];

// 1. « N vérifications » ↔ le nombre d'appels `run` dans ./check.
// Les appels `run "…"` — certains sont indentés dans un `if`, d'où l'absence d'ancre.
// La définition de la fonction s'écrit `run() {`, elle n'est donc pas comptée.
const verifications = (lire('check').match(/\brun "/g) ?? []).length;
for (const fichier of ['README.md', 'CLAUDE.md']) {
  const texte = lire(fichier);
  for (const m of texte.matchAll(/(\d+)\s+vérifications/g)) {
    if (Number(m[1]) !== verifications) {
      erreurs.push(`${fichier} annonce « ${m[1]} vérifications », ./check en lance ${verifications}`);
    }
  }
}

// 1b. Le compte des comportements n'est plus écrit NULLE PART à la main : `./check` délègue
// à `check-comportements.mjs`, qui le déduit du contrat. Ce qui reste à vérifier est donc
// que la délégation existe — un `./check` qui recommencerait à compter lui-même rouvrirait
// la porte. (Il a d'abord retranché « 3 » aux tests verts, puis annoncé « sur 24 » : deux
// nombres écrits à la main, les deux devenus faux.)
const comportements = (lire('tests/ux/_contrat.ts').match(/^\s*c\('[VW]\d{3}'/gm) ?? []).length;
const verif = lire('check');
if (!verif.includes('tools/check-comportements.mjs')) {
  erreurs.push('check ne lance pas tools/check-comportements.mjs : les comportements portés sortiraient du verdict');
}
if (/portés \(sur \d+\)/.test(verif)) {
  erreurs.push('check compte les comportements lui-même : ce nombre se déduit du contrat, il ne s’écrit pas');
}

// 2. Les comptes de piles d'invariants de CLAUDE.md ↔ le skill, source de vérité.
// (check-invariants.mjs vérifie déjà le tableau DU SKILL contre les invariants eux-mêmes ;
//  ici on ferme le maillon suivant, la copie lue à chaque session.)
const skill = lire('.claude/skills/hean-invariants/SKILL.md');
const piles = { rayé: 0, test: 0, irréductible: 0 };
for (const m of skill.matchAll(/^- pile\s*:\s*(\S+)\s*$/gm)) {
  if (m[1] in piles) piles[m[1]]++;
}
const total = Object.values(piles).reduce((a, b) => a + b, 0);
// Les « planifiées » : la dette assumée. C'est le nombre le plus vivant des cinq — il bouge
// à chaque règle branchée sur un test — donc le plus exposé à devenir faux en silence.
const planifiees = (skill.match(/^- couverture\s*:\s*planifiée/gm) ?? []).length;

const claude = lire('CLAUDE.md');
// Les invariants DÉJÀ actifs : couverture « test X » ou « architecture X ». C'est le
// pendant des « planifiées », et il bouge au même rythme — chaque règle branchée sur un
// test fait monter l'un et baisser l'autre. Écrit à la main dans CLAUDE.md, ce nombre
// serait faux au premier branchement ; il l'était d'ailleurs déjà à la phase 2.
const actives = (skill.match(/^- couverture\s*:\s*(test|architecture) /gm) ?? []).length;

// 2b. Le nombre de règles annoncé par le README. Il annonçait « les 21 règles » pour 22 —
// faux depuis la naissance d'INV-22, et invisible parce que seul CLAUDE.md était vérifié.
// Un garde-fou qui ne couvre qu'un des deux fichiers où le nombre est écrit ne couvre pas
// la règle, seulement l'endroit où quelqu'un y a pensé.
const regles = /`hean-invariants` \(les (\d+) règles\)/.exec(lire('README.md'))?.[1];
if (regles === undefined) {
  erreurs.push("README.md : impossible d'y lire le nombre de règles — le format a changé");
} else if (Number(regles) !== total) {
  erreurs.push(`README.md annonce « les ${regles} règles », il y en a ${total}`);
}

const attendus = [
  [/\*\*(\d+) règles\*\*/, total, 'le total des invariants'],
  [/dont \*\*(\d+) déjà actives\*\*/, actives, 'les invariants déjà couverts par un test'],
  [/\*\*Rayées \((\d+)\)\*\*/, piles['rayé'], 'les invariants rayés'],
  [/\*\*Devenues des tests \((\d+)\)\*\*/, piles['test'], 'les invariants devenus des tests'],
  [/\*\*Irréductibles \((\d+)\s*\+/, piles['irréductible'], 'les invariants irréductibles'],
  [/\*\*(\d+) « planifiées »\*\*/, planifiees, 'les couvertures planifiées'],
];
for (const [re, reel, quoi] of attendus) {
  const m = re.exec(claude);
  if (!m) erreurs.push(`CLAUDE.md : impossible d'y lire ${quoi} — le format a changé`);
  else if (Number(m[1]) !== reel) erreurs.push(`CLAUDE.md annonce ${m[1]} pour ${quoi}, il y en a ${reel}`);
}

if (erreurs.length) {
  for (const e of erreurs) console.log(`  ${e}`);
  console.log('check-comptes : ÉCHEC');
  process.exit(1);
}
console.log(
  `check-comptes : OK (${verifications} vérifications, ${total} invariants dont ${actives} actifs et ` +
    `${planifiees} planifiés, ` +
    `${comportements} comportements)`,
);
