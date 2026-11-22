// Comportements de l'audio (phase 1) et du NAS (phase 5).
// Six des sept sont des pièges de plateforme : ils reviendront quelle que soit la techno.
import { describe, it } from 'vitest';
import { aPorter } from './_contrat';

describe('audio', () => {
  it('V007 — les touches media contrôlent Hean même si Apple Music tourne', () => aPorter('V007'));
  it('V008 — une session AirPlay survit à un changement de device', () => aPorter('V008'));
});

describe('réseau', () => {
  it('V009 — aucun dialogue Keychain au lancement', () => aPorter('V009'));
  it('W005 — l’UI ne gèle pas quand le NAS est injoignable', () => aPorter('W005'));
  it('W008 — les fichiers temporaires SMB ne s’accumulent pas', () => aPorter('W008'));
});
