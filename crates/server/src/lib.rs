pub mod auth;
pub mod config;
pub mod errors;
pub mod handlers;
pub mod router;
pub mod state;
pub mod types;

use crate::{
    config::{get_config, Config},
    router::create_router,
    state::build_app_state,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, info};
use tracing_subscriber::FmtSubscriber;

/// Configures and runs the web server.
///
/// This function initializes the application state, creates the router,
/// and starts the Axum server.
pub async fn run(listener: TcpListener, config: Config) -> anyhow::Result<()> {
    debug!(?config, "Server configuration loaded");

    let app_state = build_app_state(config).await?;
    let app = create_router(app_state);

    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

/// The library's main entry point.
///
/// Sets up logging, configuration, and the TCP listener, then calls `run`.
pub async fn start() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config = get_config()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);

    run(listener, config).await
}
