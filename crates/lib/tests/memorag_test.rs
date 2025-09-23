//! # MemoRag E2E Test: From Contradiction to Consolidation
//!
//! This integration test implements the exact scenario described in `MEMORAG_PLAN.md`
//! to provide a concrete, end-to-end validation of the "memory stream" concept.
//!
//! ## Test Scenario: Evolving Information
//!
//! 1.  **Ingest Day 1:** A product page for "WidgetPro" is ingested with a price of $99.
//! 2.  **Ingest Day 2:** The page is updated, and the new version is ingested with a price of $119.
//! 3.  **Ingest Day 3:** A final version is ingested with the price at $129.
//!
//! ## Verification Steps
//!
//! 1.  **Baseline Test (Failure Case):**
//!     - Ask the RAG system for the price of "WidgetPro".
//!     - **Assert:** The system, confused by three conflicting documents, provides an
//!       ambiguous answer containing multiple prices.
//!
//! 2.  **Curator Execution:**
//!     - The `Curator` service is run.
//!     - The `Curator` identifies the three versions of the "WidgetPro" page as related.
//!     - It uses an LLM to synthesize them into a single, new, consolidated memory that
//!       states the final price is $129.
//!     - This new memory is ingested back into the knowledge base, and the old versions are deleted.
//!
//! 3.  **MemoRag Test (Success Case):**
//!     - Ask the RAG system the same question again.
//!     - **Assert:** The system now retrieves the new, synthesized memory as the top result
//!       and provides a single, correct, and confident answer: "The price of WidgetPro is $129."

mod common;

use anyhow::Result;
use anyrag::{
    curator::Curator,
    ingest::{IngestionPrompts, Ingestor},
    providers::{ai::AiProvider, db::sqlite::SqliteProvider},
    search::{hybrid_search, HybridSearchOptions, HybridSearchPrompts},
    types::{ContentType, ExecutePromptOptions, PromptClientBuilder, SearchResult},
};
use anyrag_web::{WebIngestStrategy, WebIngestor};
use common::{setup_tracing, MockAiProvider};
use core_access::get_or_create_user;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::info;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

const QUESTION: &str = "What is the price of WidgetPro?";
const MOCK_PROMPTS: IngestionPrompts<'_> = IngestionPrompts {
    restructuring_system_prompt: "Restructure this content.",
    metadata_extraction_system_prompt: "Extract metadata from this content.",
};

