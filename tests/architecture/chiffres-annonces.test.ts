// INV-28 — une durée annoncée à l'utilisateur dit sur place si elle est mesurée ou estimée.
//
// Née le 2026-11-26, de trois cas en une soirée. L'interface annonçait « environ deux
// minutes » pour le premier scan réseau : un chiffre d'ordre de grandeur, écrit avant que
// le scan n'ait jamais tourné. Le vrai a duré **36 minutes** (ADR 0016 §3). Une attente
// annoncée vingt fois trop courte est pire que pas de chiffre du tout — c'est ce que le
// commentaire de `Reseau.svelte` dit lui-même, puis contredit deux lignes plus haut. Le
// même soir, l'ADR 0015 chiffrait le coût de l'option B à « une saisie de mot de passe »
// deux fois de suite, avant qu'un relevé du trousseau ne le mesure à **zéro**.
//
// Ce que ces trois cas ont en commun n'est pas l'erreur : c'est qu'**on ne pouvait pas
// savoir, en lisant, que le chiffre n'avait jamais été mesuré**. Un chiffre faux se
// corrige ; un chiffre dont l'origine est muette se recopie.
//
// LA FENÊTRE EST LA LIGNE, et c'est un choix — tenu par une assertion, parce qu'un choix
// argumenté en commentaire n'est tenu par rien. Les deux autres formes ont été essayées
// sur la matière réelle avant d'être écartées : le « bloc jusqu'à la ligne vide » couvre
// ici 44 lignes dont trois portent « mesuré » ou « estimé », donc il laissait passer la
// faute de la ligne 95 ; la fenêtre « ±2 lignes » aussi, parce que le « mesurée » était
// exactement deux lignes sous elle. La ligne n'a aucun réglage à régler, et son coût est
// d'écriture : la durée et son origine se mettent côte à côte, et un paragraphe ne se
// remet pas en forme librement — un retour à la ligne entre le nombre et son unité, ou
// entre la durée et son « mesuré », change le verdict sans changer une affirmation.
//
// ⚠️ IL NE VOIT QUE LES CHIFFRES, et le vague lui échappe : « dure des minutes »,
// « quelques secondes » passent, alors que le flou est justement ce qu'on écrit quand on
// n'a pas mesuré — la phrase d'origine de `Reseau.svelte` était « un scan réseau dure des
// minutes », et seul l'« environ deux minutes » de la ligne suivante l'a fait rougir. Le
// trou est écrit et non comblé pour une raison nommée : `src/App.svelte` porte la même
// tournure et il est tenu par une autre branche (PR #25, 107 lignes modifiées). Élargir
// la grammaire sans pouvoir corriger ce qu'elle attrape ferait naître la règle rouge.
//
// CE DÉTECTEUR SE CONTOURNE en écrivant « mesuré » sans avoir mesuré. C'est assumé, et
// c'est la même porte que le marqueur des fichiers générés (cf. SKILL.md) : elle se voit
// dans un diff, là où un chiffre muet ne se voit nulle part.
import { describe, expect, it } from 'vitest';
import { lire, sources } from './_lecture';

/** Les durées en chiffres (`36 min`) et en mots (`deux minutes`, `une demi-heure`). */
const DUREE =
  /\b(?:\d+(?:[.,]\d+)?\s*(?:ms|s|secondes?|min|minutes?|h|heures?)|(?:une|deux|trois|quatre|cinq|six|sept|huit|neuf|dix|quinze|vingt|trente|quarante|soixante)\s+(?:secondes?|minutes?|heures?)|demi-heures?)\b/gi;

/**
 * Une durée dont l'unité vit dans le NOM de la constante, pas à côté du nombre.
 *
 * Deuxième grammaire et non un terme de plus dans `DUREE`, pour une raison qui se mesure :
 * `DUREE` porte le drapeau `i`, donc un `[A-Z]` y matcherait aussi les minuscules et le
 * détecteur attraperait `etat.duree_ms = 0` — un champ de l'état, pas un chiffre annoncé.
 * La casse haute est la convention qui distingue une constante d'un champ, et elle ne tient
 * que dans une expression sans `i`.
 *
 * Né le 2026-11-27 de `VISEE_TENUE_MS = 300`, premier exemplaire du dépôt : `DUREE` exige
 * l'unité **collée aux chiffres**, donc ce qui suivait `300` était `;` et la ligne était
 * totalement invisible. Relevé par `invariant-guardian`, tranché par Thomas.
 */
