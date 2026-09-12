//! Telechargement, verification et installation des moteurs externes.
//!
//! Un moteur se decrit par une [`EngineSpec`] : une adresse, une empreinte, un
//! chemin d'executable. Tout le reste — recuperation, verification, depliage,
//! controle — est commun. Ajouter un moteur revient a ajouter une constante.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use derovia_core::CoreError;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// La forme sous laquelle un moteur est publie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    /// Une archive ZIP, depliee telle quelle.
    Zip,
    /// Un paquet MSI, deplie par **installation administrative**.
    ///
    /// `msiexec /a` extrait le contenu du paquet dans un dossier sans rien
    /// installer sur le systeme : ni entree au registre, ni raccourci, ni droits
    /// administrateur. Le moteur vit alors dans les donnees de l'application et
    /// disparait avec elle.
    MsiAdministrative,
}

/// La description complete d'un moteur externe.
#[derive(Debug, Clone, Copy)]
pub struct EngineSpec {
    /// Identifiant court, utilise dans les chemins et par l'interface.
    pub id: &'static str,
    /// Le nom lisible du moteur.
    pub label: &'static str,
    /// La version epinglee.
    pub version: &'static str,
    /// L'adresse de l'archive officielle.
    pub url: &'static str,
    /// L'empreinte SHA-256 de cette archive exactement.
    pub sha256: &'static str,
    /// Le poids du telechargement, en octets.
    pub download_bytes: u64,
    /// Le poids une fois deplie, en octets.
    pub installed_bytes: u64,
    /// Le chemin de l'executable, relatif au dossier du moteur.
    pub executable: &'static str,
    /// La facon de deplier l'archive.
    pub archive: ArchiveKind,
}

/// Pandoc — la reference pour la conversion de balisage.
///
/// La version **et son empreinte** sont epinglees : telecharger « la derniere
/// version » rendrait l'installation ni reproductible ni verifiable. Pour monter
/// de version, remplacer les champs ensemble apres avoir recalcule l'empreinte.
pub const PANDOC: EngineSpec = EngineSpec {
    id: "pandoc",
    label: "Pandoc",
    version: "3.11",
    url: "https://github.com/jgm/pandoc/releases/download/3.11/pandoc-3.11-windows-x86_64.zip",
    sha256: "2ab72baf2399450e148ddf7a2a8689806c42e1bba71862b57e220fd9b8456d3d",
    download_bytes: 41_761_100,
    installed_bytes: 234_000_000,
    executable: "pandoc-3.11/pandoc.exe",
    archive: ArchiveKind::Zip,
};

/// Tous les moteurs que la suite sait installer.
///
/// L'ecran de preparation du premier lancement parcourt cette liste : ajouter
/// un moteur a la suite, c'est ajouter une entree ici, rien d'autre.
pub const ENGINES: &[EngineSpec] = &[PANDOC];

/// Retrouve un moteur par son identifiant.
#[must_use]
pub fn find(id: &str) -> Option<&'static EngineSpec> {
    ENGINES.iter().find(|spec| spec.id == id)
}

/// L'etat de chacun des moteurs connus.
#[must_use]
pub fn all_statuses(base: &Path) -> Vec<EngineStatus> {
    ENGINES.iter().map(|spec| status(base, spec)).collect()
}

/// L'etat d'installation d'un moteur externe.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    /// L'identifiant du moteur.
    pub id: &'static str,
    /// Son nom lisible.
    pub label: &'static str,
    /// Vrai quand l'executable est present et repond.
    pub installed: bool,
    /// La version rapportee par l'executable, quand il est installe.
    pub version: Option<String>,
    /// Le poids du telechargement, en octets.
    pub download_bytes: u64,
    /// Le poids sur le disque une fois installe, en octets.
    pub installed_bytes: u64,
}

/// Le dossier dedie a un moteur.
fn engine_dir(base: &Path, spec: &EngineSpec) -> PathBuf {
    base.join("engines").join(spec.id)
}

/// Le chemin de l'executable d'un moteur.
#[must_use]
pub fn executable_path(base: &Path, spec: &EngineSpec) -> PathBuf {
    engine_dir(base, spec).join(spec.executable)
}

