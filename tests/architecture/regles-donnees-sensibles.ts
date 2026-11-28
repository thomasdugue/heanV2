// Les règles d'INV-29 : qu'est-ce qu'une donnée sensible, et où les chercher.
//
// Séparé de `aucune-donnee-sensible.test.ts` comme `regles-backend.ts` l'est de son test : les
// détecteurs et leurs échantillons d'un côté, les assertions de l'autre. Les deux fichiers
// tenaient dans un seul jusqu'à ce qu'il franchisse les 400 lignes (check-sizes).
//
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
import { RACINE } from './_lecture';

/** Ce module : le seul fichier du dépôt qui porte des échantillons fautifs. */
export const CE_FICHIER = 'tests/architecture/regles-donnees-sensibles.ts';
export const AIGUILLES = 'tools/fuites-aiguilles.txt';

/**
 * Les échantillons fautifs, **comme données et non comme littéraux éparpillés**.
 *
 * La première version excluait ce fichier **en entier** des quatre détecteurs : l'exception la
 * plus large possible, dans le seul fichier du dépôt qui parle de formats de secrets — un vrai
 * justificatif collé ici y aurait été invisible. Relevé par `invariant-guardian`.
 *
 * Maintenant ce fichier est lu comme les autres, et seules **ces valeurs** y sont tolérées.
 *
 * ⚠️ Une liste de tolérances **se remplit en silence** : y coller une ligne dispenserait sa
 * valeur dans tout ce fichier, sans qu'aucune assertion bouge — un passe-droit à une ligne,
 * dans le seul fichier où un vrai justificatif est plausible. D'où « chaque échantillon est
 * consommé » plus bas : une valeur doit apparaître **deux fois** dans ce fichier, ici et dans
 * une assertion. Une entrée ajoutée pour être dispensée n'apparaît qu'une fois, et rougit.
 * Relevé par `invariant-guardian` : le commentaire d'origine annonçait ce mécanisme sans
 * l'avoir.
 */
export const ECHANTILLONS = {
  motDePasseApple: 'abcd-efgh-ijkl-mnop',
  motDePasseAppleMajuscules: 'ABCD-EFGH-IJKL-MNOP',
  cleSentry: '0123456789abcdef0123456789abcdef@o4509',
  jetonGitHub: 'ghp_AbCdEfGhIj0123',
  jetonDeWorkflow: 'ghs_AbCdEfGhIj0123',
  clePrivee: 'BEGIN RSA PRIVATE KEY',
  courrielDunTiers: 'quelquun@exemple-tiers.fr',
  courrielAuTldMoldave: 'thomas@exemple.md',
  secretEnClair: 'ceciestunvraisecret',
  secretEnMajuscules: 'motdepasseenclair',
  secretBase64AvecSlash: 'wJalrXUtnFEMI/K7MDENG/bPxRfiCYEX',
  secretCotePourRust: 'enclairici-mais-pas-un-chemin',
  aiguilleDessai: 'aiguille-dessai',
} as const;
const VALEURS_ECHANTILLONS: ReadonlySet<string> = new Set(Object.values(ECHANTILLONS));

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
] as const;

/** Ce que `fichier` est autorisé à porter. Seul ce fichier-ci a des échantillons. */
export function tolere(fichier: string, valeur: string): boolean {
  return fichier === CE_FICHIER && VALEURS_ECHANTILLONS.has(valeur);
}

export function fichiersSuivis(): string[] {
  const tous = execFileSync('git', ['ls-files', '-z'], { cwd: RACINE, encoding: 'utf8' })
    .split('\0')
    .filter((f) => f !== '');
  return tous.filter((f) => !NON_LUS.some((n) => n.motif.test(f)));
}

// --- les détecteurs, chacun une fonction pure -------------------------------------------

/**
 * Mot de passe app-specific Apple, DSN Sentry, jetons GitHub et Slack, clé AWS, clé privée.
 *
 * **Insensible à la casse** sur le mot de passe Apple : il est généré en minuscules, mais la
 * fuite d'origine vivait dans un guide markdown **recopié à la main**, où rien ne l'impose.
 * Mesuré : zéro faux positif sur les 373 fichiers suivis, la bordure suffit à écarter le
 * kebab-case. Les cinq préfixes de jeton GitHub et pas seulement `ghp_` : `ghs_` est celui
 * d'un jeton de workflow, donc la forme qu'un `.yml` porte — et la seule chose qui couvre la
 * ligne YAML nue que `AFFECTATION` refuse exprès de voir.
 */
