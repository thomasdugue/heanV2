// L'état partagé de la bibliothèque. Un seul objet, **jamais réassigné**.
//
// V013 — en V1, `library = {}` cassait la référence que les autres modules détenaient : ils
// continuaient à lire l'ancien objet, donc une vue affichait une bibliothèque périmée sans
// qu'aucune erreur n'apparaisse. Le correctif V1 était une règle écrite (« ne réassigne
// jamais les objets d'état partagés »), c'est-à-dire une chose à retenir.
//
// Ici c'est le runtime qui refuse : l'objet est **gelé**. Ses champs ne peuvent plus être
// remplacés — en module ES, donc en mode strict, une affectation lève une TypeError. Le
// contenu, lui, reste mutable : on vide et on remplit le tableau en place. C'est la même
// décision que `$state.raw` côté performance (docs/PERFORMANCE.md) : la liste de pistes
// n'est pas réactive, et dix mille pistes enveloppées dans des signaux coûteraient de la
// mémoire et du temps à chaque mutation.
//
// « Chargée ? » n'est PAS un champ de cet objet, et c'est voulu : un champ gelé ne pourrait
// plus changer, et il faudrait le tenir à jour ailleurs — deux endroits pour un seul fait,
// soit exactement la famille de défaut que la base SQLite fait disparaître côté backend.
// C'est une fonction, `estChargee()`, et elle n'a qu'une source.

import type { Track } from '../../ipc/client';

export interface EtatBibliotheque {
  /** Toutes les pistes connues, dans l'ordre rendu par le dépôt (tri total, fait en SQL). */
  readonly pistes: Track[];
  /** Lookup O(1) par chemin (B13). Reconstruit en place, jamais remplacé. */
  readonly parChemin: Map<string, Track>;
}

/** L'instance unique. `freeze` la protège de la faute de V013. */
export const bibliotheque: EtatBibliotheque = Object.freeze({
  pistes: [] as Track[],
  parChemin: new Map<string, Track>(),
});

let chargee = false;

/**
 * Faux jusqu'au premier chargement réussi. Une bibliothèque vide et une bibliothèque pas
 * encore chargée ne s'affichent pas pareil — V001 (« la home s'affiche complète, sans état
 * partiel ») en dépendra.
 */
export function estChargee(): boolean {
  return chargee;
}

/** Remplace le CONTENU de l'état, sans toucher à son identité ni à celle de ses champs. */
export function remplacerLesPistes(pistes: readonly Track[]): void {
  bibliotheque.pistes.length = 0;
  bibliotheque.pistes.push(...pistes);
  bibliotheque.parChemin.clear();
  for (const p of pistes) bibliotheque.parChemin.set(p.path, p);
  chargee = true;
}

/** Met à jour en place les pistes que le backend vient de renvoyer après une édition. */
export function majDesPistes(modifiees: readonly Track[]): void {
  for (const p of modifiees) {
    const i = bibliotheque.pistes.findIndex((x) => x.path === p.path);
    if (i >= 0) bibliotheque.pistes[i] = p;
    else bibliotheque.pistes.push(p);
    bibliotheque.parChemin.set(p.path, p);
  }
}

/** Retire une piste de l'état, après son exclusion côté backend (INV-06). */
export function oublierLaPiste(chemin: string): void {
  const i = bibliotheque.pistes.findIndex((x) => x.path === chemin);
  if (i >= 0) bibliotheque.pistes.splice(i, 1);
  bibliotheque.parChemin.delete(chemin);
}

/** Remet l'état à zéro. Réservé aux tests : l'application ne décharge jamais sa bibliothèque. */
export function reinitialiserPourTest(): void {
  bibliotheque.pistes.length = 0;
  bibliotheque.parChemin.clear();
  chargee = false;
}
