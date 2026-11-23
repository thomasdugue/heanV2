// Graphe d'imports : aucun cycle, et les zones ne se traversent pas.
//
// V1 : le médiateur `app.js` n'existait QUE pour briser des cycles. Sans cycle, il
// n'a plus de raison d'être. Et la « règle de zone » (ne jamais toucher l'audio quand on
// travaille sur l'UI) reposait sur la discipline : ici elle devient mécanique.
import { describe, expect, it } from 'vitest';
import { sources } from './_lecture';
import { franchissementsDeZone, imports, premierCycle, resoudre } from './regles-imports';

// Frontières de zone. Le frontend du player est de l'UI malgré son nom : la règle ne vise
// que les vrais chemins de chaque zone (mémoire de l'agent de relecture de V1, 2026-10-29).
const ZONES = [
  ['src/lib/audio', 'src/lib/ui'],
  ['src/lib/ui', 'src/lib/audio'],
] as const;

function graphe(): Map<string, string[]> {
  const g = new Map<string, string[]>();
  for (const [f, c] of sources('src', ['.ts', '.svelte'])) {
    g.set(
      f,
      imports(c)
        .map((s) => resoudre(f, s))
        .filter((x): x is string => x !== null),
    );
  }
  return g;
}

describe('graphe d’imports', () => {
  it('ne contient aucun cycle', () => {
    expect(premierCycle(graphe())).toBeNull();
  });

  it('le détecteur repère bien un cycle', () => {
    const g = new Map([
      ['a.ts', ['b.ts']],
      ['b.ts', ['c.ts']],
      ['c.ts', ['a.ts']],
    ]);
    expect(premierCycle(g)).toBe('a.ts → b.ts → c.ts → a.ts');
  });

  it('aucune zone n’en importe une autre', () => {
    const fautes = [...graphe().entries()].flatMap(([f]) =>
      franchissementsDeZone(f, imports(sources('src', ['.ts', '.svelte']).find(([x]) => x === f)?.[1] ?? ''), ZONES),
    );
    expect(fautes).toEqual([]);
  });

  it('le détecteur repère un franchissement de zone', () => {
    expect(
      franchissementsDeZone('src/lib/audio/moteur.ts', ['../ui/vue'], ZONES),
    ).toEqual(['src/lib/audio/moteur.ts → ../ui/vue']);
  });

  it('le graphe contient de vraies arêtes', () => {
    // Un graphe sans arête n'a jamais de cycle. `resoudre()` rend `null` pour tout import
    // qu'il n'arrive pas à faire correspondre à un fichier, et sa liste d'extensions
    // (`'', '.ts', '.svelte', '/index.ts'`) est écrite à la main : le jour où une convention
    // change — un `.svelte.ts`, un sous-dossier avec `index.svelte` — les imports concernés
    // se résolvent silencieusement en `null`. Le détecteur de cycles tourne alors sur un
    // nuage de nœuds isolés : vert à jamais, et pour rien. Son auto-test, lui, resterait
    // vert aussi, puisqu'il travaille sur un graphe écrit à la main.
    const aretes = [...graphe().values()].flat();
    expect(aretes.length, 'aucun import résolu : le détecteur de cycles ne voit rien').toBeGreaterThan(0);
  });
});
