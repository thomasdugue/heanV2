# 0020 — Les captures de la bibliothèque sortent du dépôt

- **Date** : 2026-11-27
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-27 — « Les retirer ».
- **Complète** : [`0019`](0019-le-depot-passe-en-public.md)

## Contexte

Les huit captures de `bootstrap/02-design/reference/captures/` ont été prises le 2026-11-21 sur la
cible de parité (commit `e12a200`), sur la **bibliothèque réelle** de Thomas : 3 443 pistes,
pochettes sous droits, noms d'artistes, noms de playlists, chemins portant son nom d'utilisateur,
nom d'hôte du NAS. 11 Mo.

Ce n'est pas une découverte faite en masquant : le `README.md` du dossier portait la consigne depuis
le jour des captures — « Ce dépôt est privé — il doit le rester, ou ces captures doivent être
retirées avant toute ouverture. » La bascule décidée par l'ADR 0019 la déclenche.

Elles ne sont pas décoratives. Elles sont la seule trace du **rendu composé** de la cible de parité
(ADR 0004) — la première et unique fois que ce design a été lancé — et elles ont produit trois
mesures de [`../MESURES-V1.md`](../MESURES-V1.md) : §9 l'extraction de couleur ambiante qui échoue,
§10 les 21 % de doublons, et le débordement de la première vignette d'album.

## Décision

**Les huit images sortent du dépôt. Le `README.md` du dossier reste**, réécrit : il garde, écran par
écran, ce que chaque capture montrait, ce qu'elles ont validé et les trois défauts qu'elles ont
révélés. C'est cette prose, et non les images, que la phase 4 lira — et elle reste vérifiable
puisque le protocole de reprise existe ([`../PROTOCOLE-CAPTURES.md`](../PROTOCOLE-CAPTURES.md)).

**Et l'interdit s'écrit dans `.gitignore`**, pas seulement dans ces pages :
`bootstrap/02-design/reference/captures/*.png` y entre. Une consigne en prose ne peut pas échouer ;
une ligne d'ignore rend la faute inexprimable sans un `git add -f`, qui se voit en revue. C'est
l'ADR [0005](0005-entretien-du-harnais-et-orchestration.md) appliqué à une décision de produit —
d'autant que `PROTOCOLE-CAPTURES.md`, le seul document qu'une session ouvrira pour reprendre la
série, disait encore de les écrire ici.

Les noms d'œuvres qui restent dans cette prose (« AIR », « PRINCE », *Moon Safari*) sont des disques
du commerce, déjà cités par `MESURES-V1.md` : ce sont les **images** qui exposaient une bibliothèque
personnelle, pas ces trois mentions.

## Ce que ça ne règle pas

⚠️ **Mesuré le 2026-11-27 : les images sont dans l'arbre de 52 des 60 commits de `origin/main`.**
La référence compte : le `main` **local** de ce poste porte un commit de plus, jamais poussé, et
compter sur lui donnait 53 sur 61. Ce qui se publie, c'est `origin/main`. Les retirer de `HEAD` les retire de la page que GitHub montre, pas du dépôt : un clone les rend toutes,
et l'interface web laisse parcourir n'importe quel commit. Et contrairement à un mot de passe, une
image **ne se révoque pas**.

Ce retrait est donc nécessaire, pas suffisant. Les deux seules formes qui le rendraient suffisant —
réécrire l'historique, ou publier un dépôt neuf à un seul commit — sont en décision ouverte dans
[l'index](README.md), avec leur coût mesuré.

## Alternatives écartées

**Les garder telles quelles.** C'est ce que la consigne du 2026-11-21 refuse, et le coût est
asymétrique : une bibliothèque personnelle publiée ne se dépublie pas, alors qu'une capture se
reprend en vingt minutes avec le protocole.

**Flouter les seules pochettes.** La mise en page, la typographie et les couleurs survivraient.
Écartée pour deux raisons mesurables sur les images : les titres de pistes, les noms de playlists et
les chemins restent lisibles **hors** des pochettes — il faudrait flouter la moitié de chaque
capture — et un flou abîme précisément ce à quoi elles servent, juger un rendu.

## Conséquences acceptées

- **La phase 4 n'a plus de référence visuelle en image**, seulement la prose du `README.md`, le CSS
  et le markup de `bootstrap/02-design/reference/`. Recul assumé : la série était de toute façon
  **en pause** et devait être reprise **en entier** après la passe « davantage de DM Sans », avec un
  ADR remplaçant `0004` — les huit comprises, puisqu'elles auraient vieilli.
- **Les captures reprises ne reviendront pas dans le dépôt** tant qu'il est public : destination
  locale, hors de git.
- `MESURES-V1.md` §9 et §10 citent des noms de fichiers qui ne sont plus dans l'arbre. Les deux
  renvois le disent désormais, pour qu'une mesure ne devienne pas une affirmation sans pièce.
