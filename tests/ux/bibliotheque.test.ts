// Comportements de la bibliothèque : édition, état partagé, persistance.
//
// Ces cinq-là sont PORTÉS (phase 2). Ce qu'ils couvrent est la moitié frontend : l'état
// partagé, le rafraîchissement des vues, et le fait que rien ne reste coincé en mémoire.
// L'autre moitié — le fichier audio, SQLite, la clé d'album — est prouvée en Rust sur de
// vrais fichiers par `src-tauri/tests/bibliotheque.rs`, et le faux backend d'ici est dérivé
// des types générés pour ne pas pouvoir mentir sur la forme de l'IPC (`_faux-backend.ts`).
import { beforeEach, describe, expect, it } from 'vitest';
import { creerDepot } from '../../src/lib/library/repository';
import { bibliotheque, estChargee, reinitialiserPourTest } from '../../src/lib/library/etat';
import { grouperEnAlbums, grouperParArtiste } from '../../src/lib/library/groupement';
import { creerFauxBackend, piste } from './_faux-backend';

beforeEach(() => reinitialiserPourTest());

const DEUX = [
  piste({ path: '/m/a.flac', titre: 'Première', artiste: 'Zoé Léger', album: 'Café Noir', cle_album: 'Café Noir' }),
  piste({
    path: '/m/b.flac',
    titre: 'Deuxième',
    artiste: 'Zoé Léger feat. Hean',
    album: 'Café Noir',
    cle_album: 'Café Noir',
    album_artiste: 'Zoé Léger',
  }),
];

