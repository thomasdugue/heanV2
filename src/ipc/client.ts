// LE SEUL module du frontend autorisé à parler à Tauri.
//
// Tout le reste du frontend importe d'ici, jamais de `@tauri-apps/api` ni de
// `window.__TAURI__` (`withGlobalTauri` est à false). Deux raisons :
//   1. le client devient substituable, donc `invoke` est mockable — c'est ce qui manquait
//      à V1, où 3 suites de tests (Navigation, PlayerControls, AlbumView) étaient skippées ;
//   2. les signatures viennent de `bindings.ts`, GÉNÉRÉ depuis le Rust par tauri-specta :
//      renommer une commande côté Rust casse la compilation du front.
//
// Vérifié par tests/architecture/ipc-un-seul-point-dentree.test.ts.
import { commands } from './bindings';

export function appVersion(): Promise<string> {
  return commands.appVersion();
}
