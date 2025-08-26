//! # Vector Search Logic Test
//!
//! This file contains a focused integration test to verify the correctness of the
//! `search_by_vector` function, isolated from any external embedding models.

use anyhow::Result;
use anyrag::{
    providers::{db::sqlite::SqliteProvider, db::storage::VectorSearch},
    search::SearchError,
};
use turso::params;

/// A helper function to set up an in-memory database with manually inserted,
/// distinct vectors. This removes any dependency on an embedding model.
async fn setup_database_with_manual_vectors() -> Result<(SqliteProvider, Vec<f32>, Vec<f32>)> {
    // 1. Create a new, isolated in-memory database.
    let provider = SqliteProvider::new(":memory:").await?;
    let conn = provider
        .db
        .connect()
        .expect("Failed to get connection for test setup");

    // 2. Define two perfectly distinct vectors.
    let qwen3_vector: Vec<f32> = vec![0.0, 1.0, 0.0, 0.0];
    let rust_vector: Vec<f32> = vec![0.0, 0.0, 1.0, 0.0];

    // 3. Convert vectors to byte slices for BLOB storage.
    // This uses the same unsafe block as the real `embed_article` function
    // to ensure the storage format is identical.
    let qwen3_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(qwen3_vector.as_ptr() as *const u8, qwen3_vector.len() * 4)
    };
    let rust_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(rust_vector.as_ptr() as *const u8, rust_vector.len() * 4)
    };

    // 4. Manually insert the data.
    conn.execute(
        "CREATE TABLE articles (id INTEGER, title TEXT, link TEXT, description TEXT, embedding BLOB);",
        (),
    ).await?;

    conn.execute(
        "INSERT INTO articles (id, title, link, description, embedding) VALUES (?, ?, ?, ?, ?)",
        params![
            1,
            "The Rise of Qwen3",
            "http://mock.com/qwen3",
            "An article about Qwen3.",
            qwen3_bytes
        ],
    )
    .await?;

    conn.execute(
        "INSERT INTO articles (id, title, link, description, embedding) VALUES (?, ?, ?, ?, ?)",
        params![
            2,
            "Web Apps with Rust",
            "http://mock.com/rust",
            "An article about Rust.",
            rust_bytes
        ],
    )
    .await?;

    Ok((provider, qwen3_vector, rust_vector))
}

/// This test verifies the core logic of `search_by_vector`.
///
/// It uses a manually prepared database with perfect, orthogonal vectors to ensure
/// that if the search function is given an exact vector, it returns the correct
/// article with a perfect similarity score of 1.0, proving the underlying SQL and
/// data handling are correct.
#[tokio::test]
async fn test_vector_search_logic_is_correct() -> Result<(), SearchError> {
    // --- Arrange ---
    // Setup the database and get back the provider and the exact vectors used.
    let (provider, qwen3_vector, _) = setup_database_with_manual_vectors()
        .await
        .expect("Database setup with manual vectors failed");
    println!("Database setup complete with manual vectors.");

    // --- Act ---
    // Search for the article using the *exact* Qwen3 vector.
    println!("Executing vector_search with the perfect Qwen3 vector...");
    let search_results = provider.vector_search(qwen3_vector, 5).await?;

    // --- Assert ---
    println!("Search results received: {search_results:?}");

    // 1. The search must return exactly one result for a perfect match.
    assert_eq!(
        search_results.len(),
        1,
        "Expected exactly one search result for a perfect vector match."
    );

    let top_result = &search_results[0];

    // 2. The result must be the correct article.
    assert_eq!(
        top_result.title, "The Rise of Qwen3",
        "The returned article is not the one we searched for."
    );

    // 3. The score (similarity) must be 1.0 for a perfect match.
    // We use a small epsilon for floating-point comparison.
    assert!(
        (1.0 - top_result.score).abs() < 1e-9,
        "Expected a perfect similarity score (1.0), but got {}",
        top_result.score
    );

    println!("Assertions passed. Vector search logic is correct.");
    Ok(())
}
