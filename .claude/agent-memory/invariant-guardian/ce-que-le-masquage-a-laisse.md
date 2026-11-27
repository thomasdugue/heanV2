---
name: ce-que-le-masquage-a-laisse
description: Ce que le masquage du 2026-11-27 (ADR 0019/0020) a délibérément laissé dans le dépôt, et pourquoi — à ne pas re-signaler comme une trouvaille
metadata:
  type: project
---

Le masquage avant la bascule en public a retiré le mot de passe app-specific Apple, le DSN Sentry,
le Team ID, l'e-mail personnel, le numéro de série du FiiO, les 22 chemins `/Users/<compte>` et,
après relecture, **le nom d'hôte du NAS** (34 occurrences dans 7 fichiers : `<NAS>` dans la prose,
`nas` dans les fixtures Rust et TS).

Restent **volontairement** : le compte `admin`, le partage `music`, le chemin `/LOSSLESS/complete`,
et le service de trousseau `com.tdugue.hean.smb`.

**Why:** ces termes sont génériques — `admin` est un défaut de NAS, `LOSSLESS` un nom de dossier —
et ils sont cités par les ADR [0015](../../../docs/adr/0015-nom-de-service-keychain-pour-les-mots-de-passe-smb.md)
et [0016](../../../docs/adr/0016-la-part-reseau-de-la-bibliotheque.md), **immuables** : les masquer
demanderait de réécrire deux décisions actées pour un gain nul. Le nom de service du trousseau, lui,
est un identifiant **fonctionnel** tranché par l'ADR 0015 et il ship dans le binaire. Seul le nom
d'hôte était identifiant, étant dérivé du patronyme — raison pour laquelle il ne se réécrit pas ici.

**How to apply:** ne pas re-signaler `admin`, `LOSSLESS` ni `com.tdugue.*` comme des données
restées en clair — c'est un choix écrit. Signaler en revanche tout **nouveau** nom d'hôte, partage
ou compte réel qu'un diff ajouterait. Et la leçon du jour, qui vaut au-delà de ce cas : un motif de
recherche **sensible à la casse** a manqué une occurrence en casse mixte dans
`src-tauri/src/network/uri.rs` — la ligne qui prouve justement que l'identifiant dérivé se
normalise —, exactement comme un motif avait manqué le mot de passe Apple à cause d'un guillemet
ouvrant. Voir
[[mesurer-commits-et-branches]] et [[verifier-le-garde-fou-des-secrets]].
