# Ce que les captures de la cible de parité ont montré

Prises le **2026-11-21 à 18 h 51-53** sur le commit `e12a200`, build de dev, bibliothèque réelle
(3 443 pistes, 91 % sur NAS). **C'était la première fois que ce design était lancé par qui que ce
soit.**

> 🗑 **Les huit images ne sont plus dans le dépôt**, retirées le 2026-11-27 avant la bascule en
> public : elles montraient une bibliothèque personnelle. Décision et alternatives écartées :
> [`0020`](../../../../docs/adr/0020-les-captures-sortent-du-depot.md). Ce fichier porte désormais
> seul ce qu'elles apprenaient — et c'est lui que la phase 4 lira. Pour en reprendre une série :
> [`PROTOCOLE-CAPTURES.md`](../../../../docs/PROTOCOLE-CAPTURES.md), destination **locale**, hors
> de git tant que le dépôt est public.

| Écran | Ce qu'il fallait y regarder |
|---|---|
| Accueil (`01-home`) | L'**eyebrow** (point pulsant + `HEAN · 21 NOV · 06:51 PM` + filet dégradé) et la **now-strip** : cover 200 px, titre en très gros, les 4 cellules hallmark (Bit depth / Sample / Format / Bit perfect), bouton play rond blanc. Puis `RECENT` **au-dessus** de « Recently Added » — le correctif `fc77789`. |
| Grille d'albums (`02-albums`) | ⚠️ La **première vignette débordait** : le titre commençait hors de la carte et se coupait. Défaut à ne pas reproduire. |
| Page album (`03-album-page`) | La structure hero / corps / aside, le watermark « AIR », les boutons Play album · Shuffle · Add to queue, l'aside Details. ⚠️ Chaque piste était **listée deux fois** — voir [`MESURES-V1.md`](../../../../docs/MESURES-V1.md) §10. |
| Menu contextuel (`04-menu-contextuel`) | Play · Add to queue · Add to playlist · Go to album · Go to artist · Info · Edit Metadata · Remove from library (en rouge). |
| Réglages, en drawer (`05-settings`) | Sections Audio / Library / Network library / Playback / Keyboard shortcuts. |
| Vue Pistes (`06-tracks`) | Les **badges de qualité ambrés** pour le hi-res (24-bit) contre gris pour le reste — une couleur hors de la règle monochrome, à documenter ou à trancher. Et beaucoup d'`Unknown Artist` / `-:--` : les pistes dont la lecture de métadonnées FLAC échoue au scan réseau. |
| Grille d'artistes (`07-artistes`) | Pochettes circulaires, index alphabétique à droite, compteur de pistes. |
| Page artiste (`08-artiste-page`) | Le **watermark** « PRINCE », la photo circulaire, les stats (albums / pistes / durée totale). |

## Ce qu'elles ont validé

- ✅ **Les `--glass-*` sont bien rétablies.** Bordures visibles sur la now-strip, les cellules
  hallmark, les sections de réglages ; l'élément actif de la sidebar se distingue. La régression de
  la PR #37 n'est pas dans la cible.
- ✅ **DM Sans se charge** — l'hypothèse « bloquée par la CSP » était fausse. Détail :
  [`../../ECARTS-DESIGN.md`](../../ECARTS-DESIGN.md) §2.

Ces deux verdicts tiennent sans les images : ils sont la raison pour laquelle la phase 0 a pu
démarrer sans attendre le design.

## Ce qu'elles ont révélé

Trois défauts, tous consignés dans [`MESURES-V1.md`](../../../../docs/MESURES-V1.md) :
l'extraction de couleur ambiante qui échoue à chaque page album (§9), 21 % de la bibliothèque en
double (§10), et le débordement de la première vignette d'album.

## ⏸ La série est en pause, pas abandonnée — décidé le 2026-11-21

Il manque : fullscreen player, fullscreen lyrics, panneaux Queue / Track Info / EQ / Lyrics, modals
(playlist, feedback, bulk edit, découverte NAS, confirm), les 6 étapes d'onboarding, la sidebar
repliée sous 900 px, le module d'indexation en cours, et la player bar en gros plan avec l'ambient
glow.

**Elles attendent volontairement la passe « davantage de DM Sans »** que Thomas fait côté design : la
typographie change sur tous ces écrans, les capturer maintenant reviendrait à les refaire.

**Quand la passe aura atterri** : réextraire `styles.css` / `views.js` / `tokens.css` depuis le
nouveau commit, écrire l'ADR qui remplace `0004`, puis **reprendre la série en entier** (les huit
d'alors comprises, puisqu'elles auraient vieilli de toute façon — c'est ce qui rend leur retrait peu
coûteux).

La liste complète et l'ordre sont dans
[`PROTOCOLE-CAPTURES.md`](../../../../docs/PROTOCOLE-CAPTURES.md).
