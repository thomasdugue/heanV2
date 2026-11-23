"""OUTIL TEMPORAIRE — passe de vérité : lire les deux côtés, et les arbitrer. À jeter.

La moitié « données » du rapport : charger ce que la V2 a indexé et ce que la V1 avait
enregistré, compter les six grandeurs que l'on compare, et **arbitrer** chaque écart en
demandant à un outil tiers ce que le fichier contient vraiment. La mise en forme et le
rapport lui-même sont dans `rapport.py`, qui importe ce module.

Le découpage n'est pas cosmétique. Ce fichier et son voisin faisaient 590 lignes à eux deux,
donc au-delà de la limite de ~400 du dépôt — et ils y échappaient seulement parce que
`tools/check-sizes.sh` ne lit pas le `.py`. Relevé par `invariant-guardian` : une règle n'est
tenue que là où le garde-fou regarde, et ce qui n'est pas acceptable est que le dépassement
soit l'accident d'une extension oubliée plutôt qu'une décision.

LECTURE SEULE sur les données de la V1, sans exception : les fichiers sont ouverts en 'r',
et rien ici n'écrit quoi que ce soit.

Deux conventions qui tiennent tout le rapport :

  - « clé » veut dire trim + NFC, c'est-à-dire exactement ce que fait `AlbumKey::new`
    côté V2 (src-tauri/src/library/cle.rs). La même normalisation est appliquée aux
    chaînes de la V1, sinon on mesurerait la normalisation au lieu de mesurer la parité.
    Elle n'est **pas** appliquée à la clé que la V2 a écrite : la renormaliser réparerait
    en silence un défaut d'INV-01, et `rapport.py` l'affirme au lieu de la corriger.
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
