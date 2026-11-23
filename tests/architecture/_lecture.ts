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

/**
 * Tous les fichiers de `dossier` dont l'extension est dans `exts`, chemins relatifs à `racine`.
 *
 * **Un corpus vide lève une erreur.** C'est la première des deux questions qu'un détecteur
 * doit savoir répondre par une assertion : *est-ce que je regarde quelque chose ?* Un
 * détecteur pointé sur un dossier renommé, déplacé ou disparu ne trouve aucune faute et
 * passe au vert — il produit de la confiance sans en mériter, et aucun test ne rougit pour
 * le dire. La garde vit ici, dans le lecteur partagé, et non dans chaque test : ainsi un
 * détecteur écrit demain en hérite sans que personne ait à y penser (ADR 0005 — par
 * mécanisme, jamais par discipline).
 *
 * Ce code tolérait l'inverse jusqu'au 2026-11-23 : un dossier absent rendait `[]` en
 * silence, « normal tant que la phase n'est pas arrivée ». L'intention était juste, mais
 * elle couvrait les deux cas d'un coup — la zone pas encore née ET la zone qu'on a perdue
 * de vue. `videPrevu` les sépare : le premier cas s'écrit, le second échoue.
 *
 * Les options sont un OBJET et non des arguments positionnels, et c'est une correction de
 * relecture : avec `fichiers(d, exts, racine, videPrevu)` d'un côté et `sources(d, exts,
 * videPrevu)` de l'autre, écrire `sources(d, exts, RACINE)` par mimétisme — la faute la plus
 * naturelle du monde — posait `videPrevu = RACINE`, donc **désarmait la garde en silence**,
 * sans que TypeScript voie quoi que ce soit (les deux sont des `string`). Une garde qui tient
 * tous les détecteurs d'un coup ne doit pas pouvoir tomber sur une faute de frappe.
 *
 * @param options.videPrevu si le corpus a de bonnes raisons d'être vide aujourd'hui, la
 *   raison et la phase qui le remplira. Une raison trop courte est refusée : une dérogation
 *   sans justification écrite est le « silence non écrit » que cette garde existe pour
 *   fermer. Un silence expliqué n'est pas un angle mort ; un silence non écrit en est un.
 */
export function fichiers(
  dossier: string,
  exts: readonly string[],
  options: { racine?: string; videPrevu?: string } = {},
): string[] {
  const { racine = RACINE, videPrevu } = options;
  if (videPrevu !== undefined && videPrevu.trim().length < 15) {
    throw new Error(
      `corpus vide annoncé sans raison écrite : ${dossier} [${exts.join(' ')}] — dire pourquoi ` +
        `ce corpus est vide et quelle phase le remplira.`,
    );
  }
  const depart = join(racine, dossier);
  const out: string[] = [];
  const descendre = (d: string): void => {
    let entrees: string[];
    try {
      entrees = readdirSync(d);
    } catch {
      return; // dossier absent : le vide se juge en sortie de parcours, pas ici
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
  if (out.length === 0 && videPrevu === undefined) {
    throw new Error(
      `corpus vide : ${dossier} [${exts.join(' ')}] — un détecteur qui ne regarde rien passe ` +
        `au vert sans rien garantir. Si ce vide est normal, passer { videPrevu: '<pourquoi, ` +
        `et quelle phase le remplira>' }.`,
    );
  }
  return out.sort();
}

/** Le contenu d'un fichier, chemin relatif à la racine. */
export function lire(chemin: string): string {
  return readFileSync(join(RACINE, chemin), 'utf8');
}

/** Les fichiers de `dossier` avec leur contenu. Corpus vide : voir `fichiers`. */
export function sources(
  dossier: string,
  exts: readonly string[],
  options: { videPrevu?: string } = {},
): [string, string][] {
  return fichiers(dossier, exts, options).map((f) => [f, lire(f)]);
}

/** Le contenu des blocs `<style>` d'un composant Svelte. */
export function blocsStyle(svelte: string): string {
  return [...svelte.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1] ?? '').join('\n');
}

/** Retire commentaires de bloc et de ligne, pour ne pas lever de faux positifs. */
export function sansCommentaires(code: string): string {
  return code.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '');
}
