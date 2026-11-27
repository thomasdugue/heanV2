# Protocole — fabriquer le dépôt public neuf

Ce que **toi seul** peux faire : créer un dépôt, pousser dessus, révoquer un justificatif. Rien
ici ne se délègue, et deux étapes ne se rattrapent pas une fois le dépôt public — l'adresse
d'auteur du commit initial, et le dernier balayage. D'où ce document plutôt qu'une improvisation.

Décidé par l'[ADR 0021](adr/0021-un-depot-neuf-a-un-seul-commit.md) : le dépôt public est un dépôt
**neuf, à un seul commit**, parce que les 26 références `refs/pull/<n>/head` que GitHub garde ne
sont pas supprimables et que 25 d'entre elles donnent encore le mot de passe, le DSN et les huit
captures. Le dépôt actuel reste **privé** et garde tout.

Compte **vingt minutes**. Les étapes sont dans l'ordre : chacune suppose la précédente.

## Avant de commencer

| | |
|---|---|
| **Le nom du dépôt public** | `hean` — le nom du produit, et celui du dossier de données (ADR [0003](adr/0003-dossier-de-donnees.md)). Il ne peut pas être `heanV2`, ce nom restant pris par le dépôt privé. Tout ce qui suit l'appelle `hean` |
| **Aucune URL en dur à reprendre** | vérifié le 2026-11-27 : les 408 fichiers suivis ne citent `thomasdugue/heanV2` nulle part. Les 21 URL de dépôt qu'ils portent visent `noirdesktop` et `noir-feedback`, les dépôts de V1 — des références historiques, qui restent justes |
| **Ce qui est en cours** | une seule PR ouverte (la #26), et zéro fichier modifié dans les six worktrees. ⚠️ Mais **« zéro fichier modifié » ne voit pas le travail déjà commité** : mesuré le 2026-11-27, `claude/coreaudio-sortie-8e47a4` porte 3 fichiers et +96 lignes absents de `main`, et sa session travaillait encore. C'est la seconde condition de l'étape 1 |

## 1. Fusionner la PR #26, côté privé

C'est elle qui porte le masquage et le retrait des captures. Le commit initial du dépôt public est
l'arbre d'après, pas d'avant.

```bash
cd $HOME/Documents/Thomas/heanV2 && git switch main && git pull --ff-only && git log --oneline -1 && git status --porcelain
```

La dernière commande doit ne **rien** afficher. L'étape 3 balaie l'arbre de travail et l'étape 4
archive `HEAD` : tant qu'un fichier est modifié, les deux ne parlent pas du même état, et c'est
celui qu'on n'a pas balayé qui serait publié.

### Et toute autre branche qui porte du travail hors de `main`

C'est la **seule** étape où le travail des autres sessions peut se perdre, et elle ne se rattrape
pas : le dépôt neuf n'a **aucun ancêtre commun** avec l'ancien. Une branche laissée derrière ne se
fusionnera plus jamais — elle se **rejoue** par `git format-patch` / `git am`, ou se refait.

```bash
cd $HOME/Documents/Thomas/heanV2 && git fetch -q origin && for b in $(git ls-remote --heads origin | sed 's#.*refs/heads/##' | grep -v '^main$'); do m=$(gh pr list --head "$b" --state merged --json mergedAt --jq '.[0].mergedAt // ""'); t=$(git log -1 --format=%ct origin/$b); if [ -n "$m" ]; then me=$(date -j -u -f '%Y-%m-%dT%H:%M:%SZ' "$m" +%s); [ "$t" -le "$me" ] && continue; fi; echo "$(git diff --numstat origin/main origin/$b | awk '{a+=$1} END{print a+0}') lignes hors de main — $b — fusion ${m:-aucune}, dernier commit $(git log -1 --format=%cI origin/$b)"; done
```

Il ne rend **pas** un verdict, il réduit vingt-et-une branches à une poignée à juger. Relevé le
2026-11-27, il en donne quatre, et les quatre réponses sont différentes :

- `claude/masquer-donnees-sensibles` — **aucune fusion** : c'est la PR #26, que l'étape fusionne.
- `claude/coreaudio-sortie-8e47a4` — **3 fichiers, +96 lignes** hors de `main`
  (`src/lib/ui/Lecture.svelte`, `docs/HISTORY.md`, `docs/PROTOCOLE-SON.md`) : **trois commits
  postérieurs à la fusion de sa PR #25**, qu'**aucune PR ouverte ne suit**. Le seul vrai travail en
  cours. ⚠️ Ne pas attendre qu'il atterrisse tout seul : rien ne le suit, et sa session a rendu son
  rapport. Sa PR se fusionne avant de continuer, ou ses trois fichiers se rejouent par patch.
  ⚠️ Et sa version de `docs/PROTOCOLE-SON.md` porte encore trois chemins `/Users/<compte>` que
  cette PR a passés à `$HOME` : une fois la #26 fusionnée, `./check` est **rouge** sur sa branche
  jusqu'à leur correction — l'exception `BLOC_DE_LANCEMENT` du test d'architecture, que la #26
  retire, est précisément ce qui les laissait passer. Session prévenue le 2026-11-27.
- `claude/coquille-visible-cef91a` et `claude/harnais-extensions-non-lues` — des **queues de deux
  à trois minutes** des 23 et 24 novembre : la session a poussé un dernier commit pendant que le
  *squash* atterrissait. Vérifié ligne par ligne le 2026-11-27, rien n'y est perdu — l'entrée de
  `HISTORY.md` est dans `main` reformulée, et les commentaires de `check` décrivaient deux angles
  morts que `check-sizes.sh` couvre depuis (`src-tauri/tests` le 23, `examples` le 25).

⚠️ **Deux pièges, et les deux m'ont fait crier au loup.** Le compte de lignes a un **fond de 400 à
1 100 lignes** sur les branches mortes : c'est la dérive des fichiers qui grossissent
(`HISTORY.md`, les ADR), pas du travail — elles sont 15 000 lignes *derrière* `main`. Et ne jamais
mesurer à trois points (`origin/main...branche`) : la base de fusion est d'avant le *squash*, donc
le diff **recompte ce qui est déjà dans `main`** — il annonçait 40 fichiers et 5 207 lignes pour
les 96 réelles de `coreaudio-sortie`.

## 2. Révoquer le mot de passe app-specific Apple

Sur appleid.apple.com → **Connexion et sécurité** → **Mots de passe pour applications**. Supprime
celui qui servait à la notarisation de V1.

Indépendant du reste, et à faire même si le dépôt public ne le portera jamais : il a existé en clair
dans **tous** les commits de `main` — le fichier qui le porte est là depuis le commit initial — et il
reste dans le dépôt privé et ses 26 refs de PR. Aucun `scripts/.env.local`
n'existe aujourd'hui (vérifié le 2026-11-27), donc révoquer ne casse aucune compilation.

Côté Sentry, voir l'[ADR 0019](adr/0019-le-depot-passe-en-public.md) : la clé exposée n'est pas
celle du projet courant, il reste à voir si son organisation existe encore sous un autre compte.

## 3. Le dernier balayage

La seule étape qui *refuse* de continuer. **Trois zéros** : un justificatif à format connu, un nom
d'hôte de machine, une capture.

```bash
cd $HOME/Documents/Thomas/heanV2 && git grep -hoIE '(^|[^a-z-])[a-z]{4}-[a-z]{4}-[a-z]{4}-[a-z]{4}([^a-z-]|$)|[0-9a-f]{32}@o[0-9]+|ghp_[A-Za-z0-9]{10}|github_pat_[A-Za-z0-9]{10}|xox[abprs]-[A-Za-z0-9]|AKIA[A-Z0-9]{8}|BEGIN [A-Z ]*PRIVATE KEY' -- . | sed 's/^[^a-z0-9]//; s/[^a-z0-9]$//' | grep -vxF 'xxxx-xxxx-xxxx-xxxx' | wc -l ; git grep -cIE '\b[a-z0-9][a-z0-9-]*\.local\b' -- . | wc -l ; git ls-files 'bootstrap/02-design/reference/captures/*.png' | wc -l
```

Puis un quatrième contrôle, qui **se lit** au lieu de se compter : les deux justificatifs vivants de
ce projet — le mot de passe SMB du NAS et le jeton du worker Cloudflare — n'ont aucun format, donc
seule la forme d'une *affectation* les voit.

```bash
git grep -nIE "(mot_de_passe|password|passwd|token|secret|dsn|api[_-]?key)[a-z_]*[[:space:]]*[:=][[:space:]]*[\"'][^\"' ]{8,}" -- .
```

**Attendu : deux lignes**, les fixtures `"secret-token-1234"` et `"uniq-test-token-9876"` de
`bootstrap/05-postmortem/chantier-wifi-sync/code/rust/wifi_sync/server.rs`. Toute autre ligne
arrête le protocole.

⚠️ **Cette commande est en guillemets doubles, et ça n'est pas un détail de style.** En guillemets
simples, l'apostrophe de la classe `["']` ferme la chaîne : zsh rend `unmatched "` et **rien ne
s'exécute**. C'est la forme qui a été commitée ici le 2026-11-27, avec son « attendu : deux lignes »
— j'avais rejoué une variante, pas le texte du document. Une commande écrite dans un document se
rejoue **telle qu'écrite**, copiée depuis le fichier : c'est la seule lecture qui vaut.

