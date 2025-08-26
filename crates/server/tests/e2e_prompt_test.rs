//! # End-to-End Prompt Execution Tests

// Include the binary's main source file to access its components.
#[path = "../src/main.rs"]
mod main;

use reqwest::Client;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

/// Spawns the application in the background for testing.
async fn spawn_app() -> String {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .try_init();

    // Use a unique in-memory database for each test run to avoid conflicts.
    let db_url = ":memory:";
    std::env::set_var("DB_URL", db_url);

    let config = main::config::get_config().expect("Failed to read configuration for test");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    tokio::spawn(async move {
        if let Err(e) = main::run(listener, config).await {
            eprintln!("Server error: {e}");
        }
    });

    sleep(Duration::from_millis(100)).await;

    address
}

#[tokio::test]
async fn test_e2e_prompt_execution() {
    let address = spawn_app().await;
    let client = Client::new();

    let payload = json!({
        "prompt": "What is the total word_count for the corpus 'kinghenryv'?",
        "table_name": "bigquery-public-data.samples.shakespeare",
        "instruction": "Answer with only the number, with thousand format."
    });

    let response = client
        .post(format!("{address}/prompt?debug=true"))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(
        response.status().is_success(),
        "Request failed with status: {}. Body: {:?}",
        response.status(),
        response.text().await
    );

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse response JSON");

    // The result can be a string or a number, so handle both cases.
    let result_value = &body["result"]["text"];
    let result = if result_value.is_string() {
        result_value.as_str().unwrap().to_owned()
    } else {
        result_value.to_string()
    };

    // Also verify that the debug field is present.
    assert!(body["debug"].is_object(), "Debug field should be present");

    println!("E2E Test Response from server: '{result}'");
    assert!(
        result.contains("27,894"),
        "Response did not contain the expected result."
    );
}