const DUREE_NOMMEE = /\b[A-Z][A-Z0-9_]*_(?:MS|S|SEC|SECONDES|MIN|MINUTES|H|HEURES)\s*=\s*\d+(?:[.,]\d+)?/g;

/** Ce qui situe un chiffre : il est mesuré, ou il s'annonce estimé. Les deux conviennent. */
const ORIGINE = /mesur[ée]|estim[ée]/i;

/**
 * Les durées d'un composant, chacune avec sa ligne et le fait qu'elle soit située ou non.
 *
 * Les blocs `<style>` sont neutralisés : un `transition: 200ms` ne s'adresse à personne.
 * Ils sont remplacés par des espaces et non supprimés, pour que les numéros de ligne du
 * message d'échec restent ceux du fichier — un détecteur dont le rapport est faux coûte
 * plus de temps qu'il n'en fait gagner.
 */
export function dureesAnnoncees(svelte: string): { ligne: number; texte: string; situee: boolean }[] {
  const sansStyle = svelte.replace(/<style[^>]*>[\s\S]*?<\/style>/g, (b) => b.replace(/[^\n]/g, ' '));
  return sansStyle
    .split('\n')
    .flatMap((l, i) =>
      [...l.matchAll(DUREE), ...l.matchAll(DUREE_NOMMEE)].map((m) => ({
        ligne: i + 1,
        texte: m[0],
        situee: ORIGINE.test(l),
      })),
    );
}

