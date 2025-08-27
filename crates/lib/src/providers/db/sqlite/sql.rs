//! # SQLite Specific SQL Queries
//!
//! This module centralizes SQL query strings for the SQLite provider.
//! This makes the core logic cleaner and isolates database-specific syntax.

/// Returns the SQL query for performing a keyword search on the `articles` table.
///
/// The query uses `LOWER()` for case-insensitive matching and expects a single
/// parameter (`?1`) for the search pattern (e.g., `%keyword%`).
///
/// # Arguments
///
/// * `limit`: The maximum number of results to return.
pub fn keyword_search_articles(limit: u32) -> String {
    format!(
        "
        SELECT title, link, description, 0.0 as score
        FROM articles
        WHERE LOWER(title) LIKE ?1 OR LOWER(description) LIKE ?1
        LIMIT {limit};
    "
    )
}

/// Returns the SQL for a keyword search on the `faq_kb` table.
///
/// The query uses `LOWER()` for case-insensitive matching on both the `question`
/// and `answer` columns. It expects a single parameter (`?1`) for the search pattern.
///
/// # Arguments
///
/// * `limit`: The maximum number of results to return.
pub fn keyword_search_faqs(limit: u32) -> String {
    format!(
        "
        SELECT answer, 0.5 as score
        FROM faq_kb
        WHERE LOWER(question) LIKE ?1 OR LOWER(answer) LIKE ?1
        LIMIT {limit};
    "
    )
}
