//! La migration one-shot de `~/Library/Application Support/noir/` vers `hean/` (ADR 0003).
//!
//! **Ce que ce module protège : la bibliothèque, les playlists, les favoris, l'historique
//! d'écoute et les sources NAS d'environ cinquante beta-testeurs.** C'est l'une des cinq
//! zones jamais déléguées (`tools/zones-interdites.txt`), et la raison y est écrite : un
//! test vert sur des fixtures ne prouve rien, parce que la réalité a des noms de fichiers en
//! NFD, des caches à demi écrits et des dossiers que personne n'avait prévus.
//!
//! ## La forme, et pourquoi c'est celle-là
//!
//! **Copier dans un dossier provisoire, vérifier, forcer sur le disque, échanger, supprimer
//! l'original.** Cinq temps, et chacun ferme un mode de défaut :
//!
//! 1. **Copier à côté** (`hean.en-cours-de-migration/`) plutôt que directement dans `hean/`.
//!    Une copie interrompue — l'application tuée, la machine qui s'endort — laisse alors un
//!    dossier provisoire et **pas de `hean/` du tout**. Le lancement suivant repart donc de
//!    zéro proprement. Si on copiait directement dans `hean/`, une interruption laisserait
//!    une bibliothèque à moitié présente, indistinguable d'une bibliothèque complète.
//! 2. **Vérifier** (`verification.rs`) : chaque fichier par sa taille, les irremplaçables
//!    par leur contenu. C'est ce qui autorise l'étape suivante.
//! 3. **Forcer sur le disque** (`durabilite.rs`) : `fs::copy` rend la main quand le cache du
//!    système est à jour, pas quand le disque l'est. Sans ce temps-là, une coupure de courant
//!    juste après la migration laissait un `hean/` d'apparence complète, des fichiers vides
//!    dedans et plus de `noir/` du tout. C'est INV-25, née de la relecture de cette PR.
//! 4. **Échanger** : un `rename` du provisoire vers `hean/`. Sur le même volume, c'est
//!    atomique — il n'existe aucun instant où `hean/` est à moitié là.
//! 5. **Supprimer l'original**, et seulement là. L'ADR 0003 l'écrit : « copie puis
//!    vérification puis suppression de l'ancien, pas un `mv` sec ».
//!
//! ## Ce qui n'est pas ici
//!
//! - **Les entrées Keychain** (mots de passe SMB). Renommer un dossier ne les déplace pas,
//!   et le nom de service que V2 emploiera est un point ouvert daté de la phase 3
//!   (ADR 0003). Tant qu'il n'est pas tranché, cette migration n'y touche pas : ne rien
//!   faire laisse les mots de passe en place, s'y prendre à moitié les perdrait.
//! - **Aucune place disque n'est estimée avant de commencer.** Une estimation peut être
//!   fausse ; un disque plein, lui, fait échouer la copie, et cet échec est traité comme
//!   tous les autres — le provisoire est effacé, l'original n'a pas bougé. Un pré-contrôle
//!   ajouterait du code et une façon de se tromper, sans rien garantir de plus.

mod durabilite;
pub mod inventaire;
pub mod verification;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use verification::Ecart;

/// Le nom du dossier provisoire, voisin du dossier d'arrivée.
const EN_COURS: &str = "hean.en-cours-de-migration";

#[derive(Debug, PartialEq)]
pub enum Etat {
    /// Faite à l'instant.
    Faite,
    /// Un témoin était déjà là : rien à faire, et c'est le cas normal à partir du deuxième
    /// lancement.
    DejaFaite,
    /// Aucun dossier `noir/` : une installation neuve, ou un testeur qui n'a jamais lancé V1.
    RienAMigrer,
    /// `hean/` existe déjà avec des données et sans témoin. On ne fusionne pas deux
    /// bibliothèques : on ne touche à rien et on le dit.
    CibleDejaPeuplee,
}

