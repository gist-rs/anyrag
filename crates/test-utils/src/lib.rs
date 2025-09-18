use anyhow::Result;
use anyrag::errors::PromptError;
use anyrag::providers::ai::AiProvider;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use turso::Database;

// --- Test Setup ---

/// A helper struct to manage database creation for each test.
pub struct TestSetup {
    pub db: Database,
}

impl TestSetup {
    /// Creates a new, isolated in-memory database and initializes the schema.
    pub async fn new() -> Result<Self> {
        let db = turso::Builder::new_local(":memory:").build().await?;
        let conn = db.connect()?;

        // Initialize the schema using the shared SQL constants.
        for statement in anyrag::providers::db::sqlite::sql::ALL_TABLE_CREATION_SQL {
            conn.execute(statement, ()).await?;
        }

        Ok(Self { db })
    }
}

// --- Mock AI Provider ---

#[derive(Clone, Debug)]
pub struct MockAiProvider {
    responses: Arc<Mutex<HashMap<String, String>>>,
    calls: Arc<Mutex<Vec<(String, String)>>>,
}

impl MockAiProvider {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Pre-programs a response for a specific prompt.
    /// The key should be a unique substring of the system prompt.
    pub fn add_response(&self, key: &str, response: &str) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(key.to_string(), response.to_string());
    }

    /// Retrieves the recorded calls for assertion.
    pub fn get_calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }
}

impl Default for MockAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, PromptError> {
        let mut calls = self.calls.lock().unwrap();
        calls.push((system_prompt.to_string(), user_prompt.to_string()));

        let responses = self.responses.lock().unwrap();
        for (key, response) in responses.iter() {
            if system_prompt.contains(key) {
                return Ok(response.clone());
            }
        }

        Err(PromptError::AiApi(format!(
            "MockAiProvider: No response programmed for system prompt. Got: '{system_prompt}'"
        )))
    }
}

// --- Test-Specific Helpers ---
#[cfg(feature = "pdf")]
pub mod helpers {
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
}
