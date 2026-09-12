//! Generation de documents PDF a partir de l'arbre syntaxique [`DocumentAST`].

use derovia_core::CoreError;
use printpdf::{BuiltinFont, IndirectFontRef, Mm, PdfDocumentReference, PdfLayerReference};

use crate::model::{ConvertOptions, DocBlock, DocumentAST};

/// Un millimetre exprime en points typographiques.
const PT_PER_MM: f32 = 72.0 / 25.4;

/// Largeur moyenne d'un caractere d'Helvetica, en fraction du corps.
///
/// Helvetica est une police proportionnelle : on ne peut pas connaitre la
/// largeur exacte d'une ligne sans mesurer chaque glyphe. Ce facteur, mesure
/// sur du texte courant francais, suffit a decider ou couper.
const AVERAGE_GLYPH_RATIO: f32 = 0.5;

/// Geometrie de la page, deduite des options de conversion.
#[derive(Debug, Clone, Copy)]
pub struct PageLayout {
    /// Largeur de page en millimetres.
    pub width_mm: f32,
    /// Hauteur de page en millimetres.
    pub height_mm: f32,
    /// Marge appliquee sur les quatre cotes, en millimetres.
    pub margin_mm: f32,
}

impl Default for PageLayout {
    fn default() -> Self {
        Self { width_mm: 210.0, height_mm: 297.0, margin_mm: 20.0 }
    }
}

impl PageLayout {
    /// Construit la geometrie a partir des options fournies par l'interface.
    ///
    /// Un format inconnu retombe sur l'A4 : mieux vaut un document correct
    /// qu'une erreur pour une chaine mal orthographiee.
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "une marge en millimetres est bornee a quelques dizaines : la precision d'un f32 la couvre largement, et printpdf ne manipule que des f32"
    )]
    pub fn from_options(options: &ConvertOptions) -> Self {
        let default = Self::default();
        let (width_mm, height_mm) = match options.page_size.as_deref().map(str::to_lowercase) {
            Some(ref size) if size == "letter" => (215.9, 279.4),
            Some(ref size) if size == "a5" => (148.0, 210.0),
            Some(ref size) if size == "a3" => (297.0, 420.0),
            _ => (default.width_mm, default.height_mm),
        };

        // Une marge absurde rendrait la zone de texte nulle et la boucle de
        // rendu infinie : on la borne a une fraction de la page.
        let margin_mm = options
            .margin_mm
            .map_or(default.margin_mm, |mm| (mm as f32).clamp(5.0, width_mm.min(height_mm) / 4.0));

        Self { width_mm, height_mm, margin_mm }
    }

    /// Ordonnee de la premiere ligne d'une page.
    fn top_y(self) -> f32 {
        self.height_mm - self.margin_mm
    }

    /// Ordonnee en dessous de laquelle il faut changer de page.
    fn bottom_y(self) -> f32 {
        self.margin_mm
    }

    /// Nombre approximatif de caracteres tenant sur une ligne.
    ///
    /// Derive de la largeur utile plutot que fixe en dur : changer le format de
    /// page ou les marges ajuste la coupure sans qu'on ait a retoucher un nombre.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "le nombre de caracteres par ligne est un petit entier positif"
    )]
    fn max_chars(self, font_size_pt: f32, indent_mm: f32) -> usize {
        let usable_mm = (self.width_mm - 2.0 * self.margin_mm - indent_mm).max(10.0);
        let glyph_pt = font_size_pt * AVERAGE_GLYPH_RATIO;
        ((usable_mm * PT_PER_MM) / glyph_pt).max(8.0) as usize
    }
}

/// Coupe un texte en lignes tenant dans la largeur donnee.
///
/// La longueur se compte en **caracteres** et non en octets : en UTF-8 un « é »
/// pese deux octets, et compter les octets couperait les lignes francaises bien
/// avant la marge.
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        if current.is_empty() {
            current.push_str(word);
            current_len = word_len;
        } else if current_len + 1 + word_len <= max_chars {
            current.push(' ');
            current.push_str(word);
            current_len += 1 + word_len;
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
            current_len = word_len;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Curseur d'ecriture : suit la page courante et l'ordonnee, et change de page
/// quand la place manque.
struct Writer<'a> {
    doc: &'a PdfDocumentReference,
    layout: PageLayout,
    layer: PdfLayerReference,
    y: f32,
}

