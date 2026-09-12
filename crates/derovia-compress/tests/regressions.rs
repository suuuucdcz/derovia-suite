//! Tests de non-regression du moteur de compression.
//!
//! Chaque test verrouille une reparation, pas une fonctionnalite.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    reason = "un test a le droit de paniquer quand une assertion est fausse"
)]

use derovia_compress::{CompressOptions, CompressionLevel, compress_file};
use image::{ImageBuffer, Rgb, Rgba};
use std::io::Cursor;

/// Encode une image RGBA en PNG.
fn png_avec_transparence() -> Vec<u8> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(64, 64, |x, y| {
        // Un damier semi-transparent : la moitie des pixels est translucide.
        let alpha = if (x / 8 + y / 8) % 2 == 0 { 0 } else { 255 };
        Rgba([200, 60, 40, alpha])
    });
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png).expect("png");
    bytes
}

/// Une image bruitee, qui ne se compresse pratiquement pas.
fn png_bruite() -> Vec<u8> {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(96, 96, |x, y| {
        let seed = x.wrapping_mul(2_654_435_761).wrapping_add(y.wrapping_mul(40_503));
        Rgb([(seed % 251) as u8, ((seed / 251) % 241) as u8, ((seed / 60_491) % 239) as u8])
    });
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png).expect("png");
    bytes
}

/// Une image transparente etait reencodee en JPEG — format sans canal alpha —
/// tout en gardant son nom `.png`. La transparence devenait du noir, sans
/// que rien ne le signale.
#[test]
fn une_image_transparente_ne_part_jamais_en_jpeg() {
    let source = png_avec_transparence();

    for niveau in [CompressionLevel::Balanced, CompressionLevel::Maximum] {
        let options = CompressOptions { level: niveau, quality: Some(40), resize_percent: None };
        let resultat = compress_file("logo.png", &source, &options).expect("compression");

        assert!(
            resultat.file_name.ends_with(".png"),
            "{niveau:?} : sortie nommee {} alors que la transparence impose le PNG",
            resultat.file_name
        );

        let relu = image::load_from_memory(&resultat.data).expect("image relisible");
        assert!(relu.color().has_alpha(), "{niveau:?} : le canal alpha a ete perdu");
    }
}

/// Le nom de sortie doit decrire le contenu reel : un PNG opaque reencode en
/// JPEG ressortait en `.png`, et plusieurs logiciels refusent alors de l'ouvrir.
#[test]
fn le_nom_de_sortie_correspond_au_format_reellement_produit() {
    let opaque: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(128, 128, |x, y| Rgb([(x * 2 % 256) as u8, (y * 2 % 256) as u8, 128]));
    let mut source = Vec::new();
    opaque.write_to(&mut Cursor::new(&mut source), image::ImageFormat::Png).expect("png");

    let options = CompressOptions {
        level: CompressionLevel::Maximum,
        quality: Some(50),
        resize_percent: None,
    };
    let resultat = compress_file("photo.png", &source, &options).expect("compression");

    // Le contenu decide du nom, dans un sens comme dans l'autre.
    let format = image::guess_format(&resultat.data).expect("format identifiable");
    let attendu = match format {
        image::ImageFormat::Jpeg => ".jpg",
        image::ImageFormat::Png => ".png",
        autre => panic!("format inattendu : {autre:?}"),
    };
    assert!(
        resultat.file_name.ends_with(attendu),
        "contenu {format:?} mais nom {}",
        resultat.file_name
    );
}

