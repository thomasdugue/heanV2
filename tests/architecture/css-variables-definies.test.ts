// INV-20 — toute variable `var(--x)` utilisée doit être définie.
//
// C'est le test qui rend le plus de toute la liste. Une variable CSS non définie rend la
// déclaration invalide « au moment du calcul » : le navigateur l'ignore SANS ERREUR, même
// avec !important. Ni la relecture, ni les tests de comportement, ni le type-checking ne
// l'attrapent. En V1, --glass-lo / --glass-md / --glass-hi ont été utilisées 45 fois sans
// définition sur toute une PR de refonte : bordures, survols et état actif de la sidebar
// invisibles, découverts quatre mois plus tard.
import { describe, expect, it } from 'vitest';
import { blocsStyle, sources } from './_lecture';
import { variablesDeclarees, variablesLuesSansRepli } from './regles-css';

/** Tout le CSS du projet : feuilles autonomes + blocs `<style>` des composants Svelte. */
function toutLeCss(): [string, string][] {
  return [
    ...sources('src', ['.css']),
    ...sources('src', ['.svelte']).map(([f, c]) => [f, blocsStyle(c)] as [string, string]),
  ];
}

describe('INV-20 — variables CSS définies', () => {
  it('toute var(--x) sans repli est déclarée quelque part', () => {
    const css = toutLeCss();
    const declarees = new Set(css.flatMap(([, c]) => [...variablesDeclarees(c)]));

    const manquantes = css.flatMap(([f, c]) =>
      [...variablesLuesSansRepli(c)].filter((v) => !declarees.has(v)).map((v) => `${f} : ${v}`),
    );

    expect(manquantes, 'variables lues mais jamais définies (invisibles en silence)').toEqual([]);
  });

  // Auto-test du détecteur : sans lui, ce fichier passerait au vert sur un dossier vide
  // sans rien garantir du tout.
  it('le détecteur repère bien le cas de V1', () => {
    const fautif = ':root { --color-bg: #000; } .side { border: 1px solid var(--glass-lo); }';
    const declarees = variablesDeclarees(fautif);
    const lues = variablesLuesSansRepli(fautif);
    expect([...lues].filter((v) => !declarees.has(v))).toEqual(['--glass-lo']);
  });

  it('une valeur de repli est tolérée', () => {
    expect([...variablesLuesSansRepli('.a { width: var(--inconnue, 100px); }')]).toEqual([]);
  });

  it('les deux moitiés du détecteur voient la matière réelle du projet', () => {
    // Le test ci-dessus compare deux ensembles. Il est vert si les deux sont pleins et
    // cohérents — mais AUSSI si l'un des deux est vide, et là il ne garantit plus rien.
    // Les deux moitiés tombent pour des raisons différentes, d'où deux assertions :
    //
    //  - les DÉCLARATIONS viennent presque toutes de `tokens.css`, qui est GÉNÉRÉ. Si son
    //    format changeait, cet ensemble se viderait — et le test rougirait bruyamment,
    //    puisque toute lecture deviendrait « non définie ». Ce côté-là échoue bien.
    //  - les LECTURES, elles, sont le côté dangereux : si `variablesLuesSansRepli` cessait
    //    de reconnaître `var(--x)`, l'ensemble des lectures serait vide, aucune ne pourrait
    //    manquer, et INV-20 — « la règle qui rend le plus de toute la liste » — passerait au
    //    vert sans surveiller quoi que ce soit. En silence.
    const css = toutLeCss();
    const declarees = new Set(css.flatMap(([, c]) => [...variablesDeclarees(c)]));
    const lues = new Set(css.flatMap(([, c]) => [...variablesLuesSansRepli(c)]));

    expect(declarees.size, 'le format de tokens.css a pu changer sous nos pieds').toBeGreaterThan(20);
    // Volontairement bas : il n'y a que cinq lectures sans repli dans tout le projet en
    // phase 2, et le seuil doit mesurer « le détecteur voit » et non « le projet est gros ».
    expect(lues.size, 'plus aucune var(--x) reconnue : INV-20 ne surveille plus rien').toBeGreaterThan(0);
  });
});
