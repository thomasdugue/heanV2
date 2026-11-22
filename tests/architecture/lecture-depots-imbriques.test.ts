// Le parcours de fichiers ne descend jamais dans un dépôt imbriqué.
//
// Règle née le 2026-11-22. `./check` était **rouge en local et vert en CI** — l'image
// inverse du défaut de chaîne d'outils trouvé le même jour, et la même famille : la
// vérification ne portait pas sur le même ensemble de fichiers des deux côtés.
//
// Cause : `.claude/worktrees/` contient un **dépôt complet par session ouverte**. La CI part
// d'un clone neuf et n'en a aucun ; la machine de Thomas en porte autant que de sessions. Les
// tests d'architecture scannaient donc, chez lui, le code d'autres branches — y compris
// `bootstrap/`, qui n'est volontairement dans aucune zone, et y compris eux-mêmes.
//
// Ce test-ci garde la garde : sans lui, la correction se perdrait au premier refactoring.
//
// ⚠️ ET LA CI NE PEUT PAS PROUVER CE CORRECTIF. Un clone neuf n'a aucun dépôt imbriqué :
// `aucun-chemin-machine.test.ts` y passait déjà AVANT la correction, et y passe après. Le
// vert de la CI et « le bug est corrigé » sont ici deux affirmations indépendantes. La seule
// preuve est une exécution dans un clone qui porte réellement des worktrees — faite le
// 2026-11-22 dans celui de Thomas, ROUGE avant, VERT après.
//
// C'est pour cela que le test ci-dessous **fabrique** son dépôt imbriqué dans un dossier
// temporaire au lieu d'espérer en trouver un : c'est ce qui le rend vrai partout, y compris
// là où le défaut ne peut pas se manifester. Toute vérification qui porte sur
// l'environnement de travail plutôt que sur le contenu du dépôt aura cette propriété.
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { fichiers } from './_lecture';

let racine: string;

beforeAll(() => {
  racine = mkdtempSync(join(tmpdir(), 'hean-lecture-'));
  writeFileSync(join(racine, 'a-moi.md'), 'un fichier de ce dépôt');

  // Un worktree, vu du disque : un dossier avec une entrée `.git` qui pointe ailleurs.
  const imbrique = join(racine, 'worktrees', 'une-session');
  mkdirSync(imbrique, { recursive: true });
  writeFileSync(join(imbrique, '.git'), 'gitdir: /ailleurs/.git/worktrees/une-session\n');
  writeFileSync(join(imbrique, 'a-quelqu-un-dautre.md'), "un fichier d'une autre branche");
});

afterAll(() => rmSync(racine, { recursive: true, force: true }));

describe('parcours de fichiers', () => {
  it('voit les fichiers du dépôt', () => {
    expect(fichiers('.', ['.md'], racine)).toContain('a-moi.md');
  });

  it('ne descend pas dans un dépôt imbriqué', () => {
    const vus = fichiers('.', ['.md'], racine);
    expect(vus.filter((f) => f.includes('une-session'))).toEqual([]);
    expect(vus).toEqual(['a-moi.md']);
  });
});
