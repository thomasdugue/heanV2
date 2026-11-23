// ADR 0002 — Svelte 5, runes UNIQUEMENT.
//
// Une grande partie des exemples en ligne est encore en Svelte 4. Mélanger les deux
// syntaxes produit un état partagé par référence, exactement le problème de V1
// (V013 : `library = {}` cassait la référence vue par les autres modules).
import { describe, expect, it } from 'vitest';
import { sources } from './_lecture';
import { imports } from './regles-imports';

/** Les traces de Svelte 4 : les stores, et les déclarations réactives `$:`. */
export function tracesDeSvelte4(code: string): string[] {
  const out: string[] = [];
  if (imports(code).some((s) => s === 'svelte/store')) out.push("import 'svelte/store'");
  const script = /<script[^>]*>([\s\S]*?)<\/script>/.exec(code)?.[1] ?? code;
  for (const m of script.matchAll(/^[^\S\n]*\$:[^\n]*/gm)) out.push(`déclaration réactive ${m[0].trim()}`);
  return out;
}

// ⚠️ CE DÉTECTEUR EST MUET AUJOURD'HUI, ET C'EST NORMAL. Il n'a aucune matière réelle à
// reconnaître : la phase 2 ne compte qu'UN composant Svelte (`App.svelte`), et pas une seule
// trace de Svelte 4 — ce qui est le but. On ne peut donc pas lui demander de « trouver au
// moins N choses réelles » sans inventer la faute qu'il cherche.
//
// La phase 3 le rendra bavard : c'est elle qui apporte les vues, donc les dizaines de
// composants où un `$:` copié d'un exemple en ligne peut se glisser. D'ici là :
//   - son corpus est garanti non vide par le lecteur partagé (`_lecture.ts`), donc il ne
//     peut pas devenir vert en ne regardant plus rien ;
//   - sa moitié « stores » repose sur `imports()`, que `ipc.test.ts` éprouve sur du vrai
//     code ;
//   - sa moitié « $: » n'est éprouvée que par l'auto-test ci-dessous, et c'est la limite
//     assumée, écrite ici.
describe('Svelte 5 — runes uniquement', () => {
  it('aucun store Svelte 4, aucune déclaration $:', () => {
    const fautes = sources('src', ['.ts', '.svelte']).flatMap(([f, c]) =>
      tracesDeSvelte4(c).map((t) => `${f} : ${t}`),
    );
    expect(fautes, 'utiliser $state / $derived / $effect').toEqual([]);
  });

  it('le détecteur repère les deux formes', () => {
    expect(tracesDeSvelte4("import { writable } from 'svelte/store';")).toEqual(["import 'svelte/store'"]);
    expect(tracesDeSvelte4('<script>\n$: double = n * 2;\n</script>')).toEqual([
      'déclaration réactive $: double = n * 2;',
    ]);
  });
});
