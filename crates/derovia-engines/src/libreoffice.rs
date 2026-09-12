//! Pilotage du moteur haute fidelite, adosse a LibreOffice.
//!
//! Il apporte ce qu'aucun autre moteur de la suite ne donne :
//!
//! - la lecture du **`.doc` binaire** de Word 97-2003, un conteneur OLE que ni
//!   le moteur interne ni Pandoc ne savent ouvrir ;
//! - une **sortie PDF fidele** — mise en page, polices, images, tableaux — la
//!   ou la generation interne ne pose que du texte, et ou Pandoc exigerait une
//!   installation LaTeX complete ;
//! - les **classeurs et presentations**, hors de portee des deux autres.
//!
//! Ces conversions sont lentes : une trentaine de secondes la premiere fois, le
//! temps que le moteur batisse son profil, quelques secondes ensuite.

use std::{
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

/// Le nom de filtre que LibreOffice attend pour une sortie, s'il sait l'ecrire.
fn filtre(target: TargetFormat) -> Option<&'static str> {
    match target {
        TargetFormat::Pdf => Some("pdf"),
        TargetFormat::Docx => Some("docx"),
        TargetFormat::Html => Some("html"),
        TargetFormat::Text => Some("txt"),
        // Le Markdown et les images ne sont pas son metier : Pandoc et le
        // moteur interne s'en chargent bien mieux.
        TargetFormat::Markdown | TargetFormat::Png | TargetFormat::Jpeg | TargetFormat::Webp => {
            None
        }
    }
}

/// L'extension a donner au fichier d'entree pour que le moteur le reconnaisse.
///
/// LibreOffice choisit son lecteur d'apres l'extension du fichier qu'on lui
/// remet : un `.docx` renomme `.tmp` serait refuse.
fn extension_source(source: SourceFormat) -> Option<&'static str> {
    match source {
        SourceFormat::DocLegacy => Some("doc"),
        SourceFormat::Docx => Some("docx"),
        SourceFormat::Odt => Some("odt"),
        SourceFormat::Spreadsheet => Some("xlsx"),
        SourceFormat::Presentation => Some("pptx"),
        SourceFormat::Html => Some("html"),
        SourceFormat::Text => Some("txt"),
        // Le PDF en entree demanderait un module de dessin, et le resultat
        // serait une page d'images plutot qu'un texte exploitable.
        SourceFormat::Pdf
        | SourceFormat::Markdown
        | SourceFormat::Image
        | SourceFormat::Unknown => None,
    }
}

/// Indique si le moteur haute fidelite sait traiter ce couple source/cible.
#[must_use]
pub fn handles(source: SourceFormat, target: TargetFormat) -> bool {
    extension_source(source).is_some() && filtre(target).is_some()
}

/// Indique si ce couple **exige** le moteur haute fidelite.
///
/// Sert a proposer l'installation au bon moment : inutile de la suggerer pour
/// une conversion que les moteurs deja presents assurent tres bien.
#[must_use]
pub fn required_for(source: SourceFormat, target: TargetFormat) -> bool {
    match source {
        // Personne d'autre ne lit le .doc binaire ni les classeurs.
        SourceFormat::DocLegacy
        | SourceFormat::Spreadsheet
        | SourceFormat::Presentation
        | SourceFormat::Odt => filtre(target).is_some(),
        // Un document vers PDF : le moteur interne y arrive, mais sans styles
        // ni images. La difference est visible a l'oeil.
        SourceFormat::Docx | SourceFormat::Html => target == TargetFormat::Pdf,
        _ => false,
    }
}

/// Un dossier de travail unique.
fn dossier_temporaire() -> PathBuf {
    let marque = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duree| duree.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("derovia-lo-{marque}-{}", std::process::id()))
}

