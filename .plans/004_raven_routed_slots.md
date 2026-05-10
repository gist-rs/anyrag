# Plan 004: Raven Routed Slots — Deterministic Slot Memory for Code RAG

## Summary

Integrate Raven RSM (Routed Slot Memory) concepts into anyrag as an **additive layer** over the existing search pipeline. Documents are assigned to named "slots" (e.g., `architecture`, `types`, `apis`, `dependencies`, `tests`, `chatter`) via deterministic keyword matching during ingestion. Search can then target active slots only, reducing context pollution and improving retrieval precision.

Key ideas from the Raven paper:
- **Fixed slot memory**: bounded number of named slots, each holding a curated document set
- **Sparse Top-K routing**: only a subset of slots are "active" per query (deterministic keyword match, NOT neural)
- **Selective decay** (Equation 18): non-frozen slots decay over time, reducing relevance scores
- **Frozen slots**: critical slots (e.g., `architecture`) never decay — always available at full strength

This is **Phase 1** — keyword-based routing only. Neural routing is deferred to Phase 2, following embedding-based pattern.

## Architecture

### Slot Schema

```rust
// crates/lib/src/slots/types.rs

/// Named slot for categorizing ingested documents.
/// Maps to Raven's "slot" concept — a bounded memory partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub id: String,           // UUID v7
    pub name: SlotName,       // e.g. "architecture", "types", "apis"
    pub description: String,
    pub is_frozen: bool,      // frozen slots never decay (Raven: unselected = preserved)
    pub decay_rate: f64,      // Raven Equation 18: λ (lambda), 0.0 = no decay
    pub max_documents: usize, // soft cap per slot
    pub keywords: Vec<String>,// routing keywords (deterministic matching)
    pub created_at: String,
    pub updated_at: String,
}

/// Default slot names for code RAG workloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SlotName {
    /// System design, module structure, high-level patterns. FROZEN — never decays.
    Architecture,
    /// Type definitions, structs, enums, type aliases.
    Types,
    /// Public API surfaces, function signatures, trait definitions.
    Apis,
    /// Crate dependencies, version constraints, feature flags.
    Dependencies,
    /// Test files, test utilities, benchmark harnesses.
    Tests,
    /// Conversational context, chat logs, informal notes. High decay.
    Chatter,
    /// User-defined custom slot.
    Custom(String),
}

/// Association between a document and a slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDocument {
    pub id: String,           // UUID v7
    pub slot_name: SlotName,
    pub document_id: String,  // FK to documents table
    pub routed_by: RouteMethod,
    pub routed_at: String,    // ISO timestamp
    pub relevance_score: f64, // initial score, decays over time
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteMethod {
    /// Phase 1: deterministic keyword matching.
    Keyword,
    /// Phase 2 (future): embedding-based neural routing.
    Neural,
}

/// Result of routing a document to slots.
#[derive(Debug, Clone)]
pub struct RouteResult {
    pub document_id: String,
    pub assigned_slots: Vec<SlotName>,
    pub matched_keywords: HashMap<SlotName, Vec<String>>,
}
```

### Raven Equation 18: Selective Decay

The Raven paper defines per-slot decay as:

```
score(t) = score_0 * exp(-λ * Δt)
```

Where:
- `score_0` = initial relevance score (1.0 at routing time)
- `λ` (lambda) = decay rate per slot (0.0 for frozen slots)
- `Δt` = time elapsed since routing in days

In SQL (Turso/libsql):

```sql
-- Compute decayed relevance for a slot document
SELECT
    sd.*,
    sd.relevance_score * EXP(-s.decay_rate * JULIANDAY('now') - JULIANDAY(sd.routed_at)) AS decayed_score
FROM slot_documents sd
JOIN rag_slots s ON s.name = sd.slot_name
WHERE s.is_frozen = FALSE  -- frozen slots skip decay calc
```

For frozen slots (`is_frozen = TRUE`), `decay_rate = 0.0`, so `exp(0) = 1.0` — score never changes.

### Keyword Router

