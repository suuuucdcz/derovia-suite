//! Tests d'integration pour derovia-compress.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    reason = "dans les tests, paniquer est le comportement attendu en cas d'assertion fausse"
)]

use derovia_compress::{CompressOptions, CompressionLevel, FileCategory, compress_file};
use image::{ImageBuffer, Rgb};
use std::io::Cursor;
use zip::ZipArchive;

#[test]
fn category_detection_identifies_all_media_types() {
    assert_eq!(FileCategory::detect("photo.jpg", b""), FileCategory::Image);
    assert_eq!(FileCategory::detect("document.pdf", b""), FileCategory::Pdf);
    assert_eq!(FileCategory::detect("tableur.csv", b""), FileCategory::Generic);
    assert_eq!(FileCategory::detect("archive.zip", b""), FileCategory::Generic);
}

#[test]
fn generic_file_is_compressed_into_valid_zip() {
    let raw_text = "Derovia - Suite Bureautique Haute Performance. ".repeat(100);
    let raw_bytes = raw_text.as_bytes();

    let opts = CompressOptions::default();
    let result = compress_file("donnees.txt", raw_bytes, &opts).expect("compression reussie");

    assert_eq!(result.file_name, "donnees.txt.zip");
    assert!(result.compressed_size < result.original_size);
    assert!(result.saved_bytes > 0);
    assert!(result.reduction_percent > 50.0); // Le texte repetitif compresse tres fort

    // Verification que le ZIP est valide et decompressible
    let mut archive = ZipArchive::new(Cursor::new(&result.data)).expect("zip valide");
    assert_eq!(archive.len(), 1);
    let file = archive.by_index(0).expect("entree zip");
    assert_eq!(file.name(), "donnees.txt");
}

#[test]
fn image_compression_reduces_size() {
    // Creation d'une image test 256x256 avec du contenu varie
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(256, 256, |x, y| {
        Rgb([((x * 7 + y * 3) % 256) as u8, ((y * 11 + x * 5) % 256) as u8, ((x ^ y) % 256) as u8])
    });

    let mut uncompressed_png = Vec::new();
    img.write_to(&mut Cursor::new(&mut uncompressed_png), image::ImageFormat::Png)
        .expect("ecriture png ok");

    let opts = CompressOptions {
        level: CompressionLevel::Maximum,
        quality: Some(50),
        resize_percent: Some(50),
    };

    let result = compress_file("image.png", &uncompressed_png, &opts).expect("compression image");
    assert!(result.compressed_size > 0);
    assert!(
        result.compressed_size < result.original_size,
        "original: {}, compressed: {}",
        result.original_size,
        result.compressed_size
    );
    assert!(result.reduction_percent > 0.0);
}

#[test]
fn empty_file_returns_error() {
    let opts = CompressOptions::default();
    let result = compress_file("vide.bin", b"", &opts);
    assert!(result.is_err());
}
