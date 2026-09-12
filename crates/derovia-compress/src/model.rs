//! Modeles et structures pour la compression multi-formats.

use serde::{Deserialize, Serialize};

/// Niveau de compression souhaite par l'utilisateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionLevel {
    /// Sans perte de qualite : recompression des flux et suppression des metadonnees inutiles.
    Lossless,
    /// Equilibre optimal entre taille et qualite visuelle.
    #[default]
    Balanced,
    /// Reduction maximale de poids.
    Maximum,
}

impl CompressionLevel {
    /// Qualite d'encodage par defaut (1 a 100) associee au niveau.
    #[must_use]
    pub fn default_quality(self) -> u8 {
        match self {
            Self::Lossless => 100,
            Self::Balanced => 80,
            Self::Maximum => 60,
        }
    }
}

/// Type de fichier detecte pour l'optimisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCategory {
    /// Image (JPEG, PNG, WebP, BMP, etc.).
    Image,
    /// Document PDF.
    Pdf,
    /// Tout autre type de fichier ou archive binaire.
    Generic,
}

impl FileCategory {
    /// Identifie la categorie de fichier a optimiser.
    #[must_use]
    pub fn detect(file_name: &str, bytes: &[u8]) -> Self {
        let ext = file_name.rsplit('.').next().map(str::to_lowercase).unwrap_or_default();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "bmp" => Self::Image,
            "pdf" => Self::Pdf,
            _ => {
                if bytes.starts_with(b"%PDF-") {
                    Self::Pdf
                } else if bytes.starts_with(&[0x89, b'P', b'N', b'G'])
                    || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
                {
                    Self::Image
                } else {
                    Self::Generic
                }
            }
        }
    }
}

/// Parametres de compression transmis par l'interface.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressOptions {
    /// Profil de compression.
    pub level: CompressionLevel,
    /// Qualite personnalisee (1 a 100).
    pub quality: Option<u8>,
    /// Pourcentage de redimensionnement pour les images (ex: 75 pour 75%).
    pub resize_percent: Option<u8>,
}

/// Bilan et donnees d'une compression.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressResult {
    /// Nom final du fichier (peut ajouter .zip si archive).
    pub file_name: String,
    /// Taille d'origine en octets.
    pub original_size: u64,
    /// Taille apres compression en octets.
    pub compressed_size: u64,
    /// Nombre d'octets economises (0 si le fichier n'a pu etre reduit).
    pub saved_bytes: u64,
    /// Pourcentage de reduction (ex: 45.2 pour -45.2%).
    pub reduction_percent: f64,
    /// Donnees binaires du fichier compresse.
    pub data: Vec<u8>,
}
