# Protocole — le son sort-il, et sort-il juste ?

Ce que **toi seul** peux vérifier : le moteur de sortie parle à CoreAudio, donc à du
matériel, et la CI n'en a pas. Ce qui s'éprouve sans toi est déjà éprouvé (le tampon
circulaire, le refus de fréquence, la lecture des périphériques) ; tout ce qui suit demande
tes oreilles ou ton DAC.

Compte **vingt minutes**. Les étapes 1 à 9 se font sans rien brancher ; 10 à 14 demandent le
DAC. Si une étape échoue, note son numéro et passe à la suivante — aucune ne dépend de la
précédente, sauf le lancement.

## Lancement

⚠️ **Tant que la PR #25 n'est pas fusionnée, ce moteur n'est pas dans le dossier
principal** : il vit sur sa branche, dans un dossier à part. C'est celui-là qu'il faut
lancer. Lancer le dossier principal ouvrirait une version **sans moteur de son** — tout ce
protocole échouerait dès l'étape 2, sans avoir rien éprouvé de ce qu'il vise.

```bash
cd "/Users/tsunami25/Documents/Thomas/heanV2/.claude/worktrees/phase-2-bibliotheque-dce2d1" && npm run tauri dev
```

Après la fusion, le dossier principal porte le moteur et la ligne redevient la courte :

```bash
cd /Users/tsunami25/Documents/Thomas/heanV2 && npm run tauri dev
```

**« Port 1420 is already in use »** : une fenêtre de développement tourne déjà quelque part,
souvent oubliée d'une session précédente. Elle tient le port, et rien d'autre ne démarrera
avant qu'elle soit arrêtée — Ctrl-C dans son terminal. Pour savoir lequel :

```bash
lsof -nP -iTCP:1420 -sTCP:LISTEN
```

La fenêtre s'ouvre sur « Hean — coquille de phase 3 ». Elle est laide : c'est voulu, le
design est pour plus tard. La section **Lecture** est en haut, sous les dossiers et le NAS.

**Échec** : la fenêtre ne s'ouvre pas, ou la section Lecture est absente.

## Déjà passées sur ton matériel, les 2026-11-26 et 27

Relevé dans le journal de ta session ou jugé par Thomas lui-même, pas déduit d'un test. Tu
peux les sauter.

| | Ce que le journal montre |
|---|---|
| **2** — une piste locale joue | `piste ouverte` puis `sortie démarrée` en **52 ms** (`03 - ATLiens.flac`, 16 bits / 44,1 kHz) |
| **9** — le 192 kHz perd le badge | `repli de fréquence : l'AUHAL rééchantillonne` · `source_hz=192000 sortie_hz=96000` · `bit_perfect=false`, sur trois fichiers |
| **11** — changer de sortie | `sortie changée, défaut système synchronisé id=86`, et la piste réouverte à `depart_ms=6315` : la position est conservée |
| **14** — le matériel est rendu | la fréquence d'origine est remise à **chaque** fermeture de flux, et pas seulement en quittant l'app |
| **8** — l'album s'enchaîne sans trou | `piste enchaînée dans le même flux fichier=07 - Supernature.flac`, et **rien** derrière : ni fréquence réglée, ni flux rouvert, ni sortie redémarrée — le tampon n'a pas été touché. Jugé sans trou à l'oreille par Thomas |
| **3** — zéro trou | aucun avertissement sur **huit** fermetures de flux, rééchantillonnage compris. Reste à tenir trois minutes d'affilée sur une seule piste |
| **4** — pause et reprise | jugées instantanées par Thomas. Elles le sont par construction : la pause n'est qu'un drapeau que lit le callback, le tampon n'est pas jeté et le flux reste ouvert — donc le journal n'en dit rien, et il n'y a rien à y chercher |
| **7** — le volume | jugé immédiat par Thomas, même remarque : un atomique, et rien dans le journal |

**5 (déplacement) a échoué, et le défaut était dans la barre, pas dans le moteur.** Le
curseur se relisait dix fois par seconde depuis l'état sondé : chaque relevé réécrivait sa
valeur sous le doigt, donc viser était impossible. Corrigé — pendant un déplacement, c'est
le doigt qui commande, et le sondage reprend la main au relâchement. Le curseur de volume
avait la même forme sans le défaut, parce que la valeur qu'il relit est celle qu'on vient
d'y poser : elle ne bouge pas d'elle-même. **À refaire après un relancement.**

