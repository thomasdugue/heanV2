// Aucune donnée sensible dans un fichier suivi par git.
//
// Né le 2026-11-28, de quatre fuites trouvées en une semaine par la RELECTURE d'un agent et
// non par une vérification : le nom d'hôte du NAS dans 7 fichiers, un mot de passe
// app-specific Apple glissé entre deux lignes déjà masquées, une clé Sentry, l'e-mail d'un
// tiers. Le dépôt est public depuis sa création (ADR 0019), donc la relecture arrive après
// la publication — et une PR fusionnée laisse une `refs/pull/<n>/head` que personne ne peut
// supprimer (ADR 0021). C'était la décision ouverte « INV-29 » de l'index des ADR.
//
// Les motifs viennent du « dernier balayage » de `docs/PROTOCOLE-DEPOT-NEUF.md` §3, écrits
// pour être tapés une fois à la main avant une bascule — donc par discipline, ce que
// l'ADR 0005 refuse. Ils y ont été corrigés trois fois sur de la matière réelle ; les
// reprendre à l'identique garde une seule source de vérité. Les trois pièges payés :
//
//   1. un motif trop large (16 classes CSS en kebab-case) ne se lit pas jusqu'au bout, et
//      c'est comme ça qu'un vrai justificatif passe. D'où les bordures, et les faux
//      positifs écartés par leur FORME et non un par un ;
//   2. une exception qui filtre la LIGNE masque un vrai secret posé à côté d'un gabarit.
//      On filtre la VALEUR ;
//   3. nommer l'aiguille la remet dans le dépôt — la donnée dans le fichier censé prouver
//      son départ. Aucun nom d'hôte, aucune adresse, aucun justificatif n'est écrit ici :
//      les aiguilles nommées vivent HACHÉES (`tools/fuites-aiguilles.txt`).
//
// ⚠️ **Et le troisième contrôle du protocole ne regarde rien.** Il cherche les noms d'hôte
// par `git grep -E '\b…\.local\b'` : `-E` est de l'ERE POSIX, où `\b` ne vaut pas bordure de
// mot et ne matche JAMAIS. Il rend zéro depuis le 2026-11-27 sans rien lire, et ce zéro a
// été lu comme une preuve. Mesuré le 2026-11-28 : la forme portable (`(^|[^a-z0-9.-])`)
// trouve douze noms, tous des gabarits (`nas.local`, `_smb._tcp.local`, `a.local`) et aucun
// vrai. La classe entière est donc du bruit ici, et seule l'aiguille exacte dit quelque
// chose — d'où le contrôle 2 par empreinte. En JavaScript `\b` fonctionne : le piège est
// celui de l'ERE, pas celui de l'idée. Le document reste à corriger, et il est tenu par une
// autre session.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { RACINE } from './_lecture';

const CE_FICHIER = 'tests/architecture/aucune-donnee-sensible.test.ts';
const AIGUILLES = 'tools/fuites-aiguilles.txt';

/**
 * Ce détecteur lit **tout** le dépôt, `bootstrap/` compris — à la différence
 * d'`aucun-chemin-machine`, qui s'arrête au code vivant. Ce n'est pas une inattention : les
 * trois justificatifs réellement fuités vivaient dans `bootstrap/`, l'archive figée de V1.
 * Un secret ne devient pas inoffensif parce que le fichier qui le porte est historique.
 *
 * La liste est **retournée** : on lit tout sauf ce qui est écrit ici, avec sa raison. Énumérer
 * ce qu'on surveille, c'est oublier la langue qu'on ajoutera demain.
 */
const NON_LUS = [
  {
    motif: /\.(woff2?|ttf|otf|png|jpe?g|gif|webp|icns|ico|flac|wav|mp3|m4a|aiff?|pdf)$/i,
    pourquoi: 'binaire : rien de lisible à y chercher, et le lire en utf8 produirait du bruit',
  },
  {
    motif: new RegExp(`^${CE_FICHIER.replace(/[.]/g, '\\.')}$`),
    pourquoi: "les échantillons fautifs de l'auto-test vivent ici, et doivent y rester faux",
  },
] as const;

