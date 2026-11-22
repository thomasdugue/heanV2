#!/usr/bin/env python3
"""Produit les trois fixtures de fichier abîmé, à partir d'un FLAC sain.

Les trois cas ne se comportent PAS de la même façon face à symphonia. Ce n'est pas une
supposition : c'est mesuré, et chaque comportement est fixé par un test dans
`src-tauri/tests/decodage.rs`.

  corrupt-entete.flac   le bloc STREAMINFO est retourné bit à bit. Symphonia REFUSE le
                        fichier dès la sonde, avec un message. C'est le SEUL des trois qui
                        produise une erreur.
  corrupt-trames.flac   400 octets retournés dans les trames audio. Symphonia ne signale
                        RIEN : il saute les trames invalides et continue. Mesuré : un seul
                        octet abîmé suffit à faire disparaître un bloc entier — 4 608
                        trames, soit un dixième de seconde — en silence.
  corrupt-tronque.flac  coupé au tiers — une copie interrompue depuis le NAS. Aucune erreur
                        non plus : le fichier joue plus court, et rien ne le signale.

Conséquence de conception, et c'est pour cela que ces fixtures existent : **l'intégrité d'un
fichier audio ne peut PAS être déduite du décodeur.** Il faudra la vérifier autrement — la
durée annoncée par le conteneur contre le nombre de trames réellement rendues — le jour où
la lecture depuis le NAS arrivera (phase 5).

Appelé par make-fixtures-audio.sh. Script one-shot : les fixtures sont commitées.
"""
import sys


def debut_des_trames(flac: bytes) -> int:
    """L'offset de la première trame audio, en sautant les blocs de métadonnées.

    Calculé et non supposé : une première version visait l'octet 4 000, qui tombait dans le
    bloc PADDING de 8 192 octets qu'ffmpeg écrit par défaut. La fixture n'abîmait alors
    rien du tout, et le test « passait » en ne prouvant rien.
    """
    if flac[:4] != b"fLaC":
        raise SystemExit("la source n'est pas un FLAC")
    offset = 4
    while True:
        entete = flac[offset]
        taille = int.from_bytes(flac[offset + 1 : offset + 4], "big")
        offset += 4 + taille
        if entete & 0x80:  # bit « dernier bloc de métadonnées »
            return offset


source, entete, trames, tronque = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
sain = open(source, "rb").read()
audio = debut_des_trames(sain)

# Le bloc STREAMINFO suit la signature « fLaC » et son en-tête de bloc : octets 8 à 42.
# L'abîmer rend les longueurs de trame incohérentes, ce que la sonde refuse.
d = bytearray(sain)
for i in range(8, 42):
    d[i] ^= 0xFF
open(entete, "wb").write(bytes(d))

# Au milieu des trames : le format reste reconnaissable, seul l'audio est faux.
d = bytearray(sain)
milieu = audio + (len(sain) - audio) // 2
for i in range(milieu, milieu + 400):
    d[i] ^= 0xFF
open(trames, "wb").write(bytes(d))

open(tronque, "wb").write(sain[: len(sain) // 3])
