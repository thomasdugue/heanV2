# Mémoire — invariant-guardian

- [Vérifier le garde-fou des secrets](verifier-le-garde-fou-des-secrets.md) — un `secrets_guard.rs` vide dans `target/` ne prouve rien ; les deux commandes qui prouvent.
- [Vérifier INV-21 complètement](verifier-inv-21-completement.md) — le test d'architecture ne scanne pas `src-tauri/tests/` : grep manuel obligatoire.
- [Vérifier les comptes d'invariants](verifier-les-comptes-dinvariants.md) — `README.md` n'est vérifié par rien : le grep à faire à chaque ajout de règle.
- [Périmètre du verdict de ./check](perimetre-du-verdict-de-check.md) — tests/ux hors verdict — et un test ux qui ne cite aucun Vxxx n'est nulle part.
- [Assertions de matière réelle](verifier-les-assertions-de-matiere-reelle.md) — les marges mesurées des cinq seuils, et le piège `sources(d, exts, RACINE)` qui désarme la garde de corpus vide.
- [Dérogation examples/ à INV-21](derogation-examples-inv-21.md) — réglée (ADR 0011, note « muet » retirée) ; ce qu'il reste à vérifier sur HORS_SCAN.
- [Périmètre de check-sizes](perimetre-de-check-sizes.md) — trous fermés le 2026-11-25 ; qui rougira le premier (migration/mod.rs, 400/400).
- [Détecteur d'INV-04 en anglais](detecteur-inv-04-en-anglais.md) — le grep du trousseau ignore `mot_de_passe` : la faute en français lui est invisible.
- [Zone audio : chemins Rust seulement](zone-audio-chemins-rust-seulement.md) — les fichiers frontend du lecteur sont de l'UI malgré leur nom
- [Les listes de zones du harnais](les-listes-de-zones-du-harnais.md) — chaque garde-fou a la sienne ; on retourne une liste d'oublis, pas une liste de choix
- [Un compteur sans lecteur](un-compteur-sans-lecteur.md) — un compteur d'anomalie n'est une trace que si quelqu'un le lit sur le chemin réel
- [Repli de fréquence et interdit de resampling](repli-de-frequence-et-interdit-de-resampling.md) — le code replie, le skill et l'ADR 0007 §5 disent « refuser » : vérifier avant de juger
