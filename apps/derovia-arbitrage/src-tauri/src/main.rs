// Sans cet attribut, lancer l'application en release ouvre une fenetre de
// console noire derriere la fenetre principale.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Derovia Arbitrage — l'application de bureau.
//!
//! Ce binaire ne contient aucune regle de calcul : il se contente d'exposer le
//! moteur [`derovia_arbitrage`] a l'interface. Toute la logique metier vit dans
//! les crates de la suite, ou elle est testable sans lancer de fenetre.

use derovia_arbitrage::{
    Arbitrage, Verdict, engine,
    presets::{self, PresetCard},
};
use derovia_core::CoreError;
use serde::Serialize;

/// Le resultat complet d'une analyse, tel que l'interface le recoit.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Analysis {
    /// Le verdict et sa trajectoire.
    verdict: Verdict,
    /// Le rendement de placement qui renverserait la decision, en pourcentage.
    ///
    /// `null` quand aucun rendement plausible ne la fait basculer : la
    /// conclusion est alors robuste, et l'interface le dit.
    indifference_return: Option<f64>,
}

/// Renvoie les trois situations proposees au demarrage, hypotheses incluses.
#[tauri::command]
fn catalogue() -> Vec<PresetCard> {
    presets::catalog()
}

/// Lance un arbitrage sur les hypotheses saisies dans l'interface.
///
/// L'erreur remonte telle quelle jusqu'au frontend : son message est deja
/// redige pour etre lu par l'utilisateur.
#[tauri::command]
fn analyser(scenario: Arbitrage) -> Result<Analysis, CoreError> {
    let verdict = engine::run(&scenario)?;
    let indifference_return = engine::indifference_return(&scenario).map(|rate| rate.as_percent());
    Ok(Analysis { verdict, indifference_return })
}

/// Le dossier ou la suite installe ses moteurs externes.
fn dossier_donnees(app: &tauri::AppHandle) -> Result<std::path::PathBuf, CoreError> {
    use tauri::Manager as _;
    app.path()
        .app_data_dir()
        .map_err(|e| CoreError::failure("moteur", format!("Dossier de données introuvable : {e}")))
}

/// Convertit un document vers le format demande.
///
/// Quand Pandoc est installe et que le couple de formats est de son ressort, la
/// conversion lui est confiee : il restitue les tableaux, les notes et les
/// styles, la ou le moteur interne ne rend que la structure. Sinon, le moteur
/// interne prend le relais — l'outil reste utilisable sans rien installer.
#[tauri::command]
fn convertir_document(
    app: tauri::AppHandle,
    file_name: String,
    input_bytes: Vec<u8>,
    target_format: derovia_convert::TargetFormat,
    options: derovia_convert::ConvertOptions,
) -> Result<derovia_convert::ConversionResult, CoreError> {
    let source = derovia_convert::SourceFormat::detect(&file_name, &input_bytes);

    if derovia_engines::handles(source, target_format)
        && let Ok(base) = dossier_donnees(&app)
        && derovia_engines::pandoc_status(&base).installed
    {
        let executable = derovia_engines::install::pandoc_path(&base);
        let data = derovia_engines::run_pandoc(&executable, &input_bytes, source, target_format)?;
        let stem = std::path::Path::new(&file_name)
            .file_stem()
            .and_then(|valeur| valeur.to_str())
            .unwrap_or("document");
        return Ok(derovia_convert::ConversionResult {
            file_name: format!("{stem}.{}", target_format.extension()),
            output_format: target_format.extension().to_uppercase(),
            original_size: input_bytes.len() as u64,
            output_size: data.len() as u64,
            data,
        });
    }

    derovia_convert::convert_document(&file_name, &input_bytes, target_format, &options)
}

/// L'etat de chacun des moteurs externes, et ce qu'ils couteraient.
#[tauri::command]
fn moteurs_statut(app: tauri::AppHandle) -> Result<Vec<derovia_engines::EngineStatus>, CoreError> {
    Ok(derovia_engines::all_statuses(&dossier_donnees(&app)?))
}

/// L'avancement du telechargement d'un moteur.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progression {
    /// L'identifiant du moteur concerne.
    id: String,
    /// Les octets deja recus.
    recus: u64,
    /// Les octets attendus.
    attendus: u64,
}

/// Telecharge et installe un moteur, en publiant l'avancement.
///
/// Le travail est lourd — des dizaines de megaoctets a recevoir puis a
/// deplier — et tourne donc hors du fil principal : sinon la fenetre resterait
/// figee pendant toute la duree.
#[tauri::command]
async fn installer_moteur(
    app: tauri::AppHandle,
    id: String,
) -> Result<derovia_engines::EngineStatus, CoreError> {
    let base = dossier_donnees(&app)?;
    let spec = derovia_engines::find(&id)
        .ok_or_else(|| CoreError::failure("moteur", format!("Moteur inconnu : {id}")))?;
    let rapporteur = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        derovia_engines::install(&base, spec, |recus, attendus| {
            use tauri::Emitter as _;
            let _ = rapporteur.emit(
                "moteur://progression",
                Progression { id: spec.id.to_owned(), recus, attendus },
            );
        })
    })
    .await
    .map_err(|e| CoreError::failure("moteur", format!("Installation interrompue : {e}")))?
}

/// Compresse et optimise n'importe quel fichier (image, PDF, archive).
#[tauri::command]
fn compresser_fichier(
    file_name: String,
    input_bytes: Vec<u8>,
    options: derovia_compress::CompressOptions,
) -> Result<derovia_compress::CompressResult, CoreError> {
    derovia_compress::compress_file(&file_name, &input_bytes, &options)
}

