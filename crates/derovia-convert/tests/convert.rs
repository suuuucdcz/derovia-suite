//! Tests d'integration pour derovia-convert.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "dans les tests, paniquer est le comportement attendu en cas d'assertion fausse"
)]

use derovia_convert::{
    ConvertOptions, DocBlock, DocumentAST, SourceFormat, TargetFormat, convert_document, docx,
    image_doc, markdown, pdf_gen,
};
use image::{ImageBuffer, Rgb};

#[test]
fn format_detection_works_for_all_extensions() {
    assert_eq!(SourceFormat::detect("test.docx", b""), SourceFormat::Docx);
    assert_eq!(SourceFormat::detect("rapport.pdf", b""), SourceFormat::Pdf);
    assert_eq!(SourceFormat::detect("notes.md", b""), SourceFormat::Markdown);
    assert_eq!(SourceFormat::detect("contrat.doc", b""), SourceFormat::DocLegacy);
    assert_eq!(SourceFormat::detect("photo.png", b""), SourceFormat::Image);
    assert_eq!(SourceFormat::detect("page.html", b""), SourceFormat::Html);
}

#[test]
fn markdown_parsing_and_generation_roundtrip() {
    let md = "# Mon Titre\n\nPremier paragraphe de test.\n\n- Puce 1\n- Puce 2\n";
    let ast = markdown::parse_markdown(md);

    assert_eq!(ast.title.as_deref(), Some("Mon Titre"));
    assert_eq!(ast.blocks.len(), 4); // 1 heading + 1 paragraph + 2 list items

    let html = markdown::ast_to_html(&ast);
    assert!(html.contains("<h1>Mon Titre</h1>"));
    assert!(html.contains("<p>Premier paragraphe de test.</p>"));
    assert!(html.contains("<li>Puce 1</li>"));

    let txt = markdown::ast_to_text(&ast);
    assert!(txt.contains("Mon Titre"));
    assert!(txt.contains("Premier paragraphe de test."));
}

#[test]
fn docx_generation_and_parsing_roundtrip() {
    let mut ast = DocumentAST::default();
    ast.blocks.push(DocBlock::Heading { level: 1, text: "Document Genere".to_string() });
    ast.blocks
        .push(DocBlock::Paragraph("Ceci est un test de conversion DOCX en Rust pur.".to_string()));

    let docx_bytes = docx::generate_docx(&ast).expect("generation docx reussie");
    assert!(!docx_bytes.is_empty());
    assert!(docx_bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04])); // Signature ZIP

    let parsed_ast = docx::parse_docx(&docx_bytes).expect("lecture docx reussie");
    assert_eq!(parsed_ast.title.as_deref(), Some("Document Genere"));
    assert_eq!(parsed_ast.blocks.len(), 2);
}

#[test]
fn pdf_generation_produces_valid_pdf_stream() {
    let mut ast = DocumentAST::default();
    ast.blocks.push(DocBlock::Heading { level: 1, text: "Facture et Rapport".to_string() });
    ast.blocks.push(DocBlock::Paragraph(
        "Ce paragraphe verifie que le moteur printpdf produit un flux PDF conforme.".to_string(),
    ));

    let pdf_bytes =
        pdf_gen::generate_pdf(&ast, &ConvertOptions::default()).expect("generation pdf reussie");
    assert!(pdf_bytes.starts_with(b"%PDF-"));
    assert!(pdf_bytes.len() > 100);
}

#[test]
fn image_conversion_and_pdf_embedding() {
    // Creation d'une petite image PNG 16x16 de test
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(16, 16, Rgb([200, 100, 50]));
    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .expect("ecriture png ok");

    // Test conversion image -> PDF
    let pdf_bytes =
        image_doc::image_to_pdf(&png_bytes, &ConvertOptions::default()).expect("image to pdf");
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    // Test conversion PNG -> JPEG
    let jpeg_bytes = image_doc::to_jpeg(&png_bytes, 82).expect("png to jpeg");
    assert!(jpeg_bytes.starts_with(&[0xFF, 0xD8, 0xFF]));
}

