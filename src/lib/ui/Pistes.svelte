<script lang="ts">
  // La liste des pistes, en tableau. DÉLIBÉRÉMENT NUE : aucune couleur, aucune police,
  // aucun espacement travaillé. Le vrai design est la phase 4, et styliser ici serait du
  // travail à jeter. Un `<table>` aligne ses colonnes tout seul, sans une ligne de CSS.
  //
  // Le titre est un `<button>` plutôt qu'une ligne cliquable : c'est ce qui donne le
  // clavier, le focus et le rôle d'accessibilité gratuitement — une `<tr>` à laquelle on
  // colle un `onclick` ne les a pas, et `svelte-check --fail-on-warnings` le refuse.
  //
  // W006 — les tags ne sont pas échappés à la main. Un titre contenant `"` ou `<` est du
  // texte pour Svelte, qui l'échappe en compilant. En V1, `escapeHtml()` oubliait les
  // guillemets et tronquait le tag à la sauvegarde.
  import type { Track } from '../../ipc/client';
  import { duree, qualite } from './affichage';

  let {
    pistes,
    choisie,
    surChoix,
  }: {
    pistes: readonly Track[];
    /** Le chemin de la piste choisie, ou `null`. Le chemin EST l'identité d'une piste. */
    choisie: string | null;
    surChoix: (piste: Track) => void;
  } = $props();
</script>

<table>
  <thead>
    <tr>
      <th></th>
      <th>Titre</th>
      <th>Artiste</th>
      <th>Album</th>
      <th>Durée</th>
      <th>Format</th>
      <th>Qualité</th>
    </tr>
  </thead>
  <tbody>
    <!-- Clé = le chemin : l'identité d'une piste selon le backend (types.rs). Sans clé,
         Svelte réutiliserait les lignes par position et la sélection sauterait au scan. -->
    {#each pistes as piste (piste.path)}
      <tr>
        <td>{piste.path === choisie ? '▸' : ''}</td>
        <td><button type="button" onclick={() => surChoix(piste)}>{piste.titre}</button></td>
        <td>{piste.artiste}</td>
        <td>{piste.album}</td>
        <td>{duree(piste.duree_ms)}</td>
        <td>{piste.format}</td>
        <td>{qualite(piste.bits, piste.echantillonnage)}</td>
      </tr>
    {/each}
  </tbody>
</table>
