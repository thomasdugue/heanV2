//! La séquence de démarrage : migrer, ouvrir la bibliothèque, reprendre la configuration.
//!
//! **Ces tests existent parce que les précédents ne voyaient pas ce qu'ils affirmaient.** La
//! migration était éprouvée en appelant `migrer_si_necessaire` deux fois de suite — ce que
//! `run()` ne fait jamais : entre les deux, il ouvre la bibliothèque, et c'est cette
//! ouverture qui créait le dossier de données et rendait l'échec définitif. Le test
//! concluait « le lancement suivant peut retenter » sur une séquence qui n'était pas celle du
//! produit.
//!
//! D'où la règle que tiennent les tests ci-dessous : **on rejoue la séquence entière**, celle
//! de `demarrage::preparer`, et jamais ses morceaux pris à part. Un test qui saute une étape
//! prouve quelque chose au sujet d'un programme qui n'existe pas.
//!
//! Règles couvertes : **INV-24** (rien n'est créé dans le dossier de données avant que la
//! migration ait rendu son verdict) et **INV-06** à travers la migration — une piste
//! supprimée dans V1 ne revient pas dans V2. Décisions : ADR 0012, qui remplace sur ce point
//! la conséquence acceptée de l'ADR 0010.
//!
//! Une seule cible de test (`cargo test --test demarrage`), découpée en modules le 2026-11-25
//! parce que ce fichier a dépassé la limite de 400 lignes d'`./check` en recevant le test
//! d'INV-26. Les nouveaux tests de la séquence de démarrage s'ajoutent dans le module du thème
//! qu'ils éprouvent, ou dans un nouveau module : pas ici.
//!
//! **Le point d'entrée garde son nom**, et les modules vivent dans `demarrage/` à côté. C'est la
//! forme que Cargo accepte — un fichier de cible plus un dossier du même nom — et elle a été
//! choisie pour une raison précise : une douzaine de textes du dépôt renvoient à
//! `src-tauri/tests/demarrage.rs`, dont l'ADR 0012, **et un ADR ne se modifie pas**. Déplacer ce
//! nom aurait transformé une décision écrite en renvoi mort. (`tests/bibliotheque/` emploie
//! l'autre forme, `main.rs` dans le dossier ; les deux sont valides, celle-ci préserve les
//! renvois.)

// Les `#[path]` sont nécessaires et non décoratifs : pour la racine d'une cible, Rust cherche
// les sous-modules **à côté** du fichier et non dans un dossier du même nom. Sans eux, `mod bac;`
// irait chercher un `bac.rs` posé dans le dossier des tests, à côté de ce fichier-ci, et la
// compilation échoue. C'est le même idiome que `src-tauri/tests/bibliotheque/main.rs` emploie
// pour atteindre son bac partagé.
#[path = "demarrage/bac.rs"]
mod bac;

#[path = "demarrage/echec_rattrapable.rs"]
mod echec_rattrapable;
#[path = "demarrage/reprise.rs"]
mod reprise;
#[path = "demarrage/retour_arriere.rs"]
mod retour_arriere;
#[path = "demarrage/schema.rs"]
mod schema;
