//! Analyse et generation de documents Microsoft Word OpenXML (.docx).

use std::io::{Cursor, Read, Write};
use zip::{ZipArchive, ZipWriter, write::FileOptions};

use derovia_core::CoreError;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::model::{DocBlock, DocumentAST};

/// Echappe les caracteres speciaux dans un contenu textuel XML.
///
/// Seuls `&`, `<` et `>` le sont : ce sont les seuls que la specification XML
/// interdit dans du texte. Echapper aussi l'apostrophe et le guillemet — utile
/// uniquement dans une valeur d'attribut — multiplierait sans raison les
/// entites a re-decoder a la relecture.
fn escape_xml(input: &str) -> String {
    input.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Resout une reference d'entite XML rencontree au fil du texte.
///
/// quick-xml n'inclut pas les entites dans les evenements `Text` : il emet un
/// `GeneralRef` distinct pour chacune. Les ignorer revient a supprimer du
/// document tout caractere echappe — et Word echappe systematiquement `&` et
/// `<`, y compris dans ses propres fichiers.
fn resolve_entity(reference: &quick_xml::events::BytesRef<'_>) -> Option<String> {
    if let Ok(Some(ch)) = reference.resolve_char_ref() {
        return Some(ch.to_string());
    }
    let name: &str = reference.as_ref();
    match name {
        "amp" => Some("&".to_owned()),
        "lt" => Some("<".to_owned()),
        "gt" => Some(">".to_owned()),
        "quot" => Some("\"".to_owned()),
        "apos" => Some("'".to_owned()),
        // Une entite inconnue est restituee telle quelle plutot que perdue :
        // mieux vaut un `&nbsp;` visible qu'un mot ampute en silence.
        _ => Some(format!("&{name};")),
    }
}

/// Extrait le contenu d'un document Word .docx sous forme d'arbre de blocs.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le fichier n'est pas une archive ZIP valide
/// ou s'il ne contient pas le fichier `word/document.xml`.
pub fn parse_docx(bytes: &[u8]) -> Result<DocumentAST, CoreError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| {
        CoreError::failure("docx", format!("Le fichier n'est pas une archive .docx valide : {e}"))
    })?;

    let mut doc_xml = String::new();
    {
        let mut entry = archive.by_name("word/document.xml").map_err(|_| {
            CoreError::failure(
                "docx",
                "Fichier word/document.xml introuvable dans le document .docx".to_owned(),
            )
        })?;
        entry.read_to_string(&mut doc_xml).map_err(|e| {
            CoreError::failure(
                "docx",
                format!("Impossible de lire le contenu du document .docx : {e}"),
            )
        })?;
    }

    let mut reader = Reader::from_str(&doc_xml);
    // Surtout pas de `trim_text` : une entite coupe le texte en fragments, et
    // rogner chacun d'eux mange les espaces qui l'entourent — « 5 &lt; 6 » se
    // relirait « 5<6 ». Le texte n'est recupere qu'a l'interieur de <w:t>, donc
    // l'indentation du XML n'entre jamais dans le contenu, et chaque paragraphe
    // est trimme une fois, a sa fermeture.

    let mut blocks = Vec::new();
    let mut current_paragraph = String::new();
    let mut is_heading = false;
    let mut heading_level = 1u8;
    let mut is_list_item = false;
    let mut in_text = false;

    let mut in_table = false;
    let mut current_cell = String::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_rows: Vec<Vec<String>> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => match e.name().as_ref() {
                "w:tbl" => {
                    in_table = true;
                    current_rows.clear();
                }
                "w:tr" => {
                    current_row.clear();
                }
                "w:tc" => {
                    current_cell.clear();
                }
                "w:p" => {
                    if !in_table {
                        is_heading = false;
                        heading_level = 1;
                        is_list_item = false;
                        current_paragraph.clear();
                    }
                }
                "w:pStyle" => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref().ends_with("val") {
                            let val = attr.value.to_lowercase();
                            if val.contains("heading1") || val.contains("titre1") {
                                is_heading = true;
                                heading_level = 1;
                            } else if val.contains("heading2") || val.contains("titre2") {
                                is_heading = true;
                                heading_level = 2;
                            } else if val.contains("heading3") || val.contains("titre3") {
                                is_heading = true;
                                heading_level = 3;
                            } else if val.contains("list") || val.contains("bullet") {
                                is_list_item = true;
                            }
                        }
                    }
                }
                "w:numPr" => {
                    is_list_item = true;
                }
                "w:t" => {
                    in_text = true;
                }
                _ => {}
            },
            Ok(Event::Text(ref e)) => {
                if in_text {
                    let text = e.as_ref();
                    if in_table {
                        current_cell.push_str(text);
                    } else {
                        current_paragraph.push_str(text);
                    }
                }
            }
            // Les entites arrivent hors des evenements Text : sans cette branche,
            // « Service R&D » se relit « Service RD ».
            Ok(Event::GeneralRef(ref reference)) => {
                if in_text && let Some(decoded) = resolve_entity(reference) {
                    if in_table {
                        current_cell.push_str(&decoded);
                    } else {
                        current_paragraph.push_str(&decoded);
                    }
                }
            }
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                "w:t" => {
                    in_text = false;
                }
                "w:tc" => {
                    let trimmed = current_cell.trim().to_string();
                    current_row.push(trimmed);
                    current_cell.clear();
                }
                "w:tr" => {
                    if !current_row.is_empty() {
                        current_rows.push(current_row.clone());
                        current_row.clear();
                    }
                }
                "w:tbl" => {
                    if !current_rows.is_empty() {
                        blocks.push(DocBlock::Table(current_rows.clone()));
                        current_rows.clear();
                    }
                    in_table = false;
                }
                "w:p" if !in_table => {
                    let trimmed = current_paragraph.trim().to_string();
                    if !trimmed.is_empty() {
                        if is_heading {
                            blocks.push(DocBlock::Heading { level: heading_level, text: trimmed });
                        } else if is_list_item {
                            blocks.push(DocBlock::ListItem(trimmed));
                        } else {
                            blocks.push(DocBlock::Paragraph(trimmed));
                        }
                    }
                    current_paragraph.clear();
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(CoreError::failure(
                    "docx",
                    format!(
                        "Erreur de lecture XML a la position {}: {e}",
                        reader.buffer_position()
                    ),
                ));
            }
            _ => {}
        }
    }

    let title = blocks.iter().find_map(|b| match b {
        DocBlock::Heading { text, .. } => Some(text.clone()),
        _ => None,
    });

    Ok(DocumentAST { title, blocks })
}

