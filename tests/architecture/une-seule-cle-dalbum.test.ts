// INV-01 — « un album = une seule entrée ». Côté frontend, la règle devient : le frontend
// ne construit JAMAIS de clé d'album.
//
// La clé est produite par `AlbumKey::new` en Rust (trim + NFC) et voyage dans le champ
// `cle_album` de chaque piste. Le frontend regroupe dessus et c'est tout.
//
// Pourquoi un test de forme en plus du type Rust : le type rend impossible une clé mal
// normalisée **en Rust**. Rien n'empêcherait une vue, un jour, de refaire
// `album.trim().normalize('NFC')` dans son coin « pour comparer deux titres » — et c'est
// exactement ce qui est arrivé en V1, où `normalizeKey()` existait à plusieurs endroits
// avec deux formats de clé qui coexistaient. Une deuxième implémentation est une deuxième
// vérité ; celle-ci se refuse mécaniquement.
//
// `tests/` est scanné en plus de `src/` : un faux backend de test qui normaliserait serait
// une deuxième implémentation lui aussi, arrivée par la porte du harnais.
import { describe, expect, it } from 'vitest';
import { sansCommentaires, sources } from './_lecture';

/** Les normalisations Unicode faites à la main. Le seul endroit légitime est le Rust. */
export function normalisationsUnicode(code: string): string[] {
  return sansCommentaires(code)
    .split('\n')
    .filter((l) => /\.normalize\s*\(/.test(l) || /\bNFC\b|\bNFD\b/.test(l))
    .map((l) => l.trim().slice(0, 100));
}

const ZONES = [
  ['src', ['.ts', '.svelte']],
  ['tests', ['.ts']],
] as const;

// Ce fichier-ci est la SEULE exception, et elle est nécessaire : il contient l'échantillon
// fautif qui prouve que le détecteur voit quelque chose. Sans auto-test, un détecteur passe
// au vert sans rien garantir (skill hean-invariants) ; sans cette exception, il se
// signalerait lui-même. L'exception porte sur un chemin exact, pas sur un motif : élargir
// la dérogation est le début de la fin d'une règle.
const AUTO_TEST = 'tests/architecture/une-seule-cle-dalbum.test.ts';

describe('INV-01 — une seule construction de clé d’album', () => {
  it('rien, hors du Rust, ne normalise une chaîne', () => {
    const fautes = ZONES.flatMap(([zone, exts]) =>
      sources(zone, exts)
        .filter(([f]) => f !== AUTO_TEST)
        .flatMap(([f, c]) => normalisationsUnicode(c).map((l) => `${f} : ${l}`)),
    );
    expect(
      fautes,
      'la clé d’album vient de AlbumKey::new (src-tauri/src/library/cle.rs) et de nulle part ailleurs',
    ).toEqual([]);
  });

  it('le détecteur repère la faute de V1', () => {
    expect(normalisationsUnicode("const cle = album.trim().normalize('NFC');")).toEqual([
      "const cle = album.trim().normalize('NFC');",
    ]);
  });

  it('il ne se déclenche pas sur un commentaire qui en parle', () => {
    expect(normalisationsUnicode("// la clé est normalisée en NFC côté Rust\nconst x = 1;")).toEqual([]);
  });

  it('l’exception ne couvre que l’auto-test, et il existe', () => {
    const fichiers = sources('tests/architecture', ['.ts']).map(([f]) => f);
    expect(fichiers).toContain(AUTO_TEST);
  });

  it('le frontend lit bien la clé que le backend lui donne', () => {
    // Un détecteur qui ne trouve rien passe au vert sans rien garantir. L'autre moitié de
    // la règle est positive : le regroupement doit s'appuyer sur `cle_album`.
    const groupement = sources('src/lib/library', ['.ts']).find(([f]) => f.endsWith('groupement.ts'));
    expect(groupement?.[1]).toContain('cle_album');
  });
});