« Pas bit-perfect : le mode exclusif est coupé » n'est pas un défaut : c'est la quatrième
des quatre conditions (ADR 0007), et c'est l'**étape 12** qui la lève en cochant la case.

Restent, et elles demandent tes oreilles : **5** (à refaire) et tout le bloc du DAC.

## Sans rien brancher

1. **La liste des sorties est juste.** Déplie « sortie ». Tu dois y voir tes
   haut-parleurs, ton écran, et tout ce qui est branché — avec, le cas échéant, la mention
   « (AirPlay — jamais bit-perfect) » ou « (Bluetooth) ».
   **Échec** : liste vide, ou un nom manquant alors que macOS le montre dans Réglages → Son.

2. **Une piste locale joue.** Ajoute un dossier de musique, scanne, clique une piste.
   Tu dois **entendre le son dans la seconde**, et la position doit avancer.
   **Échec** : silence, ou un message d'erreur, ou la position reste à 0:00.
   ⚠️ **Une piste du NAS ne joue pas** et dit « la lecture réseau reste à porter ». C'est
   normal à ce stade, ce n'est pas un bug à signaler.

3. **Le compteur de trous reste à zéro.** En bas de la section Lecture :
   « Trous dans le son depuis le début : **0** ». Laisse jouer trois minutes.
   **Échec** : n'importe quel nombre au-dessus de zéro. C'est le défaut le plus important de
   toute cette liste — il veut dire que le son a coupé, même si tu ne l'as pas entendu.

4. **Pause et reprise sont instantanées.** Le son s'arrête net, reprend exactement où il
   s'était arrêté.
   **Échec** : un délai, un craquement, ou une reprise qui a sauté un bout.

5. **Le déplacement dans la piste est net.** Tire la barre « position ».
   **Échec** : tu entends un fragment de l'ancienne position avant la nouvelle (un hoquet),
   ou la barre recule d'elle-même juste après.

6. **Piste suivante et précédente répondent.** Un court silence est normal en changeant
   de piste à la main.
   **Échec** : rien ne se passe, ou ça joue la mauvaise piste.

7. **Le volume agit tout de suite**, et l'indicateur passe à « Pas bit-perfect : le volume
   n'est pas à 100 % » dès que tu bouges le curseur. Remets-le à 100 %.
   **Échec** : un délai de plusieurs secondes avant d'entendre le changement (cela
   voudrait dire que le volume est appliqué trop tôt dans la chaîne).

8. **L'enchaînement d'un album est sans trou.** Prends un album dont deux pistes se suivent
   sans silence entre elles — un live, un concept album. Clique la première et attends le
   passage à la seconde **sans toucher à rien**.
   **Échec** : un silence, un clic, ou une coupure au moment du passage. Le compteur de trous
   doit aussi rester à 0.

9. **Un fichier que la sortie ne sait pas jouer nativement affiche LES DEUX fréquences, et
   perd le badge bit-perfect.** C'est le point le plus important de la liste.
   Choisis « Haut-parleurs Mac mini » comme sortie — ils plafonnent à **96 kHz**, mesuré le
   2026-11-26 — puis joue un FLAC en 192 kHz. Tu dois lire :
   « Source : **192 kHz / 24 bits** · Sortie : **96 kHz (rééchantillonné)** », et juste en
   dessous « **Pas bit-perfect** : la sortie ne sait pas jouer cette fréquence. Le son est
   joué, mais il est converti — ce n'est pas une panne. »
   **Échec** : ça joue en affichant « Bit-perfect », ou la sortie affiche 192 kHz alors que
   le matériel ne sait pas le faire. Ce serait une altération que rien ne signale, et c'est
   exactement ce que l'indicateur existe pour empêcher.

9bis. **Et le même fichier sur un DAC qui sait faire 192 kHz affiche les deux fois 192 kHz**,
   avec le badge. C'est le témoin de l'étape précédente : sans lui, un indicateur bloqué sur
   « rééchantillonné » passerait pour juste.

## Avec le DAC

10. **Le DAC apparaît sans relancer l'app.** Branche-le pendant que l'app tourne, puis
    déplie « sortie ».
    **Échec** : il n'y est pas.

11. **Un 24/192 joue sur le DAC, et le DAC passe vraiment à 192 kHz.** Choisis le DAC, joue
    le fichier, puis ouvre **Configuration audio et MIDI** (dans Utilitaires) et regarde le
    format du DAC : il doit afficher 192 000 Hz.
    **Échec** : le son sort mais le DAC affiche une autre fréquence → c'est un
    rééchantillonnage silencieux, le défaut que tout ce travail existe pour fermer.

