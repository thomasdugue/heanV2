import { defineConfig } from 'vitest/config';

// Les 24 comportements de bootstrap/01-produit/UX-ANTI-REGRESSION.md.
// Ils échouent tous aujourd'hui, et c'est VOULU : chacun est une feature à porter.
// Le compte vert/rouge est le tableau de bord de la reconstruction.
export default defineConfig({
  test: {
    environment: 'node',
    include: ['tests/ux/**/*.test.ts'],
    reporters: ['default'],
  },
});
