// Détecteurs sur le graphe d'imports du frontend.
import { existsSync } from 'node:fs';
import { dirname, join, normalize } from 'node:path';
import { RACINE } from './_lecture';

/** Les spécificateurs importés par un fichier TS ou Svelte. */
export function imports(code: string): string[] {
  const out: string[] = [];
  for (const m of code.matchAll(/(?:^|\n)\s*(?:import|export)[\s\S]{0,400}?from\s*['"]([^'"]+)['"]/g)) {
    out.push(m[1] as string);
  }
  for (const m of code.matchAll(/\bimport\(\s*['"]([^'"]+)['"]\s*\)/g)) out.push(m[1] as string);
  return out;
}

const EXTENSIONS = ['', '.ts', '.svelte', '/index.ts'];

/** Résout un import relatif en chemin de fichier relatif à la racine, ou `null`. */
export function resoudre(depuis: string, spec: string): string | null {
  if (!spec.startsWith('.')) return null;
  const base = normalize(join(dirname(depuis), spec));
  for (const ext of EXTENSIONS) {
    if (existsSync(join(RACINE, base + ext))) return base + ext;
  }
  return null;
}

/** Le premier cycle trouvé dans le graphe, sous forme de chemin lisible, ou `null`. */
export function premierCycle(graphe: Map<string, string[]>): string | null {
  const ETAT = new Map<string, 'en-cours' | 'fini'>();
  const pile: string[] = [];

  const visiter = (n: string): string | null => {
    if (ETAT.get(n) === 'fini') return null;
    if (ETAT.get(n) === 'en-cours') return [...pile.slice(pile.indexOf(n)), n].join(' → ');
    ETAT.set(n, 'en-cours');
    pile.push(n);
    for (const v of graphe.get(n) ?? []) {
      const c = visiter(v);
      if (c) return c;
    }
    pile.pop();
    ETAT.set(n, 'fini');
    return null;
  };

  for (const n of graphe.keys()) {
    const c = visiter(n);
    if (c) return c;
  }
  return null;
}

/** Les imports d'un fichier qui franchissent une frontière de zone interdite. */
export function franchissementsDeZone(
  fichier: string,
  specs: string[],
  zones: readonly (readonly [string, string])[],
): string[] {
  const out: string[] = [];
  for (const [source, interdite] of zones) {
    if (!fichier.startsWith(source)) continue;
    for (const s of specs) {
      const cible = s.startsWith('.') ? normalize(join(dirname(fichier), s)) : s;
      if (cible.startsWith(interdite)) out.push(`${fichier} → ${s}`);
    }
  }
  return out;
}
