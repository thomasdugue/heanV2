# Marché — de quoi arbitrer une décision produit

> Les trois études de [`recherche/`](recherche/) font ~470 lignes et personne ne les relit avant un
> arbitrage. Ce document garde ce qui est **stable et actionnable** et nomme ce qui **ne doit pas être
> cité**. Aucune décision ici : elles sont dans [`NON-GOALS.md`](NON-GOALS.md) et
> [`../../docs/adr/`](../../docs/adr/). **Sources par initiale** —
> **[P]** [`pricing-gtm.md`](recherche/pricing-gtm.md) ·
> **[M]** [`market-viability.md`](recherche/market-viability.md) ·
> **[G]** [`gtm-playbook-solo-dev.md`](recherche/gtm-playbook-solo-dev.md) : ⚠️ **les trois datent de
> mars-avril 2026** et parlent d'un produit nommé « Noir ». **[A]** = relevé sur `audirvana.com`
> (`/price/`, `/audirvana-origin/`, `/try/`, `/linux/`) **le 2026-11-28**, seuls chiffres à jour.

## 1. Le fait le plus solide : le rejet de l'abonnement est un mécanisme, pas une humeur

Observé trois fois, chaque fois avec le même dénouement. **Audirvāna** passe à l'abonnement seul en juin 2021 (depuis une
perpétuelle à ~75 €) et est **forcé** de réintroduire une perpétuelle, « Origin » [P] [M]. **Waves Audio** fait de même en
2023 et recule **en quelques jours** ; **Avid** fait le même demi-tour sur Pro Tools [P]. **Roon** : les hausses de la
licence à vie (499 → 699 → 830 $) ont déclenché le même rejet [M]. La démographie l'explique : segment plus âgé, budget de
retraite surveillé, qui dépense 1 500 à 50 000 $ en matériel **qu'il possède** [P]. Et le **Mac App Store est fermé pour
raison technique**, pas commerciale : argument déjà écrit dans [`NON-GOALS.md`](NON-GOALS.md) § Plateformes.

**Et la perpétuelle d'en face n'en est pas une** : Origin n'engage que « two years min. OS compatibility updates » et une
mise à jour de features par an « until 2027 » [A] — perpétuelle d'usage, pas de mises à jour. **C'est la faiblesse attaquable
du modèle concurrent.** D'où les conséquences produit, qui sont l'utilité de cette section :

| Conséquence | Détail |
|---|---|
| Pas de compte obligatoire pour utiliser l'app | L'essai se télécharge sans formulaire ni carte [P] [M] [A] ; un mur d'inscription se lit comme le premier pas vers l'abonnement |
| Validation serveur discrète, longue grâce hors ligne | Un ping mensuel suffit, 30 à 90 jours de grâce [M] |
| DRM léger | 2-3 appareils [P] ou 3-5 [M] — les études divergent. **La priorité est une expérience légitime sans friction, pas un DRM incassable** ; l'hostilité au DRM de type iLok est documentée des deux côtés |

## 2. La carte des prix, et le creux

| Lecteur | Prix | Modèle |
|---|---|---|
| foobar2000 | gratuit [P] | freeware |
| Colibri | 19,99 $ [P] | perpétuelle |
| Swinsian 3 | 34,95 $ [P] [M] | perpétuelle |
| **Hean (prix étudié)** | **49 € early-bird → 69 €** | perpétuelle ([`NON-GOALS.md`](NON-GOALS.md)) |
| Audirvāna Studio | **6,99 €/mois · 69,99 €/an** la 1ʳᵉ année, **renouvellement 79,99 €/an** [A] | abonnement |
| JRiver MC 34 | ⚠️ 99,98 $ [P] · 69,98-89,98 $ par plateforme [G] · 60-90 $ [M] | perpétuelle par version |
| **Audirvāna Origin** | **149,99 €** [A] | perpétuelle (limitée, §1) |
| **Origin + « Signal Processing Suite »** | **219,99 €** [A] — le SPS seul vaut **89,99 €** et porte l'EQ 10 bandes, la convolution, le crossfeed et la balance | **l'équivalent fonctionnel de Hean** |
| Roon | 14,99 $/mois · ⚠️ 149,88 $/an [P] ou 180 $/an [G] · **829,99 $ à vie** | abonnement + à vie |
| HQPlayer 5 | 340 $ [P] | perpétuelle par version |

