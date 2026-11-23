// APPARIEMENT (niveau 2) — le contrat de 24 lignes et les tests qui le couvrent.
//
// Ce test-ci, lui, DOIT être vert : il garantit qu'aucun comportement du contrat n'a été
// oublié, et qu'aucun test ne cite un comportement disparu. C'est ce qui empêche le tableau
// de bord de mentir.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { CONTRAT } from './_contrat';

const ICI = import.meta.dirname;

/**
 * Les comportements cités par un test, **qu'il soit porté ou non**.
 *
 * La citation est le TITRE du test, `it('V004 — …')`, et non l'appel à `aPorter` : porter
 * un comportement consiste précisément à remplacer cet appel par une vraie assertion, et
 * un appariement qui ne reconnaîtrait que les non-portés déclarerait « comportement
 * perdu » à chaque progrès du projet. Le titre, lui, reste.
 */
function idsCites(): Set<string> {
  const out = new Set<string>();
  for (const f of fichiersDeTest()) {
    for (const m of readFileSync(join(ICI, f), 'utf8').matchAll(/^\s*it\('([VW]\d{3})\b/gm)) {
      out.add(m[1] as string);
    }
  }
  return out;
}

function fichiersDeTest(): string[] {
  return readdirSync(ICI).filter((x) => x.endsWith('.test.ts') && x !== 'contrat-complet.test.ts');
}

/**
 * Les tests de comportement d'un fichier, avec leur corps : du `it('Vxxx` jusqu'au `it(`
 * suivant, ou jusqu'à la fin du fichier. Le découpage est fait par indices et non par une
 * borne de fin d'expression régulière, parce que `\\Z` n'existe pas en JavaScript : la
 * première version de ce détecteur laissait donc le dernier test de chaque fichier
 * entièrement hors de son examen.
 */
export function corpsDesTests(texte: string): [string, string][] {
  const debuts = [...texte.matchAll(/^[^\S\n]*it\(/gm)].map((m) => m.index);
  const out: [string, string][] = [];
  for (const [i, debut] of debuts.entries()) {
    const corps = texte.slice(debut, debuts[i + 1] ?? texte.length);
    const id = /^[^\S\n]*it\('([VW]\d{3})/.exec(corps)?.[1];
    if (id) out.push([id, corps]);
  }
  return out;
}

/** Les comportements encore marqués « à porter ». Le complément, ce sont les portés. */
function idsAPorter(): Set<string> {
  const out = new Set<string>();
  for (const f of fichiersDeTest()) {
    for (const m of readFileSync(join(ICI, f), 'utf8').matchAll(/aPorter\('([VW]\d{3})'\)/g)) {
      out.add(m[1] as string);
    }
  }
  return out;
}

describe('contrat anti-régression', () => {
  it('compte bien 24 comportements', () => {
    expect(CONTRAT).toHaveLength(24);
  });

  it('chaque comportement du contrat a un test', () => {
    const cites = idsCites();
    expect(CONTRAT.filter((x) => !cites.has(x.id)).map((x) => x.id)).toEqual([]);
  });

  it('aucun test ne cite un comportement absent du contrat', () => {
    const connus = new Set(CONTRAT.map((x) => x.id));
    expect([...idsCites()].filter((id) => !connus.has(id))).toEqual([]);
    // Et un `aPorter('V0xx')` oublié sur un comportement disparu du contrat lèverait
    // au lancement du test ; ici on le refuse aussi à la lecture.
    expect([...idsAPorter()].filter((id) => !connus.has(id))).toEqual([]);
  });

  it('un comportement porté contient une vraie assertion', () => {
    // Le complément nécessaire de la citation par le titre : `it('V005 — …', () => {})`
    // serait compté PORTÉ et passerait au vert, sans rien vérifier. Relevé par
    // invariant-guardian à la relecture de la phase 2 — c'est la deuxième moitié de la
    // question « ce comportement est-il vraiment tenu ? », et la première ne suffit pas.
    const sans = fichiersDeTest().flatMap((f) =>
      corpsDesTests(readFileSync(join(ICI, f), 'utf8'))
        .filter(([, corps]) => !corps.includes('aPorter(') && !corps.includes('expect('))
        .map(([id]) => `${id} (${f})`),
    );
    expect(sans, 'un test porté sans assertion est vert pour rien').toEqual([]);
  });

  it('le détecteur de test vide repère un corps sans assertion', () => {
    // Le détecteur mis à l'épreuve sur un échantillon fautif, plutôt que supposé. Le
    // SECOND cas compte autant que le premier : le découpage se faisait d'abord par une
    // expression régulière dont la borne de fin n'existe pas en JavaScript (`\Z`), donc
    // le DERNIER test d'un fichier n'était pas examiné du tout — et c'est justement là
    // qu'on ajoute un test.
    const vide = corpsDesTests("  it('V005 — on peut glisser une piste', () => {});\n");
    expect(vide).toHaveLength(1);
    expect(vide[0]?.[1].includes('expect(')).toBe(false);

    const deux = corpsDesTests(
      "  it('V005 — a', () => { expect(1).toBe(1); });\n  it('V006 — b', () => {});\n",
    );
    expect(deux.map(([id]) => id)).toEqual(['V005', 'V006']);
    expect(deux[1]?.[1].includes('expect(')).toBe(false);
  });

  it('le tableau de bord de ./check ne peut pas mentir', () => {
    // `./check` déduit le nombre de comportements PORTÉS du nombre de tests rouges :
    // 24 moins les rouges. Ça ne tient que si chaque comportement a exactement un test et
    // si ce fichier-ci, l'appariement, est vert — c'est pourquoi il est aussi dans
    // `npm run test`, donc dans le verdict, et pas seulement dans le tableau de bord.
    const portes = CONTRAT.filter((x) => !idsAPorter().has(x.id)).map((x) => x.id);
    expect(idsCites().size).toBe(CONTRAT.length);
    expect(portes.length + idsAPorter().size).toBe(CONTRAT.length);
  });
});
