# Captures de la cible de parité

Prises le **2026-11-21 à 18 h 51-53** sur le commit `e12a200`, build de dev, bibliothèque réelle
(3 443 pistes, 91 % sur NAS). **C'est la première fois que ce design était lancé par qui que ce
soit.**

| Fichier | Écran | Ce qu'il faut y regarder |
|---|---|---|
| `01-home.png` | Accueil | L'**eyebrow** (point pulsant + `HEAN · 21 NOV · 06:51 PM` + filet dégradé) et la **now-strip** : cover 200 px, titre en très gros, les 4 cellules hallmark (Bit depth / Sample / Format / Bit perfect), bouton play rond blanc. Puis `RECENT` **au-dessus** de « Recently Added » — le correctif `fc77789`. |
| `02-albums.png` | Grille d'albums | ⚠️ La **première vignette déborde** : le titre commence hors de la carte et se coupe. Défaut à ne pas reproduire. |
| `03-album-page.png` | Page album | La structure hero / corps / aside, le watermark « AIR », les boutons Play album · Shuffle · Add to queue, l'aside Details. ⚠️ Chaque piste est **listée deux fois** — voir `../../../../docs/MESURES-V1.md` §10. |
| `04-menu-contextuel.png` | Menu contextuel | Play · Add to queue · Add to playlist · Go to album · Go to artist · Info · Edit Metadata · Remove from library (en rouge). |
| `05-settings.png` | Réglages (drawer) | Sections Audio / Library / Network library / Playback / Keyboard shortcuts. |
| `06-tracks.png` | Vue Pistes | Les **badges de qualité ambrés** pour le hi-res (24-bit) contre gris pour le reste — une couleur hors de la règle monochrome, à documenter ou à trancher. Et beaucoup d'`Unknown Artist` / `-:--` : ce sont les pistes dont la lecture de métadonnées FLAC échoue au scan réseau. |
| `07-artistes.png` | Grille d'artistes | Pochettes circulaires, index alphabétique à droite, compteur de pistes. |
| `08-artiste-page.png` | Page artiste | Le **watermark** « PRINCE », la photo circulaire, les stats (albums / pistes / durée totale). |

## Ce qu'elles ont validé

- ✅ **Les `--glass-*` sont bien rétablies.** Bordures visibles sur la now-strip, les cellules
  hallmark, les sections de réglages ; l'élément actif de la sidebar se distingue. La régression de
  la PR #37 n'est pas dans la cible.
- ✅ **DM Sans se charge** — l'hypothèse « bloquée par la CSP » était fausse. Détail :
  [`../../ECARTS-DESIGN.md`](../../ECARTS-DESIGN.md) §2.

## Ce qu'elles ont révélé

Trois défauts, tous consignés dans [`../../../../docs/MESURES-V1.md`](../../../../docs/MESURES-V1.md) :
l'extraction de couleur ambiante qui échoue à chaque page album (§9), 21 % de la bibliothèque en
double (§10), et le débordement de la première vignette d'album.

## ⚠️ Ces images contiennent des données personnelles

Chemins de bibliothèque avec le nom d'utilisateur, nom d'hôte du NAS, noms de playlists, contenu
réel de la bibliothèque. **Ce dépôt est privé — il doit le rester**, ou ces captures doivent être
retirées avant toute ouverture.

## ⏸ La série est en pause, pas abandonnée — décidé le 2026-11-21

Il manque : fullscreen player, fullscreen lyrics, panneaux Queue / Track Info / EQ / Lyrics, modals
(playlist, feedback, bulk edit, découverte NAS, confirm), les 6 étapes d'onboarding, la sidebar
repliée sous 900 px, le module d'indexation en cours, et la player bar en gros plan avec l'ambient
glow.

**Elles attendent volontairement la passe « davantage de DM Sans »** que Thomas fait côté design : la
typographie change sur tous ces écrans, les capturer maintenant reviendrait à les refaire. Les huit
captures existantes suffisent à ce dont on avait besoin aujourd'hui — valider la cible.

**Quand la passe aura atterri** : réextraire `styles.css` / `views.js` / `tokens.css` depuis le
nouveau commit, écrire l'ADR qui remplace `0004`, puis **reprendre la série en entier** (les huit
d'aujourd'hui comprises, puisqu'elles auront vieilli).

La liste complète et l'ordre sont dans
[`../../../../docs/PROTOCOLE-CAPTURES.md`](../../../../docs/PROTOCOLE-CAPTURES.md).
