# 0004 — La cible de parité est le redesign v3, commit `e12a200`

- **Date** : 2026-11-21
- **État** : Acceptée
- **Décideur** : Thomas

## Contexte

La contrainte n°2 du brief est « même design que V1 ». Encore faut-il savoir **quel** V1 : le
2026-11-21, en vérifiant une impression de Thomas (« il y a un écart entre la version de
l'application et les derniers trucs que j'avais faits, notamment les refacto design »), on a trouvé
**trois** designs différents, dont deux hors de `main`.

| Version | Où | Date |
|---|---|---|
| Design « beta » | `main`, tag `v0.2.0-beta.1` | mai 2026 |
| Redesign v3 | PR [#37](https://github.com/thomasdugue/noirdesktop/pull/37), branche `claude/kind-cerf-A9weP`, 23 commits | juin 2026 |
| Redesign v3 porté sur main d'octobre | commit `e12a200`, branche `thomasdugue/design-migration-audit` (clone `noirdesktop-v1`) | 2026-10-29 |

Le premier bootstrap (rédigé le matin du 2026-11-21) avait extrait le design depuis `main`, et
`DEMARRER.md` §A1 proposait de figer la parité sur `v0.2.0-beta.1`. **C'était la mauvaise cible** :
elle ignore six mois de travail de design, dont Thomas se souvenait précisément.

Trois faits ont orienté la décision :

1. Le checkout principal de Thomas (`~/Documents/Thomas/noirdesktop`) était posé sur la branche du
   redesign. L'app qu'il voyait en lançant `dev.sh` **n'était pas** celle qu'il avait distribuée.
2. La PR #37 est **visuellement cassée** : `--glass-lo/md/hi` y sont utilisées 45 fois sans être
   définies (leur définition avait disparu avec le revert des Lots A-H), donc bordures, survols et
   état actif de la sidebar sont invisibles. Une variable CSS non définie est ignorée en silence.
3. La migration d'octobre corrige exactement ça, et va plus loin côté JS (ordre de pose de l'index
   de piste avant le rendu de la queue, drapeau bit-perfect non supposé vrai avant confirmation de
   Rust, modes de shuffle).

## Décision

**La parité se mesure contre le commit `e12a200`** de `thomasdugue/design-migration-audit`, dans le
clone `noirdesktop-v1` — pas contre `main`, pas contre `v0.2.0-beta.1`, pas contre la PR #37.

Conséquences concrètes, déjà appliquées :

- `bootstrap/02-design/reference/styles.css` et `views.js` viennent de ce commit ;
- `bootstrap/02-design/tokens.css` est extrait de ce `styles.css` — **deux** blocs `:root` ;
- le CSS de la beta est conservé sous `styles.css.beta-v0.2.0`, uniquement pour mesurer l'écart avec
  ce que les testeurs ont installé ;
- les captures d'écran à faire (§A2 de `DEMARRER.md`) doivent être prises sur ce commit.

Ce commit n'existait que dans un arbre de travail Conductor jusqu'au 2026-11-21. Il est maintenant
commité, mais **pas poussé** : il ne vit que dans le clone local `noirdesktop-v1`.

## Alternatives écartées

| Alternative | Pourquoi écartée |
|---|---|
| **`v0.2.0-beta.1`** (la beta distribuée) | C'est le seul design que des utilisateurs ont réellement vu, et le seul validé sur du vrai matériel. Écartée parce que ce n'est pas ce que Thomas veut reproduire : six mois de travail de design sont postérieurs, et c'est précisément l'écart qu'il a signalé. Reproduire la beta, ce serait réécrire V2 pour arriver à un design que son auteur a déjà abandonné. |
| **La PR #37** (juin) | C'est la version « officielle » du redesign, poussée et visible sur GitHub. Écartée pour une raison factuelle : elle est cassée (les 45 `var(--glass-*)` non définies), et sa base de mai ignore tous les correctifs de l'été. |
| **Attendre de merger la PR #37 dans V1, puis viser `main`** | Le plus propre sur le papier. Écartée parce que ça fait dépendre le démarrage de V2 d'un chantier sur V1 — merge, validation visuelle sur macOS, correction de la régression CSS — alors que le commit d'octobre contient déjà le résultat. |

## Conséquences acceptées

- **Le design de référence n'a jamais été validé visuellement.** Le corps de la PR #37 le dit :
  *« l'agent ne peut pas lancer l'app Tauri […] le rendu visuel est à confirmer sur ton Mac »*. La
  migration d'octobre n'a pas été lancée non plus. On reproduit donc un design **jamais vu**. C'est
  la raison pour laquelle les captures d'écran (§A2) passent de « important » à **bloquant** : elles
  sont à la fois la référence de V2 et la première validation de la cible.
- **Le prototype d'origine est perdu.** Le redesign v3 est le portage d'un « prototype Hean v4 »
  livré dans un bundle `design_handoff`, qui vivait dans `/tmp/proto/player_test_1/`. Introuvable sur
  le disque. On reproduit donc le portage, pas le prototype — les écarts de fidélité relevés dans les
  commits de juin ne sont plus vérifiables.
- **La cible vit sur une branche locale non poussée.** Tant qu'elle n'est pas sur GitHub, une panne
  de disque emporte la cible de parité de V2. À pousser.
- **Les ~50 testeurs recevront un saut visuel**, pas une évolution progressive : ils passent du
  design de mai à celui d'octobre d'un coup. À dire dans les notes de version.
- **Le régime de V1 pendant la reconstruction reste à trancher** (gel total / hotfix critiques
  seulement / évolution continue). La question est moins urgente qu'elle n'y paraissait : le travail
  de design sur V1 est de fait arrêté depuis octobre, et il n'est pas commité. Mais le bug d'auto-
  updater (aucun testeur n'a jamais reçu de mise à jour) plaide pour au moins un hotfix.