const JUSTIFICATIFS =
  /(?<![A-Za-z-])[A-Za-z]{4}-[A-Za-z]{4}-[A-Za-z]{4}-[A-Za-z]{4}(?![A-Za-z-])|[0-9a-f]{32}@o?[0-9]+|gh[psour]_[A-Za-z0-9]{10,}|github_pat_[A-Za-z0-9]{10,}|xox[abprs]-[A-Za-z0-9]|AKIA[A-Z0-9]{8}|BEGIN [A-Z ]*PRIVATE KEY/g;
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
// ⚠️ **Aucune extension qui soit aussi un TLD réel.** La première liste portait `.md` et
// `.sh` : `thomas@exemple.md` passait donc inaperçu, alors que `.md` est le domaine de la
// Moldavie. Ne restent que des extensions d'actif, dont aucune n'est un TLD délégué.
const COURRIELS_HORS_SUJET =
  /@[0-9]+x\.(?:png|jpe?g|webp|gif|svg)$|\.(?:png|jpe?g|webp|gif|svg|woff2?|ico|css)$|^git@github\.com$|@hean\.app$|@(?:users\.noreply\.github\.com|example\.(?:com|org)|test|localhost)$|^(?:noreply|no-reply)@/i;

export function courriels(texte: string): string[] {
  return [...texte.matchAll(COURRIEL)].map((m) => m[0]).filter((v) => !COURRIELS_HORS_SUJET.test(v));
}

/**
 * Le mot de passe SMB du NAS et le jeton du worker n'ont AUCUN format : seule la forme d'une
 * affectation les voit. Deux fixtures du chantier WiFi Sync y répondent légitimement — leur
 * valeur est inventée et publique depuis V1.
 */
/**
 * ⚠️ **Le guillemet reste exigé, et c'est mesuré.** Rendre ce guillemet optionnel ferait voir
 * `KEY=valeur` et `token: valeur`, mais remonte **52 valeurs** sur ce dépôt, dont
 * `Option<String>`, `password.to_string(` et `--color-bg` : c'est le piège 1, et la fenêtre
 * garde donc son ancre. Sont pris, parce qu'ils ne coûtent aucun bruit : la **clé
 * guillemetée** (JSON) et les emballages `String::from("…")` / `format!("…")`, la forme Rust
 * de ce dépôt. Un secret à format connu se reconnaît seul (`JUSTIFICATIFS`) ; seul un secret
 * sans format a besoin d'une syntaxe.
 */
