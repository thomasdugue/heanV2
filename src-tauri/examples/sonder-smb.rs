//! Sonde une source réseau et dit **à quelle étape** ça casse.
//!
//! Un exemple et non un test : il touche un vrai NAS, donc il ne peut pas tourner en CI.
//! Ses `println!` sont sa raison d'être (dérogation à INV-21, ADR 0011).
//!
//! Il n'affiche **jamais** le mot de passe : il le lit par le même chemin que l'application
//! (`network::identifiants`) et n'en dit que la longueur.
//!
//! ```
//! cargo run --example sonder-smb -- <id-de-la-source>
//! ```

use hean_lib::library::{dossier, Depot};
use hean_lib::network::identifiants;
use pavao::{SmbClient, SmbCredentials, SmbDirentType, SmbOptions};

fn essai(titre: &str, serveur: &str, partage: &str, chemin: &str, u: &str, mdp: &str, wg: &str) {
    print!("  {titre:<58} ");
    let mut creds = SmbCredentials::default().server(serveur).share(partage);
    if !u.is_empty() {
        creds = creds.username(u).password(mdp);
    }
    if !wg.is_empty() {
        creds = creds.workgroup(wg);
    }
    match SmbClient::new(creds, SmbOptions::default().one_share_per_server(true)) {
        Err(e) => println!("connexion refusée : {e}"),
        Ok(c) => match c.list_dir(chemin) {
            Err(e) => println!("list_dir échoue : {e}"),
            Ok(entrees) => {
                let dossiers = entrees.iter().filter(|e| matches!(e.get_type(), SmbDirentType::Dir)).count();
                let partages = entrees.iter().filter(|e| matches!(e.get_type(), SmbDirentType::FileShare)).count();
                println!(
                    "OK — {} entrées ({dossiers} dossiers, {partages} partages) : {:?}",
                    entrees.len(),
                    entrees.iter().take(6).map(|e| e.name().to_string()).collect::<Vec<_>>()
                );
            }
        },
    }
}

