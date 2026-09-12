//! Extraction structuree et haute fidelite de texte et mise en page depuis un fichier PDF vectoriel.

use std::collections::{BTreeMap, HashSet};

use derovia_core::CoreError;
use lopdf::content::Content;
use lopdf::{Document, Encoding, Object, ObjectId};

use crate::model::{DocBlock, DocumentAST};

/// Fragment de texte brut extrait d'un flux d'instructions PDF.
#[derive(Debug, Clone)]
struct TextFragment {
    text: String,
    x: f32,
    y: f32,
    font_size: f32,
    is_bold: bool,
}

/// Ligne visuelle recomposee par alignement horizontal de fragments sur une meme ligne.
#[derive(Debug, Clone)]
struct VisualLine {
    text: String,
    y: f32,
    font_size: f32,
    is_bold: bool,
}

/// Proprietes d'une police de caracteres de page PDF.
#[derive(Debug, Clone, Default)]
struct FontProperties {
    is_bold: bool,
}

/// Extrait le texte d'un document PDF et le structure sous forme de [`DocumentAST`].
///
/// Cette fonction effectue une analyse de mise en page (Layout Analysis) en :
/// 1. Decodant le texte avec gestion de secours multi-encodages (ToUnicode, UTF-16, UTF-8, Windows-1252) ;
/// 2. Reconstituant les lignes visuelles avec calcul du kerning et des espacements ;
/// 3. Identifiant les titres (Heading 1, 2, 3), les listes et les tableaux ;
/// 4. Fusionnant les lignes decoupees en paragraphes fluides et continus.
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le fichier PDF est corrompu ou ne contient aucun texte exploitable.
pub fn extract_pdf_text(bytes: &[u8]) -> Result<DocumentAST, CoreError> {
    let doc = Document::load_mem(bytes).map_err(|e| {
        CoreError::failure("pdf_extract", format!("Impossible d'ouvrir le fichier PDF : {e}"))
    })?;

    let pages = doc.get_pages();
    let mut all_visual_lines = Vec::new();

    for (page_num, page_id) in pages {
        let page_lines = extract_page_lines(&doc, page_id, page_num);
        all_visual_lines.extend(page_lines);
    }

    if all_visual_lines.is_empty() {
        return Err(CoreError::failure(
            "pdf_extract",
            "Aucune couche de texte exploitable n'a ete detectee dans ce PDF (document possiblement scanne sous forme d'image pure)."
                .to_owned(),
        ));
    }

    Ok(build_ast_from_lines(all_visual_lines))
}

/// Extrait les lignes visuelles d'une page individuelle.
fn extract_page_lines(doc: &Document, page_id: ObjectId, _page_num: u32) -> Vec<VisualLine> {
    let mut font_map = BTreeMap::new();
    if let Ok(page_fonts) = doc.get_page_fonts(page_id) {
        for (name, font_dict) in page_fonts {
            let base_font = font_dict
                .get(b"BaseFont")
                .and_then(Object::as_name)
                .map(|bytes| String::from_utf8_lossy(bytes).to_lowercase())
                .unwrap_or_default();
            let is_bold = base_font.contains("bold")
                || base_font.contains("black")
                || base_font.contains("heavy")
                || base_font.contains("semibold")
                || base_font.contains("demi");
            let encoding = font_dict.get_font_encoding(doc).ok();
            font_map.insert(name, (encoding, FontProperties { is_bold }));
        }
    }

    let mut fragments = Vec::new();
    let mut visited_forms = HashSet::new();

    if let Ok(content_data) = doc.get_page_content(page_id)
        && let Ok(content) = Content::decode(&content_data)
    {
        process_content_operations(&content, &font_map, doc, &mut visited_forms, &mut fragments);
    }

    reconstruct_visual_lines(fragments)
}

