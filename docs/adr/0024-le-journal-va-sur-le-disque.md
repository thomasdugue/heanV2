# 0024 — Le journal va sur le disque, en plus de la console

- **Date** : 2026-11-28
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-28 — « Les trois premiers ».
- **Tranche** la décision ouverte « V2 n'écrit aucun journal sur disque » de [l'index](README.md).

## Contexte

`observability.rs` n'installait qu'un `fmt::layer()` vers la console, et `tracing-appender`
n'était même pas déclaré dans `Cargo.toml` : il n'existait **aucun moyen** d'écrire un fichier.
Le commentaire en tête de ce module reproche à V1 d'avoir eu des bugs « annoncés en clair en dev
et invisibles en production » — V2 avait la même propriété, en pire.

Ça a déjà coûté, et pas en phase 6 comme annoncé : **sur la semaine du 21 au 28 novembre, un
aller-retour humain par test matériel.** Thomas, qui ne lit pas le code, recopiait des blocs de
journal dans la conversation ; il a dû écrire « *je souhaite que tu verifies toi meme le
terminal* » ; l'agent a appelé `read_terminal` 45 fois sur les deux sessions d'implémentation,
et a fini par écrire `journaliser_les_anomalies()` pour relire ses propres compteurs audio faute
de pouvoir les retrouver après coup. La mesure du scan de 36 minutes et l'écriture du trousseau
de 01:07:35Z n'ont jamais existé ailleurs que dans un défilement de terminal.

## Décision

Une **couche fichier** s'ajoute à la console : `~/Library/Application Support/hean/logs/`
(le dossier de l'[ADR 0003](0003-dossier-de-donnees.md), jamais un second chemin), rotation
**journalière**, **quinze** fichiers gardés, écriture **non bloquante**.

Le chemin entre dans `CLAUDE.md`, en remplacement de la ligne `core.hooksPath` que `./check`
pose désormais tout seul : **une ligne pour une ligne**, le fichier reste à 149 sur 150.

## Alternatives écartées

- **Rotation par taille** : on cherche *un jour de test*, pas un volume.
- **Écriture bloquante** : plus simple, mais elle met des écritures disque sur le chemin de
  l'appelant. L'écrivain non bloquant exige en échange qu'un garde vive aussi longtemps que le
  process **et soit lâché à la fin**.
  ⚠️ **Première version de cet ADR et de ce module : l'inverse, et deux fois.** Elle gardait le
  garde dans un `static OnceLock` en affirmant que « lâché, il emporte les lignes en attente ».
  La source de `tracing-appender` 0.2.5 dit le contraire : c'est le `Drop` du garde qui demande
  le vidage et l'attend, donc un `OnceLock` — qui n'est jamais lâché — **ne vide jamais**, et
  les lignes d'avant un arrêt, les seules qu'on relira, sont précisément celles qui manquent.
  Et `non_blocking()` est **lossy par défaut** (`is_lossy: true`) : à saturation il jette les
  lignes en silence. Les deux relevés par `invariant-guardian`, vérifiés dans la source,
  corrigés avant la fusion — `lossy(false)`, et le garde rendu par `init` puis lié dans `run`.
  Le test d'intégration échoue si on ne le lie pas : c'est ce qui le rend non décoratif.
- **Refuser le démarrage si le journal ne s'ouvre pas** : un journal qui manque est un
  désagrément, un démarrage refusé serait une panne. `HOME` absent laisse la console seule.

## Conséquences acceptées

- **Quinze jours de journaux sur le disque de Thomas**, quelques mégaoctets. Ces fichiers
  portent des chemins de sa bibliothèque et des noms de ses périphériques : ils sont **hors du
  dépôt**, et `.gitignore` n'a pas à les connaître puisqu'ils ne sont pas dans l'arbre.
- `cargo test` n'écrit **pas** dans le vrai dossier : `init_avec(None)` est la porte de sortie,
  et c'est ce qui rend le cas « pas de dossier » testable sans toucher à l'environnement du
  processus de test — même forme que `library::dossier::depuis_home`.
- Le raccord est **éprouvé**, pas supposé : un test unitaire prouve que l'écrivain pose une
  ligne sur le disque, et `src-tauri/tests/journal_fichier.rs` — dans son propre processus,
  parce que `try_init` n'installe qu'un souscripteur par processus — prouve qu'un
  `tracing::info!` du vrai code y atterrit. Le premier a d'ailleurs corrigé ce document : la
  date se place **entre** le préfixe et le suffixe (`hean.2026-11-28.log`), pas à la fin.
