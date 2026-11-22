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
});
