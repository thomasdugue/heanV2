// INV-29 — aucune donnée sensible dans un fichier suivi par git. Les assertions ; les
// détecteurs, leurs motifs et le raisonnement qui les a façonnés sont dans
// `regles-donnees-sensibles.ts`.
//
// Deux blocs, et les deux sont nécessaires : l'**auto-test** donne à chaque détecteur la faute
// qu'il est censé voir — il ne prouve que ce qu'on lui a inventé — et la **matière réelle**
// passe les détecteurs sur les fichiers suivis. Ils ont aussi été passés une fois sur l'arbre
// d'avant le masquage (`4af1343`) : 13 correspondances dans 4 fichiers, exactement les fuites
// que la relecture avait trouvées à la main (ADR 0023).
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { RACINE } from './_lecture';
import {
  affectations,
  AIGUILLES,
  aiguilles,
  courriels,
  ECHANTILLONS,
  fichiersSuivis,
  justificatifs,
  registreAiguilles,
  tolere,
} from './regles-donnees-sensibles';

// --- l'auto-test : chaque détecteur reçoit la faute qu'il est censé voir ------------------

const E = ECHANTILLONS;
const CE_FICHIER_DE_TEST = 'tests/architecture/aucune-donnee-sensible.test.ts';

describe("le détecteur voit la faute qu'on lui donne", () => {
  it('un mot de passe app-specific, dans les deux casses, et pas son gabarit', () => {
    expect(justificatifs(`mdp : ${E.motDePasseApple}`)).toEqual([E.motDePasseApple]);
    // La fuite d'origine vivait dans un guide recopié à la main : la casse n'y est pas garantie.
    expect(justificatifs(`MDP : ${E.motDePasseAppleMajuscules}`)).toEqual([
      E.motDePasseAppleMajuscules,
    ]);
    expect(justificatifs('mdp : xxxx-xxxx-xxxx-xxxx')).toEqual([]);
    // Le piège 1 : une classe CSS en kebab-case n'est pas un justificatif.
    expect(justificatifs('class="track-info-edit-field"')).toEqual([]);
    // Le piège 2 : un vrai secret posé sur la MÊME ligne qu'un gabarit se voit quand même.
    expect(justificatifs(`gabarit xxxx-xxxx-xxxx-xxxx et vrai ${E.motDePasseApple}`)).toEqual([
      E.motDePasseApple,
    ]);
  });

  it('une clé Sentry, les jetons GitHub, une clé privée', () => {
    expect(justificatifs(`https://${E.cleSentry}.ingest`)).toHaveLength(1);
    expect(justificatifs(E.jetonGitHub)).toHaveLength(1);
    // `ghs_` est le préfixe des jetons de workflow : la forme qu'un `.yml` porte, et la seule
    // qui couvre la ligne YAML nue que la fenêtre des affectations refuse exprès de voir.
    expect(justificatifs(`  token: ${E.jetonDeWorkflow}`)).toHaveLength(1);
    expect(justificatifs(`-----${E.clePrivee}-----`)).toHaveLength(1);
  });

  it("l'adresse d'un tiers, et pas une URL de fonte ni un nom retina", () => {
    expect(courriels(`écrire à ${E.courrielDunTiers}`)).toEqual([E.courrielDunTiers]);
    // `.md` est le domaine de la Moldavie : une extension qui est un TLD ne peut pas servir
    // de filtre, et la première version écartait celle-ci.
    expect(courriels(`écrire à ${E.courrielAuTldMoldave}`)).toEqual([E.courrielAuTldMoldave]);
    expect(courriels('family=DM+Sans:wght@100..800')).toEqual([]);
    expect(courriels('<img src="logo@2x.png">')).toEqual([]);
    expect(courriels('git@github.com:thomasdugue/hean.git')).toEqual([]);
    expect(courriels('contact@hean.app')).toEqual([]);
  });

  it('une affectation de justificatif, et pas les fixtures de V1', () => {
    expect(affectations(`let mot_de_passe = "${E.secretEnClair}";`)).toEqual([E.secretEnClair]);
    expect(affectations('let token = "secret-token-1234";')).toEqual([]);
    // Trop court pour être un secret : huit caractères au minimum.
    expect(affectations('let token = "court";')).toEqual([]);
  });

  it("les majuscules comptent, mais un renvoi de variable n'est pas une valeur", () => {
    expect(affectations(`APPLE_PASSWORD="${E.secretEnMajuscules}"`)).toEqual([
      E.secretEnMajuscules,
    ]);
    expect(affectations('APPLE_PASSWORD="$APPLE_ID_PASSWORD"')).toEqual([]);
    expect(affectations('APPLE_PASSWORD="${APPLE_ID_PASSWORD}"')).toEqual([]);
    expect(affectations("const TOKEN = '{{jeton_du_modele}}'")).toEqual([]);
    expect(affectations('HEAN_SENTRY_DSN="https://<CLE>@<ORG>.ingest.sentry.io/<PROJET>"')).toEqual(
      [],
    );
  });

  it("la forme shell nue, celle que les deux ADR citaient sans la voir", () => {
    expect(affectations(`APPLE_ID_PASSWORD=${E.secretEnMajuscules}`)).toEqual([
      E.secretEnMajuscules,
    ]);
    expect(affectations(`export NOIR_WORKER_SECRET=${E.secretEnClair}`)).toEqual([
      E.secretEnClair,
    ]);
    // `=` seulement : avec `:`, une annotation de type Rust entrerait.
    expect(affectations('let secret: Option<String> = None;')).toEqual([]);
    expect(affectations('API_KEY=$CLE_DU_WORKER')).toEqual([]);
  });

  it('la clé guillemetée et la forme Rust du dépôt', () => {
    expect(affectations(`"api_key": "${E.secretEnClair}"`)).toEqual([E.secretEnClair]);
    expect(affectations(`let mot_de_passe = String::from("${E.secretCotePourRust}");`)).toEqual([
      E.secretCotePourRust,
    ]);
  });

  it("un secret à slash n'est pas un chemin, et un chemin suivi n'est pas un secret", () => {
    // Le défaut mesuré : l'exception « chemin » écartait un secret base64 sur deux.
    expect(affectations(`NOIR_WORKER_SECRET = "${E.secretBase64AvecSlash}"`)).toEqual([
      E.secretBase64AvecSlash,
    ]);
    // Et le chemin reste écarté, parce qu'il est SUIVI — pas parce qu'il a un slash.
    const suivis = new Set(['src-tauri/src/secrets.rs']);
    expect(affectations("const SECRETS = 'src-tauri/src/secrets.rs'", suivis)).toEqual([]);
    expect(affectations("const SECRETS = 'src-tauri/src/secrets.rs'")).toEqual([
      'src-tauri/src/secrets.rs',
    ]);
  });

  it('une aiguille nommée, par son empreinte et jamais par sa valeur', () => {
    const e = createHash('sha256').update(E.aiguilleDessai).digest('hex');
    const empreintes = new Map([[e, "le nom d'hôte du NAS"]]);
    expect(aiguilles(`le serveur ${E.aiguilleDessai} répond`, empreintes)).toEqual([
      "le nom d'hôte du NAS",
    ]);
    // La casse ne protège pas : le jeton est mis en minuscules avant le hachage.
    expect(aiguilles('le serveur Aiguille-Dessai répond', empreintes)).toHaveLength(1);
    expect(aiguilles('rien ici', empreintes)).toEqual([]);
    // Registre vide : le détecteur se tait au lieu de tout accuser. C'est pour ça que le vide
    // doit se voir ailleurs — voir « le registre d'aiguilles n'est pas muet ».
    expect(aiguilles(`le serveur ${E.aiguilleDessai} répond`, new Map())).toEqual([]);
  });
});