⚠️ Les lignes marquées portent **les deux valeurs exprès** : les études se contredisent, ne pas choisir sans revérifier. Le
relevé [A] en tranche une — Origin est bien à 149,99 €, donc [P] avait raison et les 119,99 €/120 € de [M] sont périmés,
quand Studio n'était juste dans aucune des deux. **La conclusion tient** : aucun lecteur activement développé n'occupe la
tranche **40-100 $** en achat unique sur macOS [P] — le creux entre Swinsian (35 $) et Origin, là où tombe le prix étudié.

## 3. Où Hean gagne, où il perd

| Terrain | État | Conséquence produit |
|---|---|---|
| **NAS/SMB natif, sans serveur ni montage** | Origin **sait** synchroniser un dossier NAS, et Audirvana vend un **CorePlayer Linux headless qui s'installe sur le NAS** (Synology DSM 7, QNAP, ARM ou Intel), piloté depuis leur app mobile [A]. Ce que **personne** ne fait : lire un NAS **depuis le Mac** sans rien installer dessus ni monter de partage dans le Finder — découverte mDNS, SMB géré par l'app, mot de passe au Keychain, téléchargement progressif, gapless à travers le réseau. Swinsian n'a **aucune** lecture réseau [P] | C'est la feature qui **justifie le prix**, mais son énoncé est « zéro configuration », pas « seul à lire un NAS ». L'ordre de portage déjà choisi — le NAS avant le reste — est celui que le marché valide. |
| UI / fullscreen | ⚠️ **Corrigé** : Origin 3.0 (« Allegro design ») est une refonte sombre et moderne [A], donc le « UI gap is real and significant » de [P] **ne tient plus contre Audirvana**. Il reste vrai contre JRiver et foobar2000 ; Roon reste la référence, à 3 à 17 fois le prix [P] | Le fullscreen player (F28) reste le « moment émotionnel » que [G] décrit comme moteur de conversion (premier payoff en moins de 60 s), mais il ne se vend plus comme « les autres sont datés ». |
| Enrichissement métadonnées | Au-dessus de **tous** les concurrents (MusicBrainz, CoverArtArchive, Deezer, WikiMedia) [P] | Déjà obligatoire par l'[ADR 0017](../../docs/adr/0017-ce-que-thomas-a-tranche-le-25-novembre.md) (B11, C05). |
| **DSD** | **Derrière tout le monde**, y compris Colibri à ~20 $ ; Audirvana va jusqu'à DSD1024 et MQA [A]. [P] l'appelle le gap de crédibilité n°1 ; [M] prévient que les communautés puristes **testeront et publieront** le manque | La décision est prise ([`NON-GOALS.md`](NON-GOALS.md) : pipeline PCM f32 de bout en bout), le coût commercial est **réel**, et la contrepartie attendue est une **réponse technique honnête et publiée** — pas le silence. |
| EQ 8 bandes, et **le prix à fonctionnalités égales** | « Compétitif avec la plupart des lecteurs, insuffisant pour les power users » [P] — mais Audirvana vend le sien à part : Hean **69 € EQ compris** contre **219,99 €** pour l'équivalent (Origin + SPS) [A] | **Argument de parité, pas argument de vente** : aucun investissement au-delà de la parité. En revanche le **69 € contre 219,99 €** est le chiffre le plus fort du document — l'EQ se vend parce qu'il est *inclus*, pas parce qu'il a huit bandes. |
| Upsampling, convolution, AU/VST, streaming TIDAL/Qobuz | Absents, derrière Roon MUSE et JRiver DSP Studio [P] | Assumés en non-goals, **aucun regret** : Audirvāna Origin, Swinsian et foobar2000 sont aussi des lecteurs de fichiers locaux [P]. |
| **Télécommande mobile** | **Manque net.** Audirvana offre une app Remote iOS/Android **gratuite**, mise en avant partout, pour Studio comme pour Origin [A]. Hean n'a rien | Constat, pas une proposition : c'est la **première raison** pour laquelle un acheteur choisira Audirvana à prix comparable. WiFi Sync est sorti du périmètre ([ADR 0017](../../docs/adr/0017-ce-que-thomas-a-tranche-le-25-novembre.md)) et rien n'est rouvert ici. |

