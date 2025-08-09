//! # Common Test Utilities
//!
//! This module provides shared utilities for testing, such as mock servers
//! and mock providers, to ensure tests are isolated and repeatable.

use dotenvy::dotenv;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initializes the tracing subscriber and loads .env for tests.
pub fn setup_tracing() {
    INIT.call_once(|| {
        dotenv().ok();
        tracing_subscriber::fmt::init();
    });
}
