// INV-25 — « la copie est forcée sur le disque avant que l'original soit supprimé ».
//
// Un test de FORME, parce que la propriété n'est pas observable à l'exécution : la prouver
// demanderait de couper le courant d'un Mac au bon millième de seconde. Ce qui est
// observable, en revanche, c'est l'ordre des quatre appels dans le code — et c'est tout ce
// qui sépare « la migration a réussi » de « les données des cinquante testeurs ont disparu ».
//
// Ce que ce test empêche concrètement : que quelqu'un trouve ces synchronisations lentes et
// les déplace après la suppression, ou les retire. Elles coûtent 90 ms sur le dossier réel
// de Thomas (mesuré le 2026-11-25, 2 444 fichiers) — mais ce chiffre-là n'est écrit nulle
// part dans le code qu'on relit quand on cherche à gagner du temps.
import { describe, expect, it } from 'vitest';
import { lire, sansCommentaires } from './_lecture';

const MIGRATION_RS = 'src-tauri/src/migration/mod.rs';

/**
 * L'ordre des quatre étapes irréversibles de la fin de migration, dédoublonné.
 *
 * Cherche les appels et jamais la prose : `sansCommentaires` d'abord, puis des motifs qui
 * portent leurs parenthèses ou leur argument. La déduplication garde la PREMIÈRE apparition
 * de chaque étape — un `rename` plus bas dans le fichier (celui des fichiers cachés) ne doit
 * pas compter pour l'échange, mais une inversion, elle, change bien l'ordre rendu.
 */
export function ordreDeLaFinDeMigration(source: string): string[] {
  const re =
    /durabilite::rendre_durable_le_renommage|durabilite::rendre_durable|std::fs::rename|std::fs::remove_dir_all\(source\)/g;
  const nom = (m: string): string => {
    if (m.endsWith('renommage')) return 'renommage-durable';
    if (m.endsWith('rendre_durable')) return 'copie-durable';
    if (m.includes('rename')) return 'echange';
    return 'suppression-de-loriginal';
  };
  return [...sansCommentaires(source).matchAll(re)]
    .map((m) => nom(m[0]))
    .filter((n, i, tous) => tous.indexOf(n) === i);
}

const ATTENDU = ['copie-durable', 'echange', 'renommage-durable', 'suppression-de-loriginal'];

describe('INV-25 — rien d’irréversible avant que la copie soit sur le disque', () => {
  it('la migration synchronise, échange, synchronise le renommage, puis supprime', () => {
    expect(ordreDeLaFinDeMigration(lire(MIGRATION_RS))).toEqual(ATTENDU);
  });

  it('le détecteur voit la forme qu’avait le code avant le 2026-11-25', () => {
    // Aucune synchronisation : `fs::copy` rend la main quand le cache du système est à jour,
    // pas le disque. Une coupure de courant dans les secondes qui suivaient laissait un
    // `hean/` d'apparence complète, des fichiers vides dedans, et plus de `noir/`.
    const avant = `
      std::fs::rename(&provisoire, cible).map_err(...)?;
      if let Err(e) = std::fs::remove_dir_all(source) {
          tracing::warn!("l'ancien dossier n'a pas pu être supprimé");
      }`;
    expect(ordreDeLaFinDeMigration(avant)).toEqual(['echange', 'suppression-de-loriginal']);
  });

  it('il voit aussi une synchronisation reléguée après la suppression', () => {
    // La faute la plus plausible : garder les appels, mais les déplacer « pour que l'app
    // démarre plus vite ». L'ordre rendu change, donc le test rougit.
    const inverse = `
      std::fs::rename(&provisoire, cible)?;
      std::fs::remove_dir_all(source).ok();
      durabilite::rendre_durable(cible).ok();`;
    expect(ordreDeLaFinDeMigration(inverse)).toEqual([
      'echange',
      'suppression-de-loriginal',
      'copie-durable',
    ]);
  });

  it('il ne se déclenche pas sur la prose qui décrit ces étapes', () => {
    // Le module explique ces cinq temps en tête de fichier, et les commentaires nomment les
    // fonctions. Un détecteur qui les comptait prouverait la documentation, pas le code.
    const prose = `
      //! 3. **Forcer sur le disque** : durabilite::rendre_durable, avant std::fs::rename.
      // puis std::fs::remove_dir_all(source), et seulement là.`;
    expect(ordreDeLaFinDeMigration(prose)).toEqual([]);
  });
});