```rust
// crates/lib/src/slots/router.rs

/// Deterministic keyword-based router. No LLM, no neural net.
/// Matches document content against slot keyword lists.
pub struct KeywordRouter {
    slot_definitions: Vec<Slot>,
}

impl KeywordRouter {
    /// Route a document to matching slots based on content analysis.
    /// Returns all matching slots (a document can live in multiple slots).
    pub fn route(&self, content: &str, metadata: &HashMap<String, String>) -> RouteResult {
        let content_lower = content.to_lowercase();
        let mut assigned_slots = Vec::new();
        let mut matched_keywords = HashMap::new();

        for slot in &self.slot_definitions {
            let matches: Vec<String> = slot.keywords.iter()
                .filter(|kw| content_lower.contains(&kw.to_lowercase()))
                .cloned()
                .collect();

            if !matches.is_empty() {
                assigned_slots.push(slot.name.clone());
                matched_keywords.insert(slot.name.clone(), matches);
            }
        }

        RouteResult {
            document_id: String::new(), // caller sets this
            assigned_slots,
            matched_keywords,
        }
    }
}

/// Default keyword rules for code RAG slots.
/// These are intentionally conservative — false negatives are better than false positives.
pub fn default_slot_keywords() -> HashMap<SlotName, Vec<String>> {
    let mut map = HashMap::new();
    map.insert(SlotName::Architecture, vec![
        "mod.rs", "lib.rs", "main.rs", "cargo.toml", "architecture",
        "module", "crate", "workspace", "design", "overview", "structure",
        "diagram", "layout",
    ]);
    map.insert(SlotName::Types, vec![
        "struct ", "enum ", "type ", "impl ", "trait ",
        "pub struct", "pub enum", "type alias", "newtype",
    ]);
    map.insert(SlotName::Apis, vec![
        "pub fn ", "pub async fn ", "pub trait ", "pub mod ",
        "#[export_name", "no_mangle", "extern \"c\"",
        "api", "endpoint", "handler", "route",
    ]);
    map.insert(SlotName::Dependencies, vec![
        "[dependencies]", "[dev-dependencies]", "cargo.toml",
        "crate =", "version =", "features =", "path =",
    ]);
    map.insert(SlotName::Tests, vec![
        "#[test]", "#[tokio::test]", "mod tests", "fn test_",
        "assert!", "assert_eq!", "assert_ne!", "#[bench]",
        "bench", "criterion", "proptest",
    ]);
    map.insert(SlotName::Chatter, vec![
        "todo", "fixme", "hack", "xxx", "note:",
        "changelog", "contributing", "readme",
    ]);
    map
}
```

### Routed Search

```rust
// crates/lib/src/slots/search.rs

/// Slot-aware search: only retrieve from active slots.
/// Wraps the existing search pipeline, adding a slot_documents JOIN filter.
pub struct SlotSearch<'a> {
    conn: &'a Connection,
    active_slots: &'a [SlotName],
    include_frozen: bool, // always include frozen slots (architecture)
}

impl SlotSearch<'_> {
    /// Build a SQL WHERE clause that filters documents to active slots only.
    /// Frozen slots are always included regardless of active_slots.
    pub fn slot_filter_sql(&self) -> (String, Vec<Value>) {
        // WHERE d.id IN (
        //   SELECT sd.document_id FROM slot_documents sd
        //   JOIN rag_slots s ON s.name = sd.slot_name
        //   WHERE s.name IN (?1, ?2, ...) OR s.is_frozen = TRUE
        // )
    }
}
```

## Database Schema

Two new tables, additive — no changes to existing tables:

```sql
-- Named slots for routed memory.
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

-- Document-to-slot associations (many-to-many).
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
```

## Dependency Additions

None — all changes use existing dependencies (turso/libsql, serde, chrono, uuid v7).

## Tasks

### Phase 1: Types & Schema
- [x] 1.1 Create `crates/lib/src/slots/mod.rs` — module index, re-exports
- [x] 1.2 Create `crates/lib/src/slots/types.rs` — `Slot`, `SlotName`, `SlotDocument`, `RouteMethod`, `RouteResult`
- [x] 1.3 Add `CREATE_RAG_SLOTS_TABLE_SQL` and `CREATE_SLOT_DOCUMENTS_TABLE_SQL` to `crates/lib/src/providers/db/sqlite/sql.rs`
- [x] 1.4 Append new table SQL to `ALL_TABLE_CREATION_SQL` array
- [x] 1.5 Add `pub mod slots;` to `crates/lib/src/lib.rs`
- [x] 1.6 Add test: slot types serialize/deserialize correctly
- [x] 1.7 Add test: schema creation succeeds on in-memory Turso DB

