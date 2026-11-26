// Les fonctions d'affichage de la coquille. Elles sont minuscules, et c'est justement pour
// ça qu'elles sont testées : une durée mal formatée ne fait échouer aucun autre test, elle
// se lit juste faux à l'écran — et personne ne regarde deux fois « 3:7 ».
import { describe, expect, it } from 'vitest';
import { compte, duree, frequence, qualite, specsSource } from '../../src/lib/ui/affichage';

describe('duree', () => {
  it('écrit les secondes sur deux chiffres', () => {
    expect(duree(187_000)).toBe('3:07');
    expect(duree(7_000)).toBe('0:07');
  });

  it('tronque au lieu d’arrondir', () => {
    // La somme des durées affichées doit égaler la durée affichée du total.
    expect(duree(239_600)).toBe('3:59');
  });

  it('passe aux heures quand il faut', () => {
    expect(duree(3_600_000)).toBe('1:00:00');
    expect(duree(4_271_000)).toBe('1:11:11');
  });

  it('ne rend jamais un nombre absurde', () => {
    expect(duree(0)).toBe('0:00');
    expect(duree(-1)).toBe('—');
    expect(duree(Number.NaN)).toBe('—');
  });
});

describe('qualite', () => {
  it('rend les deux champs quand ils sont là', () => {
    expect(qualite(24, 192_000)).toBe('24 bits / 192 kHz');
  });

  it('écrit la décimale à la française', () => {
    expect(qualite(16, 44_100)).toBe('16 bits / 44,1 kHz');
  });

  it('se contente de ce qu’elle a', () => {
    // Un MP3 n'a pas de profondeur de bits : la ligne ne doit pas commencer par « / ».
    expect(qualite(null, 44_100)).toBe('44,1 kHz');
    expect(qualite(24, null)).toBe('24 bits');
    expect(qualite(null, null)).toBe('');
  });
});

describe('compte', () => {
  it('accorde le pluriel', () => {
    expect(compte(1, 'piste')).toBe('1 piste');
    expect(compte(306, 'piste')).toBe('306 pistes');
    expect(compte(0, 'piste')).toBe('0 pistes');
  });

  it('accepte un pluriel irrégulier', () => {
    expect(compte(2, 'illisible', 'illisibles')).toBe('2 illisibles');
  });
});

describe('frequence', () => {
  it('écrit les kHz à la française, au plus une décimale', () => {
    expect(frequence(44100)).toBe('44,1 kHz');
    expect(frequence(48000)).toBe('48 kHz');
    expect(frequence(192000)).toBe('192 kHz');
    expect(frequence(88200)).toBe('88,2 kHz');
  });

  it('refuse de rendre un chiffre quand il n’y en a pas', () => {
    // Zéro arrive réellement : avant la première lecture, l'état partagé est à zéro.
    // Afficher « 0 kHz » laisserait croire à une mesure, alors qu'il n'y en a pas encore.
    expect(frequence(0)).toBe('—');
    expect(frequence(-1)).toBe('—');
    expect(frequence(Number.NaN)).toBe('—');
  });
});

describe('specsSource', () => {
  it('écrit les deux champs de l’indicateur', () => {
    expect(specsSource(192000, 24)).toBe('192 kHz / 24 bits');
    expect(specsSource(44100, 16)).toBe('44,1 kHz / 16 bits');
  });

  it('se passe de la profondeur quand le conteneur ne la déclare pas', () => {
    // Un MP3 n'a pas de profondeur d'origine : le backend envoie 0, et « 0 bits » serait
    // une affirmation fausse là où il n'y a rien à dire.
    expect(specsSource(44100, 0)).toBe('44,1 kHz');
  });
});
