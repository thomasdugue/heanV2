<script lang="ts">
  // Désigner un dossier de musique et le scanner. Nu, comme le reste de la coquille.
  //
  // Un champ texte et pas un sélecteur de fichiers macOS : le sélecteur natif demanderait
  // le plugin `tauri-plugin-dialog`, donc une dépendance et une permission de plus, pour
  // un écran qui sera refait en phase 4. Le vrai sélecteur arrivera avec le vrai design.
  import type { RapportDeScan } from '../../ipc/client';
  import { compte } from './affichage';

  let {
    dossiers,
    rapport,
    occupe,
    surAjout,
    surScan,
  }: {
    dossiers: readonly string[];
    /** Le rapport du dernier scan, ou `null` si aucun scan n'a eu lieu dans cette session. */
    rapport: RapportDeScan | null;
    occupe: boolean;
    surAjout: (chemin: string) => void;
    surScan: () => void;
  } = $props();

  let saisie = $state('');

  function ajouter(): void {
    const chemin = saisie.trim();
    if (chemin === '') return;
    surAjout(chemin);
    saisie = '';
  }
</script>

<section>
  <h2>Dossiers de musique</h2>

  {#if dossiers.length === 0}
    <p>Aucun dossier enregistré. Colle le chemin d’un dossier ci-dessous, puis scanne.</p>
  {:else}
    <ul>
      {#each dossiers as dossier (dossier)}
        <li>{dossier}</li>
      {/each}
    </ul>
  {/if}

  <p>
    <label for="chemin">Ajouter un dossier</label>
    <!-- `onkeydown` sur Entrée : la touche la plus naturelle après avoir collé un chemin. -->
    <input
      id="chemin"
      type="text"
      size="60"
      placeholder="/chemin/vers/ma/musique"
      bind:value={saisie}
      onkeydown={(e) => {
        if (e.key === 'Enter') ajouter();
      }}
    />
    <button type="button" onclick={ajouter} disabled={occupe || saisie.trim() === ''}>Ajouter</button>
    <button type="button" onclick={surScan} disabled={occupe || dossiers.length === 0}>Scanner</button>
  </p>

  {#if rapport}
    <!-- Le scan a eu lieu, et on le dit même quand il n'a rien trouvé : « 0 ajoutée » est
         une information, l'absence de message serait une ambiguïté. V004 vit un étage plus
         bas — c'est le dépôt qui décide de ne PAS recharger la liste si tout est à zéro. -->
    <p>
      Scan terminé : {compte(rapport.ajoutees, 'piste ajoutée', 'pistes ajoutées')},
      {compte(rapport.modifiees, 'modifiée', 'modifiées')},
      {compte(rapport.disparues, 'disparue', 'disparues')},
      {compte(rapport.illisibles, 'fichier illisible', 'fichiers illisibles')}.
    </p>
  {/if}
</section>
