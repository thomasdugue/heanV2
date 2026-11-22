// APPARIEMENT (niveau 2) — le contrat de 24 lignes et les tests qui le couvrent.
//
// Ce test-ci, lui, DOIT être vert : il garantit qu'aucun comportement du contrat n'a été
// oublié, et qu'aucun test ne cite un comportement disparu. C'est ce qui empêche le tableau
// de bord de mentir.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { CONTRAT } from './_contrat';

const ICI = import.meta.dirname;

/**
 * Les comportements cités par un test, **qu'il soit porté ou non**.
 *
 * La citation est le TITRE du test, `it('V004 — …')`, et non l'appel à `aPorter` : porter
 * un comportement consiste précisément à remplacer cet appel par une vraie assertion, et
 * un appariement qui ne reconnaîtrait que les non-portés déclarerait « comportement
 * perdu » à chaque progrès du projet. Le titre, lui, reste.
 */
function idsCites(): Set<string> {
  const out = new Set<string>();
  for (const f of fichiersDeTest()) {
    for (const m of readFileSync(join(ICI, f), 'utf8').matchAll(/^\s*it\('([VW]\d{3})\b/gm)) {
      out.add(m[1] as string);
    }
  }
  return out;
}

function fichiersDeTest(): string[] {
  return readdirSync(ICI).filter((x) => x.endsWith('.test.ts') && x !== 'contrat-complet.test.ts');
}

/** Les comportements encore marqués « à porter ». Le complément, ce sont les portés. */
function idsAPorter(): Set<string> {
  const out = new Set<string>();
  for (const f of fichiersDeTest()) {
    for (const m of readFileSync(join(ICI, f), 'utf8').matchAll(/aPorter\('([VW]\d{3})'\)/g)) {
      out.add(m[1] as string);
    }
  }
  return out;
}

describe('contrat anti-régression', () => {
  it('compte bien 24 comportements', () => {
    expect(CONTRAT).toHaveLength(24);
  });

  it('chaque comportement du contrat a un test', () => {
    const cites = idsCites();
    expect(CONTRAT.filter((x) => !cites.has(x.id)).map((x) => x.id)).toEqual([]);
  });

  it('aucun test ne cite un comportement absent du contrat', () => {
    const connus = new Set(CONTRAT.map((x) => x.id));
    expect([...idsCites()].filter((id) => !connus.has(id))).toEqual([]);
    // Et un `aPorter('V0xx')` oublié sur un comportement disparu du contrat lèverait
    // au lancement du test ; ici on le refuse aussi à la lecture.
    expect([...idsAPorter()].filter((id) => !connus.has(id))).toEqual([]);
  });

  it('le tableau de bord de ./check ne peut pas mentir', () => {
    // `./check` déduit le nombre de comportements PORTÉS du nombre de tests rouges :
    // 24 moins les rouges. Ça ne tient que si chaque comportement a exactement un test et
    // si ce fichier-ci, l'appariement, est vert — c'est pourquoi il est aussi dans
    // `npm run test`, donc dans le verdict, et pas seulement dans le tableau de bord.
    const portes = CONTRAT.filter((x) => !idsAPorter().has(x.id)).map((x) => x.id);
    expect(idsCites().size).toBe(CONTRAT.length);
    expect(portes.length + idsAPorter().size).toBe(CONTRAT.length);
  });
});
