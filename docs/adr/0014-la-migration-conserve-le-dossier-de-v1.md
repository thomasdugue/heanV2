# 0014 — La migration conserve le dossier de V1, moins son cache de téléchargement

- **Date** : 2026-11-25
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-25, en **deux** réponses, et les deux sont citées parce
  que c'est ce qui rend une attribution revérifiable :
  1. « Je fais garder ton dossier V1 au lieu de le supprimer ? » → **« Oui, garde-le »** ;
  2. « Je fais supprimer le cache de 289 Mo en migrant, et garder le reste ? » → **« Oui, jette
     le cache »**.
  **La première a été écrite dans cet ADR avant que la seconde soit posée**, et la section
  « question ouverte » qui portait le cache est devenue la décision n° 2. C'est dit plutôt que
  lissé : un document qui prétend avoir toujours su ce qu'il sait est un document qu'on ne peut
  plus relire.
  Ce que ces deux réponses couvrent est **exactement** leur lettre. « Jette le cache » vise
  `smb_buffer/`, pas « les caches » : voir le périmètre ci-dessous, et ce qui n'a pas été
  arbitré.
  **La forme de la décision appartient à la session** : implémentation, tests, et le fait d'en
  faire un invariant. Même distinction que pour les [ADR 0010](0010-methode-de-migration-des-donnees-v1.md)
  et [0012](0012-un-echec-de-migration-doit-rester-rattrapable.md) : « Thomas a décidé ceci » est
  ce qui fait qu'une règle ne se rediscute pas, donc ça ne s'écrit que quand c'est vrai, et avec
  ses mots.
- **Remplace** : le quatrième temps de l'[ADR 0010](0010-methode-de-migration-des-donnees-v1.md),
  « puis supprimer l'original », ainsi que sa conséquence acceptée « un pic transitoire de
  ~283 Mo sur le disque du testeur » et son *écarté* « garder l'original au cas où : ce serait
  283 Mo dupliqués ». Le reste de l'ADR 0010 — copier à côté, vérifier, échanger, ne jamais
  fusionner deux bibliothèques — tient entièrement.

## Décision

**La migration de `~/Library/Application Support/noir/` vers `hean/` ne supprime pas le dossier
d'origine.** Il reste en place comme retour arrière vers la V1 — **à l'exception de son cache de
téléchargement `smb_buffer/`**, qui est supprimé après un échange réussi et vérifié.

C'est **INV-26**, tenu par deux mécanismes et non par une phrase :

- un test de forme refuse que l'appel de suppression reparaisse dans `migration/mod.rs`
  (`tests/architecture/migration-durable.test.ts`) ;
- un test de comportement rejoue la séquence réelle de démarrage **deux fois**
  (`src-tauri/tests/demarrage/retour_arriere.rs`) : la source survit intacte, et le second démarrage ne retouche
  à rien.

Et la migration **dit** où les données d'origine sont restées, et combien d'octets le cache a
rendus, par deux lignes de journal. Un retour arrière dont personne ne connaît l'existence n'en
est pas un.

### Le périmètre du cache, au mot près

**`noir/smb_buffer/` et rien d'autre.** C'est une suppression chez cinquante personnes : elle ne
s'étend pas par déduction.

- Trois autres fichiers de `noir/` portent le mot *cache* — `tracks_cache.json` (1,7 Mo),
  `added_dates_cache.json` (512 Ko), `network_scan_cache.json` (2,2 Mo) — et ne sont **pas**
  jetables : c'est l'état de la bibliothèque V1, et c'est `tracks_cache.json` qui porte les
  **3 443 pistes** de Thomas, donc c'est lui qui rend le retour arrière utile. Les supprimer
  viderait le filet qu'on vient de décider de garder, pour 4 Mo. Thomas ne les a pas arbitrés.
- **Après le succès, jamais avant.** La suppression du cache occupe, dans l'ordre d'INV-25, le
  rang qu'avait celle du dossier : rien ne se supprime tant que l'arrivée n'est pas vérifiée et
  durable. Le test de forme de `migration-durable.test.ts` tient cet ordre et distingue les deux
  suppressions, pour qu'INV-26 ne passe pas au vert sur la faute qu'il existe pour voir.
- **Un échec n'est pas fatal.** Un cache qui résiste coûte 289 Mio, pas une donnée, et faire
  échouer une migration pour de la place disque serait l'inverse de l'ADR 0012. Avertissement au
  journal, migration réussie — éprouvé en retirant le droit d'écrire sur `noir/`.
- **Ce que `smb_buffer/` est** : des fichiers téléchargés du NAS pendant la lecture progressive,
  nommés par empreinte, que V1 supprime elle-même ensuite. Ceux de Thomas avaient quatre jours :
  ce ménage n'avait pas eu lieu. Les reperdre ne perd rien, ils se retéléchargent. C'est la seule
  chose de `noir/` dont on puisse le dire.

