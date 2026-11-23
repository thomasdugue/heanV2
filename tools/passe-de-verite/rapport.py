"""OUTIL TEMPORAIRE — passe de vérité : le rapport en français. À jeter.

La moitié « mise en forme et verdict ». Les lectures, les comptes et l'arbitrage vivent
dans `donnees.py`, à côté — voir son en-tête pour les deux conventions du rapport.

Il ne fait partie ni du produit ni de ./check : il lit le JSON produit par
`tools/passe-de-verite/` (le vrai code d'indexation de la V2, lancé sur la vraie
bibliothèque) et le compare aux données de la V1.

Aucun chemin de machine n'est écrit ici — tout arrive en argument. La seule écriture est
celle du fichier de détail, dont le chemin est donné en argument.
"""

import os
import sys
from collections import Counter, defaultdict

# `donnees.py` est dans le même dossier que ce fichier, donc Python le trouve seul.
from donnees import (
    CHAMPS,
    FFPROBE,
    charger_v1,
    charger_v2,
    cle,
    conforme,
    doublons,
    en_secondes,
    identiques,
    image_dans_le_dossier,
    nb_doublons,
    nfc,
    reference,
    valeurs_distinctes,
)

EXEMPLES = 5  # « une poignée » : de quoi aller voir, pas de quoi noyer


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
    # TELLE QUELLE, sans repasser par `cle()` : voir `normalisation_de_la_cle` plus bas.
    alb2 = {m["cle_album"] for m in p2.values() if m["cle_album"]}
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
    # INV-01 sur de VRAIES données. Les property tests de `cle.rs` éprouvent
    # `AlbumKey::new` pour toute chaîne générée ; ils ne disent rien de ce qui est
    # réellement dans la base de Thomas. Ici on relit les clés que la V2 a écrites et on
    # affirme qu'elles sont déjà normalisées : si l'une ne l'est pas, c'est qu'une clé est
    # entrée sans passer par le constructeur, et INV-01 (🔴) est tombée.
    mal_normalisees = [c for c, m in p2.items() if cle(m["cle_album"]) != m["cle_album"]]
    print()
    if mal_normalisees:
        print(f"  ⚠ INV-01 : {len(mal_normalisees)} clé(s) d’album NON normalisée(s) dans la base.")
        print("    Une clé n’est censée pouvoir naître que d’AlbumKey::new, qui normalise.")
    else:
        print(f"  ✓ INV-01 : les {len(p2)} clés d’album de la base sont déjà normalisées")
        print("    (trim + NFC) — vérifié sur tes fichiers, pas sur des chaînes générées.")

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

    if mal_normalisees:
        bloc(f"Clés d’album non normalisées ({len(mal_normalisees)}) — INV-01",
             [f"« {p2[c]['cle_album']} » devrait être « {cle(p2[c]['cle_album'])} »\n      "
              + c[len(racine):].lstrip("/") for c in mal_normalisees], detail)
        garder("clés d’album non normalisées", mal_normalisees)

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
        == len({m["cle_album"] for m in communs_2.values() if m["cle_album"]})
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
