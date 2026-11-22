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
const ZONES = ['src', 'src-tauri/src', 'docs', '.claude', 'tools'] as const;
const EXTS = ['.ts', '.svelte', '.css', '.rs', '.md', '.sh', '.mjs'] as const;

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
