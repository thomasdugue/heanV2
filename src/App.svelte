<script lang="ts">
  // Socle de phase 0 : la coquille prouve que la chaîne Vite → Svelte 5 → Tauri → IPC
  // typée fonctionne de bout en bout. Aucune feature ici (voir FEATURE-SET.md).
  import { appVersion } from './ipc/client';

  let version = $state<string | null>(null);
  let erreur = $state<string | null>(null);

  $effect(() => {
    appVersion()
      .then((v) => (version = v))
      .catch((e: unknown) => (erreur = String(e)));
  });
</script>

<main>
  <h1>Hean</h1>
  {#if erreur}
    <p class="erreur">{erreur}</p>
  {:else if version}
    <p>socle v{version}</p>
  {:else}
    <p>…</p>
  {/if}
</main>

<style>
  main {
    display: grid;
    place-content: center;
    min-height: 100vh;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: var(--font-body);
  }
  h1 {
    font-size: var(--fs-hero);
  }
  .erreur {
    color: var(--color-error);
  }
</style>
