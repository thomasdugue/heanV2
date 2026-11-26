// INV-28 — une durée annoncée à l'utilisateur dit sur place si elle est mesurée ou estimée.
//
// Née le 2026-11-26, de trois cas en une soirée. L'interface annonçait « environ deux
// minutes » pour le premier scan réseau : un chiffre d'ordre de grandeur, écrit avant que
// le scan n'ait jamais tourné. Le vrai a duré **36 minutes** (ADR 0016 §5). Une attente
// annoncée vingt fois trop courte est pire que pas de chiffre du tout — c'est ce que le
// commentaire de `Reseau.svelte` dit lui-même, puis contredit deux lignes plus haut. Le
// même soir, l'ADR 0015 chiffrait le coût de l'option B à « une saisie de mot de passe »
// deux fois de suite, avant qu'un relevé du trousseau ne le mesure à **zéro**.
//
// Ce que ces trois cas ont en commun n'est pas l'erreur : c'est qu'**on ne pouvait pas
// savoir, en lisant, que le chiffre n'avait jamais été mesuré**. Un chiffre faux se
// corrige ; un chiffre dont l'origine est muette se recopie.
//
// LA FENÊTRE EST LA LIGNE, et c'est un choix. Les autres formes ont été essayées sur la
// matière réelle avant d'être écartées : le « bloc jusqu'à la ligne vide » rend vert tout
// le `{#if}` de `Reseau.svelte` (douze lignes dont une porte « mesuré »), donc il
// laisserait passer la faute de la ligne 95 ; la fenêtre « ±2 lignes » aussi, à une ligne
// près. La ligne n'a aucun réglage à régler, et son seul coût est d'écriture : la durée et
// son origine se mettent côte à côte.
//
// CE DÉTECTEUR SE CONTOURNE en écrivant « mesuré » sans avoir mesuré. C'est assumé, et
// c'est la même porte que le marqueur des fichiers générés (cf. SKILL.md) : elle se voit
// dans un diff, là où un chiffre muet ne se voit nulle part.
import { describe, expect, it } from 'vitest';
import { lire, sources } from './_lecture';

/** Les durées en chiffres (`36 min`) et en mots (`deux minutes`, `une demi-heure`). */
const DUREE =
  /\b(?:\d+(?:[.,]\d+)?\s*(?:ms|s|secondes?|min|minutes?|h|heures?)|(?:une|deux|trois|quatre|cinq|six|sept|huit|neuf|dix|quinze|vingt|trente|quarante|soixante)\s+(?:secondes?|minutes?|heures?)|demi-heures?)\b/gi;

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
      [...l.matchAll(DUREE)].map((m) => ({ ligne: i + 1, texte: m[0], situee: ORIGINE.test(l) })),
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

  it('une transition CSS n’est pas une annonce', () => {
    expect(dureesAnnoncees('<style>\n.a { transition: 200ms ease; }\n</style>')).toEqual([]);
  });

  it('il voit encore la durée réelle du projet', () => {
    // L'auto-test prouve que le détecteur voit la faute qu'on lui a INVENTÉE ; celui-ci
    // qu'il voit le texte RÉEL, et les deux ne se remplacent pas. Cette assertion se périme
    // d'elle-même le jour où plus aucun composant n'annonce de durée — et c'est sa qualité :
    // elle rougira pour dire « il n'y a plus rien à surveiller », pas pour rien.
    const vues = dureesAnnoncees(lire('src/lib/ui/Reseau.svelte'));
    expect(vues.filter((d) => d.situee).length, 'plus une seule durée vue dans Reseau.svelte').toBeGreaterThan(0);
  });
});
