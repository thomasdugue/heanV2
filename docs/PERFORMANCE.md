# Performance et poids — ce que la stack change

> Question posée le 2026-11-21 : la promesse de Hean est une app **très légère, peu gourmande en
> mémoire, aux performances redoutables**. Est-ce que TypeScript + Svelte 5 + SQLite + tauri-specta
> coûte quelque chose là-dessus ?
>
> Réponse courte : **non, et c'est chiffrable.** Mais il y a trois pièges précis, tous évitables, et
> un seul mérite un garde-fou en CI.

## Les chiffres réels de V1

Mesurés sur la release v0.2.0-beta.1 et le code actuel :

| | Poids |
|---|---|
| **Frontend complet** (16 076 l. de JS + 10 654 l. de CSS + HTML + police Geist Mono) | **960 Ko** |
| **Binaire Rust** (release, LTO, strip, codegen-units=1) | **16 Mo** |
| **Tarball updater** `Hean.app.tar.gz` | **29 Mo** |
| **DMG distribué** | **33 Mo** |

Le chiffre qui décide de tout : **le frontend pèse 960 Ko, soit ~3 % du binaire Rust et ~1 % de
l'app distribuée.** Tout ce qu'on fait au frontend — framework, bundler, types — déplace ~1 % du
poids de l'application. L'écart entre les 16 Mo de binaire et les 29-33 Mo d'archive, c'est
essentiellement les **~92 dylibs Samba** de `libsmbclient`, et aucune des six décisions n'y touche.

> L'app installée localement (`/Applications/Hean.app`, 16 Mo, **sans dossier `Frameworks`**) est une
> build locale sans bundling de dylibs. La répartition exacte binaire / dylibs dans l'app distribuée
> demande une vraie build de release pour être mesurée — à faire en phase 7.

## Impact décision par décision

### TypeScript — coût nul, probablement négatif

TypeScript **disparaît à la compilation**. Il ne produit aucun code à l'exécution. Zéro octet, zéro
cycle.

Et V1 servait son JavaScript **non minifié, avec les commentaires** (`frontendDist: "../src"`, pas de
bundler). Avec Vite, le même code minifié et tree-shaké sortira probablement entre 150 et 250 Ko au
lieu de 550 Ko. **Le frontend de V2 devrait être plus léger que celui de V1.**

### Svelte 5 — quelques Ko, et des opérations DOM plus efficaces

Svelte est un **compilateur**, pas un runtime. Il n'y a pas de VDOM à embarquer. Le cœur de
réactivité des runes pèse quelques Ko gzippés, et les composants compilent en opérations DOM
directes — souvent moins de code que l'équivalent écrit à la main.

Comparé à V1, qui faisait du `innerHTML` et du `querySelector` manuels : à poids égal ou inférieur,
avec moins de reflows.

### SQLite — +~1 Mo sur le binaire, et de la mémoire en moins

`rusqlite` en `bundled` compile l'amalgame SQLite dans le binaire : de l'ordre de **1 Mo**, soit
**+6 % du binaire et +3 % du DMG**.

En échange, du côté mémoire, c'est un gain : V1 chargeait `tracks_cache.json` **en entier** dans la
mémoire Rust (un `Vec` de toutes les pistes) **et** dans le JS (`library.tracks` plus la Map
`tracksByPath`). Deux copies complètes de la bibliothèque en RAM. SQLite permet de ne garder que ce
qui est affiché — à condition de le vouloir (voir le piège n°2).

### tauri-specta — coût nul côté exécution

Les types TypeScript sont **générés au build** et disparaissent à la compilation. Côté Rust, les
`impl Type` ajoutent des métadonnées de l'ordre de quelques dizaines de Ko. Si l'export des bindings
est fait dans un test ou un binaire séparé (la pratique recommandée), même ça ne part pas en release.

### L'audio — strictement aucun impact

C'est le point le plus important pour un lecteur audiophile : **toute la chaîne temps réel est en
Rust**. Décodage Symphonia, resampler, RingBuffer lock-free, EQ biquad, volume, callback CoreAudio.
Le frontend n'intervient que pour envoyer des commandes et afficher une position interpolée.

Le choix du framework UI a **zéro effet** sur la latence, le bit-perfect, le gapless ou les glitchs.
La promesse « performances redoutables » vit dans le thread audio, et aucune des six décisions ne
l'approche.

