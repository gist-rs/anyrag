pub mod config;
mod errors;
pub mod handlers;
pub mod router;
pub mod state;

pub use self::router::create_router;
use self::{
    config::{get_config, Config},
    state::build_app_state,
};
use std::net::SocketAddr;
use tracing::{debug, info};
use tracing_subscriber::FmtSubscriber;

/// The main entry point for running the server.
///
/// This function initializes the application state, creates the router,
/// and starts the Axum server.
pub async fn run(listener: tokio::net::TcpListener, config: Config) -> anyhow::Result<()> {
    debug!(?config, "Server configuration loaded");

    let app_state = build_app_state(config).await?;
    let app = create_router(app_state);

    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

/// The main function of the application.
///
/// This function is responsible for setting up the environment, loading configuration,
/// initializing the logger, binding to a TCP port, and starting the server.
#[tokio::main]
#[cfg_attr(test, allow(dead_code))]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from a .env file if it exists.
    dotenvy::dotenv().ok();

    // Initialize the tracing subscriber for logging.
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Load the application configuration from environment variables.
    let config = get_config()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);

    // Run the server.
    run(listener, config).await
}
