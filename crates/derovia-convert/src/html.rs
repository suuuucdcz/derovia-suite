//! Extraction du contenu d'une page HTML vers un [`DocumentAST`].
//!
//! Ce n'est pas un moteur de rendu : on ne cherche ni la mise en page, ni le
//! style, seulement la structure editoriale — titres, paragraphes, listes,
//! cellules de tableau. Tout le reste du balisage est ecarte.
//!
//! L'implementation precedente se contentait de remplacer `<br>` et `</p>` par
//! des sauts de ligne avant de passer le tout au lecteur Markdown : les autres
//! balises ressortaient telles quelles dans le document produit.

use crate::model::{DocBlock, DocumentAST};

/// Balises dont le contenu n'a rien a faire dans un document.
const IGNORED_CONTENT: [&str; 4] = ["script", "style", "head", "noscript"];

/// Decode les entites HTML les plus courantes.
fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        match tail.find(';').filter(|end| *end <= 12) {
            Some(end) => {
                let name = &tail[1..end];
                let decoded = match name {
                    "amp" => Some("&".to_owned()),
                    "lt" => Some("<".to_owned()),
                    "gt" => Some(">".to_owned()),
                    "quot" => Some("\"".to_owned()),
                    "apos" | "#39" => Some("'".to_owned()),
                    "nbsp" => Some("\u{a0}".to_owned()),
                    "eacute" => Some("é".to_owned()),
                    "egrave" => Some("è".to_owned()),
                    "agrave" => Some("à".to_owned()),
                    "ccedil" => Some("ç".to_owned()),
                    "ugrave" => Some("ù".to_owned()),
                    "laquo" => Some("«".to_owned()),
                    "raquo" => Some("»".to_owned()),
                    _ => numeric_entity(name),
                };
                match decoded {
                    Some(text) => out.push_str(&text),
                    // Entite inconnue : on la restitue plutot que de l'effacer.
                    None => out.push_str(&tail[..=end]),
                }
                rest = &tail[end + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Decode une entite numerique `&#233;` ou `&#xE9;`.
fn numeric_entity(name: &str) -> Option<String> {
    let digits = name.strip_prefix('#')?;
    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<u32>().ok()?,
    };
    char::from_u32(code).map(|c| c.to_string())
}

/// Replie les suites de blancs, comme le fait le rendu HTML.
///
/// Seuls les blancs ASCII sont replies. L'espace insecable (U+00A0) est un
/// caractere a part entiere en typographie francaise — « 12 % », « mot : » — et
/// `split_whitespace` l'aurait converti en espace ordinaire.
fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut espace_en_attente = false;

    for ch in text.chars() {
        if ch.is_ascii_whitespace() {
            espace_en_attente = !out.is_empty();
        } else {
            if espace_en_attente {
                out.push(' ');
                espace_en_attente = false;
            }
            out.push(ch);
        }
    }
    out
}

/// Extrait la structure editoriale d'une page HTML.
///
/// Les titres `<h1>` a `<h6>` deviennent des [`DocBlock::Heading`], les `<li>`
/// des [`DocBlock::ListItem`], les `<p>` et cellules de tableau des paragraphes.
#[must_use]
pub fn parse_html(source: &str) -> DocumentAST {
    let mut blocks = Vec::new();
    let mut buffer = String::new();
    // Le bloc en cours : None tant qu'aucune balise structurante n'est ouverte.
    let mut pending: Option<DocBlock> = None;

    let bytes = source.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        let Some(open) = source[index..].find('<') else {
            buffer.push_str(&source[index..]);
            break;
        };
        buffer.push_str(&source[index..index + open]);
        let after = index + open + 1;

        let Some(close) = source[after..].find('>') else {
            // Un « < » non ferme est du texte, pas une balise.
            buffer.push_str(&source[index + open..]);
            break;
        };
        let raw = &source[after..after + close];
        index = after + close + 1;

        let closing = raw.starts_with('/');
        let name: String = raw
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();

        if IGNORED_CONTENT.contains(&name.as_str()) && !closing {
            // On saute tout le contenu jusqu'a la balise fermante.
            let needle = format!("</{name}");
            if let Some(end) = source[index..].to_ascii_lowercase().find(&needle) {
                index += end;
            }
            continue;
        }

        let flush =
            |blocks: &mut Vec<DocBlock>, buffer: &mut String, pending: &mut Option<DocBlock>| {
                let text = collapse(&decode_entities(buffer));
                buffer.clear();
                if text.is_empty() {
                    *pending = None;
                    return;
                }
                match pending.take() {
                    Some(DocBlock::Heading { level, .. }) => {
                        blocks.push(DocBlock::Heading { level, text });
                    }
                    Some(DocBlock::ListItem(_)) => blocks.push(DocBlock::ListItem(text)),
                    _ => blocks.push(DocBlock::Paragraph(text)),
                }
            };

        match name.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                flush(&mut blocks, &mut buffer, &mut pending);
                if !closing {
                    let level = name[1..].parse::<u8>().unwrap_or(1).min(6);
                    pending = Some(DocBlock::Heading { level, text: String::new() });
                }
            }
            "li" => {
                flush(&mut blocks, &mut buffer, &mut pending);
                if !closing {
                    pending = Some(DocBlock::ListItem(String::new()));
                }
            }
            "p" | "div" | "td" | "th" | "tr" | "section" | "article" | "blockquote" => {
                flush(&mut blocks, &mut buffer, &mut pending);
            }
            "br" => buffer.push(' '),
            // Les balises en ligne (b, i, span, a...) ne coupent pas le texte.
            _ => {}
        }
    }

    let trailing = collapse(&decode_entities(&buffer));
    if !trailing.is_empty() {
        match pending.take() {
            Some(DocBlock::Heading { level, .. }) => {
                blocks.push(DocBlock::Heading { level, text: trailing });
            }
            Some(DocBlock::ListItem(_)) => blocks.push(DocBlock::ListItem(trailing)),
            _ => blocks.push(DocBlock::Paragraph(trailing)),
        }
    }

    let title = blocks.iter().find_map(|block| match block {
        DocBlock::Heading { text, .. } => Some(text.clone()),
        _ => None,
    });

    DocumentAST { title, blocks }
}
