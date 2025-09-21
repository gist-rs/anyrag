//! # gof: A CLI for Project-Aware RAG
//!
//! This is the main entry point for the `gof` command-line interface.
//! In adherence with project rules, this binary is a "thin entrypoint."
//! All logic is delegated to the `gof` library crate.

use anyhow::Result;
use clap::Parser;
use gof::{run, Cli};
use tracing_subscriber::{fmt, EnvFilter};

// --- Main Application Entry ---

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup logging
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive("gof=info".parse()?))
        .with_ansi(false) // Make logs clean for file output or CI
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // 2. Parse CLI arguments
    let cli = Cli::parse();

    // 3. Call the library's run function and handle the final result
    if let Err(e) = run(cli).await {
        // Use debug formatting for more detailed error context
        eprintln!("[gof error] Failed to execute command: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
}
