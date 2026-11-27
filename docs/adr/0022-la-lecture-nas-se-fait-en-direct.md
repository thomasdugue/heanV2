# 0022 — La lecture d'une piste du NAS se fait en direct, sans fichier temporaire

- **Date** : 2026-11-28
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-28 — « Lire directement sur le NAS ».
- **Complète** : [`0016`](0016-la-part-reseau-de-la-bibliotheque.md)

## Contexte

**91 % de la bibliothèque de Thomas est sur SMB** — 3 137 pistes sur 3 443, mesuré
([`../MESURES-V1.md`](../MESURES-V1.md) §1). Le scan réseau existe depuis l'ADR
[0016](0016-la-part-reseau-de-la-bibliotheque.md) : ces pistes sont donc visibles, avec leurs
tags et leur taille. **Mais aucune ne joue.** Deux raisons, et aucune n'est un oubli :
`audio/decode.rs` ouvre un `File` local, et `App.svelte` écarte explicitement les chemins
`smb://` plutôt que de laisser le moteur rendre « fichier illisible », une erreur juste pour
une mauvaise raison.

Autrement dit, le lecteur joue aujourd'hui **9 % de la bibliothèque**. C'est le plus grand
écart qui reste entre la coquille et un lecteur utilisable.

## Décision

Une piste du NAS se lit **en direct sur le partage**, par lectures à une position donnée.
**Rien n'est écrit sur le disque de l'utilisateur.**

La longueur du flux est **demandée au partage**, dans la même ouverture que la première
lecture — un positionnement en fin de fichier, qui ne coûte donc aucun tour de réseau de plus
— et non reprise de la bibliothèque. Elle est donc connue avant la première
lecture, exacte, et constante pendant toute la piste : c'est ce qui rend le déplacement dans
une piste distante juste par construction.

**Pourquoi pas la taille stockée en bibliothèque, qui aurait évité ce tour de réseau** : elle
peut être périmée, et le projet le sait déjà. Là où le serveur ne répond pas à `readdirplus`,
une piste déjà connue n'est jamais relue, donc un fichier modifié depuis une autre machine
garde sa vieille taille (ADR [0016](0016-la-part-reseau-de-la-bibliotheque.md) §4, inscrit aux
« Bugs connus »). Une longueur fausse ne donne pas une erreur lisible : elle donne un
déplacement qui vise à côté et une fin de piste tronquée. Le tour de réseau achète donc une
propriété, pas du confort — et il évite au passage que le moteur audio dépende de la
bibliothèque.

## Ce que la plateforme impose, et qui n'est pas un choix

`pavao::SmbFile<'_>` **emprunte** le `SmbClient`, et ce client vit derrière le mutex de
process qu'impose `libsmbclient` (PLAT-2, 🔴 — deux clients concurrents donnent EINVAL). Un
handle de fichier ne peut donc **pas** être gardé ouvert entre deux prises du verrou : chaque
lecture est un cycle complet « verrou → ouvrir → se positionner → lire → fermer ».

Conséquence directe, et c'est elle qu'il faut retenir : **un cache de blocs en lecture
anticipée n'est pas une optimisation, c'est une condition de fonctionnement.** Symphonia
demande de petites tranches ; une ouverture SMB par petite tranche serait inutilisable. On lit
donc par blocs larges et on sert les petites demandes depuis la mémoire.

## Le risque, et comment il se mesure

Le risque est le réseau : si le débit moyen ne suit pas, le son troue. Le tampon circulaire
de 5 secondes absorbe la latence tant que le débit moyen suffit — c'est la latence qui est
absorbée, pas un débit insuffisant.

**Ce risque se mesure au lieu de se discuter**, et c'est ce qui a permis de trancher : le
compteur de trous existe et vient d'être éprouvé sur le matériel de Thomas le 2026-11-27 —
3 min 15 s d'un morceau d'affilée, zéro trou, relevé dans le journal et non déduit. Une piste
du NAS qui troue le dira donc d'elle-même, chiffré, sur le vrai NAS. Si le verdict est
mauvais, l'alternative écartée ci-dessous redevient le repli — mais sur une mesure, pas sur
une intuition.

## Alternatives écartées

**Porter le téléchargement progressif de V1.** Un fil de fond télécharge le morceau par
tranches de 64 Kio dans `smb_buffer/{hash}.tmp`, la lecture démarre à 4 Mio reçus, et
`SmbProgressiveFile` lit le fichier temporaire avec des attentes bloquantes. Éprouvé des mois
en production, donc son comportement réseau est connu.

Écarté sur un **défaut mesuré**, ce que la règle « porter d'abord, améliorer ensuite » exige
pour autoriser une reprise de conception : **le nettoyage de `smb_buffer` ne nettoyait pas**
— l'un des trois bugs trouvés le 2026-11-21 en instrumentant V1, et l'un des trois qui étaient
**silencieux en production** ([`../MESURES-V1.md`](../MESURES-V1.md) §7). Le défaut n'est pas
dans le choix de télécharger : il est dans le fait que chaque écoute laisse un fichier, donc
qu'il faut un ménage, donc qu'il y a un ménage à rater. La lecture directe fait disparaître la
classe entière plutôt que son instance.

Deux autres différences, notées pour ne pas les redécouvrir : la longueur annoncée à Symphonia
était le nombre d'octets **déjà téléchargés**, donc une valeur qui grandit pendant la lecture,
là où la nôtre est la taille réelle connue d'avance ; et chaque écoute payait l'écriture puis
la relecture du morceau sur le disque.

**Faire les deux, avec repli automatique sur le téléchargement.** Le plus robuste sur le
papier. Écarté parce que c'est deux chemins à écrire, à éprouver et à maintenir au lieu d'un,
et que l'accumulation de chemins parallèles est la cause racine que le postmortem de V1
désigne. Si la mesure l'exige, on changera de chemin — on n'en gardera pas deux.

## Ce que cet ADR ne tranche pas

- **Le gapless entre deux pistes du NAS.** V1 préchargeait 60 s avant la fin sur SMB contre
  10 s en local, pour couvrir la latence. La valeur juste pour la lecture directe est à
  **mesurer** une fois qu'elle marche, pas à recopier.
- **Les pochettes depuis le NAS**, qui restent à porter.
- **La taille des blocs lus et la profondeur de l'anticipation.** Ce sont des réglages, donc
  ils se mesurent sur le NAS de Thomas et s'écrivent avec leur mesure (INV-28).