fn main() {
    tracing_subscriber::fmt().with_writer(std::io::stderr).with_target(false).without_time().init();
    let id = std::env::args().skip(1).find(|a| !a.starts_with("--")).unwrap_or_default();
    let d = dossier::dossier_de_donnees().expect("dossier de données");
    let depot = Depot::ouvrir(&dossier::base_dans(&d)).expect("base");
    let sources = depot.sources_reseau().expect("sources");
    let source = if id.is_empty() {
        sources.first().cloned().expect("aucune source réseau enregistrée")
    } else {
        sources.into_iter().find(|s| s.id == id).expect("source inconnue")
    };

    println!("source   : {} ({})", source.nom, source.id);
    println!("hôte     : {}", source.hote);
    println!("partage  : {}", source.partage);
    println!("chemin   : {}", source.chemin_distant);
    println!("user     : {:?}  invité={}  domaine={:?}", source.utilisateur, source.invite, source.domaine);

    // INV-04 (🔴) vaut jusque dans un outil de diagnostic, et ici plus qu'ailleurs : les
    // essais ci-dessous envoient le **vrai** nom d'utilisateur. Avec un mot de passe vide,
    // c'est une dizaine d'authentifications ratées d'affilée sur le compte de Thomas, donc le
    // risque de verrouillage qu'INV-04 existe pour empêcher — dans le seul outil du dépôt qui
    // parle au vrai NAS. Une sonde qui verrouille le compte qu'elle diagnostique est pire que
    // pas de sonde. Relevé par `invariant-guardian` le 2026-11-26 : le repli était
    // `String::new()`, que le détecteur d'architecture ne voit pas puisqu'il ne cherche que
    // le littéral `unwrap_or_default()`.
    let mdp = if source.invite {
        String::new() // une source invité n'envoie pas d'identifiants du tout : `u` est vide
    } else {
        match identifiants::mot_de_passe(&source.id) {
            Ok(m) => {
                println!("mot de passe : trouvé au trousseau, {} caractères", m.chars().count());
                m
            }
            Err(e) => {
                println!("mot de passe : ABSENT — {e}");
                println!(
                    "\nLa sonde s'arrête ici, exprès. Essayer le compte « {} » avec un mot de\n\
                     passe vide, c'est une dizaine d'authentifications ratées d'affilée sur le\n\
                     NAS, donc un risque de verrouillage du compte. Saisir le mot de passe dans\n\
                     Hean, puis relancer la sonde.",
                    source.utilisateur
                );
                return;
            }
        }
    };
    let u = if source.invite { String::new() } else { source.utilisateur.clone() };
    let wg = source.domaine.clone().unwrap_or_default();
    let srv = format!("smb://{}", source.hote);
    let partage = format!("/{}", source.partage.trim_matches('/'));
    let chemin = format!("/{}", source.chemin_distant.trim_matches('/'));

    println!("\n— niveau serveur (teste l'authentification seule) —");
    essai("serveur, list_dir(\"\")", &srv, "", "", &u, &mdp, &wg);
    essai("serveur, list_dir(\"/\")", &srv, "", "/", &u, &mdp, &wg);

    println!("\n— niveau partage (ce que fait l'app) —");
    essai("partage, list_dir(\"/\")", &srv, &partage, "/", &u, &mdp, &wg);
    essai("partage, list_dir(chemin distant)", &srv, &partage, &chemin, &u, &mdp, &wg);
    essai("partage, list_dir(chemin sans slash)", &srv, &partage, chemin.trim_start_matches('/'), &u, &mdp, &wg);
    essai("partage dans l'URI, chemin complet", &srv, "", &format!("{partage}{chemin}"), &u, &mdp, &wg);

    println!("\n— list_dirplus, ce que l'app essaie EN PREMIER —");
    {
        let mut creds = SmbCredentials::default().server(&srv).share(&partage);
        if !u.is_empty() {
            creds = creds.username(&u).password(&mdp);
        }
        let c = SmbClient::new(creds, SmbOptions::default().one_share_per_server(true))
            .expect("connexion");
        print!("  list_dirplus(chemin distant)                               ");
        match c.list_dirplus(&chemin) {
            Ok(e) => println!("OK — {} entrées", e.len()),
            Err(e) => println!("ÉCHOUE : {e}"),
        }
        // Et juste après, sur LE MÊME client : est-ce que list_dir marche encore ?
        print!("  puis list_dir(chemin distant) sur le MÊME client           ");
        match c.list_dir(&chemin) {
            Ok(e) => println!("OK — {} entrées", e.len()),
            Err(e) => println!("ÉCHOUE AUSSI : {e}"),
        }
    }

    // Les fichiers que le scan a dits illisibles : vraiment abîmés, ou mal lus par nous ?
    if std::env::args().any(|a| a == "--illisibles") {
        println!("\n— les fichiers que le scan n'a pas su lire —");
        let mut creds = SmbCredentials::default().server(&srv).share(&partage);
        if !u.is_empty() {
            creds = creds.username(&u).password(&mdp);
        }
        let c = SmbClient::new(creds, SmbOptions::default().one_share_per_server(true))
            .expect("connexion");
        let entrees = c.list_dirplus(&chemin).expect("liste");
        let mut vides = 0;
        for e in entrees.iter().filter(|e| e.name().ends_with(".flac") || e.name().ends_with(".mp3")) {
            if e.size < 4096 {
                println!("  {:>10} octets  {}", e.size, e.name());
                vides += 1;
            }
        }
        println!("  → {vides} fichier(s) audio de moins de 4 Ko à la racine du dossier scanné");
    }

    println!("\n— variantes d'identité —");
    essai("sans domaine explicite", &srv, &partage, &chemin, &u, &mdp, "");
    essai("domaine WORKGROUP", &srv, &partage, &chemin, &u, &mdp, "WORKGROUP");
    essai("anonyme (invité)", &srv, &partage, &chemin, "", "", "");
}
