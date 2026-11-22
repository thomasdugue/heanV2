# 0006 — Le socle de phase 0, et la forme que prend le harnais

- **Date** : 2026-11-22
- **État** : Acceptée
- **Décideur** : Thomas
- **Suit** : [0002](0002-stack-technique.md), [0005](0005-entretien-du-harnais-et-orchestration.md)

## Contexte

L'[ADR 0005](0005-entretien-du-harnais-et-orchestration.md) acte *comment* le harnais
s'entretient (généré > apparié > réclamé) et *quand* on délègue. Il ne dit pas sous quelle
forme tout cela existe dans le dépôt. La phase 0 l'a construit ; ces choix-là engagent le
reste du projet, donc ils sont ici.

## Décisions

### 1. `./check` est la seule commande, et la CI n'en a qu'une étape

La CI est **un job, sur `macos-latest`, qui lance `./check`** et rien d'autre. En V1 la CI
avait ses propres étapes (`node --check`, Jest, `cargo test`) : elle pouvait donc diverger du
local sans que personne le voie. Ajouter une vérification se fait désormais *dans* `./check`,
et la CI en hérite — la divergence devient impossible, pas seulement improbable.

`macos-latest` n'est pas un confort : CoreAudio, souvlaki et `libsmbclient` sont `cfg(macos)`.
`./check` installe lui-même les dépendances npm manquantes, ce qui supprime le dernier cas
particulier que l'agent `test-runner` devait connaître.

