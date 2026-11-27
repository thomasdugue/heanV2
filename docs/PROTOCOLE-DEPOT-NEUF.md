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
| **Rien en cours à sauver** | mesuré le 2026-11-27 : une seule PR ouverte (la #26), et **zéro fichier modifié** dans les six worktrees |

## 1. Fusionner la PR #26, côté privé

C'est elle qui porte le masquage et le retrait des captures. Le commit initial du dépôt public est
l'arbre d'après, pas d'avant.

```bash
cd $HOME/Documents/Thomas/heanV2 && git switch main && git pull --ff-only && git log --oneline -1
```

## 2. Révoquer le mot de passe app-specific Apple

Sur appleid.apple.com → **Connexion et sécurité** → **Mots de passe pour applications**. Supprime
celui qui servait à la notarisation de V1.

Indépendant du reste, et à faire même si le dépôt public ne le portera jamais : il a existé en clair
dans 60 commits, et il reste dans le dépôt privé et ses 26 refs de PR. Aucun `scripts/.env.local`
n'existe aujourd'hui (vérifié le 2026-11-27), donc révoquer ne casse aucune compilation.

Côté Sentry, voir l'[ADR 0019](adr/0019-le-depot-passe-en-public.md) : la clé exposée n'est pas
celle du projet courant, il reste à voir si son organisation existe encore sous un autre compte.

## 3. Le dernier balayage

La seule étape qui *refuse* de continuer. Les trois comptes doivent valoir **zéro** : un
justificatif, une capture, le nom du NAS.

```bash
cd $HOME/Documents/Thomas/heanV2 && git grep -nIE '(^|[^a-z-])[a-z]{4}-[a-z]{4}-[a-z]{4}-[a-z]{4}([^a-z-]|$)|[0-9a-f]{32}@o[0-9]+|ghp_[A-Za-z0-9]{10}|github_pat_[A-Za-z0-9]{10}|xox[abprs]-[A-Za-z0-9]|AKIA[A-Z0-9]{8}|BEGIN [A-Z ]*PRIVATE KEY' -- . | grep -v 'xxxx-xxxx-xxxx-xxxx' | wc -l ; git ls-files 'bootstrap/02-design/reference/captures/*.png' | wc -l ; git grep -liI 'dugstation' -- . | wc -l
```

**Échec** : n'importe quel nombre au-dessus de zéro. Relance sans les deux derniers filtres, la
sortie dit le fichier et la ligne. Ne passe pas à l'étape 4 : après, ce qui est publié est publié.

⚠️ Le motif a été **resserré sur de la matière réelle le 2026-11-27** : `[a-z]{4}-` répété quatre
fois sans ancre attrapait 16 fichiers de classes CSS en kebab-case (`track-info-edit-field`). Un
balayage qui crie au loup seize fois ne se lit pas jusqu'au bout, et c'est exactement comme ça qu'un
vrai justificatif passe. La seule exception est `xxxx-xxxx-xxxx-xxxx`, le **gabarit** écrit dans
quatre documents dont `secrets.env.template` ; elle est étroite et nommée, et aucun mot de passe
réel ne la prend.

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

## 7. Vérifier côté GitHub, pas côté disque

Un clone frais : c'est ce qu'un lecteur extérieur verra, et la seule preuve qui compte.

```bash
gh api repos/thomasdugue/hean/commits --jq 'length' ; gh api repos/thomasdugue/hean/git/refs --jq '.[].ref' ; rm -rf $HOME/hean-verif && git clone -q https://github.com/thomasdugue/hean.git $HOME/hean-verif && git -C $HOME/hean-verif rev-list --all --count ; git -C $HOME/hean-verif grep -lIE '(^|[^a-z-])[a-z]{4}-[a-z]{4}-[a-z]{4}-[a-z]{4}([^a-z-]|$)|[0-9a-f]{32}@o[0-9]+' -- . | grep -v 'xxxx-xxxx-xxxx-xxxx' | wc -l
```

Attendu : `1`, puis `refs/heads/main` **seul**, puis `1`, puis `0`.

## 8. Le dossier de travail change

On **clone** le dépôt neuf à côté ; on ne rebranche pas l'ancien. Rebrancher laisserait un dépôt
local dont `main` ne correspond à rien côté serveur, avec 25 références de suivi périmées.

```bash
git clone git@github.com:thomasdugue/hean.git $HOME/Documents/Thomas/hean && cd $HOME/Documents/Thomas/hean && git config core.hooksPath tools/git-hooks && npm install && ./check
```

`$HOME/Documents/Thomas/heanV2` devient **l'archive, à ne pas supprimer** : c'est la seule copie
locale de l'historique. Ses six worktrees ne portent aucun travail non commité, et les treize
branches qui paraissent en avance sur `main` le sont par *squash* — leur contenu est dans l'arbre.

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
