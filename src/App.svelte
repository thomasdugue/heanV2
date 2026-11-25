<script lang="ts">
  // LA COQUILLE VISIBLE. Son seul but : que Thomas ouvre une fenêtre et voie sa vraie
  // bibliothèque, lue par le code de la phase 2. Rien d'autre.
  //
  // Elle est LAIDE EXPRÈS. Pas de tokens, pas de polices, pas de couleurs : le design est
  // la phase 4, et la cible de parité est le redesign v3 (ADR 0004). Toute mise en forme
  // écrite ici serait du travail à jeter — et pire, elle donnerait l'impression que le
  // design est « commencé » alors qu'il ne l'est pas.
  //
  // Ce qu'elle ne fait pas : jouer. Le moteur de la phase 1 décode un fichier et sait
  // écrire un WAV, mais la sortie vers la carte son — le backend CoreAudio — n'est pas
  // écrite (voir `src-tauri/src/audio/mod.rs`, « ce que ce module ne contient pas encore »).
  // Un bouton qui passerait par `afplay` ferait sortir du son sans rien prouver du moteur ;
  // Thomas a tranché le 2026-11-23 : pas de son provisoire. Le clic choisit donc une piste
  // et montre ce que Hean jouera, et la fenêtre le dit en clair.
  import { appVersion, type RapportDeScan, type SourceReseau, type Track } from './ipc/client';
  // `bibliotheque` vient du dépôt et pas de `etat.ts` : le dépôt est LE point d'entrée du
  // frontend sur la bibliothèque, et il réexporte l'état pour ça.
  import { bibliotheque, creerDepot } from './lib/library/repository';
  import Dossiers from './lib/ui/Dossiers.svelte';
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
    await depot.charger();
    dossiers = await depot.dossiers();
    sources = await depot.sourcesReseau();
  }

  // Un seul passage : ce bloc ne LIT aucun état réactif, donc il ne se redéclenche pas.
  $effect(() => {
    void tenter(demarrer);
  });

  // V012 vu d'ici : la vue s'abonne et se recopie, sans condition. L'`$effect` rend la
  // fonction de désabonnement, que Svelte appelle à la destruction du composant.
  $effect(() => depot.surChangement(relire));

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

  <section>
    <h2>Lecture</h2>
    <!-- La phrase la plus importante de cette fenêtre : dire ce qui n'existe pas encore,
         pour que son absence ne soit pas prise pour une panne. -->
    <p>
      <strong>Le son n’est pas encore branché</strong> — et ce n’est pas un bug à signaler.
      Le moteur sait lire un fichier et en sortir les échantillons exacts, mais la dernière
      étape, l’envoi vers la carte son, reste à écrire. Un clic choisit une piste et montre
      ce que Hean jouera.
    </p>
    {#if choisie}
      <p>
        Choisi : <strong>{choisie.titre}</strong> — {choisie.artiste} · {choisie.album} ·
        {duree(choisie.duree_ms)} · {choisie.format}{qualite(choisie.bits, choisie.echantillonnage) === ''
          ? ''
          : ` · ${qualite(choisie.bits, choisie.echantillonnage)}`}
      </p>
      <p>{choisie.path}</p>
    {:else}
      <p>Aucune piste choisie.</p>
    {/if}
  </section>

  <section>
    <h2>Bibliothèque</h2>
    {#if occupe && pistes.length === 0}
      <p>Lecture de la bibliothèque…</p>
    {:else if pistes.length === 0}
      <p>La bibliothèque est vide. Ajoute un dossier ci-dessus, puis scanne.</p>
    {:else}
      <p>{compte(pistes.length, 'piste')}.</p>
      <Pistes {pistes} choisie={choisie?.path ?? null} surChoix={(p) => (choisie = p)} />
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
