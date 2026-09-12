//! Modeles et structures pour la conversion documentaire.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Formats sources reconnus par le convertisseur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceFormat {
    /// Document Microsoft Word moderne (.docx).
    Docx,
    /// Document Microsoft Word binaire 97-2003 (.doc).
    DocLegacy,
    /// Document PDF (.pdf).
    Pdf,
    /// Document texte balise Markdown (.md).
    Markdown,
    /// Document texte brut (.txt).
    Text,
    /// Image matricielle (PNG, JPEG, WebP, etc.).
    Image,
    /// Page HTML (.html, .htm).
    Html,
    /// Format inconnu ou non supporte.
    Unknown,
}

impl SourceFormat {
    /// Detecte le format source a partir du nom de fichier et/ou de ses octets magiques.
    #[must_use]
    pub fn detect(file_name: &str, bytes: &[u8]) -> Self {
        let ext = file_name.rsplit('.').next().map(str::to_lowercase).unwrap_or_default();

        match ext.as_str() {
            "docx" => Self::Docx,
            "doc" => Self::DocLegacy,
            "pdf" => Self::Pdf,
            "md" | "markdown" => Self::Markdown,
            "txt" => Self::Text,
            "html" | "htm" => Self::Html,
            "png" | "jpg" | "jpeg" | "webp" | "bmp" => Self::Image,
            _ => {
                // Verification par signatures d'octets magiques
                if bytes.starts_with(b"%PDF-") {
                    Self::Pdf
                } else if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
                    // Signature ZIP (potentiellement DOCX)
                    Self::Docx
                } else if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
                    // Signature OLE Compound Document (DOC legacy)
                    Self::DocLegacy
                } else if bytes.starts_with(&[0x89, b'P', b'N', b'G'])
                    || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
                    || bytes.starts_with(b"RIFF")
                {
                    Self::Image
                } else {
                    Self::Unknown
                }
            }
        }
    }
}

/// Formats cibles de conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetFormat {
    /// Format PDF vectoriel.
    Pdf,
    /// Format Markdown.
    #[serde(alias = "md")]
    Markdown,
    /// Format HTML structure.
    Html,
    /// Format Texte brut.
    #[serde(alias = "txt")]
    Text,
    /// Format Microsoft Word OpenXML (.docx).
    Docx,
    /// Image PNG.
    Png,
    /// Image JPEG.
    #[serde(alias = "jpg")]
    Jpeg,
    /// Image WebP, encodee sans perte.
    Webp,
}

impl TargetFormat {
    /// Extension standard de fichier associee a ce format.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Text => "txt",
            Self::Docx => "docx",
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }

    /// Nom lisible pour l'utilisateur.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Pdf => "PDF (.pdf)",
            Self::Markdown => "Markdown (.md)",
            Self::Html => "Page web (.html)",
            Self::Text => "Texte brut (.txt)",
            Self::Docx => "Word (.docx)",
            Self::Png => "Image PNG (.png)",
            Self::Jpeg => "Image JPEG (.jpg)",
            Self::Webp => "Image WebP (.webp)",
        }
    }
}

impl fmt::Display for TargetFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Options de configuration de la conversion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOptions {
    /// Format de page souhaité (ex: "A4", "Letter"). Par defaut "A4".
    pub page_size: Option<String>,
    /// Marge en millimetres. Par defaut 15.0 mm.
    pub margin_mm: Option<f64>,
    /// Qualite de compression d'image si conversion en image (1 a 100).
    pub image_quality: Option<u8>,
}

impl ConvertOptions {
    /// Qualite d'encodage JPEG, bornee au domaine valide.
    ///
    /// Une valeur hors bornes venue de l'interface ne doit pas atteindre
    /// l'encodeur : elle y produirait une image illisible ou une panique.
    #[must_use]
    pub fn jpeg_quality(&self) -> u8 {
        self.image_quality.unwrap_or(82).clamp(1, 100)
    }
}

/// Arbre syntaxique simplifie d'un document texte pour conversion multi-formats.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentAST {
    /// Titre principal du document si identifie.
    pub title: Option<String>,
    /// Blocs constitutifs du document.
    pub blocks: Vec<DocBlock>,
}

/// Type de bloc dans un document.
#[derive(Debug, Clone, PartialEq)]
pub enum DocBlock {
    /// Titre de section avec niveau hierarchique (1 a 6).
    Heading {
        /// Niveau hierarchique.
        level: u8,
        /// Texte du titre.
        text: String,
    },
    /// Paragraphe de texte standard.
    Paragraph(String),
    /// Element de liste a puces.
    ListItem(String),
    /// Tableau avec lignes et colonnes de texte.
    Table(Vec<Vec<String>>),
}

/// Resultat final d'une conversion de document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    /// Nom du fichier resultant (avec sa nouvelle extension).
    pub file_name: String,
    /// Format cible produit.
    pub output_format: String,
    /// Donnees binaires du fichier produit.
    pub data: Vec<u8>,
    /// Taille d'origine en octets.
    pub original_size: u64,
    /// Taille convertie en octets.
    pub output_size: u64,
}
