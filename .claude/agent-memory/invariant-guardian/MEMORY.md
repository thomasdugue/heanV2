# Mémoire — invariant-guardian

- [Vérifier le garde-fou des secrets](verifier-le-garde-fou-des-secrets.md) — un `secrets_guard.rs` vide dans `target/` ne prouve rien ; les deux commandes qui prouvent.
- [Vérifier INV-21 complètement](verifier-inv-21-completement.md) — le test d'architecture ne scanne pas `src-tauri/tests/` : grep manuel obligatoire.
- [Vérifier les comptes d'invariants](verifier-les-comptes-dinvariants.md) — `README.md` n'est vérifié par rien : le grep à faire à chaque ajout de règle.
- [Périmètre du verdict de ./check](perimetre-du-verdict-de-check.md) — tests/ux est hors verdict : un comportement porté qui régresse laisse ./check vert.
- [Assertions de matière réelle](verifier-les-assertions-de-matiere-reelle.md) — les marges mesurées des cinq seuils, et le piège `sources(d, exts, RACINE)` qui désarme la garde de corpus vide.
- [Dérogation examples/ à INV-21](derogation-examples-inv-21.md) — exception à une règle 🔴 écrite sans ADR, et le rouge volontaire attendu au merge de la migration.
- [Périmètre de check-sizes](perimetre-de-check-sizes.md) — `src-tauri/tests/` est hors du scan des 400 lignes, et `bibliotheque.rs` dépasse déjà.
