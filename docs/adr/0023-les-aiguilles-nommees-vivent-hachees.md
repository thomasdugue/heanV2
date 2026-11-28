# 0023 — Les données sensibles sont tenues par un détecteur, et les aiguilles nommées vivent hachées

- **Date** : 2026-11-28
- **État** : Acceptée
- **Décideur** : **Thomas**, le 2026-11-28 — « Les trois premiers », en réponse aux trois trous
  relevés par une rétrospective des sept premiers jours.
- **Complète** : [`0019`](0019-le-depot-passe-en-public.md), [`0021`](0021-un-depot-neuf-a-un-seul-commit.md)
- **Tranche** la décision ouverte « INV-29 » de [l'index](README.md).

## Contexte

En une semaine, **quatre données sensibles sont entrées dans un dépôt public** et en sont
ressorties par la **relecture d'un agent**, jamais par une vérification : le nom d'hôte du NAS
dans 7 fichiers, un mot de passe app-specific Apple *entre deux lignes déjà masquées*, une clé
Sentry, l'e-mail d'un tiers. Une relecture à l'œil a vu deux justificatifs sur trois.

La fenêtre ne se referme pas : le dépôt est public **depuis sa création**, et une PR fusionnée
laisse une `refs/pull/<n>/head` que le propriétaire ne peut pas supprimer — mesuré sur les 26
d'ici ([`0021`](0021-un-depot-neuf-a-un-seul-commit.md)). Ce qui entre ne sort plus.

Les motifs, eux, existaient déjà : les quatre contrôles du « dernier balayage » de
[`../PROTOCOLE-DEPOT-NEUF.md`](../PROTOCOLE-DEPOT-NEUF.md) §3, corrigés trois fois sur de la
matière réelle. Ils étaient écrits pour être **tapés une fois, à la main, avant une bascule** :
de la discipline, que l'[ADR 0005](0005-entretien-du-harnais-et-orchestration.md) refuse par
principe. Le harnais est gelé ([`0017`](0017-ce-que-thomas-a-tranche-le-25-novembre.md)), et le
seuil est franchi deux fois : le défaut est arrivé quatre fois, et il porte un risque légal.

## Décision

**Les quatre contrôles deviennent un test d'architecture**, `aucune-donnee-sensible.test.ts`
(INV-29, 🔴), qui tourne à chaque `./check`. Il lit **tout** le dépôt, `bootstrap/` compris :
les trois justificatifs fuités y vivaient, et un secret ne devient pas inoffensif parce que le
fichier qui le porte est historique.

**Les aiguilles nommées** — le nom du NAS, une adresse personnelle : des données sans format,
qu'aucune classe ne cerne — **vivent hachées** dans `tools/fuites-aiguilles.txt` (sha256 du
jeton en minuscules, une étiquette par ligne). Le détecteur hache les jetons des fichiers
suivis et compare. La valeur ne s'écrit **nulle part** : ni dans le dépôt, ni dans la sortie du
test, qui ne nomme que l'étiquette.

## Alternatives écartées

- **Chercher la classe entière** (`*.local` pour les noms d'hôte), ce que fait le protocole.
  Mesuré le 2026-11-28 : douze noms, **tous des gabarits** (`nas.local`, `_smb._tcp.local`,
  `a.local`) et aucun vrai. Un détecteur qui crie au loup douze fois ne se lit pas jusqu'au
  bout — c'est le premier des trois pièges que ce balayage a déjà payés.
- **Écrire l'aiguille en clair** dans le détecteur : c'est le troisième piège, déjà payé le
  2026-11-27 — la donnée remise dans le fichier censé prouver son départ.
- **La taper à l'exécution** (`read -r NOM`), ce que le protocole propose en secours. Ne marche
  que si un humain est là ; ni en CI, ni au pre-commit, ni dans une session d'agent.
- **Un fichier local ignoré par git**, hors du dépôt. Marche chez Thomas, ne marche pas en CI,
  et un contrôle qui ne tourne qu'à un endroit redevient de la discipline.

## Conséquences acceptées

- **Un hachage n'est pas un coffre** : un nom d'hôte court se retrouve par force brute. Ce
  n'est pas la menace visée — la menace est de **republier par inadvertance** ce qu'on vient de
  retirer, et contre celle-là une empreinte suffit.
- **Zéro aiguille enregistrée est l'état du jour.** Le mécanisme existe, en poser une coûte une
  ligne, et le test affiche le compte pour que ce zéro se lise comme un choix et non comme une
  preuve. **Enregistrer le nom du NAS reste à faire, et c'est à Thomas de le décider.**
- Deux défauts des motifs d'origine sont corrigés en les portant, et les deux donnaient un vert
  imméritée : le contrôle des noms d'hôte cherchait `\b…\b` en **ERE POSIX**, où `\b` ne matche
  jamais — il rendait zéro sans rien lire depuis le 2026-11-27 ; et celui des affectations était
  sensible à la casse, donc aveugle à `PASSWORD=…`. Le document qui les porte **reste à
  corriger** : il est tenu par une autre session (ADR 0013).
