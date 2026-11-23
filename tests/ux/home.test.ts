// Comportements de la page d'accueil et des carousels.
// Rouges tant que les vues n'existent pas (phase 3) — sauf V004, qui est une décision du
// dépôt de bibliothèque et non un fait d'affichage : porté en phase 2.
import { beforeEach, describe, expect, it } from 'vitest';
import { creerDepot } from '../../src/lib/library/repository';
import { bibliotheque, reinitialiserPourTest } from '../../src/lib/library/etat';
import { creerFauxBackend, piste } from './_faux-backend';
import { aPorter } from './_contrat';

beforeEach(() => reinitialiserPourTest());

describe('home', () => {
  it('V001 — s’affiche complète au démarrage, sans flash ni état partiel', () => aPorter('V001'));
  it('V002 — Recently Played : 3 / 2 / 1 colonnes selon la largeur, max 6 items', () => aPorter('V002'));
  it('V003 — les carousels vont jusqu’au bord droit, la grille ne gonfle pas', () => aPorter('V003'));
  it('V004 — pas de rechargement si le scan n’a rien trouvé de nouveau', async () => {
    const backend = creerFauxBackend([piste({ path: '/m/a.flac', titre: 'Première' })]);
    const depot = creerDepot(backend);
    await depot.charger();
    expect(backend.appels['listTracks']).toBe(1);

    // Un scan qui ne trouve rien : la liste ne doit PAS être redemandée. En V1,
    // `scan_complete` rechargeait toujours la bibliothèque, et ce rechargement entrait en
    // course avec le premier rendu de la home — l'utilisateur voyait un état partiel.
    // Une absence d'appel ne se voit pas autrement qu'en comptant.
    backend.prochainRapport = { ajoutees: 0, modifiees: 0, disparues: 0, illisibles: 0 };
    const rien = await depot.scanner();
    expect(rien.ajoutees).toBe(0);
    expect(backend.appels['listTracks']).toBe(1);

    // Des fichiers illisibles ne sont pas un changement non plus : rien de nouveau n'est
    // entré dans la bibliothèque.
    backend.prochainRapport = { ajoutees: 0, modifiees: 0, disparues: 0, illisibles: 4 };
    await depot.scanner();
    expect(backend.appels['listTracks']).toBe(1);

    // Mais dès qu'il trouve quelque chose, la liste est rechargée — et une seule fois.
    backend.magasin.set('/m/b.flac', piste({ path: '/m/b.flac', titre: 'Deuxième' }));
    backend.prochainRapport = { ajoutees: 1, modifiees: 0, disparues: 0, illisibles: 0 };
    await depot.scanner();
    expect(backend.appels['listTracks']).toBe(2);
    expect(bibliotheque.pistes).toHaveLength(2);
  });
  it('V011 — la scrollbar des carousels est invisible au repos', () => aPorter('V011'));
  it('W007 — une bibliothèque uniquement NAS s’affiche', () => aPorter('W007'));
});