/// Produit une archive .docx valide et enrichie avec styles Word natifs (Calibri, marges, interlignes, titres).
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si la compression ou l'ecriture du fichier ZIP echoue.
pub fn generate_docx(ast: &DocumentAST) -> Result<Vec<u8>, CoreError> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // Les parties fixes du document. Ajouter une partie, c'est ajouter une
        // ligne ici — et non recopier huit lignes de gestion d'erreur.
        const PARTS: [(&str, &str); 7] = [
            ("[Content_Types].xml", CONTENT_TYPES_XML),
            ("_rels/.rels", RELS_XML),
            ("word/_rels/document.xml.rels", DOC_RELS_XML),
            ("word/styles.xml", STYLES_XML),
            ("word/fontTable.xml", FONT_TABLE_XML),
            ("word/settings.xml", SETTINGS_XML),
            ("word/numbering.xml", NUMBERING_XML),
        ];

        for (name, content) in PARTS {
            zip.start_file(name, options).map_err(|e| {
                CoreError::failure("docx", format!("Erreur lors de la creation de {name} : {e}"))
            })?;
            zip.write_all(content.as_bytes()).map_err(|e| {
                CoreError::failure("docx", format!("Erreur d'ecriture de {name} : {e}"))
            })?;
        }

        // La partie variable : le corps du document.
        zip.start_file("word/document.xml", options).map_err(|e| {
            CoreError::failure(
                "docx",
                format!("Erreur lors de la creation de word/document.xml : {e}"),
            )
        })?;

        let mut doc_xml = String::with_capacity(8192);
        doc_xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
        doc_xml.push_str(r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#);

        for block in &ast.blocks {
            match block {
                DocBlock::Heading { level, text } => {
                    let escaped = escape_xml(text);
                    let lvl = level.clamp(&1, &3);
                    doc_xml.push_str(&format!(
                        r#"<w:p><w:pPr><w:pStyle w:val="Heading{lvl}"/><w:keepNext/></w:pPr><w:r><w:t xml:space="preserve">{escaped}</w:t></w:r></w:p>"#
                    ));
                }
                DocBlock::Paragraph(text) => {
                    let escaped = escape_xml(text);
                    doc_xml.push_str(&format!(
                        r#"<w:p><w:pPr><w:pStyle w:val="Normal"/></w:pPr><w:r><w:t xml:space="preserve">{escaped}</w:t></w:r></w:p>"#
                    ));
                }
                DocBlock::ListItem(item) => {
                    let escaped = escape_xml(item);
                    doc_xml.push_str(&format!(
                        // La puce vient de la numerotation Word (numbering.xml),
                        // pas d'un caractere ecrit dans le texte : sinon elle se
                        // retrouve collee au contenu a la relecture, et Word ne
                        // voit pas une vraie liste.
                        r#"<w:p><w:pPr><w:pStyle w:val="ListBullet"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t xml:space="preserve">{escaped}</w:t></w:r></w:p>"#
                    ));
                }
                DocBlock::Table(rows) => {
                    doc_xml.push_str(r#"<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/><w:tblW w:w="0" w:type="auto"/><w:tblLook w:val="04A0" w:firstRow="1" w:lastRow="0" w:firstColumn="1" w:lastColumn="0" w:noHBand="0" w:noVBand="1"/></w:tblPr>"#);
                    for (row_idx, row) in rows.iter().enumerate() {
                        doc_xml.push_str("<w:tr>");
                        for cell in row {
                            let escaped = escape_xml(cell);
                            if row_idx == 0 {
                                doc_xml.push_str(&format!(
                                    r#"<w:tc><w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="F1F5F9"/></w:tcPr><w:p><w:pPr><w:pStyle w:val="Normal"/></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t xml:space="preserve">{escaped}</w:t></w:r></w:p></w:tc>"#
                                ));
                            } else {
                                doc_xml.push_str(&format!(
                                    r#"<w:tc><w:p><w:pPr><w:pStyle w:val="Normal"/></w:pPr><w:r><w:t xml:space="preserve">{escaped}</w:t></w:r></w:p></w:tc>"#
                                ));
                            }
                        }
                        doc_xml.push_str("</w:tr>");
                    }
                    doc_xml.push_str("</w:tbl>");
                }
            }
        }

        // Section setup: Format A4 (11906 x 16838 dxa) avec marges standard de 2,54 cm (1440 dxa)
        doc_xml.push_str(r#"<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>"#);
        zip.write_all(doc_xml.as_bytes()).map_err(|e| {
            CoreError::failure("docx", format!("Erreur d'ecriture word/document.xml : {e}"))
        })?;

        zip.finish().map_err(|e| {
            CoreError::failure(
                "docx",
                format!("Erreur lors de la finalisation du fichier .docx : {e}"),
            )
        })?;
    }

    Ok(buffer)
}

const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/fontTable.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml"/>
  <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>
</Types>"#;

const RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

const DOC_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable" Target="fontTable.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>
</Relationships>"#;

/// Definition de la liste a puces utilisee par les blocs [`DocBlock::ListItem`].
///
/// Sans cette partie, Word affiche les elements de liste comme de simples
/// paragraphes indentes : la puce n'existe pas.
const NUMBERING_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:multiLevelType w:val="hybridMultilevel"/>
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="&#xF0B7;"/>
      <w:lvlJc w:val="left"/>
      <w:pPr>
        <w:ind w:left="720" w:hanging="360"/>
      </w:pPr>
      <w:rPr>
        <w:rFonts w:ascii="Symbol" w:hAnsi="Symbol" w:hint="default"/>
      </w:rPr>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault>
      <w:rPr>
        <w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:eastAsia="Calibri" w:cs="Calibri"/>
        <w:sz w:val="22"/>
        <w:szCs w:val="22"/>
        <w:color w:val="1E293B"/>
        <w:lang w:val="fr-FR"/>
      </w:rPr>
    </w:rPrDefault>
    <w:pPrDefault>
      <w:pPr>
        <w:spacing w:after="140" w:line="276" w:lineRule="auto"/>
      </w:pPr>
    </w:pPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:qFormat/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:qFormat/>
    <w:pPr>
      <w:keepNext/>
      <w:spacing w:before="280" w:after="120"/>
    </w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="Calibri Light" w:hAnsi="Calibri Light"/>
      <w:b/>
      <w:bCs/>
      <w:color w:val="0F172A"/>
      <w:sz w:val="36"/>
      <w:szCs w:val="36"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:qFormat/>
    <w:pPr>
      <w:keepNext/>
      <w:spacing w:before="200" w:after="80"/>
    </w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="Calibri Light" w:hAnsi="Calibri Light"/>
      <w:b/>
      <w:bCs/>
      <w:color w:val="1E293B"/>
      <w:sz w:val="28"/>
      <w:szCs w:val="28"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading3">
    <w:name w:val="heading 3"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:qFormat/>
    <w:pPr>
      <w:keepNext/>
      <w:spacing w:before="140" w:after="60"/>
    </w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="Calibri" w:hAnsi="Calibri"/>
      <w:b/>
      <w:bCs/>
      <w:color w:val="334155"/>
      <w:sz w:val="24"/>
      <w:szCs w:val="24"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="ListBullet">
    <w:name w:val="List Bullet"/>
    <w:basedOn w:val="Normal"/>
    <w:qFormat/>
    <w:pPr>
      <w:ind w:left="720" w:hanging="360"/>
      <w:spacing w:after="80" w:line="240" w:lineRule="auto"/>
    </w:pPr>
  </w:style>
  <w:style w:type="table" w:styleId="TableGrid">
    <w:name w:val="Table Grid"/>
    <w:tblPr>
      <w:tblBorders>
        <w:top w:val="single" w:sz="4" w:space="0" w:color="CBD5E1"/>
        <w:left w:val="single" w:sz="4" w:space="0" w:color="CBD5E1"/>
        <w:bottom w:val="single" w:sz="4" w:space="0" w:color="CBD5E1"/>
        <w:right w:val="single" w:sz="4" w:space="0" w:color="CBD5E1"/>
        <w:insideH w:val="single" w:sz="4" w:space="0" w:color="E2E8F0"/>
        <w:insideV w:val="single" w:sz="4" w:space="0" w:color="E2E8F0"/>
      </w:tblBorders>
      <w:tblCellMar>
        <w:top w:w="120" w:type="dxa"/>
        <w:left w:w="160" w:type="dxa"/>
        <w:bottom w:w="120" w:type="dxa"/>
        <w:right w:w="160" w:type="dxa"/>
      </w:tblCellMar>
    </w:tblPr>
  </w:style>
</w:styles>"#;

const FONT_TABLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:fonts xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:font w:name="Calibri">
    <w:panose1 w:val="020F0502020204030204"/>
    <w:charset w:val="00"/>
    <w:family w:val="swiss"/>
    <w:pitch w:val="variable"/>
  </w:font>
  <w:font w:name="Calibri Light">
    <w:panose1 w:val="020F0302020204030204"/>
    <w:charset w:val="00"/>
    <w:family w:val="swiss"/>
    <w:pitch w:val="variable"/>
  </w:font>
</w:fonts>"#;

const SETTINGS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:defaultTabStop w:val="720"/>
  <w:characterSpacingControl w:val="doNotCompress"/>
</w:settings>"#;
