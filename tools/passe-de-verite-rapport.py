#!/usr/bin/env python3
"""OUTIL TEMPORAIRE — la moitié « comparaison » de la passe de vérité. À jeter.

Il ne fait partie du produit ni de ./check : il lit le JSON qu'a produit
`tools/passe-de-verite/` (le vrai code d'indexation de la V2, lancé sur la vraie
bibliothèque) et le compare aux données de la V1, puis écrit un rapport en français.

LECTURE SEULE sur les données de la V1, sans exception : ce script ouvre ses fichiers en
'r' et n'écrit qu'à l'endroit qu'on lui donne en argument. Aucun chemin de machine n'est
écrit ici — tout arrive en argument.

Deux conventions qui tiennent tout le rapport :

  - « clé » veut dire trim + NFC, c'est-à-dire exactement ce que fait `AlbumKey::new`
    côté V2 (src-tauri/src/library/cle.rs). La même normalisation est appliquée aux
    chaînes de la V1, sinon on mesurerait la normalisation au lieu de mesurer la parité.
  - une mesure que la V2 ne fait pas encore (les pochettes) est annoncée comme une mesure
    DE L'OUTIL. On ne présente jamais un chiffre de l'outil comme un chiffre de la V2.
"""

import json
import os
import shutil
import subprocess
import sys
import unicodedata
from collections import Counter, defaultdict

EXEMPLES = 5  # « une poignée » : de quoi aller voir, pas de quoi noyer


def nfc(s):
    return unicodedata.normalize("NFC", s or "")


def cle(s):
    """trim + NFC — la normalisation d'`AlbumKey::new`, appliquée des deux côtés."""
    return nfc((s or "").strip())


def lire_json(chemin):
    with open(chemin, "r", encoding="utf-8") as f:
        return json.load(f)


# --- chargement ----------------------------------------------------------------------


def charger_v2(chemin):
    d = lire_json(chemin)
    pistes = {}
    for p in d["pistes"]:
        pistes[nfc(p["path"])] = p
    return {
        "racine": d["racine"],
        "secondes": d["secondes"],
        "rapport": d["rapport"],
        "trouvees": [nfc(c) for c in d["trouvees"]],
        "pistes": pistes,
        "illisibles": [nfc(c) for c in d["illisibles"]],
        "pochette": {nfc(c) for c in d["pochette_integree"]},
    }


def charger_v1(dossier, racine):
    """Les pistes de la V1 qui sont sous la racine scannée, plus son cache de pochettes."""
    racine = nfc(racine).rstrip("/") + "/"
    pistes = {}
    hors_racine = 0
    for t in lire_json(os.path.join(dossier, "tracks_cache.json"))["tracks"]:
        chemin = nfc(t["path"])
        if not chemin.startswith(racine):
            hors_racine += 1
            continue
        pistes[chemin] = t["metadata"]
    pochettes = set()
    chemin_couvertures = os.path.join(dossier, "cover_cache.json")
    if os.path.exists(chemin_couvertures):
        for c, jpg in lire_json(chemin_couvertures)["entries"].items():
            if jpg:
                pochettes.add(nfc(c))
    return {"pistes": pistes, "pochettes": pochettes, "hors_racine": hors_racine}


# --- les six comptes -----------------------------------------------------------------


def valeurs_distinctes(pistes, champ):
    return {cle(m.get(champ)) for m in pistes.values() if cle(m.get(champ))}


def doublons(pistes, titre, artiste, duree_s):
    """Groupes de pistes que la MÊME règle désigne comme un même morceau, des deux côtés.

    Ni la V1 ni la V2 ne stockent de drapeau « doublon » : la V1 le calculait à la volée,
    et la V2 a laissé la question ouverte pour la phase 3. Comparer deux chiffres venus de
    deux règles différentes ne dirait rien — donc l'outil applique SA règle, la même aux
    deux jeux de données, et l'annonce.

    La règle : même artiste, même titre (casse ignorée) et même durée à la seconde.
    """
    groupes = defaultdict(list)
    for chemin, m in pistes.items():
        d = duree_s(m)
        if d is None:
            continue
        k = (cle(artiste(m)).lower(), cle(titre(m)).lower(), round(d))
        if not k[1]:
            continue
        groupes[k].append(chemin)
    return {k: v for k, v in groupes.items() if len(v) > 1}


