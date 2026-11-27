// La part réseau, vue du frontend (ADR 0016).
//
// **Aucun de ces tests ne cite un identifiant du contrat**, et c'est délibéré. Les trois
// comportements réseau du contrat — V009 (aucun dialogue Keychain au lancement), W005 (l'UI
// ne gèle pas quand le NAS est injoignable) et W007 (une bibliothèque uniquement NAS
// s'affiche sur la home) — restent **à porter**, chacun pour une raison précise :
//
//   - **V009** : rien ne touche au trousseau au lancement, et c'est tenu côté Rust
//     (`network::identifiants` n'est appelé qu'au scan). Mais l'affirmer demande de mesurer
//     sur une build **signée**, puisque l'ACL d'une entrée Keychain est liée à la signature
//     du binaire et pas au bundle identifier (ADR 0015). Phase 6.
//   - **W005** : la cause principale est fermée — le scan ne tient pas le verrou de la
//     bibliothèque pendant ses minutes de parcours (ADR 0016 §5). Mais « l'UI ne gèle pas »
//     se mesure sur l'UI, pas sur un verrou, et il n'y a pas encore d'écran à mesurer.
//   - **W007** : ce qui suit prouve que la couche d'affichage tient une bibliothèque
//     **entièrement** distante. Ce qu'elle ne prouve pas, c'est « sur la home » : la home
//     n'existe pas (phase 3/4). Le porter maintenant serait annoncer plus que ce qui est fait
//     — et c'est exactement ce que `check-comportements` existe pour empêcher.
//
// Ce qui est éprouvé ici est donc la substance disponible : que rien dans le frontend ne
// suppose un chemin de fichier local, et que les deux chemins de scan partagent leur
// condition de rechargement au lieu de la recopier.
import { beforeEach, describe, expect, it } from 'vitest';
import { creerDepot } from '../../src/lib/library/repository';
import { bibliotheque, reinitialiserPourTest } from '../../src/lib/library/etat';
import { grouperEnAlbums, grouperParArtiste } from '../../src/lib/library/groupement';
import { creerFauxBackend, piste } from './_faux-backend';

beforeEach(() => reinitialiserPourTest());

/** L'identifiant réel de la source de Thomas, et la forme réelle de ses chemins. */
const SOURCE = 'dcdd24af-ca15-48a4-adde-18b5fcfa92f0';
const PREFIXE = `smb://${SOURCE}/music/LOSSLESS/complete`;

const UNIQUEMENT_NAS = [
  piste({
    path: `${PREFIXE}/To Pimp a Butterfly (2015)/16 - Mortal Man.mp3`,
    titre: 'Mortal Man',
    artiste: 'Kendrick Lamar',
    album: 'To Pimp a Butterfly',
    cle_album: 'To Pimp a Butterfly',
  }),
  piste({
    path: `${PREFIXE}/To Pimp a Butterfly (2015)/07 - Alright.mp3`,
    titre: 'Alright',
    artiste: 'Kendrick Lamar',
    album: 'To Pimp a Butterfly',
    cle_album: 'To Pimp a Butterfly',
  }),
];