const AFFECTATION =
  /["']?(?:mot_de_passe|password|passwd|token|secret|dsn|api[_-]?key)[a-z_]*["']?\s*[:=]\s*(?:String::from\(|format!\()?["']([^"' ]{8,})/gi;
const FIXTURES = new Set(['secret-token-1234', 'uniq-test-token-9876']);

/**
 * La forme **shell / `.env`** : clé en majuscules, `=`, valeur nue. C'est celle que les deux
 * ADR citent comme exemple de ce qui fuit (`APPLE_ID_PASSWORD=…`), et aucun guillemet ne la
 * borne — `AFFECTATION` ne la voyait donc pas, alors que les documents annonçaient l'inverse.
 *
 * Elle est **restreinte** là où la variante générale ne l'était pas : `=` seulement (jamais
 * `:`, qui ferait entrer `let secret: Option<String>`), clé en début de ligne, valeur sans
 * espace ni guillemet. Mesuré le 2026-11-28 sur les fichiers suivis : **une** correspondance,
 * le gabarit déjà nommé dans `GABARITS`. Relevé par `invariant-guardian`.
 */
const AFFECTATION_NUE =
  /(?:^|\n)[ \t]*(?:export[ \t]+)?[A-Za-z_]\w*(?:PASSWORD|PASSWD|TOKEN|SECRET|DSN|API_?KEY|MOT_DE_PASSE)\w*[ \t]*=[ \t]*([^"'\s#]{8,})/gi;

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
] as const;

/**
 * ⚠️ **Le chemin s'écarte par la liste des fichiers suivis, pas par sa forme.** La première
 * version écartait toute valeur contenant un `/`, au motif que `SECRETS =
 * 'src-tauri/src/secrets.rs'` en est un. Mesuré par `invariant-guardian` : elle écartait aussi
 * `"wJalrXUtnFEMI/K7MDENG/bPxRfiCYEX"`, et **un secret base64 sur deux porte un `/`**. Une
 * exception ne doit pas couvrir plus large que son objet ; celle-ci s'ancre donc sur son objet
 * réel — un chemin qui existe — et elle se périme toute seule le jour où le fichier disparaît.
 */
export function affectations(texte: string, cheminsSuivis: ReadonlySet<string> = new Set()): string[] {
  return [...texte.matchAll(AFFECTATION), ...texte.matchAll(AFFECTATION_NUE)]
    .map((m) => m[1] ?? '')
    .filter(
      (v) =>
        !FIXTURES.has(v) &&
        !GABARITS.has(v) &&
        !REFERENCES.some((r) => r.motif.test(v)) &&
        !cheminsSuivis.has(v),
    );
}

/**
 * Les aiguilles nommées — le nom du NAS, une adresse personnelle : des données sans format,
 * qu'aucune classe ne cerne. Elles vivent hachées, et le dépôt public ne les porte jamais en
 * clair. Ce n'est pas un coffre (un nom court se retrouve par force brute) : c'est ce qu'il
 * faut pour la menace réelle, REPUBLIER par inadvertance ce qu'on vient de retirer.
 */
// Un jeton d'au moins quatre caractères, sans espace. **Donc une aiguille plus courte, ou qui
// en contient une, ne sera jamais trouvée** — par exemple un nom d'hôte en deux mots. La
// limite est écrite ici plutôt que découverte : une aiguille qui ne rentre pas dans cette
// fenêtre doit se masquer autrement, et le registre le dit aussi.
const JETON = /[A-Za-z0-9][A-Za-z0-9._-]{3,}/g;

/**
 * Le registre : ses empreintes, sa dérogation écrite, et **ses lignes illisibles**.
 *
 * ⚠️ Les trois portes par lesquelles ce registre pouvait redevenir muet sans rougir, relevées
 * par `invariant-guardian` : une empreinte malformée (63 caractères, une majuscule) était
 * **jetée en silence** et la dérogation permanente gardait le contrôle vert ; dérogation et
 * empreintes pouvaient coexister, donc une dérogation oubliée rattrapait un retrait
 * d'empreinte ; et la dérogation n'avait pas d'échéance, là où le corpus vide de `_lecture.ts`
 * exige « la raison **et la phase qui le remplira** ». Les trois sont fermées par `illisibles`,
 * par l'assertion de conflit, et par l'exigence d'une date ou d'une phase.
 */
export function registreAiguilles(): {
  empreintes: Map<string, string>;
  videPrevu: string | undefined;
  illisibles: string[];
} {
  const empreintes = new Map<string, string>();
  const illisibles: string[] = [];
  const chemin = join(RACINE, AIGUILLES);
  if (!existsSync(chemin)) return { empreintes, videPrevu: undefined, illisibles };
  let videPrevu: string | undefined;
  for (const brute of readFileSync(chemin, 'utf8').split('\n')) {
    const ligne = brute.trim();
    const derogation = /^#\s*AUCUNE-AIGUILLE\s*:\s*(.+)$/.exec(ligne);
    if (derogation?.[1] !== undefined) {
      videPrevu = derogation[1].trim();
      continue;
    }
    if (ligne === '' || ligne.startsWith('#')) continue;
    const m = /^([0-9a-f]{64})(?:\s+(.*))?$/.exec(ligne);
    if (m?.[1] === undefined) {
      illisibles.push(ligne.slice(0, 20));
      continue;
    }
    empreintes.set(m[1], m[2]?.trim() || '(sans étiquette)');
  }
  return { empreintes, videPrevu, illisibles };
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

