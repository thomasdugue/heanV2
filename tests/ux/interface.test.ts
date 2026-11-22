// Comportements de l'interface : listes, pages artiste, sidebar, mosaïques.
// Rouges en phase 0 : les vues arrivent en phase 3.
import { describe, it } from 'vitest';
import { aPorter } from './_contrat';

describe('interface', () => {
  it('V005 — on peut glisser une piste vers une playlist', () => aPorter('V005'));
  it('V006 — la vue Pistes tient 60 fps avec 10 000+ pistes', () => aPorter('V006'));
  it('V010 — les « loose tracks » d’une page artiste font toute la largeur', () => aPorter('V010'));
  it('V016 — sous 900 px la sidebar se réduit à 48 px', () => aPorter('V016'));
  it('W001 — les vignettes de playlist s’affichent au premier démarrage', () => aPorter('W001'));
  it('W002 — la mosaïque 28×28 n’a pas de cellule noire fantôme', () => aPorter('W002'));
  it('W003 — un album « feat. » n’apparaît qu’une fois dans une mosaïque', () => aPorter('W003'));
  it('W004 — l’icône shuffle ne bouge pas quand on change de mode', () => aPorter('W004'));
});
