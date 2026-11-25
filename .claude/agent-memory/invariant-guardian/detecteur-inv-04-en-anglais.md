---
name: detecteur-inv-04-en-anglais
description: Le détecteur d'INV-04 (unwrap_or_default sur un identifiant) ne cherche que des mots ANGLAIS alors que le code du trousseau est écrit en français — le seul site de faute réaliste lui est invisible
metadata:
  type: reference
---

`tests/architecture/regles-backend.ts` → `defautsSilencieuxSurIdentifiants()` ne retient une
ligne que si elle porte `unwrap_or_default()` **et** un mot de
`/password|keychain|credential|secret/i`. Rien pour `mot_de_passe`, `trousseau`,
`identifiant`.

Or le module du trousseau est entièrement en français (`src-tauri/src/network/identifiants.rs`,
API `identifiants::mot_de_passe`). Mesuré le 2026-11-25 :

- `let mot_de_passe = identifiants::mot_de_passe(&source.id).unwrap_or_default();` → **non vu**
- `lire_du_trousseau(service, compte).unwrap_or_default()` → **non vu**
- `retrieve_password().unwrap_or_default()` (la ligne de V1, celle de l'auto-test) → vu

L'assertion de matière réelle ajoutée le 2026-11-25 passe **parce qu'elle injecte la faute sur
l'appel `security_framework::passwords::get_generic_password`**, dont le nom anglais contient
`password`. Elle ne prouve donc pas que le détecteur voit les appelants, et c'est là que la
faute de V1 se reproduirait (`network/mod.rs`, l'appel qui alimente `ClientSmb`).

**Why:** INV-04 est 🔴 (risque de verrouillage du compte du NAS) et son mécanisme est un grep
à mots-clés ; un dépôt dont le code est en français et le détecteur en anglais a une règle
tenue sur la moitié de son vocabulaire. Même famille que le trou d'extensions de
`check-sizes` : une liste écrite avant le code qu'elle protège vise le nom qu'on imaginait.
**How to apply:** si le diff touche au trousseau, retester le détecteur à la main sur la ligne
française avant de conclure « la règle est tenue ». Le correctif tient en trois mots ajoutés à
la regex (`mot_de_passe|trousseau|identifiant`) ; si un diff futur les ajoute, cette fiche se
périme — le vérifier plutôt que de la croire. Voir [[verifier-les-assertions-de-matiere-reelle]].
