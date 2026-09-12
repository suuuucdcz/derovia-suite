//! Pilotage de l'executable Pandoc.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use derovia_convert::{SourceFormat, TargetFormat};
use derovia_core::CoreError;

/// Empeche l'apparition d'une fenetre de console derriere l'application.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Prepare la commande, sans fenetre de console sous Windows.
fn command(executable: &Path) -> Command {
    let mut commande = Command::new(executable);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        commande.creation_flags(CREATE_NO_WINDOW);
    }
    commande
}

/// La version rapportee par l'executable, si celui-ci repond.
#[must_use]
pub fn version(executable: &Path) -> Option<String> {
    let sortie = command(executable).arg("--version").output().ok()?;
    if !sortie.status.success() {
        return None;
    }
    String::from_utf8_lossy(&sortie.stdout)
        .lines()
        .next()
        .map(|ligne| ligne.trim().to_owned())
        .filter(|ligne| !ligne.is_empty())
}

/// Le nom de format que Pandoc attend pour une entree, s'il sait la lire.
fn reader_name(source: SourceFormat) -> Option<&'static str> {
    match source {
        SourceFormat::Docx => Some("docx"),
        SourceFormat::Html => Some("html"),
        // Du texte brut est du Markdown valide : le lire ainsi preserve les
        // paragraphes, la ou un lecteur « plain » n'existe pas chez Pandoc.
        SourceFormat::Markdown | SourceFormat::Text => Some("markdown"),
        // Pandoc n'a aucun lecteur PDF, ne lit pas le .doc binaire, et laisse
        // les classeurs et presentations au moteur haute fidelite.
        SourceFormat::Pdf
        | SourceFormat::DocLegacy
        | SourceFormat::Image
        | SourceFormat::Odt
        | SourceFormat::Spreadsheet
        | SourceFormat::Presentation
        | SourceFormat::Unknown => None,
    }
}

/// Le nom de format que Pandoc attend pour une sortie, s'il sait l'ecrire.
fn writer_name(target: TargetFormat) -> Option<&'static str> {
    match target {
        TargetFormat::Docx => Some("docx"),
        TargetFormat::Markdown => Some("markdown"),
        TargetFormat::Html => Some("html"),
        TargetFormat::Text => Some("plain"),
        // Pandoc ne produit un PDF qu'avec un moteur LaTeX installe a cote, et
        // ne dessine pas d'image : ces cibles restent au moteur interne.
        TargetFormat::Pdf | TargetFormat::Png | TargetFormat::Jpeg | TargetFormat::Webp => None,
    }
}

/// Indique si Pandoc sait traiter ce couple source/cible.
///
/// Quand la reponse est fausse, l'appelant retombe sur le moteur interne.
#[must_use]
pub fn handles(source: SourceFormat, target: TargetFormat) -> bool {
    reader_name(source).is_some() && writer_name(target).is_some()
}

/// Un chemin de travail unique, dans le dossier temporaire du systeme.
fn chemin_temporaire(extension: &str) -> PathBuf {
    let marque = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duree| duree.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("derovia-{marque}-{}.{extension}", std::process::id()))
}

/// Nettoie un fichier de travail sans jamais faire echouer la conversion.
fn effacer(chemin: &Path) {
    let _ = fs::remove_file(chemin);
}

/// Convertit un document en passant par Pandoc.
///
/// L'echange se fait par fichiers temporaires plutot que par les flux standard :
/// les formats binaires comme `.docx` ne traversent pas proprement une sortie
/// standard, et Pandoc emet un avertissement si on le lui demande.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le couple de formats n'est pas de son ressort,
/// si l'executable ne peut etre lance, ou si Pandoc refuse le document.
pub fn run_pandoc(
    executable: &Path,
    input_bytes: &[u8],
    source: SourceFormat,
    target: TargetFormat,
) -> Result<Vec<u8>, CoreError> {
    let (Some(lecteur), Some(redacteur)) = (reader_name(source), writer_name(target)) else {
        return Err(CoreError::failure(
            "pandoc",
            "Ce couple de formats n'est pas du ressort de Pandoc.".to_owned(),
        ));
    };

    let entree = chemin_temporaire("in");
    let sortie = chemin_temporaire("out");

    fs::write(&entree, input_bytes).map_err(|e| {
        CoreError::failure("pandoc", format!("Impossible de préparer le document : {e}"))
    })?;

    let resultat = command(executable)
        .args([
            OsStr::new("--from"),
            OsStr::new(lecteur),
            OsStr::new("--to"),
            OsStr::new(redacteur),
            // Un document complet plutot qu'un fragment : sans cela, la sortie
            // HTML n'aurait ni en-tete ni encodage declare.
            OsStr::new("--standalone"),
            OsStr::new("--output"),
            sortie.as_os_str(),
            entree.as_os_str(),
        ])
        .output();

    effacer(&entree);

    let sortie_commande = resultat.map_err(|e| {
        effacer(&sortie);
        CoreError::failure("pandoc", format!("Pandoc n'a pas pu être lancé : {e}"))
    })?;

    if !sortie_commande.status.success() {
        effacer(&sortie);
        // Le message de Pandoc est explicite ; on le transmet plutot que de le
        // remplacer par un « echec de conversion » qui n'apprend rien.
        let details = String::from_utf8_lossy(&sortie_commande.stderr);
        let details = details.trim();
        return Err(CoreError::failure(
            "pandoc",
            if details.is_empty() {
                "Pandoc a refusé le document.".to_owned()
            } else {
                format!("Pandoc a refusé le document : {details}")
            },
        ));
    }

    let produit = fs::read(&sortie)
        .map_err(|e| CoreError::failure("pandoc", format!("Document produit illisible : {e}")))?;
    effacer(&sortie);
    Ok(produit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pandoc_couvre_les_conversions_de_balisage() {
        assert!(handles(SourceFormat::Docx, TargetFormat::Markdown));
        assert!(handles(SourceFormat::Markdown, TargetFormat::Docx));
        assert!(handles(SourceFormat::Html, TargetFormat::Text));
    }

    #[test]
    fn pandoc_ne_touche_ni_au_pdf_ni_aux_images() {
        // Pandoc n'a pas de lecteur PDF.
        assert!(!handles(SourceFormat::Pdf, TargetFormat::Markdown));
        // Et ne produit un PDF qu'avec un moteur LaTeX, absent de la suite.
        assert!(!handles(SourceFormat::Docx, TargetFormat::Pdf));
        assert!(!handles(SourceFormat::Image, TargetFormat::Png));
        // Le .doc binaire ne lui est pas accessible non plus.
        assert!(!handles(SourceFormat::DocLegacy, TargetFormat::Docx));
    }
}
