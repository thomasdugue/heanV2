<script lang="ts">
  // LA COQUILLE VISIBLE. Son seul but : que Thomas ouvre une fenêtre et voie sa vraie
  // bibliothèque, lue par le code de la phase 2. Rien d'autre.
  //
  // Elle est LAIDE EXPRÈS. Pas de tokens, pas de polices, pas de couleurs : le design est
  // la phase 4, et la cible de parité est le redesign v3 (ADR 0004). Toute mise en forme
  // écrite ici serait du travail à jeter — et pire, elle donnerait l'impression que le
  // design est « commencé » alors qu'il ne l'est pas.
  //
  // ELLE JOUE, depuis le 2026-11-26. Un clic sur une piste lance l'album entier à partir
  // d'elle, par le backend CoreAudio. Ce qui reste hors de portée est nommé, et ce n'est pas
  // un oubli : **une piste du NAS ne se joue pas encore** (il faudrait toucher à
  // `src-tauri/src/network/`, hors du périmètre de la session — ADR 0013), et l'égaliseur,
  // les touches media et le plein écran ne sont pas portés.
  import {
    appVersion,
    playbackSeek,
    playbackSetPaused,
    playbackSkip,
    playbackState,
    playbackStop,
    playTracks,
    refreshAudioDevices,
    setAudioDevice,
    setExclusiveMode,
    setVolume,
    type EtatLecture,
    type RapportDeScan,
    type SourceReseau,
    type Track,
  } from './ipc/client';
  // `bibliotheque` vient du dépôt et pas de `etat.ts` : le dépôt est LE point d'entrée du
  // frontend sur la bibliothèque, et il réexporte l'état pour ça.
  import { bibliotheque, creerDepot } from './lib/library/repository';
  import Dossiers from './lib/ui/Dossiers.svelte';
  import Lecture from './lib/ui/Lecture.svelte';
  import Pistes from './lib/ui/Pistes.svelte';
  import Reseau from './lib/ui/Reseau.svelte';
  import { compte, duree, qualite } from './lib/ui/affichage';

  const depot = creerDepot();

  // `$state.raw` et non `$state` : dix mille pistes enveloppées dans des signaux coûteraient
  // de la mémoire et du temps à chaque mutation (docs/PERFORMANCE.md). La liste est
  // remplacée en bloc, jamais modifiée piste par piste, donc la réactivité fine ne sert à
  // rien ici.
  let pistes = $state.raw<readonly Track[]>([]);
  let dossiers = $state.raw<readonly string[]>([]);
  let choisie = $state<Track | null>(null);
  let rapport = $state<RapportDeScan | null>(null);
  // 91 % de la bibliothèque de Thomas est sur son NAS (docs/MESURES-V1.md §1) : ces trois
  // états-ci ne sont pas une annexe de l'écran, ils en sont la plus grande part.
  let sources = $state.raw<readonly SourceReseau[]>([]);
  let rapportReseau = $state<RapportDeScan | null>(null);
  let scanEnCours = $state<string | null>(null);
  // L'état de la lecture est SONDÉ, pas poussé : `playback_state` toutes les 100 ms. Voir
  // `src-tauri/src/commands/lecture.rs` pour le pourquoi. Conséquence agréable : il n'y a
  // aucun état de lecture local à garder en phase avec le backend, donc rien à désynchroniser.
  let lecture = $state<EtatLecture | null>(null);
  let version = $state<string | null>(null);
  let erreur = $state<string | null>(null);
  let occupe = $state(true);

  /** Lit l'état partagé du dépôt et le recopie dans l'état local de la vue. */
  function relire(): void {
    pistes = bibliotheque.pistes.slice();
  }

  /**
   * Toute action passe par ici. Une seule écriture de `erreur` dans tout le composant :
   * en V1, chaque appel avait son propre `catch`, et plusieurs n'en avaient aucun — une
   * commande qui échouait ne laissait aucune trace à l'écran.
   */
  async function tenter(action: () => Promise<void>): Promise<void> {
    occupe = true;
    erreur = null;
    try {
      await action();
    } catch (e: unknown) {
      erreur = String(e);
    } finally {
      occupe = false;
    }
  }

  async function demarrer(): Promise<void> {
    version = await appVersion();
    await refreshAudioDevices();
    await depot.charger();
    dossiers = await depot.dossiers();
    sources = await depot.sourcesReseau();
  }

  // Un seul passage : ce bloc ne LIT aucun état réactif, donc il ne se redéclenche pas.
  $effect(() => {
    void tenter(demarrer);
  });

  // Le sondage de l'état de lecture. 100 ms : assez pour que la position avance de façon
  // lisible, assez peu pour ne rien coûter. Il ne passe PAS par `tenter` — une erreur de
  // sondage ne doit pas effacer le message affiché ni allumer l'indicateur d'occupation dix
  // fois par seconde. L'`$effect` rend la fonction d'arrêt, que Svelte appelle à la
  // destruction du composant.
  $effect(() => {
    let tic = 0;
    const relever = async (): Promise<void> => {
      try {
        // UN seul aller-retour : l'état porte la liste des sorties avec lui.
        lecture = await playbackState();
        // Toutes les 2 s, on demande au moteur de RELIRE le matériel. Sans ça, brancher le
        // DAC pendant que l'app tourne ne le ferait pas apparaître dans la liste — c'est
        // l'étape 10 du protocole de test, et elle échouerait.
        if (tic % 20 === 0) await refreshAudioDevices();
        tic += 1;
      } catch {
        // Le backend n'a pas répondu : on garde le dernier état connu plutôt que de vider
        // l'affichage. Une barre qui saute à zéro sur un sondage raté serait un faux signal.
      }
    };
    void relever();
    const minuteur = setInterval(() => void relever(), 100);
    return () => clearInterval(minuteur);
  });

  // V012 vu d'ici : la vue s'abonne et se recopie, sans condition. L'`$effect` rend la
  // fonction de désabonnement, que Svelte appelle à la destruction du composant.
  $effect(() => depot.surChangement(relire));

  /**
   * Un ordre de lecture. Les commandes générées rendent `Promise<null>` et `tenter` attend
   * `Promise<void>` : l'adaptation se fait ici une fois, et pas à chaque appel.
   */
  function ordre(action: () => Promise<unknown>): void {
    void tenter(async () => {
      await action();
    });
  }

  /**
   * Un clic sur une piste joue **l'album à partir d'elle**, et pas la piste seule : c'est
   * l'enchaînement sans trou qui est le jalon de Thomas, et il ne se vérifie qu'avec une
   * file. La file est la liste affichée, dans l'ordre affiché.
   *
   * Les pistes du NAS sont écartées ici : leur lecture demande de toucher au module réseau,
   * hors du périmètre de cette session. Les laisser passer donnerait « fichier illisible »,
   * une erreur juste pour une mauvaise raison.
   */
  function jouer(piste: Track): void {
    choisie = piste;
    const locales = pistes.filter((p) => !p.path.startsWith('smb://'));
    const depart = locales.findIndex((p) => p.path === piste.path);
    if (depart < 0) {
      erreur = 'Les pistes du NAS ne se jouent pas encore : la lecture réseau reste à porter.';
      return;
    }
    ordre(() => playTracks(locales.map((p) => p.path), depart));
  }

  function lectureOuPause(): void {
    ordre(() => playbackSetPaused(lecture?.en_pause !== true));
  }

  function ajouterUnDossier(chemin: string): void {
    void tenter(async () => {
      dossiers = await depot.ajouterUnDossier(chemin);
    });
  }

  function scanner(): void {
    void tenter(async () => {
      rapport = await depot.scanner();
    });
  }

  function ajouterUneSource(
    nom: string,
    hote: string,
    partage: string,
    cheminDistant: string,
    utilisateur: string,
  ): void {
    void tenter(async () => {
      sources = await depot.ajouterUneSourceReseau(nom, hote, partage, cheminDistant, utilisateur);
    });
  }

  function enregistrerLeMotDePasse(sourceId: string, motDePasse: string): void {
    void tenter(() => depot.enregistrerLeMotDePasse(sourceId, motDePasse));
  }

  /**
   * Un scan réseau dure des minutes. `scanEnCours` existe pour que la fenêtre dise
   * **laquelle** est en cours, et le `finally` pour que ce soit vrai même quand le scan
   * échoue — un indicateur qui reste allumé après une panne est pire que pas d'indicateur.
   */
  function scannerUneSource(sourceId: string): void {
    scanEnCours = sourceId;
    void tenter(async () => {
      try {
        rapportReseau = await depot.scannerUneSourceReseau(sourceId);
      } finally {
        scanEnCours = null;
      }
    });
  }
