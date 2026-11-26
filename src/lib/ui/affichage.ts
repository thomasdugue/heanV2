// Les quelques fonctions d'affichage de la coquille. Pures, donc testées sans DOM.
//
// Elles sont ici et pas dans un composant parce que « 3:07 » et « 24 bits / 192 kHz » sont
// exactement le genre de petit calcul qu'on finit par réécrire dans trois vues — la famille
// de défaut la plus coûteuse de V1 (une normalisation écrite plusieurs fois, ADR 0009). Un
// seul endroit, et un test unitaire dessus.

/**
 * Une durée en millisecondes, rendue `m:ss` (ou `h:mm:ss` au-delà de l'heure).
 *
 * Les secondes sont TRONQUÉES, pas arrondies : un morceau de 3 min 59,6 s s'affiche
 * « 3:59 » et non « 4:00 », pour que le total d'un album corresponde à la somme de ce qui
 * est écrit en face de chaque piste.
 */
export function duree(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return '—';
  const total = Math.floor(ms / 1000);
  const s = total % 60;
  const m = Math.floor(total / 60) % 60;
  const h = Math.floor(total / 3600);
  const ss = String(s).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${m}:${ss}`;
}

/**
 * La qualité d'un fichier en une ligne : `24 bits / 192 kHz`. Les deux champs sont
 * optionnels côté backend (un MP3 n'a pas de profondeur de bits), et chacun s'affiche
 * seul si l'autre manque.
 */
export function qualite(bits: number | null, echantillonnage: number | null): string {
  const morceaux: string[] = [];
  if (bits !== null && bits > 0) morceaux.push(`${bits} bits`);
  if (echantillonnage !== null && echantillonnage > 0) {
    // kHz avec au plus une décimale : 44 100 → « 44,1 kHz », 192 000 → « 192 kHz ».
    const khz = echantillonnage / 1000;
    morceaux.push(`${(Math.round(khz * 10) / 10).toString().replace('.', ',')} kHz`);
  }
  return morceaux.join(' / ');
}

/** Un compte avec son nom au pluriel accordé : `1 piste`, `306 pistes`. */
export function compte(n: number, singulier: string, pluriel = `${singulier}s`): string {
  return `${n} ${n === 1 ? singulier : pluriel}`;
}

/**
 * Une fréquence d'échantillonnage, écrite court : `44,1 kHz`, `192 kHz`.
 *
 * Même règle que `qualite` ci-dessus — au plus une décimale, virgule française — et c'est
 * pour ça que les deux partagent ce fichier plutôt que de vivre dans les composants : V1 a
 * payé trois bugs pour avoir réécrit une normalisation à plusieurs endroits (ADR 0009).
 */
export function frequence(hz: number): string {
  if (!Number.isFinite(hz) || hz <= 0) return '—';
  const khz = hz / 1000;
  return `${(Math.round(khz * 10) / 10).toString().replace('.', ',')} kHz`;
}

/**
 * Les specs de la source, telles que l'indicateur les montre : `192 kHz / 24 bits`.
 * Équivalent du champ `source-specs` de V1 (`playback.js:878`).
 */
export function specsSource(hz: number, bits: number): string {
  const morceaux = [frequence(hz)];
  if (bits > 0) morceaux.push(`${bits} bits`);
  return morceaux.join(' / ');
}
