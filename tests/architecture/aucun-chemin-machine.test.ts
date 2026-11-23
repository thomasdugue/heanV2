// Aucune référence hors du dépôt (HARNAIS.md §9).
//
// V1 : le meilleur document design du projet vivait HORS du dépôt, et SAVOIR-FRONTEND.md
// décrivait des chemins de vignettes (`~/.noir_desktop/`) qui n'existaient plus. Une
// référence qu'on ne peut pas ouvrir depuis le dépôt est une référence qui se périme sans
// que personne le voie.
import { execFileSync } from 'node:child_process';
import { describe, expect, it } from 'vitest';
import { RACINE, lire } from './_lecture';

// Chemins absolus de machine. Deux exceptions légitimes :
//   - `~/Library/Application Support/hean/`, le dossier de données de l'app (ADR 0003) ;
//   - les BLOCS DE LANCEMENT `cd <chemin absolu> && <commande>`, convention explicite de
//     Thomas : un protocole de test s'exécute à la main, on ne veut pas chercher la commande.
// Ce qui reste visé est la seule chose dangereuse : une RÉFÉRENCE à un fichier qu'on ne
// peut pas ouvrir depuis le dépôt.
const CHEMIN_MACHINE = /\/(?:Users|home)\/[a-z0-9._-]+\//i;
const BLOC_DE_LANCEMENT = /^[`$\s]*cd\s+["'`]?\//;

export function cheminsMachine(texte: string): string[] {
  return texte
    .split('\n')
    .filter((l) => CHEMIN_MACHINE.test(l) && !BLOC_DE_LANCEMENT.test(l))
    .map((l) => l.trim().slice(0, 120));
}

// `bootstrap/` n'est PAS dans les zones, et c'est un choix, pas un oubli : c'est l'archive
// figée de V1 (ADR 0004). Ses chemins absolus sont un relevé historique — les « corriger »
// falsifierait le document. Et `03-architecture/code-reference/rust/sentry_init.rs` y porte
// des chemins de machine comme **fixtures d'un anonymiseur de chemins** : un faux positif
// permanent, et la démonstration que toute règle de ce genre doit s'arrêter au code vivant.
const ZONES = ['src', 'src-tauri/src', 'docs', '.claude', 'tools'] as const;

/**
 * Les fichiers des zones que ce détecteur ne lit PAS, avec leur raison écrite.
 *
 * ⚠️ **CE TEST SCANNAIT PAR EXTENSION jusqu'au 2026-11-23**, et il lui manquait `.py`,
 * `.json` et `.txt`. Trois fichiers Python vivaient donc hors d'une règle qui s'applique à
 * tout le dépôt, et `.claude/settings.json` avec eux — alors que c'est justement un fichier
 * où un chemin en dur serait un défaut réel : il porte une commande de hook, et V1 a payé
 * pour savoir ce que coûte un chemin qui ne se résout que chez une personne.
 *
 * **Le correctif n'est pas d'allonger la liste, c'est de la retourner.** Énumérer ce qu'on
 * surveille, c'est oublier la langue qu'on ajoutera demain — c'est ce qui a créé ce trou,
 * celui de `check-sizes.sh` (shell et Python non mesurés, un fichier de 569 lignes passé
 * vert) et l'impossibilité d'attraper `tools/git-hooks/pre-commit`, qui n'a pas d'extension.
 * On lit donc **tout, sauf ce qui est écrit ici**. Le sens de l'échec s'inverse, et c'est
 * tout l'intérêt : un binaire nouveau fait rougir et quelqu'un écrit l'exception, au lieu
 * qu'une langue nouvelle échappe en silence, ce qui ne réveille personne.
 */
const NON_LUS = [
  {
    motif: /\.(woff2?|ttf|otf|png|jpe?g|gif|webp|icns|ico|flac|wav|mp3|m4a|aiff?|pdf)$/i,
    pourquoi: 'binaire : aucun chemin lisible à y chercher, et le lire en utf8 produirait du bruit',
  },
] as const;