#[tokio::test]
async fn test_memorag_e2e_scenario() -> Result<()> {
    // --- 1. Arrange ---
    setup_tracing();
    let db_provider = SqliteProvider::new(":memory:").await?;
    db_provider.initialize_schema().await?;
    let user = get_or_create_user(&db_provider.db, "test@example.com", None).await?;
    let server = MockServer::start().await;
    let widget_pro_url = server.uri() + "/widget-pro";

    // --- AI Response Queue for the Entire Test Flow ---
    let mock_ai_responses = vec![
        // Ingestion Day 1
        "sections:\n  - title: 'Day 1'\n    faqs:\n      - question: 'Price'\n        answer: 'The price is $99.'".to_string(), // Restructure
        json!([{"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "WidgetPro"}]).to_string(), // Metadata
        // Ingestion Day 2
        "sections:\n  - title: 'Day 2'\n    faqs:\n      - question: 'Price'\n        answer: 'The price is $119.'".to_string(), // Restructure
        json!([{"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "WidgetPro"}]).to_string(), // Metadata
        // Ingestion Day 3
        "sections:\n  - title: 'Day 3'\n    faqs:\n      - question: 'Price'\n        answer: 'The price is $129.'".to_string(), // Restructure
        json!([{"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "WidgetPro"}]).to_string(), // Metadata
        // Baseline RAG
        json!({"entities": [], "keyphrases": ["WidgetPro"]}).to_string(), // Query Analysis
        "The price is listed as $99, $119, and $129.".to_string(), // Synthesis
        // Curator
        "The current price for WidgetPro is $129.".to_string(), // Synthesis
        // MemoRag RAG
        json!({"entities": [], "keyphrases": ["WidgetPro"]}).to_string(), // Query Analysis
        "The price of WidgetPro is $129.".to_string(), // Synthesis
    ];

    let mock_ai_provider: Arc<dyn AiProvider> = Arc::new(MockAiProvider::new(mock_ai_responses));
    let ingestor = WebIngestor::new(&db_provider.db, mock_ai_provider.as_ref(), MOCK_PROMPTS);

    // --- 2. Ingest Evolving Data (Day 1, 2, 3) ---
    // Use the now-fixed WebIngestor to create the versioned documents.
    let source_json =
        json!({ "url": widget_pro_url, "strategy": WebIngestStrategy::RawHtml }).to_string();

    Mock::given(method("GET"))
        .and(path("/widget-pro"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Price is $99"))
        .mount(&server)
        .await;
    ingestor.ingest(&source_json, Some(&user.id)).await?;
    sleep(Duration::from_millis(10)).await;

    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/widget-pro"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Price is $119"))
        .mount(&server)
        .await;
    ingestor.ingest(&source_json, Some(&user.id)).await?;
    sleep(Duration::from_millis(10)).await;

    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/widget-pro"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Price is $129"))
        .mount(&server)
        .await;
    ingestor.ingest(&source_json, Some(&user.id)).await?;

    // --- 3. Baseline Test (Expect Failure) ---
    info!("--- Running Baseline RAG Test (Expecting Ambiguity) ---");
    let (baseline_context, baseline_results) =
        run_hybrid_search(&db_provider, mock_ai_provider.clone(), QUESTION, &user.id).await?;
    assert_eq!(
        baseline_results.len(),
        3,
        "Should find all 3 versions before curation."
    );
    let baseline_answer =
        run_synthesis(mock_ai_provider.clone(), QUESTION, &baseline_context).await?;
    info!("Baseline Answer: {}", baseline_answer);
    assert!(
        baseline_answer.contains("$99")
            && baseline_answer.contains("$119")
            && baseline_answer.contains("$129"),
        "Baseline answer should be ambiguous and contain all three prices."
    );

    // --- 4. Run the Curator ---
    info!("--- Running the Curator to Synthesize Knowledge ---");
    let curator = Curator::new(&db_provider, mock_ai_provider.as_ref());
    curator
        .synthesize_by_source(&widget_pro_url, &user.id)
        .await?;

    // --- 5. MemoRag Test (Expect Success) ---
    info!("--- Running MemoRag RAG Test (Expecting Correctness) ---");
    let (memorag_context, memorag_results) =
        run_hybrid_search(&db_provider, mock_ai_provider.clone(), QUESTION, &user.id).await?;
    assert_eq!(
        memorag_results.len(),
        1,
        "Should only find one document after curation."
    );
    assert!(
        memorag_results[0].title.contains("Synthesis of"),
        "The top search result should be the synthesized document."
    );
    let memorag_answer =
        run_synthesis(mock_ai_provider.clone(), QUESTION, &memorag_context).await?;
    info!("MemoRag Answer: {}", memorag_answer);
    assert!(
        memorag_answer.contains("$129"),
        "MemoRag answer should contain the final price."
    );
    assert!(
        !memorag_answer.contains("$99") && !memorag_answer.contains("$119"),
        "MemoRag answer should NOT contain the old prices."
    );

    Ok(())
}

// --- Helper Functions ---
async fn run_hybrid_search(
    sqlite_provider: &SqliteProvider,
    ai_provider: Arc<dyn AiProvider>,
    question: &str,
    user_id: &str,
) -> Result<(String, Vec<SearchResult>)> {
    let storage_provider_arc = Arc::new(sqlite_provider.clone());
    let search_options = HybridSearchOptions {
        query_text: question.to_string(),
        owner_id: Some(user_id.to_string()),
        limit: 5,
        prompts: HybridSearchPrompts {
            analysis_system_prompt: "Analyze this query.",
            analysis_user_prompt_template: "{prompt}",
        },
        use_keyword_search: true,
        use_vector_search: false,
        embedding_api_url: "",
        embedding_model: "",
        embedding_api_key: None,
        temporal_ranking_config: None,
    };
    let search_results = hybrid_search(storage_provider_arc, ai_provider, search_options).await?;
    let context = search_results
        .iter()
        .map(|r| r.description.clone())
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    Ok((context, search_results))
}

async fn run_synthesis(
    ai_provider: Arc<dyn AiProvider>,
    question: &str,
    context: &str,
) -> Result<String> {
    let client = PromptClientBuilder::new()
        .ai_provider(dyn_clone::clone_box(ai_provider.as_ref()))
        .storage_provider(Box::new(SqliteProvider::new(":memory:").await.unwrap()))
        .build()?;
    let options = ExecutePromptOptions {
        prompt: question.to_string(),
        content_type: Some(ContentType::Knowledge),
        context: Some(context.to_string()),
        ..Default::default()
    };
    let result = client.execute_prompt_with_options(options).await?;
    Ok(result.text)
}
