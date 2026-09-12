//! Tests du pilotage de Pandoc.
//!
//! Pandoc pese 223 Mo : on ne le telecharge pas depuis la suite de tests. Les
//! tests qui ont besoin de l'executable le cherchent dans la variable
//! d'environnement `DEROVIA_PANDOC` et s'abstiennent poliment s'il est absent,
//! plutot que d'echouer sur une machine qui ne l'a pas.
//!
//! ```sh
//! DEROVIA_PANDOC=/chemin/vers/pandoc.exe cargo test -p derovia-engines
//! ```

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "un test a le droit de paniquer quand une assertion est fausse"
)]

use std::path::PathBuf;

use derovia_convert::{SourceFormat, TargetFormat};
use derovia_engines::pandoc;

/// L'executable Pandoc, quand la machine en dispose.
fn executable() -> Option<PathBuf> {
    let chemin = PathBuf::from(std::env::var_os("DEROVIA_PANDOC")?);
    chemin.is_file().then_some(chemin)
}

const SOURCE: &str = "# Résumé du dossier\n\n\
     L'élève a reçu 12 % ; c'est l'« essentiel » chez R&D et 5 < 6.\n\n\
     - Coût déjà réglé\n\
     - Où ça ?\n\n\
     | Poste | Montant |\n\
     |---|---|\n\
     | Loyer | 1 100 € |\n";

#[test]
fn pandoc_repond_et_annonce_sa_version() {
    let Some(chemin) = executable() else { return };
    let version = pandoc::version(&chemin).expect("Pandoc doit annoncer sa version");
    assert!(version.starts_with("pandoc"), "version inattendue : {version}");
}

/// Le point qui justifie Pandoc : ce que le moteur interne ne sait pas faire.
#[test]
fn un_aller_retour_markdown_docx_preserve_accents_et_tableaux() {
    let Some(chemin) = executable() else { return };

    let docx =
        pandoc::run_pandoc(&chemin, SOURCE.as_bytes(), SourceFormat::Markdown, TargetFormat::Docx)
            .expect("markdown vers docx");
    assert!(docx.starts_with(&[0x50, 0x4B, 0x03, 0x04]), "le .docx doit etre une archive ZIP");

    let retour = pandoc::run_pandoc(&chemin, &docx, SourceFormat::Docx, TargetFormat::Markdown)
        .expect("docx vers markdown");
    let texte = String::from_utf8_lossy(&retour);

    for attendu in ["Résumé", "L'élève", "R&D", "Coût déjà réglé", "Où ça ?"] {
        assert!(texte.contains(attendu), "« {attendu} » perdu : {texte}");
    }
    // Le tableau doit survivre — c'est precisement ce que le moteur interne perd.
    assert!(texte.contains("Loyer") && texte.contains("1 100"), "tableau perdu : {texte}");
}

#[test]
fn un_document_illisible_remonte_le_message_de_pandoc() {
    let Some(chemin) = executable() else { return };
    let erreur = pandoc::run_pandoc(
        &chemin,
        b"ceci n'est pas une archive docx",
        SourceFormat::Docx,
        TargetFormat::Markdown,
    )
    .expect_err("un faux .docx doit etre refuse");
    assert!(erreur.to_string().contains("Pandoc"), "message inattendu : {erreur}");
}

#[test]
fn un_couple_hors_perimetre_est_refuse_sans_lancer_le_binaire() {
    // Aucun executable n'est necessaire : le refus est decide en amont.
    let erreur = pandoc::run_pandoc(
        &PathBuf::from("pandoc-inexistant.exe"),
        b"%PDF-1.4",
        SourceFormat::Pdf,
        TargetFormat::Markdown,
    )
    .expect_err("Pandoc ne lit pas le PDF");
    assert!(erreur.to_string().contains("ressort"), "message inattendu : {erreur}");
}

/// L'installation complete : reseau, verification d'empreinte, extraction.
///
/// Marque `ignore` parce qu'elle telecharge reellement 40 Mo ; elle n'a pas sa
/// place dans une execution ordinaire de la suite de tests.
///
/// ```sh
/// cargo test -p derovia-engines -- --ignored --nocapture
/// ```
#[test]
#[ignore = "telecharge 40 Mo depuis GitHub"]
fn l_installation_complete_aboutit_a_un_pandoc_fonctionnel() {
    let base = std::env::temp_dir().join(format!("derovia-install-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);

    let avant = derovia_engines::pandoc_status(&base);
    assert!(!avant.installed, "le dossier de test doit partir vide");

    let mut dernier = 0_u64;
    let statut = derovia_engines::install_pandoc(&base, |recus, attendus| {
        assert!(recus <= attendus.max(recus), "progression incoherente");
        dernier = recus;
    })
    .expect("installation reussie");

    assert!(statut.installed, "Pandoc doit repondre apres installation");
    assert!(dernier > 1_000_000, "la progression doit avoir ete rapportee");

    // Et il doit convertir pour de vrai, pas seulement exister sur le disque.
    let executable = derovia_engines::install::pandoc_path(&base);
    let docx = pandoc::run_pandoc(
        &executable,
        SOURCE.as_bytes(),
        SourceFormat::Markdown,
        TargetFormat::Docx,
    )
    .expect("conversion apres installation");
    assert!(docx.starts_with(&[0x50, 0x4B, 0x03, 0x04]));

    let _ = std::fs::remove_dir_all(&base);
}
