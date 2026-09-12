//! Mise en archive ZIP des fichiers qu'aucun optimiseur dedie ne couvre.

use derovia_core::CoreError;
use std::io::{Cursor, Write};
use zip::{CompressionMethod, ZipWriter, write::FileOptions};

use crate::model::CompressionLevel;

/// Place un fichier quelconque dans une archive ZIP.
///
/// Le ZIP est toujours sans perte : le niveau demande ne change pas le resultat
/// final, seulement l'effort de recherche du compresseur. Le profil *Maximum*
/// pousse Deflate a fond ; les autres gardent le reglage d'equilibre, bien plus
/// rapide pour un gain de quelques pourcents.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si la creation du flux ZIP echoue.
pub fn compress_to_zip(
    file_name: &str,
    input_bytes: &[u8],
    level: CompressionLevel,
) -> Result<Vec<u8>, CoreError> {
    let effort = match level {
        CompressionLevel::Maximum => 9,
        CompressionLevel::Lossless | CompressionLevel::Balanced => 6,
    };

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(effort));

        zip.start_file(file_name, options)
            .map_err(|e| CoreError::failure("zip", format!("Erreur creation fichier ZIP : {e}")))?;
        zip.write_all(input_bytes)
            .map_err(|e| CoreError::failure("zip", format!("Erreur ecriture flux ZIP : {e}")))?;
        zip.finish().map_err(|e| {
            CoreError::failure("zip", format!("Erreur finalisation archive ZIP : {e}"))
        })?;
    }

    Ok(buffer)
}
