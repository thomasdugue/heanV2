// Aucune couleur écrite en dur hors du fichier de tokens.
// V1 : styles.css faisait 10 654 lignes et DESIGN-SYSTEM.md divergeait du code sur
// 5 couleurs — personne ne pouvait dire laquelle faisait foi. Ici, une seule peut.
import { describe, expect, it } from 'vitest';
import { blocsStyle, lire, sources } from './_lecture';
import { couleursEnDur } from './regles-css';

const FICHIER_DE_TOKENS = 'src/styles/tokens.css';

describe('couleurs en dur', () => {
  it("n'apparaissent que dans le fichier de tokens", () => {
    const css: [string, string][] = [
      ...sources('src', ['.css']).filter(([f]) => f !== FICHIER_DE_TOKENS),
      ...sources('src', ['.svelte']).map(([f, c]) => [f, blocsStyle(c)] as [string, string]),
    ];

    const fautes = css.flatMap(([f, c]) => couleursEnDur(c).map((v) => `${f} : ${v}`));
    expect(fautes, `couleurs hors de ${FICHIER_DE_TOKENS}`).toEqual([]);
  });

  it('le détecteur repère un hexadécimal et un rgba() littéral', () => {
    expect(couleursEnDur('.a { color: #ff4444; background: rgba(0,0,0,.5); }')).toEqual([
      '#ff4444',
      'rgba(0,0,0,.5)',
    ]);
  });

  it('tolère une composition à partir de tokens', () => {
    expect(couleursEnDur('.a { color: rgb(var(--color-ambient-r), 0, 0); }')).toEqual([]);
  });

  it('le détecteur reconnaît les deux notations du vrai fichier de tokens', () => {
    // L'auto-test ci-dessus prouve que le détecteur voit la faute qu'on lui a INVENTÉE.
    // Celui-ci prouve qu'il voit le vrai format du projet — et les deux ne se remplacent
    // pas. `tokens.css` est GÉNÉRÉ (npm run gen) : son format peut changer sans que
    // personne décide de le changer. S'il passait un jour à une notation que `couleursEnDur`
    // ne connaît pas, l'interdiction au-dessus deviendrait vraie partout et muette partout.
    // Le seul fichier du dépôt qui a le DROIT de contenir des couleurs est donc aussi le
    // seul endroit où l'on peut vérifier qu'on sait encore les voir.
    //
    // UNE ASSERTION PAR FAMILLE, et c'est une correction de relecture. Un seuil global
    // (`> 10` sur les 33 couleurs du fichier) était vert si UNE SEULE famille restait
    // reconnue : casser la détection des hexadécimaux laissait les 11 `rgba()` passer le
    // seuil, et inversement. L'assertion mesurait « au moins une notation est encore vue »
    // là où son commentaire promettait « le détecteur est entier ». Par famille, il n'y a
    // plus de seuil arbitraire à choisir, donc plus de marge à se tromper — et une baisse
    // légitime du nombre de `rgba()` ne fait plus rougir pour une non-raison.
    const trouvees = couleursEnDur(lire(FICHIER_DE_TOKENS));
    const hexadecimaux = trouvees.filter((v) => v.startsWith('#'));
    const fonctions = trouvees.filter((v) => /^[a-z-]+\(/i.test(v));

    expect(hexadecimaux.length, `plus un seul #rrggbb vu dans ${FICHIER_DE_TOKENS}`).toBeGreaterThan(0);
    expect(fonctions.length, `plus un seul rgba()/hsl() vu dans ${FICHIER_DE_TOKENS}`).toBeGreaterThan(0);
  });
});
