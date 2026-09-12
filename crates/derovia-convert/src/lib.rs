//! # derovia-convert
//!
//! Moteur de conversion documentaire et d'images de la suite Derovia.
//!
//! Ce crate opere entierement hors-ligne en Rust pur, sans recours a des
//! binaires externes (pas de LibreOffice, ni de Python, ni de runtime tiers).
//!
//! ## Formats
//!
//! | Sens | Formats |
//! |---|---|
//! | Entree | `.docx`, `.pdf`, `.md`, `.txt`, `.html`, images PNG / JPEG / WebP / BMP |
//! | Sortie | `.pdf`, `.docx`, `.md`, `.html`, `.txt`, images PNG / JPEG / WebP |
//!
//! Une absence est volontaire, et signalee a l'utilisateur plutot que contournee
//! par un resultat approximatif : le **`.doc`** binaire de Word 97-2003 est un
//! conteneur OLE dont la lecture fiable demande un analyseur dedie. Le
//! convertisseur le refuse avec une consigne claire, au lieu d'en extraire un
//! texte mutile.
//!
//! Le WebP en sortie est encode sans perte par libwebp : fidele a l'original,
//! souvent plus compact qu'un PNG, plus lourd qu'un JPEG de qualite moyenne.

pub mod docx;
pub mod html;
pub mod image_doc;
pub mod markdown;
pub mod model;
pub mod pdf_extract;
pub mod pdf_gen;

pub use model::{
    ConversionResult, ConvertOptions, DocBlock, DocumentAST, SourceFormat, TargetFormat,
};

use derovia_core::CoreError;

/// Isole le radical d'un nom de fichier, extension exclue.
///
/// `rapport.final.docx` donne `rapport.final` : seule la derniere extension
/// tombe. Couper au premier point amputerait la moitie du nom.
fn file_stem(file_name: &str) -> &str {
    match file_name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => file_name,
    }
}

/// Convertit un fichier d'entree vers le format cible demande.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le format d'entree n'est pas exploitable, s'il
/// est corrompu, ou si le couple source/cible n'a pas de sens.
pub fn convert_document(
    file_name: &str,
    input_bytes: &[u8],
    target: TargetFormat,
    options: &ConvertOptions,
) -> Result<ConversionResult, CoreError> {
    if input_bytes.is_empty() {
        return Err(CoreError::failure(
            "conversion",
            "Le fichier fourni est vide (0 octet)".to_owned(),
        ));
    }

    let source = SourceFormat::detect(file_name, input_bytes);
    let out_file_name = format!("{}.{}", file_stem(file_name), target.extension());

    let data = if source == SourceFormat::Image {
        convert_from_image(input_bytes, target, options)?
    } else {
        let ast = extract_document(source, input_bytes)?;
        render_document(&ast, target, options)?
    };

    Ok(ConversionResult {
        file_name: out_file_name,
        output_format: target.extension().to_uppercase(),
        original_size: input_bytes.len() as u64,
        output_size: data.len() as u64,
        data,
    })
}

/// Traite les entrees image, qui ne passent pas par l'arbre documentaire.
fn convert_from_image(
    input_bytes: &[u8],
    target: TargetFormat,
    options: &ConvertOptions,
) -> Result<Vec<u8>, CoreError> {
    match target {
        TargetFormat::Pdf => image_doc::image_to_pdf(input_bytes, options),
        TargetFormat::Png => image_doc::to_png(input_bytes),
        TargetFormat::Jpeg => image_doc::to_jpeg(input_bytes, options.jpeg_quality()),
        TargetFormat::Webp => image_doc::to_webp(input_bytes),
        TargetFormat::Docx | TargetFormat::Markdown | TargetFormat::Html | TargetFormat::Text => {
            Err(CoreError::failure(
                "conversion",
                "Une image ne contient pas de texte a transposer. Choisissez PDF, PNG, JPEG ou WebP."
                    .to_owned(),
            ))
        }
    }
}

/// Construit l'arbre documentaire a partir du format source.
fn extract_document(source: SourceFormat, input_bytes: &[u8]) -> Result<DocumentAST, CoreError> {
    match source {
        SourceFormat::Docx => docx::parse_docx(input_bytes),
        SourceFormat::Pdf => pdf_extract::extract_pdf_text(input_bytes),
        SourceFormat::Html => Ok(html::parse_html(&String::from_utf8_lossy(input_bytes))),
        SourceFormat::Markdown | SourceFormat::Text => {
            Ok(markdown::parse_markdown(&String::from_utf8_lossy(input_bytes)))
        }
        SourceFormat::DocLegacy => Err(CoreError::failure(
            "format_non_supporte",
            "Le format Word 97-2003 (.doc) n'est pas pris en charge. Ouvrez le document \
             dans Word et enregistrez-le en .docx, puis relancez la conversion."
                .to_owned(),
        )),
        // Ces formats bureautiques demandent le moteur haute fidelite. Le dire
        // vaut mieux que d'en tirer une approximation.
        SourceFormat::Odt | SourceFormat::Spreadsheet | SourceFormat::Presentation => {
            Err(CoreError::failure(
                "moteur_requis",
                "Ce format demande le moteur haute fidélité. Installez-le depuis \
                 les options du convertisseur."
                    .to_owned(),
            ))
        }
        // Traiter un binaire inconnu comme du texte produirait une suite de
        // caracteres de remplacement, presentee comme une conversion reussie.
        SourceFormat::Image | SourceFormat::Unknown => Err(CoreError::failure(
            "format_non_supporte",
            "Ce type de fichier n'est pas reconnu par le convertisseur.".to_owned(),
        )),
    }
}

/// Produit le fichier final a partir de l'arbre documentaire.
fn render_document(
    ast: &DocumentAST,
    target: TargetFormat,
    options: &ConvertOptions,
) -> Result<Vec<u8>, CoreError> {
    match target {
        TargetFormat::Pdf => pdf_gen::generate_pdf(ast, options),
        TargetFormat::Docx => docx::generate_docx(ast),
        TargetFormat::Markdown => Ok(markdown::ast_to_markdown(ast).into_bytes()),
        TargetFormat::Html => Ok(markdown::ast_to_html(ast).into_bytes()),
        TargetFormat::Text => Ok(markdown::ast_to_text(ast).into_bytes()),
        TargetFormat::Png | TargetFormat::Jpeg | TargetFormat::Webp => Err(CoreError::failure(
            "conversion",
            "Transformer un document en image demanderait un moteur de rendu que la \
             suite n'embarque pas. Choisissez PDF, Word, Markdown, HTML ou texte."
                .to_owned(),
        )),
    }
}
