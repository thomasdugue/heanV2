<script lang="ts">
  // LES COMMANDES DE LECTURE, et rien de plus. Laide exprès, comme le reste de la coquille :
  // le design est la phase 4 et la cible de parité est le redesign v3 (ADR 0004). Toute mise
  // en forme écrite ici serait du travail à jeter.
  //
  // Ce qu'elle montre et qui n'est PAS cosmétique, parce que ce sont les trois choses que
  // Thomas ne peut pas vérifier autrement :
  //   1. **les deux fréquences, et le badge bit-perfect** — Hean n'est pas bit-perfect en
  //      permanence : il l'est quand le DAC sait décoder, et quand il ne sait pas il replie
  //      et l'affiche. Un lecteur qui annoncerait « bit-perfect » sans condition mentirait
  //      dès qu'il y a repli ou qu'un AirPlay est choisi (PLAT-1) ;
  //   2. **le compteur de trous** — « ça ne coupe pas » n'est une promesse vérifiable que
  //      s'il y a un nombre en face. V1 n'en avait aucun ;
  //   3. **le message du moteur** — un refus de lecture doit se lire, pas finir dans un
  //      journal que personne n'ouvre (MESURES-V1.md §8).
  import type { EtatLecture } from '../../ipc/client';
  import { duree, frequence, specsSource } from './affichage';

  interface Props {
    etat: EtatLecture | null;
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

  /** Vrai quand l'AUHAL (ou macOS, en AirPlay) rééchantillonne. */
  const replie = $derived(etat !== null && etat.en_lecture && !etat.sans_reechantillonnage);

  /** Pourquoi la lecture n'est pas bit-perfect, en une phrase lisible. */
  const pourquoiPas = $derived.by(() => {
    if (!etat || !etat.en_lecture) return '';
    const manque: string[] = [];
    if (etat.airplay) manque.push('macOS rééchantillonne pour AirPlay');
    else if (!etat.sans_reechantillonnage) manque.push('la sortie ne sait pas jouer cette fréquence');
    if (etat.volume !== 1) manque.push('le volume n’est pas à 100 %');
    if (!etat.mode_exclusif) manque.push('le mode exclusif est coupé');
    return manque.join(', ');
  });

  /**
   * Combien de temps la visée garde la main après le relâchement, en millisecondes. Cadence
   * estimée et jamais mesurée : de quoi couvrir quelques sondages, le temps que le moteur
   * applique le saut et republie la position.
   */
  const VISEE_TENUE_MS = 300; // estimé, jamais mesuré — INV-28 veut l'origine sur cette ligne

  /**
   * La position que le doigt vise, pendant qu'il déplace la barre et juste après.
   *
   * SANS CE VERROU, LE DÉPLACEMENT EST IMPOSSIBLE À VISER : l'état est sondé dix fois par
   * seconde, et chaque relevé réécrit la valeur du curseur sous le doigt — le pouce pousse,
   * le sondage ramène, et le curseur retombe sur la position qui joue. C'est aussi ce qui
   * rend le défaut invisible à la relecture du curseur de volume juste en dessous, qui a la
   * même forme et marche : la valeur qu'on y relit est celle qu'on vient de poser, elle ne
   * bouge pas d'elle-même.
   *
   * `jusqua` est à l'infini tant que le doigt est posé, puis devient une échéance : le
   * moteur applique le saut en quelques millisecondes, mais un sondage peut tomber entre
   * l'envoi et l'application, et celui-là annoncerait encore l'ancienne position.
   */
  let visee = $state<{ cible: number; jusqua: number } | null>(null);

  // Une échéance relue à chaque sondage plutôt qu'un verrou à rendre : un `$effect` qui
  // lirait `visee` pour la remettre à zéro se redéclencherait lui-même.
  const positionAffichee = $derived.by(() => {
    const sondee = etat?.position_ms ?? 0;
    if (visee === null) return sondee;
    return Date.now() < visee.jusqua ? visee.cible : sondee;
  });
</script>

<section>
  <h2>Lecture</h2>

  {#if etat && etat.message !== ''}
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
      {duree(positionAffichee)} / {dureeConnue ? duree(etat.duree_ms) : 'durée inconnue'}
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
          value={positionAffichee}
          disabled={!dureeConnue}
          oninput={(e) => {
            visee = { cible: Number(e.currentTarget.value), jusqua: Number.POSITIVE_INFINITY };
          }}
          onchange={(e) => {
            const cible = Number(e.currentTarget.value);
            visee = { cible, jusqua: Date.now() + VISEE_TENUE_MS };
            surDeplacement(cible);
          }}
          onpointercancel={() => {
            visee = null;
          }}
        />
      </label>
    </p>
    {#if etat.piste !== ''}<p>{etat.piste}</p>{/if}
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
        {#each etat?.peripheriques ?? [] as p (p.id)}
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
    <!-- L'INDICATEUR. C'est la pièce qui rend le repli de fréquence honnête plutôt que
         caché : Hean n'est pas bit-perfect en permanence, il l'est quand le DAC sait
         décoder — et quand il ne sait pas, les deux fréquences sont écrites côte à côte.
         V1 faisait exactement ça (`playback.js:862-903`) : deux champs, source et sortie,
         avec une flèche et une couleur cyan quand il y a repli — délibérément pas rouge,
         parce que le repli fonctionne, ce n'est pas une panne. -->
    <p>
      Source : <strong>{specsSource(etat.source_hz, etat.source_bits)}</strong>
      &nbsp;·&nbsp; Sortie :
      <strong>{frequence(etat.sortie_hz)}{replie ? ' (rééchantillonné)' : ''}</strong>
    </p>
    <p>
      {#if etat.bit_perfect}
        <strong>Bit-perfect</strong> — aucun échantillon n’est modifié entre le fichier et le DAC.
      {:else}
        <strong>Pas bit-perfect</strong>{pourquoiPas ? ` : ${pourquoiPas}.` : '.'}
        {#if replie}
          Le son est joué, mais il est converti — ce n’est pas une panne.
        {/if}
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
