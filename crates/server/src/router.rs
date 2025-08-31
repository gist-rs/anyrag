use super::{handlers::*, state::AppState};
use axum::extract::DefaultBodyLimit;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

/// Creates the Axum router with all the application routes.
pub fn create_router(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/prompt", post(prompt_handler))
        .route("/ingest", post(ingest_handler))
        .route("/ingest/text", post(ingest_text_handler))
        .route(
            "/ingest/file",
            post(ingest_file_handler).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route("/ingest/pdf_url", post(ingest_pdf_url_handler))
        .route("/ingest/sheet_faq", post(ingest_sheet_faq_handler))
        .route("/embed/new", post(embed_new_handler))
        .route("/search/vector", post(vector_search_handler))
        .route("/search/keyword", post(keyword_search_handler))
        .route("/search/knowledge", post(knowledge_search_handler))
        .route("/knowledge/ingest", post(knowledge_ingest_handler))
        .route("/knowledge/export", get(knowledge_export_handler))
        .route(
            "/search/knowledge_graph",
            post(knowledge_graph_search_handler),
        )
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}