// --- la matière réelle -------------------------------------------------------------------

describe('le dépôt ne porte aucune donnée sensible', () => {
  const suivis = fichiersSuivis();
  const ensembleSuivis: ReadonlySet<string> = new Set(suivis);
  const { empreintes, videPrevu, illisibles } = registreAiguilles();
  const lu = (f: string): string => readFileSync(join(RACINE, f), 'utf8');
  /** Les fichiers où `detecte` trouve quelque chose qui n'est pas un échantillon toléré. */
  const fautifs = (detecte: (texte: string, f: string) => string[]): string[] =>
    suivis.filter((f) => detecte(lu(f), f).some((v) => !tolere(f, v)));

  it('il regarde quelque chose', () => {
    // Première question d'un détecteur : est-ce que je lis un corpus ? Un `git ls-files`
    // vide passerait tous les contrôles au vert sans rien garantir.
    expect(suivis.length).toBeGreaterThan(300);
  });

  it('aucun justificatif à format connu', () => {
    // On nomme le fichier, jamais la valeur : cette sortie finit dans une PR ou un journal.
    expect(fautifs(justificatifs)).toEqual([]);
  });

  it('aucune adresse e-mail hors des adresses publiques du projet', () => {
    expect(fautifs(courriels)).toEqual([]);
  });

  it('aucune affectation qui ressemble à un justificatif', () => {
    expect(fautifs((t) => affectations(t, ensembleSuivis))).toEqual([]);
  });

  it('chaque échantillon toléré est consommé par une assertion', () => {
    // Un échantillon toléré doit être **référencé par une assertion**. Une entrée ajoutée
    // seulement pour que sa valeur soit dispensée ne l'est par personne, et rougit ici. C'est
    // ce qui empêche la liste de tolérances de grandir en silence — `invariant-guardian` a
    // relevé que sa première version annonçait ce mécanisme sans l'avoir.
    //
    // On compte les références à la CLÉ (`E.motDePasseApple`) et non la valeur : les
    // assertions passent toutes par l'identifiant, jamais par la chaîne.
    const assertions = readFileSync(join(RACINE, CE_FICHIER_DE_TEST), 'utf8');
    const orphelins = Object.keys(ECHANTILLONS).filter(
      (nom) => !assertions.includes(`E.${nom}`) && !assertions.includes(`ECHANTILLONS.${nom}`),
    );
    expect(orphelins).toEqual([]);
  });

  it("le registre d'aiguilles n'est pas muet", () => {
    // **Un registre vide est un détecteur muet**, et c'est le défaut que ce test a eu le jour
    // de sa naissance : `aiguilles()` rend `[]` dès que le registre est vide, donc la classe
    // « nom d'hôte » — la fuite qui avait touché 7 fichiers — n'était couverte par rien, et
    // la seule chose qui devait le signaler était un compte dans un message d'assertion, donc
    // affiché seulement en cas d'échec : jamais. Relevé par `invariant-guardian`.
    //
    // Même traitement qu'un corpus vide dans `_lecture.ts` : rouge par défaut, et une
    // dérogation qui s'ÉCRIT dans le registre (`# AUCUNE-AIGUILLE: …`), pas dans un commentaire
    // de code que personne ne relit.
    // Une ligne illisible est une empreinte PERDUE : 63 caractères ou une majuscule, et le
    // registre repart muet derrière une dérogation permanente.
    expect(illisibles, `lignes illisibles de ${AIGUILLES}`).toEqual([]);

    // Dérogation ET empreintes : la dérogation oubliée rattraperait un retrait d'empreinte.
    if (empreintes.size > 0) {
      expect(
        videPrevu,
        `${AIGUILLES} porte ${empreintes.size} empreinte(s) ET une dérogation : retirer la ` +
          `ligne « # AUCUNE-AIGUILLE: » maintenant qu'elle n'a plus d'objet`,
      ).toBeUndefined();
      return;
    }

    const raison = videPrevu ?? '';
    expect(
      raison.length,
      `${AIGUILLES} ne contient aucune empreinte : enregistrer une aiguille, ou écrire ` +
        `pourquoi il n'y en a pas, dans une ligne « # AUCUNE-AIGUILLE: <raison> » du registre`,
    ).toBeGreaterThan(30);
    // **Et une échéance**, comme le corpus vide de `_lecture.ts` exige « la phase qui le
    // remplira » : sans date ni phase, une dérogation est une couverture définitive écrite en
    // petit. C'est la classe « nom d'hôte » — la fuite des 7 fichiers, sur une règle 🔴 — qui
    // reste découverte en attendant.
    expect(
      /\b(20\d{2}-\d{2}-\d{2}|phase\s*\d)/i.test(raison),
      `la dérogation de ${AIGUILLES} doit porter une date (AAAA-MM-JJ) ou une phase : ` +
        `« ${raison.slice(0, 60)}… »`,
    ).toBe(true);
  });

  it('aucune aiguille nommée', () => {
    const touchees = suivis.flatMap((f) => aiguilles(lu(f), empreintes));
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