// Corpus : les composants Svelte, et eux seuls. Les `.ts` d'interface en sont dehors sur une
// mesure, pas sur une intuition — `src/lib/ui/affichage.ts` écrit « un morceau de 3 min
// 59,6 s s'affiche », qui est un exemple de formatage et non une annonce. Les y faire entrer
// demande de distinguer les deux, ce qui est un autre travail. La limite est donc écrite ici
// plutôt que devinée plus tard.
describe('chiffres annoncés à l’utilisateur (INV-28)', () => {
  it('toute durée dit sur sa ligne si elle est mesurée ou estimée', () => {
    const fautes = sources('src', ['.svelte']).flatMap(([f, c]) =>
      dureesAnnoncees(c)
        .filter((d) => !d.situee)
        .map((d) => `${f}:${d.ligne} « ${d.texte} »`),
    );
    expect(fautes, 'écrire « mesuré » (et où) ou « estimé » sur la même ligne').toEqual([]);
  });

  it('le détecteur repère les deux écritures et tolère la ligne qui se situe', () => {
    expect(dureesAnnoncees('<p>environ deux minutes</p>').map((d) => d.texte)).toEqual(['deux minutes']);
    expect(dureesAnnoncees('<p>36 min</p>')[0]?.situee).toBe(false);
    expect(dureesAnnoncees('<p>36 min, mesuré le 2026-11-25</p>')[0]?.situee).toBe(true);
    expect(dureesAnnoncees('<p>une demi-heure — estimée</p>')[0]?.situee).toBe(true);
  });

  it('une durée dont l’unité est dans le nom est une durée', () => {
    expect(dureesAnnoncees('const VISEE_TENUE_MS = 300;')[0]).toEqual({
      ligne: 1,
      texte: 'VISEE_TENUE_MS = 300',
      situee: false,
    });
    expect(dureesAnnoncees('const X_MS = 300; // estimé')[0]?.situee).toBe(true);
    expect(dureesAnnoncees('const ATTENTE_MIN = 2;').length).toBe(1);
  });

  it('un champ en minuscules n’est pas une constante annoncée', () => {
    // La raison d'être d'une deuxième expression sans `i` : repliée dans `DUREE`, qui porte
    // ce drapeau, la casse haute cessait d'être une exigence et ces trois lignes rougissaient
    // — un champ de l'état sondé n'est pas un chiffre montré à quelqu'un. Sans cette
    // assertion, la fusion des deux grammaires reste une « simplification » tentante.
    expect(dureesAnnoncees('let duree_ms = 0;')).toEqual([]);
    expect(dureesAnnoncees('etat.position_ms = 1200;')).toEqual([]);
    expect(dureesAnnoncees('const dureeMs = 300;')).toEqual([]);
  });

  it('il voit la constante réelle du lecteur', () => {
    // La matière réelle, pas la faute inventée : les deux ne se remplacent pas. Cette
    // assertion se périme d'elle-même le jour où plus aucune constante de durée ne vit dans
    // un `.svelte` — et c'est sa qualité, elle rougira pour dire qu'il n'y a plus rien à
    // surveiller. Le dépôt n'en porte qu'une, d'où le seuil à 1.
    const vues = dureesAnnoncees(lire('src/lib/ui/Lecture.svelte')).filter((d) =>
      /^[A-Z][A-Z0-9_]*_/.test(d.texte),
    );
    expect(vues.length, 'plus aucune constante de durée dans un composant').toBeGreaterThan(0);
    expect(vues.every((d) => d.situee), 'une constante de durée ne dit pas son origine').toBe(true);
  });

  it('la fenêtre est la ligne, et les deux formes écartées sont interdites', () => {
    // Sans ces deux lignes, le choix argumenté en tête de fichier n'est tenu par rien, et
    // c'est la faute même de cette soirée : élargir la fenêtre au bloc ou à ±2 lignes —
    // le réflexe de bonne foi de qui voit rouge alors qu'un commentaire juste au-dessus
    // dit « mesuré » — laisse les quatre autres assertions vertes, donc INV-28 retombe
    // dans l'état qui l'a fait naître. Mesuré sur les deux mutations par
    // `invariant-guardian` le 2026-11-26, qui a aussi relevé que l'argument n'avait pas
    // de gage. Les auto-tests ne pouvaient pas le voir : ils tiennent tous sur une ligne.
    // (Un seul élément rendu dans chaque cas : seules les durées sont rapportées, et
    // « mesuré hier » n'en est pas une. L'index est donc 0 des deux côtés.)
    expect(dureesAnnoncees('<p>mesuré hier</p>\n<p>36 min</p>')[0]).toEqual({
      ligne: 2,
      texte: '36 min',
      situee: false,
    });
    expect(dureesAnnoncees('<p>36 min</p>\n<p>c’était mesuré</p>')[0]?.situee).toBe(false);
  });

  it('le numéro de ligne rapporté est celui du fichier', () => {
    // La seule raison de remplacer un bloc `<style>` par des espaces plutôt que de le
    // supprimer. Elle était écrite et non tenue : sans cette assertion, une faute en
    // ligne 4 se rapportait « ligne 2 » et rien ne rougissait.
    expect(dureesAnnoncees('<style>\n.a { color: red; }\n</style>\n<p>36 min</p>')[0]?.ligne).toBe(4);
  });

  it('une transition CSS n’est pas une annonce', () => {
    expect(dureesAnnoncees('<style>\n.a { transition: 200ms ease; }\n</style>')).toEqual([]);
  });

  it('il voit encore la durée réelle du projet', () => {
    // L'auto-test prouve que le détecteur voit la faute qu'on lui a INVENTÉE ; celui-ci
    // qu'il voit le texte RÉEL, et les deux ne se remplacent pas. Cette assertion se périme
    // d'elle-même le jour où plus aucun composant n'annonce de durée — et c'est sa qualité :
    // elle rougira pour dire « il n'y a plus rien à surveiller », pas pour rien.
    // `> 1` et non `> 0` : le fichier en porte deux, une par écriture — « 36 min » en
    // chiffres et « demi-heure » en mots. Un seuil à zéro restait vert si une seule des
    // deux grammaires était encore reconnue, ce qui est la faiblesse payée sur `tokens.css`.
    const vues = dureesAnnoncees(lire('src/lib/ui/Reseau.svelte'));
    expect(vues.filter((d) => d.situee).length, 'une écriture de durée n’est plus vue').toBeGreaterThan(1);
  });
});
