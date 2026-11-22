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

// 2. Les comptes de piles d'invariants de CLAUDE.md ↔ le skill, source de vérité.
// (check-invariants.mjs vérifie déjà le tableau DU SKILL contre les invariants eux-mêmes ;
//  ici on ferme le maillon suivant, la copie lue à chaque session.)
const skill = lire('.claude/skills/hean-invariants/SKILL.md');
const piles = { rayé: 0, test: 0, irréductible: 0 };
for (const m of skill.matchAll(/^- pile\s*:\s*(\S+)\s*$/gm)) {
  if (m[1] in piles) piles[m[1]]++;
}
const total = Object.values(piles).reduce((a, b) => a + b, 0);

const claude = lire('CLAUDE.md');
const attendus = [
  [/\*\*(\d+) règles\*\*/, total, 'le total des invariants'],
  [/\*\*Rayées \((\d+)\)\*\*/, piles['rayé'], 'les invariants rayés'],
  [/\*\*Devenues des tests \((\d+)\)\*\*/, piles['test'], 'les invariants devenus des tests'],
  [/\*\*Irréductibles \((\d+)\s*\+/, piles['irréductible'], 'les invariants irréductibles'],
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
console.log(`check-comptes : OK (${verifications} vérifications, ${total} invariants)`);
