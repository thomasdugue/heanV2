# Chaque garde-fou porte sa propre liste de dossiers

Vu **trois fois** : `src-tauri/tests` le 2026-11-23, `src-tauri/examples` le 2026-11-25, et une
cinquième liste cachée dans un seul fichier le même jour. Les listes connues :
`tools/check-sizes.sh` (ZONES), les quatre règles Rust d'INV-06, `aucun-chemin-machine`,
`references-de-tests`, `backend.test.ts`.

Un dossier de code vivant qui naît entre dans trois listes sur cinq, et **l'oubli est
invisible** : c'est « n'oublie pas de répéter ça partout », la forme de règle qu'INV-06 existe
pour ne plus écrire — appliquée au harnais lui-même.

**La nuance qui compte** : on retourne une liste quand elle énumère ce qu'on a **oublié**, pas
quand elle énumère ce qu'on a **choisi**. Dériver le périmètre de `git ls-files '*.rs'` ramène
les 36 fichiers Rust de `bootstrap/` — l'archive figée de V1 (ADR 0004), dont 13 dépassent
400 lignes, le plus gros à 2 664. Les « corriger » falsifierait le relevé historique.

**Et élargir le corpus d'un fichier ne suffit pas** : lors de la PR #8, trois règles sur quatre
ont été couvertes ; la quatrième gardait sa propre liste écrite à la main juste à côté. Une
faute injectée dans `examples/migrer.rs` est restée invisible, et **seule l'injection l'a dit**.

Déplacé ici depuis le bandeau de `check` le 2026-11-25 (95 lignes de récit lues par toute
session qui ouvrait le fichier). La question « une seule liste des zones Rust vivantes ? » est
une décision ouverte de Thomas, inscrite dans l'index des ADR.
