// Un backend de bibliothèque en mémoire, pour éprouver les comportements du frontend sans
// lancer l'application.
//
// Pourquoi c'est légitime, et où est la limite : ce faux-ci répond aux mêmes signatures que
// l'IPC réelle — il implémente le type `Backend`, lui-même dérivé des bindings GÉNÉRÉS
// depuis le Rust. Si une commande change de forme côté Rust, `npm run typecheck` casse ici.
// Un faux qui dérive de la vraie chose est un harnais ; un faux écrit à la main et libre de
// dériver est un décor.
//
// Ce qu'il NE prouve pas : rien de ce qui se passe dans SQLite ou dans le fichier audio. Ces
// deux-là sont prouvés en Rust, sur de vrais fichiers, par `src-tauri/tests/bibliotheque/`
// — INV-07 (l'écriture va dans le fichier, `edition.rs`), INV-01 (une seule clé d'album,
// `album.rs`) et la permanence de l'exclusion (INV-06, `scan.rs`). Ici on prouve la moitié frontend : que rien ne reste coincé dans
// l'état local, et que toutes les vues sont prévenues.

import type { Backend } from '../../src/lib/library/repository';
import type { Edition, RapportDeScan, Track } from '../../src/ipc/client';

/** Une piste complète à partir de ce qui compte pour le test. */
export function piste(p: Partial<Track> & { path: string }): Track {
  return {
    titre: p.path,
    artiste: '',
    album: '',
    album_artiste: null,
    cle_album: '',
    annee: null,
    piste: null,
    disque: null,
    genre: null,
    duree_ms: 1000,
    format: 'FLAC',
    echantillonnage: 44100,
    bits: 16,
    debit: null,
    ...p,
  };
}

export interface FauxBackend extends Backend {
  /** Le magasin « persistant » : il survit à la création d'un nouveau dépôt, c'est-à-dire
   *  à un redémarrage de l'application (V015). */
  readonly magasin: Map<string, Track>;
  /** Combien de fois chaque commande a été appelée. C'est ce qui permet de prouver qu'un
   *  rechargement n'a PAS eu lieu (V004) — une absence ne se voit pas autrement. */
  readonly appels: Record<string, number>;
  /** Ce que le prochain scan annoncera. */
  prochainRapport: RapportDeScan;
}

const RIEN: RapportDeScan = { ajoutees: 0, modifiees: 0, disparues: 0, illisibles: 0 };

export function creerFauxBackend(initiales: readonly Track[] = []): FauxBackend {
  const magasin = new Map<string, Track>(initiales.map((t) => [t.path, t]));
  const appels: Record<string, number> = {};
  const compter = (nom: string): void => {
    appels[nom] = (appels[nom] ?? 0) + 1;
  };

  // L'édition est appliquée comme le fait le Rust : les champs à `null`/absents ne bougent
  // pas, et la valeur rendue est celle du magasin APRÈS écriture — l'équivalent de la
  // relecture du fichier (INV-07).
  const appliquer = (chemin: string, e: Edition): Track => {
    const avant = magasin.get(chemin);
    if (!avant) throw new Error(`piste inconnue dans la bibliothèque : ${chemin}`);
    const apres: Track = {
      ...avant,
      ...(e.titre != null ? { titre: e.titre } : {}),
      ...(e.artiste != null ? { artiste: e.artiste } : {}),
      // L'album change, mais PAS la clé : construire une clé d'album est le travail de
      // `AlbumKey::new` côté Rust, et il n'y a qu'une implémentation au monde (INV-01). Un
      // faux backend qui normaliserait ici en serait une deuxième — c'est-à-dire la faute
      // exacte de V1, réintroduite par la porte du harnais. Ce que la clé devient après
      // une édition d'album est prouvé en Rust (`src-tauri/tests/bibliotheque/edition.rs`).
      ...(e.album != null ? { album: e.album } : {}),
      ...(e.annee != null ? { annee: e.annee } : {}),
      ...(e.piste != null ? { piste: e.piste } : {}),
      ...(e.genre != null ? { genre: e.genre } : {}),
    };
    magasin.set(chemin, apres);
    return apres;
  };

  const faux: FauxBackend = {
    magasin,
    appels,
    prochainRapport: RIEN,

    listTracks: () => {
      compter('listTracks');
      return Promise.resolve([...magasin.values()]);
    },
    scanLibrary: () => {
      compter('scanLibrary');
      return Promise.resolve(faux.prochainRapport);
    },
    writeMetadata: (path, edition) => {
      compter('writeMetadata');
      return Promise.resolve(appliquer(path, edition));
    },
    bulkEdit: (paths, edition) => {
      compter('bulkEdit');
      return Promise.resolve(paths.map((p) => appliquer(p, edition)));
    },
    excludeTrack: (path) => {
      compter('excludeTrack');
      magasin.delete(path);
      return Promise.resolve(null);
    },
    addLibraryRoot: (path) => {
      compter('addLibraryRoot');
      return Promise.resolve([path]);
    },
    libraryRoots: () => {
      compter('libraryRoots');
      return Promise.resolve([]);
    },
  };
  return faux;
}
