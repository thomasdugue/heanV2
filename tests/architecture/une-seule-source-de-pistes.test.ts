// INV-06 — « la suppression de pistes est permanente ».
//
// En V1 la règle disait : « le filtre excluded_paths doit être vérifié à CHAQUE point
// d'entrée, et dans tout nouveau point d'entrée ». Une règle formulée comme « n'oublie pas
// de répéter ça partout » finit violée. Ici elle devient : il n'y a QU'UN point d'entrée.
//
// Le test se dérive du catalogue IPC généré, donc il couvrira automatiquement les commandes
// qui n'existent pas encore.
import { describe, expect, it } from 'vitest';
import { lire, sources } from './_lecture';

const DEPOT_TS = 'src/lib/library/repository.ts';
const DEPOT_RS = 'src-tauri/src/library/repository.rs';

/** Les commandes dont le type de retour parle de pistes, lues dans les bindings générés. */
export function commandesDePistes(bindings: string): string[] {
  return [...bindings.matchAll(/^\t(\w+):[^\n]*?__TAURI_INVOKE<([^>]*)>/gm)]
    .filter((m) => /\bTrack\b/.test(m[2] as string))
    .map((m) => m[1] as string);
}

describe('INV-06 — une seule construction de liste de pistes', () => {
  it('les commandes qui rendent des pistes ne sont appelées que par le dépôt', () => {
    const commandes = commandesDePistes(lire('src/ipc/bindings.ts'));
    const fautes = sources('src', ['.ts', '.svelte'])
      .filter(([f]) => f !== DEPOT_TS && !f.startsWith('src/ipc/'))
      .flatMap(([f, c]) => commandes.filter((n) => c.includes(n)).map((n) => `${f} appelle ${n}`));
    expect(fautes, `seul ${DEPOT_TS} construit une liste de pistes`).toEqual([]);
  });

  it('le détecteur reconnaît une commande qui rend des pistes', () => {
    const faux = '\tlistTracks: (o?: InvokeOptions) => __TAURI_INVOKE<Track[]>("list_tracks", undefined, o),';
    expect(commandesDePistes(faux)).toEqual(['listTracks']);
  });

  it('excluded_paths ne s’applique qu’à un seul endroit côté Rust', () => {
    const fautes = sources('src-tauri/src', ['.rs'])
      .filter(([f, c]) => f !== DEPOT_RS && c.includes('excluded_paths'))
      .map(([f]) => f);
    expect(fautes, `le filtre vit dans ${DEPOT_RS}, et nulle part ailleurs`).toEqual([]);
  });
});