/**
 * Les fichiers des zones SUIVIS PAR GIT. C'est ce qui rend ce contrôle identique en local et
 * en CI, et c'est **la condition pour pouvoir retourner la liste** : en lisant tout ce qui
 * traîne sur le disque, un `.DS_Store` ferait rougir chez Thomas et passer en CI — le défaut
 * du 2026-11-22, réintroduit par le correctif d'un autre. Même choix que `tools/check-adr.mjs`.
 */
function fichiersSuivisParGit(): string[] {
  return execFileSync('git', ['ls-files', '-z', '--', ...ZONES], { cwd: RACINE, encoding: 'utf8' })
    .split('\0')
    .filter((f) => f !== '');
}

/** Ce que le détecteur lit vraiment : tout le suivi, moins les exceptions écrites. */
function fichiersLus(): string[] {
  return fichiersSuivisParGit().filter((f) => !NON_LUS.some((n) => n.motif.test(f)));
}

// ⚠️ CE DÉTECTEUR EST MUET PAR CONSTRUCTION, et c'est voulu : la seule matière qu'il pourrait
// trouver dans les zones vivantes serait une faute. Zéro trouvaille est son état sain, donc
// aucune assertion « il trouve au moins N choses réelles » n'a de sens ici — contrairement à
// `references-de-tests.test.ts`, dont la matière (les citations) est légitime et abondante.
//
// Ce qui le protège à la place, et qui est le vrai risque d'un test qui scanne cinq zones :
// les deux assertions de corpus ci-dessous, qui répondent à « est-ce que je regarde quelque
// chose ? » et « est-ce que mes exceptions avalent quelque chose de réel ? ».
describe('aucune référence hors du dépôt', () => {
  it('aucun chemin absolu de machine dans le code, la doc ou les skills', () => {
    const fautes = fichiersLus().flatMap((f) =>
      cheminsMachine(lire(f)).map((l) => `${f} : ${l}`),
    );
    expect(fautes, 'un chemin de machine ne se résout que chez une personne').toEqual([]);
  });

  it('il lit vraiment tout le dépôt vivant, pas une poignée de fichiers', () => {
    // Un détecteur qui ne regarde rien passe au vert sans rien garantir. Le seuil est bas
    // exprès : il mesure « les cinq zones sont là et sont lues », pas la taille du projet.
    expect(fichiersLus().length, 'les cinq zones ont-elles changé de nom ?').toBeGreaterThan(50);
  });

  it('les exceptions écrites correspondent à de la matière réelle', () => {
    // Le pendant de l'assertion précédente, et elle compte autant : une exception trop large
    // viderait le corpus sans rien dire. Celle-ci se périme d'elle-même — le jour où le dépôt
    // ne porte plus un seul binaire dans ces zones, ce test demande de retirer l'exception
    // devenue morte, au lieu de la laisser couvrir d'avance ce que personne n'a examiné.
    // Volontairement « au moins un » et non un compte exact : ajouter une police ou une
    // icône est une opération banale, qui n'a rien à voir avec cette règle et ne doit pas
    // la faire rougir. Ce qu'on surveille est qu'elle ne devienne pas une exception morte.
    const exclus = fichiersSuivisParGit().filter((f) => NON_LUS.some((n) => n.motif.test(f)));
    expect(
      exclus.length,
      'exception sans objet : la retirer, ou dire ce qu’elle couvre désormais',
    ).toBeGreaterThan(0);
  });

  it('le détecteur repère le cas de V1', () => {
    expect(cheminsMachine('voir /Users/thomas/dev/noirdesktop/design.html')).toEqual([
      'voir /Users/thomas/dev/noirdesktop/design.html',
    ]);
  });

  it('ne se déclenche pas sur le dossier de données de l’app', () => {
    expect(cheminsMachine('~/Library/Application Support/hean/')).toEqual([]);
  });

  it('tolère un bloc de lancement, qui est une convention voulue', () => {
    expect(cheminsMachine('cd /Users/x/Documents/heanV2 && ./check')).toEqual([]);
  });
});
