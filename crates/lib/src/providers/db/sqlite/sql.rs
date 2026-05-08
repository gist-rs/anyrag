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
        source_url TEXT,
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

/// SQL to create the `episodes` table for RIIR translation episodic memory.
pub const CREATE_EPISODES_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS episodes (
        id TEXT PRIMARY KEY,
        source_language TEXT NOT NULL,
        source_code TEXT NOT NULL,
        generated_rust TEXT NOT NULL,
        retrieved_context TEXT,         -- JSON array of SearchResult
        hidden_state TEXT,              -- JSON array of f64 (embedding vector)
        compilation_result TEXT NOT NULL, -- JSON: CompilationResult enum
        created_at TEXT NOT NULL,
        synthesized BOOLEAN DEFAULT FALSE
    );
    CREATE INDEX IF NOT EXISTS idx_episodes_compilation ON episodes(compilation_result);
    CREATE INDEX IF NOT EXISTS idx_episodes_created ON episodes(created_at);
    CREATE INDEX IF NOT EXISTS idx_episodes_synthesized ON episodes(synthesized);
";

/// SQL to create the `rag_slots` table for Raven Routed Slot Memory.
/// Named slots for categorizing ingested documents into bounded memory partitions.
pub const CREATE_RAG_SLOTS_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS rag_slots (
        name TEXT PRIMARY KEY,              -- SlotName enum as snake_case string
        description TEXT NOT NULL DEFAULT '',
        is_frozen BOOLEAN NOT NULL DEFAULT FALSE,
        decay_rate REAL NOT NULL DEFAULT 0.1,  -- λ (lambda), Raven Eq. 18
        max_documents INTEGER NOT NULL DEFAULT 1000,
        keywords TEXT NOT NULL DEFAULT '[]',    -- JSON array of routing keywords
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_rag_slots_frozen ON rag_slots(is_frozen);
";

/// SQL to create the `slot_documents` table for document-to-slot associations.
/// Many-to-many relationship between documents and slots.
pub const CREATE_SLOT_DOCUMENTS_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS slot_documents (
        id TEXT PRIMARY KEY,                -- UUID v7
        slot_name TEXT NOT NULL,            -- FK to rag_slots.name
        document_id TEXT NOT NULL,          -- FK to documents.id
        routed_by TEXT NOT NULL DEFAULT 'keyword',  -- RouteMethod enum
        routed_at TEXT NOT NULL DEFAULT (datetime('now')),
        relevance_score REAL NOT NULL DEFAULT 1.0,
        FOREIGN KEY (slot_name) REFERENCES rag_slots(name) ON DELETE CASCADE,
        FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_slot_documents_slot ON slot_documents(slot_name);
    CREATE INDEX IF NOT EXISTS idx_slot_documents_document ON slot_documents(document_id);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_slot_documents_unique ON slot_documents(slot_name, document_id);
";

/// An array containing all the schema creation SQL statements.
/// This allows them to be executed in order to set up a new database.
pub const ALL_TABLE_CREATION_SQL: &[&str] = &[
    CREATE_USERS_TABLE_SQL,
    CREATE_DOCUMENTS_TABLE_SQL,
    CREATE_DOCUMENT_EMBEDDINGS_TABLE_SQL,
    CREATE_CONTENT_METADATA_TABLE_SQL,
    CREATE_EPISODES_TABLE_SQL,
    CREATE_RAG_SLOTS_TABLE_SQL,
    CREATE_SLOT_DOCUMENTS_TABLE_SQL,
];
