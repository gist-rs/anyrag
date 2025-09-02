use super::{handlers, state::AppState};
use axum::extract::DefaultBodyLimit;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

/// Creates the Axum router with all the application routes.
pub fn create_router(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::root))
        .route("/health", get(handlers::health_check))
        .route("/prompt", post(handlers::prompt_handler))
        .route("/ingest", post(handlers::ingest_handler))
        .route("/ingest/text", post(handlers::ingest_text_handler))
        .route(
            "/ingest/file",
            post(handlers::ingest_file_handler).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route("/ingest/pdf_url", post(handlers::ingest_pdf_url_handler))
        .route(
            "/ingest/sheet_faq",
            post(handlers::ingest_sheet_faq_handler),
        )
        .route("/embed/new", post(handlers::embed_new_handler))
        .route("/search/vector", post(handlers::vector_search_handler))
        .route("/search/keyword", post(handlers::keyword_search_handler))
        .route("/search/hybrid", post(handlers::hybrid_search_handler))
        .route(
            "/search/knowledge",
            post(handlers::knowledge_search_handler),
        )
        .route(
            "/knowledge/ingest",
            post(handlers::knowledge_ingest_handler),
        )
        .route("/knowledge/export", get(handlers::knowledge_export_handler))
        .route(
            "/search/knowledge_graph",
            post(handlers::knowledge_graph_search_handler),
        )
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}
