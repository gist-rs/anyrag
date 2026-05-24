# Handover 002: Raven Routed Slots Implementation

## What Happened

Implemented **Plan 004: Raven Routed Slots** — a deterministic slot memory system for code RAG. Documents are assigned to named "slots" (e.g., `architecture`, `types`, `apis`, `dependencies`, `tests`, `chatter`) via keyword matching during ingestion. Search can target active slots only, reducing context pollution and improving retrieval precision.

Key achievements:
- **Two new tables**: `rag_slots` and `slot_documents` — additive to existing schema, zero changes to existing tables
- **Deterministic keyword router**: No LLM, no neural net — pure string matching against slot keyword lists
- **Selective decay (Raven Equation 18)**: `score(t) = score_0 * exp(-λ * Δt)` — per-slot configurable, frozen slots never decay
- **Frozen architecture slot**: System design docs always available at full strength (decay_rate=0.0)
- **6 server endpoints**: `GET /slots`, `POST /slots`, `GET /slots/{name}/documents`, `DELETE /slots/{name}/documents/{doc_id}`, `POST /slots/reindex`, `POST /search/slots`
- **47 unit tests**: All passing, covering types, router, seeder, decay, search SQL generation
- **Zero regressions**: All existing tests continue to pass

## Where Is the Plan/Code/Test

### Plan
- `.plans/004_raven_routed_slots.md` — full plan with architecture, schema, tasks

### Code — New Files (7 files)
| File | Purpose |
|------|---------|
| `crates/lib/src/slots/mod.rs` | Module index, re-exports |
| `crates/lib/src/slots/types.rs` | `Slot`, `SlotName`, `SlotDocument`, `RouteMethod`, `RouteResult` |
| `crates/lib/src/slots/router.rs` | `KeywordRouter`, `default_slot_keywords()` with unit tests |
| `crates/lib/src/slots/seeder.rs` | `seed_default_slots()` for code RAG with unit tests |
| `crates/lib/src/slots/decay.rs` | Raven Equation 18 implementation with unit tests |
| `crates/lib/src/slots/search.rs` | Slot filter SQL, `SlotSearchConfig`, slot document queries with unit tests |
| `crates/lib/src/slots/ingest.rs` | `SlotIngester` — route, persist, reindex, decay batch with unit tests |
| `crates/server/src/handlers/slots.rs` | 6 HTTP handler endpoints |

### Code — Modified Files (4 files)
| File | Change |
|------|--------|
| `crates/lib/src/lib.rs` | Added `pub mod slots;` and re-exports |
| `crates/lib/src/providers/db/sqlite/sql.rs` | Added `CREATE_RAG_SLOTS_TABLE_SQL`, `CREATE_SLOT_DOCUMENTS_TABLE_SQL` |
| `crates/lib/src/Cargo.toml` | Made `uuid` non-optional with `v7` feature |
| `crates/server/src/handlers/mod.rs` | Added `pub mod slots;` and `pub use slots::*;` |
| `crates/server/src/router.rs` | Added 6 slot routes with `delete` import |

### Tests
- 47 inline unit tests across `types`, `router`, `seeder`, `decay`, `search`, `ingest` modules
- Run: `cargo test -p anyrag --lib -- slots`
- Result: **49 total tests pass** (47 slots + 2 existing)

### Branch
- `feature/004_raven_routed_slots` (from `develop`)

## Reflection — Struggling/Solved

### Solved
1. **Turso type inference**: `Vec<_>` params need explicit `Vec<turso::Value>` annotation — discovered from existing codebase patterns in `curator.rs` and `sqlite/mod.rs`
2. **`Eq` derive on `f64`**: Removed `Eq` from `Slot` struct since it contains `f64` (only `PartialEq` is valid)
3. **Clippy `vec_init_then_push`**: Refactored `seed_default_slots()` to use `vec![]` macro
4. **SQL substring false positive**: Test checking `!contains("IN")` matched "is_frozen" — fixed to check `sd.slot_name IN` specifically
5. **SQL column name mismatch**: Used `decayed_score` in SQL but test checked for `decay_score` — fixed test

### No Blockers
Clean implementation — the existing codebase patterns made it straightforward.

## Remain Work

### Integration Tests (not yet done)
- [ ] 5.8 Test: search with `active_slots = ["apis"]` returns API docs + frozen architecture docs
- [ ] 5.9 Test: search with no active slots returns only frozen slot documents
- [ ] 5.10 Test: slot search endpoint returns 404 for documents not in any slot
- [ ] 6.6 Test: create custom slot, ingest doc, verify routing
- [ ] 6.7 Test: reindex re-routes documents after keyword changes

These should be added to `crates/server/tests/` or `crates/lib/tests/slots_test.rs` as integration tests requiring a real Turso DB.

### Benchmarks (not yet done)
- [ ] 7.1 Benchmark: keyword routing throughput (docs/sec) for 1000 documents
- [ ] 7.2 Benchmark: slot-filtered search vs unfiltered search latency
- [ ] 7.3 Benchmark: decay calculation overhead on 10K slot_documents

### Future (Phase 2 — Out of Scope)
- Neural router following `katgpt-rs` pattern
- LLM-based slot routing
- Slot-based re-ranking integration with existing RRF pipeline

## Issues Ref

None — no issues encountered during implementation.

## How to Dev/Test

### Run slot unit tests
```sh
cargo test -p anyrag --lib -- slots
```

### Run full workspace tests
```sh
cargo test --workspace
```

### Check compilation
```sh
cargo check --workspace --quiet
```

### Run clippy
```sh
cargo clippy --workspace --fix --allow-dirty
```

### Start server with slot endpoints
```sh
RUST_LOG=info cargo run -p anyrag-server --quiet
```

### Test endpoints manually
```sh
# List all slots (auto-seeds defaults if empty)
curl http://localhost:3000/slots

# Create a custom slot
curl -X POST http://localhost:3000/slots \
  -H 'Content-Type: application/json' \
  -d '{"name":"my_slot","description":"Custom slot","decay_rate":0.1,"keywords":["custom","keyword"]}'

# Slot-filtered search
curl -X POST http://localhost:3000/search/slots \
  -H 'Content-Type: application/json' \
  -d '{"active_slots":["apis","types"],"include_frozen":true,"limit":10}'

# List documents in a slot
curl http://localhost:3000/slots/architecture/documents

# Reindex all documents
curl -X POST http://localhost/3000/slots/reindex
```

### Key Design Decisions
1. **Additive only** — no changes to existing tables, endpoints, or search pipeline
2. **SlotIngester is separate** — not integrated into existing `Ingestor` trait, called explicitly by users who want slot routing
3. **Frozen slots always included** — `SlotSearchConfig::include_frozen` defaults to `true`
4. **`SlotName::Custom(String)`** — extensible enum for user-defined slots
5. **`uuid::Uuid::now_v7()`** — time-ordered UUIDs for slot documents