use super::{handlers::*, state::AppState};
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
        .route("/embed", post(embed_handler))
        .route("/embed/new", post(embed_new_handler))
        .route("/embed/faqs/new", post(embed_faqs_new_handler))
        .route("/search/vector", post(vector_search_handler))
        .route("/search/keyword", post(keyword_search_handler))
        .route("/search/hybrid", post(hybrid_search_handler))
        .route("/search/knowledge", post(knowledge_search_handler))
        .route("/knowledge/ingest", post(knowledge_ingest_handler))
        .route("/knowledge/export", get(knowledge_export_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}