#[derive(Debug)]
pub struct Rapport {
    pub etat: Etat,
    pub fichiers: u32,
    pub octets: u64,
    /// Les fichiers délibérément laissés derrière (`smb_buffer/`, fichiers cachés).
    pub ignores: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error("copie de {fichier} : {source}")]
    Copie {
        fichier: String,
        #[source]
        source: std::io::Error,
    },
    /// Le message porte **tous** les écarts : qui doit comprendre ce qui est arrivé à ses
    /// données a besoin de leur étendue, pas d'un symptôme.
    #[error("la copie ne correspond pas à l'original, rien n'a été supprimé — {}", lister(ecarts))]
    Verification { ecarts: Vec<Ecart> },
    /// Un échec de synchronisation **arrête la migration**, et c'est voulu : l'étape suivante
    /// est la suppression de `noir/`, qu'on ne fait pas sur une copie dont on ne sait pas si
    /// elle a atteint le disque (INV-25, et `durabilite` pour le mode de défaut).
    #[error("la copie n'a pas pu être forcée sur le disque ({dossier}) : {source}")]
    Durabilite {
        dossier: String,
        #[source]
        source: std::io::Error,
    },
    #[error("échange du dossier provisoire vers {cible} : {source}")]
    Echange {
        cible: String,
        #[source]
        source: std::io::Error,
    },
    #[error("lecture de {dossier} : {source}")]
    Lecture {
        dossier: String,
        #[source]
        source: std::io::Error,
    },
}

fn lister(ecarts: &[Ecart]) -> String {
    ecarts.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ; ")
}

/// Migre si c'est nécessaire et pas encore fait. **Idempotente** : le témoin écrit dans le
/// dossier d'arrivée est la seule source de vérité sur « est-ce déjà passé ? ».
pub fn migrer_si_necessaire(source: &Path, cible: &Path) -> Result<Rapport, Erreur> {
    let vide = Rapport { etat: Etat::DejaFaite, fichiers: 0, octets: 0, ignores: 0 };

    if cible.join(inventaire::TEMOIN).exists() {
        return Ok(vide);
    }
    if !source.is_dir() {
        return Ok(Rapport { etat: Etat::RienAMigrer, ..vide });
    }
    if contient_des_donnees(cible) {
        tracing::warn!(
            cible = %cible.display(),
            "le dossier de données existe déjà sans témoin de migration : on n'y touche pas"
        );
        return Ok(Rapport { etat: Etat::CibleDejaPeuplee, ..vide });
    }

    let (relatifs, ignores) = inventorier(source)?;
    let provisoire = cible.with_file_name(EN_COURS);

    // Un provisoire qui traîne est le reste d'une tentative interrompue : il ne contient
    // rien qu'on ne sache refaire, et le garder ferait accumuler des copies partielles.
    let _ = std::fs::remove_dir_all(&provisoire);

    match copier(source, &provisoire, &relatifs) {
        Ok(octets) => {
            let ecarts = verification::verifier(source, &provisoire, &relatifs);
            if !ecarts.is_empty() {
                let _ = std::fs::remove_dir_all(&provisoire);
                return Err(Erreur::Verification { ecarts });
            }
            // Le témoin est écrit AVANT l'échange : il doit faire partie du dossier au
            // moment où celui-ci devient `hean/`, sinon une interruption entre les deux
            // laisserait une bibliothèque complète que le lancement suivant croirait
            // non migrée — et qui refuserait de démarrer en la trouvant « déjà peuplée ».
            ecrire_le_temoin(&provisoire, source, relatifs.len(), octets, ignores)?;

            // **La copie est forcée sur le disque avant toute chose irréversible** (INV-25).
            // `fs::copy` rend la main quand le cache du système est à jour, pas le disque : une
            // coupure de courant dans les secondes qui suivaient laissait un `hean/` avec son
            // témoin et des fichiers vides, `noir/` déjà supprimé. Un échec ici arrête la
            // migration — rien n'est échangé, rien n'est supprimé, le lancement suivant repart.
            // (`erreur` et non `source` : dans cette fonction, `source` est le dossier de V1,
            // et une liaison qui l'éclipserait sur plusieurs lignes est un piège.)
            if let Err(erreur) = durabilite::rendre_durable(&provisoire) {
                let _ = std::fs::remove_dir_all(&provisoire);
                return Err(Erreur::Durabilite {
                    dossier: provisoire.display().to_string(),
                    source: erreur,
                });
            }

            // L'échange est un `rename`, et un `rename` exige une cible vide. Un dossier
            // d'arrivée qui ne contient qu'un `.DS_Store` est vide au sens où nous
            // l'entendons (voir `contient_des_donnees`) mais pas au sens du système : sans
            // cette ligne, la migration partirait puis échouerait à la dernière étape, à
            // chaque démarrage. Les fichiers cachés sont donc **déplacés** dans le
            // provisoire, et non supprimés : ils retrouvent leur place après l'échange.
            //
            // **La nuance, relevée en relecture** : si le `rename` juste en dessous échoue,
            // ils restent dans le provisoire — que le démarrage suivant efface. Ils seraient
            // donc perdus sur ce chemin-là, et « jamais supprimés » serait trop dire. Ce qui
            // est en jeu est un fichier de préférences d'affichage du Finder, que le Finder
            // réécrit ; ce qui compte ici est de ne pas l'écrire plus fort que ce n'est vrai.
            deplacer_les_fichiers_caches(cible, &provisoire);

            std::fs::rename(&provisoire, cible).map_err(|source| Erreur::Echange {
                cible: cible.display().to_string(),
                source,
            })?;

            // L'original, enfin — **et seulement si le renommage lui-même est durable**
            // (INV-25) : c'est l'entrée du dossier parent qui porte le nouveau nom, et elle
            // peut n'être pas encore sur le disque. Sinon on garde `noir/`, et le coût de le
            // garder est plus faible qu'il n'y paraît : les fichiers migrés sont des **clones**
            // (voir `verification.rs`), donc ils partagent leurs blocs avec `hean/` et ne sont
            // pas comptés deux fois. Ce que `noir/` occupe encore en propre est ce qui n'a PAS
            // été migré — surtout les 289 Mo de `smb_buffer/` — c'est-à-dire environ la moitié
            // de ses 572 Mo apparents. Dans tous les cas, moins qu'un dossier de données qui
            // n'existerait plus sous aucun des deux noms. Le témoin étant dans `hean/`, le lancement suivant ne recommencera
            // pas — ce qui est correct, puisque les données, elles, sont bien arrivées.
            match durabilite::rendre_durable_le_renommage(cible) {
                // Un échec de suppression n'est pas fatal : les données sont en sécurité à
                // l'arrivée, et le témoin empêchera de recommencer.
                Ok(()) => {
                    if let Err(e) = std::fs::remove_dir_all(source) {
                        tracing::warn!(
                            source = %source.display(),
                            erreur = %e,
                            "l'ancien dossier n'a pas pu être supprimé — les données sont pourtant bien migrées"
                        );
                    }
                }
                Err(e) => tracing::error!(
                    cible = %cible.display(),
                    erreur = %e,
                    "le renommage n'a pas pu être forcé sur le disque : l'ancien dossier est conservé"
                ),
            }
            tracing::info!(
                fichiers = relatifs.len(),
                octets,
                ignores,
                "migration des données de V1 terminée"
            );
            Ok(Rapport {
                etat: Etat::Faite,
                fichiers: u32::try_from(relatifs.len()).unwrap_or(u32::MAX),
                octets,
                ignores,
            })
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&provisoire);
            Err(e)
        }
    }
}