## 4. Qui achète : deux segments, pas douze

Deux seulement sont à forte intention, et c'est [M] qui les chiffre. **Réfugiés d'Apple Music** : 500 k à 2 M de Mac
frustrés, 1-3 % de conversion → 5 k à 60 k clients ; motif constant : bibliothèque locale corrompue, vidée au redémarrage,
traitée en citoyen de seconde classe par un design *cloud-first*. **Frustrés de foobar2000** : 1 à 3 M, 2-3 % → 20 k à 90 k ;
motif unique, **des heures de configuration** pour ce que d'autres font en quelques secondes.

**Conséquence produit, la plus importante de ce document** : ces deux segments ne demandent **aucune feature exotique**, ils
demandent que ça marche sans configurer. Donc l'onboarding ([`FEATURE-SET.md`](FEATURE-SET.md) §8), le démarrage instantané
(B01) et le scan différentiel (B03) sont des **features de vente**, pas du confort. Prudence sur un troisième segment, les
casques / Head-Fi (3 à 5 M, 1-2 % → 30 k à 100 k) : ils portent le phénomène documenté **« 2 000 $ de casque, mais pas 50 $
de logiciel »**, réel et seulement « en lente érosion » selon [M].

## 5. Les canaux, pour le jour où le son sortira

Le calendrier en 8 semaines de [G] est **inutilisable aujourd'hui** : il finit par un Show HN, alors que l'app ne joue pas
encore. Ce qui restera vrai ce jour-là : le **forum Software d'AudiophileStyle** (les développeurs y annoncent leurs
produits, sans sponsoring ni palier d'ancienneté [G]), **Darko Audio** (licence gratuite et permanente ; il se dit
explicitement *not pay-to-play* [G]) et l'**angle Rust/Tauri pour Hacker News** (23-25 % de macOS dans l'audience [P]).
**Head-Fi exige un sponsoring payant** pour toute annonce, estimé 500 à 2 000 $/mois et non public [M] : à ne pas viser.

## 6. Ce que ces études ne prouvent pas

- **Les prix se contredisent** (§2) et **tout date de mars-avril 2026** : le relevé [A] montre qu'ils ont bougé partout où il
  les touche.
- **Les projections de revenus sont incompatibles d'un facteur 3 à 4.** [M] donne un scénario « réaliste » à **416 ventes et
  ~22 400 € nets** en année 1 (pessimiste : ~5 200 €). [P] raisonne sur 1 200 unités à 69 € et en conclut « confortable temps
  plein ». Or [P] **n'est pas une prévision** : c'est une illustration d'élasticité, trois lignes d'un tableau comparant
  49/69/79 €. ⚠️ **Le chiffre de 82 800 € ne doit jamais être cité.** Le seul citable est celui de [M], repris dans
  [`NON-GOALS.md`](NON-GOALS.md).
- **TAM / SAM / SOM** : annotés *Low* ou *Low-medium confidence* par [M] **elle-même**, et dérivés de pourcentages posés à la
  main (« 1 à 2 % des utilisateurs Mac ») : ordre de grandeur, aucune décision dessus.
- **La section marque est à rejouer, pas à archiver.** [M] démontre une collision pour « Noir » (extension Safari *Noir —
  Dark Mode for Safari*, Noir Labs, NI NOIRE) et chiffre un dépôt EUIPO à 850 € la classe, 1 100 à 2 200 € avec conseil —
  mais tout cela vaut pour **« Noir Desktop »**. **Rien dans ce dépôt ne montre qu'on ait vérifié « Hean »** : collision,
  SEO, classe 9 de Nice, question **ouverte**.

## 7. Le trou : il n'y a pas de produit payant

