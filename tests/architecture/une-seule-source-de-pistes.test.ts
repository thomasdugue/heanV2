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
const REPRISE_RS = 'src-tauri/src/library/reprise_v1.rs';

/**
 * Dans quel ordre la reprise de la configuration de V1 écrit ses deux listes.
 *
 * Cherche les appels, pas les commentaires : `exclure(` et `ajouter_racine(` suivis d'une
 * parenthèse ouvrante, ce qu'une phrase en prose n'a pas. Rend la suite des deux noms dans
 * leur ordre d'apparition, dédoublonnée.
 */
export function ordreDeLaReprise(source: string): string[] {
  return [...source.matchAll(/\b(exclure|ajouter_racine)\s*\(/g)]
    .map((m) => (m[1] === 'exclure' ? 'exclusions' : 'racines'))
    .filter((nom, i, tous) => tous.indexOf(nom) === i);
}

/**
 * Les commandes dont le type de retour parle de pistes, lues dans les bindings générés.
 *
 * Le nom du type est cherché sous ses TROIS formes, et c'est la leçon de la phase 2 : dès
 * qu'un type porte un champ `Option`, specta en émet deux versions — `Track_Serialize` et
 * `Track_Deserialize` — plus l'alias `Track`. Écrit en phase 0 contre un exemple à la main,
 * ce détecteur ne cherchait que `\bTrack\b` : il serait devenu aveugle à la première
 * commande réelle, donc INV-06 aurait été désarmé **en silence**, sans aucun test rouge.
 * C'est le modèle de défaut que ce harnais existe pour fermer ; il s'est présenté ici dans
 * le harnais lui-même.
 */
export function commandesDePistes(bindings: string): string[] {
  return [...bindings.matchAll(/^\t(\w+):[^\n]*?__TAURI_INVOKE<([^>]*)>/gm)]
    .filter((m) => /\bTrack(_Serialize|_Deserialize)?\b/.test(m[2] as string))
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

  it('le détecteur reconnaît aussi la forme que specta génère vraiment', () => {
    const reel =
      '\tlistTracks: (o?: InvokeOptions) => __TAURI_INVOKE<Track_Serialize[]>("list_tracks", undefined, o),';
    expect(commandesDePistes(reel)).toEqual(['listTracks']);
  });

  it('il ne se déclenche pas sur un type qui ressemble au nom', () => {
    const autre = '\tgetTracker: (o?: InvokeOptions) => __TAURI_INVOKE<Tracker>("get_tracker", undefined, o),';
    expect(commandesDePistes(autre)).toEqual([]);
  });

  it('il trouve bien les commandes réelles du projet', () => {
    // Un détecteur qui ne trouve RIEN passe au vert sans rien garantir : c'est le piège
    // décrit dans le skill hean-invariants. Tant que la bibliothèque existe, il doit voir
    // au moins une commande.
    expect(commandesDePistes(lire('src/ipc/bindings.ts')).length).toBeGreaterThan(0);
  });

  it('excluded_paths ne s’applique qu’à un seul endroit côté Rust', () => {
    // `src-tauri/tests/` est scanné en plus de `src/`, et c'est la même asymétrie que celle
    // trouvée sur INV-21 en phase 1 : un test d'intégration qui interrogerait la table
    // d'exclusion directement serait un deuxième endroit où le filtre vit — invisible tant
    // que personne n'aurait pensé à regarder là. Relevé par invariant-guardian, phase 2.
    const fautes = [...sources('src-tauri/src', ['.rs']), ...sources('src-tauri/tests', ['.rs'])]
      .filter(([f, c]) => f !== DEPOT_RS && c.includes('excluded_paths'))
      .map(([f]) => f);
    expect(fautes, `le filtre vit dans ${DEPOT_RS}, et nulle part ailleurs`).toEqual([]);
  });

  it('la reprise de V1 pose les exclusions AVANT les racines', () => {
    // Un test de forme, parce que la propriété n'est pas observable à l'exécution : elle ne
    // se manifeste que si une insertion échoue au milieu, ce qu'on ne sait pas provoquer
    // sans ouvrir une porte dans le code pour le faire.
    //
    // Ce qu'elle protège : chaque insertion est sa propre transaction, et un échec de reprise
    // n'empêche pas l'application de démarrer. Avec les racines en premier, l'état atteint
    // était « des dossiers de musique et aucune exclusion » — donc le premier scan ramenait
    // les pistes supprimées. Dans l'autre ordre, le pire état possible est « des exclusions
    // et aucune racine » : rien à scanner, donc rien à ressusciter.
    expect(ordreDeLaReprise(lire(REPRISE_RS))).toEqual(['exclusions', 'racines']);
  });

  it('le détecteur voit l’ordre inverse', () => {
    // L'auto-test, sur la forme exacte qu'avait le défaut avant le 2026-11-23.
    const fautif = `
      for path in &config.racines {
          depot.ajouter_racine(Path::new(path))?;
      }
      for path in &config.exclusions {
          depot.exclure(path)?;
      }`;
    expect(ordreDeLaReprise(fautif)).toEqual(['racines', 'exclusions']);
  });
});
