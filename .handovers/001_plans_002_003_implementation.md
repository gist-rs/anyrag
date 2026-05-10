# Handover 001: Plans 002 & 003 Implementation

## What Happened

Implemented both Plan 002 (Context Pollution Prevention + Concept Sharding) and Plan 003 (Self-Improving Cycle — JSONL Export Pipeline) for the anyrag project. Both plans are now feature-complete with all core logic, API endpoints, and tests passing.

### Plan 002: Context Pollution Prevention

1. **Source-Type Tagging** — Added `SearchSourceType` enum (`Code`, `Documentation`, `Faq`, `Unknown`) to `SearchResult`. All search result construction sites updated: SQLite provider (`vector_search`, `keyword_search`, `metadata_search`), GitHub search (`keyword_search_for_repo`, `vector_search_for_repo`), and hybrid search chunk expansion.

2. **Weighted RRF** — Replaced hardcoded `100.0` bias in `reciprocal_rank_fusion` with configurable `RrfWeights` struct. New `reciprocal_rank_fusion_weighted()` applies source-type multipliers (`code_boost`, `doc_penalty`). Old function preserved as backward-compatible wrapper. `QueryContext` enum (`CodeGeneration`, `Explanation`, `Debugging`) maps to preset weights.

3. **Concept Classification** — Added `RustConcept` enum (10 variants: Lifetimes, Macros, Async, Traits, Generics, ErrorHandling, Ownership, FFI, Testing, Concurrency). `classify_concepts()` extracts concepts from query entities/keyphrases via keyword matching. `tag_rust_concepts()` tags document content during ingestion. `store_concept_tags()` persists tags as `RUST_CONCEPT` metadata subtype.

### Plan 003: Self-Improving Cycle

1. **Episodic Memory** — `TranslationEpisode`, `CompilationResult`, `EpisodicStats` types. `EpisodicIngester` with `record_episode`, `verify_episode`, `get_successful_episodes`, `get_stats`, `mark_synthesized`. New `episodes` table in SQLite schema.

2. **Training Data Synthesis** — Extended `Curator` with `synthesize_training_data()` that groups successful episodes by source language and uses LLM to create canonical Q&A pairs.

3. **JSONL Export** — Extended `Curator` with `export_training_jsonl()` that combines FAQ pairs + translation episodes into JSONL for fine-tuning.

4. **Cycle Orchestrator** — `SelfImprovingCycle` state machine (7 states: Collecting → ReadyToSynthesize → Synthesizing → ReadyToExport → Exporting → Training → Upgrading). `CycleConfig` with configurable thresholds. Integrated into `AppState` with `Mutex<SelfImprovingCycle>`.

5. **API Endpoints** — `POST /episodes`, `GET /episodes`, `GET /episodes/stats`, `POST /episodes/{id}/verify`, `GET /cycle/status`, `POST /cycle/trigger`.

## Where Is the Plan/Code/Test

### Plans
- `.plans/002_context_pollution_prevention.md` — 26/26 tasks complete
- `.plans/003_self_improving_cycle.md` — 33/35 tasks complete (2 docs tasks remain)

### Code — New Files
| File | Purpose |
|------|---------|
| `crates/lib/src/ingest/episodic.rs` | Episodic memory ingester (5 async functions) |
| `crates/lib/src/cycle.rs` | Self-improving cycle state machine |
| `crates/server/src/handlers/episodes.rs` | Episode + cycle API handlers (6 endpoints) |
| `crates/lib/tests/weighted_rrf_test.rs` | Weighted RRF tests (6 tests) |
| `crates/lib/tests/concept_classification_test.rs` | Concept classification tests (13 tests) |