describe('réseau', () => {
  it('une bibliothèque entièrement distante se charge et se regroupe', async () => {
    const backend = creerFauxBackend(UNIQUEMENT_NAS);
    const depot = creerDepot(backend);
    await depot.charger();

    // La substance de W007 : rien dans cette couche ne suppose un chemin local. En V1, la
    // home « supposait au moins une source locale » — et 91 % de la bibliothèque de Thomas
    // est distante, donc cette supposition était fausse pour l'essentiel de sa musique.
    expect(bibliotheque.pistes).toHaveLength(2);
    expect(bibliotheque.pistes.every((p) => p.path.startsWith('smb://'))).toBe(true);
    // Le lookup par chemin (B13) marche avec une URI comme avec un chemin de fichier :
    // l'identité d'une piste est son chemin, quel qu'il soit.
    expect(bibliotheque.parChemin.get(UNIQUEMENT_NAS[0]!.path)?.titre).toBe('Mortal Man');

    const albums = grouperEnAlbums(bibliotheque.pistes);
    expect(albums).toHaveLength(1);
    expect(albums[0]?.pistes).toHaveLength(2);
    const artistes = grouperParArtiste(bibliotheque.pistes);
    expect(artistes).toHaveLength(1);
    expect(artistes[0]?.albums).toHaveLength(1);
  });

  it('un scan réseau ne recharge la liste que s’il a trouvé quelque chose', async () => {
    // La même condition que V004, et c'est le point : **la même fonction**. Avec deux
    // chemins de scan, deux copies de la condition seraient la famille de défaut de V1 —
    // celle qu'on oublie de mettre à jour est celle qui compte, et ici c'est celle qui porte
    // 91 % de la bibliothèque.
    const backend = creerFauxBackend(UNIQUEMENT_NAS);
    const depot = creerDepot(backend);
    await depot.charger();
    await depot.ajouterUneSourceReseau('nas', 'nas.local', 'music', '/LOSSLESS/complete', 'admin');
    const [source] = await depot.sourcesReseau();
    expect(source).toBeDefined();
    expect(backend.appels['listTracks']).toBe(1);

    backend.prochainRapport = { ajoutees: 0, modifiees: 0, disparues: 0, illisibles: 0 };
    await depot.scannerUneSourceReseau(source!.id);
    expect(backend.appels['listTracks']).toBe(1);

    // Des fichiers illisibles ne sont pas un changement : sur un NAS c'est le cas courant
    // (42 des 3 137 entrées du cache de V1 n'avaient ni codec ni durée).
    backend.prochainRapport = { ajoutees: 0, modifiees: 0, disparues: 0, illisibles: 42 };
    await depot.scannerUneSourceReseau(source!.id);
    expect(backend.appels['listTracks']).toBe(1);

    backend.prochainRapport = { ajoutees: 3137, modifiees: 0, disparues: 0, illisibles: 0 };
    await depot.scannerUneSourceReseau(source!.id);
    expect(backend.appels['listTracks']).toBe(2);
  });

  it('ajouter deux fois le même partage ne fait qu’une source', async () => {
    // L'identifiant est dérivé de l'hôte, du partage et du chemin (ADR 0016) : un UUID
    // donnerait deux sources pour les mêmes fichiers, donc une bibliothèque en double.
    const depot = creerDepot(creerFauxBackend());
    await depot.ajouterUneSourceReseau('n', 'nas.local', 'music', '/Albums', 'moi');
    const deux = await depot.ajouterUneSourceReseau('n', 'nas.local', 'music', '/Albums', 'moi');
    expect(deux).toHaveLength(1);
  });

  it('un mot de passe descend et ne remonte jamais', async () => {
    // La propriété qui compte : aucune méthode du dépôt ne rend un mot de passe. S'il
    // remontait, il apparaîtrait dans les outils de développement de la WebView (ADR 0015).
    const backend = creerFauxBackend();
    const depot = creerDepot(backend);
    await depot.ajouterUneSourceReseau('n', 'nas.local', 'music', '/Albums', 'moi');
    const [source] = await depot.sourcesReseau();

    await depot.enregistrerLeMotDePasse(source!.id, 'ceci-est-un-secret');
    expect(backend.motsDePasse.get(source!.id)).toBe('ceci-est-un-secret');

    // Rien de ce que l'interface peut lire ne contient le secret.
    const lisible = JSON.stringify(await depot.sourcesReseau());
    expect(lisible).not.toContain('ceci-est-un-secret');
    expect(Object.keys(source!)).not.toContain('mot_de_passe');
  });

  it('un scan sur une source inconnue remonte une erreur au lieu de ne rien faire', async () => {
    // Un échec silencieux est ce que V1 faisait avec `unwrap_or_default()` : l'utilisateur
    // voyait « rien ne se passe » et personne ne pouvait diagnostiquer. Ici l'erreur remonte
    // jusqu'à l'écran, telle quelle.
    const depot = creerDepot(creerFauxBackend());
    await expect(depot.scannerUneSourceReseau('source-qui-nexiste-pas')).rejects.toThrow(
      /identifiant source-qui-nexiste-pas/,
    );
  });
});
