use anyhow::Result;
use printpdf::{
    BuiltinFont, Layer, Mm, Op, ParsedFont, PdfDocument, PdfPage, PdfSaveOptions, Pt, TextItem,
    TextMatrix, TextRenderingMode,
};

/// Generates a simple, single-page PDF with the given text content, compatible with printpdf v0.8.2.
pub fn generate_test_pdf(text: &str) -> Result<Vec<u8>> {
    let mut doc = PdfDocument::new("Test PDF");
    let mut page = PdfPage::new(Mm(210.0), Mm(297.0), vec![]);
    let layer_def = Layer::new("Layer 1");
    let layer_id = doc.add_layer(&layer_def);

    // Get the font bytes for a built-in font and parse it.
    let font_bytes = BuiltinFont::Helvetica.get_subset_font().bytes;
    let font = ParsedFont::from_bytes(&font_bytes, 0, &mut Vec::new())
        .ok_or_else(|| anyhow::anyhow!("Failed to parse built-in font"))?;
    let font_id = doc.add_font(&font);

    let ops = vec![
        Op::BeginLayer {
            layer_id: layer_id.clone(),
        },
        Op::SetFontSize {
            size: Pt(12.0),
            font: font_id.clone(),
        },
        Op::StartTextSection,
        Op::SetTextMatrix {
            matrix: TextMatrix::Translate(Mm(10.0).into(), Mm(280.0).into()),
        },
        Op::SetTextRenderingMode {
            mode: TextRenderingMode::Fill,
        },
        Op::WriteText {
            items: vec![TextItem::Text(text.to_string())],
            font: font_id,
        },
        Op::EndTextSection,
        Op::EndLayer { layer_id },
    ];

    page.ops = ops;
    doc.pages.push(page);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    if !warnings.is_empty() {
        // In a test context, it's fine to just print warnings.
        eprintln!("PDF generation warnings: {warnings:?}");
    }

    Ok(bytes)
}
