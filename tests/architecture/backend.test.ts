// Règles de forme du backend Rust.
import { describe, expect, it } from 'vitest';
import { sources } from './_lecture';
import { defautsSilencieuxSurIdentifiants, ecrituresDirectes, lecturesDeSecret } from './regles-backend';

const MODULE_DES_SECRETS = 'src-tauri/src/secrets.rs';
// `build.rs` reste hors scope : ses `println!("cargo::…")` sont le protocole obligatoire de
// Cargo, pas de la journalisation. Il n'est dans aucun des deux dossiers scannés.
//
// `src-tauri/tests/` EST scanné, et ça n'a pas toujours été le cas : tant que ce dossier ne
// contenait qu'un fichier, le trou ne se voyait pas. La phase 1 y a ajouté deux tests
// d'intégration, qui échappaient donc à une règle 🔴. Le garde-fou doit couvrir la règle, pas
// seulement l'endroit où on pensait qu'elle s'appliquait — sinon la règle n'est tenue que là
// où quelqu'un y a pensé, et c'est le modèle de défaut que V1 a payé le plus cher.
const RUST = () => [...sources('src-tauri/src', ['.rs']), ...sources('src-tauri/tests', ['.rs'])];

describe('INV-21 — rien n’écrit directement sur stdout', () => {
  it('aucun println!/eprintln!/dbg! dans le backend', () => {
    const fautes = RUST().flatMap(([f, c]) => ecrituresDirectes(c).map((m) => `${f} : ${m}`));
    expect(fautes, 'tout passe par tracing — V1 : 252 println! contre 3 tracing::').toEqual([]);
  });

  it('le détecteur repère les quatre macros', () => {
    expect(ecrituresDirectes('fn f() { println!("a"); eprintln!("b"); dbg!(1); }')).toEqual([
      'println!',
      'eprintln!',
      'dbg!',
    ]);
  });

  it('ne se déclenche pas sur un commentaire', () => {
    expect(ecrituresDirectes('// on n’utilise plus println! ici\nfn f() {}')).toEqual([]);
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