**Échec** : n'importe quel nombre au-dessus de zéro. Relance sans les deux derniers filtres, la
sortie dit le fichier et la ligne. Ne passe pas à l'étape 4 : après, ce qui est publié est publié.

⚠️ **Trois choses ont été corrigées ici le 2026-11-27, chacune sur de la matière réelle.**

1. **Le motif était trop large** : `[a-z]{4}-` répété quatre fois sans ancre attrapait 16 fichiers
   de classes CSS en kebab-case (`track-info-edit-field`). Un balayage qui crie au loup seize fois
   ne se lit pas jusqu'au bout, et c'est exactement comme ça qu'un vrai justificatif passe.
2. **L'exception se filtrait par ligne**, donc elle masquait aussi un vrai secret posé sur la même
   ligne qu'un gabarit. Elle filtre maintenant la **valeur** (`grep -oE` puis `grep -vxF`) : les
   sept correspondances du dépôt sont les sept gabarits `xxxx-xxxx-xxxx-xxxx`, et rien d'autre ne
   peut se glisser derrière.
3. **Le troisième compte nommait le nom d'hôte du NAS**, donc ce document le remettait dans le
   dépôt — la donnée dans le fichier censé prouver son départ, et un compteur qui ne pouvait plus
   atteindre zéro. Il cherche désormais **tout** nom d'hôte `.local`, ce qui couvre la classe
   entière sans écrire l'aiguille. Pour vérifier un nom précis, tape-le à l'exécution :
   `read -r NOM; git grep -liI "$NOM" -- . | wc -l` — il ne s'écrit nulle part.

