// PROPERTY-BASED TESTING sur le regroupement (HARNAIS.md §4).
//
// On ne teste pas des exemples, on teste des propriétés : ce qui doit rester vrai pour
// **toute** entrée générée. Le problème de V1 était une seule cause pour trois bugs — une
// normalisation de clé d'album écrite plusieurs fois (invariant 1, régression W003, et un
// lookup à quatre niveaux dans la home parce que deux formats de clé coexistaient). Des
// exemples choisis à la main ne l'auraient pas trouvée : on ne pense qu'aux cas auxquels on
// pense.
//
// Le partage des rôles : la NORMALISATION est en Rust et ses propriétés sont éprouvées là
// (`src-tauri/src/library/cle.rs`, avec proptest — idempotence, équivalence NFC/NFD sur
// toute chaîne Unicode générée, espaces de bord). Il n'y a qu'une implémentation au monde,
// et c'est le sens d'INV-01. Ici on éprouve ce qui est vraiment au frontend : le
// REGROUPEMENT, et son indifférence à l'ordre d'arrivée des pistes.
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { grouperEnAlbums, grouperParArtiste } from '../../src/lib/library/groupement';
import type { Track } from '../../src/ipc/client';

/** Une piste générée. La clé d'album vient du backend : ici on la fournit, on ne la calcule pas. */
const unePiste = (cles: readonly string[]) =>
  fc
    .record({
      chemin: fc.string({ minLength: 1, maxLength: 20 }),
      titre: fc.string({ maxLength: 20 }),
      artiste: fc.string({ maxLength: 20 }),
      cle: fc.constantFrom(...cles),
      albumArtiste: fc.option(fc.string({ maxLength: 20 }), { nil: null }),
    })
    .map(
      (x): Track => ({
        path: `/m/${x.chemin}.flac`,
        titre: x.titre,
        artiste: x.artiste,
        album: x.cle,
        album_artiste: x.albumArtiste,
        cle_album: x.cle,
        annee: null,
        piste: null,
        disque: null,
        genre: null,
        duree_ms: 1000,
        format: 'Flac',
        echantillonnage: 44100,
        bits: 16,
        debit: null,
      }),
    );

/** Des pistes aux chemins distincts — deux pistes ne partagent pas un chemin (B13). */
const desPistes = (cles: readonly string[], max = 30) =>
  fc
    .array(unePiste(cles), { minLength: 1, maxLength: max })
    .map((pistes) => pistes.map((p, i) => ({ ...p, path: `${p.path}#${i}` })));

describe('regroupement en albums', () => {
  it('N pistes d’un même album ne font qu’UNE entrée, quels que soient leurs artistes', () => {
    fc.assert(
      fc.property(desPistes(['Café Noir']), (pistes) => {
        const albums = grouperEnAlbums(pistes);
        expect(albums).toHaveLength(1);
        expect(albums[0]?.pistes).toHaveLength(pistes.length);
      }),
      { numRuns: 300 },
    );
  });

  it('le nombre d’albums est exactement le nombre de clés distinctes', () => {
    fc.assert(
      fc.property(desPistes(['Café Noir', 'Été', 'Sans Pochette']), (pistes) => {
        const distinctes = new Set(pistes.map((p) => p.cle_album));
        expect(grouperEnAlbums(pistes)).toHaveLength(distinctes.size);
      }),
      { numRuns: 300 },
    );
  });

  it('le regroupement est invariant par permutation', () => {
    // La propriété qui tue la famille de bugs de la mosaïque (INV-11, régression du
    // 2026-03-16) : le résultat ne doit pas dépendre de l'ordre d'arrivée, c'est-à-dire de
    // l'ordre du scan, c'est-à-dire de rien.
    fc.assert(
      fc.property(desPistes(['Café Noir', 'Été', 'Sans Pochette']), (pistes) => {
        const direct = grouperEnAlbums(pistes).map((a) => a.cle);
        const inverse = grouperEnAlbums([...pistes].reverse()).map((a) => a.cle);
        expect(inverse).toEqual(direct);
      }),
      { numRuns: 300 },
    );
  });

  it('aucune piste n’est perdue ni dupliquée', () => {
    fc.assert(
      fc.property(desPistes(['Café Noir', 'Été']), (pistes) => {
        const dedans = grouperEnAlbums(pistes).flatMap((a) => a.pistes.map((p) => p.path));
        expect(new Set(dedans).size).toBe(dedans.length);
        expect([...dedans].sort()).toEqual(pistes.map((p) => p.path).sort());
      }),
      { numRuns: 200 },
    );
  });
});

describe('regroupement par artiste', () => {
  it('`albums` est TOUJOURS un Array, et se compte avec .length (V014)', () => {
    fc.assert(
      fc.property(desPistes(['Café Noir', 'Été']), (pistes) => {
        for (const a of grouperParArtiste(pistes)) {
          expect(Array.isArray(a.albums)).toBe(true);
          expect(a.albums.length).toBe(new Set(a.pistes.map((p) => p.cle_album)).size);
        }
      }),
      { numRuns: 300 },
    );
  });

  it('un album aux artistes différents reste UN album chez son album artist', () => {
    // La régression W003, en propriété : quel que soit le nombre de variations « feat. »,
    // un album dont toutes les pistes portent le même album artist n'apparaît qu'une fois.
    fc.assert(
      fc.property(
        fc.array(fc.string({ maxLength: 15 }), { minLength: 1, maxLength: 10 }),
        (artistes) => {
          const pistes: Track[] = artistes.map((artiste, i) => ({
            path: `/m/${i}.flac`,
            titre: `piste ${i}`,
            artiste: `${artiste} feat. Hean`,
            album_artiste: 'Zoé Léger',
            album: 'Café Noir',
            cle_album: 'Café Noir',
            annee: null,
            piste: i,
            disque: null,
            genre: null,
            duree_ms: 1000,
            format: 'Flac',
            echantillonnage: 44100,
            bits: 16,
            debit: null,
          }));
          const parArtiste = grouperParArtiste(pistes);
          expect(parArtiste).toHaveLength(1);
          expect(parArtiste[0]?.albums.length).toBe(1);
        },
      ),
      { numRuns: 200 },
    );
  });

  it('toutes les pistes se retrouvent chez exactement un artiste', () => {
    fc.assert(
      fc.property(desPistes(['Café Noir', 'Été']), (pistes) => {
        const dedans = grouperParArtiste(pistes).flatMap((a) => a.pistes.map((p) => p.path));
        expect(new Set(dedans).size).toBe(pistes.length);
      }),
      { numRuns: 200 },
    );
  });
});