Vérifié par `grep` : **aucune ligne** de licence, d'essai, d'activation ni de LemonSqueezy — ni dans les 16 076 lignes de JS
et 12 319 de Rust de V1 ([`../03-architecture/ARCHITECTURE-V1.md`](../03-architecture/ARCHITECTURE-V1.md)), ni dans les 86
commandes IPC ([`../03-architecture/API-IPC-V1.md`](../03-architecture/API-IPC-V1.md)), ni dans les 124 features de
[`FEATURE-SET.md`](FEATURE-SET.md). Or tout le GTM repose dessus : essai de **14 jours** sans carte [P] [G], activation au
premier lancement, et un *Merchant of Record* pour la TVA européenne (LemonSqueezy, ~9-10 % de frais → 62-63 € nets sur une
vente à 69 € [M], « ~65 € » [G]). **Question ouverte** : l'essai d'Audirvana fait **30 jours** [A], soit le double — à
arbitrer, pas ici.

**Conclusion** : « même feature set que V1 » **ne produira pas un produit vendable**. C'est une famille de features **hors
parité**, qu'il vaut mieux nommer maintenant que découvrir en phase 6, et qui méritera **son propre ADR** le jour où elle
s'ouvre. Deux corollaires que les ADR ne portent pas :

- [P] s'appuie sur « Rust compilé = obfuscation naturelle », piratage estimé 10-20 %. **Le dépôt public
  ([ADR 0019](../../docs/adr/0019-le-depot-passe-en-public.md)) affaiblit cet argument** — sans drame, le produit vendu étant
  le binaire signé et notarisé, mais autant que ce soit écrit.
- **L'updater de V1 renvoyait 404 depuis le lancement** (« Bugs connus » du `CLAUDE.md`, bloc repris en phase 6). Sans
  updater, une **licence perpétuelle n'est pas crédible** : ce qu'on vend, c'est le droit aux versions suivantes — et c'est
  précisément là qu'Origin est attaquable (§1).

## 8. Lecture commerciale du feature set

⚠️ **Cette section ne modifie pas la cible de parité.**
L'[ADR 0017](../../docs/adr/0017-ce-que-thomas-a-tranche-le-25-novembre.md) a **reporté** la question « 124 features ou cible
réduite » jusqu'à ce que le son sorte des enceintes, et personne ne retire une ligne de [`FEATURE-SET.md`](FEATURE-SET.md) de
son propre chef. Ce qui suit servira **le jour où** cet arbitrage sera rouvert, rien de plus.

**Tier 1 — ce qui justifie le prix.** D01-D16 (NAS complet) · A02-A05 et A10 (CoreAudio direct, bit-perfect, sample rate
switching, hog mode, gapless) · A12 (device following) · B01, B03. Et **A17 à part** : l'affichage source → sortie avec badge
bit-perfect est une feature **marketing** autant que technique — c'est la preuve mesurable que réclame Audio Science Review,
qui teste au DiffMaker et au RightMark [M]. Déjà acté par
l'[ADR 0018](../../docs/adr/0018-le-bit-perfect-est-un-etat-affiche.md).

**Tier 2 — différenciateurs réels que les études ne voient pas.** F26/F29 (paroles synchronisées : **aucune des trois études
ne les mentionne**, et ni Audirvāna Origin, ni Swinsian, ni foobar2000 ne le font correctement) · F28 (fullscreen player) ·
C01-C08 (pipeline covers) · F10 (mixes Discovery).

**Tier 3 — parité sans ambition.** EQ (A08), raccourcis personnalisables (§7), tri (F16), scrollbar alphabétique (F17),
multi-sélection (F15), détection de doublons (B16).

**Ce que Hean n'a pas et qu'Audirvana a** [A] — relevé, pas programme : app Remote iOS/Android gratuite (§3), vue dossiers,
synchronisation de la bibliothèque Apple, renderer UPnP par défaut, Chromecast, multizone, DSD1024, MQA, 32 bits /
1411,2 kHz, macOS 10.15+ (Hean exige 11+), quinze ans d'existence, partenariats OEM. Les deux premiers sont déjà suivis comme
**décisions ouvertes** dans [l'index des ADR](../../docs/adr/README.md), gelées jusqu'au jalon son.

⚠️ **Un risque de dépendance** : B11 (genres) et C05 (images d'artistes) reposent sur l'API Deezer, **non officielle**.
Obligatoires par décision ([ADR 0017](../../docs/adr/0017-ce-que-thomas-a-tranche-le-25-novembre.md)), et ce n'est pas remis
en cause ici — mais si Deezer coupe, **deux features meurent**.
