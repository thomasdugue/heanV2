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
 * Les mots qui désignent un identifiant, **dans les deux langues du dépôt**.
 *
 * ⚠️ **La liste n'était qu'en anglais jusqu'au 2026-11-25, et c'est ce qui a failli coûter
 * la règle.** Le détecteur a été écrit en phase 0, contre la ligne de V1
 * (`retrieve_password().unwrap_or_default()`), dans un projet dont tout le code est en
 * français. Quand le vrai module du trousseau est arrivé — `network/identifiants.rs`,
 * `mot_de_passe()`, `lire_du_trousseau()` — **aucun de ses appels n'était vu** :
 *
 * | ligne réaliste | vue avant le 2026-11-25 ? |
 * |---|---|
 * | `let mot_de_passe = identifiants::mot_de_passe(&source.id).unwrap_or_default();` | **non** |
 * | `lire_du_trousseau(service, compte).unwrap_or_default()` | **non** |
 * | `retrieve_password().unwrap_or_default()` (V1, et l'auto-test) | oui |
 *
 * Pire : l'assertion de matière réelle ajoutée le même jour **passait quand même**, parce
 * qu'elle injectait la faute sur `security_framework::passwords::get_generic_password` —
 * dont le nom **anglais** contient `password`. Elle prouvait donc que le détecteur voit
 * l'API tierce, pas qu'il voit **les appelants**, qui sont le site réaliste de la faute.
 * Relevé par `invariant-guardian`. La leçon tient en une phrase : **un détecteur de mots
 * doit parler la langue du code qu'il lit**, et son auto-test doit porter sur la ligne que
 * quelqu'un écrirait vraiment.
 */
const MOTS_DIDENTIFIANT = /password|keychain|credential|secret|mot_de_passe|trousseau|identifiant/i;

/**
 * INV-04 — un échec Keychain doit remonter une erreur, jamais un mot de passe vide.
 * `unwrap_or_default()` sur un identifiant est l'erreur exacte de V1 : elle fait tenter au
 * NAS une connexion avec un mot de passe vide, donc elle risque le verrouillage du compte.
 */
export function defautsSilencieuxSurIdentifiants(rust: string): string[] {
  const code = sansCommentaires(rust);
  return code
    .split('\n')
    .filter((l) => /unwrap_or_default\s*\(\s*\)/.test(l) && MOTS_DIDENTIFIANT.test(l))
    .map((l) => l.trim());
}
