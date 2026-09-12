//! Conversion d'images entre formats, et mise en page d'une image dans un PDF.

use derovia_core::CoreError;
use image::{ImageOutputFormat, ImageResult};
use printpdf::{Mm, PdfDocument, Pt};
use std::io::Cursor;

use crate::{model::ConvertOptions, pdf_gen::PageLayout};

/// Encode l'image chargee dans le format demande.
fn encode(input_bytes: &[u8], format: ImageOutputFormat) -> Result<Vec<u8>, CoreError> {
    let img = image::load_from_memory(input_bytes).map_err(|e| {
        CoreError::failure("decodage_image", format!("Format d'image non reconnu : {e}"))
    })?;

    let mut output = Vec::new();
    let written: ImageResult<()> = img.write_to(&mut Cursor::new(&mut output), format);
    written.map_err(|e| {
        CoreError::failure("encodage_image", format!("Erreur lors de l'encodage de l'image : {e}"))
    })?;
    Ok(output)
}

/// Reencode une image en PNG, sans perte.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le decodage ou l'encodage echoue.
pub fn to_png(input_bytes: &[u8]) -> Result<Vec<u8>, CoreError> {
    encode(input_bytes, ImageOutputFormat::Png)
}

/// Reencode une image en JPEG a la qualite demandee.
///
/// La qualite est supposee deja bornee a 1-100 par [`ConvertOptions::jpeg_quality`].
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le decodage ou l'encodage echoue.
pub fn to_jpeg(input_bytes: &[u8], quality: u8) -> Result<Vec<u8>, CoreError> {
    encode(input_bytes, ImageOutputFormat::Jpeg(quality.clamp(1, 100)))
}

/// Reencode une image en WebP.
///
/// L'encodeur embarque (libwebp, via la feature `webp-encoder`) travaille sans
/// perte : le fichier produit est fidele a l'original, souvent plus compact
/// qu'un PNG mais plus lourd qu'un JPEG de qualite moyenne.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le decodage ou l'encodage echoue.
pub fn to_webp(input_bytes: &[u8]) -> Result<Vec<u8>, CoreError> {
    encode(input_bytes, ImageOutputFormat::WebP)
}

/// Place une image seule sur une page PDF, centree et mise a l'echelle.
///
/// L'orientation suit celle de l'image : une photo plus large que haute donne
/// une page paysage. L'image occupe toute la zone disponible dans les marges,
/// sans jamais etre deformee.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si l'image est illisible ou si l'encodage PDF echoue.
#[allow(
    clippy::cast_precision_loss,
    reason = "une dimension d'image en pixels tient tres largement dans la mantisse d'un f32"
)]
pub fn image_to_pdf(input_bytes: &[u8], options: &ConvertOptions) -> Result<Vec<u8>, CoreError> {
    let img = image::load_from_memory(input_bytes).map_err(|e| {
        CoreError::failure("chargement_image", format!("Impossible de charger l'image : {e}"))
    })?;

    if img.width() == 0 || img.height() == 0 {
        return Err(CoreError::failure(
            "dimensions_image",
            "Dimensions de l'image invalides (largeur ou hauteur nulle)".to_owned(),
        ));
    }

    let image_width = img.width() as f32;
    let image_height = img.height() as f32;

    // La page reprend le format demande, en basculant en paysage si l'image
    // l'est : une photo panoramique sur une page portrait gaspille la moitie
    // de la feuille.
    let layout = PageLayout::from_options(options);
    let (page_w_mm, page_h_mm) = if image_width > image_height {
        (layout.height_mm.max(layout.width_mm), layout.height_mm.min(layout.width_mm))
    } else {
        (layout.height_mm.min(layout.width_mm), layout.height_mm.max(layout.width_mm))
    };

    let title = options.page_size.as_deref().unwrap_or("Image");
    let (doc, page, layer) = PdfDocument::new(title, Mm(page_w_mm), Mm(page_h_mm), "Image");
    let current_layer = doc.get_page(page).get_layer(layer);

    let available_w_mm = (page_w_mm - layout.margin_mm * 2.0).max(10.0);
    let available_h_mm = (page_h_mm - layout.margin_mm * 2.0).max(10.0);

    let page_w_pt = Mm(page_w_mm).into_pt().0;
    let page_h_pt = Mm(page_h_mm).into_pt().0;
    let available_w_pt = Mm(available_w_mm).into_pt().0;
    let available_h_pt = Mm(available_h_mm).into_pt().0;

    // A 72 dpi, un pixel vaut un point : l'echelle se lit directement en
    // rapport de la place disponible sur la taille naturelle de l'image.
    let scale = (available_w_pt / image_width).min(available_h_pt / image_height);
    let final_w_pt = image_width * scale;
    let final_h_pt = image_height * scale;

    printpdf::Image::from_dynamic_image(&img).add_to_layer(
        current_layer,
        printpdf::ImageTransform {
            translate_x: Some(Pt((page_w_pt - final_w_pt) / 2.0).into()),
            translate_y: Some(Pt((page_h_pt - final_h_pt) / 2.0).into()),
            scale_x: Some(scale),
            scale_y: Some(scale),
            dpi: Some(72.0),
            ..Default::default()
        },
    );

    doc.save_to_bytes()
        .map_err(|e| CoreError::failure("encodage_pdf", format!("Erreur enregistrement PDF : {e}")))
}
