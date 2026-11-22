import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Tauri lance `vite` lui-même : port fixe, pas de clear screen (sinon les erreurs Rust
// disparaissent de la console), et `envPrefix` pour que seules les vars HEAN_* fuitent
// dans le bundle.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  envPrefix: ['VITE_', 'HEAN_'],
  // Sans ça, Vite scanne les .html de `bootstrap/` (les références de V1, conservées pour la
  // parité) comme s'ils étaient des points d'entrée de l'app, et avertit sur leurs imports.
  optimizeDeps: { entries: ['index.html'] },
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  build: {
    // macOS 11+ seulement (ADR 0002) : la WebView est WebKit, pas besoin de transpiler plus bas.
    target: 'safari15',
    // Vite 8 minifie avec oxc ; demander esbuild obligerait à l'installer en plus, pour rien.
    sourcemap: true,
  },
});
