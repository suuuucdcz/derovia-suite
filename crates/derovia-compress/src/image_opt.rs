//! Optimisation et compression des images matricielles.

use derovia_core::CoreError;
use image::ImageOutputFormat;
use std::io::Cursor;

use crate::model::{CompressOptions, CompressionLevel};

/// Une image optimisee, avec l'extension qui correspond reellement a son contenu.
#[derive(Debug, Clone)]
pub struct OptimisedImage {
    /// Les octets encodes.
    pub data: Vec<u8>,
    /// L'extension du format produit, sans le point.
    ///
    /// Elle peut differer de celle du fichier d'origine : reencoder un PNG en
    /// JPEG sans renommer le fichier produirait un `.png` dont le contenu est
    /// du JPEG, que plusieurs logiciels refusent d'ouvrir.
    pub extension: &'static str,
}

/// Optimise et compresse des octets d'image selon les options choisies.
///
/// Le format de sortie n'est pas celui d'entree mais celui qui convient :
///
/// - en niveau *sans perte*, le PNG conserve tout ;
/// - une image **avec transparence** reste en PNG quoi qu'il arrive, car le
///   JPEG n'a pas de canal alpha et aplatirait les zones transparentes en noir ;
/// - sinon le JPEG, bien plus compact sur une photographie.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si l'image est illisible ou ne peut etre encodee.
pub fn compress_image(
    input_bytes: &[u8],
    options: &CompressOptions,
) -> Result<OptimisedImage, CoreError> {
    let mut img = image::load_from_memory(input_bytes).map_err(|e| {
        CoreError::failure("chargement_image", format!("Format d'image non reconnu : {e}"))
    })?;

    if let Some(percent) = options.resize_percent
        && (1..100).contains(&percent)
    {
        let factor = f64::from(percent) / 100.0;
        img = img.resize(
            scale_dimension(img.width(), factor),
            scale_dimension(img.height(), factor),
            image::imageops::FilterType::Lanczos3,
        );
    }

    // Le JPEG n'a pas de canal alpha : convertir une image transparente le
    // remplacerait par du noir, sans que l'utilisateur soit prevenu.
    let has_alpha = img.color().has_alpha();
    let keep_png = has_alpha || options.level == CompressionLevel::Lossless;

    let mut data = Vec::new();
    let mut cursor = Cursor::new(&mut data);

    let extension = if keep_png {
        img.write_to(&mut cursor, ImageOutputFormat::Png).map_err(|e| {
            CoreError::failure("encodage_png", format!("Erreur encodage PNG : {e}"))
        })?;
        "png"
    } else {
        let quality =
            options.quality.unwrap_or_else(|| options.level.default_quality()).clamp(1, 100);
        img.write_to(&mut cursor, ImageOutputFormat::Jpeg(quality)).map_err(|e| {
            CoreError::failure("encodage_jpeg", format!("Erreur encodage JPEG : {e}"))
        })?;
        "jpg"
    };

    Ok(OptimisedImage { data, extension })
}

/// Applique un facteur d'echelle a une dimension, en restant dans les bornes utiles.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "le produit d'une dimension u32 par un facteur de 0 a 1 reste dans u32"
)]
fn scale_dimension(value: u32, factor: f64) -> u32 {
    let scaled = (f64::from(value) * factor).round();
    (scaled.max(1.0) as u32).max(1)
}