/// Traite les operations d'un flux de contenu PDF.
#[allow(
    clippy::cast_precision_loss,
    reason = "le nombre de caracteres est petit et la conversion en f32 est exacte"
)]
fn process_content_operations(
    content: &Content,
    fonts: &BTreeMap<Vec<u8>, (Option<Encoding>, FontProperties)>,
    doc: &Document,
    visited_forms: &mut HashSet<ObjectId>,
    fragments: &mut Vec<TextFragment>,
) {
    let mut current_font_encoding: Option<&Encoding> = None;
    let mut current_font_props = FontProperties::default();
    let mut current_font_size: f32 = 11.0;

    let mut text_x: f32 = 0.0;
    let mut text_y: f32 = 0.0;
    let mut line_x: f32 = 0.0;
    let mut line_y: f32 = 0.0;
    let mut leading: f32 = 14.0;

    for op in &content.operations {
        match op.operator.as_str() {
            "BT" => {
                text_x = 0.0;
                text_y = 0.0;
                line_x = 0.0;
                line_y = 0.0;
            }
            "ET" => {}
            "Tf" => {
                if let Some(font_name_obj) = op.operands.first()
                    && let Ok(font_name) = font_name_obj.as_name()
                {
                    if let Some((enc, props)) = fonts.get(font_name) {
                        current_font_encoding = enc.as_ref();
                        current_font_props = props.clone();
                    } else {
                        current_font_encoding = None;
                        current_font_props = FontProperties::default();
                    }
                }
                if let Some(size_obj) = op.operands.get(1)
                    && let Ok(size) = size_obj.as_f32()
                    && size > 0.0
                {
                    current_font_size = size;
                }
            }
            "Tm" => {
                if let (Some(op4), Some(op5)) = (op.operands.get(4), op.operands.get(5))
                    && let (Ok(e), Ok(f)) = (op4.as_f32(), op5.as_f32())
                {
                    line_x = e;
                    line_y = f;
                    text_x = e;
                    text_y = f;
                }
            }
            "Td" => {
                if let (Some(op0), Some(op1)) = (op.operands.first(), op.operands.get(1))
                    && let (Ok(dx), Ok(dy)) = (op0.as_f32(), op1.as_f32())
                {
                    line_x += dx;
                    line_y += dy;
                    text_x = line_x;
                    text_y = line_y;
                }
            }
            "TD" => {
                if let (Some(op0), Some(op1)) = (op.operands.first(), op.operands.get(1))
                    && let (Ok(dx), Ok(dy)) = (op0.as_f32(), op1.as_f32())
                {
                    leading = -dy;
                    line_x += dx;
                    line_y += dy;
                    text_x = line_x;
                    text_y = line_y;
                }
            }
            "T*" => {
                line_y -= leading;
                text_x = line_x;
                text_y = line_y;
            }
            "TL" => {
                if let Some(l_obj) = op.operands.first()
                    && let Ok(l) = l_obj.as_f32()
                    && l > 0.0
                {
                    leading = l;
                }
            }
            "Tj" | "TJ" => {
                let text = decode_operands(&op.operands, current_font_encoding);
                if !text.is_empty() {
                    let char_count = text.chars().count() as f32;
                    fragments.push(TextFragment {
                        text,
                        x: text_x,
                        y: text_y,
                        font_size: current_font_size,
                        is_bold: current_font_props.is_bold,
                    });
                    text_x += char_count * current_font_size * 0.5;
                }
            }
            "'" => {
                line_y -= leading;
                text_x = line_x;
                text_y = line_y;
                let text = decode_operands(&op.operands, current_font_encoding);
                if !text.is_empty() {
                    let char_count = text.chars().count() as f32;
                    fragments.push(TextFragment {
                        text,
                        x: text_x,
                        y: text_y,
                        font_size: current_font_size,
                        is_bold: current_font_props.is_bold,
                    });
                    text_x += char_count * current_font_size * 0.5;
                }
            }
            "\"" => {
                if op.operands.len() >= 3 {
                    line_y -= leading;
                    text_x = line_x;
                    text_y = line_y;
                    if let Some(text_obj) = op.operands.get(2) {
                        let text =
                            decode_operands(std::slice::from_ref(text_obj), current_font_encoding);
                        if !text.is_empty() {
                            let char_count = text.chars().count() as f32;
                            fragments.push(TextFragment {
                                text,
                                x: text_x,
                                y: text_y,
                                font_size: current_font_size,
                                is_bold: current_font_props.is_bold,
                            });
                            text_x += char_count * current_font_size * 0.5;
                        }
                    }
                }
            }
            "Do" => {
                // Exploration des formulaires XObjects (/Subtype /Form)
                if let Some(name_obj) = op.operands.first()
                    && let Ok(form_name) = name_obj.as_name()
                {
                    process_form_xobject(doc, form_name, fonts, visited_forms, fragments);
                }
            }
            _ => {}
        }
    }
}