/// Informations sur un fichier sauvegardé sur le disque.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FichierSauvegarde {
    nom: String,
    chemin: String,
    taille: u64,
}

/// Reduit un nom de fichier a un composant simple, sur pour etre concatene.
///
/// Le nom vient de l'interface, qui le tient du fichier depose par
/// l'utilisateur. Sans ce filtrage, `PathBuf::join` accepterait un chemin
/// absolu — et un nom comme `C:\Windows\System32\x.dll` ne serait pas ecrit
/// dans Telechargements mais a l'endroit indique.
fn nom_de_fichier_sur(file_name: &str) -> Result<String, CoreError> {
    let base = std::path::Path::new(file_name)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default()
        .trim();

    // Les caracteres interdits par Windows sont remplaces plutot que refuses :
    // l'utilisateur veut son fichier, pas une lecon sur NTFS.
    let assaini: String = base
        .chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') { '_' } else { c })
        .collect();

    if assaini.is_empty() || assaini == "." || assaini == ".." {
        return Err(CoreError::failure(
            "sauvegarde",
            "Le nom de fichier est inexploitable.".to_owned(),
        ));
    }
    Ok(assaini)
}

/// Enregistre un fichier directement dans le dossier Téléchargements de l'utilisateur.
#[tauri::command]
fn sauvegarder_fichier(file_name: String, data: Vec<u8>) -> Result<FichierSauvegarde, CoreError> {
    if data.is_empty() {
        return Err(CoreError::failure(
            "sauvegarde",
            "Le contenu à enregistrer est vide (0 octet)".to_owned(),
        ));
    }

    let file_name = nom_de_fichier_sur(&file_name)?;

    let download_dir = std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .map(|profil| profil.join("Downloads"))
        .ok_or_else(|| {
            CoreError::failure(
                "sauvegarde",
                "Impossible de localiser votre dossier utilisateur.".to_owned(),
            )
        })?;

    // Une erreur de creation ne doit pas etre avalee : sinon l'ecriture echoue
    // ensuite avec un message qui n'explique rien.
    std::fs::create_dir_all(&download_dir).map_err(|e| {
        CoreError::failure(
            "sauvegarde",
            format!("Impossible d'ouvrir le dossier Téléchargements : {e}"),
        )
    })?;

    let stem =
        std::path::Path::new(&file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("fichier");
    let ext = std::path::Path::new(&file_name).extension().and_then(|s| s.to_str()).unwrap_or("");

    let mut target_path = download_dir.join(&file_name);
    let mut counter = 1;
    while target_path.exists() {
        let new_name = if ext.is_empty() {
            format!("{stem} ({counter})")
        } else {
            format!("{stem} ({counter}).{ext}")
        };
        target_path = download_dir.join(new_name);
        counter += 1;
    }

    std::fs::write(&target_path, &data).map_err(|e| {
        CoreError::failure("sauvegarde", format!("Impossible d'enregistrer le fichier : {e}"))
    })?;

    let final_name =
        target_path.file_name().and_then(|n| n.to_str()).unwrap_or(&file_name).to_owned();

    let taille = data.len() as u64;
    let chemin = target_path.to_string_lossy().into_owned();

    Ok(FichierSauvegarde { nom: final_name, chemin, taille })
}

/// Ouvre l'explorateur Windows et sélectionne le fichier sauvegardé.
///
/// Le chemin est verifie avant d'etre transmis : il doit designer un fichier
/// existant du dossier Téléchargements. Passer une chaine arbitraire a une
/// ligne de commande reviendrait a laisser l'interface lancer ce qu'elle veut.
#[tauri::command]
fn ouvrir_dans_explorateur(chemin: String) -> Result<(), CoreError> {
    let cible = std::path::Path::new(&chemin);
    if !cible.is_file() {
        return Err(CoreError::failure(
            "explorateur",
            "Le fichier n'existe plus à cet emplacement.".to_owned(),
        ));
    }

    let telechargements = std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .map(|profil| profil.join("Downloads"));
    let autorise = telechargements
        .and_then(|dossier| {
            let dossier = dossier.canonicalize().ok()?;
            let cible = cible.canonicalize().ok()?;
            Some(cible.starts_with(&dossier))
        })
        .unwrap_or(false);

    if !autorise {
        return Err(CoreError::failure(
            "explorateur",
            "Seuls les fichiers du dossier Téléchargements peuvent être affichés.".to_owned(),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // `/select,` exige une ligne de commande brute ; le chemin a ete
        // canonicalise et confine ci-dessus, il ne peut plus porter de guillemet
        // qui refermerait l'argument.
        std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{}\"", cible.display()))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| {
                CoreError::failure(
                    "explorateur",
                    format!("Impossible d'ouvrir l'explorateur : {e}"),
                )
            })?;
    }

    Ok(())
}

#[allow(
    clippy::expect_used,
    reason = "si la fenetre ne s'ouvre pas, il n'y a rien a rattraper : mieux vaut le dire"
)]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            catalogue,
            analyser,
            convertir_document,
            moteurs_statut,
            installer_moteur,
            compresser_fichier,
            sauvegarder_fichier,
            ouvrir_dans_explorateur
        ])
        .run(tauri::generate_context!())
        .expect("impossible d'ouvrir la fenetre Derovia");
}
