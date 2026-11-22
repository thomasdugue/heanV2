// Détecteurs CSS. Fonctions pures : chaque test les lance sur le vrai code ET sur un
// échantillon fautif, pour prouver que le détecteur détecte (un test d'architecture sur un
// dossier encore vide passerait sinon sans rien garantir).

/** Les variables déclarées : `--x: valeur;`, où qu'elles soient (`:root` ou portée locale). */
export function variablesDeclarees(css: string): Set<string> {
  return new Set([...css.matchAll(/(--[a-zA-Z0-9-]+)\s*:/g)].map((m) => m[1] as string));
}

/**
 * Les variables LUES sans valeur de repli : `var(--x)`.
 * `var(--x, 50px)` est tolérée — le repli rend la déclaration valide même sans définition.
 */
export function variablesLuesSansRepli(css: string): Set<string> {
  const out = new Set<string>();
  for (const m of css.matchAll(/var\(\s*(--[a-zA-Z0-9-]+)\s*([,)])/g)) {
    if (m[2] === ')') out.add(m[1] as string);
  }
  return out;
}

const MOTS_AUTORISES = /^(transparent|currentColor|inherit|initial|unset|none)$/;

/**
 * Les couleurs écrites en dur. Une couleur n'a le droit d'exister que dans le fichier de
 * tokens : sinon la dérive du design devient invisible.
 * `rgb(var(--x) ...)` est tolérée : c'est une composition à partir d'un token, pas une valeur.
 */
export function couleursEnDur(css: string): string[] {
  const out: string[] = [];
  for (const m of css.matchAll(/#[0-9a-fA-F]{3,8}\b/g)) out.push(m[0]);
  for (const m of css.matchAll(/\b(?:rgba?|hsla?|color-mix|oklch|lab)\(([^()]*(?:\([^()]*\)[^()]*)*)\)/g)) {
    if (!(m[1] ?? '').includes('var(')) out.push(m[0]);
  }
  for (const m of css.matchAll(/:\s*(white|black|red|blue|green|grey|gray|yellow|orange|purple|pink)\s*[;!]/gi)) {
    if (!MOTS_AUTORISES.test(m[1] as string)) out.push(m[1] as string);
  }
  return out;
}
