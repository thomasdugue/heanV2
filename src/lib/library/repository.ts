// LE dépôt de bibliothèque du frontend — le seul module autorisé à appeler une commande
// qui rend des pistes (INV-06).
//
// `tests/architecture/une-seule-source-de-pistes.test.ts` refuse que `listTracks`,
// `writeMetadata` ou `bulkEdit` apparaissent ailleurs que dans ce fichier. En V1, la règle
// disait « vérifie le filtre d'exclusion à CHAQUE point d'entrée », il y en avait trois
// listés à la main, et le quatrième l'a oubliée. Ici il n'y a qu'un point d'entrée : les
// vues parleront à ce module, jamais à l'IPC.
//
// Le dépôt est une FABRIQUE qui prend son backend en paramètre, avec l'IPC réelle par
// défaut. C'est ce qui rend les comportements de `tests/ux/` vérifiables sans lancer
// l'application — ce que V1 ne pouvait pas faire, et qui lui a coûté trois suites de tests
// skippées (Navigation, PlayerControls, AlbumView).

import {
  addLibraryRoot,
  addNetworkSource,
  bulkEdit,
  excludeTrack,
  libraryRoots,
  listTracks,
  networkSources,
  saveNetworkPassword,
  scanLibrary,
  scanNetworkSource,
  writeMetadata,
  type Edition,
  type RapportDeScan,
  type SourceReseau,
  type Track,
} from '../../ipc/client';
import { bibliotheque, estChargee, majDesPistes, oublierLaPiste, remplacerLesPistes } from './etat';

/**
 * La surface IPC dont le dépôt a besoin. Le type est **dérivé des signatures générées** :
 * si une commande Rust change de forme, `bindings.ts` change, et ce type ne compile plus.
 * Un faux backend de test doit donc rester fidèle, sinon `npm run typecheck` échoue.
 */
export interface Backend {
  listTracks: () => Promise<Track[]>;
  scanLibrary: () => Promise<RapportDeScan>;
  writeMetadata: (path: string, edition: Edition) => Promise<Track>;
  bulkEdit: (paths: string[], edition: Edition) => Promise<Track[]>;
  excludeTrack: (path: string) => Promise<null>;
  addLibraryRoot: (path: string) => Promise<string[]>;
  libraryRoots: () => Promise<string[]>;
  // Les sources réseau (ADR 0016). Elles passent par ce dépôt-ci et pas par un second :
  // 91 % de la bibliothèque de Thomas vient de là, donc un scan réseau change la même
  // liste de pistes qu'un scan de disque, et c'est le même rechargement conditionnel qui
  // doit s'appliquer (V004).
  networkSources: () => Promise<SourceReseau[]>;
  addNetworkSource: (
    nom: string,
    hote: string,
    partage: string,
    cheminDistant: string,
    utilisateur: string,
  ) => Promise<SourceReseau[]>;
  saveNetworkPassword: (sourceId: string, motDePasse: string) => Promise<null>;
  scanNetworkSource: (sourceId: string) => Promise<RapportDeScan>;
}

/** L'IPC réelle, assemblée ici — le seul endroit du frontend où ces noms ont le droit d'être. */
export const IPC_REELLE: Backend = {
  listTracks: () => listTracks(),
  scanLibrary: () => scanLibrary(),
  writeMetadata: (path, edition) => writeMetadata(path, edition),
  bulkEdit: (paths, edition) => bulkEdit(paths, edition),
  excludeTrack: (path) => excludeTrack(path),
  addLibraryRoot: (path) => addLibraryRoot(path),
  libraryRoots: () => libraryRoots(),
  networkSources: () => networkSources(),
  addNetworkSource: (nom, hote, partage, cheminDistant, utilisateur) =>
    addNetworkSource(nom, hote, partage, cheminDistant, utilisateur),
  saveNetworkPassword: (sourceId, motDePasse) => saveNetworkPassword(sourceId, motDePasse),
  scanNetworkSource: (sourceId) => scanNetworkSource(sourceId),
};

/** Un abonné : une vue qui veut être prévenue quand la bibliothèque change. */
export type Abonne = () => void;

