// Règles de forme du backend Rust.
import { describe, expect, it } from 'vitest';
import { sources } from './_lecture';
import { defautsSilencieuxSurIdentifiants, ecrituresDirectes, lecturesDeSecret } from './regles-backend';

const MODULE_DES_SECRETS = 'src-tauri/src/secrets.rs';
// `build.rs` est hors de src-tauri/src/ : ses `println!("cargo::…")` sont le protocole
// obligatoire de Cargo, pas de la journalisation. Il n'est donc jamais scanné.
const RUST = () => sources('src-tauri/src', ['.rs']);

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
