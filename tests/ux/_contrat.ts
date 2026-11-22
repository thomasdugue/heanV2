// Le contrat anti-régression : les 24 comportements de
// bootstrap/01-produit/UX-ANTI-REGRESSION.md.
//
// Chacun a été CASSÉ AU MOINS UNE FOIS en V1 puis corrigé. V2 repart de zéro, mais le
// comportement attendu reste le contrat — et la cause V1 sert à reconnaître le piège dans
// une autre implémentation.
//
// EN PHASE 0, LES 24 SONT ROUGES. C'est voulu : aucune feature n'existe encore. Le compte
// vert/rouge est le tableau de bord de la reconstruction, et `./check` l'affiche sans le
// compter dans son verdict.

export interface Comportement {
  /** ID de la spec V1 (section 13) ou ajouté après (W0xx), gardé pour la traçabilité. */
  readonly id: string;
  readonly attendu: string;
  readonly causeV1: string;
  /** Vrai si le piège vient de la plateforme : il reviendra quelle que soit la techno. */
  readonly plateforme: boolean;
  /** La phase de PLAN-DE-RECONSTRUCTION.md qui doit le faire passer au vert. */
  readonly phase: number;
}

const c = (
  id: string,
  attendu: string,
  causeV1: string,
  plateforme: boolean,
  phase: number,
): Comportement => ({ id, attendu, causeV1, plateforme, phase });

export const CONTRAT: readonly Comportement[] = [
  c('V001', 'La home s’affiche complète au démarrage, sans flash ni état partiel',
    'transitionView synchrone qui n’attendait pas le render async ; renders obsolètes non annulés', false, 3),
  c('V002', 'Recently Played : 3 colonnes ≥ 768 px, 2 entre 480 et 768, 1 en dessous, max 6 items',
    'width sans box-sizing: border-box', false, 3),
  c('V003', 'Les carousels s’étendent jusqu’au bord droit, la grille ne gonfle pas',
    'min-width: 0 manquant sur le conteneur flex → colonnes à 1492 px, grille à ~4500 px', true, 3),
  c('V004', 'Pas de rechargement de la bibliothèque si le scan n’a rien trouvé de nouveau',
    'scan_complete rechargeait toujours → race avec le premier render de la home', false, 2),
  c('V005', 'On peut glisser une piste vers une playlist',
    'le drag & drop HTML5 ne fonctionne pas dans la WebView (PLAT-4)', true, 3),
  c('V006', 'La vue Pistes reste à 60 fps avec 10 000+ pistes',
    'création/destruction de nœuds DOM à chaque frame', false, 3),
  c('V007', 'Les touches media contrôlent Hean, même si Apple Music tourne',
    'global shortcuts Tauri F7/F8/F9 interceptés en priorité par Apple Music (PLAT-3)', true, 1),
  c('V008', 'Une session AirPlay survit à un changement de device',
    'set_system_default_device ne réactive pas un device AirPlay stale (PLAT-1)', true, 1),
  c('V009', 'Aucun dialogue Keychain au lancement',
    'retrieve_password() appelé sans vérifier d’abord le cache de session', true, 5),
  c('V010', 'La section « loose tracks » d’une page artiste fait toute la largeur',
    'contrainte à 180 px par le repeat(auto-fill, 180px) du parent', true, 3),
  c('V011', 'La scrollbar des carousels est invisible au repos, visible au survol',
    'comportement voulu, facile à perdre', false, 3),
  c('V012', 'Après un bulk edit, toutes les vues se rafraîchissent',
    'condition if (includes(currentView)) qui excluait certaines vues', false, 2),
  c('V013', 'Les objets d’état partagés ne sont jamais réassignés',
    'library = {} cassait la référence vue par les autres modules', true, 2),
  c('V014', 'artist.albums se compte avec .length',
    'passé de Set à Array sans mettre à jour les appelants', true, 2),
  c('V015', 'Une métadonnée éditée survit au prochain démarrage',
    'METADATA_CACHE et TRACKS_CACHE désynchronisés', false, 2),
  c('V016', 'Sous 900 px la sidebar se réduit à 48 px',
    'style.width inline (du resize) écrasait la media query', false, 3),
  c('W001', 'Les vignettes de playlist de la sidebar s’affichent au premier démarrage',
    'covers chargées en async dans des cellules déconnectées par un rebuild concurrent', false, 3),
  c('W002', 'La mosaïque 28×28 n’a pas de cellule noire fantôme',
    'bug WebKit : dans un petit CSS Grid une cellule vide résout height: 100% contre le conteneur (PLAT-5). Corrigé 4 fois', true, 3),
  c('W003', 'Un album avec des artistes « feat. » différents n’apparaît qu’une fois dans une mosaïque',
    'déduplication par artist+album au lieu d’album seul', false, 3),
  c('W004', 'L’icône shuffle ne bouge pas quand on change de mode',
    'statut collé au texte du bouton + logique dupliquée avec un autre glyphe', false, 3),
  c('W005', 'L’UI ne gèle pas quand le NAS est injoignable',
    'get_cover / write_metadata faisaient de l’I/O SMB bloquante sur le thread de commande', false, 5),
  c('W006', 'Un tag contenant un guillemet n’est pas tronqué à la sauvegarde',
    'escapeHtml() n’échappait pas les guillemets', false, 2),
  c('W007', 'Une bibliothèque uniquement NAS s’affiche sur la home',
    'la home supposait au moins une source locale', false, 5),
  c('W008', 'Les fichiers temporaires SMB ne s’accumulent pas',
    'smb_buffer/ jamais nettoyé', false, 5),
];

const PAR_ID = new Map(CONTRAT.map((x) => [x.id, x]));

/**
 * Marque un comportement comme « pas encore porté ». Le test échoue avec, dans son message,
 * ce qui est attendu et le piège de V1 — pour que l'échec soit une consigne, pas un bruit.
 *
 * Quand la feature arrive : remplacer l'appel par la vraie assertion.
 */
export function aPorter(id: string): never {
  const x = PAR_ID.get(id);
  if (!x) throw new Error(`${id} ne figure pas dans le contrat anti-régression`);
  throw new Error(
    `${x.id} — pas encore porté (phase ${x.phase}).\n` +
      `  attendu  : ${x.attendu}\n` +
      `  piège V1 : ${x.causeV1}${x.plateforme ? '\n  ⚠ piège de plateforme : il reviendra quelle que soit la techno' : ''}`,
  );
}
