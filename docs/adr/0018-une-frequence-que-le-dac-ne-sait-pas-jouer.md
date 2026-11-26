# 0018 — Une fréquence que le DAC ne sait pas jouer : refuser, et le dire

- **Date** : 2026-11-26
- **État** : Acceptée
- **Décideur** : Thomas — **et c'est une décision qu'il peut vouloir annuler**, voir la fin
- **Suit** : [0002](0002-stack-technique.md), [0007](0007-format-interne-audio-et-preuve-du-bit-perfect.md)

## Contexte

Le portage du moteur de sortie a mis au jour un défaut de V1, **mesuré en lisant son code** et
écrit dans [`../MESURES-V1.md`](../MESURES-V1.md) §11 : quand le DAC ne gère pas la fréquence du
fichier, V1 en choisit une autre et joue quand même (`coreaudio_backend.rs:761`, appelé ligne
1226). Le HAL de macOS rééchantillonne pour combler l'écart, donc **le signal est altéré** — ce que
« bit-perfect » promet d'exclure — et le seul signe est un `println!` que l'app installée ne capture
pas. Un fichier 24/192 sur un DAC qui plafonne à 96 kHz est joué à 96 kHz, et personne ne le sait.

La question n'est pas technique. Techniquement, les trois comportements possibles coûtent le même
travail. Elle est **produit** : que doit-il se passer quand Hean ne peut pas tenir sa promesse ?

## Décision

**Hean refuse de jouer, et dit pourquoi en une phrase que Thomas peut lire.** Le message nomme le
périphérique, la fréquence du fichier et celles que le périphérique propose :

> *Le DAC ne sait pas jouer ce fichier à sa fréquence native (192 000 Hz) ; il ne propose que
> 44 100, 48 000, 88 200, 96 000 Hz. Hean ne rééchantillonne pas : la lecture est refusée plutôt
> que dégradée en silence.*

Mécanisme, et il existait **avant** cette décision : `BackendAudio::preparer()` rend le format
**réellement obtenu** et non `Ok(())` ([ADR 0007](0007-format-interne-audio-et-preuve-du-bit-perfect.md)
§5). Le backend CoreAudio ne « corrige » donc rien tout seul — il négocie, puis rend ce qu'il a
obtenu ; et quand l'écart existe, il **ne change pas la fréquence du périphérique du tout**, pour ne
pas laisser derrière lui un réglage que personne n'a demandé. C'est l'appelant qui constate et
refuse.

### L'exception AirPlay, qui n'en est pas une

En AirPlay, Hean **ne touche pas** à la fréquence du périphérique et joue à sa fréquence native.
Deux raisons qui se cumulent : changer la fréquence d'un périphérique AirPlay tue la session réseau
(**PLAT-1**, 🔴), et macOS y rééchantillonne de toute façon en interne. AirPlay n'est donc **jamais**
bit-perfect, et ça ne se corrige pas.

Ça se **dit** : la lecture a lieu, et l'app annonce que le son passe par un rééchantillonnage de
macOS. Refuser AirPlay serait perdre une fonction qui marche en V1 ; se taire serait la tromperie
que cet ADR existe pour fermer. La règle commune aux deux cas est donc plus simple que les deux
cas : **on ne rééchantillonne jamais sans le dire.**

## Alternatives écartées

**Rééchantillonner en silence, comme V1.** C'est le comportement d'aujourd'hui, donc l'option « ne
rien faire ». Écartée parce qu'elle ment : l'app continue d'afficher qu'elle est bit-perfect. Et le
mensonge est pire que le défaut — un défaut se trouve, une mesure fausse se croit. C'est la famille
de défaut la plus coûteuse de V1 (`MESURES-V1.md` §8) : ce qui ne s'annonce sur aucun canal.

**Rééchantillonner, mais l'afficher.** Défendable, et c'est l'option à rouvrir en premier si le
refus gêne Thomas à l'usage. Écartée pour maintenant pour deux raisons. La première est que le
rééchantillonneur n'existe pas et n'est pas dans le périmètre de la phase : « l'afficher » voudrait
dire aujourd'hui *laisser le HAL de macOS le faire* et l'annoncer — c'est-à-dire un
rééchantillonnage sur lequel nous n'avons aucune prise ni aucune mesure. La seconde est qu'un
avertissement qu'on voit à chaque lecture cesse d'être lu ; un refus, lui, se remarque une fois et
se répare (brancher l'autre DAC, ou choisir d'autoriser le rééchantillonnage).

**Demander à chaque fois.** Une boîte de dialogue au milieu d'un album. Non.

**Choisir un multiple exact (192 → 96 kHz) plutôt que n'importe quelle fréquence.** Un
rééchantillonnage par 2 est effectivement moins destructeur qu'un 192 → 44,1. Écartée : ça rend le
défaut plus discret sans le supprimer, et « moins faux » n'est pas une catégorie que ce projet sait
tenir — il faudrait alors dire *à quel point* c'est faux, et personne ne lirait.

## Conséquences acceptées

- **Un fichier peut ne pas se jouer sur un périphérique donné**, et c'est voulu. Le cas réel le plus
  probable est le haut-parleur interne du Mac face à un 24/192. Le message dit quoi faire.
- **Rien n'est encore réglable.** Il n'y a pas de bouton « autorise le rééchantillonnage » : la
  décision est câblée. Le jour où Thomas la veut réglable, c'est un ADR qui remplace celui-ci, pas
  une option qu'on ajoute en passant — parce qu'un réglage par défaut est une décision déguisée.
- **Hean ne laisse pas le périphérique dans un état qu'il n'a pas choisi.** La fréquence du DAC est
  remise à sa valeur d'origine à l'arrêt de la lecture, et elle n'est pas changée du tout quand la
  lecture est refusée.

## 🔶 Ce que Thomas pourrait vouloir annuler

Cet ADR tranche à l'aveugle sur un point : **personne n'a encore rencontré le cas.** Il n'est pas
dit que le DAC de Thomas refuse une seule de ses 3 421 pistes — s'il monte à 192 kHz, cet ADR ne se
déclenchera jamais et son coût est nul. Mais s'il s'avère qu'un dossier entier devient injouable sur
le haut-parleur du Mac en déplacement, le refus deviendra pénible, et l'option « rééchantillonner en
le disant » est celle à rouvrir. Le relevé à faire est écrit dans le protocole de test de la
session : jouer un 24/192 **sur le haut-parleur interne** et regarder ce que l'app répond.
