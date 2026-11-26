<script lang="ts">
  // Les sources réseau : les voir, en ajouter une, saisir un mot de passe, scanner.
  // Nu, comme le reste de la coquille — le design est la phase 4 (ADR 0004).
  //
  // Ce que cet écran doit dire et que les autres n'ont pas à dire : **un scan réseau dure
  // des dizaines de minutes**. Il lit l'en-tête de chaque fichier : 36 min mesurées le
  // 2026-11-25 sur les 3 116 pistes trouvées, là où l'ADR 0016 §3 avait estimé ~2 min. Le
  // coût est la latence — ~690 ms par fichier mesurés contre ~41 ms estimés — et non le
  // débit. Une attente annoncée est une attente ; une attente non annoncée est une panne.
  import type { RapportDeScan, SourceReseau } from '../../ipc/client';
  import { compte } from './affichage';

  let {
    sources,
    rapport,
    occupe,
    enCours,
    surAjout,
    surMotDePasse,
    surScan,
  }: {
    sources: readonly SourceReseau[];
    /** Le rapport du dernier scan réseau, ou `null` si aucun n'a eu lieu dans cette session. */
    rapport: RapportDeScan | null;
    occupe: boolean;
    /** L'identifiant de la source en cours de scan, ou `null`. Sert à dire *laquelle*. */
    enCours: string | null;
    surAjout: (nom: string, hote: string, partage: string, cheminDistant: string, utilisateur: string) => void;
    surMotDePasse: (sourceId: string, motDePasse: string) => void;
    surScan: (sourceId: string) => void;
  } = $props();

  let nom = $state('');
  let hote = $state('');
  let partage = $state('');
  let cheminDistant = $state('');
  let utilisateur = $state('');

  /** Le mot de passe saisi, par source. Il descend et ne remonte jamais (ADR 0015). */
  let motsDePasse = $state<Record<string, string>>({});

  const pretAAjouter = $derived(hote.trim() !== '' && partage.trim() !== '');

  function ajouter(): void {
    if (!pretAAjouter) return;
    surAjout(nom.trim(), hote.trim(), partage.trim(), cheminDistant.trim() || '/', utilisateur.trim());
    nom = '';
    hote = '';
    partage = '';
    cheminDistant = '';
    utilisateur = '';
  }

  function envoyerLeMotDePasse(id: string): void {
    const mdp = motsDePasse[id] ?? '';
    if (mdp === '') return;
    surMotDePasse(id, mdp);
    // Effacé de l'état de la vue dès qu'il est parti : il n'a aucune raison d'y rester.
    motsDePasse = { ...motsDePasse, [id]: '' };
  }
</script>

<section>
  <h2>NAS et sources réseau</h2>

  {#if sources.length === 0}
    <p>Aucune source réseau. Si tu en avais une dans la version précédente, elle est reprise
      au premier lancement ; sinon, renseigne-la ci-dessous.</p>
  {:else}
    <ul>
      {#each sources as source (source.id)}
        <li>
          <strong>{source.nom}</strong> — smb://{source.hote}/{source.partage}{source.chemin_distant}
          · utilisateur {source.utilisateur === '' ? 'invité' : source.utilisateur}
          {#if !source.active}· <em>désactivée</em>{/if}
          <br />
          <label for={`mdp-${source.id}`}>Mot de passe</label>
          <input
            id={`mdp-${source.id}`}
            type="password"
            size="20"
            autocomplete="off"
            value={motsDePasse[source.id] ?? ''}
            oninput={(e) => (motsDePasse = { ...motsDePasse, [source.id]: e.currentTarget.value })}
          />
          <button
            type="button"
            onclick={() => envoyerLeMotDePasse(source.id)}
            disabled={occupe || (motsDePasse[source.id] ?? '') === ''}>Enregistrer</button
          >
          <button type="button" onclick={() => surScan(source.id)} disabled={occupe || !source.active}>
            Scanner cette source
          </button>
          {#if enCours === source.id}
            <!-- Dire LAQUELLE est en cours, et combien de temps ça prend. Sans ça, une
                 fenêtre qui ne bouge pas pendant tout ce temps ressemble à une fenêtre
                 bloquée — et c'est un rapport de bug pour rien. -->
            <!-- La durée est **mesurée**, pas estimée : voir docs/HISTORY.md, scan réel du
                 2026-11-25 sur le NAS de Thomas. Une estimation optimiste dans une interface
                 est pire que pas de chiffre — elle fait croire à une panne. -->
            <em>Scan en cours… le premier scan lit chaque fichier sur le NAS, et c'est long :
              <strong>compte une bonne demi-heure pour 3 000 pistes</strong> — mesuré, pas
              estimé. Les suivants ne relisent rien et prennent quelques secondes. Tu peux
              continuer à te servir de la fenêtre pendant ce temps.</em>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <p>
    <label for="hote">Ajouter une source</label>
    <input id="hote" type="text" size="18" placeholder="hôte (mon-nas.local)" bind:value={hote} />
    <input type="text" size="10" placeholder="partage" bind:value={partage} aria-label="partage" />
    <input type="text" size="18" placeholder="/dossier (optionnel)" bind:value={cheminDistant} aria-label="dossier distant" />
    <input type="text" size="12" placeholder="utilisateur" bind:value={utilisateur} aria-label="utilisateur" />
    <input type="text" size="12" placeholder="nom affiché (optionnel)" bind:value={nom} aria-label="nom affiché" />
    <button type="button" onclick={ajouter} disabled={occupe || !pretAAjouter}>Ajouter</button>
  </p>

  {#if rapport}
    <!-- Même forme que le rapport d'un scan de disque, et le même raisonnement : « 0
         ajoutée » est une information, l'absence de message serait une ambiguïté. -->
    <p>
      Scan réseau terminé : {compte(rapport.ajoutees, 'piste ajoutée', 'pistes ajoutées')},
      {compte(rapport.modifiees, 'modifiée', 'modifiées')},
      {compte(rapport.disparues, 'disparue', 'disparues')},
      {compte(rapport.illisibles, 'fichier illisible', 'fichiers illisibles')}.
    </p>
  {/if}
</section>