def nb_doublons(groupes):
    """Le nombre de fichiers EN TROP : un groupe de 3 compte 2 doublons."""
    return sum(len(v) - 1 for v in groupes.values())


def image_dans_le_dossier(chemin, cache={}):
    """Une image posée à côté du fichier (cover.jpg, folder.png…), comme la V1 en cherchait."""
    dossier = os.path.dirname(chemin)
    if dossier not in cache:
        try:
            noms = os.listdir(dossier)
        except OSError:
            noms = []
        cache[dossier] = any(
            n.lower().endswith((".jpg", ".jpeg", ".png", ".webp")) for n in noms
        )
    return cache[dossier]


# --- comparaison champ par champ -----------------------------------------------------

# V1 et V2 ne nomment pas les mêmes choses pareil. Chaque ligne : nom lisible, champ V1,
# champ V2, et comment on décide que deux valeurs disent la même chose.
CHAMPS = [
    ("titre", "title", "titre", "texte"),
    ("artiste", "artist", "artiste", "texte"),
    ("album", "album", "album", "texte"),
    ("genre", "genre", "genre", "texte"),
    ("année", "year", "annee", "nombre"),
    ("n° de piste", "track", "piste", "nombre"),
    ("n° de disque", "disc", "disque", "nombre"),
    ("durée", "duration", "duree_ms", "duree"),
    ("échantillonnage", "sampleRate", "echantillonnage", "nombre"),
    ("profondeur (bits)", "bitDepth", "bits", "nombre"),
    ("format", "codec", "format", "casse"),
    ("débit", "bitrate", "debit", "debit"),
]


def identiques(mode, v1, v2):
    if mode == "texte":
        return cle(v1) == cle(v2)
    if mode == "casse":
        return cle(v1).lower() == cle(v2).lower()
    if mode == "nombre":
        return (v1 or None) == (v2 or None)
    if mode == "duree":
        # V1 en secondes flottantes, V2 en millisecondes. Une seconde de tolérance : les
        # deux arrondissent, et un écart d'une seconde n'est pas un bug de bibliothèque.
        if v1 is None or v2 is None:
            return v1 == v2
        return abs(float(v1) - (v2 / 1000.0)) <= 1.0
    if mode == "debit":
        # Le débit est un calcul, pas un tag : la V1 et lofty ne l'arrondissent pas
        # pareil. 2 % de tolérance, et au-delà c'est un vrai écart.
        if not v1 or not v2:
            return (v1 or None) == (v2 or None)
        return abs(v1 - v2) <= max(2, 0.02 * max(v1, v2))
    raise ValueError(mode)


# --- l'arbitre : un outil tiers tranche --------------------------------------------
#
# Sans arbitre, « la V1 dit Rap, la V2 dit Rap/Hip Hop » n'apprend rien : on ne sait pas
# laquelle se trompe. C'est le même raisonnement que `src-tauri/tests/bit_perfect.rs`, qui
# ne compare pas notre sortie à elle-même mais à un fichier écrit par ffmpeg : on demande à
# un décodeur indépendant ce que le fichier contient vraiment, et on attribue l'écart.
#
# Si ffprobe est absent, le rapport reste descriptif et le dit — il ne devine pas.

FFPROBE = shutil.which("ffprobe")


def reference(chemin, cache={}):
    """Ce qu'un tiers (ffprobe) lit dans le fichier. `None` s'il ne sait pas le lire."""
    if chemin in cache:
        return cache[chemin]
    valeur = None
    if FFPROBE:
        try:
            r = subprocess.run(
                [FFPROBE, "-v", "error", "-print_format", "json",
                 "-show_format", "-show_streams", chemin],
                capture_output=True, text=True, timeout=30,
            )
            d = json.loads(r.stdout) if r.stdout.strip() else {}
            tags = {k.lower(): v for k, v in (d.get("format", {}).get("tags") or {}).items()}
            flux = next((f for f in d.get("streams", []) if f.get("codec_type") == "audio"), {})
            for k, v in (flux.get("tags") or {}).items():
                tags.setdefault(k.lower(), v)
            valeur = {
                "titre": tags.get("title"),
                "artiste": tags.get("artist"),
                "album": tags.get("album"),
                "genre": tags.get("genre"),
                "année": entier(tags.get("date") or tags.get("year")),
                "n° de piste": entier(tags.get("track")),
                "n° de disque": entier(tags.get("disc") or tags.get("discnumber")),
                "durée": flottant(d.get("format", {}).get("duration")),
                "échantillonnage": entier(flux.get("sample_rate")),
                "profondeur (bits)": entier(flux.get("bits_per_raw_sample")
                                            or flux.get("bits_per_sample")),
                "format": flux.get("codec_name"),
            }
        except Exception:
            valeur = None
    cache[chemin] = valeur
    return valeur


