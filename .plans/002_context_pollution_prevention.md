# Plan 002: Context Pollution Prevention + Concept Sharding

## Objective

Prevent RIIR-specific context pollution by weighting code search results higher than documentation results, and add concept-level query routing to improve retrieval precision for Rust-specific queries.

## The Problem

### Context Pollution

When using anyrag for "Rewrite it in Rust" tasks, the RAG pipeline returns mixed results:
- Code examples from `/search/examples` → useful, executable, correct
- Documentation from `/search/knowledge` → explanatory, not executable
- The synthesis LLM receives both → sometimes generates prose instead of code

The research warns: "anyrag must heavily weight the Code RAG (`/search/examples`) over standard documentation when doing RIIR to prevent the LLM from generating explanatory text instead of executable code."

### No Concept Sharding

All vectors go to a single SQLite database. Queries about "lifetimes" are mixed with queries about "macros" and "async". The research calls for concept sharding: "queries are classified by anyrag's Query Analysis (e.g., 'Lifetimes', 'Macros') and routed to specific, smaller shards."

## Architecture

### Part 1: Source-Type Tagging + Weighted RRF

```rust
// crates/lib/src/types.rs — extend SearchResult

pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub description: String,
    pub score: f64,
    // NEW: source type for weighted fusion
    pub source_type: SearchSourceType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchSourceType {
    Code,        // from /search/examples or code ingestion
    Documentation, // from /search/knowledge (web, PDF, text)
    Faq,         // from structured YAML FAQ
    Unknown,     // legacy untagged results
}
```

```rust
// crates/lib/src/rerank.rs — weighted RRF

pub struct RrfWeights {
    pub metadata: f64,     // default: 100.0 (current hardcoded value)
    pub vector: f64,       // default: 1.0
    pub keyword: f64,      // default: 1.0
    pub code_boost: f64,   // default: 10.0 — boost code results
    pub doc_penalty: f64,  // default: 0.5 — dampen doc results for code queries
}

pub fn reciprocal_rank_fusion_weighted(
    result_sets: &[Vec<SearchResult>],
    weights: &RrfWeights,
) -> Vec<SearchResult> {
    // Same RRF logic but:
    // 1. Apply source_type-based boost/penalty
    // 2. Use configurable weights instead of hardcoded values
}
```

```rust
// crates/server/src/handlers/search.rs — RIIR-aware search

pub struct SearchRequest {
    pub query: String,
    // NEW: query context for source weighting
    pub context: Option<QueryContext>,
}

pub enum QueryContext {
    CodeGeneration,  // weight: code_boost=10, doc_penalty=0.5
    Explanation,     // weight: code_boost=1, doc_penalty=1 (balanced)
    Debugging,       // weight: code_boost=5, doc_penalty=0.8
}

impl QueryContext {
    pub fn rrf_weights(&self) -> RrfWeights {
        match self {
            Self::CodeGeneration => RrfWeights {
                code_boost: 10.0,
                doc_penalty: 0.5,
                ..Default::default()
            },
            Self::Explanation => RrfWeights::default(),
            Self::Debugging => RrfWeights {
                code_boost: 5.0,
                doc_penalty: 0.8,
                ..Default::default()
            },
        }
    }
}
```

### Part 2: Concept Sharding via Metadata Tags

Instead of separate databases (complex, requires schema changes), use metadata filtering on the existing single DB:

```rust
// crates/lib/src/search.rs — concept-aware search

/// Concept tags for Rust-specific queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RustConcept {
    Lifetimes,
    Macros,
    Async,
    Traits,
    Generics,
    ErrorHandling,
    Ownership,
    FFI,
    Testing,
    Concurrency,
}

/// Classify a query into concept tags using the existing LLM query analysis.
/// Reuses the entity/keyphrase extraction already in the search pipeline.
fn classify_concepts(entities: &[String], keyphrases: &[String]) -> Vec<RustConcept> {
    // Simple keyword mapping (no extra LLM call needed)
    let mut concepts = Vec::new();
    for entity in entities.iter().chain(keyphrases.iter()) {
        let e = entity.to_lowercase();
        if e.contains("lifetime") || e.contains("'a") { concepts.push(RustConcept::Lifetimes); }
        if e.contains("macro") || e.contains("macro_rules") { concepts.push(RustConcept::Macros); }
        if e.contains("async") || e.contains("await") || e.contains("tokio") { concepts.push(RustConcept::Async); }
        if e.contains("trait") || e.contains("impl") { concepts.push(RustConcept::Traits); }
        if e.contains("generic") || e.contains("<t>") { concepts.push(RustConcept::Generics); }
        if e.contains("result") || e.contains("option") || e.contains("?") { concepts.push(RustConcept::ErrorHandling); }
        if e.contains("ownership") || e.contains("borrow") || e.contains("move") { concepts.push(RustConcept::Ownership); }
        if e.contains("ffi") || e.contains("extern") || e.contains("unsafe") { concepts.push(RustConcept::FFI); }
        if e.contains("test") || e.contains("#[test]") { concepts.push(RustConcept::Testing); }
        if e.contains("thread") || e.contains("mutex") || e.contains("arc") { concepts.push(RustConcept::Concurrency); }
    }
    concepts.sort();
    concepts.dedup();
    concepts
}
```