/// Vrai si le dossier existe et contient des données.
///
/// Le témoin n'en est pas : c'est nous qui l'écrivons. **Les fichiers cachés non plus**, et
/// c'est la correction d'un défaut : un seul `.DS_Store`, déposé par un passage dans le
/// Finder, suffisait à faire prendre un dossier vide pour une bibliothèque en place — donc à
/// éteindre la migration définitivement. L'inventaire ignore déjà les fichiers cachés
/// (`inventaire::a_migrer`) ; ce test-ci doit s'accorder avec lui, sans quoi le même fichier
/// est « sans intérêt » d'un côté et « une bibliothèque » de l'autre.
fn contient_des_donnees(dossier: &Path) -> bool {
    std::fs::read_dir(dossier).is_ok_and(|mut e| {
        e.any(|x| {
            x.is_ok_and(|x| {
                let nom = x.file_name();
                nom != inventaire::TEMOIN && !inventaire::est_cache(&nom)
            })
        })
    })
}

/// Déplace les fichiers cachés du dossier d'arrivée vers le provisoire, pour que l'échange
/// trouve une cible vide.
///
/// Chaque échec est **journalisé**, et c'est la correction d'un `let _ =` muet relevé en
/// relecture : un fichier caché qu'on ne sait pas déplacer fait échouer l'échange, donc
/// désormais refuser le démarrage (INV-24) — et l'erreur d'échange nomme la cible, pas le
/// fichier qui bloque. Sans cette ligne, le refus de démarrer serait indiagnosticable.
fn deplacer_les_fichiers_caches(cible: &Path, provisoire: &Path) {
    let Ok(entrees) = std::fs::read_dir(cible) else {
        return;
    };
    for entree in entrees.flatten() {
        let nom = entree.file_name();
        if !inventaire::est_cache(&nom) {
            continue;
        }
        if let Err(e) = std::fs::rename(entree.path(), provisoire.join(&nom)) {
            tracing::warn!(
                fichier = %entree.path().display(),
                erreur = %e,
                "fichier caché non déplaçable : l'échange du dossier va échouer à cause de lui"
            );
        }
    }
}

