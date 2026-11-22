// LE SEUL module du frontend autorisé à parler à Tauri.
//
// Tout le reste du frontend importe d'ici, jamais de `@tauri-apps/api` ni de
// `window.__TAURI__` (`withGlobalTauri` est à false). Deux raisons :
//   1. le client devient substituable, donc `invoke` est mockable — c'est ce qui manquait
//      à V1, où 3 suites de tests (Navigation, PlayerControls, AlbumView) étaient skippées ;
//   2. les signatures viennent de `bindings.ts`, GÉNÉRÉ depuis le Rust par tauri-specta :
//      renommer une commande côté Rust casse la compilation du front.
//
// Vérifié par tests/architecture/ipc.test.ts.
import { commands } from './bindings';

// Les types voyagent dans UN sens, et les noms le disent. `Track_Serialize` est la forme
// qu'une piste prend en remontant du Rust, `Edition_Deserialize` celle qu'une édition prend
// en descendant — specta émet les deux versions de tout type portant un champ optionnel
// (voir src-tauri/src/library/types.rs). Le reste du frontend n'a pas à connaître ce
// détail : il lit `Track` et `Edition`.
export type { RapportDeScan } from './bindings';
export type { Track_Serialize as Track, Edition_Deserialize as Edition } from './bindings';

export function appVersion(): Promise<string> {
  return commands.appVersion();
}

// Les commandes de la bibliothèque sont réexportées SOUS LEUR NOM GÉNÉRÉ, et c'est voulu.
// `tests/architecture/une-seule-source-de-pistes.test.ts` tient INV-06 en cherchant ces
// noms-là dans tout `src/` : les renommer ici (`listerPistes`…) rendrait le détecteur
// aveugle, puisqu'il ne verrait plus passer le nom qu'il surveille. Un seul fichier hors de
// `src/ipc/` a le droit de les employer : `src/lib/library/repository.ts`.
export const { listTracks, scanLibrary, writeMetadata, bulkEdit, excludeTrack, addLibraryRoot, libraryRoots } =
  commands;
