// Aucune couleur écrite en dur hors du fichier de tokens.
// V1 : styles.css faisait 10 654 lignes et DESIGN-SYSTEM.md divergeait du code sur
// 5 couleurs — personne ne pouvait dire laquelle faisait foi. Ici, une seule peut.
import { describe, expect, it } from 'vitest';
import { blocsStyle, sources } from './_lecture';
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
});
