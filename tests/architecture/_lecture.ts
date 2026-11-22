// Outils de lecture partagés par les tests d'architecture.
// Ces tests portent sur la FORME du code, pas sur son comportement : ils sont le seul
// mécanisme qui contraint le code qui sera écrit demain (HARNAIS.md §2).
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

export const RACINE = resolve(import.meta.dirname, '../..');

const IGNORES = new Set(['node_modules', 'target', 'dist', '.git', 'icons']);

/**
 * Un dossier qui contient une entrée `.git` est un dépôt à part : un clone imbriqué, ou un
 * worktree de session sous `.claude/worktrees/`. Ses fichiers ne sont PAS ceux de ce
 * dépôt-ci — ils viennent d'une autre branche, parfois d'un autre état du projet.
 *
 * Sans cette garde, `./check` était **rouge en local et vert en CI** : la CI part d'un clone
 * neuf sans worktree, alors que la machine de Thomas en porte un par session. Le même code
 * n'était donc pas vérifié sur le même ensemble de fichiers des deux côtés. C'est la même
 * famille de défaut que la chaîne d'outils non épinglée (ADR 0008), vue de l'autre bout.
 */
function estUnDepotSepare(chemin: string): boolean {
  return existsSync(join(chemin, '.git'));
}

/** Tous les fichiers de `dossier` dont l'extension est dans `exts`, chemins relatifs à `racine`. */
export function fichiers(dossier: string, exts: readonly string[], racine: string = RACINE): string[] {
  const depart = join(racine, dossier);
  const out: string[] = [];
  const descendre = (d: string): void => {
    let entrees: string[];
    try {
      entrees = readdirSync(d);
    } catch {
      return; // dossier pas encore créé : normal tant que la phase n'est pas arrivée
    }
    for (const e of entrees) {
      if (IGNORES.has(e)) continue;
      const p = join(d, e);
      if (statSync(p).isDirectory()) {
        if (!estUnDepotSepare(p)) descendre(p);
      } else if (exts.some((x) => e.endsWith(x))) out.push(relative(racine, p));
    }
  };
  descendre(depart);
  return out.sort();
}

/** Le contenu d'un fichier, chemin relatif à la racine. */
export function lire(chemin: string): string {
  return readFileSync(join(RACINE, chemin), 'utf8');
}

/** Les fichiers de `dossier` avec leur contenu. */
export function sources(dossier: string, exts: readonly string[]): [string, string][] {
  return fichiers(dossier, exts).map((f) => [f, lire(f)]);
}

/** Le contenu des blocs `<style>` d'un composant Svelte. */
export function blocsStyle(svelte: string): string {
  return [...svelte.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1] ?? '').join('\n');
}

/** Retire commentaires de bloc et de ligne, pour ne pas lever de faux positifs. */
export function sansCommentaires(code: string): string {
  return code.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '');
}
