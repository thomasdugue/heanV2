// INV-06 — « la suppression de pistes est permanente ».
//
// En V1 la règle disait : « le filtre excluded_paths doit être vérifié à CHAQUE point
// d'entrée, et dans tout nouveau point d'entrée ». Une règle formulée comme « n'oublie pas
// de répéter ça partout » finit violée. Ici elle devient : il n'y a QU'UN point d'entrée.
//
// Le test se dérive du catalogue IPC généré, donc il couvrira automatiquement les commandes
// qui n'existent pas encore.
import { describe, expect, it } from 'vitest';
import { lire, sansCommentaires, sources } from './_lecture';

const DEPOT_TS = 'src/lib/library/repository.ts';
const DEPOT_RS = 'src-tauri/src/library/repository.rs';
const REPRISE_RS = 'src-tauri/src/library/reprise_v1.rs';
const LIGNES_RS = 'src-tauri/src/library/lignes.rs';

/** Tout le Rust du projet : le backend ET ses tests d'intégration. */
const RUST = (): [string, string][] => [
  ...sources('src-tauri/src', ['.rs']),
  ...sources('src-tauri/tests', ['.rs']),
];

/** Les façons de parler à SQLite. Si rusqlite en ajoutait une, elle manquerait ici. */
const SQL =
  /\.(execute|execute_batch|prepare|query_row|query_map|pragma_update|pragma_query)\s*\(/;

/**
 * Vrai si ce code sait à la fois **nommer** la clé d'exclusion de V1 et **parler** à SQLite.
 *
 * C'est le contournement trouvé en relecture de la PR #8, et il faut le dire en entier pour
 * qu'on ne « simplifie » pas ce test : la valeur de `CLE_V1_EXCLUSIONS` *est* le nom de la
 * table que le détecteur ci-dessous cherche à la lettre. Un fichier qui tient la constante et
 * un `execute` peut donc écrire `DELETE FROM {CLE_V1_EXCLUSIONS}` — effacer la liste des
 * pistes supprimées, donc violer INV-06 🔴 — **sans qu'un seul caractère de `excluded_paths`
 * apparaisse** dans le dépôt.
 *
 * Le verrou principal est le compilateur : la constante est `pub(crate)`, donc elle ne sort
 * plus du crate (test ci-dessous). Celui-ci est la seconde ligne, à l'intérieur du crate.
 */
export function nommeLaCleEtParleASqlite(code: string): boolean {
  const propre = sansCommentaires(code);
  return /\bCLE_V1_EXCLUSIONS\b/.test(propre) && SQL.test(propre);
}

/**
 * Vrai si ce code appelle `lignes::inserer`, la seule écriture de la table des pistes.
 *
 * `inserer` a dû devenir `pub(super)` le jour où `repository.rs` a frôlé les 400 lignes et
 * que les écritures de ligne sont parties dans `lignes.rs` : la surface d'écriture de
 * `tracks` s'est élargie d'un fichier à tout le module `library`. Relevé en relecture. Un
 * deuxième écrivain ne violerait pas INV-06 par lui-même — mais il contournerait les
 * décisions que `repository.rs` prend AVANT d'écrire, dont le filtre d'exclusion.
 */
export function appelleInserer(code: string): boolean {
  return /\binserer\s*\(/.test(sansCommentaires(code));
}

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

  it('la clé d’exclusion de V1 ne sort pas du crate', () => {
    // `pub` et non `pub(crate)` suffirait à rouvrir le contournement décrit plus haut, et
    // c'est une modification d'un mot que personne ne relirait comme un danger.
    const declarations = [...lire(DEPOT_RS).matchAll(/^(pub(?:\(crate\))?) const CLE_V1_\w+/gm)];
    expect(declarations.length, 'les deux clés du config.json de V1 sont déclarées ici').toBe(2);
    expect(
      declarations.map((m) => m[1]),
      'exportées hors du crate, ces clés rendent le détecteur d’INV-06 contournable',
    ).toEqual(['pub(crate)', 'pub(crate)']);
  });

  it('aucun autre fichier ne nomme cette clé en parlant à SQLite', () => {
    const fautes = RUST()
      .filter(([f, c]) => f !== DEPOT_RS && nommeLaCleEtParleASqlite(c))
      .map(([f]) => f);
    expect(fautes, `nommer la clé ET exécuter du SQL, c’est nommer la table`).toEqual([]);
  });

  it('le détecteur voit le contournement, et pas la simple lecture de la clé', () => {
    const faute = 'conn.execute(&format!("DELETE FROM {CLE_V1_EXCLUSIONS}"), []).ok();';
    expect(nommeLaCleEtParleASqlite(faute)).toBe(true);
    // `reprise_v1.rs` nomme la clé pour lire un JSON, et ne touche jamais à SQLite : c'est
    // exactement ce que ce détecteur doit laisser passer.
    expect(nommeLaCleEtParleASqlite('liste(&lu, CLE_V1_EXCLUSIONS)')).toBe(false);
    expect(nommeLaCleEtParleASqlite('conn.execute("DELETE FROM racines", []).ok();')).toBe(false);
  });

  it('seul le dépôt écrit dans la table des pistes', () => {
    const fautes = RUST()
      .filter(([f]) => f !== DEPOT_RS && f !== LIGNES_RS)
      .filter(([, c]) => appelleInserer(c))
      .map(([f]) => f);
    expect(fautes, `l’écriture d’une piste passe par ${DEPOT_RS}, qui applique ses règles d’abord`)
      .toEqual([]);
  });

  it('le détecteur d’écriture voit un appel et pas une phrase', () => {
    expect(appelleInserer('inserer(&tx, &lues.track, &lues.cle, t)?;')).toBe(true);
    expect(appelleInserer('    // on pourrait inserer (ici) sans passer par le dépôt')).toBe(false);
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