/// Aucune voie ne doit rendre un fichier plus lourd que l'original : le chemin
/// PDF s'en gardait, le chemin image non, et l'interface annoncait alors 0 %
/// en remettant un fichier gonfle.
#[test]
fn le_fichier_rendu_n_est_jamais_plus_lourd_que_l_original() {
    let bruit = png_bruite();

    for niveau in
        [CompressionLevel::Lossless, CompressionLevel::Balanced, CompressionLevel::Maximum]
    {
        let options = CompressOptions { level: niveau, quality: None, resize_percent: None };
        let resultat = compress_file("bruit.png", &bruit, &options).expect("compression");

        assert!(
            resultat.compressed_size <= resultat.original_size,
            "{niveau:?} : {} octets rendus pour {} recus",
            resultat.compressed_size,
            resultat.original_size
        );
        assert_eq!(
            resultat.compressed_size as usize,
            resultat.data.len(),
            "{niveau:?} : la taille annoncee ne correspond pas aux donnees"
        );
    }
}

/// Le bilan doit rester coherent : octets economises et pourcentage doivent
/// decouler des tailles, sans jamais annoncer un gain inexistant.
#[test]
fn le_bilan_de_compression_est_coherent() {
    let texte = "Derovia, suite d'outils. ".repeat(200);
    let resultat = compress_file("notes.txt", texte.as_bytes(), &CompressOptions::default())
        .expect("compression");

    assert_eq!(resultat.saved_bytes, resultat.original_size - resultat.compressed_size);
    assert!(resultat.reduction_percent > 0.0);
    assert!(resultat.file_name.ends_with(".zip"));
}

/// Une qualite hors bornes venue de l'interface atteignait l'encodeur JPEG.
#[test]
fn une_qualite_hors_bornes_ne_fait_pas_echouer_la_compression() {
    let opaque: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(32, 32, Rgb([10, 20, 30]));
    let mut source = Vec::new();
    opaque.write_to(&mut Cursor::new(&mut source), image::ImageFormat::Png).expect("png");

    for qualite in [0u8, 1, 100, 255] {
        let options = CompressOptions {
            level: CompressionLevel::Balanced,
            quality: Some(qualite),
            resize_percent: None,
        };
        assert!(
            compress_file("a.png", &source, &options).is_ok(),
            "qualite {qualite} refusee alors qu'elle doit etre ramenee dans le domaine"
        );
    }
}

/// Un redimensionnement ne doit jamais produire une dimension nulle.
#[test]
fn un_redimensionnement_extreme_reste_valide() {
    let opaque: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(40, 10, Rgb([90, 90, 90]));
    let mut source = Vec::new();
    opaque.write_to(&mut Cursor::new(&mut source), image::ImageFormat::Png).expect("png");

    let options = CompressOptions {
        level: CompressionLevel::Maximum,
        quality: Some(60),
        resize_percent: Some(1),
    };
    let resultat = compress_file("mince.png", &source, &options).expect("compression");
    let relu = image::load_from_memory(&resultat.data).expect("image relisible");
    assert!(relu.width() >= 1 && relu.height() >= 1);
}

/// Le contrat entre l'interface et le moteur.
///
/// Cette liste doit rester alignee sur les valeurs du menu « Niveau
/// d'optimisation », dans `apps/derovia-arbitrage/src/workspace-compress.ts`.
#[test]
fn chaque_niveau_du_menu_correspond_a_un_profil_du_moteur() {
    for valeur in ["balanced", "lossless", "maximum"] {
        let niveau: CompressionLevel =
            serde_json::from_value(serde_json::Value::String(valeur.to_owned()))
                .unwrap_or_else(|e| panic!("l'interface propose « {valeur} », refuse : {e}"));
        assert!((1..=100).contains(&niveau.default_quality()));
    }
}

/// Les options envoyees par l'interface doivent se deserialiser telles quelles.
#[test]
fn les_options_de_l_interface_sont_acceptees() {
    let envoye = serde_json::json!({ "level": "maximum", "quality": 80, "resizePercent": 50 });
    let options: CompressOptions = serde_json::from_value(envoye).expect("options acceptees");
    assert_eq!(options.level, CompressionLevel::Maximum);
    assert_eq!(options.quality, Some(80));
    assert_eq!(options.resize_percent, Some(50));
}