### Code — Modified Files
| File | Changes |
|------|---------|
| `crates/lib/src/types.rs` | `SearchSourceType`, `QueryContext`, `RustConcept`, `TranslationEpisode`, `CompilationResult`, `EpisodicStats` |
| `crates/lib/src/rerank.rs` | `RrfWeights`, `reciprocal_rank_fusion_weighted()`, backward-compatible wrapper |
| `crates/lib/src/search.rs` | `classify_concepts()`, `tag_rust_concepts()`, `concepts` field on `HybridSearchOptions` |
| `crates/lib/src/curator.rs` | `synthesize_training_data()`, `export_training_jsonl()`, `SynthesisStats`, `TrainingExport` |
| `crates/lib/src/ingest/knowledge.rs` | `store_concept_tags()` for metadata tagging |
| `crates/lib/src/ingest/mod.rs` | Added `pub mod episodic;` |
| `crates/lib/src/lib.rs` | Added `pub mod cycle;`, re-exports for all new types |
| `crates/lib/src/providers/db/sqlite/sql.rs` | `CREATE_EPISODES_TABLE_SQL` |
| `crates/lib/src/providers/db/sqlite/mod.rs` | `SearchSourceType` tagging on search results |
| `crates/server/src/handlers/search.rs` | `context` field on `SearchRequest`, weighted RRF in hybrid handler |
| `crates/server/src/handlers/mod.rs` | Added `pub mod episodes;` |
| `crates/server/src/router.rs` | 6 new routes for episodes + cycle |
| `crates/server/src/state.rs` | `cycle: Arc<Mutex<SelfImprovingCycle>>` in AppState |
| `crates/github/src/ingest/search_logic.rs` | `SearchSourceType::Code` on GitHub search results |
| Various example/test files | Added `source_type` and `concepts: None` fields |

## Reflection — Struggling / Solved

1. **SearchResult breaking change** — Adding `source_type` field to `SearchResult` broke ~20 construction sites across the workspace (lib, server, github, gof, tests, examples). Solved by systematic grep + targeted fixes. `#[serde(default)]` and `#[default]` on `SearchSourceType` handled backward-compatible deserialization.

2. **Sub-agent formatting error** — One edit to `search.rs` left `<newtext>/<old_text>` markers in the file, corrupting the handler. Caught by `cargo check`, fixed with a clean overwrite.

3. **Type mismatch in curator** — `episodic::get_successful_episodes` takes `Option<&str>` for `since`, but curator was passing `since.map(String::from)` (producing `Option<String>`). Fixed by passing `since` directly since curator methods already take `Option<&str>`.

4. **HybridSearchOptions missing field cascade** — Adding `concepts: Option<Vec<RustConcept>>` to `HybridSearchOptions` broke 5 files (generation_handlers, knowledge handler, 3 tests, 1 example). Solved by delegating to a sub-agent for systematic fixes.

## Remain Work

### Plan 002
- Manual integration testing with real RIIR queries to verify code boost works end-to-end
- Optional: CLI `--context` flag for `gof` search (skipped — not in gof scope)

### Plan 003
- [ ] 5.3 Document: how microgpt-rs Plan 008 trainer consumes the JSONL
- [ ] 5.4 Document: how microgpt-rs hot-reloads trained model weights
- JSONL export endpoint wiring to file system (currently returns JSONL in response body)
- Hot-reload trigger to microgpt-rs API (placeholder exists in `CycleConfig::model_api_url`)
- Background tick task (currently manual trigger via `POST /cycle/trigger`)

## Issues Ref

No new issues created. Pre-existing failures remain:
- `anyrag-github` extractor tests (2 failures — file path related, not from our changes)
- `anyrag` zai doc-test (missing `reev_agent` crate — pre-existing)

## How to Dev/Test

### Build & Check
```bash
cd /Users/katopz/git/gist/anyrag
cargo check --workspace
cargo clippy --workspace
```

### Run Tests
```bash
# New tests for Plan 002
cargo test -p anyrag --test weighted_rrf_test --test concept_classification_test

# Existing RRF test (still passes)
cargo test -p anyrag --test rerank_test

# Full lib tests (excludes doc-tests)
cargo test -p anyrag --lib --tests

# Full workspace (has 2 pre-existing failures in github crate)
cargo test --workspace
```

### API Usage Examples

**Record a translation episode:**
```bash
curl -X POST http://localhost:9090/episodes \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_language": "python",
    "source_code": "def hello(): print(\"hello\")",
    "generated_rust": "fn hello() { println!(\"hello\"); }",
    "retrieved_context": [],
    "hidden_state": null
  }'
```

**Verify episode compilation:**
```bash
curl -X POST http://localhost:9090/episodes/{id}/verify \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"compilation_result": {"type": "success", "warnings": 0, "clippy_lints": 1}}'
```

**Weighted RRF search (RIIR context):**
```bash
curl -X POST http://localhost:9090/search/hybrid \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "how to use lifetimes", "context": "code_generation", "mode": "rrf"}'
```

**Cycle status & trigger:**
```bash
curl http://localhost:9090/cycle/status -H "Authorization: Bearer $TOKEN"
curl -X POST http://localhost:9090/cycle/trigger -H "Authorization: Bearer $TOKEN"
```