function fichiersSuivis(): string[] {
  const tous = execFileSync('git', ['ls-files', '-z'], { cwd: RACINE, encoding: 'utf8' })
    .split('\0')
    .filter((f) => f !== '');
  return tous.filter((f) => !NON_LUS.some((n) => n.motif.test(f)));
}

// --- les détecteurs, chacun une fonction pure -------------------------------------------

/** Mot de passe app-specific Apple, DSN Sentry, jetons GitHub et Slack, clé AWS, clé privée. */
const JUSTIFICATIFS =
  /(?<![a-z-])[a-z]{4}-[a-z]{4}-[a-z]{4}-[a-z]{4}(?![a-z-])|[0-9a-f]{32}@o[0-9]+|ghp_[A-Za-z0-9]{10}|github_pat_[A-Za-z0-9]{10}|xox[abprs]-[A-Za-z0-9]|AKIA[A-Z0-9]{8}|BEGIN [A-Z ]*PRIVATE KEY/g;
/** Les gabarits de documentation, écartés par VALEUR et non par ligne (piège 2). */
const GABARITS = new Set(['xxxx-xxxx-xxxx-xxxx']);

export function justificatifs(texte: string): string[] {
  return [...texte.matchAll(JUSTIFICATIFS)].map((m) => m[0]).filter((v) => !GABARITS.has(v));
}

/**
 * Une adresse e-mail. Le dernier label est en LETTRES : sans ça `wght@100..800` (une URL de
 * fonte) et `rust@1.93` (l'épinglage de l'ADR 0008) passaient pour des adresses. Trois
 * familles de faux positifs mesurées le 2026-11-28 et écartées par leur forme (piège 1) :
 * les noms retina (`logo@2x.png`), `git@github.com` d'une URL SSH, et `@hean.app`,
 * l'adresse de contact du produit — publique par destination.
 */
const COURRIEL = /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}/g;
const COURRIELS_HORS_SUJET =
  /@[0-9]+x\.(?:png|jpe?g|webp|gif|svg)$|\.(?:png|jpe?g|webp|gif|svg|woff2?|ico|css|js|ts|md|html)$|^git@github\.com$|@hean\.app$|@(?:users\.noreply\.github\.com|example\.(?:com|org)|test|localhost)$|^(?:noreply|no-reply)@/i;

export function courriels(texte: string): string[] {
  return [...texte.matchAll(COURRIEL)].map((m) => m[0]).filter((v) => !COURRIELS_HORS_SUJET.test(v));
}

/**
 * Le mot de passe SMB du NAS et le jeton du worker n'ont AUCUN format : seule la forme d'une
 * affectation les voit. Deux fixtures du chantier WiFi Sync y répondent légitimement — leur
 * valeur est inventée et publique depuis V1.
 */