## Les trois pièges, et comment ne pas tomber dedans

### Piège n°1 — envelopper 10 000 pistes dans de la réactivité

Svelte 5 crée un signal par `$state`. Déclarer la bibliothèque entière en `$state` produirait des
dizaines de milliers d'objets réactifs : mémoire multipliée, et un coût à chaque mutation.

**Règle** : la liste de pistes vit en `$state.raw` (ou hors de tout état réactif). La réactivité ne
s'applique qu'à ce qui est affiché et à l'état d'UI — vue courante, tri, sélection, filtre. `$state.raw`
existe exactement pour ça.

### Piège n°2 — perdre le pool de nœuds du virtual scroll

V1 tient 60 fps sur 10 000+ pistes grâce à un pool **fixe de 60 nœuds** recyclés, avec un item à
48 px et de la délégation d'événements. Laisser un framework rendre 10 000 lignes, même avec une
réactivité fine, sera plus lent que ça.

**Règle** : le virtual scroll reste une implémentation explicite — un composant qui rend 60 lignes et
met à jour leurs props. C'est parfaitement idiomatique en Svelte, et ça conserve la performance de V1.

### Piège n°3 — la dérive npm 🔴 le seul vrai risque nouveau

V1 avait **zéro dépendance de production côté frontend**. Pas de bundler, pas de framework, donc
aucune porte d'entrée. Ouvrir `package.json` ouvre la porte : une lib de dates, un pack d'icônes, un
composant de slider, un utilitaire de couleurs — et le bundle passe de 200 Ko à 2 Mo sans qu'aucune
décision n'ait été prise.

**C'est ça qui menace la promesse de légèreté, pas Svelte.** Et c'est ce qui justifie un garde-fou
mécanique plutôt qu'une bonne résolution : voir ci-dessous.

## Le garde-fou : `check-weight.sh`

Ajouté à `./check`. Inerte jusqu'à ce qu'un build existe, puis il refuse :

| Budget | Valeur |
|---|---|
| `dist/` total (bundle construit) | **≤ 600 Ko** — V1 servait 960 Ko non minifiés ; V2 doit faire mieux |
| Dépendances de production npm | **≤ 3** (Svelte compte comme devDependency : il disparaît au build) |

Toute dépendance de production supplémentaire demande un **ADR** : ce qu'elle apporte, son poids
mesuré, et pourquoi on ne l'écrit pas à la main. C'est volontairement contraignant — c'est le prix de
la promesse.

## Les budgets de performance restent à mesurer sur V1

Ce document traite du **poids**. Les budgets de **vitesse** sont encore vides parce que V1 n'a pas été
mesuré. C'est le préalable A3 de [`DEMARRER.md`](DEMARRER.md), et cette question le fait passer de
« phase 5 » à « maintenant » : sans les chiffres de V1, on ne pourra pas prouver que V2 ne régresse
pas.

À mesurer sur V1, avec la vraie bibliothèque, et à reporter dans
[`../bootstrap/06-setup-v2/HARNAIS.md`](../bootstrap/06-setup-v2/HARNAIS.md) §8 :

| Mesure | V1 | Budget V2 |
|---|---|---|
| Démarrage jusqu'à UI utilisable | à mesurer | ≤ V1 |
| Images par seconde du virtual scroll | 60 attendu | 60 |
| Mémoire après ~1 000 covers défilées | à mesurer | ≤ V1 |
| Recherche sur la bibliothèque complète | < 16 ms (documenté) | < 16 ms |
| Poids du DMG | 33 Mo | ≤ 35 Mo |

## Conclusion

| | Effet |
|---|---|
| Poids de l'app | **+~1 Mo sur 33** (SQLite), moins ce que la minification fait gagner. Dans le bruit. |
| Mémoire | **Neutre à meilleur**, si le piège n°1 est évité. |
| Performance perçue | **Neutre à meilleur** : démarrage plus rapide via index SQLite, scroll identique si le pool est conservé. |
| Performance audio | **Inchangée.** Rien dans ces décisions n'approche le thread temps réel. |
| Risque réel | **La dérive des dépendances npm** — traitée par un budget en CI, pas par la discipline. |