</script>

<main>
  <h1>Hean — coquille de phase 3</h1>

  {#if erreur}
    <!-- L'erreur est affichée telle quelle, sans traduction : c'est elle qu'on colle dans
         un rapport de bug, et la reformuler ferait perdre ce qui sert à diagnostiquer. -->
    <p>Erreur : {erreur}</p>
  {/if}

  <Dossiers {dossiers} {rapport} {occupe} surAjout={ajouterUnDossier} surScan={scanner} />

  <Reseau
    {sources}
    rapport={rapportReseau}
    {occupe}
    enCours={scanEnCours}
    surAjout={ajouterUneSource}
    surMotDePasse={enregistrerLeMotDePasse}
    surScan={scannerUneSource}
  />

  <Lecture
    etat={lecture}
    surLectureOuPause={lectureOuPause}
    surArret={() => ordre(() => playbackStop())}
    surPrecedente={() => ordre(() => playbackSkip(-1))}
    surSuivante={() => ordre(() => playbackSkip(1))}
    surDeplacement={(ms) => ordre(() => playbackSeek(ms))}
    surVolume={(v) => ordre(() => setVolume(v))}
    surPeripherique={(id) => ordre(() => setAudioDevice(id))}
    surModeExclusif={(actif) => ordre(() => setExclusiveMode(actif))}
  />

  {#if choisie}
    <section>
      <h2>Piste choisie</h2>
      <p>
        <strong>{choisie.titre}</strong> — {choisie.artiste} · {choisie.album} ·
        {duree(choisie.duree_ms)} · {choisie.format}{qualite(choisie.bits, choisie.echantillonnage) === ''
          ? ''
          : ` · ${qualite(choisie.bits, choisie.echantillonnage)}`}
      </p>
      <p>{choisie.path}</p>
    </section>
  {/if}

  <section>
    <h2>Bibliothèque</h2>
    {#if occupe && pistes.length === 0}
      <p>Lecture de la bibliothèque…</p>
    {:else if pistes.length === 0}
      <p>La bibliothèque est vide. Ajoute un dossier ci-dessus, puis scanne.</p>
    {:else}
      <p>{compte(pistes.length, 'piste')}.</p>
      <Pistes {pistes} choisie={choisie?.path ?? null} surChoix={jouer} />
    {/if}
  </section>

  <footer><p>socle v{version ?? '…'}</p></footer>
</main>

<style>
  /* LE MINIMUM VITAL, et rien de plus. Deux variables et une police système : pas un
     espacement choisi, pas une couleur inventée, aucune valeur à jeter en phase 4.

     Pourquoi passer par les variables du socle plutôt que de ne rien déclarer du tout : la
     fenêtre est `transparent` avec barre de titre en surimpression (tauri.conf.json), donc
     sans fond opaque on voit le bureau au travers — et une couleur écrite en dur est
     refusée par `tests/architecture/css-couleurs-en-dur.test.ts`. Les couleurs système
     (`Canvas`/`CanvasText`) feraient l'affaire, mais elles videraient le corpus d'INV-20 :
     ce sont aujourd'hui les seules lectures de variable CSS de tout le projet, et le
     détecteur qui tient la règle « la plus rentable de la liste » cesserait de surveiller
     quoi que ce soit, en silence. On ne désarme pas un garde-fou pour une coquille
     jetable.

     Le retrait du haut existe pour que les premières lignes ne passent pas sous les boutons
     de fenêtre. */
  main {
    min-height: 100vh;
    padding: 2.5rem 1rem 1rem;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: system-ui;
  }
</style>
