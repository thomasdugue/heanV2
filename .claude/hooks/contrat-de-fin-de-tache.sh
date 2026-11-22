#!/usr/bin/env bash
# LE CONTRAT DE FIN DE TÂCHE (ENTRETIEN-DU-HARNAIS.md, niveau 3).
#
# Un seul garde-fou : il regarde ce qui a changé et RÉCLAME ce qui manque. Il généralise
# l'ancien invariant-reminder.sh de V1, qui ne couvrait qu'un seul artefact.
#
# ⚠️ RÈGLE D'OR : il offre TOUJOURS une sortie explicite. « Non, pas de règle ici » est une
# réponse valable — mais il faut la donner, et elle est enregistrée. Un garde-fou sans porte
# de sortie est contourné en une semaine, et c'est alors pire que rien.
#
# Il ne réclame jamais ce qui est générable : chaque question posée à un humain est un aveu
# d'échec du niveau 1.

input=$(cat)
[ "$(printf '%s' "$input" | jq -r '.stop_hook_active // false')" = "true" ] && exit 0

root=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
cd "$root" || exit 0
base=$(git merge-base HEAD origin/main 2>/dev/null) || exit 0

change=$( { git diff --name-only "$base"; git ls-files --others --exclude-standard; } | sort -u )
[ -z "$change" ] && exit 0
a() { printf '%s\n' "$change" | grep -qE "$1"; }

demandes=""
ajouter() { demandes="${demandes}
- $1"; }

# 1. Une commande IPC a bougé → les types doivent avoir été régénérés.
if a '^src-tauri/src/(commands/|lib\.rs)' && ! a '^src/ipc/bindings\.ts$'; then
  ajouter "Une commande IPC a changé mais \`src/ipc/bindings.ts\` n'est pas dans le diff : lance \`npm run gen\` et commite le résultat."
fi

# 2. La source des tokens a bougé → la feuille servie doit suivre.
if a '^bootstrap/02-design/tokens\.css$' && ! a '^src/styles/tokens\.css$'; then
  ajouter "La source des tokens a changé sans que \`src/styles/tokens.css\` suive : lance \`npm run gen\`. Et si la cible de parité change vraiment, ça demande un ADR (la cible actuelle est fixée par l'ADR 0004)."
fi

# 3. Le contrat anti-régression a bougé → le tableau d'avancement doit suivre.
if a '^tests/ux/_contrat\.ts$' && ! a '^docs/generated/ux\.md$'; then
  ajouter "Le contrat anti-régression a changé sans que \`docs/generated/ux.md\` suive : lance \`npm run gen\`."
fi

# 4. Une zone jamais déléguée a été touchée → le dire, explicitement.
zones=$(grep -v '^\s*#' tools/zones-interdites.txt 2>/dev/null | grep -v '^\s*$' | cut -d'|' -f2 | tr -s ' ' '\n' | grep -v '^$' | sed 's/\*\*$//;s/\*$//')
touchees=""
for z in $zones; do
  printf '%s\n' "$change" | grep -q "^${z}" && touchees="$touchees $z"
done
if [ -n "$touchees" ]; then
  ajouter "Le diff touche une zone qui ne se délègue jamais ($(echo "$touchees" | tr -s ' ')). Confirme à Thomas que ce travail n'a pas été délégué à un modèle moins coûteux — aucun test vert n'y prouve rien (ADR 0005)."
fi

# 5. Un correctif de bug → le test qui échouait avant lui.
sujets=$(git log --format=%s "$base"..HEAD 2>/dev/null)
if printf '%s' "$sujets" | grep -qiE '^(fix|correctif|corrige)' && ! a '^(tests/|src-tauri/src/.*test)'; then
  ajouter "Un commit annonce un correctif, mais aucun test n'a bougé. **Tout défaut qui atteint Thomas est un trou dans le harnais** : le correctif n'est complet qu'accompagné de la vérification qui l'aurait vu (ADR 0005)."
fi

# 6. Du code applicatif a changé → la question de V1, celle qui compte le plus.
code=$(printf '%s\n' "$change" | grep -E '^(src|src-tauri/src)/' | grep -v 'test')
if [ -n "$code" ] && ! printf '%s\n' "$change" | grep -qx '.claude/skills/hean-invariants/SKILL.md'; then
  ajouter "Du code applicatif a changé sans que le fichier d'invariants bouge. Si ce travail corrige un bug qui pourrait revenir, ou établit une règle à respecter à l'avenir : propose à Thomas, en une phrase simple, la règle à ajouter (quoi + pourquoi) et sa gravité, et demande-lui seulement « oui / non »."
fi

[ -z "$demandes" ] && exit 0

# Une seule demande par ensemble de fichiers : le bruit tue la lecture.
etat="$(git rev-parse --git-dir)/hean-contrat-de-fin-de-tache"
empreinte=$(printf '%s%s' "$change" "$demandes" | shasum | cut -d' ' -f1)
grep -qx "$empreinte" "$etat" 2>/dev/null && exit 0
echo "$empreinte" >> "$etat"

raison="Contrat de fin de tâche — il manque ceci avant de terminer :
${demandes}

PORTE DE SORTIE : chacun de ces points peut être écarté. Si l'un ne s'applique pas (refacto,
documentation, feature sans risque de régression), dis-le à Thomas en une phrase avec la
raison, et termine. « Non, pas de règle ici » est une réponse valable — mais il faut la
donner. Ne relance pas ce rappel : il ne se redéclenchera pas pour ces mêmes fichiers."

jq -n --arg r "$raison" '{decision: "block", reason: $r}'
