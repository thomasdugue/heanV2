<script lang="ts">
  // LES COMMANDES DE LECTURE, et rien de plus. Laide exprès, comme le reste de la coquille :
  // le design est la phase 4 et la cible de parité est le redesign v3 (ADR 0004). Toute mise
  // en forme écrite ici serait du travail à jeter.
  //
  // Ce qu'elle montre et qui n'est PAS cosmétique, parce que ce sont les trois choses que
  // Thomas ne peut pas vérifier autrement :
  //   1. **bit-perfect ou non, et pourquoi pas** — un lecteur qui l'affiche sans condition
  //      mentirait dès qu'un AirPlay est choisi (ADR 0018) ;
  //   2. **le compteur de trous** — « ça ne coupe pas » n'est une promesse vérifiable que
  //      s'il y a un nombre en face. V1 n'en avait aucun ;
  //   3. **le message du moteur** — un refus de lecture doit se lire, pas finir dans un
  //      journal que personne n'ouvre (MESURES-V1.md §8).
  import type { EtatLecture, PeripheriqueAudio } from '../../ipc/client';
  import { duree } from './affichage';

  interface Props {
    etat: EtatLecture | null;
    peripheriques: readonly PeripheriqueAudio[];
    surLectureOuPause: () => void;
    surArret: () => void;
    surPrecedente: () => void;
    surSuivante: () => void;
    surDeplacement: (positionMs: number) => void;
    surVolume: (volume: number) => void;
    surPeripherique: (id: number) => void;
    surModeExclusif: (actif: boolean) => void;
  }

  const {
    etat,
    peripheriques,
    surLectureOuPause,
    surArret,
    surPrecedente,
    surSuivante,
    surDeplacement,
    surVolume,
    surPeripherique,
    surModeExclusif,
  }: Props = $props();

  // `$derived` et non une variable recalculée à la main : c'est la forme Svelte 5, et elle
  // rend impossible l'oubli de mise à jour qui a coûté V014 en V1.
  const joue = $derived(etat?.en_lecture === true);
  const dureeConnue = $derived((etat?.duree_ms ?? 0) > 0);

  /** Pourquoi la lecture n'est pas bit-perfect, en une phrase lisible. */
  const pourquoiPas = $derived.by(() => {
    if (!etat || !etat.en_lecture) return '';
    const manque: string[] = [];
    if (!etat.sans_reechantillonnage) manque.push('la sortie rééchantillonne');
    if (etat.volume !== 1) manque.push('le volume n’est pas à 100 %');
    if (!etat.mode_exclusif) manque.push('le mode exclusif est coupé');
    return manque.join(', ');
  });
</script>

<section>
  <h2>Lecture</h2>

  {#if etat?.message}
    <!-- Affiché tel quel : c'est le texte du moteur, et le reformuler ferait perdre la
         fréquence et le nom du périphérique, qui sont ce qui dit quoi faire. -->
    <p><strong>{etat.message}</strong></p>
  {/if}

  <p>
    <button onclick={surPrecedente} disabled={!joue}>⏮ précédente</button>
    <button onclick={surLectureOuPause} disabled={!joue}>
      {etat?.en_pause ? '▶ reprendre' : '⏸ pause'}
    </button>
    <button onclick={surSuivante} disabled={!joue}>suivante ⏭</button>
    <button onclick={surArret} disabled={!joue}>⏹ arrêter</button>
  </p>

  {#if etat && joue}
    <p>
      {duree(etat.position_ms)} / {dureeConnue ? duree(etat.duree_ms) : 'durée inconnue'}
      — piste {etat.index + 1} sur {etat.total}
    </p>
    <p>
      <label>
        position
        <!-- Désactivé quand la durée est inconnue : une barre sans échelle ne se déplace
             nulle part, et un curseur qui ne fait rien est pire qu'un curseur absent. -->
        <input
          type="range"
          min="0"
          max={Math.max(etat.duree_ms, 1)}
          step="1000"
          value={etat.position_ms}
          disabled={!dureeConnue}
          onchange={(e) => surDeplacement(Number(e.currentTarget.value))}
        />
      </label>
    </p>
    {#if etat.piste}<p>{etat.piste}</p>{/if}
  {:else}
    <p>Rien ne joue. Clique une piste de la bibliothèque pour lancer l’album.</p>
  {/if}

  <p>
    <label>
      volume
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={etat?.volume ?? 1}
        oninput={(e) => surVolume(Number(e.currentTarget.value))}
      />
    </label>
    {Math.round((etat?.volume ?? 1) * 100)} %
  </p>

  <p>
    <label>
      sortie
      <select
        value={String(etat?.peripherique_id ?? '')}
        onchange={(e) => surPeripherique(Number(e.currentTarget.value))}
      >
        {#each peripheriques as p (p.id)}
          <!-- L'AirPlay est annoncé AVANT le choix : macOS y rééchantillonne et on ne peut
               pas l'en empêcher (PLAT-1). Le découvrir après coup serait une surprise. -->
          <option value={String(p.id)}>
            {p.nom}{p.airplay ? ' (AirPlay — jamais bit-perfect)' : ''}{p.bluetooth
              ? ' (Bluetooth)'
              : ''}
          </option>
        {/each}
      </select>
    </label>
  </p>

  <p>
    <label>
      <input
        type="checkbox"
        checked={etat?.mode_exclusif ?? false}
        onchange={(e) => surModeExclusif(e.currentTarget.checked)}
      />
      mode exclusif (Hean réserve le DAC — impossible en AirPlay)
    </label>
  </p>

  {#if etat && joue}
    <p>
      {#if etat.bit_perfect}
        <strong>Bit-perfect</strong> — aucun échantillon n’est modifié entre le fichier et le DAC.
      {:else}
        <strong>Pas bit-perfect</strong>{pourquoiPas ? ` : ${pourquoiPas}.` : '.'}
      {/if}
    </p>
    <p>
      Trous dans le son depuis le début : <strong>{etat.famines}</strong>
      {#if etat.famines > 0}
        — ce n’est pas normal, et c’est à signaler.
      {/if}
    </p>
  {/if}
</section>
