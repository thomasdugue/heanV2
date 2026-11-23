// Regrouper des pistes en albums et en artistes.
//
// Ce module ne CONSTRUIT jamais de clé d'album : il regroupe sur `piste.cle_album`, que le
// backend a produite avec `AlbumKey::new` (INV-01). C'est la décision qui tue la famille de
// bugs la plus récurrente de V1 — là-bas, `normalizeKey()` (trim + NFC) était écrite
// plusieurs fois, et deux formats de clé finissaient par coexister. Une seule
// implémentation, et elle est en Rust ; `tests/architecture/une-seule-cle-dalbum.test.ts`
// refuse que le frontend normalise quoi que ce soit.

import type { Track } from '../../ipc/client';

export interface Album {
  /** La clé venue du backend. Jamais recalculée ici. */
  readonly cle: string;
  /** Le nom affichable, tel qu'il est tagué dans la première piste rencontrée. */
  readonly nom: string;
  readonly pistes: readonly Track[];
}

export interface Artiste {
  readonly nom: string;
  /**
   * **Un Array, et il se compte avec `.length`** (V014). En V1 ce champ est passé d'un Set
   * à un Array sans que les appelants suivent : `artist.albums.size` rendait `undefined`,
   * et l'affichage disait « undefined albums ». Ici le type l'interdit — `.size` sur un
   * Array ne compile pas, et `npm run typecheck` fait partie de `./check`.
   */
  readonly albums: readonly Album[];
  readonly pistes: readonly Track[];
}

/**
 * Les albums, un par clé. Déterministe : l'ordre ne dépend pas de l'ordre d'entrée, il est
 * trié par nom d'album puis par clé — sans quoi deux appels sur les mêmes pistes dans un
 * ordre différent rendraient deux listes différentes.
 */
export function grouperEnAlbums(pistes: readonly Track[]): Album[] {
  const par = new Map<string, Track[]>();
  for (const p of pistes) {
    const deja = par.get(p.cle_album);
    if (deja) deja.push(p);
    else par.set(p.cle_album, [p]);
  }
  return [...par.entries()]
    .map(([cle, siennes]) => ({
      cle,
      // Le nom affiché vient de la première piste par ordre de chemin : stable, donc deux
      // scans dans un ordre différent n'affichent pas un nom différent.
      nom: [...siennes].sort((a, b) => a.path.localeCompare(b.path))[0]?.album ?? '',
      pistes: siennes,
    }))
    .sort((a, b) => a.nom.localeCompare(b.nom) || a.cle.localeCompare(b.cle));
}

/**
 * Les artistes, avec leurs albums.
 *
 * L'artiste retenu est l'**album artist** quand il existe, sinon l'artiste de la piste :
 * c'est ce qui évite qu'un album avec des invités (« Zoé Léger feat. Hean ») se disperse
 * entre plusieurs pages d'artiste. La clé d'album, elle, ignore l'artiste de toute façon —
 * c'est la régression W003.
 */
export function grouperParArtiste(pistes: readonly Track[]): Artiste[] {
  const par = new Map<string, Track[]>();
  for (const p of pistes) {
    const nom = p.album_artiste?.trim() || p.artiste;
    const deja = par.get(nom);
    if (deja) deja.push(p);
    else par.set(nom, [p]);
  }
  return [...par.entries()]
    .map(([nom, siennes]) => ({ nom, albums: grouperEnAlbums(siennes), pistes: siennes }))
    .sort((a, b) => a.nom.localeCompare(b.nom));
}
