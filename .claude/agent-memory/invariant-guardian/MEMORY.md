# Mémoire — invariant-guardian

- [Vérifier le garde-fou des secrets](verifier-le-garde-fou-des-secrets.md) — un `secrets_guard.rs` vide dans `target/` ne prouve rien ; les deux commandes qui prouvent.
- [Vérifier INV-21 complètement](verifier-inv-21-completement.md) — le test d'architecture ne scanne pas `src-tauri/tests/` : grep manuel obligatoire.
- [Vérifier les comptes d'invariants](verifier-les-comptes-dinvariants.md) — `README.md` n'est vérifié par rien : le grep à faire à chaque ajout de règle.
- [Périmètre du verdict de ./check](perimetre-du-verdict-de-check.md) — tests/ux est hors verdict : un comportement porté qui régresse laisse ./check vert.