/// Les chemins relatifs à migrer, et le nombre de fichiers délibérément ignorés.
fn inventorier(source: &Path) -> Result<(Vec<PathBuf>, u32), Erreur> {
    let mut a_migrer = Vec::new();
    let mut ignores = 0u32;
    descendre(source, source, &mut a_migrer, &mut ignores)?;
    a_migrer.sort();
    Ok((a_migrer, ignores))
}

fn descendre(
    racine: &Path,
    dossier: &Path,
    out: &mut Vec<PathBuf>,
    ignores: &mut u32,
) -> Result<(), Erreur> {
    let entrees = std::fs::read_dir(dossier).map_err(|source| Erreur::Lecture {
        dossier: dossier.display().to_string(),
        source,
    })?;
    for entree in entrees {
        let entree = entree.map_err(|source| Erreur::Lecture {
            dossier: dossier.display().to_string(),
            source,
        })?;
        let chemin = entree.path();
        let relatif = chemin.strip_prefix(racine).unwrap_or(&chemin).to_path_buf();
        if !inventaire::a_migrer(&relatif) {
            *ignores += compter_les_fichiers(&chemin);
            continue;
        }
        // Un lien symbolique n'est pas suivi : il pourrait pointer hors du dossier de
        // données, et une migration ne doit copier que ce qu'elle a vu.
        match entree.file_type() {
            Ok(t) if t.is_dir() => descendre(racine, &chemin, out, ignores)?,
            Ok(t) if t.is_file() => out.push(relatif),
            _ => *ignores += 1,
        }
    }
    Ok(())
}

fn compter_les_fichiers(chemin: &Path) -> u32 {
    if chemin.is_file() {
        return 1;
    }
    let Ok(entrees) = std::fs::read_dir(chemin) else {
        return 0;
    };
    entrees
        .flatten()
        .map(|e| compter_les_fichiers(&e.path()))
        .sum()
}

fn copier(source: &Path, provisoire: &Path, relatifs: &[PathBuf]) -> Result<u64, Erreur> {
    // Le dossier provisoire est créé explicitement, et pas seulement comme parent du
    // premier fichier copié : un `noir/` qui ne contiendrait que des fichiers ignorés
    // n'aurait aucun fichier à copier, et le témoin n'aurait alors nulle part à s'écrire.
    std::fs::create_dir_all(provisoire).map_err(|source| Erreur::Copie {
        fichier: provisoire.display().to_string(),
        source,
    })?;
    let mut octets = 0u64;
    for relatif in relatifs {
        let arrivee = provisoire.join(relatif);
        if let Some(parent) = arrivee.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Erreur::Copie {
                fichier: relatif.display().to_string(),
                source,
            })?;
        }
        octets += std::fs::copy(source.join(relatif), &arrivee).map_err(|source| Erreur::Copie {
            fichier: relatif.display().to_string(),
            source,
        })?;
    }
    Ok(octets)
}

fn ecrire_le_temoin(
    dossier: &Path,
    source: &Path,
    fichiers: usize,
    octets: u64,
    ignores: u32,
) -> Result<(), Erreur> {
    let secondes = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let contenu = serde_json::json!({
        "migre_depuis": source.display().to_string(),
        "le": secondes,
        "fichiers": fichiers,
        "octets": octets,
        "ignores": ignores,
        "note": "Écrit par heanV2 après vérification de la copie (ADR 0003). \
                 Le supprimer ferait recommencer la migration au prochain lancement.",
    });
    std::fs::write(
        dossier.join(inventaire::TEMOIN),
        format!("{contenu:#}\n"),
    )
    .map_err(|source| Erreur::Copie {
        fichier: inventaire::TEMOIN.to_string(),
        source,
    })
}
