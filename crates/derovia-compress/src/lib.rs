//! # derovia-compress
//!
//! Moteur de compression universel (images, PDF, archives) de la suite Derovia.
//!
//! Ce crate opere entierement hors-ligne en Rust pur, sans recours a des
//! binaires externes.
//!
//! Deux garanties valent pour tous les formats :
//!
//! - **le fichier rendu n'est jamais plus lourd que l'original.** Quand aucune
//!   reduction n'est possible, l'original est restitue tel quel et le bilan
//!   annonce 0 % — plutot qu'un fichier gonfle presente comme « compresse » ;
//! - **le nom de sortie correspond au contenu produit.** Un PNG reencode en
//!   JPEG ressort en `.jpg`, jamais en `.png`.

pub mod archive;
pub mod image_opt;
pub mod model;
pub mod pdf_opt;

pub use model::{CompressOptions, CompressResult, CompressionLevel, FileCategory};

use derovia_core::CoreError;

/// Separe un nom de fichier en radical et extension.
fn split_name(file_name: &str) -> (&str, &str) {
    match file_name.rsplit_once('.') {
        // Un nom commencant par un point est un nom cache, pas une extension.
        Some((stem, ext)) if !stem.is_empty() => (stem, ext),
        _ => (file_name, ""),
    }
}

/// Compresse et optimise n'importe quel fichier selon sa nature.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le fichier est vide ou si le traitement echoue.
#[allow(
    clippy::cast_precision_loss,
    reason = "le calcul du pourcentage de reduction n'exige pas une precision au-dela de f64"
)]
pub fn compress_file(
    file_name: &str,
    input_bytes: &[u8],
    options: &CompressOptions,
) -> Result<CompressResult, CoreError> {
    if input_bytes.is_empty() {
        return Err(CoreError::failure(
            "compression",
            "Le fichier fourni est vide (0 octet)".to_owned(),
        ));
    }

    let category = FileCategory::detect(file_name, input_bytes);
    let (stem, _) = split_name(file_name);

    let (candidate_name, candidate) = match category {
        FileCategory::Image => {
            let optimised = image_opt::compress_image(input_bytes, options)?;
            (format!("{stem}.{}", optimised.extension), optimised.data)
        }
        FileCategory::Pdf => (file_name.to_owned(), pdf_opt::compress_pdf(input_bytes)?),
        FileCategory::Generic => {
            let archived = archive::compress_to_zip(file_name, input_bytes, options.level)?;
            let zip_name = if file_name.ends_with(".zip") {
                file_name.to_owned()
            } else {
                format!("{file_name}.zip")
            };
            (zip_name, archived)
        }
    };

    // Aucune voie n'a le droit de rendre un fichier plus lourd que l'original :
    // un PDF deja optimise ou un JPEG deja compresse gonflent facilement a la
    // reecriture. Dans ce cas on restitue l'original, et le bilan dit 0 %.
    let (out_name, data) = if candidate.len() < input_bytes.len() {
        (candidate_name, candidate)
    } else {
        (file_name.to_owned(), input_bytes.to_vec())
    };

    let original_size = input_bytes.len() as u64;
    let compressed_size = data.len() as u64;
    let saved_bytes = original_size.saturating_sub(compressed_size);
    let reduction_percent = if saved_bytes > 0 {
        let pct = (saved_bytes as f64 / original_size as f64) * 100.0;
        (pct * 10.0).round() / 10.0
    } else {
        0.0
    };

    Ok(CompressResult {
        file_name: out_name,
        original_size,
        compressed_size,
        saved_bytes,
        reduction_percent,
        data,
    })
}
