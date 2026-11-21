# Protocole — capturer la cible de parité (préalable A2)

Objectif : produire les captures qui serviront de **référence visuelle de V2**, et par la même
occasion **valider la cible** — personne ne l'a encore vue tourner. Au passage, trancher la
typographie (préalable A4).

## Lancement

```bash
cd /Users/tsunami25/Documents/Thomas/noirdesktop && git checkout thomasdugue/design-migration-audit
```

```bash
cd /Users/tsunami25/Documents/Thomas/noirdesktop/noir-tauri && ./scripts/dev.sh
```

**Pourquoi le checkout principal et pas le workspace Conductor** : il a déjà
`scripts/.env.local` (sans lequel `dev.sh` s'arrête net) et **17 Go de cache de build Rust**. Le
workspace `yellowknife` n'a ni l'un ni l'autre — ce serait une compilation à froid de 10 à 20 minutes
et un `dev.sh` qui refuse de démarrer.

La branche est déjà récupérée (`git fetch` fait le 2026-11-21). Le basculement est propre : les deux
fichiers non suivis à la racine ne sont pas suivis dans la branche cible, et le HEAD actuel
(`522c75e`) est contenu dans `origin/claude/kind-cerf-A9weP` — rien à perdre.

Vérifier après le checkout :

```bash
cd /Users/tsunami25/Documents/Thomas/noirdesktop && git log -1 --format=%h
```

Doit afficher `e12a200`. Si ce n'est pas ça, les captures ne valent rien.

---

## 1. Typographie — 2 minutes, à faire en premier

Avec les devtools (clic droit → Inspecter, ou `Cmd+Option+I`) :

| Quoi vérifier | Où regarder | Ce qu'on cherche |
|---|---|---|
| Erreur CSP | onglet Console | un message bloquant sur `fonts.googleapis.com` |
| Police du corps | un paragraphe de piste → Computed → `font-family` | `Geist Mono` attendu |
| Police d'un gros titre | un titre de section, un nom d'artiste → Computed | `DM Sans` **ou** `-apple-system` / `SF Pro` — c'est ça qui tranche |
| Watermark artiste | page artiste, le nom géant en fond | il est spécifié en DM Sans **900** ; s'il tombe sur San Francisco, ça se voit |

Si DM Sans n'arrive pas, le design de référence suppose une police qui n'a jamais été chargée — et
l'arbitrage devient : l'embarquer en local, ou assumer Geist Mono partout.

**Fermer les devtools avant de continuer** : le processus d'inspection fausse tout ce qui suit.

---

## 2. Les captures

Destination : `bootstrap/02-design/reference/captures/`

Nommage : `NN-ecran.png`, numéroté dans l'ordre ci-dessous, pour que le dossier se lise comme un
parcours. `Cmd+Shift+4` puis espace capture la fenêtre seule, sans le bureau.

### Ce qui n'a jamais été vu, et qui compte le plus

Le redesign v3 réécrit ces écrans. Ce sont les captures les plus importantes — et si quelque chose
cloche, c'est là.

- [ ] `01-home-haut.png` — **l'eyebrow** (point qui pulse + `HEAN · DATE · HEURE` + filet dégradé) et
      la **now-strip** en dessous : grille 200 px / 1fr / auto, fond animé qui dérive, halo coloré
      pris sur la pochette, cellules « hallmark » (Bit depth, Sample, Format, Bit perfect) en glass
- [ ] `02-home-now-strip-en-lecture.png` — la même, musique lancée : les barres d'EQ doivent bouger
- [ ] `03-home-carrousels.png` — les en-têtes de section, l'eyebrow **au-dessus** du titre
- [ ] `04-album-hero.png` — la page album en hero + corps + aside (structure entièrement nouvelle)
- [ ] `05-album-corps.png` — la liste de pistes et l'aside, défilé
- [ ] `06-artiste.png` — le **watermark** (nom géant teinté par la pochette) + les stats
- [ ] `07-menu-contextuel.png` — clic droit sur une piste
- [ ] `08-coeur.png` — le « heart burst » au moment du clic sur un favori
- [ ] `09-settings.png` — le drawer de réglages (il a changé de forme)

### Le reste, pour la complétude

- [ ] `10-albums-grille.png` · `11-artistes-grille.png` · `12-pistes.png` (avec multi-sélection)
- [ ] `13-playlist.png` · `14-discovery-mix.png`
- [ ] `15-fullscreen-player.png` · `16-fullscreen-lyrics.png`
- [ ] `17-queue.png` · `18-track-info.png` (lecture **et** édition) · `19-eq.png` · `20-lyrics.png`
- [ ] `21-settings-*.png` — une par section
- [ ] `22-modal-playlist.png` · `23-modal-feedback.png` · `24-modal-bulk-edit.png` ·
      `25-modal-nas.png` · `26-modal-confirm.png`
- [ ] `27-onboarding-1..6.png` — les 6 étapes
- [ ] `28-sidebar-replieee.png` — fenêtre sous 900 px de large
- [ ] `29-indexation.png` — le module de scan en cours
- [ ] `30-player-bar.png` — avec le badge bit-perfect et l'ambient glow sous la barre

### Ce qu'il faut regarder en capturant

Les quatre `--glass-*` étaient cassées sur la PR #37 et c'est la migration d'octobre qui les
rétablit. Donc, en passant : **les bordures sont-elles visibles ? les survols réagissent-ils ?
l'élément actif de la sidebar se distingue-t-il ?** Si la réponse est non quelque part, la régression
n'est pas complètement corrigée et il faut le dire avant de reproduire quoi que ce soit.

---

## 3. Ce qui ne peut PAS se faire dans cette session

**La mesure de mémoire sous défilement.** Elle doit tourner sur `/Applications/Hean.app`, la build
**release**, parce que les chiffres déjà consignés dans [`MESURES-V1.md`](MESURES-V1.md) viennent de
là. Un build de dev n'a pas le même profil mémoire — comparer les deux ne voudrait rien dire.

C'est donc un exercice séparé, sur l'app installée :

```bash
cd /Users/tsunami25/Documents/Thomas/heanV2 && ./tools/measure-app.sh noir-tauri 90
```

Découpage : 10 s au repos → Albums et défiler toute la grille jusqu'à ~45 s → 15 s au repos → trois
requêtes dans la recherche → repos.

---

## Après

```bash
cd /Users/tsunami25/Documents/Thomas/noirdesktop && git checkout claude/kind-cerf-A9weP
```

Pour remettre le checkout principal où il était — ou le laisser sur la cible, c'est elle qui compte
maintenant.

Et si le rendu ne te convient pas : la cible de parité se change par un nouvel ADR qui remplace
[`adr/0004`](adr/0004-cible-de-parite.md), pas en cours de route.
