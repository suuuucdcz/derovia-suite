//! Analyse et generation pour les formats Markdown, HTML et Texte brut.

use crate::model::{DocBlock, DocumentAST};

/// Transforme une chaine de texte brut ou Markdown en arbre de blocs [`DocumentAST`].
#[must_use]
pub fn parse_markdown(text: &str) -> DocumentAST {
    let mut blocks = Vec::new();
    let mut current_paragraph = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current_paragraph.is_empty() {
                blocks.push(DocBlock::Paragraph(current_paragraph.trim().to_string()));
                current_paragraph.clear();
            }
            continue;
        }

        if let Some(heading) = trimmed.strip_prefix("### ") {
            if !current_paragraph.is_empty() {
                blocks.push(DocBlock::Paragraph(current_paragraph.trim().to_string()));
                current_paragraph.clear();
            }
            blocks.push(DocBlock::Heading { level: 3, text: heading.trim().to_string() });
        } else if let Some(heading) = trimmed.strip_prefix("## ") {
            if !current_paragraph.is_empty() {
                blocks.push(DocBlock::Paragraph(current_paragraph.trim().to_string()));
                current_paragraph.clear();
            }
            blocks.push(DocBlock::Heading { level: 2, text: heading.trim().to_string() });
        } else if let Some(heading) = trimmed.strip_prefix("# ") {
            if !current_paragraph.is_empty() {
                blocks.push(DocBlock::Paragraph(current_paragraph.trim().to_string()));
                current_paragraph.clear();
            }
            blocks.push(DocBlock::Heading { level: 1, text: heading.trim().to_string() });
        } else if let Some(item) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* "))
        {
            if !current_paragraph.is_empty() {
                blocks.push(DocBlock::Paragraph(current_paragraph.trim().to_string()));
                current_paragraph.clear();
            }
            blocks.push(DocBlock::ListItem(item.trim().to_string()));
        } else if trimmed.starts_with('|') && trimmed.ends_with('|') {
            if !current_paragraph.is_empty() {
                blocks.push(DocBlock::Paragraph(current_paragraph.trim().to_string()));
                current_paragraph.clear();
            }
            // Ligne de separateur de tableau : ignorer
            if trimmed.contains("---") {
                continue;
            }
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
        } else {
            if !current_paragraph.is_empty() {
                current_paragraph.push(' ');
            }
            current_paragraph.push_str(trimmed);
        }
    }

    if !current_paragraph.is_empty() {
        blocks.push(DocBlock::Paragraph(current_paragraph.trim().to_string()));
    }

    let title = blocks.iter().find_map(|b| match b {
        DocBlock::Heading { level: 1, text } => Some(text.clone()),
        _ => None,
    });

    DocumentAST { title, blocks }
}

/// Convertit un [`DocumentAST`] en texte au format Markdown.
#[must_use]
pub fn ast_to_markdown(ast: &DocumentAST) -> String {
    let mut out = String::with_capacity(1024);
    for block in &ast.blocks {
        match block {
            DocBlock::Heading { level, text } => {
                let hashes = "#".repeat(*level as usize);
                out.push_str(&format!("{hashes} {text}\n\n"));
            }
            DocBlock::Paragraph(text) => {
                out.push_str(text);
                out.push_str("\n\n");
            }
            DocBlock::ListItem(item) => {
                out.push_str(&format!("- {item}\n"));
            }
            DocBlock::Table(rows) => {
                for (i, row) in rows.iter().enumerate() {
                    out.push('|');
                    for cell in row {
                        out.push_str(&format!(" {cell} |"));
                    }
                    out.push('\n');
                    if i == 0 {
                        out.push('|');
                        for _ in row {
                            out.push_str(" --- |");
                        }
                        out.push('\n');
                    }
                }
                out.push('\n');
            }
        }
    }
    out
}

/// Convertit un [`DocumentAST`] en texte brut simple.
#[must_use]
pub fn ast_to_text(ast: &DocumentAST) -> String {
    let mut out = String::with_capacity(1024);
    for block in &ast.blocks {
        match block {
            DocBlock::Heading { text, .. } => {
                out.push_str(text);
                out.push_str("\n\n");
            }
            DocBlock::Paragraph(text) => {
                out.push_str(text);
                out.push_str("\n\n");
            }
            DocBlock::ListItem(item) => {
                out.push_str(&format!("• {item}\n"));
            }
            DocBlock::Table(rows) => {
                for row in rows {
                    out.push_str(&row.join("\t"));
                    out.push('\n');
                }
                out.push('\n');
            }
        }
    }
    out
}

/// Convertit un [`DocumentAST`] en document HTML autonome stylise.
#[must_use]
pub fn ast_to_html(ast: &DocumentAST) -> String {
    let title = ast.title.as_deref().unwrap_or("Document");
    let mut out = format!(
        r#"<!doctype html>
<html lang="fr">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{}</title>
  <style>
    body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; line-height: 1.6; color: #1d1d1f; max-width: 800px; margin: 40px auto; padding: 0 20px; background: #fff; }}
    h1 {{ font-size: 2rem; border-bottom: 1px solid #e5e5ea; padding-bottom: 8px; margin-top: 24px; }}
    h2 {{ font-size: 1.5rem; margin-top: 20px; }}
    h3 {{ font-size: 1.2rem; margin-top: 16px; }}
    p {{ margin: 12px 0; }}
    ul {{ padding-left: 24px; }}
    table {{ border-collapse: collapse; width: 100%; margin: 16px 0; }}
    th, td {{ border: 1px solid #d2d2d7; padding: 8px 12px; text-align: left; }}
    th {{ background: #f5f5f7; }}
  </style>
</head>
<body>
"#,
        escape_html(title)
    );

    for block in &ast.blocks {
        match block {
            DocBlock::Heading { level, text } => {
                let lvl = level.clamp(&1, &6);
                out.push_str(&format!("<h{lvl}>{}</h{lvl}>\n", escape_html(text)));
            }
            DocBlock::Paragraph(text) => {
                out.push_str(&format!("<p>{}</p>\n", escape_html(text)));
            }
            DocBlock::ListItem(item) => {
                out.push_str(&format!("<ul><li>{}</li></ul>\n", escape_html(item)));
            }
            DocBlock::Table(rows) => {
                out.push_str("<table>\n");
                for (i, row) in rows.iter().enumerate() {
                    let tag = if i == 0 { "th" } else { "td" };
                    out.push_str("  <tr>\n");
                    for cell in row {
                        out.push_str(&format!("    <{tag}>{}</{tag}>\n", escape_html(cell)));
                    }
                    out.push_str("  </tr>\n");
                }
                out.push_str("</table>\n");
            }
        }
    }

    out.push_str("</body>\n</html>");
    out
}

fn escape_html(input: &str) -> String {
    input.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
