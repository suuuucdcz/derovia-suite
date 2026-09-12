//! Tests de non-regression.
//!
//! Chaque test de ce fichier correspond a un defaut constate sur une version
//! livree. Ils ne verifient pas des fonctionnalites, ils verrouillent des
//! reparations : si l'un d'eux repasse au rouge, un bug connu est revenu.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    reason = "un test a le droit de paniquer quand une assertion est fausse"
)]

use derovia_convert::{
    ConvertOptions, DocBlock, SourceFormat, TargetFormat, convert_document, docx, html, markdown,
    pdf_gen,
};

/// Extrait le texte du premier bloc, quel qu'en soit le type.
fn premier_texte(ast: &derovia_convert::DocumentAST) -> String {
    match ast.blocks.first() {
        Some(
            DocBlock::Heading { text, .. } | DocBlock::Paragraph(text) | DocBlock::ListItem(text),
        ) => text.clone(),
        autre => panic!("aucun bloc textuel : {autre:?}"),
    }
}

/// Les caracteres echappes en XML etaient purement supprimes a la relecture.
///
/// quick-xml n'inclut pas les entites dans les evenements `Text` : il emet un
/// `GeneralRef` distinct. Les ignorer faisait lire « Service R&D » en
/// « Service RD » — y compris dans les fichiers produits par Word lui-meme,
/// qui echappe systematiquement `&` et `<`.
#[test]
fn les_caracteres_echappes_survivent_a_un_aller_retour_docx() {
    let cas = [
        "Service R&D",
        "5 < 6 et 7 > 2",
        "Il dit \"bonjour\"",
        "L'usine d'à côté",
        "Résumé & coût",
        "Tableau 1 < 2 & 3 > 0",
    ];

    for source in cas {
        let ast = markdown::parse_markdown(&format!("{source}\n"));
        let bytes = docx::generate_docx(&ast).expect("generation docx");
        let relu = docx::parse_docx(&bytes).expect("relecture docx");
        assert_eq!(premier_texte(&relu), source, "texte altere par l'aller-retour docx");
    }
}

/// La puce etait ecrite dans le texte du paragraphe au lieu d'etre une vraie
/// liste Word : elle se retrouvait collee au contenu a la relecture.
#[test]
fn une_liste_docx_ne_colle_pas_sa_puce_au_texte() {
    let ast = markdown::parse_markdown("- Premier\n- Second\n");
    let bytes = docx::generate_docx(&ast).expect("generation docx");
    let relu = docx::parse_docx(&bytes).expect("relecture docx");

    assert_eq!(relu.blocks.len(), 2);
    for bloc in &relu.blocks {
        let DocBlock::ListItem(texte) = bloc else { panic!("bloc inattendu : {bloc:?}") };
        assert!(!texte.starts_with('•'), "la puce est entree dans le texte : {texte:?}");
    }
    assert_eq!(markdown::ast_to_markdown(&relu), "- Premier\n- Second\n");
}

/// L'archive doit declarer la numerotation, sinon Word affiche des paragraphes
/// indentes sans puce.
#[test]
fn le_docx_produit_declare_sa_numerotation() {
    let ast = markdown::parse_markdown("- Seul\n");
    let bytes = docx::generate_docx(&ast).expect("generation docx");
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("archive docx valide");
    assert!(archive.by_name("word/numbering.xml").is_ok(), "numbering.xml absent");
}

/// `rsplit(...).next_back()` remontait jusqu'au premier point : un nom compose
/// perdait tout ce qui suivait.
#[test]
fn un_nom_de_fichier_a_plusieurs_points_garde_son_radical() {
    let resultat = convert_document(
        "mon.rapport.final.md",
        b"# Test\n",
        TargetFormat::Text,
        &ConvertOptions::default(),
    )
    .expect("conversion");
    assert_eq!(resultat.file_name, "mon.rapport.final.txt");
}

/// Le `.doc` binaire etait « lu » par balayage d'octets : le resultat melait du
/// contenu OLE au texte et perdait tous les accents. Mieux vaut refuser.
#[test]
fn un_doc_binaire_est_refuse_avec_une_consigne_utile() {
    // Signature OLE d'un document Word 97-2003.
    let mut faux_doc = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    faux_doc.extend_from_slice(&[0u8; 600]);

    let erreur =
        convert_document("contrat.doc", &faux_doc, TargetFormat::Pdf, &ConvertOptions::default())
            .expect_err("le .doc doit etre refuse");

    let message = erreur.to_string();
    assert!(message.contains(".docx"), "le message doit indiquer la marche a suivre : {message}");
}

/// Le HTML etait passe au lecteur Markdown apres un simple remplacement de
/// `<br>` et `</p>` : toutes les autres balises ressortaient dans le texte.
#[test]
fn le_html_est_reellement_analyse_et_non_recopie() {
    let source = r"<html><body><h1>Titre&nbsp;principal</h1>
        <p>Un <b>paragraphe</b> avec R&amp;D et 5 &lt; 6.</p>
        <ul><li>Premier</li><li>Second</li></ul>
        <script>var x = '<p>piege</p>';</script></body></html>";

    let ast = html::parse_html(source);
    let rendu = markdown::ast_to_text(&ast);

    // Le texte peut legitimement contenir « < » : c'est le decodage de `&lt;`.
    // Ce qu'il ne doit plus contenir, ce sont des balises.
    for balise in ["<h1>", "<p>", "<b>", "<ul>", "<li>", "</p>", "<body>"] {
        assert!(!rendu.contains(balise), "balise {balise} restee dans le texte : {rendu:?}");
    }
    assert!(rendu.contains("R&D"), "entite non decodee : {rendu:?}");
    assert!(rendu.contains("5 < 6"), "entite non decodee : {rendu:?}");
    assert!(!rendu.contains("piege"), "le contenu de <script> a ete conserve");
    assert_eq!(ast.title.as_deref(), Some("Titre\u{a0}principal"));
    assert_eq!(ast.blocks.iter().filter(|b| matches!(b, DocBlock::ListItem(_))).count(), 2);
}

