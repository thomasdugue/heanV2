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

function idsCites(): Set<string> {
  const out = new Set<string>();
  for (const f of readdirSync(ICI).filter((x) => x.endsWith('.test.ts') && x !== 'contrat-complet.test.ts')) {
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
  });
});