### Phase 2: Default Slot Seeding & Keyword Router
- [x] 2.1 Create `crates/lib/src/slots/router.rs` — `KeywordRouter`, `default_slot_keywords()`
- [x] 2.2 Create `crates/lib/src/slots/seeder.rs` — `seed_default_slots()` for code RAG schema
- [x] 2.3 Implement `KeywordRouter::route()` — content + metadata → `RouteResult`
- [x] 2.4 Default slot config: `architecture` (frozen, decay_rate=0.0), `types` (0.05), `apis` (0.05), `dependencies` (0.1), `tests` (0.1), `chatter` (0.5)
- [x] 2.5 Add test: `architecture` slot keywords match `mod.rs` and `lib.rs` content
- [x] 2.6 Add test: single document routes to multiple slots (e.g., `types` + `apis`)
- [x] 2.7 Add test: frozen slot has `decay_rate = 0.0`
- [x] 2.8 Add test: `chatter` slot has highest decay rate

### Phase 3: Ingestion Integration (Additive Hook)
- [x] 3.1 Create `crates/lib/src/slots/ingest.rs` — `SlotIngester` that wraps existing `Ingestor` trait
- [x] 3.2 After document insertion, call `KeywordRouter::route()` on content
- [x] 3.3 Insert matching rows into `slot_documents` table
- [x] 3.4 Add `route_to_slots` flag to ingestion config (opt-in, default off) — implemented as separate `SlotIngester`, effectively opt-in
- [x] 3.5 Add test: ingest a Rust struct → routes to `types` slot
- [x] 3.6 Add test: ingest a `#[test]` function → routes to `tests` slot
- [x] 3.7 Add test: ingest `cargo.toml` content → routes to `dependencies` slot

### Phase 4: Selective Decay (Raven Equation 18)
- [x] 4.1 Create `crates/lib/src/slots/decay.rs` — decay calculation logic
- [x] 4.2 Implement `decayed_score(relevance_score, decay_rate, routed_at) -> f64`
- [x] 4.3 Implement `apply_decay_batch()` — SQL UPDATE for all non-frozen slot documents
- [x] 4.4 Add decay SQL to slot search queries: `relevance_score * EXP(-decay_rate * JULIANDAY('now') - JULIANDAY(routed_at))`
- [x] 4.5 Add test: frozen slot score remains 1.0 after 30 days
- [x] 4.6 Add test: `chatter` slot decays to ~0.01 after 7 days with λ=0.5
- [x] 4.7 Add test: `types` slot decays slowly (~0.7 after 7 days with λ=0.05)

### Phase 5: Routed Search Endpoint
- [x] 5.1 Create `crates/lib/src/slots/search.rs` — `SlotSearch` with slot filter
- [x] 5.2 Add `POST /search/slots` endpoint to `crates/server/src/handlers/slots.rs`
- [x] 5.3 Request accepts `active_slots: Vec<SlotName>` and always includes frozen slots
- [x] 5.4 Build SQL filter: `WHERE document_id IN (SELECT ... FROM slot_documents WHERE slot_name IN (...) OR is_frozen = TRUE)`
- [x] 5.5 Slots filter candidates only; existing RRF handles ranking separately — out of scope for Phase 1
- [x] 5.6 Add `SlotSearchRequest` and `SlotSearchResponse` types in `crates/server/src/handlers/slots.rs`
- [x] 5.7 Register route in `crates/server/src/router.rs`
- [x] 5.8 Add test: search with `active_slots = ["apis"]` returns API docs + frozen architecture docs
- [x] 5.9 Add test: search with no active slots returns only frozen slot documents
- [x] 5.10 Add test: slot search endpoint returns 404 for documents not in any slot

### Phase 6: Slot Management Endpoints
- [x] 6.1 Add `GET /slots` — list all slots with document counts
- [x] 6.2 Add `POST /slots` — create custom slot (name, keywords, decay_rate)
- [x] 6.3 Add `GET /slots/{name}/documents` — list documents in a slot
- [x] 6.4 Add `DELETE /slots/{name}/documents/{doc_id}` — remove document from slot
- [x] 6.5 Add `POST /slots/reindex` — re-route all documents through keyword router
- [x] 6.6 Add test: create custom slot, ingest doc, verify routing
- [x] 6.7 Add test: reindex re-routes documents after keyword changes

