# Invariants — ce qui protège quoi

<!-- FICHIER GÉNÉRÉ — NE PAS ÉDITER. Source : .claude/skills/hean-invariants/SKILL.md. Régénérer : npm run gen -->

23 règles : 6 rayées par construction, 15 devenues des tests, 2 irréductibles.
En V1, **3 sur 20** étaient couvertes par un test.

| # | Règle | Pile | Gravité | Ce qui la protège |
|---|---|---|---|---|
| INV-06 | Suppression de pistes = PERMANENTE | ✂️ rayé | 🔴 BLOQUANT | sans objet — architecture `une-seule-source-de-pistes.test.ts` |
| INV-08 | METADATA_CACHE : UPDATE, jamais REMOVE | ✂️ rayé | 🔴 BLOQUANT | sans objet — SQLite, un seul écrivain, transactions (ADR 0002) |
| INV-09 | start_background_scan() : re-application du cache de métadonnées | ✂️ rayé | 🔴 BLOQUANT | sans objet — SQLite, un seul écrivain, transactions (ADR 0002) |
| INV-10 | METADATA_CACHE : jamais de donnée périmée | ✂️ rayé | 🔴 BLOQUANT | sans objet — SQLite, un seul écrivain, transactions (ADR 0002) |
| INV-14 | Mosaïque de playlist : rebuild automatique après chargement async des covers | ✂️ rayé | 🟡 AVERTISSEMENT | sans objet — Svelte 5 : le DOM découle de l'état (ADR 0002) |
| INV-17 | option_env!() pour TOUS les secrets compile-time | ✂️ rayé | 🔴 BLOQUANT | sans objet — `src-tauri/build.rs` + `compile_error!()`, et architecture |
| INV-01 | Un album = une seule entrée | 🧪 test | 🔴 BLOQUANT | test `cle.rs` — type `AlbumKey` à constructeur unique + proptests, et |
| INV-02 | Pas de duplication visuelle sur la home | 🧪 test | 🟡 AVERTISSEMENT | planifiée (phase 3) — test de comportement sur « Recently Played » |
| INV-03 | SMB : connexion périmée = retry automatique | 🧪 test | 🔴 BLOQUANT | planifiée (phase 5) — conteneur Samba + proxy à injection de panne |
| INV-04 | SMB : un échec Keychain remonte une erreur, jamais un mot de passe vide | 🧪 test | 🔴 BLOQUANT | architecture `backend.test.ts` |
| INV-05 | SMB : l'erreur du thread de download remonte à l'utilisateur | 🧪 test | 🟡 AVERTISSEMENT | planifiée (phase 5) — conteneur Samba |
| INV-07 | Une métadonnée éditée est écrite dans le fichier audio | 🧪 test | 🔴 BLOQUANT | test `edition.rs` — écriture lofty, puis relecture du fichier |
| INV-11 | Mosaïque de playlist : déduplication par album seul | 🧪 test | 🟡 AVERTISSEMENT | planifiée (phase 3) — property test « invariant par permutation » |
| INV-12 | Mosaïque de playlist : le fallback internet est obligatoire | 🧪 test | 🟡 AVERTISSEMENT | planifiée (phase 4) — test des 4 niveaux du pipeline de covers |
| INV-15 | Boutons « Add to Queue » / « Add to Playlist » au survol | 🧪 test | 🟡 AVERTISSEMENT | planifiée (phase 4) — régression visuelle, état `hover` |
| INV-16 | Sentry : before_send jette les events si le toggle est éteint | 🧪 test | 🔴 BLOQUANT | planifiée (phase 6) — test unitaire avec `SENTRY_ENABLED = false` |
| INV-19 | Bouton shuffle : une seule source pour l'icône et le mode | 🧪 test | 🟡 AVERTISSEMENT | planifiée (phase 3) — un seul module exporte le glyphe et le cycle de mode |
| INV-20 | Toute variable `var(--x)` utilisée est définie | 🧪 test | 🟡 AVERTISSEMENT | architecture `css-variables-definies.test.ts` |
| INV-21 | Aucun `println!` / `eprintln!` dans le backend | 🧪 test | 🔴 BLOQUANT | architecture `backend.test.ts` |
| INV-22 | L'intégrité d'un fichier audio ne se vérifie jamais avec le décodeur | 🧪 test | 🔴 BLOQUANT | planifiée (phase 5) — contrôle hors décodeur : durée annoncée par le conteneur contre trames réellement rendues |
| INV-23 | Un libellé destiné à l'affichage ne vient jamais d'un `{:?}` | 🧪 test | 🟡 AVERTISSEMENT | test `bibliotheque/format.rs` — table `FileType` → libellé, une assertion par fixture audio |
| INV-13 | Mosaïque 28×28 : positionnement absolu, jamais CSS Grid | 🪨 irréductible | 🟡 AVERTISSEMENT | manuelle — piège PLAT-5, détecté a posteriori par la régression visuelle |
| INV-18 | dylibbundler doit être suivi du fix des LC_RPATH dupliqués | 🪨 irréductible | 🔴 BLOQUANT | manuelle — piège PLAT-6, vérifiable dans le pipeline de release (phase 6) |
