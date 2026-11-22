// Svelte 5, runes OBLIGATOIRES (ADR 0002). Les stores Svelte 4 (`writable`, `$:`) sont
// interdits : un test d'architecture (tests/architecture/svelte5-runes.test.ts) le vérifie.
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
  compilerOptions: { runes: true },
};