/// Le chemin de l'executable Pandoc.
#[must_use]
pub fn pandoc_path(base: &Path) -> PathBuf {
    executable_path(base, &PANDOC)
}

/// Indique si un moteur est installe et utilisable.
#[must_use]
pub fn status(base: &Path, spec: &EngineSpec) -> EngineStatus {
    let chemin = executable_path(base, spec);
    // On ne se contente pas de la presence du fichier : un executable present
    // mais incomplet — telechargement interrompu, extraction partielle — doit
    // compter comme absent.
    let version = chemin.is_file().then(|| crate::pandoc::version(&chemin)).flatten();
    EngineStatus {
        id: spec.id,
        label: spec.label,
        installed: version.is_some(),
        version,
        download_bytes: spec.download_bytes,
        installed_bytes: spec.installed_bytes,
    }
}

/// L'etat de Pandoc.
#[must_use]
pub fn pandoc_status(base: &Path) -> EngineStatus {
    status(base, &PANDOC)
}

/// Telecharge, verifie et installe un moteur.
///
/// `progress` recoit (octets recus, octets attendus) au fil du telechargement.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le reseau est indisponible, si l'empreinte de
/// l'archive ne correspond pas a celle attendue, ou si le depliage echoue.
pub fn install(
    base: &Path,
    spec: &EngineSpec,
    mut progress: impl FnMut(u64, u64),
) -> Result<EngineStatus, CoreError> {
    let dossier = engine_dir(base, spec);
    fs::create_dir_all(&dossier).map_err(|e| {
        CoreError::failure("moteur", format!("Impossible de créer le dossier du moteur : {e}"))
    })?;

    let extension = match spec.archive {
        ArchiveKind::Zip => "zip",
        ArchiveKind::MsiAdministrative => "msi",
    };
    let archive = dossier.join(format!("telechargement.{extension}"));
    telecharger(spec.url, &archive, spec.download_bytes, &mut progress)?;

    if empreinte_sha256(&archive)? != spec.sha256 {
        // L'archive est effacee : la garder inviterait a la reutiliser plus tard
        // sans la reverifier.
        let _ = fs::remove_file(&archive);
        return Err(CoreError::failure(
            "moteur",
            format!(
                "Le téléchargement de {} ne correspond pas à l'empreinte attendue. \
                 Il a été interrompu ou altéré ; rien n'a été installé.",
                spec.label
            ),
        ));
    }

    match spec.archive {
        ArchiveKind::Zip => deplier_zip(&archive, &dossier)?,
        ArchiveKind::MsiAdministrative => deplier_msi(&archive, &dossier)?,
    }
    let _ = fs::remove_file(&archive);

    let statut = status(base, spec);
    if !statut.installed {
        return Err(CoreError::failure(
            "moteur",
            format!("{} a été extrait mais ne répond pas.", spec.label),
        ));
    }
    Ok(statut)
}

/// Installe Pandoc.
///
/// # Erreurs
///
/// Voir [`install`].
pub fn install_pandoc(
    base: &Path,
    progress: impl FnMut(u64, u64),
) -> Result<EngineStatus, CoreError> {
    install(base, &PANDOC, progress)
}

/// Recupere un fichier en flux, en signalant l'avancement.
fn telecharger(
    url: &str,
    destination: &Path,
    taille_annoncee: u64,
    progress: &mut impl FnMut(u64, u64),
) -> Result<(), CoreError> {
    let reponse = ureq::get(url).call().map_err(|e| {
        CoreError::failure(
            "moteur",
            format!("Téléchargement impossible — vérifiez votre connexion. ({e})"),
        )
    })?;

    let attendu = reponse
        .headers()
        .get("content-length")
        .and_then(|valeur| valeur.to_str().ok())
        .and_then(|valeur| valeur.parse::<u64>().ok())
        .unwrap_or(taille_annoncee);

    let mut source = reponse.into_body().into_reader();
    let mut fichier = File::create(destination).map_err(|e| {
        CoreError::failure("moteur", format!("Impossible d'écrire le téléchargement : {e}"))
    })?;

    // Un tampon de 64 Kio : assez grand pour ne pas multiplier les appels
    // systeme, assez petit pour que la progression reste fluide a l'ecran.
    let mut tampon = vec![0_u8; 64 * 1024];
    let mut recus = 0_u64;
    loop {
        let lus = source.read(&mut tampon).map_err(|e| {
            CoreError::failure("moteur", format!("Téléchargement interrompu : {e}"))
        })?;
        if lus == 0 {
            break;
        }
        fichier
            .write_all(tampon.get(..lus).unwrap_or_default())
            .map_err(|e| CoreError::failure("moteur", format!("Écriture impossible : {e}")))?;
        recus += lus as u64;
        progress(recus, attendu);
    }
    Ok(())
}

