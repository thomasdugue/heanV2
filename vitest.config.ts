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
    include: ['tests/architecture/**/*.test.ts', 'tests/unit/**/*.test.ts'],
    reporters: ['default'],
  },
});