12. **Le mode exclusif s'active, et l'indicateur passe à « Bit-perfect ».** Coche la case.
    Il faut les quatre conditions réunies : pas de rééchantillonnage, volume à 100 %, mode
    exclusif, égaliseur plat (il n'existe pas encore, donc il est plat).
    **Échec** : « Pas bit-perfect » alors que les trois conditions sont visiblement remplies
    — la phrase dit alors laquelle manque, note-la.

13. **Débrancher le DAC en pleine lecture ne fait pas planter l'app.** Le son peut s'arrêter
    ou basculer sur les haut-parleurs ; les deux sont acceptables à ce stade.
    **Échec** : l'app se ferme, se fige, ou le son continue sur un périphérique muet.

13bis. **Un déplacement juste après un saut ne doit pas ajouter de trou.** Tire la barre de
   position, laisse jouer dix secondes, regarde le compteur.
   **Échec** : le compteur monte. Le temps de reconstituer l'avance après un saut est normal
   et n'est pas compté ; au-delà d'environ sept dixièmes de seconde, c'en est un vrai.

14. **En quittant, le DAC retrouve sa fréquence d'origine.** Note la fréquence du DAC dans
    Configuration audio et MIDI **avant** de lancer Hean, puis vérifie après l'avoir quitté.
    **Échec** : le DAC reste à 192 kHz. Hean ne doit pas laisser ton matériel dans un état
    que tu n'as pas demandé.

## Les deux pièges que je ne peux pas déclencher ici

Ils demandent de débrancher ou de changer de sortie au mauvais moment, et c'est justement là
que V1 se cassait.

17. **Changer la sortie dans les Réglages Son de macOS PENDANT une piste.** Le son continue
   sur l'ancienne sortie — c'est le comportement actuel, et c'est assumé : suivre ce
   changement en cours de piste n'est pas porté. Ce que l'app ne doit **pas** faire, c'est
   afficher le nom de la nouvelle sortie pendant que l'ancienne joue.
   **Échec** : le nom affiché change alors que le son ne bouge pas. Ce serait un indicateur
   qui mentirait sur la sortie, après en avoir menti sur la fréquence.

18. **Débrancher le DAC entre deux pistes d'un album.** Attends au plus quinze secondes.
   **Échec** : la lecture reste figée indéfiniment, barre immobile, sans message. Tu dois
   voir « La sortie audio ne répond plus — la fin de la piste est perdue. »

## Si tu as un AirPlay sous la main

15. **Choisir un AirPlay joue, et annonce que ce n'est pas bit-perfect.** macOS
    rééchantillonne de son côté et on ne peut pas l'en empêcher — l'app doit le dire plutôt
    que de prétendre le contraire.
    **Échec** : silence, ou « Bit-perfect » affiché.

16. **Quitter l'AirPlay pour le DAC, puis y revenir, fonctionne.** C'est le piège de
    plateforme le plus coûteux de V1 : un AirPlay cesse d'exister pour macOS dès qu'il n'est
    plus la sortie par défaut.
    **Échec** : l'AirPlay disparaît de la liste, ou y revenir ne donne aucun son.
    *(C'est le comportement **V008** du contrat anti-régression, et il reste marqué « à
    porter » : je n'ai aucun moyen de le vérifier sans AirPlay. Ton verdict ici est la seule
    preuve qui existera.)*

## Les trois vérifications de machine, si tu veux les relancer toi-même

Elles ne produisent aucun son et ne changent aucun réglage : elles listent les
périphériques, lisent une fréquence, et vérifient que le HAL accepte le format de flux de
Hean. Lancées le 2026-11-26 sur ton Mac mini : les trois passent.

```bash
cd /Users/tsunami25/Documents/Thomas/heanV2 && cargo test --manifest-path src-tauri/Cargo.toml --test sortie_coreaudio -- --ignored --nocapture
```

## Ce que ce protocole ne couvre pas, et pourquoi

| | |
|---|---|
| **La lecture d'un fichier du NAS** | elle demande de toucher au module réseau, hors du périmètre de la session (ADR 0013) — et 91 % de ta bibliothèque est là |
| L'égaliseur | ses huit fréquences de bandes sont une donnée de parité que le bootstrap ne contient pas |
| Les touches media, le plein écran, les paroles | hors périmètre, explicitement |
| Le délai de démarrage mesuré | il demande un marqueur dans l'app (MESURES-V1.md §3) |