/// Calcule l'empreinte SHA-256 d'un fichier, sans le charger en memoire.
fn empreinte_sha256(chemin: &Path) -> Result<String, CoreError> {
    let mut fichier = File::open(chemin)
        .map_err(|e| CoreError::failure("moteur", format!("Fichier téléchargé illisible : {e}")))?;
    let mut hacheur = Sha256::new();
    let mut tampon = vec![0_u8; 64 * 1024];
    loop {
        let lus = fichier
            .read(&mut tampon)
            .map_err(|e| CoreError::failure("moteur", format!("Lecture impossible : {e}")))?;
        if lus == 0 {
            break;
        }
        hacheur.update(tampon.get(..lus).unwrap_or_default());
    }
    Ok(format!("{:x}", hacheur.finalize()))
}

/// Deplie une archive ZIP.
fn deplier_zip(archive: &Path, destination: &Path) -> Result<(), CoreError> {
    let fichier = File::open(archive)
        .map_err(|e| CoreError::failure("moteur", format!("Archive illisible : {e}")))?;
    let mut zip = zip::ZipArchive::new(fichier)
        .map_err(|e| CoreError::failure("moteur", format!("Archive invalide : {e}")))?;

    for index in 0..zip.len() {
        let mut entree = zip.by_index(index).map_err(|e| {
            CoreError::failure("moteur", format!("Entrée d'archive illisible : {e}"))
        })?;

        // `enclosed_name` refuse les chemins qui sortiraient du dossier cible :
        // une archive forgee pourrait sinon ecrire n'importe ou sur le disque.
        let Some(relatif) = entree.enclosed_name() else {
            continue;
        };
        let cible = destination.join(relatif);

        if entree.is_dir() {
            fs::create_dir_all(&cible).map_err(|e| {
                CoreError::failure("moteur", format!("Création de dossier impossible : {e}"))
            })?;
            continue;
        }
        if let Some(parent) = cible.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CoreError::failure("moteur", format!("Création de dossier impossible : {e}"))
            })?;
        }
        let mut sortie = File::create(&cible)
            .map_err(|e| CoreError::failure("moteur", format!("Extraction impossible : {e}")))?;
        std::io::copy(&mut entree, &mut sortie)
            .map_err(|e| CoreError::failure("moteur", format!("Extraction interrompue : {e}")))?;
    }
    Ok(())
}

/// Deplie un paquet MSI sans l'installer sur le systeme.
///
/// `msiexec /a … /qn TARGETDIR=…` realise une **installation administrative** :
/// elle extrait l'arborescence du paquet dans le dossier indique, sans ecrire
/// au registre, sans raccourci, et sans droits administrateur. Le moteur reste
/// confine aux donnees de l'application.
fn deplier_msi(archive: &Path, destination: &Path) -> Result<(), CoreError> {
    let sortie = Command::new("msiexec.exe")
        .arg("/a")
        .arg(archive)
        .arg("/qn")
        .arg(format!("TARGETDIR={}", destination.display()))
        .output()
        .map_err(|e| CoreError::failure("moteur", format!("msiexec introuvable : {e}")))?;

    if !sortie.status.success() {
        return Err(CoreError::failure(
            "moteur",
            format!(
                "Le dépliage du paquet a échoué (code {}).",
                sortie.status.code().unwrap_or(-1)
            ),
        ));
    }
    Ok(())
}