### Et ce que le balayage ne peut pas juger

Trois choses partiront, aucune n'est un justificatif ni une donnée personnelle — donc aucun motif ne
les verra, et c'est à toi d'en décider maintenant plutôt qu'après :

- **471 lignes de stratégie commerciale** : `pricing-gtm.md` (prix d'appel, « €49 puis €69 »),
  `market-viability.md` (positionnement concurrentiel) et `gtm-playbook-solo-dev.md` (plan de
  lancement), dans `bootstrap/01-produit/recherche/`. L'[ADR 0019](adr/0019-le-depot-passe-en-public.md)
  l'a accepté en conséquence explicite ; si tu changes d'avis, c'est le moment, pas après.
- **Les Issues sont actives** sur un dépôt créé public, et `.github/ISSUE_TEMPLATE/` invite à
  signaler des bugs d'une beta **jamais distribuée** (ADR [0017](adr/0017-ce-que-thomas-a-tranche-le-25-novembre.md)),
  en pointant un chemin de journaux périmé depuis l'ADR [0003](adr/0003-dossier-de-donnees.md).
- **Deux copies de `GeistMono-Variable.woff2`** sont redistribuées sans leur licence. L'OFL demande
  qu'elle accompagne la fonte ; le dépôt n'a aucun fichier de licence (choix de l'ADR 0019 pour le
  code, qui ne dit rien des fontes).

## 4. Fabriquer l'arbre neuf

`git archive` n'exporte **que les fichiers suivis** : ni `scripts/.env.local`, ni `node_modules`, ni
`.DS_Store`, ni les captures que `.gitignore` refuse désormais (ADR
[0020](adr/0020-les-captures-sortent-du-depot.md)). C'est le mécanisme qui rend la faute
inexprimable, plutôt qu'une consigne de ne pas copier le mauvais dossier.

```bash
rm -rf $HOME/hean-public && mkdir -p $HOME/hean-public && git -C $HOME/Documents/Thomas/heanV2 archive HEAD | tar -x -C $HOME/hean-public && find $HOME/hean-public -type f | wc -l
```

Les deux comptes doivent être **égaux** — c'est l'égalité qui vérifie, pas le chiffre, qui bouge à
chaque commit (408 le 2026-11-27) :

```bash
git -C $HOME/Documents/Thomas/heanV2 ls-files | wc -l
```

## 5. Le commit initial, avec la bonne adresse

L'adresse est **dans la commande**, pas dans un rappel : `git config user.email` vaut encore
l'adresse personnelle sur ce poste (vérifié le 2026-11-27), et un commit initial ne se corrige pas
sans tout refaire.

```bash
cd $HOME/hean-public && git init -b main -q && git add -A && git -c user.email=45101565+thomasdugue@users.noreply.github.com -c user.name='Thomas Dugué' commit -q -m "heanV2 — l'arbre au premier jour public

Réécriture de Hean, lecteur de musique audiophile macOS. Ce dépôt commence à un
seul commit : l'historique du développement reste dans un dépôt privé, et ce
qu'il a décidé est ici, dans docs/adr/ et docs/HISTORY.md (ADR 0021)." && git log --format='%an <%ae>%n%s' -1
```

**Échec** : l'adresse affichée contient `gmail`. Recommence en supprimant le dossier.

## 6. Créer le dépôt public et pousser

```bash
cd $HOME/hean-public && gh repo create thomasdugue/hean --public --source=. --remote=origin --description "Hean — lecteur de musique audiophile pour macOS. Tous droits réservés." --push
```

Puis, dans le même mouvement, **les deux protections que GitHub offre gratuitement sur un dépôt
public** — et qui sont le seul mécanisme disponible tant qu'INV-29 n'existe pas :

```bash
echo '{"security_and_analysis":{"secret_scanning":{"status":"enabled"},"secret_scanning_push_protection":{"status":"enabled"}}}' | gh api -X PATCH repos/thomasdugue/hean --input - --jq '.security_and_analysis'
```

La seconde **refuse le push** qui porte un justificatif reconnu, au lieu de le signaler après coup —
et dans un dépôt public, après coup est trop tard : la PR aura créé sa `refs/pull/1/head`. Les mêmes
deux réglages sont dans Settings → Code security, si l'API bronche.

## 7. Vérifier côté GitHub, pas côté disque

Un clone frais : c'est ce qu'un lecteur extérieur verra, et la seule preuve qui compte.

```bash
gh api repos/thomasdugue/hean/commits --jq 'length' ; gh api repos/thomasdugue/hean/git/refs --jq '.[].ref' ; rm -rf $HOME/hean-verif && git clone -q https://github.com/thomasdugue/hean.git $HOME/hean-verif && git -C $HOME/hean-verif rev-list --all --count ; git -C $HOME/hean-verif grep -hoIE '(^|[^a-z-])[a-z]{4}-[a-z]{4}-[a-z]{4}-[a-z]{4}([^a-z-]|$)|[0-9a-f]{32}@o[0-9]+|ghp_[A-Za-z0-9]{10}|github_pat_[A-Za-z0-9]{10}|xox[abprs]-[A-Za-z0-9]|AKIA[A-Z0-9]{8}|BEGIN [A-Z ]*PRIVATE KEY' -- . | sed 's/^[^a-z0-9]//; s/[^a-z0-9]$//' | grep -vxF 'xxxx-xxxx-xxxx-xxxx' | wc -l
```

Attendu : `1`, puis `refs/heads/main` **seul**, puis `1`, puis `0`.

⚠️ C'est le **même** balayage qu'à l'étape 3, et c'est volontaire : la seule preuve d'après-publication
doit rendre le même verdict que celle d'avant, sinon on ne sait pas lequel croire. Une première
version écrivait ici `git grep -lIE … | grep -v …`, où `-l` sort des **noms de fichiers** : le filtre
ne trouvait aucun gabarit à exclure et l'étape annonçait cinq justificatifs dans un dépôt déjà
public. Repasser le dépôt en privé pour rien, ou apprendre à ignorer le nombre — les deux issues
sont mauvaises.

## 8. Le dossier de travail change

On **clone** le dépôt neuf à côté ; on ne rebranche pas l'ancien. Rebrancher laisserait un dépôt
local dont `main` ne correspond à rien côté serveur, avec 25 références de suivi périmées.

```bash
git clone git@github.com:thomasdugue/hean.git $HOME/Documents/Thomas/hean && cd $HOME/Documents/Thomas/hean && git config user.email 45101565+thomasdugue@users.noreply.github.com && git config core.hooksPath tools/git-hooks && npm install && ./check
```

⚠️ **La ligne `user.email` n'est pas décorative.** L'étape 5 protège le commit n°1 ; sans elle, le
commit n°2 et tous les suivants repartiraient avec l'adresse personnelle, cette fois dans un dépôt
**public**, et leur PR créerait la `refs/pull/1/head` que l'ADR 0021 décrit comme indestructible —
l'argument de la décision retourné contre son exécution. Le réglage de compte GitHub « Block
command line pushes that expose my email » ferme la même porte pour tous les dépôts à venir, et
rend la faute inexprimable au lieu de la rappeler.

`$HOME/Documents/Thomas/heanV2` devient **l'archive, à ne pas supprimer** : c'est la seule copie
locale de l'historique. Ses six worktrees ne portent aucun travail non commité, et les branches qui
paraissent en avance sur `main` le sont par *squash* — leur contenu est dans l'arbre.

## 9. La première PR est le test de la CI

C'est tout l'objet de la bascule : vérifier que le job tourne et rend un verdict, sur des runners
gratuits cette fois. La CI n'utilise aucun secret, il n'y a donc rien à reporter dans les réglages
du dépôt neuf.

**Échec** : le job échoue encore en trois secondes sur l'annotation de facturation. Cela voudrait
dire que le blocage porte sur le compte et pas sur le dépôt — à ce stade, repasser le dépôt en
privé ne coûte rien et l'alternative « supprimer le run redondant sur `main` » de l'ADR 0019 reste
disponible.

## Ce qui ne doit jamais être tapé

`git push --mirror`, `git push --all`, `git push --tags` depuis
`$HOME/Documents/Thomas/heanV2` vers le dépôt neuf. Chacun y enverrait le passé que tout ce
protocole existe pour laisser derrière. C'est la raison de l'étape 4 : dans
`$HOME/hean-public`, ce passé n'existe pas, donc la faute n'est pas exprimable.