/// Les accents doivent traverser la generation PDF puis la re-extraction.
#[test]
fn les_accents_survivent_a_un_aller_retour_pdf() {
    let ast = markdown::parse_markdown("# Résumé\n\nL'élève a reçu 12 % très tôt.\n");
    let pdf = pdf_gen::generate_pdf(&ast, &ConvertOptions::default()).expect("generation pdf");

    let texte = convert_document("a.pdf", &pdf, TargetFormat::Text, &ConvertOptions::default())
        .expect("pdf vers texte");
    let rendu = String::from_utf8_lossy(&texte.data).to_string();

    assert!(rendu.contains("Résumé"), "accents perdus : {rendu:?}");
    assert!(rendu.contains("L'élève a reçu"), "apostrophe ou accents perdus : {rendu:?}");
}

/// Un binaire non reconnu etait interprete comme du texte : la conversion
/// « reussissait » en produisant une suite de caracteres de remplacement.
#[test]
fn un_binaire_inconnu_est_refuse_plutot_que_transcrit() {
    let binaire = [0x00, 0x01, 0x02, 0xFF, 0xFE, 0x7F, 0x80, 0x90];
    let erreur =
        convert_document("inconnu.bin", &binaire, TargetFormat::Text, &ConvertOptions::default())
            .expect_err("un binaire inconnu doit etre refuse");
    assert!(erreur.to_string().contains("reconnu"), "message inattendu : {erreur}");
}

/// Le WebP etait propose comme format cible alors qu'aucun encodeur n'etait
/// compile : le bouton echouait a tous les coups. L'encodeur libwebp est
/// desormais embarque, et ce test verifie qu'il produit un fichier reel plutot
/// que de se fier a la presence d'une option dans un menu.
#[test]
fn le_webp_est_reellement_encode_et_relisible() {
    assert_eq!(SourceFormat::detect("photo.webp", b""), SourceFormat::Image);

    let source: image::RgbImage = image::ImageBuffer::from_fn(24, 24, |x, y| {
        image::Rgb([(x * 10) as u8, (y * 10) as u8, 64])
    });
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(source)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("png de depart");

    let resultat =
        convert_document("photo.png", &png, TargetFormat::Webp, &ConvertOptions::default())
            .expect("le WebP doit etre produit, pas refuse");

    assert_eq!(resultat.file_name, "photo.webp");
    assert!(resultat.data.starts_with(b"RIFF"), "en-tete WebP absente");
    assert_eq!(&resultat.data[8..12], b"WEBP", "le conteneur n'est pas du WebP");

    let relu = image::load_from_memory(&resultat.data).expect("le WebP produit doit se relire");
    assert_eq!((relu.width(), relu.height()), (24, 24));
}

/// Les options etaient declarees puis jamais lues. Elles doivent maintenant
/// changer quelque chose au document produit.
#[test]
fn les_options_de_page_influencent_le_pdf() {
    let ast = markdown::parse_markdown("Un paragraphe de test suffisamment long pour occuper.\n");

    let a4 = pdf_gen::generate_pdf(&ast, &ConvertOptions::default()).expect("pdf a4");
    let a3 = pdf_gen::generate_pdf(
        &ast,
        &ConvertOptions { page_size: Some("A3".to_owned()), ..ConvertOptions::default() },
    )
    .expect("pdf a3");

    assert_ne!(a4, a3, "le format de page demande n'a eu aucun effet");
}

/// Une qualite hors bornes venue de l'interface ne doit pas atteindre l'encodeur.
#[test]
fn une_qualite_hors_bornes_est_ramenee_dans_le_domaine_valide() {
    let zero = ConvertOptions { image_quality: Some(0), ..ConvertOptions::default() };
    let enorme = ConvertOptions { image_quality: Some(255), ..ConvertOptions::default() };
    assert_eq!(zero.jpeg_quality(), 1);
    assert_eq!(enorme.jpeg_quality(), 100);
}

/// Le contrat entre l'interface et le moteur : les chaines envoyees doivent se
/// deserialiser en formats reels.
///
/// C'est precisement la ou le WebP echouait : l'option existait dans le menu,
/// le moteur l'acceptait, et l'echec n'arrivait qu'a l'encodage. Cette liste
/// doit rester alignee sur `CIBLES` dans `apps/derovia-arbitrage/src/workspace-convert.ts`.
#[test]
fn chaque_valeur_du_menu_correspond_a_un_format_du_moteur() {
    let valeurs_du_menu = ["pdf", "docx", "markdown", "html", "text", "png", "jpeg", "webp"];

    for valeur in valeurs_du_menu {
        let format: TargetFormat = serde_json::from_value(serde_json::Value::String(
            valeur.to_owned(),
        ))
        .unwrap_or_else(|erreur| {
            panic!("l'interface propose « {valeur} », refuse par le moteur : {erreur}")
        });

        // Et l'extension produite doit correspondre a ce que l'utilisateur a choisi.
        let attendue = match valeur {
            "markdown" => "md",
            "text" => "txt",
            "jpeg" => "jpg",
            autre => autre,
        };
        assert_eq!(format.extension(), attendue, "extension inattendue pour « {valeur} »");
    }
}
