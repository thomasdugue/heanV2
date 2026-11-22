---
name: verifier-les-comptes-dinvariants
description: Où les comptes d'invariants sont réclamés dans la doc, lesquels sont vérifiés par check-invariants.mjs et lequel ne l'est pas (README.md) — à grepper à chaque ajout ou retrait de règle
metadata:
  type: reference
---

Le nombre de règles est affirmé dans plusieurs documents. Depuis le 2026-11-22,
`tools/check-invariants.mjs` en vérifie deux séries :

- §3 : le tableau de tri du skill `hean-invariants` (une ligne par pile) ;
- §4 : les comptes de `CLAUDE.md` (total, rayées, devenues des tests, irréductibles,
  « planifiées »).

Ne sont vérifiés par **rien** : `README.md` (tableau « Organisation du dépôt », qui cite
« les N règles » de `hean-invariants`) et les commentaires narratifs des scripts/HISTORY.
`docs/generated/invariants.md` est généré (`npm run gen`) donc sûr, mais son décalage
n'apparaît qu'au `./check` complet (`tools/check-generated.sh`).

Le grep à faire à chaque diff qui ajoute ou retire un INV :
`grep -rn "N règles\|N invariants" --include="*.md" . | grep -v node_modules | grep -v bootstrap/`
(les comptes de `bootstrap/` parlent de V1 : 19/20, ils sont historiques et figés).

**Why:** relecture du commit `ea280aa` (entrée d'INV-22) : CLAUDE.md, le skill et le
fichier généré ont été mis à jour, `README.md` est resté à « les 21 règles » — exactement
la dérive « un compte faux dans cinq documents parce que rien ne le vérifiait » que le
harnais existe pour empêcher.
**How to apply:** ne jamais conclure « les comptes sont bons » sur la foi d'un
`node tools/check-invariants.mjs` vert — `README.md` lui échappe (corrigé par ailleurs dans
la PR #3 de la session de phase 0, qui retire les chiffres décoratifs plutôt que de les
maintenir). En revanche l'angle mort `src-tauri/tests/` de la §2 est **refermé depuis le
2026-11-22** : le parcours des citations couvre les trois dossiers, et la mutation le
confirme (une citation `INV-99` dans `decodage.rs` fait échouer le script). Quand INV-22
passera de `planifiée` à `test <nom>` en phase 5, son test pourra donc vivre dans
`src-tauri/tests/`. Voir aussi [[verifier-inv-21-completement]].