describe('bibliothèque', () => {
  it('V012 — après un bulk edit, toutes les vues se rafraîchissent', async () => {
    const backend = creerFauxBackend(DEUX);
    const depot = creerDepot(backend);
    await depot.charger();

    // Trois vues, comme en V1 : une liste, une page d'artiste, une sidebar. En V1, le
    // rafraîchissement était gardé par `if (viewsToRefresh.includes(currentView))`, et une
    // vue non listée gardait les anciennes valeurs à l'écran.
    const prevenues: string[] = [];
    for (const nom of ['pistes', 'artiste', 'sidebar']) {
      depot.surChangement(() => prevenues.push(nom));
    }

    await depot.editerEnLot(['/m/a.flac', '/m/b.flac'], { genre: 'Chanson' });

    expect(prevenues).toEqual(['pistes', 'artiste', 'sidebar']);
    expect(bibliotheque.pistes.every((p) => p.genre === 'Chanson')).toBe(true);

    // Et un désabonnement coupe bien le lien : sans quoi une vue détruite serait prévenue
    // pour toujours, ce qui est l'autre moitié du même bug.
    prevenues.length = 0;
    const couper = depot.surChangement(() => prevenues.push('éphémère'));
    couper();
    await depot.editerEnLot(['/m/a.flac'], { genre: 'Jazz' });
    expect(prevenues).not.toContain('éphémère');
    expect(prevenues).toEqual(['pistes', 'artiste', 'sidebar']);
  });

  it('V013 — les objets d’état partagés ne sont jamais réassignés', async () => {
    const backend = creerFauxBackend(DEUX);
    const depot = creerDepot(backend);

    // Ce qu'une vue retient : l'objet, et ses champs. En V1, `library = {}` cassait cette
    // référence et la vue continuait à lire l'ancien objet — sans erreur.
    const vu = bibliotheque;
    const sesPistes = bibliotheque.pistes;
    const sonIndex = bibliotheque.parChemin;

    await depot.charger();
    expect(vu).toBe(bibliotheque);
    expect(sesPistes).toBe(bibliotheque.pistes);
    expect(sonIndex).toBe(bibliotheque.parChemin);
    expect(sesPistes).toHaveLength(2);
    expect(sonIndex.get('/m/a.flac')?.titre).toBe('Première');

    // Un deuxième chargement ne remplace rien non plus.
    await depot.charger();
    expect(sesPistes).toBe(bibliotheque.pistes);
    expect(sesPistes).toHaveLength(2);

    // Et la faute de V1 est désormais IMPOSSIBLE, pas seulement déconseillée : l'objet est
    // gelé, donc en module ES (mode strict) l'affectation lève. C'est le détecteur mis à
    // l'épreuve — une règle qu'on n'a jamais vue refuser quelque chose ne garantit rien.
    expect(() => {
      (bibliotheque as unknown as { pistes: unknown[] }).pistes = [];
    }).toThrow(TypeError);
    expect(bibliotheque.pistes).toBe(sesPistes);

    expect(estChargee()).toBe(true);
  });

  it('V014 — artist.albums se compte avec .length', async () => {
    const backend = creerFauxBackend(DEUX);
    const depot = creerDepot(backend);
    await depot.charger();

    const artistes = grouperParArtiste(bibliotheque.pistes);
    // Les deux pistes partagent l'album, et l'album artist les réunit sous un seul artiste.
    expect(artistes).toHaveLength(1);
    const zoe = artistes[0];
    if (!zoe) throw new Error('un artiste attendu');

    // En V1 ce champ est passé d'un Set à un Array sans que les appelants suivent :
    // `artist.albums.size` rendait `undefined`, et l'écran affichait « undefined albums ».
    expect(Array.isArray(zoe.albums)).toBe(true);
    expect(zoe.albums.length).toBe(1);
    expect(zoe.pistes.length).toBe(2);

    // Et `.size` ne doit JAMAIS compiler. Si quelqu'un repassait `albums` à un Set, cette
    // ligne cesserait d'être une erreur, et `@ts-expect-error` ferait alors échouer
    // `npm run typecheck` — qui fait partie de `./check`. Le type est le test.
    // @ts-expect-error — `albums` est un Array : `.size` n'existe pas.
    expect(zoe.albums.size).toBeUndefined();
  });

  it('V015 — une métadonnée éditée survit au prochain démarrage', async () => {
    const backend = creerFauxBackend(DEUX);
    await creerDepot(backend).editer('/m/a.flac', { genre: 'Jazz Manouche', annee: 1954 });

    // « Redémarrage » : un dépôt tout neuf, un état local remis à zéro, le même backend.
    // Si l'édition était restée dans l'état local du frontend — le défaut de V1, où
    // METADATA_CACHE et TRACKS_CACHE se désynchronisaient — elle aurait disparu ici.
    reinitialiserPourTest();
    const apres = creerDepot(backend);
    await apres.charger();

    const rechargee = bibliotheque.parChemin.get('/m/a.flac');
    expect(rechargee?.genre).toBe('Jazz Manouche');
    expect(rechargee?.annee).toBe(1954);
    // Les champs non édités n'ont pas bougé.
    expect(rechargee?.titre).toBe('Première');
  });

  it('W006 — un tag contenant un guillemet n’est pas tronqué', async () => {
    // En V1, `escapeHtml()` n'échappait pas les guillemets et le tag était tronqué **à
    // l'enregistrement** : la donnée était perdue, pas seulement mal affichée. La leçon
    // tient en une phrase — l'échappement est l'affaire de l'AFFICHAGE, et Svelte s'en
    // charge au rendu ; le chemin de données, lui, ne transforme rien.
    const perilleux = `Tiré d'un "live" <b>&</b> ailleurs — 100% ' ok`;
    const backend = creerFauxBackend(DEUX);
    const depot = creerDepot(backend);
    await depot.charger();

    await depot.editer('/m/a.flac', { titre: perilleux });
    expect(bibliotheque.parChemin.get('/m/a.flac')?.titre).toBe(perilleux);

    reinitialiserPourTest();
    const apres = creerDepot(backend);
    await apres.charger();
    const rechargee = bibliotheque.parChemin.get('/m/a.flac');
    expect(rechargee?.titre).toBe(perilleux);
    expect(rechargee?.titre).toContain('"');
    expect(rechargee?.titre?.length).toBe(perilleux.length);
  });

  // Hors contrat anti-régression, mais c'est le même mécanisme : un album reste un album
  // quel que soit l'ordre d'arrivée de ses pistes (INV-01, HARNAIS.md §4).
  it('un album ne compte qu’une entrée quels que soient ses artistes', () => {
    expect(grouperEnAlbums(DEUX)).toHaveLength(1);
    expect(grouperEnAlbums([...DEUX].reverse())).toHaveLength(1);
  });
});
