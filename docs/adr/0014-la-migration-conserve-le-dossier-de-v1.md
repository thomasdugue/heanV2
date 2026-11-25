# 0014 — La migration conserve le dossier de V1

- **Date** : 2026-11-25
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-25. Question posée : « Je fais garder ton dossier V1 au
  lieu de le supprimer ? » Sa réponse, et c'est ce qui rend l'attribution revérifiable :
  **« Oui, garde-le »**.
  Ce qu'il a choisi est donc exactement cela — **ne rien supprimer** — et pas « supprimer sauf le
  cache ». La question du cache SMB est ouverte plus bas, elle ne se déduit pas de celle-ci.
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
d'origine.** Il reste en place, intact, comme retour arrière vers la V1.

C'est **INV-26**, tenu par deux mécanismes et non par une phrase :

- un test de forme refuse que l'appel de suppression reparaisse dans `migration/mod.rs`
  (`tests/architecture/migration-durable.test.ts`) ;
- un test de comportement rejoue la séquence réelle de démarrage **deux fois**
  (`src-tauri/tests/demarrage/retour_arriere.rs`) : la source survit intacte, et le second démarrage ne retouche
  à rien.

Et la migration **dit** où les données d'origine sont restées, par une ligne de journal. Un
retour arrière dont personne ne connaît l'existence n'en est pas un.

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

## Ce que la décision ne dit pas — question ouverte

**`smb_buffer/` vaut-il d'être supprimé, lui ?** Il pèse 289 Mio à lui seul, en trois fichiers
FLAC téléchargés depuis le NAS les 21 novembre. C'est un **cache reconstructible** : V1 était
censé le purger à chaque lancement (comportement W008, correctif de V1 au commit `eaee676`) et il
ne l'a pas fait. Le supprimer ne perdrait donc aucune donnée — les fichiers se retéléchargent — et
ramènerait le coût du retour arrière à **presque zéro**.

Mais Thomas a répondu « garde-le » à « garder ton dossier V1 », pas à « garder tout sauf le
cache ». La question est donc **posée et non tranchée** : elle est au tableau des décisions
ouvertes de [`README.md`](README.md).

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
