use super::{handlers, state::AppState};
use axum::extract::DefaultBodyLimit;
use axum::{
    routing::{delete, get, post},
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
        .route("/embed/new", post(handlers::embed_new_handler))
        .route("/search/vector", post(handlers::vector_search_handler))
        .route("/search/keyword", post(handlers::keyword_search_handler))
        .route("/search/hybrid", post(handlers::hybrid_search_handler))
        .route(
            "/search/knowledge",
            post(handlers::knowledge_search_handler),
        )
        .route("/knowledge/export", get(handlers::knowledge_export_handler))
        // --- Slot Management & Routed Search Routes ---
        // --- Domain Classification Route ---
        .route("/classify/domain", post(handlers::classify_domain_handler))
        // --- Catalog-Driven Domain Shaping Routes (Plan 007) ---
        .route("/v1/models", get(handlers::list_domain_models_handler))
        .route(
            "/v1/models/{domain}",
            get(handlers::get_domain_model_handler),
        )
        .route("/v1/tokenize", post(handlers::tokenize_handler))
        .route("/v1/detokenize", post(handlers::detokenize_handler))
        // --- Slot Management & Routed Search Routes ---
        .route("/search/slots", post(handlers::slot_search_handler))
        .route("/slots", get(handlers::list_slots_handler))
        .route("/slots", post(handlers::create_slot_handler))
        .route("/slots/reindex", post(handlers::reindex_slots_handler))
        .route(
            "/slots/{name}/documents",
            get(handlers::list_slot_documents_handler),
        )
        .route(
            "/slots/{name}/documents/{doc_id}",
            delete(handlers::remove_document_from_slot_handler),
        )
        // --- Episodes & Self-Improving Cycle Routes ---
        .route(
            "/episodes",
            post(handlers::episodes::record_episode_handler),
        )
        .route("/episodes", get(handlers::episodes::list_episodes_handler))
        .route(
            "/episodes/stats",
            get(handlers::episodes::episode_stats_handler),
        )
        .route(
            "/episodes/{id}/verify",
            post(handlers::episodes::verify_episode_handler),
        )
        .route(
            "/cycle/status",
            get(handlers::episodes::cycle_status_handler),
        )
        .route(
            "/cycle/trigger",
            post(handlers::episodes::cycle_trigger_handler),
        );

    // Conditionally add routes by re-binding the router variable.
    // This avoids the `unused_mut` warning when no features are enabled.
    let mut router = router;

    #[cfg(feature = "text")]
    {
        router = router.route(
            "/ingest/text",
            post(handlers::ingest::text::ingest_text_handler),
        );
    }

    #[cfg(feature = "pdf")]
    {
        router = router.route(
            "/ingest/pdf",
            post(handlers::ingest::pdf::ingest_pdf_handler)
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        );
    }

    #[cfg(feature = "sheets")]
    {
        router = router.route(
            "/ingest/sheet",
            post(handlers::ingest::sheet::ingest_sheet_handler),
        );
    }

    #[cfg(feature = "web")]
    {
        router = router.route(
            "/ingest/web",
            post(handlers::ingest::web::ingest_web_handler),
        );
    }

    #[cfg(feature = "github")]
    {
        router = router
            .route(
                "/ingest/github",
                post(handlers::ingest::github::ingest_github_handler),
            )
            .route(
                "/examples/{repo_name}",
                get(handlers::ingest::github::get_latest_examples_handler),
            )
            .route(
                "/examples/{repo_name}/{version}",
                get(handlers::ingest::github::get_versioned_examples_handler),
            )
            .route(
                "/search/examples",
                post(handlers::ingest::github::search_examples_handler),
            );
    }

    #[cfg(feature = "rss")]
    {
        router = router.route(
            "/ingest/rss",
            post(handlers::ingest::rss::ingest_rss_handler),
        );
    }

    #[cfg(feature = "firebase")]
    {
        router = router.route(
            "/ingest/firebase",
            post(handlers::ingest::firebase::ingest_firebase_handler),
        );
    }

    #[cfg(feature = "graph_db")]
    {
        router = router
            .route(
                "/search/knowledge_graph",
                post(handlers::knowledge_graph_search_handler),
            )
            .route("/graph/build", post(handlers::graph_build_handler));
    }

    router
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}