def entier(v):
    """« 2/12 » → 2, « 2016-05-01 » → 2016, « » → None. Les tags sont du texte libre."""
    if v in (None, "", 0):
        return None
    t = str(v).split("/")[0].split("-")[0].strip()
    try:
        n = int(t)
    except ValueError:
        return None
    return n or None


def flottant(v):
    try:
        return float(v)
    except (TypeError, ValueError):
        return None


def conforme(mode, ref, val):
    """La valeur dit-elle la même chose que la référence du tiers ?

    Pour un texte, la référence peut être MULTIPLE : un tag Vorbis ou ID3 sépare ses
    valeurs par des points-virgules (« Rap/Hip Hop;Rap français »). Une valeur qui est
    l'une d'elles est conforme ; un fragment (« Rap ») ne l'est pas.
    """
    if mode in ("texte", "casse"):
        attendues = {cle(x) for x in str(ref).split(";") if cle(x)}
        if mode == "casse":
            return cle(val).lower() in {a.lower() for a in attendues}
        return cle(val) in attendues
    if mode == "nombre":
        return (val or None) == (ref or None)
    if mode == "duree":
        return val is not None and abs(val - ref) <= 1.0
    return None


def en_secondes(nom, v1, v2):
    """Ramène les deux côtés à la même unité : la V1 compte en secondes, la V2 en ms."""
    if nom != "durée":
        return v1, v2
    return (flottant(v1), (v2 / 1000.0) if v2 else None)


# --- mise en forme -------------------------------------------------------------------


def taille(chemin):
    try:
        n = os.path.getsize(chemin)
    except OSError:
        return " (fichier absent du disque)"
    if n == 0:
        return "  ← FICHIER DE 0 OCTET"
    return ""


def etiquette(chemin, m=None, racine=""):
    court = chemin[len(racine):].lstrip("/") if chemin.startswith(racine) else chemin
    court += taille(chemin)
    if m:
        titre = cle(m.get("title") or m.get("titre")) or "(sans titre)"
        album = cle(m.get("album")) or "(sans album)"
        return f"« {titre} » — {album}\n      {court}"
    return f"  {court}"


def bloc(titre, lignes, detail, limite=EXEMPLES):
    """Affiche une poignée d'exemples à l'écran, et TOUT dans le fichier de détail."""
    print(f"\n  {titre}")
    if not lignes:
        print("    (rien)")
    for ligne in lignes[:limite]:
        print(f"    • {ligne}")
    if len(lignes) > limite:
        print(f"    … et {len(lignes) - limite} autres — tout est dans {detail}")


def tableau(lignes):
    largeur = max(len(n) for n, *_ in lignes)
    print(f"  {'Ce qu’on compte'.ljust(largeur)}  {'V1':>8} {'V2':>8} {'écart':>8}")
    print("  " + "─" * (largeur + 28))
    for nom, a, b, note in lignes:
        if a is None or b is None:
            ecart = "—"
            marque = "·"
        else:
            ecart = f"{b - a:+d}" if b != a else "0"
            marque = "✓" if a == b else "⚠"
        a_txt = "—" if a is None else str(a)
        b_txt = "—" if b is None else str(b)
        print(f"  {nom.ljust(largeur)}  {a_txt:>8} {b_txt:>8} {ecart:>8}  {marque} {note}")


# --- le rapport ----------------------------------------------------------------------