impl<'a> Writer<'a> {
    fn new(doc: &'a PdfDocumentReference, layout: PageLayout, layer: PdfLayerReference) -> Self {
        Self { doc, layout, layer, y: layout.top_y() }
    }

    /// Garantit qu'il reste la place d'ecrire une ligne, quitte a ouvrir une page.
    fn ensure_room(&mut self) {
        if self.y < self.layout.bottom_y() {
            let (page, layer) =
                self.doc.add_page(Mm(self.layout.width_mm), Mm(self.layout.height_mm), "Contenu");
            self.layer = self.doc.get_page(page).get_layer(layer);
            self.y = self.layout.top_y();
        }
    }

    /// Ecrit une ligne et descend le curseur.
    fn line(&mut self, text: &str, size: f32, indent_mm: f32, font: &IndirectFontRef, step: f32) {
        self.ensure_room();
        self.layer.use_text(text, size, Mm(self.layout.margin_mm + indent_mm), Mm(self.y), font);
        self.y -= step;
    }

    /// Laisse un blanc entre deux blocs.
    fn gap(&mut self, mm: f32) {
        self.y -= mm;
    }
}

/// Genere un document PDF vectoriel a partir d'un [`DocumentAST`].
///
/// # Erreurs
///
/// Renvoie une [`CoreError`] si le chargement des polices ou l'encodage PDF echoue.
pub fn generate_pdf(ast: &DocumentAST, options: &ConvertOptions) -> Result<Vec<u8>, CoreError> {
    let layout = PageLayout::from_options(options);
    let title = ast.title.as_deref().unwrap_or("Document Derovia");

    let (doc, page1, layer1) =
        printpdf::PdfDocument::new(title, Mm(layout.width_mm), Mm(layout.height_mm), "Contenu");

    let regular = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| {
        CoreError::failure("chargement_police", format!("Erreur police Helvetica : {e}"))
    })?;
    let bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| {
        CoreError::failure("chargement_police", format!("Erreur police Helvetica-Bold : {e}"))
    })?;

    let first_layer = doc.get_page(page1).get_layer(layer1);
    let mut writer = Writer::new(&doc, layout, first_layer);

    for block in &ast.blocks {
        match block {
            DocBlock::Heading { level, text } => {
                let (size, step) = match level {
                    1 => (20.0, 9.0),
                    2 => (15.0, 7.0),
                    _ => (12.0, 6.0),
                };
                writer.gap(step);
                for line in wrap_text(text, layout.max_chars(size, 0.0)) {
                    writer.line(&line, size, 0.0, &bold, step);
                }
                writer.gap(4.0);
            }
            DocBlock::Paragraph(text) => {
                for line in wrap_text(text, layout.max_chars(10.0, 0.0)) {
                    writer.line(&line, 10.0, 0.0, &regular, 5.0);
                }
                writer.gap(3.0);
            }
            DocBlock::ListItem(item) => {
                let lines = wrap_text(item, layout.max_chars(10.0, 6.0));
                for (index, line) in lines.iter().enumerate() {
                    // Seule la premiere ligne porte la puce ; les suivantes
                    // s'alignent dessous.
                    let rendered =
                        if index == 0 { format!("• {line}") } else { format!("  {line}") };
                    writer.line(&rendered, 10.0, 4.0, &regular, 5.0);
                }
            }
            DocBlock::Table(rows) => {
                for row in rows {
                    let joined = row.join("  |  ");
                    for line in wrap_text(&joined, layout.max_chars(9.0, 0.0)) {
                        writer.line(&line, 9.0, 0.0, &regular, 4.5);
                    }
                }
                writer.gap(4.0);
            }
        }
    }

    doc.save_to_bytes()
        .map_err(|e| CoreError::failure("encodage_pdf", format!("Erreur d'encodage PDF : {e}")))
}
