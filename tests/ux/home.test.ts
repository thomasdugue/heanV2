// Comportements de la page d'accueil et des carousels.
// Rouges en phase 0 : la home n'existe pas encore (phase 3).
import { describe, it } from 'vitest';
import { aPorter } from './_contrat';

describe('home', () => {
  it('V001 — s’affiche complète au démarrage, sans flash ni état partiel', () => aPorter('V001'));
  it('V002 — Recently Played : 3 / 2 / 1 colonnes selon la largeur, max 6 items', () => aPorter('V002'));
  it('V003 — les carousels vont jusqu’au bord droit, la grille ne gonfle pas', () => aPorter('V003'));
  it('V004 — pas de rechargement si le scan n’a rien trouvé de nouveau', () => aPorter('V004'));
  it('V011 — la scrollbar des carousels est invisible au repos', () => aPorter('V011'));
  it('W007 — une bibliothèque uniquement NAS s’affiche', () => aPorter('W007'));
});
