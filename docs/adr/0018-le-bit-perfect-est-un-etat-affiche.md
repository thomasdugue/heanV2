# 0018 — Le bit-perfect est un état affiché, pas une propriété permanente

- **Date** : 2026-11-26
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-26, en corrigeant une analyse fausse de la session
  d'orchestration (voir § « Ce qui a provoqué cet ADR »).
- **Complète** : la formule « bit-perfect = pas de rééchantillonnage », qui circule dans ce
  dépôt depuis le 2026-11-21 sans ADR qui la porte, **et la prescription de l'ADR 0007 § 5**
  (voir plus bas). Ne remplace rien.

## La décision, dans les mots de Thomas

> « Tout n'est pas tout le temps en bit-perfect. C'est uniquement bit-perfect quand le DAC est
> capable de décoder. Dans la V1, quand il est en bit-perfect, le player l'indiquait. Quand il
> y avait un downgrade, le player indiquait quel était l'échantillonnage entrant et quel était
> l'échantillonnage de sortie, pour que l'utilisateur sache s'il y a une altération. »

Donc : **le repli de fréquence est un comportement voulu, et l'affichage de l'écart est ce qui
le rend acceptable.** Ce qui est interdit n'est pas de rééchantillonner — c'est de le faire
**sans le dire**. Un lecteur qui annonce « bit-perfect » alors qu'il rééchantillonne mentirait ;
un lecteur qui refuse de jouer un fichier que le DAC ne gère pas nativement serait inutilisable.

## Ce que V1 fait, vérifié des deux côtés de la chaîne

| où | quoi |
|---|---|
| `audio/coreaudio_backend.rs:761` (V1) | `find_best_supported_rate` : correspondance exacte, sinon **la plus petite fréquence supportée ≥ source**, sinon la plus haute disponible |
| `audio_engine.rs:546` (V1) | `is_mismatch = source_sr != output_sr`, émis avec les deux fréquences et la profondeur dans l'événement `playback_audio_specs` |
| `src/playback.js:873-891` (V1) | affiche **source** (`192kHz/24bit`) et **sortie** ; classe `bit-perfect` + animation quand elles coïncident, classe `resampled` (cyan, **volontairement pas rouge** — le repli fonctionne, ce n'est pas une panne) et flèche sinon |
| `src/fullscreen-player.js:337-347` (V1) | recopie cet état dans le lecteur plein écran |

**Le mécanisme est en Rust, l'honnêteté du mécanisme est en JS.** C'est la raison pour laquelle
l'analyse qui n'a lu que le Rust a conclu l'inverse de la vérité.

## Conséquence sur la formule qui circulait

Le skill `hean-invariants` et l'[ADR 0007](0007-format-interne-audio-et-preuve-du-bit-perfect.md)
(ligne 11) énoncent **« bit-perfect = pas de rééchantillonnage »** comme un interdit qui ne se
rouvre pas, et l'attribuent à l'[ADR 0002](0002-stack-technique.md). **L'ADR 0002 ne dit pas
ça** : il tranche CoreAudio contre CPAL, et ne parle pas de fréquence d'échantillonnage.
La formule était donc un interdit sans source, plus absolu que la décision réelle.

Lue au pied de la lettre par une session qui porte la sortie audio, elle aurait fait refuser le
repli — donc supprimer une fonction que Thomas veut. Elle est reformulée, pas supprimée :

> **Jamais de rééchantillonnage par confort, par optimisation ou par défaut de conception. Le
> repli de fréquence est permis quand le périphérique ne sait pas faire autrement, et il
> s'affiche : fréquence entrante, fréquence sortante, et pas de badge bit-perfect.**

L'ADR 0007 n'est pas modifié — il ne se modifie pas. Sa ligne 55 tenait d'ailleurs déjà la
bonne forme pour le volume : « bit-perfect si, et seulement si, le volume est à 100 % ». Le
bit-perfect y était **déjà** conditionnel ; seule la condition de fréquence manquait.

### Et une phrase de l'ADR 0007 prescrit l'inverse, elle aussi

Relevé par la relecture d'invariants de la session CoreAudio, et vérifié : l'ADR 0007 § 5 ne
se contente pas de citer la formule, **il en tire une consigne** — « Rendre le format obtenu
permet de *constater* l'écart, donc **de le refuser** — puisque le bit-perfect interdit de
rééchantillonner. »

Cette phrase-là est plus dangereuse que la formule, parce qu'elle dit quoi *faire*. Lue par
une session de phase 4 ou 5 qui trouverait ensuite le repli dans le code, elle lui ferait
**supprimer le repli en croyant corriger une faute**.

Ce qui vaut, et qui remplace la fin de cette phrase :

> Rendre le format obtenu permet de **constater** l'écart, donc de **l'afficher** — fréquence
> entrante, fréquence sortante, et pas de badge bit-perfect. Le refuser serait refuser de
> jouer le fichier.

La première moitié reste juste, et c'est elle qui compte : c'est parce que `preparer()` rend
le format réellement obtenu que l'écart est connu, donc affichable. **Le mécanisme était bon,
seule la conclusion était trop raide.** L'ADR 0007 n'est toujours pas modifié : c'est ici que
la correction vit, et l'index des ADR est le chemin qui y mène.

## Ce qui a provoqué cet ADR

La session d'orchestration a lu `coreaudio_backend.rs:761`, n'a pas lu `playback.js`, et a
écrit à Thomas que « le son est altéré et rien ne te le dit », puis a demandé à la session
CoreAudio d'inscrire le repli comme défaut mesuré et de le corriger. Thomas a corrigé l'erreur.

Deux fautes du même jour, de la même forme — une conclusion tirée d'un seul bout de la chaîne
(l'autre : un coût de ressaisie de mot de passe annoncé sans regarder le trousseau, où il était
déjà payé). D'où la règle écrite dans `hean-invariants` : **avant de qualifier un comportement
de V1 de défaut, lire la chaîne entière et citer `fichier:ligne` des deux côtés.** Et son
corollaire, demandé par Thomas : **quand il décrit un comportement de V1, le vérifier comme
n'importe quelle autre affirmation et lui dire ce qui diffère.** Appliqué le jour même — il a
dit « on passe à l'échantillonnage inférieur » là où le code cherche d'abord la fréquence
**supérieure**.

## Question ouverte, qui appartient à Thomas

L'affichage met **toujours** une flèche vers le bas, alors que le repli choisit en priorité une
fréquence **supérieure**. Un 44,1 kHz joué à 48 s'affiche donc « 48 kHz ↓ ». Les deux chiffres
restent visibles, donc l'information n'est pas fausse — seule la flèche l'est. Cosmétique, non
mesuré comme gênant, **à ne pas corriger en passant** (règle « porter d'abord »).

## Alternatives écartées

| Alternative | Pourquoi écartée |
|---|---|
| **Refuser de jouer un fichier que le DAC ne gère pas nativement** | C'est ce que la formule absolue impliquait. Un lecteur qui refuse de jouer est inutilisable, et le DAC de Thomas fixe alors le catalogue qu'il peut écouter. |
| **Rééchantillonner sans l'afficher** | La faute réelle, et la seule que cet ADR interdit : l'app annoncerait une qualité qu'elle ne fournit pas. |
| **Un nouvel ADR qui remplace l'ADR 0007** | 0007 n'est pas en cause : il décrit le format interne et la preuve du bit-perfect, et sa ligne 55 va déjà dans ce sens. Seule la formule qu'il cite de 0002 était trop large. |