## Pourquoi — et ce que la mesure a changé

L'ADR 0010 avait écarté cette option, sur un raisonnement explicite : garder l'original, ce
serait « 283 Mo dupliqués sur le disque ». **Cette prémisse est fausse, et c'est mesuré.**

Sur APFS, `std::fs::copy` ne recopie pas les octets : il **clone**. Mesuré le 2026-11-25 — copier
un fichier de 400 Mo n'a consommé **aucun** espace disque, et `du` annonce 400 Mo pour chacun des
deux exemplaires, parce qu'ils partagent leurs blocs. Les deux dossiers étant tous deux sous
`Application Support`, donc sur le même volume, c'est toujours ce qui se passe.

Donc, mesuré sur le dossier réel de Thomas le 2026-11-25 :

| | |
|---|---|
| `noir/` en entier | 572 Mio, 2 446 fichiers |
| migré vers `hean/` | 2 443 fichiers — **clonés**, donc partagés, comptés une seule fois |
| ce que `noir/` occupe encore **en propre** | **289 Mio**, soit son seul `smb_buffer/` (3 fichiers) |

Le coût réel de garder le dossier de V1 est donc **289 Mio, et c'est son cache SMB** — pas les
572 Mio apparents, et surtout pas une duplication des données de l'utilisateur.

Ce que la décision achète, en face : un testeur dont V2 se révélerait inutilisable peut
rouvrir V1 et retrouver ses playlists, son historique et ses sources NAS. Pendant une
reconstruction complète, avec une cinquantaine de testeurs, c'est le seul retour arrière qui ne
dépende pas d'une sauvegarde que personne n'a vérifiée.

## Ce que la décision ne dit toujours pas

**Rien ne nettoiera `noir/` plus tard.** Une fois le cache parti, ce qui reste — 283 Mio de
fichiers **clonés**, donc partagés avec `hean/` et gratuits — n'a ni échéance, ni purge
automatique, ni interface pour le supprimer. Le jour où V2 sera jugée sûre, ce sera une décision
à prendre, pas un comportement à attendre.

**Et les trois fichiers d'état de V1 n'ont pas été arbitrés.** Ils pèsent 4 Mo et portent la
bibliothèque de V1. On n'y touche pas, et la question ne se posera que si quelqu'un veut
récupérer ces 4 Mo — ce qui n'a pas de sens aujourd'hui.

## Conséquences acceptées

- **289 Mio restent occupés** sur le disque de chaque testeur migré, tant que la question
  ci-dessus n'est pas tranchée. Aucun mécanisme ne le rappelle : c'est assumé.
- **V1 reste utilisable** après la migration. Deux applications peuvent donc écrire dans deux
  dossiers de données différents, et ce qui est fait dans l'une n'apparaît pas dans l'autre. Ce
  n'est pas une fusion — l'ADR 0010 §6 l'interdit toujours — c'est une divergence, et c'est le
  prix d'un retour arrière. Le testeur qui rouvre V1 après avoir utilisé V2 perd ce qu'il a fait
  dans V2, et réciproquement.
- **Rien ne nettoie `noir/` plus tard.** Il n'y a ni échéance, ni purge automatique, ni interface
  pour le supprimer. Le jour où V2 sera jugée sûre, ce sera une décision à prendre, pas un
  comportement à attendre.
- **Le témoin devient le seul garde-fou contre une remigration.** Avant, deux choses empêchaient
  de remigrer : le témoin, et l'absence de source. Il n'en reste qu'une — d'où le test
  d'idempotence, qui exige `DejaFaite` et non `CibleDejaPeuplee` (`src-tauri/tests/migration.rs`).
  Et d'où INV-25, dont la gravité change avec cet ADR : le témoin certifie désormais une copie
  dont l'original survit, donc une copie trouée n'est plus une perte mais une bibliothèque
  incomplète que personne ne saurait diagnostiquer.

## Écartées

**Supprimer, mais seulement après N lancements réussis.** Le filet disparaîtrait tout seul, donc
au moment où l'on cesse d'y penser — et « réussi » n'a pas de définition ici : l'application
démarre très bien sur une bibliothèque incomplète. Un compteur qui décide d'une suppression est
un mécanisme qui se trompe en silence.

**Déplacer `noir/` dans la corbeille au lieu de le supprimer.** Même geste pour l'utilisateur,
mais la corbeille se vide — souvent sans qu'on regarde ce qu'il y a dedans — et macOS ne garantit
pas qu'un dossier de 572 Mio y entre sans copie. On troquerait une règle claire contre un
comportement dépendant du système.

**Le dire dans l'interface.** Souhaitable, et hors de portée aujourd'hui : il n'y a pas encore
d'interface où le dire, et l'échec de migration n'a lui-même pas de canal d'affichage (ADR 0012).
La ligne de journal est ce qui existe ; l'interface est une question pour la phase 3.