def main():
    if len(sys.argv) != 4:
        sys.exit("usage : passe-de-verite-rapport.py <v2.json> <dossier-donnees-v1> <detail.txt>")
    chemin_v2, dossier_v1, detail = sys.argv[1:4]

    v2 = charger_v2(chemin_v2)
    racine = nfc(v2["racine"]).rstrip("/")
    v1 = charger_v1(dossier_v1, racine)

    p1, p2 = v1["pistes"], v2["pistes"]
    ens1, ens2 = set(p1), set(p2)
    communs = sorted(ens1 & ens2)

    print("╭──────────────────────────────────────────────────────────────────────────╮")
    print("│  PASSE DE VÉRITÉ — la bibliothèque V2 sur tes vrais fichiers             │")
    print("╰──────────────────────────────────────────────────────────────────────────╯")
    print()
    print(f"  Dossier scanné  : {racine}")
    print(f"  Données V1 lues : {dossier_v1}  (en lecture seule, rien n’a été modifié)")
    print(f"  Scan V2         : {v2['secondes']:.1f} s pour {len(v2['trouvees'])} fichiers")
    if v1["hors_racine"]:
        print(f"  Mises de côté   : {v1['hors_racine']} pistes V1 hors de ce dossier (le NAS)")

    # --- les six comptes ---
    alb1 = valeurs_distinctes(p1, "album")
    alb2 = {cle(m["cle_album"]) for m in p2.values() if cle(m["cle_album"])}
    art1 = valeurs_distinctes(p1, "artist")
    art2 = valeurs_distinctes(p2, "artiste")

    d1 = doublons(p1, lambda m: m.get("title"), lambda m: m.get("artist"),
                  lambda m: m.get("duration"))
    d2 = doublons(p2, lambda m: m.get("titre"), lambda m: m.get("artiste"),
                  lambda m: (m["duree_ms"] / 1000.0) if m.get("duree_ms") else None)

    # Le cache de pochettes de la V1 n'est PAS la liste des pistes qui ont une pochette :
    # il ne contient que celles rapatriées d'internet (15 entrées locales sur 1828 chez
    # Thomas). Le comparer à une mesure du disque donnerait un écart énorme et faux — et
    # un chiffre faux dans un rapport de vérité est pire que pas de chiffre. Donc la
    # colonne V1 reste vide, et on dit pourquoi.
    locales_v1 = {c for c in v1["pochettes"] if c in ens1}
    note_pochette = (
        f"la V1 n’en comptait pas ({len(locales_v1)} entrées dans son cache internet)"
    )
    sans_poch2 = sorted(
        c for c in ens2 if c not in v2["pochette"] and not image_dans_le_dossier(c)
    )

    print()
    tableau([
        ("Titres indexés", len(p1), len(p2), ""),
        ("Albums (clé trim+NFC)", len(alb1), len(alb2), ""),
        ("Artistes", len(art1), len(art2), ""),
        ("Doublons (règle de l’outil)", nb_doublons(d1), nb_doublons(d2), ""),
        ("Sans pochette (mesure outil)", None, len(sans_poch2), note_pochette),
        ("Tags illisibles", None, v2["rapport"]["illisibles"], "la V1 ne comptait pas ça"),
    ])

    lignes_detail = []

    def garder(titre, lignes):
        lignes_detail.append(f"\n=== {titre} ({len(lignes)}) ===")
        lignes_detail.extend(f"  {l}" for l in lignes)

    # --- les écarts, un par un ---
    print()
    print("  ── Les écarts, avec de quoi aller voir ──────────────────────────────────")

    manque_v2 = sorted(ens1 - ens2)
    nouveau_v2 = sorted(ens2 - ens1)
    if manque_v2 or nouveau_v2:
        if manque_v2:
            bloc(f"Dans la V1 mais PAS dans la V2 ({len(manque_v2)})",
                 [etiquette(c, p1[c], racine) for c in manque_v2], detail)
            garder("dans la V1 mais pas dans la V2", manque_v2)
        if nouveau_v2:
            bloc(f"Dans la V2 mais PAS dans la V1 ({len(nouveau_v2)})",
                 [etiquette(c, p2[c], racine) for c in nouveau_v2], detail)
            garder("dans la V2 mais pas dans la V1", nouveau_v2)
    else:
        print("\n  Les deux versions indexent EXACTEMENT les mêmes fichiers.")

    if v2["illisibles"]:
        bloc(f"Fichiers que la V2 n’a pas su lire ({len(v2['illisibles'])})",
             [etiquette(c, None, racine).strip() for c in v2["illisibles"]], detail)
        garder("illisibles pour la V2", v2["illisibles"])

    if alb1 != alb2:
        seuls1, seuls2 = sorted(alb1 - alb2), sorted(alb2 - alb1)
        if seuls1:
            bloc(f"Albums vus par la V1 seule ({len(seuls1)})", seuls1, detail)
            garder("albums V1 seule", seuls1)
        if seuls2:
            bloc(f"Albums vus par la V2 seule ({len(seuls2)})", seuls2, detail)
            garder("albums V2 seule", seuls2)

    if art1 != art2:
        seuls1, seuls2 = sorted(art1 - art2), sorted(art2 - art1)
        if seuls1:
            bloc(f"Artistes vus par la V1 seule ({len(seuls1)})", seuls1, detail)
            garder("artistes V1 seule", seuls1)
        if seuls2:
            bloc(f"Artistes vus par la V2 seule ({len(seuls2)})", seuls2, detail)
            garder("artistes V2 seule", seuls2)

    if nb_doublons(d1) != nb_doublons(d2):
        seuls = sorted(set(d1) ^ set(d2))
        bloc(f"Groupes de doublons vus d’un seul côté ({len(seuls)})",
             [f"{a} — « {t} » ({s} s) : V1 {len(d1.get(k, []))} / V2 {len(d2.get(k, []))}"
              for k in seuls for a, t, s in [k]], detail)
        garder("groupes de doublons divergents", [str(k) for k in seuls])
    elif d2:
        bloc(f"Doublons, les mêmes des deux côtés — un échantillon ({len(d2)} groupes)",
             [f"« {t} » de {a} ({s} s) × {len(v)}"
              for (a, t, s), v in sorted(d2.items(), key=lambda x: -len(x[1]))], detail, 3)
        garder("tous les groupes de doublons",
               [f"{k} -> {chemins}" for k, chemins in sorted(d2.items())])

    if sans_poch2:
        bloc(f"Ni pochette intégrée, ni image dans le dossier ({len(sans_poch2)}) — "
             "mesure de l’outil, la V2 n’indexe pas encore les pochettes",
             [etiquette(c, p2.get(c), racine) for c in sans_poch2], detail)
        garder("sans pochette (mesure de l’outil)", sans_poch2)

    # --- champ par champ, sur les pistes présentes des deux côtés ---
    print()
    print("  ── Champ par champ, sur les "
          f"{len(communs)} pistes présentes des deux côtés ───────")
    print()
    ecarts = Counter()
    verdicts = defaultdict(Counter)  # champ -> {« V2 » : n, « V1 » : n, « aucun » : n}
    exemples = defaultdict(list)
    for chemin in communs:
        a, b = p1[chemin], p2[chemin]
        for nom, c1, c2, mode in CHAMPS:
            if identiques(mode, a.get(c1), b.get(c2)):
                continue
            ecarts[nom] += 1
            # On demande au tiers ce que le fichier contient VRAIMENT, et on attribue.
            ref = reference(chemin)
            x1, x2 = en_secondes(nom, a.get(c1), b.get(c2))
            attendu = (ref or {}).get(nom)
            qui = "sans arbitre"
            # La V1 complétait les genres manquants depuis internet, et le notait
            # (`genre_enriched`). Un fichier sans tag genre que la V1 affichait quand même
            # n'est pas une lecture fautive de la V2 : c'est une feature pas encore portée.
            # Le drapeau est dans les données de la V1 — autant le lire que le deviner.
            if nom == "genre" and a.get("genre_enriched") and not attendu:
                verdicts[nom]["genre enrichi par la V1 (feature non portée)"] += 1
                exemples[nom].append(
                    f"V1 « {a.get(c1)} »  ≠  V2 « {b.get(c2)} »   "
                    "[la V1 l’avait complété depuis internet, le fichier n’a pas de genre]"
                    f"\n      {chemin[len(racine):].lstrip('/')}"
                )
                continue
            if attendu not in (None, ""):
                ok1 = conforme(mode, attendu, x1)
                ok2 = conforme(mode, attendu, x2)
                if ok1 is None or ok2 is None:
                    qui = "sans arbitre"
                elif ok2 and not ok1:
                    qui = "V2 a raison"
                elif ok1 and not ok2:
                    qui = "V1 avait raison"
                elif ok1 and ok2:
                    qui = "les deux passent"
                else:
                    qui = "aucun des deux"
            verdicts[nom][qui] += 1
            exemples[nom].append(
                f"V1 « {a.get(c1)} »  ≠  V2 « {b.get(c2)} »   [{qui}"
                + (f", le fichier dit « {attendu} »]" if attendu not in (None, "") else "]")
                + f"\n      {chemin[len(racine):].lstrip('/')}"
            )
    if not FFPROBE:
        print("  (ffprobe absent : les écarts sont décrits, pas arbitrés)")
        print()
    largeur = max(len(n) for n, *_ in CHAMPS)
    for nom, *_ in CHAMPS:
        n = ecarts[nom]
        if n == 0:
            print(f"  {nom.ljust(largeur)}  {0:>5} écart(s)  ✓")
            continue
        detail_v = ", ".join(f"{q} ×{c}" for q, c in verdicts[nom].most_common())
        print(f"  {nom.ljust(largeur)}  {n:>5} écart(s)  ⚠  {detail_v}")
    for nom, *_ in CHAMPS:
        if ecarts[nom]:
            bloc(f"{nom} — {ecarts[nom]} écart(s)", exemples[nom], detail)
            garder(f"champ {nom}", [e.replace("\n      ", " | ") for e in exemples[nom]])

    # --- verdict ---
    # Les comptes d'albums, d'artistes et de doublons DÉPENDENT des pistes indexées : un
    # seul fichier qui manque d'un côté fait bouger trois lignes du tableau. On recompte
    # donc tout sur les seules pistes communes : si l'écart disparaît, c'est que la cause
    # unique est la liste des fichiers, et non la façon de les regrouper.
    communs_1 = {c: p1[c] for c in communs}
    communs_2 = {c: p2[c] for c in communs}
    cascade = (
        len(valeurs_distinctes(communs_1, "album"))
        == len({cle(m["cle_album"]) for m in communs_2.values() if cle(m["cle_album"])})
        and len(valeurs_distinctes(communs_1, "artist"))
        == len(valeurs_distinctes(communs_2, "artiste"))
    )

    comptes = (len(manque_v2) + len(nouveau_v2)
               + abs(len(alb1) - len(alb2)) + abs(len(art1) - len(art2))
               + abs(nb_doublons(d1) - nb_doublons(d2)))
    cumul = Counter()
    for nom in verdicts:
        cumul.update(verdicts[nom])
    print()
    print("  ── Verdict ──────────────────────────────────────────────────────────────")
    print()
    if comptes == 0:
        print("  COMPTES : justes. La V2 indexe les mêmes fichiers que la V1, et en tire")
        print("  le même nombre d’albums, d’artistes et de doublons.")
    elif cascade:
        n = len(manque_v2) + len(nouveau_v2)
        print(f"  COMPTES : justes, à {n} fichier(s) près — et les écarts d’albums et")
        print("  d’artistes en découlent tous. Sur les pistes que les deux versions")
        print("  indexent, la V2 trouve exactement les mêmes albums et les mêmes artistes")
        print("  que la V1. Le tableau bouge parce que la liste de fichiers diffère, pas")
        print("  parce que la V2 regrouperait autrement.")
    else:
        print(f"  COMPTES : {comptes} divergence(s) — et elles ne viennent pas seulement")
        print("  des fichiers listés ci-dessus : à liste de pistes égale, la V2 ne trouve")
        print("  pas le même nombre d’albums ou d’artistes que la V1.")
    print()
    if not sum(ecarts.values()):
        print("  MÉTADONNÉES : aucune divergence sur les pistes communes.")
    else:
        print(f"  MÉTADONNÉES : {sum(ecarts.values())} écart(s) sur les pistes communes,")
        print("  arbitrés en lisant les fichiers avec ffprobe :")
        for q, c in cumul.most_common():
            print(f"    · {q} : {c}")
        print()
        print("  « V2 a raison » veut dire que la V1 affichait autre chose que le fichier.")
        print("  « V1 avait raison » est la seule catégorie qui demande une correction.")
    print()
    print(f"  Détail complet (toutes les lignes, pas seulement {EXEMPLES}) : {detail}")
    print()

    with open(detail, "w", encoding="utf-8") as f:
        f.write(f"passe de vérité — détail complet\nracine : {racine}\n")
        f.write("\n".join(lignes_detail) + "\n")


if __name__ == "__main__":
    main()
