// Détecteurs sur le backend Rust. Les tests d'architecture du backend vivent côté
// TypeScript parce que `./check` est le point d'entrée unique : une seule liste de règles,
// un seul endroit où la lire. (HARNAIS.md §2 autorise explicitement le grep à règles
// explicites plutôt qu'une analyse d'arbre.)
import { sansCommentaires } from './_lecture';

/** Les appels d'écriture directe sur stdout/stderr. Tout doit passer par `tracing`. */
export function ecrituresDirectes(rust: string): string[] {
  const code = sansCommentaires(rust);
  return [...code.matchAll(/\b(println!|eprintln!|print!|eprint!|dbg!)/g)].map((m) => m[1] as string);
}

/** Les lectures de secret compile-time. Elles n'ont droit de cité que dans `secrets.rs`. */
export function lecturesDeSecret(rust: string): string[] {
  return [...sansCommentaires(rust).matchAll(/\boption_env!\s*\(\s*"([^"]+)"/g)].map((m) => m[1] as string);
}

/**
 * INV-04 — un échec Keychain doit remonter une erreur, jamais un mot de passe vide.
 * `unwrap_or_default()` sur un identifiant est l'erreur exacte de V1 : elle fait tenter au
 * NAS une connexion avec un mot de passe vide, donc elle risque le verrouillage du compte.
 */
export function defautsSilencieuxSurIdentifiants(rust: string): string[] {
  const code = sansCommentaires(rust);
  return code
    .split('\n')
    .filter((l) => /unwrap_or_default\s*\(\s*\)/.test(l) && /password|keychain|credential|secret/i.test(l))
    .map((l) => l.trim());
}
