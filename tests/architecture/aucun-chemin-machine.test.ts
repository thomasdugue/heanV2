// Aucune référence hors du dépôt (HARNAIS.md §9).
//
// V1 : le meilleur document design du projet vivait HORS du dépôt, et SAVOIR-FRONTEND.md
// décrivait des chemins de vignettes (`~/.noir_desktop/`) qui n'existaient plus. Une
// référence qu'on ne peut pas ouvrir depuis le dépôt est une référence qui se périme sans
// que personne le voie.
import { describe, expect, it } from 'vitest';
import { sources } from './_lecture';

// Chemins absolus de machine. Deux exceptions légitimes :
//   - `~/Library/Application Support/hean/`, le dossier de données de l'app (ADR 0003) ;
//   - les BLOCS DE LANCEMENT `cd <chemin absolu> && <commande>`, convention explicite de
//     Thomas : un protocole de test s'exécute à la main, on ne veut pas chercher la commande.
// Ce qui reste visé est la seule chose dangereuse : une RÉFÉRENCE à un fichier qu'on ne
// peut pas ouvrir depuis le dépôt.
const CHEMIN_MACHINE = /\/(?:Users|home)\/[a-z0-9._-]+\//i;
const BLOC_DE_LANCEMENT = /^[`$\s]*cd\s+["'`]?\//;

export function cheminsMachine(texte: string): string[] {
  return texte
    .split('\n')
    .filter((l) => CHEMIN_MACHINE.test(l) && !BLOC_DE_LANCEMENT.test(l))
    .map((l) => l.trim().slice(0, 120));
}

// `bootstrap/` n'est PAS dans les zones, et c'est un choix, pas un oubli : c'est l'archive
// figée de V1 (ADR 0004). Ses chemins absolus sont un relevé historique — les « corriger »
// falsifierait le document. Et `03-architecture/code-reference/rust/sentry_init.rs` y porte
// des chemins de machine comme **fixtures d'un anonymiseur de chemins** : un faux positif
// permanent, et la démonstration que toute règle de ce genre doit s'arrêter au code vivant.
// `src-tauri/tests` y est depuis le 2026-11-23 : c'était le même angle mort que celui de
// `check-sizes.sh` et d'INV-21. `tests/` (le TypeScript) n'y est PAS, et ce n'est pas un choix :
// élargir le ferait rougir sur les échantillons fautifs de ce fichier et de son voisin, qui
// demandent une exception par chemin exact (voir `AUTO_TEST` dans `une-seule-cle-dalbum`).
const ZONES = ['src', 'src-tauri/src', 'src-tauri/tests', 'docs', '.claude', 'tools'] as const;
const EXTS = ['.ts', '.svelte', '.css', '.rs', '.md', '.sh', '.mjs'] as const;

// ⚠️ MUET PAR CONSTRUCTION, et là aussi c'est voulu : la seule matière réelle que ce
// détecteur pourrait trouver dans les zones vivantes serait une faute. Zéro trouvaille est
// donc son état sain, et aucune assertion « il trouve au moins N choses réelles » n'a de
// sens ici — contrairement à `references-de-tests.test.ts`, dont la matière (les citations)
// est légitime et abondante.
//
// Ce qui le protège à la place :
//   - le lecteur partagé refuse un corpus vide, donc aucune des cinq zones ne peut
//     disparaître en silence — et c'est le vrai risque de ce test, qui en scanne cinq ;
//   - ses trois auto-tests éprouvent le détecteur ET ses deux exceptions, dans les deux
//     sens (il voit le cas de V1, il ne se déclenche pas sur le dossier de données ni sur un
//     bloc de lancement). La matière des exceptions, elle, est bien réelle : `bootstrap/`
//     en porte, et c'est pour ça qu'il est hors zones.
describe('aucune référence hors du dépôt', () => {
  it('aucun chemin absolu de machine dans le code, la doc ou les skills', () => {
    const fautes = ZONES.flatMap((z) =>
      sources(z, EXTS).flatMap(([f, c]) => cheminsMachine(c).map((l) => `${f} : ${l}`)),
    );
    expect(fautes, 'un chemin de machine ne se résout que chez une personne').toEqual([]);
  });

  it('le détecteur repère le cas de V1', () => {
    expect(cheminsMachine('voir /Users/thomas/dev/noirdesktop/design.html')).toEqual([
      'voir /Users/thomas/dev/noirdesktop/design.html',
    ]);
  });

  it('ne se déclenche pas sur le dossier de données de l’app', () => {
    expect(cheminsMachine('~/Library/Application Support/hean/')).toEqual([]);
  });

  it('tolère un bloc de lancement, qui est une convention voulue', () => {
    expect(cheminsMachine('cd /Users/x/Documents/heanV2 && ./check')).toEqual([]);
  });
});
