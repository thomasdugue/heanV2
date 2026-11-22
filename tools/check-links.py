#!/usr/bin/env python3
"""Vérifie que tous les liens relatifs des fichiers markdown résolvent."""
import os, re, sys

root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
bad = []
for dirpath, dirnames, filenames in os.walk(root):
    # Un dossier qui contient une entrée `.git` est un dépôt à part (clone imbriqué, ou
    # worktree de session sous `.claude/worktrees/`). Ses liens sont ceux d'une AUTRE
    # branche : les valider ici rendrait ce script rouge en local et vert en CI, qui part
    # d'un clone neuf. Même famille de défaut que la chaîne d'outils non épinglée (ADR 0007).
    dirnames[:] = [d for d in dirnames
                   if d not in {'.git', 'node_modules', 'target'}
                   and not os.path.exists(os.path.join(dirpath, d, '.git'))]
    for fn in filenames:
        if not fn.endswith('.md'):
            continue
        path = os.path.join(dirpath, fn)
        text = open(path, encoding='utf-8', errors='replace').read()
        for m in re.finditer(r'\]\(([^)#][^)]*)\)', text):
            target = m.group(1).split('#')[0].strip()
            if not target or target.startswith(('http://', 'https://', 'mailto:')):
                continue
            if not os.path.exists(os.path.normpath(os.path.join(dirpath, target))):
                bad.append((os.path.relpath(path, root), target))

for f, t in bad:
    print(f"  lien cassé : {f} -> {t}")
print(f"check-links : {len(bad)} lien(s) cassé(s)")
sys.exit(1 if bad else 0)