const AFFECTATION =
  /(?:mot_de_passe|password|passwd|token|secret|dsn|api[_-]?key)[a-z_]*\s*[:=]\s*["']([^"' ]{8,})/gi;
const FIXTURES = new Set(['secret-token-1234', 'uniq-test-token-9876']);

/**
 * Ce qui a la FORME d'une affectation sans en porter la valeur. Le balayage du protocole est
 * sensible à la casse, donc il ne voyait que les mots-clés en minuscules et manquait
 * `PASSWORD=…` en majuscules — où un vrai secret se poserait tout aussi bien. L'élargir à
 * la casse fait remonter treize références de variables, mesurées le 2026-11-28 : toutes des
 * renvois, aucune valeur. On les écarte par leur forme, jamais une par une (piège 1).
 */
const REFERENCES = [
  { motif: /^[$]/, pourquoi: 'renvoi de variable shell : `PASSWORD="$APPLE_ID_PASSWORD"`' },
  { motif: /^[{]{2}/, pourquoi: 'gabarit de modèle : `TOKEN = \'{{jeton}}\'`' },
  { motif: /[<][A-Z_]+[>]/, pourquoi: 'gabarit de documentation : `DSN="https://<CLE>@<ORG>…"`' },
  {
    motif: /^[a-z0-9_.-]+(?:\/[a-z0-9_.+-]+)+$/i,
    pourquoi: "chemin du dépôt : `SECRETS = 'src-tauri/src/secrets.rs'`",
  },
] as const;

export function affectations(texte: string): string[] {
  return [...texte.matchAll(AFFECTATION)]
    .map((m) => m[1] ?? '')
    .filter((v) => !FIXTURES.has(v) && !REFERENCES.some((r) => r.motif.test(v)));
}

/**
 * Les aiguilles nommées — le nom du NAS, une adresse personnelle : des données sans format,
 * qu'aucune classe ne cerne. Elles vivent hachées, et le dépôt public ne les porte jamais en
 * clair. Ce n'est pas un coffre (un nom court se retrouve par force brute) : c'est ce qu'il
 * faut pour la menace réelle, REPUBLIER par inadvertance ce qu'on vient de retirer.
 */
const JETON = /[A-Za-z0-9][A-Za-z0-9._-]{3,}/g;

export function empreintesAiguilles(): Map<string, string> {
  const out = new Map<string, string>();
  const chemin = join(RACINE, AIGUILLES);
  if (!existsSync(chemin)) return out;
  for (const ligne of readFileSync(chemin, 'utf8').split('\n')) {
    const nue = (ligne.split('#')[0] ?? '').trim();
    const m = /^([0-9a-f]{64})\s*(.*)$/.exec(nue);
    if (m?.[1] !== undefined) out.set(m[1], m[2]?.trim() || '(sans étiquette)');
  }
  return out;
}

export function aiguilles(texte: string, empreintes: Map<string, string>): string[] {
  if (empreintes.size === 0) return [];
  const trouvees = new Set<string>();
  for (const m of texte.matchAll(JETON)) {
    const e = createHash('sha256').update(m[0].toLowerCase()).digest('hex');
    const etiquette = empreintes.get(e);
    if (etiquette !== undefined) trouvees.add(etiquette);
  }
  return [...trouvees];
}

// --- l'auto-test : chaque détecteur reçoit la faute qu'il est censé voir ------------------

describe("le détecteur voit la faute qu'on lui donne", () => {
  it('un mot de passe app-specific, et pas son gabarit', () => {
    expect(justificatifs('mdp : abcd-efgh-ijkl-mnop')).toEqual(['abcd-efgh-ijkl-mnop']);
    expect(justificatifs('mdp : xxxx-xxxx-xxxx-xxxx')).toEqual([]);
    // Le piège 1 : une classe CSS en kebab-case n'est pas un justificatif.
    expect(justificatifs('class="track-info-edit-field"')).toEqual([]);
    // Le piège 2 : un vrai secret posé sur la MÊME ligne qu'un gabarit se voit quand même.
    expect(justificatifs('gabarit xxxx-xxxx-xxxx-xxxx et vrai abcd-efgh-ijkl-mnop')).toEqual([
      'abcd-efgh-ijkl-mnop',
    ]);
  });

  it('une clé Sentry, un jeton GitHub, une clé privée', () => {
    expect(justificatifs('https://0123456789abcdef0123456789abcdef@o4509.ingest')).toHaveLength(1);
    expect(justificatifs('ghp_AbCdEfGhIj0123')).toHaveLength(1);
    expect(justificatifs('-----BEGIN RSA PRIVATE KEY-----')).toHaveLength(1);
  });

  it("l'adresse d'un tiers, et pas une URL de fonte ni un nom retina", () => {
    expect(courriels('écrire à quelquun@exemple-tiers.fr')).toEqual(['quelquun@exemple-tiers.fr']);
    expect(courriels('family=DM+Sans:wght@100..800')).toEqual([]);
    expect(courriels('<img src="logo@2x.png">')).toEqual([]);
    expect(courriels('git@github.com:thomasdugue/hean.git')).toEqual([]);
    expect(courriels('contact@hean.app')).toEqual([]);
  });

  it('une affectation de justificatif, et pas les fixtures de V1', () => {
    expect(affectations('let mot_de_passe = "ceciestunvraisecret";')).toEqual([
      'ceciestunvraisecret',
    ]);
    expect(affectations('let token = "secret-token-1234";')).toEqual([]);
    // Trop court pour être un secret : huit caractères au minimum.
    expect(affectations('let token = "court";')).toEqual([]);
  });

  it('les majuscules comptent, mais un renvoi de variable n\'est pas une valeur', () => {
    // Ce que le balayage du protocole ne voyait pas, faute de casse.
    expect(affectations('APPLE_PASSWORD="motdepasseenclair"')).toEqual(['motdepasseenclair']);
    // Et les quatre formes de renvoi, qui ne portent aucune valeur.
    expect(affectations('APPLE_PASSWORD="$APPLE_ID_PASSWORD"')).toEqual([]);
    expect(affectations('APPLE_PASSWORD="${APPLE_ID_PASSWORD}"')).toEqual([]);
    expect(affectations("const TOKEN = '{{jeton_du_modele}}'")).toEqual([]);
    expect(affectations('HEAN_SENTRY_DSN="https://<CLE>@<ORG>.ingest.sentry.io/<PROJET>"')).toEqual(
      [],
    );
    expect(affectations("const SECRETS = 'src-tauri/src/secrets.rs'")).toEqual([]);
  });

  it('une aiguille nommée, par son empreinte et jamais par sa valeur', () => {
    const valeur = 'aiguille-dessai';
    const e = createHash('sha256').update(valeur).digest('hex');
    const empreintes = new Map([[e, "le nom d'hôte du NAS"]]);
    expect(aiguilles(`le serveur ${valeur} répond`, empreintes)).toEqual(["le nom d'hôte du NAS"]);
    // La casse ne protège pas : le jeton est mis en minuscules avant le hachage.
    expect(aiguilles('le serveur Aiguille-Dessai répond', empreintes)).toHaveLength(1);
    expect(aiguilles('rien ici', empreintes)).toEqual([]);
    // Aucune aiguille enregistrée : le détecteur se tait au lieu de tout accuser.
    expect(aiguilles(`le serveur ${valeur} répond`, new Map())).toEqual([]);
  });
});

// --- la matière réelle -------------------------------------------------------------------

describe('le dépôt ne porte aucune donnée sensible', () => {
  const suivis = fichiersSuivis();
  const empreintes = empreintesAiguilles();

  it('il regarde quelque chose', () => {
    // Première question d'un détecteur : est-ce que je lis un corpus ? Un `git ls-files`
    // vide passerait tous les contrôles au vert sans rien garantir.
    expect(suivis.length).toBeGreaterThan(300);
  });

  it('aucun justificatif à format connu', () => {
    const fautifs = suivis.filter((f) => justificatifs(readFileSync(join(RACINE, f), 'utf8')).length > 0);
    // On nomme le fichier, jamais la valeur : cette sortie finit dans une PR ou un journal.
    expect(fautifs).toEqual([]);
  });

  it('aucune adresse e-mail hors des adresses publiques du projet', () => {
    const fautifs = suivis.filter((f) => courriels(readFileSync(join(RACINE, f), 'utf8')).length > 0);
    expect(fautifs).toEqual([]);
  });

  it('aucune affectation qui ressemble à un justificatif', () => {
    const fautifs = suivis.filter((f) => affectations(readFileSync(join(RACINE, f), 'utf8')).length > 0);
    expect(fautifs).toEqual([]);
  });

  it('aucune aiguille nommée', () => {
    // Zéro aiguille enregistrée est l'état normal tant que Thomas n'en a pas posé : le
    // mécanisme existe, et en enregistrer une coûte une ligne. Le compte est affiché pour
    // que ce zéro soit lu comme un choix et non comme une preuve.
    const touchees = suivis.flatMap((f) => aiguilles(readFileSync(join(RACINE, f), 'utf8'), empreintes));
    expect(touchees, `${empreintes.size} aiguille(s) enregistrée(s) dans ${AIGUILLES}`).toEqual([]);
  });

  it('aucune capture de la bibliothèque suivie par git (ADR 0020)', () => {
    // Le README qui décrit ce que montraient les huit images reste, et c'est le but de
    // l'ADR 0020 : la description publique, les images non. On ne compte que des images.
    const images = suivis.filter(
      (f) =>
        f.startsWith('bootstrap/02-design/reference/captures/') &&
        /\.(png|jpe?g|webp|gif)$/i.test(f),
    );
    expect(images).toEqual([]);
  });
});