```rust
// crates/lib/src/search.rs — concept-filtered vector search

/// Vector search filtered by concept metadata.
pub async fn vector_search_by_concept(
    db: &dyn DbProvider,
    embedding: &[f64],
    concepts: &[RustConcept],
    limit: usize,
) -> Result<Vec<SearchResult>> {
    // Add concept tags to the WHERE clause of the vector search query
    // e.g., WHERE metadata LIKE '%{"concept":"Lifetimes"}%'
    // Falls back to unfiltered search if no concepts match
}
```

```rust
// crates/lib/src/ingest/ — tag documents with concepts during ingestion

/// During ingestion, auto-tag documents with Rust concepts
/// based on content analysis (keyword matching, no LLM needed).
pub fn tag_rust_concepts(content: &str) -> Vec<RustConcept> {
    // Same keyword mapping as classify_concepts, applied to content
}
```

## Dependency Additions

None — all changes use existing dependencies (SQLite, existing LLM provider).

## Tasks

### Phase 1: Source-Type Tagging
- [x] 1.1 Add `SearchSourceType` enum to `crates/lib/src/types.rs`
- [x] 1.2 Add `source_type` field to `SearchResult`
- [x] 1.3 Tag results from `/search/examples` as `Code`
- [x] 1.4 Tag results from `/search/knowledge` as `Documentation` or `Faq`
- [x] 1.5 Update all search functions to populate `source_type`
- [x] 1.6 Add test: source type correctly populated for each search type

### Phase 2: Weighted RRF
- [x] 2.1 Add `RrfWeights` struct to `crates/lib/src/rerank.rs`
- [x] 2.2 Add `reciprocal_rank_fusion_weighted()` function
- [x] 2.3 Replace hardcoded `100.0` bias with `RrfWeights::default()`
- [x] 2.4 Add `QueryContext` enum to server types
- [x] 2.5 Update `/search/hybrid` and `/search/knowledge` handlers to accept `context` param
- [x] 2.6 Add test: `CodeGeneration` context boosts code results above doc results
- [x] 2.7 Add test: `Explanation` context treats all sources equally

### Phase 3: Concept Classification
- [x] 3.1 Add `RustConcept` enum to `crates/lib/src/types.rs`
- [x] 3.2 Add `classify_concepts()` to `crates/lib/src/search.rs`
- [x] 3.3 Add concept tagging to ingestion pipeline (`tag_rust_concepts`)
- [x] 3.4 Store concept tags in document metadata JSON
- [x] 3.5 Add concept-filtered vector search
- [x] 3.6 Add test: concept classification from query entities
- [x] 3.7 Add test: concept filtering returns only matching documents

### Phase 4: Integration & Validation
- [x] 4.1 Update `/search/knowledge` to use concept classification + weighted RRF
- [x] 4.2 Update `/search/examples` to tag results as `Code` source type
- [x] 4.3 Add CLI flag: `--context code_generation` for gof search
- [x] 4.4 Manual test: RIIR query returns mostly code results, not prose
- [x] 4.5 Run `cargo test --workspace`
- [x] 4.6 Run `cargo clippy --workspace`

## Key Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Keyword-based concept classification misses edge cases | Some queries unclassified | Falls back to unfiltered search; add more keywords iteratively |
| Code boost too aggressive | Misses relevant documentation | `QueryContext` makes it tunable per request |
| Metadata JSON size grows with concept tags | Slightly larger DB rows | Tags are small strings; SQLite handles this fine |
| Breaking change to `SearchResult` | Downstream code may need updates | `source_type` defaults to `Unknown`; backward compatible |

## Expected Outcomes

1. `SearchSourceType` — every search result knows its origin (code vs doc vs FAQ)
2. `RrfWeights` — configurable fusion weights replacing hardcoded values
3. `QueryContext` — client can request code-biased or balanced search
4. `RustConcept` — auto-tagging and concept-filtered search
5. RIIR queries return executable code, not explanatory prose

## Files to Create/Modify

| File | Action | Phase |
|------|--------|-------|
| `crates/lib/src/types.rs` | Add `SearchSourceType`, `RustConcept`, `QueryContext` | 1, 3 |
| `crates/lib/src/rerank.rs` | Add `RrfWeights`, `reciprocal_rank_fusion_weighted` | 2 |
| `crates/lib/src/search.rs` | Add `classify_concepts`, concept-filtered search | 3 |
| `crates/lib/src/ingest/mod.rs` | Add `tag_rust_concepts` | 3 |
| `crates/server/src/handlers/search.rs` | Accept `context` param, use weighted RRF | 2, 4 |
| `crates/cli/src/main.rs` | Add `--context` flag | 4 |

## References

- `.research/00_Neuro-Symbolic LLM Architecture.md` — §Context Pollution, §Concept Sharding
- `.research/01_Advanced Neuro-Symbolic Rust Translation.md` — §Latency Mitigation via anyrag
- `microgpt-rs/.plans/009_rest_speculative_decoding.md` — REST bridge consumes these APIs