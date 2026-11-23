// Tout chemin de test cité dans un commentaire doit exister.
//
// Règle née le 2026-11-22, dans la session qui a posé ce harnais : `secrets.rs` et
// `client.ts` — les deux fichiers les plus sensibles du socle — annonçaient être « vérifiés
// par » des fichiers de test renommés en cours de route. La règle était bien tenue, mais
// ailleurs. Quelqu'un venant vérifier aurait trouvé un fichier absent, conclu « pas de
// test », et aurait pu supprimer la règle. C'est exactement le mécanisme de dérive que ce
// harnais existe pour fermer — et rien ne l'attrapait.
//
// Relevé par l'agent invariant-guardian, en relecture du diff de la phase 0.
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { RACINE, sources } from './_lecture';

// `src-tauri/tests` y est depuis le 2026-11-23 (même angle mort que `check-sizes.sh`). `tests/`
// n'y est pas : ce fichier cite exprès des chemins morts pour éprouver son détecteur, et se
// signalerait lui-même — il faudrait une exception par chemin exact.
const ZONES = ['src', 'src-tauri/src', 'src-tauri/tests', 'tools', '.claude/skills', 'docs/adr'] as const;
const EXTS = ['.ts', '.svelte', '.css', '.rs', '.mjs', '.md'] as const;

/**
 * Les chemins de fichiers de test cités dans un texte. Le préfixe `src-tauri/` est capturé,
 * sinon `src-tauri/tests/export_bindings.rs` serait lu comme `tests/export_bindings.rs` et
 * déclaré absent : un détecteur qui lève un faux positif finit ignoré.
 */
export function cheminsDeTestsCites(texte: string): string[] {
  const re = /(?<![\w/-])(?:src-tauri\/)?tests\/[A-Za-z0-9_./-]*\.(?:test\.)?(?:ts|rs)\b/g;
  return [...texte.matchAll(re)].map((m) => m[0]);
}

describe('références de tests', () => {
  it('tout chemin tests/… cité quelque part existe vraiment', () => {
    const morts = ZONES.flatMap((z) =>
      sources(z, EXTS).flatMap(([f, c]) =>
        cheminsDeTestsCites(c)
          .filter((p) => !existsSync(join(RACINE, p)))
          .map((p) => `${f} cite ${p}, qui n'existe pas`),
      ),
    );
    expect(morts, 'une référence morte fait conclure « pas de test » à tort').toEqual([]);
  });

  it('le détecteur repère le cas rencontré en phase 0', () => {
    expect(cheminsDeTestsCites('// Vérifié par tests/architecture/ipc-renomme.test.ts.')).toEqual([
      'tests/architecture/ipc-renomme.test.ts',
    ]);
  });

  it('ne tronque pas un chemin préfixé', () => {
    expect(cheminsDeTestsCites('voir src-tauri/tests/export_bindings.rs')).toEqual([
      'src-tauri/tests/export_bindings.rs',
    ]);
  });

  it('le détecteur trouve de vraies citations dans le dépôt', () => {
    // Cette règle est une interdiction : elle est verte quand elle ne trouve AUCUNE
    // référence morte. Elle est donc aussi verte quand elle ne trouve aucune référence du
    // tout — et son auto-test, qui travaille sur une phrase écrite ici, ne ferait pas la
    // différence. Or le dépôt en porte une quinzaine, et c'est précisément ce que la règle
    // existe pour surveiller : si l'expression cessait de les reconnaître (un chemin
    // désormais écrit entre backticks, un dossier `tests/` déplacé), le garde-fou
    // disparaîtrait sans un seul test rouge.
    const cites = ZONES.flatMap((z) => sources(z, EXTS).flatMap(([, c]) => cheminsDeTestsCites(c)));
    expect(cites.length, 'plus aucune citation reconnue : la règle ne surveille plus rien')
      .toBeGreaterThan(5);
  });
});