**Écarté** : garder un job `js` et un job `rust` séparés comme en V1 (plus parallèle, mais
c'est exactement la porte par laquelle la CI diverge).

### 2. Les tests d'architecture sont en TypeScript, y compris ceux du backend Rust

`HARNAIS.md` §2 autorise explicitement le grep à règles explicites plutôt qu'une analyse
d'arbre. On en tire une conséquence : **une seule liste de règles de forme, à un seul
endroit**, qui couvre le CSS, le TypeScript *et* le Rust. Chercher la règle « pas de
`println!` » dans `tests/architecture/` plutôt que dans un `#[test]` perdu dans le crate est
ce qui fait qu'on la trouve.

Et une règle qui coûte peu et change tout : **chaque test d'architecture embarque un
auto-test de son détecteur**, qui le lance sur un échantillon fautif. Sans cela, un test de
forme sur un dossier encore vide passe au vert sans rien garantir — il donne de la confiance
sans en produire, ce qui est pire que son absence.

**Écarté** : `dependency-cruiser` (une dépendance de plus, et il ne voit pas le Rust).

### 3. `src/styles/tokens.css` est généré depuis la cible de parité

`bootstrap/02-design/tokens.css` est un *document* (il contient un `@font-face` au mauvais
chemin et des pseudo-déclarations `<runtime>` hors bloc) : il n'est pas servable. La feuille
réellement servie en est donc **extraite par un script**, avec le catalogue et les valeurs
typées côté TypeScript.

Conséquence voulue : **les tokens de V2 ne peuvent pas diverger de la cible de parité.** Les
changer demande de changer la cible, donc un ADR — ce qui est précisément ce qui arrivera à
la passe DM Sans. En V1, `DESIGN-SYSTEM.md` divergeait du code sur 5 couleurs, une ombre et
la typographie, et personne ne pouvait dire lequel faisait foi.

Les 10 variables posées par JS à l'exécution reçoivent une **valeur par défaut obligatoire** :
sans elle, toute déclaration qui les lit est invalidée en silence avant le premier
`setProperty()`. C'est la mécanique exacte de INV-20.

### 4. `withGlobalTauri: false`, et un seul module parle à Tauri

`window.__TAURI__` n'existe pas. Tout passe par `src/ipc/client.ts`, qui réexporte les
bindings générés. C'est ce qui rend le client **substituable**, donc `invoke` mockable — les
3 suites de tests que V1 avait dû skipper redeviennent écrivables. Un test d'architecture
refuse tout import de `@tauri-apps/api` hors de `src/ipc/`.

### 5. Un secret manquant ne compile pas, avec une porte de sortie tracée

`build.rs` écrit un `compile_error!()` dans `OUT_DIR` que `src/secrets.rs` inclut : **un
build release sans secrets ne compile pas**. V1 s'en protégeait avec un wrapper `release.sh`
qu'il suffisait d'oublier une fois.

`HEAN_ALLOW_MISSING_SECRETS=1` dégrade l'erreur en avertissement, pour un build local de
test. Ce n'est pas une faiblesse : c'est la règle d'or des garde-fous de l'ADR 0005 — sans
porte de sortie explicite, un garde-fou est contourné en une semaine.

### 6. Les invariants ont un format lisible par une machine, et ils sont 21

Chaque règle porte quatre champs obligatoires (`pile`, `gravité`, `couverture`, `pourquoi`)
que `tools/check-invariants.mjs` lit. Un invariant sans test fait échouer `./check`, un test
qui cite une règle disparue aussi, et **les compteurs annoncés dans le skill sont vérifiés
contre la réalité**.

Cette dernière vérification vient d'un constat fait en phase 0 : V1 avait **20** invariants,
pas 19. Cinq documents annonçaient 19 — le compte datait d'avant l'ajout de la règle CSS le
2026-10-29, et rien ne le vérifiait. C'est la dérive de documentation du postmortem, en
miniature, et elle ne peut plus se reproduire.

S'y ajoute une 21ᵉ règle née du postmortem : **INV-21, aucun `println!` dans le backend**.
Gravité 🔴 par défaut, faute de classement — à confirmer.

### 7. Le bloc `updater` n'est pas repris en phase 0

La configuration de V1 pointe vers un endpoint qui **renvoie 404 depuis le lancement** : la
mise à jour automatique n'a jamais fonctionné pour aucun testeur. Recopier cette
configuration reviendrait à recommitter un bug connu. Elle reviendra en phase 6, avec la
vérification qui l'aurait vu : un appel réel sur l'endpoint, dans l'audit hebdomadaire.

### 8. Pas d'ESLint : `oxlint` + `svelte-check --fail-on-warnings`

`tsc --noEmit` en `strict` (plus `noUncheckedIndexedAccess` et `exactOptionalPropertyTypes`)
et `svelte-check` en échec sur avertissement font l'essentiel du travail, y compris sur les
fichiers `.svelte` qu'`oxlint` ne lit pas. Ajouter ESLint apporterait une quinzaine de
dépendances de développement et un graphe de versions à entretenir, pour un gain marginal.

**Écarté** : ESLint + `typescript-eslint` + `eslint-plugin-svelte`, à reconsidérer le jour où
une règle utile manque réellement.

### 9. Les 24 comportements UX sont rouges, et hors du verdict de `./check`

Ils échouent tous, exprès : aucune feature n'existe. Les compter dans le verdict rendrait
`./check` rouge en permanence, donc illisible — et un harnais qu'on n'écoute plus est pire
que pas de harnais. Ils sont lancés à part, affichés comme **tableau de bord** (« N
comportements restent à porter »), et un test d'appariement, lui vert, garantit qu'aucun
comportement du contrat n'a été oublié et qu'aucun test ne cite un comportement disparu.

## Conséquences acceptées

- **`tauri-specta` pour Tauri v2 n'existe qu'en release candidate** (`2.0.0-rc.29`, avec
  `specta` épinglé à l'exact). C'est le cœur de la décision IPC de l'ADR 0002. Mitigation :
  les versions sont épinglées au patch près et les bindings sont commités, donc un changement
  de sortie de l'outil se voit dans un diff au lieu de se découvrir à l'exécution.
- **Un nouvel artefact généré fait échouer `./check` tant qu'il n'est pas indexé par git.**
  C'est voulu (c'est ce qui détecte la dérive), et le message le dit.
- **L'icône de l'application est un repère, pas la vraie.** Elle est dérivée de
  `app_logo.svg`, qui n'est pas carré. La vraie icône de V1 n'est pas dans le bootstrap :
  elle arrivera en phase 6.
- **`transparent: true` est repris tel quel de V1**, sans `macOSPrivateApi`. La fenêtre de V1
  n'était donc probablement pas réellement transparente. C'est la parité, et la question se
  tranchera en regardant l'app tourner.
- **`@types/node` est visible depuis tout le projet**, parce qu'il n'y a qu'un `tsconfig.json`.
  Si du code frontend finit par appeler une API Node, il faudra un second tsconfig.