#[test]
fn landscape_and_portrait_images_produce_adapted_pdf_pages() {
    // 1. Image paysage (100x50)
    let landscape_img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(100, 50, Rgb([50, 150, 250]));
    let mut landscape_png = Vec::new();
    landscape_img
        .write_to(&mut std::io::Cursor::new(&mut landscape_png), image::ImageFormat::Png)
        .expect("ecriture landscape png");

    let pdf_landscape = image_doc::image_to_pdf(&landscape_png, &ConvertOptions::default())
        .expect("paysage to pdf");
    assert!(pdf_landscape.starts_with(b"%PDF-"));

    // 2. Image portrait (50x100)
    let portrait_img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(50, 100, Rgb([250, 100, 50]));
    let mut portrait_png = Vec::new();
    portrait_img
        .write_to(&mut std::io::Cursor::new(&mut portrait_png), image::ImageFormat::Png)
        .expect("ecriture portrait png");

    let pdf_portrait = image_doc::image_to_pdf(&portrait_png, &ConvertOptions::default())
        .expect("portrait to pdf");
    assert!(pdf_portrait.starts_with(b"%PDF-"));
}

#[test]
fn pdf_to_docx_roundtrip_extracts_structure_and_styles() {
    // 1. Creation d'un document structure avec Titre, Paragraphe et Listes
    let mut initial_ast = DocumentAST::default();
    initial_ast
        .blocks
        .push(DocBlock::Heading { level: 1, text: "Rapport Financier Annuel".to_string() });
    initial_ast.blocks.push(DocBlock::Paragraph(
        "Ce document valide l'extraction fidele et la reconstitution de paragraphes en Rust pur."
            .to_string(),
    ));
    initial_ast.blocks.push(DocBlock::ListItem("Premier element valide".to_string()));
    initial_ast.blocks.push(DocBlock::ListItem("Deuxieme element valide".to_string()));

    // 2. Generation PDF
    let pdf_bytes = pdf_gen::generate_pdf(&initial_ast, &ConvertOptions::default())
        .expect("generation pdf reussie");
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    // 3. Conversion PDF -> DOCX
    let opts = ConvertOptions::default();
    let docx_res = convert_document("rapport.pdf", &pdf_bytes, TargetFormat::Docx, &opts)
        .expect("conversion pdf vers docx reussie");
    assert_eq!(docx_res.output_format, "DOCX");
    assert!(!docx_res.data.is_empty());

    // 4. Verification de l'archive DOCX (styles.xml et document.xml)
    let cursor = std::io::Cursor::new(&docx_res.data);
    let mut archive = zip::ZipArchive::new(cursor).expect("archive zip docx valide");
    assert!(archive.by_name("word/styles.xml").is_ok());
    assert!(archive.by_name("word/fontTable.xml").is_ok());
    assert!(archive.by_name("word/document.xml").is_ok());

    // 5. Lecture du DOCX produit
    let docx_ast = docx::parse_docx(&docx_res.data).expect("lecture docx");
    assert!(docx_ast.blocks.len() >= 2);
    let all_text = docx_ast
        .blocks
        .iter()
        .map(|b| match b {
            DocBlock::Heading { text, .. }
            | DocBlock::Paragraph(text)
            | DocBlock::ListItem(text) => text.as_str(),
            DocBlock::Table(_) => "",
        })
        .collect::<Vec<_>>()
        .join(" ");

    assert!(all_text.contains("Rapport Financier Annuel"));
    assert!(all_text.contains("Ce document valide"));
}

#[test]
fn full_conversion_pipeline_rejects_empty_file() {
    let opts = ConvertOptions::default();
    let res = convert_document("vide.txt", b"", TargetFormat::Pdf, &opts);
    assert!(res.is_err());
}
