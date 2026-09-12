//! Optimisation et recompression des flux de donnees PDF.

use derovia_core::CoreError;
use lopdf::Document;

/// Compresse et optimise les flux d'objets d'un document PDF.
///
/// Le gain porte sur la structure du fichier — flux recompresses, objets
/// orphelins supprimes. Les images incorporees ne sont pas reencodees : sur un
/// PDF essentiellement compose de scans, la reduction reste donc modeste.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le PDF est corrompu ou protege par mot de passe.
pub fn compress_pdf(input_bytes: &[u8]) -> Result<Vec<u8>, CoreError> {
    let mut doc = Document::load_mem(input_bytes).map_err(|e| {
        CoreError::failure("chargement_pdf", format!("Impossible de charger le PDF : {e}"))
    })?;

    // Un PDF chiffre se charge sans erreur, mais ses flux restent illisibles :
    // les recompresser produirait un fichier corrompu presente comme valide.
    if doc.trailer.get(b"Encrypt").is_ok() {
        return Err(CoreError::failure(
            "pdf_protege",
            "Ce PDF est protege par mot de passe : il doit etre deverrouille avant compression."
                .to_owned(),
        ));
    }

    // Compression des flux d'objets sans compression
    doc.compress();

    // Suppression des objets et dictionnaires orphelins non references
    doc.prune_objects();

    let mut output = Vec::with_capacity(input_bytes.len());
    doc.save_to(&mut output).map_err(|e| {
        CoreError::failure(
            "ecriture_pdf",
            format!("Impossible de re-ecrire le PDF compresse : {e}"),
        )
    })?;

    // Le garde-fou « jamais plus lourd que l'original » est applique une seule
    // fois, dans `compress_file`, pour que toutes les voies suivent la meme regle.
    Ok(output)
}
