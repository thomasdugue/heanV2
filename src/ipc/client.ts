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
// Une source réseau remonte du Rust : c'est la forme `_Serialize` qu'on lit. Elle descend
// aussi, mais jamais telle quelle — `addNetworkSource` prend ses champs un par un, ce qui
// fait que l'interface ne peut pas fabriquer une source avec un identifiant de son choix.
export type { SourceReseau_Serialize as SourceReseau } from './bindings';
// L'état de la lecture et les sorties audio remontent du Rust : forme `_Serialize`. Aucun des
// deux ne descend — l'interface n'envoie que des ordres (jouer, se déplacer, régler), jamais
// un état complet, donc elle ne peut pas inventer une lecture qui n'a pas lieu.
export type { EtatLecture_Serialize as EtatLecture, PeripheriqueAudio } from './bindings';

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

// Les commandes réseau (ADR 0016). Aucune ne rend de piste : un scan rend un rapport, et
// c'est le dépôt qui recharge la liste si ce rapport dit que quelque chose a bougé (V004).
export const { networkSources, addNetworkSource, saveNetworkPassword, scanNetworkSource } = commands;

// Les commandes de la lecture (ADR 0018 pour le refus d'une fréquence non servie). Aucune
// n'attend que le son sorte : elles postent un ordre au fil de lecture et rendent la main.
// `playbackState` est le SEUL chemin par lequel l'interface apprend où en est la lecture —
// il n'y a pas d'événement, donc pas d'état local à garder en phase (V013).
export const {
  playTracks,
  playbackPause,
  playbackResume,
  playbackStop,
  playbackSeek,
  playbackNext,
  playbackPrevious,
  setVolume,
  audioDevices,
  setAudioDevice,
  setExclusiveMode,
  refreshAudioDevices,
  playbackState,
} = commands;