export interface Depot {
  /** Charge la bibliothèque depuis la base. Idempotent. */
  charger: () => Promise<void>;
  /**
   * Lance un scan. Ne recharge la liste **que si le scan a trouvé quelque chose** (V004).
   * Rend le rapport, pour que l'appelant puisse l'afficher.
   */
  scanner: () => Promise<RapportDeScan>;
  /** Édite une piste (INV-07 : l'écriture va dans le fichier audio, côté Rust). */
  editer: (chemin: string, edition: Edition) => Promise<void>;
  /** Édite N pistes d'un coup (B08). */
  editerEnLot: (chemins: readonly string[], edition: Edition) => Promise<void>;
  /** Exclut une piste, définitivement (INV-06). */
  exclure: (chemin: string) => Promise<void>;
  ajouterUnDossier: (chemin: string) => Promise<string[]>;
  dossiers: () => Promise<string[]>;
  /** Les sources réseau enregistrées (ADR 0016). */
  sourcesReseau: () => Promise<SourceReseau[]>;
  ajouterUneSourceReseau: (
    nom: string,
    hote: string,
    partage: string,
    cheminDistant: string,
    utilisateur: string,
  ) => Promise<SourceReseau[]>;
  /** Enregistre un mot de passe dans le trousseau macOS. Il descend et ne remonte jamais. */
  enregistrerLeMotDePasse: (sourceId: string, motDePasse: string) => Promise<void>;
  /**
   * Scanne une source réseau. **Même condition de rechargement qu'un scan de disque**, et
   * littéralement la même fonction (`rechargerSiChange`) : c'est la raison pour laquelle
   * cette méthode vit ici et pas dans un second dépôt — deux endroits appliqueraient la
   * condition, et l'un des deux finirait par l'oublier (V004).
   */
  scannerUneSourceReseau: (sourceId: string) => Promise<RapportDeScan>;
  /** S'abonner aux changements. Rend la fonction de désabonnement. */
  surChangement: (abonne: Abonne) => () => void;
}

export function creerDepot(backend: Backend = IPC_REELLE): Depot {
  const abonnes = new Set<Abonne>();

  /**
   * V012 — **inconditionnel**. En V1, le rafraîchissement était gardé par un
   * `if (viewsToRefresh.includes(currentView))` qui excluait certaines vues : après un bulk
   * edit, une vue non listée gardait les anciennes valeurs à l'écran. Ici tous les abonnés
   * sont prévenus, sans condition et sans savoir qui ils sont. Une vue qui n'a rien à
   * refaire ne fait rien ; une vue oubliée, elle, n'existe pas.
   */
  const prevenirTousLesAbonnes = (): void => {
    // Une COPIE de la liste : un abonné qui se désabonne pendant qu'on le prévient (une vue
    // détruite par ce qu'elle vient d'apprendre) ne doit pas faire sauter les suivants.
    const aPrevenir = Array.from(abonnes);
    for (const a of aPrevenir) a();
  };

  const charger = async (): Promise<void> => {
    remplacerLesPistes(await backend.listTracks());
    prevenirTousLesAbonnes();
  };

  /**
   * **LA condition de V004, écrite une seule fois.** En V1, `scan_complete` rechargeait
   * toujours la bibliothèque, et ce rechargement entrait en course avec le premier rendu de
   * la home — l'utilisateur voyait un état partiel, ou un clignotement.
   *
   * Elle est une fonction et non deux `if` parce qu'il y a maintenant **deux** chemins de
   * scan, le disque et le réseau (ADR 0016), et que 91 % de la bibliothèque de Thomas passe
   * par le second. Deux copies de la condition, c'est la famille de défaut de V1 : celle
   * qu'on oublie de mettre à jour est précisément celle qui compte.
   */
  const rechargerSiChange = async (rapport: RapportDeScan): Promise<RapportDeScan> => {
    if (rapport.ajoutees === 0 && rapport.modifiees === 0 && rapport.disparues === 0) {
      return rapport;
    }
    await charger();
    return rapport;
  };

  return {
    charger,

    scanner: async () => rechargerSiChange(await backend.scanLibrary()),

    editer: async (chemin, edition) => {
      // La piste rendue par le backend vient de la RELECTURE du fichier audio (INV-07) :
      // on range donc celle-là, et pas ce qu'on croyait avoir écrit.
      majDesPistes([await backend.writeMetadata(chemin, edition)]);
      prevenirTousLesAbonnes();
    },

    editerEnLot: async (chemins, edition) => {
      majDesPistes(await backend.bulkEdit([...chemins], edition));
      prevenirTousLesAbonnes();
    },

    exclure: async (chemin) => {
      await backend.excludeTrack(chemin);
      oublierLaPiste(chemin);
      prevenirTousLesAbonnes();
    },

    ajouterUnDossier: (chemin) => backend.addLibraryRoot(chemin),
    dossiers: () => backend.libraryRoots(),

    sourcesReseau: () => backend.networkSources(),
    ajouterUneSourceReseau: (nom, hote, partage, cheminDistant, utilisateur) =>
      backend.addNetworkSource(nom, hote, partage, cheminDistant, utilisateur),

    enregistrerLeMotDePasse: async (sourceId, motDePasse) => {
      await backend.saveNetworkPassword(sourceId, motDePasse);
    },

    scannerUneSourceReseau: async (sourceId) =>
      rechargerSiChange(await backend.scanNetworkSource(sourceId)),

    surChangement: (abonne) => {
      abonnes.add(abonne);
      return () => abonnes.delete(abonne);
    },
  };
}

export { bibliotheque, estChargee };
export type { SourceReseau };