/// Extrait le contenu d'un Form XObject s'il n'a pas encore ete visite.
fn process_form_xobject(
    doc: &Document,
    _form_name: &[u8],
    fonts: &BTreeMap<Vec<u8>, (Option<Encoding>, FontProperties)>,
    visited_forms: &mut HashSet<ObjectId>,
    fragments: &mut Vec<TextFragment>,
) {
    for (&id, obj) in &doc.objects {
        if visited_forms.contains(&id) {
            continue;
        }
        let Ok(stream) = obj.as_stream() else {
            continue;
        };
        let Ok(subtype) = stream.dict.get(b"Subtype").and_then(Object::as_name) else {
            continue;
        };
        if subtype == b"Form" {
            visited_forms.insert(id);
            if let Ok(decompressed) = stream.decompressed_content()
                && let Ok(content) = Content::decode(&decompressed)
            {
                process_content_operations(&content, fonts, doc, visited_forms, fragments);
            }
        }
    }
}

/// Decodage robuste des operandes textuels avec gestion des espacements de kerning (`TJ`).
fn decode_operands(operands: &[Object], encoding: Option<&Encoding>) -> String {
    let mut text = String::new();
    for op in operands {
        match op {
            Object::String(bytes, _) => {
                text.push_str(&decode_bytes(bytes, encoding));
            }
            Object::Array(arr) => {
                for item in arr {
                    match item {
                        Object::String(bytes, _) => {
                            text.push_str(&decode_bytes(bytes, encoding));
                        }
                        Object::Integer(i)
                            if *i < -120 && !text.ends_with(' ') && !text.is_empty() =>
                        {
                            text.push(' ');
                        }
                        Object::Real(r)
                            if *r < -120.0 && !text.ends_with(' ') && !text.is_empty() =>
                        {
                            text.push(' ');
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    text
}

/// Decodage multi-niveaux d'une sequence d'octets PDF.
fn decode_bytes(bytes: &[u8], encoding: Option<&Encoding>) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    // 1. Encodage ToUnicode ou table de correspondances du document
    if let Some(enc) = encoding
        && let Ok(decoded) = enc.bytes_to_string(bytes)
    {
        let clean = decoded.trim();
        if !clean.is_empty() && clean.chars().any(char::is_alphanumeric) {
            return decoded;
        }
    }

    // 2. Detection UTF-16BE (BOM 0xFE, 0xFF)
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let rest = bytes.get(2..).unwrap_or(&[]);
        let mut u16_chars = Vec::with_capacity(rest.len() / 2);
        for chunk in rest.chunks(2) {
            if let (Some(&b0), Some(&b1)) = (chunk.first(), chunk.get(1)) {
                u16_chars.push(u16::from_be_bytes([b0, b1]));
            }
        }
        if let Ok(s) = String::from_utf16(&u16_chars) {
            return s;
        }
    }

    // 3. Essai UTF-8
    if let Ok(s) = std::str::from_utf8(bytes)
        && s.chars().any(char::is_alphanumeric)
    {
        return s.to_string();
    }

    // 4. Repli Windows-1252 / ISO-8859-1 (standard PDFDocEncoding)
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            0x00..=0x08 | 0x0B..=0x0C | 0x0E..=0x1F => {}
            0x09 => out.push('\t'),
            0x0A | 0x0D => out.push(' '),
            0x20..=0x7E => out.push(b as char),
            0x80 => out.push('€'),
            0x82 => out.push('‚'),
            0x83 => out.push('ƒ'),
            0x84 => out.push('„'),
            0x85 => out.push('…'),
            0x88 => out.push('ˆ'),
            0x89 => out.push('‰'),
            0x8B => out.push('‹'),
            0x8C => out.push('Œ'),
            0x91 => out.push('‘'),
            0x92 => out.push('’'),
            0x93 => out.push('“'),
            0x94 => out.push('”'),
            0x95 => out.push('•'),
            0x96 => out.push('–'),
            0x97 => out.push('—'),
            0x99 => out.push('™'),
            0x9B => out.push('›'),
            0x9C => out.push('œ'),
            0x9F => out.push('Ÿ'),
            0xA0..=0xFF => out.push(b as char),
            _ => {}
        }
    }
    out
}

/// Reconstitue les lignes visuelles a partir des fragments spatiaux.
#[allow(
    clippy::cast_precision_loss,
    reason = "le nombre d'elements et de caracteres tient aisement dans un f32 sans perte"
)]
fn reconstruct_visual_lines(mut fragments: Vec<TextFragment>) -> Vec<VisualLine> {
    if fragments.is_empty() {
        return Vec::new();
    }

    fragments.retain(|f| !f.text.trim().is_empty());

    // Regroupement par coordonnee Y similaire (+/- 3 pt)
    let mut clusters: Vec<Vec<TextFragment>> = Vec::new();
    for frag in fragments {
        let mut matched = false;
        for cluster in &mut clusters {
            if cluster.first().is_some_and(|first| (first.y - frag.y).abs() <= 3.0) {
                cluster.push(frag.clone());
                matched = true;
                break;
            }
        }
        if !matched {
            clusters.push(vec![frag]);
        }
    }

    // Tri vertical descendant (du haut de la page vers le bas)
    clusters.sort_by(|a, b| {
        let y_a = a.first().map_or(0.0, |f| f.y);
        let y_b = b.first().map_or(0.0, |f| f.y);
        y_b.partial_cmp(&y_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut lines = Vec::with_capacity(clusters.len());
    for mut cluster in clusters {
        // Tri horizontal croissant (de gauche a droite)
        cluster.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));

        let mut line_text = String::new();
        let mut max_font_size: f32 = 10.0;
        let mut bold_chars = 0;
        let mut total_chars = 0;
        let mut avg_y = 0.0;

        for (idx, frag) in cluster.iter().enumerate() {
            avg_y += frag.y;
            if frag.font_size > max_font_size {
                max_font_size = frag.font_size;
            }
            let trimmed = frag.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            let char_len = trimmed.chars().count();
            total_chars += char_len;
            if frag.is_bold {
                bold_chars += char_len;
            }

            if idx > 0
                && !line_text.is_empty()
                && !line_text.ends_with(' ')
                && !trimmed.starts_with(' ')
            {
                line_text.push(' ');
            }
            line_text.push_str(trimmed);
        }

        if !line_text.trim().is_empty() {
            let is_bold = total_chars > 0 && (bold_chars * 2 >= total_chars);
            lines.push(VisualLine {
                text: line_text.trim().to_string(),
                y: if cluster.is_empty() { 0.0 } else { avg_y / cluster.len() as f32 },
                font_size: max_font_size,
                is_bold,
            });
        }
    }

    lines
}

/// Reconstruit la hierarchie de blocs [`DocumentAST`] a partir des lignes visuelles.
fn build_ast_from_lines(lines: Vec<VisualLine>) -> DocumentAST {
    if lines.is_empty() {
        return DocumentAST::default();
    }

    // Calcul de la taille de police mediane
    let mut sizes: Vec<f32> =
        lines.iter().map(|l| l.font_size).filter(|&s| (6.0..=36.0).contains(&s)).collect();
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_size =
        if sizes.is_empty() { 11.0 } else { sizes.get(sizes.len() / 2).copied().unwrap_or(11.0) };

    let mut blocks = Vec::new();
    let mut current_paragraph = String::new();
    let mut last_y: Option<f32> = None;

    let flush_para = |current: &mut String, blocks: &mut Vec<DocBlock>| {
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            blocks.push(DocBlock::Paragraph(trimmed));
            current.clear();
        }
    };

    for line in lines {
        let trimmed = line.text.trim();
        if trimmed.is_empty() {
            continue;
        }

        let is_h1 = line.font_size >= (median_size * 1.45).max(18.0)
            || (line.font_size >= 16.0 && line.is_bold && trimmed.len() <= 80);
        let is_h2 = !is_h1
            && (line.font_size >= (median_size * 1.25).max(14.0)
                || (line.font_size >= 13.0 && line.is_bold && trimmed.len() <= 100));
        let is_h3 =
            !is_h1 && !is_h2 && line.is_bold && trimmed.len() <= 90 && !trimmed.ends_with('.');

        // 1. Titres
        if is_h1 {
            flush_para(&mut current_paragraph, &mut blocks);
            blocks.push(DocBlock::Heading { level: 1, text: trimmed.to_string() });
            last_y = Some(line.y);
            continue;
        } else if is_h2 {
            flush_para(&mut current_paragraph, &mut blocks);
            blocks.push(DocBlock::Heading { level: 2, text: trimmed.to_string() });
            last_y = Some(line.y);
            continue;
        } else if is_h3 {
            flush_para(&mut current_paragraph, &mut blocks);
            blocks.push(DocBlock::Heading { level: 3, text: trimmed.to_string() });
            last_y = Some(line.y);
            continue;
        }

        // 2. Elements de listes a puces
        let bullet_markers = ['•', '◦', '▪', '▫', '-', '*', '–', '—'];
        let is_bullet = bullet_markers.iter().any(|&b| trimmed.starts_with(b));
        if is_bullet {
            flush_para(&mut current_paragraph, &mut blocks);
            let cleaned = trimmed.trim_start_matches(&bullet_markers[..]).trim().to_string();
            if !cleaned.is_empty() {
                blocks.push(DocBlock::ListItem(cleaned));
            }
            last_y = Some(line.y);
            continue;
        }

        // Listes numerotees (ex: "1. ", "2) ")
        if let Some(space_idx) = trimmed.find(' ')
            && let (Some(prefix), Some(content)) =
                (trimmed.get(..space_idx), trimmed.get(space_idx + 1..))
        {
            let num_len = prefix.len().saturating_sub(1);
            let num_part = prefix.get(..num_len).unwrap_or("");
            if (prefix.ends_with('.') || prefix.ends_with(')'))
                && !num_part.is_empty()
                && num_part.chars().all(|c| c.is_ascii_digit())
            {
                flush_para(&mut current_paragraph, &mut blocks);
                let content_trimmed = content.trim().to_string();
                if !content_trimmed.is_empty() {
                    blocks.push(DocBlock::ListItem(content_trimmed));
                }
                last_y = Some(line.y);
                continue;
            }
        }

        // 3. Lignes de tableau
        if trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2 {
            flush_para(&mut current_paragraph, &mut blocks);
            let cells: Vec<String> = trimmed
                .split('|')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_string())
                .collect();
            if !cells.is_empty() {
                if let Some(DocBlock::Table(rows)) = blocks.last_mut() {
                    rows.push(cells);
                } else {
                    blocks.push(DocBlock::Table(vec![cells]));
                }
            }
            last_y = Some(line.y);
            continue;
        }

        // 4. Paragraphe continu (fusion des lignes coupées)
        let is_large_gap = match last_y {
            Some(prev_y) => (prev_y - line.y) > (median_size * 1.8),
            None => false,
        };

        if is_large_gap {
            flush_para(&mut current_paragraph, &mut blocks);
        }

        if !current_paragraph.is_empty() {
            if !current_paragraph.ends_with(' ') {
                current_paragraph.push(' ');
            }
            current_paragraph.push_str(trimmed);
        } else {
            current_paragraph.push_str(trimmed);
        }

        last_y = Some(line.y);
    }

    flush_para(&mut current_paragraph, &mut blocks);

    let title = blocks.iter().find_map(|b| match b {
        DocBlock::Heading { text, .. } => Some(text.clone()),
        _ => None,
    });

    DocumentAST { title, blocks }
}
