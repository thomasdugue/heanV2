// INV-25 — « la copie est sur le disque avant que le témoin ne la certifie ».
// INV-26 — « la migration ne supprime jamais le dossier de V1 ».
//
// Deux tests de FORME, parce qu'aucune des deux propriétés ne s'observe à l'exécution ici : la
// première demanderait de couper le courant d'un Mac au bon millième de seconde, la seconde est
// bien éprouvée par un test de comportement (`src-tauri/tests/demarrage/retour_arriere.rs`) mais ce qui doit
// rougir est la **réapparition de l'appel**, pas seulement son effet.
//
// Ce que ces tests empêchent concrètement :
//   - qu'on trouve ces synchronisations lentes et qu'on les déplace ou les retire. Elles coûtent
//     90 ms sur le dossier réel de Thomas (mesuré le 2026-11-25, 2 444 fichiers) — un chiffre
//     qui n'est pas sous les yeux de qui cherche à gagner du temps ;
//   - qu'un « nettoyage » de phase 3 remette la suppression de `noir/`. Elle était là jusqu'au
//     2026-11-25, elle rendait la main plus propre, et Thomas a décidé l'inverse (ADR 0014,
//     « Oui, garde-le ») : son dossier de V1 est son retour arrière vers la V1.
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

// Plus de `suppression-de-loriginal` : c'est l'objet d'INV-26, et l'absence se vérifie plus bas.
const ATTENDU = ['copie-durable', 'echange', 'renommage-durable'];

describe('INV-25 — rien n’est certifié avant que la copie soit sur le disque', () => {
  it('la migration synchronise, échange, puis synchronise le renommage', () => {
    expect(ordreDeLaFinDeMigration(lire(MIGRATION_RS))).toEqual(ATTENDU);
  });

  it('INV-26 — et elle ne supprime pas le dossier de V1', () => {
    // L'étape qui manque à la liste ci-dessus, et qui doit continuer à manquer. Le détecteur
    // sait la reconnaître — les deux tests suivants le prouvent sur la forme qu'elle avait —
    // donc son absence ici est une mesure, pas un silence.
    expect(
      ordreDeLaFinDeMigration(lire(MIGRATION_RS)),
      'le dossier de V1 est le retour arrière de Thomas (ADR 0014) : rien ne doit le supprimer',
    ).not.toContain('suppression-de-loriginal');
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
