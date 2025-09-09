//! # SQLite Specific SQL Queries & Schema
//!
//! This module centralizes SQL query strings and schema definitions for the SQLite provider.
//! This makes the core logic cleaner and isolates database-specific syntax.

// --- Core Schema Definitions (V2 - Normalized) ---

/// SQL to create the `users` table.
pub const CREATE_USERS_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY, -- The pseudonymized user ID
        role TEXT NOT NULL DEFAULT 'user',
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    );
";

/// SQL to create the `documents` table, the central source of truth for content.
pub const CREATE_DOCUMENTS_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS documents (
        id TEXT PRIMARY KEY,
        owner_id TEXT, -- Nullable for public content
        source_url TEXT UNIQUE,
        title TEXT,
        content TEXT NOT NULL,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        expires_at DATETIME,
        FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
    );
";

/// SQL to create the `document_embeddings` table, optimized for vector search.
pub const CREATE_DOCUMENT_EMBEDDINGS_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS document_embeddings (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        document_id TEXT NOT NULL,
        model_name TEXT NOT NULL,
        embedding BLOB NOT NULL,
        FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_embeddings_document_id ON document_embeddings(document_id);
";

/// SQL to create the `faq_items` table for structured Q&A data.
pub const CREATE_FAQ_ITEMS_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS faq_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        document_id TEXT NOT NULL,
        owner_id TEXT, -- Denormalized for efficient filtering
        question TEXT NOT NULL,
        answer TEXT NOT NULL,
        FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE,
        FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
    );
";

/// SQL to create the `content_metadata` table for fast, hybrid metadata filtering.
pub const CREATE_CONTENT_METADATA_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS content_metadata (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        document_id TEXT NOT NULL,
        owner_id TEXT, -- Denormalized for efficient filtering
        metadata_type TEXT NOT NULL, -- 'ENTITY', 'KEYPHRASE'
        metadata_subtype TEXT, -- e.g., 'PERSON', 'PRODUCT', 'CONCEPT'
        metadata_value TEXT NOT NULL,
        FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE,
        FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_metadata_value ON content_metadata(metadata_value);
    CREATE INDEX IF NOT EXISTS idx_metadata_owner_id ON content_metadata(owner_id);
";

/// An array containing all the schema creation SQL statements.
/// This allows them to be executed in order to set up a new database.
pub const ALL_TABLE_CREATION_SQL: &[&str] = &[
    CREATE_USERS_TABLE_SQL,
    CREATE_DOCUMENTS_TABLE_SQL,
    CREATE_DOCUMENT_EMBEDDINGS_TABLE_SQL,
    CREATE_FAQ_ITEMS_TABLE_SQL,
    CREATE_CONTENT_METADATA_TABLE_SQL,
];