### Phase 7: Benchmarks & Cleanup
- [x] 7.1 Benchmark: keyword routing throughput (docs/sec) for 1000 documents
- [x] 7.2 Benchmark: slot-filtered search vs unfiltered search latency
- [x] 7.3 Benchmark: decay calculation overhead on 10K slot_documents
- [x] 7.4 Run `cargo clippy --workspace --allow-dirty`
- [x] 7.5 Run `cargo test --workspace`
- [x] 7.6 Verify existing search pipeline is untouched (regression test)

## Key Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Keyword router too aggressive, misroutes docs | Wrong slot results | Conservative keyword lists; doc can be in multiple slots; manual override via API |
| Decay makes useful docs vanish | Lost context | Low default decay rates; frozen architecture slot; reindex endpoint to re-route |
| Slot_documents table grows large | Query slowdown | Unique index on (slot_name, document_id); periodic cleanup of decayed docs |
| Breaking existing search pipeline | Regression | All slot features are opt-in; existing endpoints untouched; separate `/search/slots` route |
| Future neural router incompatible with schema | Migration pain | `RouteMethod` enum already has `Neural` variant; same schema works for both |

## Expected Outcomes

1. **Two new tables** — `rag_slots` and `slot_documents`, additive to existing schema
2. **Deterministic keyword router** — no LLM, no neural net, pure string matching
3. **Selective decay** — Raven Equation 18 implemented in SQL, per-slot configurable
4. **Frozen architecture slot** — system design docs always available at full strength
5. **Routed search endpoint** — `/search/slots` targets active slots only
6. **Slot management API** — CRUD for custom slots, reindex endpoint
7. **Zero changes to existing pipeline** — all existing endpoints and flows untouched

## Files to Create

| File | Phase | Purpose |
|------|-------|---------|
| `crates/lib/src/slots/mod.rs` | 1 | Module index, re-exports |
| `crates/lib/src/slots/types.rs` | 1 | Slot, SlotName, SlotDocument, RouteMethod, RouteResult |
| `crates/lib/src/slots/router.rs` | 2 | KeywordRouter, default_slot_keywords() |
| `crates/lib/src/slots/seeder.rs` | 2 | seed_default_slots() for code RAG |
| `crates/lib/src/slots/ingest.rs` | 3 | Slot-aware ingestion hook |
| `crates/lib/src/slots/decay.rs` | 4 | Raven Equation 18 implementation |
| `crates/lib/src/slots/search.rs` | 5 | Slot-filtered search logic |
| `crates/lib/tests/slots_test.rs` | 5-6 | Integration tests |

## Files to Modify

| File | Phase | Change |
|------|-------|--------|
| `crates/lib/src/lib.rs` | 1 | Add `pub mod slots;` |
| `crates/lib/src/providers/db/sqlite/sql.rs` | 1 | Add slot table SQL constants |
| `crates/server/src/handlers/search.rs` | 5 | Add `/search/slots` handler |
| `crates/server/src/handlers/mod.rs` | 5-6 | Export slot handlers |
| `crates/server/src/router.rs` | 5-6 | Register slot routes |
| `crates/server/src/types.rs` | 5 | Add SlotSearchRequest/Response |

## Out of Scope

- **Neural router** — Phase 2, embedding-based routing
- **LLM-based slot routing** — intentionally deterministic only
- **Modification of existing search endpoints** — `/search/hybrid`, `/search/knowledge`, `/search/examples` are untouched
- **Slot-based re-ranking** — slots only filter candidates; existing RRF handles ranking
- **Multi-tenant slot isolation** — slots are global per DB, not per user (future consideration)
- **Slot migration/compaction** — cleanup of heavily decayed documents is manual for now
- **Vector-level slot indexing** — slots filter at document level, not embedding level
- **Slot embeddings** — no slot-level embeddings or slot similarity search

## Cross-Project References

- Raven RSM paper — Equation 18 (selective decay), fixed slot memory, sparse Top-K routing
- `microgpt-rs/.plans/*` — plan format reference
- microgpt-rs — Phase 2 neural router will follow this pattern
- `anyrag/.plans/002_context_pollution_prevention.md` — `RrfWeights`, `SearchSourceType`, `RustConcept` (slot search reuses these)
- `anyrag/.plans/003_self_improving_cycle.md` — episodic memory schema pattern (Turso/libsql with UUID v7)