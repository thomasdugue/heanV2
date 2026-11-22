import { defineConfig } from 'vitest/config';

// Tests d'architecture + unitaires. Ce sont ceux qui DOIVENT être verts.
// Les 24 comportements UX sont dans vitest.ux.config.ts : ils sont rouges par construction
// en phase 0 (tableau de bord du projet), et `./check` ne les compte pas dans son verdict.
//
// Quand les premiers composants Svelte arriveront (phase 3), ajouter ici le plugin
// @sveltejs/vite-plugin-svelte et l'environnement jsdom pour tests/unit.
export default defineConfig({
  test: {
    environment: 'node',
    // `tests/ux/contrat-complet.test.ts` est ici EN PLUS d'être dans la suite UX, et c'est
    // un correctif de phase 2 : c'est le test qui empêche le tableau de bord de mentir
    // (aucun comportement perdu, aucun test orphelin), et il ne tournait que dans la suite
    // UX — donc hors du verdict de `./check`. Le garde-fou du tableau de bord était lui-même
    // hors garde-fou : un comportement supprimé par mégarde aurait fait baisser le compte
    // sans rendre `./check` rouge.
    include: [
      'tests/architecture/**/*.test.ts',
      'tests/unit/**/*.test.ts',
      'tests/ux/contrat-complet.test.ts',
    ],
    reporters: ['default'],
  },
});
