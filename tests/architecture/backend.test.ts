// Règles de forme du backend Rust.
import { describe, expect, it } from 'vitest';
import { fichiers, lire, sources } from './_lecture';
import { defautsSilencieuxSurIdentifiants, ecrituresDirectes, lecturesDeSecret } from './regles-backend';

const MODULE_DES_SECRETS = 'src-tauri/src/secrets.rs';

// `src-tauri/tests/` EST scanné, et ça n'a pas toujours été le cas : tant que ce dossier ne
// contenait qu'un fichier, le trou ne se voyait pas. La phase 1 y a ajouté deux tests
// d'intégration, qui échappaient donc à une règle 🔴. Le garde-fou doit couvrir la règle, pas
// seulement l'endroit où on pensait qu'elle s'appliquait — sinon la règle n'est tenue que là
// où quelqu'un y a pensé, et c'est le modèle de défaut que V1 a payé le plus cher.
const DOSSIERS_SCANNES = ['src-tauri/src', 'src-tauri/tests'] as const;
const RUST = () => DOSSIERS_SCANNES.flatMap((d) => sources(d, ['.rs']));

/**
 * Les dérogations à INV-21 — les seuls fichiers Rust du backend qui ont le droit d'écrire
 * directement sur stdout.
 *
 * **Elles étaient jusqu'ici une omission, pas une décision.** `RUST()` nomme deux dossiers ;
 * tout ce qui est ailleurs sous `src-tauri/` échappait à une règle 🔴 BLOQUANT sans que rien
 * ne l'écrive ni ne le compte. `build.rs` était hors scan **et** expliqué, ce qui était la
 * bonne forme ; `examples/` était hors scan et **expliqué nulle part**. Le test
 * « la liste est exacte » ci-dessous ferme l'écart : un nouveau `.rs` posé hors des dossiers
 * scannés fait rougir `./check` tant que sa dérogation n'est pas écrite ici. C'est la
 * différence entre une dérogation et un oubli.
 *
 * **Le texte d'INV-21 exige un ADR pour toute exception, et il y en a un** :
 * `docs/adr/0011-derogation-inv-21-pour-les-exemples.md`, tranché par Thomas le 2026-11-23.
 * Relevé par `invariant-guardian` : une exception écrite est *visible*, ce qui vaut mieux
 * qu'une omission — mais visible n'est pas *décidée*, et une exception à une règle 🔴 se
 * classe comme la règle. Lire cet ADR avant d'ajouter une entrée ci-dessous : il dit sur
 * quoi la dérogation tient (un exemple n'est pas dans le binaire livré, propriété de Cargo
 * et non bonne conduite de qui l'écrit), donc ce qu'une nouvelle entrée devrait prouver.
 */
/**
 * Un motif qui finit par `/` désigne un DOSSIER et couvre ce qu'il contient ; sinon il
 * désigne un fichier et ne couvre que lui, à l'exact. Relevé en relecture : un `startsWith`
 * pour tout le monde faisait couvrir `build.rs2.rs` par la dérogation de `build.rs`. Élargir
 * une dérogation est le début de la fin d'une règle, même par accident.
 */
const couvrePar = (fichier: string, motif: string): boolean =>
  motif.endsWith('/') ? fichier.startsWith(motif) : fichier === motif;

const HORS_SCAN: readonly { motif: string; pourquoi: string; muet?: string }[] = [
  {
    motif: 'src-tauri/build.rs',
    pourquoi:
      'ses println!("cargo::…") sont le protocole obligatoire de Cargo, pas de la journalisation : ' +
      'ce sont des directives lues par cargo, et aucun tracing ne peut les remplacer',
  },
  {
    motif: 'src-tauri/examples/',
    pourquoi:
      "un exemple se lance à la main en ligne de commande : sans sortie sur stdout il n'a aucun " +
      "intérêt, et il n'est pas dans le binaire livré, donc il ne peut pas rendre l'app aveugle " +
      'chez un testeur — ce que la règle protège. Même forme que build.rs',
    // Ce dossier n'existe pas sur cette branche : `examples/migrer.rs` arrive avec la
    // migration noir → hean. La dérogation est écrite d'avance, et la note ci-dessous se
    // périme d'elle-même — le jour où le dossier arrive, le second test ci-dessous rougit et
    // demande de la retirer. C'est voulu : une dérogation doit être confirmée quand elle
    // entre réellement en vigueur, pas héritée en silence.
    muet: "le dossier n'existe pas encore sur cette branche (migration noir → hean)",
  },
];

