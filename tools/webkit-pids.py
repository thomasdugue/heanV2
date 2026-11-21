#!/usr/bin/env python3
"""Imprime les PID des processus WebKit nés APRÈS le processus donné.

Les processus WebKit d'une WKWebView sont des services XPC rattachés à launchd, pas des
enfants de l'app : on ne peut pas les retrouver par le ppid. Le filtre retenu est l'âge —
un processus WebKit démarré avant l'app n'est pas le sien. Imparfait si une autre app
WebKit démarre pendant la mesure, mais c'est la meilleure approximation sans API privée.
"""
import re, subprocess, sys

def elapsed(e):
    m = re.match(r'(?:(\d+)-)?(?:(\d+):)?(\d+):(\d+)$', e.strip())
    if not m: return None
    d, h, mi, s = m.groups()
    return int(d or 0) * 86400 + int(h or 0) * 3600 + int(mi or 0) * 60 + int(s)

main = int(sys.argv[1])
rows = subprocess.run(["ps", "-axo", "pid,etime,comm"], capture_output=True, text=True).stdout
ages, out = {}, []
for line in rows.splitlines()[1:]:
    parts = line.split(None, 2)
    if len(parts) < 3: continue
    pid, et, comm = parts
    try: pid = int(pid)
    except ValueError: continue
    ages[pid] = (elapsed(et), comm)

ref = ages.get(main, (None, ""))[0]
if ref is None: sys.exit(0)
for pid, (age, comm) in ages.items():
    if age is None: continue
    if re.search(r'WebKit\.(WebContent|GPU|Networking)', comm) and age <= ref:
        out.append(str(pid))
print(" ".join(out))