/// Convertit un document via le moteur haute fidelite.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le couple de formats n'est pas de son ressort,
/// si l'executable ne peut etre lance, ou s'il ne produit rien.
pub fn convert(
    executable: &Path,
    input_bytes: &[u8],
    source: SourceFormat,
    target: TargetFormat,
) -> Result<Vec<u8>, CoreError> {
    let (Some(extension), Some(filtre_sortie)) = (extension_source(source), filtre(target)) else {
        return Err(CoreError::failure(
            "moteur_haute_fidelite",
            "Ce couple de formats n'est pas du ressort de ce moteur.".to_owned(),
        ));
    };

    let travail = dossier_temporaire();
    let entree = travail.join(format!("document.{extension}"));
    let sortie = travail.join("sortie");
    let profil = travail.join("profil");

    fs::create_dir_all(&sortie).map_err(|e| {
        CoreError::failure("moteur_haute_fidelite", format!("Dossier de travail refusé : {e}"))
    })?;
    fs::write(&entree, input_bytes).map_err(|e| {
        CoreError::failure("moteur_haute_fidelite", format!("Document illisible : {e}"))
    })?;

    // Un profil dedie, jetable, pour chaque conversion. Sans lui, le moteur
    // partagerait le profil d'un LibreOffice installe sur la machine — et
    // refuserait de demarrer si l'utilisateur l'a ouvert au meme moment.
    let profil_uri = format!("file:///{}", profil.display().to_string().replace('\\', "/"));

    let mut commande = Command::new(executable);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        commande.creation_flags(CREATE_NO_WINDOW);
    }

    let resultat = commande
        .arg("--headless")
        .arg("--norestore")
        .arg(format!("-env:UserInstallation={profil_uri}"))
        .arg("--convert-to")
        .arg(filtre_sortie)
        .arg("--outdir")
        .arg(&sortie)
        .arg(&entree)
        .output();

    let produit = (|| {
        let sortie_commande = resultat.map_err(|e| {
            CoreError::failure(
                "moteur_haute_fidelite",
                format!("Le moteur n'a pas pu être lancé : {e}"),
            )
        })?;

        if !sortie_commande.status.success() {
            let details = String::from_utf8_lossy(&sortie_commande.stderr);
            return Err(CoreError::failure(
                "moteur_haute_fidelite",
                format!("Le moteur a refusé le document : {}", details.trim()),
            ));
        }

        // Le fichier produit porte le radical du fichier d'entree ; on le
        // retrouve plutot que de le reconstruire, le moteur ayant parfois son
        // mot a dire sur l'extension.
        let attendu = sortie.join(format!("document.{filtre_sortie}"));
        fs::read(&attendu).map_err(|e| {
            CoreError::failure(
                "moteur_haute_fidelite",
                format!("Le moteur n'a produit aucun document exploitable : {e}"),
            )
        })
    })();

    // Le dossier de travail disparait quoi qu'il arrive : chaque conversion
    // laisse sinon un profil complet de plusieurs megaoctets derriere elle.
    let _ = fs::remove_dir_all(&travail);
    produit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_moteur_couvre_ce_que_les_autres_ne_savent_pas_faire() {
        assert!(handles(SourceFormat::DocLegacy, TargetFormat::Pdf));
        assert!(handles(SourceFormat::Spreadsheet, TargetFormat::Pdf));
        assert!(handles(SourceFormat::Docx, TargetFormat::Pdf));
    }

    #[test]
    fn il_laisse_aux_autres_ce_qu_ils_font_mieux() {
        // Le Markdown est le domaine de Pandoc.
        assert!(!handles(SourceFormat::Docx, TargetFormat::Markdown));
        // Les images sont celui du moteur interne.
        assert!(!handles(SourceFormat::Image, TargetFormat::Png));
        // Et le PDF en entree n'est le domaine de personne ici.
        assert!(!handles(SourceFormat::Pdf, TargetFormat::Docx));
    }

    #[test]
    fn il_n_est_exige_que_la_ou_il_est_indispensable() {
        // Sans lui, ces formats sont inaccessibles.
        assert!(required_for(SourceFormat::DocLegacy, TargetFormat::Pdf));
        assert!(required_for(SourceFormat::Spreadsheet, TargetFormat::Pdf));
        // Un PDF fidele : les autres y arrivent, mais moins bien.
        assert!(required_for(SourceFormat::Docx, TargetFormat::Pdf));
        // Ici les moteurs deja presents suffisent largement.
        assert!(!required_for(SourceFormat::Docx, TargetFormat::Markdown));
        assert!(!required_for(SourceFormat::Markdown, TargetFormat::Docx));
    }
}
