use super::{handlers, state::AppState};
use axum::extract::DefaultBodyLimit;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

/// Creates the Axum router with all the application routes.
pub fn create_router(app_state: AppState) -> Router {
    let router = Router::new()
        .route("/", get(handlers::root))
        .route("/health", get(handlers::health_check))
        .route("/documents", get(handlers::get_documents_handler))
        // --- OAuth 2.0 Authentication Routes ---
        .route("/auth/login/google", get(handlers::google_login_handler))
        .route(
            "/auth/callback/google",
            get(handlers::google_auth_callback_handler),
        )
        .route("/auth/me", get(handlers::get_me_handler))
        .route("/users", get(handlers::get_users_handler))
        .route("/prompt", post(handlers::prompt_handler))
        .route("/db/query", post(handlers::db_query_handler))
        .route("/gen/text", post(handlers::gen_text_handler))
        .route("/ingest/text", post(handlers::ingest_text_handler))
        .route(
            "/ingest/pdf",
            post(handlers::ingest_pdf_handler).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route("/ingest/sheet", post(handlers::ingest_sheet_handler))
        .route("/ingest/web", post(handlers::ingest_web_handler))
        .route("/ingest/github", post(handlers::ingest_github_handler))
        .route(
            "/examples/{repo_name}",
            get(handlers::get_latest_examples_handler),
        )
        .route(
            "/examples/{repo_name}/{version}",
            get(handlers::get_versioned_examples_handler),
        )
        .route("/embed/new", post(handlers::embed_new_handler))
        .route("/search/vector", post(handlers::vector_search_handler))
        .route("/search/keyword", post(handlers::keyword_search_handler))
        .route("/search/hybrid", post(handlers::hybrid_search_handler))
        .route(
            "/search/knowledge",
            post(handlers::knowledge_search_handler),
        )
        .route("/search/examples", post(handlers::search_examples_handler))
        .route("/knowledge/export", get(handlers::knowledge_export_handler));

    // Conditionally add routes by re-binding the router variable.
    // This avoids the `unused_mut` warning when no features are enabled.
    let router = {
        #[cfg(feature = "rss")]
        let router = router.route("/ingest/rss", post(handlers::ingest_rss_handler));

        #[cfg(feature = "firebase")]
        let router = router.route("/ingest/firebase", post(handlers::ingest_firebase_handler));

        #[cfg(feature = "graph_db")]
        let router = router
            .route(
                "/search/knowledge_graph",
                post(handlers::knowledge_graph_search_handler),
            )
            .route("/graph/build", post(handlers::graph_build_handler));

        router
    };

    router
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}
