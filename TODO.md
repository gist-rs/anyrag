# TODO: Implement High-Performance Indexed Vector Search

This document outlines the steps required to upgrade the vector search functionality from the current exhaustive (brute-force) method to a high-performance, indexed-based (ANN) search.

These changes are dependent on a future version of the `turso` crate that includes support for the `vector_top_k` table-valued function.

---

### Prerequisite: Update Dependencies

Before proceeding, update the `turso` crate version in the following files to a version that supports `vector_top_k`.

-   `anyrag/crates/lib/Cargo.toml`
-   `anyrag/crates/server/Cargo.toml`

Update the dependency line, for example:
```toml
turso = { workspace = true } # Ensure the workspace version is updated to the new one
```

---

### Step 1: Update Database Schema for Vector Indexing

Modify the `create_table_if_not_exists` function in `anyrag/crates/lib/src/ingest/mod.rs` to define a dimension-specific vector column and create the vector index.

**File:** `anyrag/crates/lib/src/ingest/mod.rs`

```rust
// Replace the existing create_table_if_not_exists function body with this:

// The dimensionality of the vectors. This must match the output of the embedding model.
// For example, models like `all-MiniLM-L6-v2` produce 384-dimensional vectors.
const EMBEDDING_DIM: usize = 384; // Adjust this to your model's dimension

let table_sql = format!(
    "
    CREATE TABLE IF NOT EXISTS articles (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        link TEXT NOT NULL,
        description TEXT,
        embedding F32_BLOB({EMBEDDING_DIM}),
        content TEXT,
        pub_date TEXT,
        source_url TEXT,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    );
"
);
conn.execute(&table_sql, ()).await?;

// Create a standard B-Tree index on the `link` column to enforce uniqueness.
let unique_link_index_sql =
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_articles_link ON articles(link);";
conn.execute(unique_link_index_sql, ()).await?;

// Create a vector index on the `embedding` column for fast similarity searches.
// The `libsql_vector_idx` function is a special marker that tells libSQL to create an
// Approximate Nearest Neighbor (ANN) index instead of a standard B-Tree index.
info!("Creating vector index on articles table if it doesn't exist...");
let vector_index_sql =
    "CREATE INDEX IF NOT EXISTS articles_embedding_idx ON articles(libsql_vector_idx(embedding));";
conn.execute(vector_index_sql, ()).await?;
info!("Vector index is ready.");

Ok(())
```

---

### Step 2: Update Search Logic to Use the Index

Modify the `search_articles_by_embedding` function in `anyrag/crates/lib/src/embedding.rs` to use the much faster `vector_top_k` function.

**File:** `anyrag/crates/lib/src/embedding.rs`

```rust
// Replace the existing SQL query generation logic inside the `search_articles_by_embedding` function.

let vector_str = format!(
    "vector('[{}]')",
    query_vector
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .join(", ")
);

// This query uses the `vector_top_k` function to perform a fast, indexed search.
// It finds the `limit` candidate rows from the index and then joins them back to the
// `articles` table to retrieve the full article details.
let sql = format!(
    "SELECT
        a.title,
        a.link,
        a.description,
        vector_distance_cos(a.embedding, {vector_str}) AS distance
     FROM
        vector_top_k('articles_embedding_idx', {vector_str}, {limit}) AS v
     JOIN
        articles AS a ON a.rowid = v.rowid
     ORDER BY
        distance ASC;"
);

info!("Executing indexed vector search query.");
let mut results = conn.query(&sql, ()).await?;
// ... (the rest of the function remains the same)
```

---

### Step 3: Verify the Implementation

After making the code changes, run the test suite to ensure that the new implementation works correctly and that no regressions were introduced. Pay special attention to the embedding and search tests.

```sh
cargo test
```

If the `test_embed_and_search_flow` test and others pass, the feature has been successfully implemented.