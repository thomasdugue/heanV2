---
name: verifier-les-assertions-de-matiere-reelle
description: Marges mesurées des seuils « le détecteur trouve au moins N choses réelles » (2026-11-23) et le piège de signature sources()/fichiers() qui désarme la garde de corpus vide — à revérifier quand ces seuils ou ces corpus bougent
metadata:
  type: reference
---

Depuis le 2026-11-23, `tests/architecture/` porte deux couches en plus des auto-tests :
une garde de **corpus vide** dans `_lecture.ts` et cinq assertions « le détecteur voit la
matière réelle ». Ce qu'il faut savoir pour les juger plutôt que les croire.

**Marges mesurées en phase 2** (si un diff change un seuil ou vide un corpus, remesurer) :

| assertion | seuil | réel mesuré |
|---|---|---|
| couleurs dans `tokens.css` | `> 10` | 33 = **22 hex + 11 `rgba()`** |
| `variablesDeclarees` | `> 20` | 73 dans `tokens.css` seul |
| `variablesLuesSansRepli` | `> 0` | **5** dans tout le projet |
| citations `tests/…` | `> 5` | 17 |
| `println!` dans `build.rs` | `> 0` | 3 (tous `cargo::…`) |

Deux conséquences qui ne se lisent pas dans le code :
- le seuil des couleurs ne mesure qu'« **au moins une** famille de notation est encore
  reconnue » : casser la détection des hex laisse 11 `rgba()` > 10, donc **vert**. C'est
  écrit dans `docs/HISTORY.md` (2026-11-23) mais pas dans le commentaire du test.
- `ecrituresDirectes` connaît **cinq** macros (`println!`, `eprintln!`, `print!`,
  `eprint!`, `dbg!`) ; l'auto-test s'appelle « les quatre macros » et n'en couvre **trois**,
  et l'assertion de matière réelle n'exerce que `println!`. `print!`/`eprint!` ne sont
  couverts par rien.

**Piège de signature** : `fichiers(dossier, exts, racine, videPrevu)` mais
`sources(dossier, exts, videPrevu)`. Écrire `sources(d, exts, RACINE)` — ce que suggère le
skill, qui dit « 4ᵉ argument » — **désarme silencieusement la garde de corpus vide** (toute
chaîne non vide vaut dérogation, et `''` aussi). À grepper sur tout diff qui ajoute un
détecteur : `grep -n "sources(.*RACINE" tests/`.

**Why:** relecture du commit `379e40f` ; les seuils sont honnêtes mais aucun n'est auto-
documenté, et un seuil sans sa marge ne se relit pas.
**How to apply:** même discipline que [[verifier-inv-21-completement]] — vérifier le
périmètre d'un garde-fou au lieu de le supposer. Les sous-vérifications rapides et sans
effet de bord sont listées là-bas.
