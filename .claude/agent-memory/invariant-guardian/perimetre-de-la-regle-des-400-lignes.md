---
name: perimetre-de-la-regle-des-400-lignes
description: check-sizes.sh ne compte que 7 extensions — un .py ou un .sh de 600 lignes passe au vert ; le wc -l à faire à la main
metadata:
  type: reference
---

`tools/check-sizes.sh` applique la règle « aucun fichier ne dépasse ~400 lignes » sur les
dossiers `src src-tauri/src tests tools`, mais **seulement** pour les extensions
`.ts .tsx .js .rs .css .mjs .svelte`. Ni `.py`, ni `.sh`, ni `.md`, ni `.json`.

Conséquence : un script Python ou Bash peut dépasser la limite sans faire rougir `./check`.
Constaté le 2026-11-23 sur `tools/passe-de-verite-rapport.py` (569 lignes) — le premier
fichier du dépôt à passer la limite, et il l'a fait dans une langue que le détecteur ne lit
pas. `bash tools/check-sizes.sh` restait vert.

Donc, à chaque diff qui ajoute un script :
`wc -l $(git diff --name-only origin/main...HEAD)` et comparer à 400, sans se fier au script.

**Why:** la leçon de V1 (lib.rs 4 870 lignes) porte sur la taille d'un fichier, pas sur son
extension ; et une règle n'est tenue que là où le garde-fou regarde — même famille de défaut
que [[verifier-inv-21-completement]].
**How to apply:** si le dépassement est dans un outil jetable et hors produit, c'est un
AVERTISSEMENT, pas un BLOQUANT — mais il faut le dire, parce que `./check` ne le dira pas.
