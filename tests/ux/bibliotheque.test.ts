// Comportements de la bibliothèque : édition, état partagé, persistance.
// Rouges en phase 0 : la bibliothèque arrive en phase 2.
import { describe, it } from 'vitest';
import { aPorter } from './_contrat';

describe('bibliothèque', () => {
  it('V012 — après un bulk edit, toutes les vues se rafraîchissent', () => aPorter('V012'));
  it('V013 — les objets d’état partagés ne sont jamais réassignés', () => aPorter('V013'));
  it('V014 — artist.albums se compte avec .length', () => aPorter('V014'));
  it('V015 — une métadonnée éditée survit au prochain démarrage', () => aPorter('V015'));
  it('W006 — un tag contenant un guillemet n’est pas tronqué', () => aPorter('W006'));
});