describe('INV-21 — rien n’écrit directement sur stdout', () => {
  it('aucun println!/eprintln!/dbg! dans le backend', () => {
    const fautes = RUST().flatMap(([f, c]) => ecrituresDirectes(c).map((m) => `${f} : ${m}`));
    expect(fautes, 'tout passe par tracing — V1 : 252 println! contre 3 tracing::').toEqual([]);
  });

  it('le détecteur repère les cinq macros', () => {
    // Le titre disait « quatre » pour trois assertions, et `ecrituresDirectes` en connaît
    // CINQ : `print!` et `eprint!` n'étaient éprouvés par rien. Défaut pré-existant, relevé
    // en relecture — et c'est une troisième variante de celui que cette session ferme : une
    // assertion qui couvre moins que ce qu'elle annonce.
    expect(
      ecrituresDirectes('fn f() { println!("a"); eprintln!("b"); print!("c"); eprint!("d"); dbg!(1); }'),
    ).toEqual(['println!', 'eprintln!', 'print!', 'eprint!', 'dbg!']);
  });

  it('ne se déclenche pas sur un commentaire', () => {
    expect(ecrituresDirectes('// on n’utilise plus println! ici\nfn f() {}')).toEqual([]);
  });

  it('le détecteur reconnaît les println! réels du projet', () => {
    // La règle est une interdiction : zéro trouvaille est son état sain, donc le test
    // principal ne peut pas prouver que le détecteur voit encore quelque chose. Son
    // auto-test, lui, ne prouve que ce qu'on a inventé pour lui — et c'est exactement ce
    // qui a laissé INV-06 désarmé pendant toute la phase 1, auto-test vert à l'appui.
    //
    // La matière réelle existe pourtant, dans le seul fichier qui a le droit d'en porter :
    // `build.rs` et ses directives Cargo. Le fichier mis hors scan devient donc la preuve
    // que la règle sait encore voir — et cette assertion se périme d'elle-même le jour où
    // `build.rs` n'écrira plus sur stdout.
    expect(ecrituresDirectes(lire('src-tauri/build.rs')).length).toBeGreaterThan(0);
  });

  it('la liste des exceptions est exacte : aucun .rs n’échappe au scan sans dérogation écrite', () => {
    const scannes = new Set(RUST().map(([f]) => f));
    const horsScan = fichiers('src-tauri', ['.rs']).filter((f) => !scannes.has(f));
    const sansDerogation = horsScan.filter((f) => !HORS_SCAN.some((e) => couvrePar(f, e.motif)));
    expect(
      sansDerogation,
      'ces fichiers Rust échappent à INV-21 par omission : les scanner, ou écrire la dérogation dans HORS_SCAN',
    ).toEqual([]);
  });

  it('chaque dérogation écrite est justifiée, et en vigueur ou dite muette', () => {
    const tous = fichiers('src-tauri', ['.rs']);
    const incoherentes = HORS_SCAN.filter((e) => {
      const existe = tous.some((f) => couvrePar(f, e.motif));
      return e.pourquoi.length < 40 || existe === (e.muet !== undefined);
    }).map((e) =>
      e.pourquoi.length < 40
        ? `${e.motif} : dérogation sans justification écrite`
        : e.muet === undefined
          ? `${e.motif} : ne correspond à aucun fichier — écrire pourquoi, ou retirer la dérogation`
          : `${e.motif} : la note « muet » est périmée, le fichier existe désormais — la retirer`,
    );
    expect(incoherentes).toEqual([]);
  });
});

describe('INV-17 — les secrets ont un seul point de lecture', () => {
  it('option_env! n’apparaît que dans secrets.rs', () => {
    const fautes = RUST()
      .filter(([f]) => f !== MODULE_DES_SECRETS)
      .flatMap(([f, c]) => lecturesDeSecret(c).map((s) => `${f} : ${s}`));
    expect(fautes, `tout secret se lit via ${MODULE_DES_SECRETS}`).toEqual([]);
  });

  it('secrets.rs lit bien les trois secrets attendus', () => {
    const attendus = ['HEAN_SENTRY_DSN', 'NOIR_WORKER_URL', 'NOIR_WORKER_SECRET'];
    const lus = lecturesDeSecret(RUST().find(([f]) => f === MODULE_DES_SECRETS)?.[1] ?? '');
    expect([...lus].sort()).toEqual([...attendus].sort());
  });
});

// ⚠️ CE DÉTECTEUR EST MUET AUJOURD'HUI, ET C'EST NORMAL : il n'y a pas une ligne de
// Keychain dans le projet avant la phase 5 (NAS/SMB). Lui demander de « trouver au moins N
// choses réelles » voudrait dire écrire la faute qu'il cherche. Ce qui le tient d'ici là :
// son corpus est garanti non vide par le lecteur partagé, et son auto-test rejoue la ligne
// exacte de V1. La phase 5 le rendra bavard — et c'est à ce moment-là qu'il faudra lui
// ajouter une assertion sur la matière réelle, c'est-à-dire sur le vrai appel au Keychain.
describe('INV-04 — un échec Keychain ne se dégrade jamais en mot de passe vide', () => {
  it('aucun unwrap_or_default() sur un identifiant', () => {
    const fautes = RUST().flatMap(([f, c]) =>
      defautsSilencieuxSurIdentifiants(c).map((l) => `${f} : ${l}`),
    );
    expect(fautes).toEqual([]);
  });

  it('le détecteur repère le cas de V1', () => {
    expect(defautsSilencieuxSurIdentifiants('let password = retrieve_password().unwrap_or_default();'))
      .toEqual(['let password = retrieve_password().unwrap_or_default();']);
  });
});
